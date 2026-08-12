// Creates, opens, renames, and deletes private Windows child files by retained directory handle.

#![allow(unsafe_code)]

use crate::{
    windows_private_native::{
        child_name, object_handle, open_relative, validate_object, TargetKind,
    },
    windows_private_path::LockedPrivatePath,
};
use std::{ffi::OsStr, fs::File, io, mem::size_of};
use windows_sys::{
    Wdk::Storage::FileSystem::{FILE_CREATE, FILE_OPEN},
    Win32::Storage::FileSystem::{
        FileDispositionInfo, FileRenameInfo, SetFileInformationByHandle, DELETE,
        FILE_DISPOSITION_INFO, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_INFO_BY_HANDLE_CLASS,
        FILE_RENAME_INFO, READ_CONTROL, SYNCHRONIZE, WRITE_DAC,
    },
};

pub(crate) fn create_child_file(directory: &LockedPrivatePath, name: &str) -> io::Result<File> {
    open_valid_file(
        directory,
        name,
        FILE_GENERIC_READ | FILE_GENERIC_WRITE | DELETE | READ_CONTROL | WRITE_DAC | SYNCHRONIZE,
        FILE_CREATE,
    )
}

pub(crate) fn open_child_file(directory: &LockedPrivatePath, name: &str) -> io::Result<File> {
    open_valid_file(
        directory,
        name,
        FILE_GENERIC_READ | DELETE | READ_CONTROL | WRITE_DAC | SYNCHRONIZE,
        FILE_OPEN,
    )
}

fn open_valid_file(
    directory: &LockedPrivatePath,
    name: &str,
    access: u32,
    disposition: u32,
) -> io::Result<File> {
    let file = open_relative(
        directory.object(),
        OsStr::new(name),
        TargetKind::File,
        access,
        disposition,
    )?;
    validate_object(&file, TargetKind::File)?;
    Ok(file)
}

pub(crate) fn rename_child_noclobber(
    directory: &LockedPrivatePath,
    child: &File,
    destination: &str,
) -> io::Result<()> {
    let name = child_name(OsStr::new(destination))?;
    let name_bytes = name.len() * size_of::<u16>();
    let byte_len = std::mem::offset_of!(FILE_RENAME_INFO, FileName) + name_bytes;
    let mut buffer = vec![0usize; byte_len.div_ceil(size_of::<usize>())];
    let info = buffer.as_mut_ptr().cast::<FILE_RENAME_INFO>();
    // SAFETY: the aligned buffer is large enough for the fixed header and complete UTF-16 name.
    unsafe {
        (*info).Anonymous.ReplaceIfExists = false;
        (*info).RootDirectory = object_handle(directory.object());
        (*info).FileNameLength = name_bytes as u32;
        std::ptr::copy_nonoverlapping(
            name.as_ptr(),
            std::ptr::addr_of_mut!((*info).FileName).cast::<u16>(),
            name.len(),
        );
    }
    set_file_information(child, FileRenameInfo, info.cast(), byte_len)
}

pub(crate) fn set_child_delete_on_close(child: &File, delete: bool) -> io::Result<()> {
    let info = FILE_DISPOSITION_INFO { DeleteFile: delete };
    set_file_information(
        child,
        FileDispositionInfo,
        std::ptr::addr_of!(info).cast(),
        size_of::<FILE_DISPOSITION_INFO>(),
    )
}

fn set_file_information(
    file: &File,
    class: FILE_INFO_BY_HANDLE_CLASS,
    information: *const core::ffi::c_void,
    byte_len: usize,
) -> io::Result<()> {
    // SAFETY: file is live and information points to byte_len initialized bytes for class.
    if unsafe {
        SetFileInformationByHandle(object_handle(file), class, information, byte_len as u32)
    } == 0
    {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
