// Walks absolute Windows private-storage paths through retained, handle-relative directories.

use crate::{
    windows_acl_policy::PrivateObjectKind,
    windows_private_files::apply_private_dacl,
    windows_private_native::{
        invalid_data, open_directory_for_acl_repair, open_relative, open_root, validate_object,
        TargetKind,
    },
};
use std::{
    ffi::OsStr,
    fs::File,
    io,
    path::{Path, PathBuf},
};
use windows_sys::{
    Wdk::Storage::FileSystem::{FILE_CREATE, FILE_OPEN},
    Win32::Storage::FileSystem::{
        FILE_ADD_FILE, FILE_ADD_SUBDIRECTORY, FILE_READ_ATTRIBUTES, FILE_TRAVERSE, READ_CONTROL,
        SYNCHRONIZE, WRITE_DAC,
    },
};

pub(crate) struct LockedPrivatePath {
    objects: Vec<File>,
    acl_indices: Vec<usize>,
    target_name: PathBuf,
}

impl LockedPrivatePath {
    pub(crate) fn object(&self) -> &File {
        self.objects.last().expect("validated private path")
    }

    pub(crate) fn acl_objects(&self) -> impl Iterator<Item = &File> {
        self.acl_indices.iter().map(|index| &self.objects[*index])
    }

    pub(crate) fn reopen_target_directory(&mut self) -> io::Result<File> {
        self.objects.pop().expect("validated private target");
        open_relative(
            self.objects.last().expect("validated private parent"),
            self.target_name.as_os_str(),
            TargetKind::Directory,
            directory_access(true, true),
            FILE_OPEN,
        )
    }

    pub(crate) fn retain_object(&mut self, object: File) {
        self.objects.clear();
        self.objects.push(object);
        self.acl_indices.clear();
    }
}

pub(crate) fn open_or_create_directory(path: &Path) -> io::Result<LockedPrivatePath> {
    open_path(path, TargetKind::Directory, true)
}

pub(crate) fn open_directory(path: &Path) -> io::Result<LockedPrivatePath> {
    open_path(path, TargetKind::Directory, false)
}

pub(crate) fn open_file(path: &Path) -> io::Result<LockedPrivatePath> {
    open_path(path, TargetKind::File, false)
}

pub(crate) fn open_child_directory(parent: &LockedPrivatePath, name: &str) -> io::Result<File> {
    open_valid_directory(parent, name, directory_access(true, true), FILE_OPEN)
}

pub(crate) fn create_child_directory(parent: &LockedPrivatePath, name: &str) -> io::Result<File> {
    open_valid_directory(parent, name, directory_access(true, true), FILE_CREATE)
}

pub(crate) fn open_child_directory_for_repair(
    parent: &LockedPrivatePath,
    name: &str,
) -> io::Result<File> {
    open_directory_for_acl_repair(parent.object(), OsStr::new(name))
}

fn open_valid_directory(
    parent: &LockedPrivatePath,
    name: &str,
    access: u32,
    disposition: u32,
) -> io::Result<File> {
    let object = open_relative(
        parent.object(),
        OsStr::new(name),
        TargetKind::Directory,
        access,
        disposition,
    )?;
    validate_object(&object, TargetKind::Directory)?;
    Ok(object)
}

