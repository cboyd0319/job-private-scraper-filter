// Implements exact Windows handle opens, validation, and child-name policy for private storage.

#![allow(unsafe_code)]

use std::{
    ffi::OsStr,
    fs::{File, OpenOptions},
    io,
    mem::size_of,
    os::windows::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
        io::{AsRawHandle, FromRawHandle},
    },
    ptr::{null, null_mut},
};
use windows_sys::{
    Wdk::{
        Foundation::OBJECT_ATTRIBUTES,
        Storage::FileSystem::{
            NtCreateFile, FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE, FILE_OPEN,
            FILE_OPEN_REPARSE_POINT, FILE_SYNCHRONOUS_IO_NONALERT,
        },
    },
    Win32::{
        Foundation::{
            RtlNtStatusToDosError, HANDLE, OBJ_CASE_INSENSITIVE, OBJ_DONT_REPARSE,
            STATUS_REPARSE_POINT_ENCOUNTERED, UNICODE_STRING,
        },
        Storage::FileSystem::{
            GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_NORMAL,
            FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
            FILE_SHARE_READ, READ_CONTROL, WRITE_DAC,
        },
        System::IO::IO_STATUS_BLOCK,
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TargetKind {
    Directory,
    File,
}

const ACL_REPAIR_ACCESS: u32 = READ_CONTROL | WRITE_DAC;

pub(crate) fn open_root(path: &std::path::Path, access: u32) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options
        .access_mode(access)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
    let object = options.open(path)?;
    validate_object(&object, TargetKind::Directory)?;
    Ok(object)
}

pub(crate) fn open_relative(
    parent: &File,
    name: &OsStr,
    kind: TargetKind,
    access: u32,
    disposition: u32,
) -> io::Result<File> {
    open_relative_with_options(parent, name, kind, access, disposition, true)
}

pub(crate) fn open_directory_for_acl_repair(parent: &File, name: &OsStr) -> io::Result<File> {
    open_relative_with_options(
        parent,
        name,
        TargetKind::Directory,
        ACL_REPAIR_ACCESS,
        FILE_OPEN,
        false,
    )
}

fn open_relative_with_options(
    parent: &File,
    name: &OsStr,
    kind: TargetKind,
    access: u32,
    disposition: u32,
    synchronous: bool,
) -> io::Result<File> {
    let mut name = child_name(name)?;
    let unicode = UNICODE_STRING {
        Length: (name.len() * size_of::<u16>()) as u16,
        MaximumLength: (name.len() * size_of::<u16>()) as u16,
        Buffer: name.as_mut_ptr(),
    };
    let attributes = OBJECT_ATTRIBUTES {
        Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: object_handle(parent),
        ObjectName: &unicode,
        Attributes: OBJ_CASE_INSENSITIVE | OBJ_DONT_REPARSE,
        SecurityDescriptor: null(),
        SecurityQualityOfService: null(),
    };
    let options = open_options(kind, synchronous);
    let mut handle = null_mut();
    let mut status_block = IO_STATUS_BLOCK::default();
    // SAFETY: all inputs remain live through this synchronous call, RootDirectory is retained,
    // and handle/status_block are initialized out-pointers of the exact API types.
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            access,
            &attributes,
            &mut status_block,
            null(),
            FILE_ATTRIBUTE_NORMAL,
            FILE_SHARE_READ,
            disposition,
            options,
            null(),
            0,
        )
    };
    if status == STATUS_REPARSE_POINT_ENCOUNTERED {
        return Err(invalid_data("private storage object is a reparse point"));
    }
    if status < 0 {
        // SAFETY: RtlNtStatusToDosError accepts an NTSTATUS returned by NtCreateFile.
        let code = unsafe { RtlNtStatusToDosError(status) };
        return Err(io::Error::from_raw_os_error(code as i32));
    }
    if handle.is_null() {
        return Err(invalid_data(
            "Windows returned a missing private-storage handle",
        ));
    }
    // SAFETY: successful NtCreateFile returned one owned handle, transferred once to File.
    Ok(unsafe { File::from_raw_handle(handle) })
}

fn open_options(kind: TargetKind, synchronous: bool) -> u32 {
    FILE_OPEN_REPARSE_POINT
        | if synchronous {
            FILE_SYNCHRONOUS_IO_NONALERT
        } else {
            0
        }
        | match kind {
            TargetKind::Directory => FILE_DIRECTORY_FILE,
            TargetKind::File => FILE_NON_DIRECTORY_FILE,
        }
}

pub(crate) fn child_name(value: &OsStr) -> io::Result<Vec<u16>> {
    let path = std::path::Path::new(value);
    let mut components = path.components();
    let valid = matches!(components.next(), Some(std::path::Component::Normal(name)) if name != "." && name != "..")
        && components.next().is_none();
    let encoded = value.encode_wide().collect::<Vec<_>>();
    if !valid
        || encoded.is_empty()
        || encoded
            .iter()
            .any(|unit| *unit == 0 || *unit == u16::from(b':'))
        || encoded.len() > (u16::MAX as usize / size_of::<u16>())
    {
        return Err(invalid_data("private storage child name is invalid"));
    }
    Ok(encoded)
}

pub(crate) fn validate_object(object: &File, kind: TargetKind) -> io::Result<()> {
    let metadata = object.metadata()?;
    if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(invalid_data("private storage object is a reparse point"));
    }
    match kind {
        TargetKind::Directory if metadata.is_dir() => Ok(()),
        TargetKind::File if metadata.is_file() && has_single_link(object)? => Ok(()),
        TargetKind::Directory => Err(invalid_data("private storage directory is invalid")),
        TargetKind::File => Err(invalid_data("private storage file is invalid")),
    }
}

fn has_single_link(object: &File) -> io::Result<bool> {
    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: object owns a live file-system handle and information is the exact out-buffer type.
    if unsafe { GetFileInformationByHandle(object_handle(object), &mut information) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(information.nNumberOfLinks == 1)
}

pub(crate) fn object_handle(object: &File) -> HANDLE {
    object.as_raw_handle().cast::<core::ffi::c_void>() as HANDLE
}

pub(crate) fn invalid_data(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acl_repair_open_asks_only_for_security_descriptor_rights() {
        assert_eq!(ACL_REPAIR_ACCESS, READ_CONTROL | WRITE_DAC);
        assert_eq!(
            open_options(TargetKind::Directory, false) & FILE_SYNCHRONOUS_IO_NONALERT,
            0
        );
    }
}
