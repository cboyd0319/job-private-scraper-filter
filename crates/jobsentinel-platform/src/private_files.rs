// Handles owner-only private file writes plus bounded Unix permission walks for app-owned trees.

#[cfg(unix)]
use rustix::fd::{AsFd, BorrowedFd, OwnedFd};
#[cfg(unix)]
use rustix::fs::{
    fchmod, fstat, mkdirat, openat, statat, AtFlags, Dir, FileType, Mode, OFlags, CWD,
};
use std::io::{self, Write};
use std::path::{Component, Path};

/// Atomically replace a local file and apply owner-only permissions where supported.
pub fn write_file_atomic_private(path: &Path, content: &str) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("Atomic write requires a parent directory"))?;
    super::ensure_private_dir(parent)
        .map_err(|_| io::Error::other("Failed to create parent directory"))?;

    let mut temp_file = tempfile::NamedTempFile::new_in(parent)?;
    temp_file.write_all(content.as_bytes())?;
    temp_file.as_file().sync_all()?;
    temp_file
        .into_temp_path()
        .persist(path)
        .map_err(|error| error.error)?;
    super::ensure_private_file(path)?;
    sync_parent_dir(parent);
    Ok(())
}

#[cfg(unix)]
fn sync_parent_dir(parent: &Path) {
    let _ = std::fs::File::open(parent).and_then(|directory| directory.sync_all());
}

