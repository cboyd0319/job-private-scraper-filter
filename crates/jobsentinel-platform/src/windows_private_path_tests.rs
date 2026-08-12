// Verifies Windows private-path rejection before any linked ancestor can redirect creation.

use super::*;

#[test]
fn relative_private_path_is_rejected() {
    let error = match open_or_create_directory(Path::new("relative-private")) {
        Ok(_) => panic!("relative private path unexpectedly opened"),
        Err(error) => error,
    };

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
}

#[test]
fn linked_ancestor_is_rejected_without_creating_outside() {
    use std::os::windows::fs::symlink_dir;

    let temp_dir = tempfile::tempdir().unwrap();
    let external = temp_dir.path().join("external");
    let linked_parent = temp_dir.path().join("linked-parent");
    std::fs::create_dir(&external).unwrap();
    match symlink_dir(&external, &linked_parent) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => return,
        Err(error) => panic!("failed to create test directory link: {error}"),
    }

    let error = match open_or_create_directory(&linked_parent.join("private")) {
        Ok(_) => panic!("linked ancestor unexpectedly opened"),
        Err(error) => error,
    };

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(!external.join("private").exists());
}

#[test]
fn held_private_directory_rejects_a_new_write_handle() {
    use std::os::windows::fs::OpenOptionsExt;

    let temp_dir = tempfile::tempdir().unwrap();
    let directory = temp_dir.path().join("private");
    std::fs::create_dir(&directory).unwrap();
    let _locked = open_or_create_directory(&directory).unwrap();
    let mut options = std::fs::OpenOptions::new();
    options
        .access_mode(windows_sys::Win32::Storage::FileSystem::FILE_WRITE_DATA)
        .share_mode(windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ)
        .custom_flags(
            windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS
                | windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT,
        );

    let error = options.open(&directory).unwrap_err();

    assert_eq!(
        error.raw_os_error(),
        Some(windows_sys::Win32::Foundation::ERROR_SHARING_VIOLATION as i32)
    );
}

#[test]
fn nested_private_directories_are_created_through_retained_parents() {
    let temp_dir = tempfile::tempdir().unwrap();
    let directory = temp_dir.path().join("publisher").join("pack");

    let locked = open_or_create_directory(&directory).unwrap();

    assert!(locked.object().metadata().unwrap().is_dir());
    assert!(directory.is_dir());
}