fn open_path(path: &Path, target_kind: TargetKind, create: bool) -> io::Result<LockedPrivatePath> {
    validate_absolute_path(path)?;
    let mut paths = path.ancestors().collect::<Vec<_>>();
    paths.reverse();
    if paths.len() < 2 {
        return Err(invalid_data("private storage path has no owned component"));
    }

    let target_index = paths.len() - 1;
    let mut objects = Vec::with_capacity(paths.len());
    objects.push(open_root_with_fallback(paths[0])?);
    let mut acl_indices = Vec::new();
    for (index, component_path) in paths.into_iter().enumerate().skip(1) {
        let is_target = index == target_index;
        let mut requires_acl = is_target;
        let kind = if is_target {
            target_kind
        } else {
            TargetKind::Directory
        };
        let name = component_path
            .file_name()
            .ok_or_else(|| invalid_data("private storage component is missing"))?;
        let parent = objects.last().expect("private storage root");
        let object = match open_existing(parent, name, kind, requires_acl) {
            Ok(object) => object,
            Err(error)
                if create
                    && kind == TargetKind::Directory
                    && error.kind() == io::ErrorKind::NotFound =>
            {
                requires_acl = true;
                match open_relative(
                    parent,
                    name,
                    kind,
                    directory_access(true, true),
                    FILE_CREATE,
                ) {
                    Ok(object) => object,
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                        open_existing(parent, name, kind, requires_acl)?
                    }
                    Err(error) => return Err(error),
                }
            }
            Err(error) => return Err(error),
        };
        validate_object(&object, kind)?;
        objects.push(object);
        if requires_acl {
            acl_indices.push(index);
        }
    }
    Ok(LockedPrivatePath {
        objects,
        acl_indices,
        target_name: path
            .file_name()
            .expect("validated private target name")
            .into(),
    })
}

fn open_existing(
    parent: &File,
    name: &OsStr,
    kind: TargetKind,
    requires_acl: bool,
) -> io::Result<File> {
    let access = match kind {
        TargetKind::Directory => directory_access(requires_acl, true),
        TargetKind::File => {
            FILE_READ_ATTRIBUTES
                | SYNCHRONIZE
                | if requires_acl {
                    READ_CONTROL | WRITE_DAC
                } else {
                    0
                }
        }
    };
    match open_relative(parent, name, kind, access, FILE_OPEN) {
        Ok(object) => Ok(object),
        Err(error)
            if error.kind() == io::ErrorKind::PermissionDenied
                && requires_acl
                && kind == TargetKind::Directory =>
        {
            let repair = open_directory_for_acl_repair(parent, name)?;
            apply_private_dacl(&repair, PrivateObjectKind::Directory)?;
            drop(repair);
            open_relative(parent, name, kind, access, FILE_OPEN)
        }
        Err(error)
            if error.kind() == io::ErrorKind::PermissionDenied && kind == TargetKind::Directory =>
        {
            open_relative(
                parent,
                name,
                kind,
                directory_access(false, false),
                FILE_OPEN,
            )
        }
        Err(error) => Err(error),
    }
}

fn directory_access(requires_acl: bool, writable: bool) -> u32 {
    FILE_READ_ATTRIBUTES
        | FILE_TRAVERSE
        | SYNCHRONIZE
        | if requires_acl {
            READ_CONTROL | WRITE_DAC
        } else {
            0
        }
        | if writable {
            FILE_ADD_FILE | FILE_ADD_SUBDIRECTORY
        } else {
            0
        }
}

fn open_root_with_fallback(path: &Path) -> io::Result<File> {
    match open_root(path, directory_access(false, true)) {
        Ok(object) => Ok(object),
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            open_root(path, directory_access(false, false))
        }
        Err(error) => Err(error),
    }
}

fn validate_absolute_path(path: &Path) -> io::Result<()> {
    let supported_prefix = matches!(
        path.components().next(),
        Some(std::path::Component::Prefix(prefix))
            if matches!(
                prefix.kind(),
                std::path::Prefix::Disk(_)
                    | std::path::Prefix::VerbatimDisk(_)
                    | std::path::Prefix::UNC(_, _)
                    | std::path::Prefix::VerbatimUNC(_, _)
            )
    );
    if !path.is_absolute()
        || !supported_prefix
        || path.components().any(|component| {
            matches!(component, std::path::Component::CurDir | std::path::Component::ParentDir)
                || matches!(component, std::path::Component::Normal(value) if value == "." || value == "..")
        })
    {
        return Err(invalid_data("private storage path is not absolute and normalized"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "windows_private_path_tests.rs"]
mod tests;