#[cfg(not(unix))]
fn sync_parent_dir(_parent: &Path) {}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ChildSymlinkBehavior {
    Ignore,
    Reject,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PrivateTreeState {
    Private,
    Missing,
    NeedsRepair,
    ManualReview,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TreeWalkMode {
    Inspect,
    Repair,
}

#[cfg(unix)]
#[derive(Default)]
struct TreeWalkState {
    needs_repair: bool,
}

#[cfg(unix)]
const PRIVATE_DIR_MODE_BITS: u32 = 0o700;
#[cfg(unix)]
const PRIVATE_FILE_MODE_BITS: u32 = 0o600;
#[cfg(unix)]
const PRIVATE_DIR_MODE: Mode = Mode::RWXU;

#[cfg(unix)]
pub(crate) fn ensure_private_dir_tree_unix(path: &Path) -> io::Result<()> {
    let root = open_or_create_private_root(path)?;
    let mut state = TreeWalkState::default();
    walk_private_directory(
        root.as_fd(),
        TreeWalkMode::Repair,
        ChildSymlinkBehavior::Ignore,
        &mut state,
    )
}

#[cfg(unix)]
pub(crate) fn inspect_private_dir_tree_unix(
    path: &Path,
    child_symlink_behavior: ChildSymlinkBehavior,
) -> PrivateTreeState {
    let root = match open_private_root(path) {
        Ok(root) => root,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return PrivateTreeState::Missing,
        Err(_) => return PrivateTreeState::ManualReview,
    };

    let mut state = TreeWalkState::default();
    match walk_private_directory(
        root.as_fd(),
        TreeWalkMode::Inspect,
        child_symlink_behavior,
        &mut state,
    ) {
        Ok(()) if state.needs_repair => PrivateTreeState::NeedsRepair,
        Ok(()) => PrivateTreeState::Private,
        Err(_) => PrivateTreeState::ManualReview,
    }
}

#[cfg(unix)]
pub(crate) fn repair_private_dir_tree_unix(path: &Path) -> io::Result<()> {
    let root = open_or_create_private_root(path)?;
    let mut state = TreeWalkState::default();
    walk_private_directory(
        root.as_fd(),
        TreeWalkMode::Repair,
        ChildSymlinkBehavior::Reject,
        &mut state,
    )
}

#[cfg(unix)]
fn walk_private_directory(
    dirfd: BorrowedFd<'_>,
    mode: TreeWalkMode,
    child_symlink_behavior: ChildSymlinkBehavior,
    state: &mut TreeWalkState,
) -> io::Result<()> {
    visit_directory(dirfd, mode, state)?;

    let entries = Dir::read_from(dirfd).map_err(io::Error::from)?;
    for entry in entries {
        let entry = entry.map_err(io::Error::from)?;
        let name = entry.file_name();
        if is_dot_entry(name) {
            continue;
        }

        let path_metadata =
            statat(dirfd, name, AtFlags::SYMLINK_NOFOLLOW).map_err(io::Error::from)?;
        let file_type = FileType::from_raw_mode(path_metadata.st_mode);

        if file_type.is_symlink() {
            if child_symlink_behavior == ChildSymlinkBehavior::Ignore {
                continue;
            }
            return Err(invalid_private_tree(
                "Private directory trees reject symbolic links inside app-owned storage",
            ));
        }

        if file_type.is_dir() {
            let child = open_directory_relative(dirfd, name)?;
            validate_opened_entry(child.as_fd(), &path_metadata, true)?;
            walk_private_directory(child.as_fd(), mode, child_symlink_behavior, state)?;
            continue;
        }

        if file_type.is_file() {
            if path_metadata.st_nlink != 1 {
                return Err(invalid_private_tree(
                    "Private directory trees require regular files with exactly one link",
                ));
            }

            let child = open_file_relative(dirfd, name)?;
            validate_opened_entry(child.as_fd(), &path_metadata, false)?;
            visit_file(child.as_fd(), mode, state)?;
            continue;
        }

        return Err(invalid_private_tree(
            "Private directory trees require ordinary local files and directories",
        ));
    }

    Ok(())
}

#[cfg(unix)]
fn visit_directory(
    dirfd: BorrowedFd<'_>,
    mode: TreeWalkMode,
    state: &mut TreeWalkState,
) -> io::Result<()> {
    let metadata = fstat(dirfd).map_err(io::Error::from)?;
    if !FileType::from_raw_mode(metadata.st_mode).is_dir() {
        return Err(invalid_private_tree(
            "Private directory tree roots must be ordinary directories",
        ));
    }
    record_or_apply_mode(
        dirfd,
        u32::from(metadata.st_mode),
        PRIVATE_DIR_MODE_BITS,
        PRIVATE_DIR_MODE,
        mode,
        state,
    )
}

#[cfg(unix)]
fn visit_file(
    filefd: BorrowedFd<'_>,
    mode: TreeWalkMode,
    state: &mut TreeWalkState,
) -> io::Result<()> {
    let metadata = fstat(filefd).map_err(io::Error::from)?;
    if !FileType::from_raw_mode(metadata.st_mode).is_file() || metadata.st_nlink != 1 {
        return Err(invalid_private_tree(
            "Private directory trees require ordinary local files",
        ));
    }
    record_or_apply_mode(
        filefd,
        u32::from(metadata.st_mode),
        PRIVATE_FILE_MODE_BITS,
        private_file_mode(),
        mode,
        state,
    )
}

#[cfg(unix)]
fn private_file_mode() -> Mode {
    Mode::RUSR | Mode::WUSR
}

#[cfg(unix)]
fn record_or_apply_mode(
    fd: BorrowedFd<'_>,
    actual_mode: u32,
    expected_bits: u32,
    expected_mode: Mode,
    mode: TreeWalkMode,
    state: &mut TreeWalkState,
) -> io::Result<()> {
    if actual_mode & 0o777 == expected_bits {
        return Ok(());
    }

    match mode {
        TreeWalkMode::Inspect => {
            state.needs_repair = true;
            Ok(())
        }
        TreeWalkMode::Repair => fchmod(fd, expected_mode).map_err(io::Error::from),
    }
}

#[cfg(unix)]
fn open_or_create_private_root(path: &Path) -> io::Result<OwnedFd> {
    walk_root_path(path, true)
}

#[cfg(unix)]
fn open_private_root(path: &Path) -> io::Result<OwnedFd> {
    walk_root_path(path, false)
}

#[cfg(unix)]
fn walk_root_path(path: &Path, create: bool) -> io::Result<OwnedFd> {
    let base = if path.is_absolute() { "/" } else { "." };
    let mut current = openat(
        CWD,
        base,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(io::Error::from)?;
    let mut saw_component = false;
    for component in path.components() {
        let Component::Normal(name) = component else {
            if matches!(component, Component::RootDir | Component::CurDir) {
                continue;
            }
            return Err(invalid_private_tree(
                "Private directory tree roots require a direct local path",
            ));
        };
        saw_component = true;
        current = match open_directory_relative(current.as_fd(), name) {
            Ok(next) => next,
            Err(error) if create && error.kind() == io::ErrorKind::NotFound => {
                match mkdirat(current.as_fd(), name, PRIVATE_DIR_MODE) {
                    Ok(()) => {}
                    Err(error) if io::Error::from(error).kind() == io::ErrorKind::AlreadyExists => {
                    }
                    Err(error) => return Err(io::Error::from(error)),
                }
                let next = open_path_component(current.as_fd(), name)?;
                fchmod(&next, PRIVATE_DIR_MODE).map_err(io::Error::from)?;
                next
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Err(error),
            Err(_) => {
                return Err(invalid_private_tree(
                    "Private directory tree roots must be owned local directories, not links",
                ));
            }
        };
    }
    if !saw_component {
        return Err(invalid_private_tree(
            "Private directory tree roots cannot be the current or filesystem root",
        ));
    }
    Ok(current)
}

#[cfg(unix)]
fn open_path_component(dirfd: BorrowedFd<'_>, name: &std::ffi::OsStr) -> io::Result<OwnedFd> {
    open_directory_relative(dirfd, name).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            error
        } else {
            invalid_private_tree(
                "Private directory tree roots must be owned local directories, not links",
            )
        }
    })
}

#[cfg(unix)]
fn open_directory_relative(
    dirfd: BorrowedFd<'_>,
    name: impl rustix::path::Arg,
) -> io::Result<OwnedFd> {
    openat(
        dirfd,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(io::Error::from)
}

#[cfg(unix)]
fn open_file_relative(dirfd: BorrowedFd<'_>, name: &std::ffi::CStr) -> io::Result<OwnedFd> {
    openat(
        dirfd,
        name,
        OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(io::Error::from)
}

#[cfg(unix)]
fn validate_opened_entry(
    fd: BorrowedFd<'_>,
    expected_metadata: &rustix::fs::Stat,
    directory: bool,
) -> io::Result<()> {
    let metadata = fstat(fd).map_err(io::Error::from)?;
    let file_type = FileType::from_raw_mode(metadata.st_mode);
    if (metadata.st_dev, metadata.st_ino) != (expected_metadata.st_dev, expected_metadata.st_ino)
        || file_type.is_dir() != directory
        || file_type.is_file() == directory
        || (!directory && metadata.st_nlink != 1)
    {
        return Err(invalid_private_tree(
            "Private directory tree entries changed while permissions were checked",
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn is_dot_entry(name: &std::ffi::CStr) -> bool {
    matches!(name.to_bytes(), b"." | b"..")
}

#[cfg(unix)]
fn invalid_private_tree(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_replaces_existing_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("config.json");
        std::fs::write(&path, "{\"old\":true}").unwrap();

        write_file_atomic_private(&path, "{\"new\":true}").unwrap();

        assert_eq!(std::fs::read_to_string(path).unwrap(), "{\"new\":true}");
    }

    #[cfg(unix)]
    #[test]
    fn private_file_candidates_open_nonblocking_before_validation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let root = temp_dir.path().canonicalize().unwrap();
        std::fs::write(root.join("candidate"), b"candidate").unwrap();
        let root = open_private_root(&root).unwrap();

        let name = std::ffi::CStr::from_bytes_with_nul(b"candidate\0").unwrap();
        let candidate = open_file_relative(root.as_fd(), name).unwrap();

        let flags = rustix::fs::fcntl_getfl(&candidate).unwrap();
        assert!(flags.contains(OFlags::NONBLOCK));
        assert!(
            validate_opened_entry(candidate.as_fd(), &fstat(&candidate).unwrap(), false).is_ok()
        );
    }
}
