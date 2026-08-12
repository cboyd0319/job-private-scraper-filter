// Verifies native Windows ACL application and hard-link rejection for private app storage.

use super::*;
use crate::windows_private_files::{ensure_private_file, open_private_file};
use std::io::Read;

#[test]
fn private_acl_can_be_reapplied_on_windows() {
    let temp_dir = tempfile::tempdir().unwrap();
    let directory = temp_dir.path().join("private");
    let file = directory.join("artifact.pack");
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(&file, b"signed pack").unwrap();

    ensure_private_dir(&directory).unwrap();
    ensure_private_file(&file).unwrap();
    ensure_private_dir(&directory).unwrap();
    ensure_private_file(&file).unwrap();
}

#[test]
fn private_file_acl_rejects_hard_links_on_windows() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file = temp_dir.path().join("artifact.pack");
    std::fs::write(&file, b"signed pack").unwrap();
    std::fs::hard_link(&file, temp_dir.path().join("linked.pack")).unwrap();

    let error = ensure_private_file(&file).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn private_file_reopens_the_validated_object_for_reading() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file = temp_dir.path().join("artifact.pack");
    std::fs::write(&file, b"signed pack").unwrap();

    let mut opened = open_private_file(&file).unwrap().unwrap();
    let mut bytes = Vec::new();
    opened.read_to_end(&mut bytes).unwrap();

    assert_eq!(bytes, b"signed pack");
}

#[test]
fn private_directory_persists_reads_and_removes_a_child_by_handle() {
    let temp_dir = tempfile::tempdir().unwrap();
    let directory = open_or_create_private_dir(&temp_dir.path().join("private"))
        .unwrap()
        .open_or_create_child("publisher")
        .unwrap()
        .open_or_create_child("pack")
        .unwrap();

    assert!(directory
        .write_file_noclobber("artifact.pack", b"signed pack")
        .unwrap());
    assert!(!directory
        .write_file_noclobber("artifact.pack", b"different")
        .unwrap());
    assert_eq!(
        directory.read_file("artifact.pack", 64).unwrap(),
        b"signed pack"
    );
    directory.remove_file("artifact.pack").unwrap();
    assert_eq!(
        directory.read_file("artifact.pack", 64).unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );
}

#[test]
fn private_directory_rejects_alternate_stream_names() {
    let temp_dir = tempfile::tempdir().unwrap();
    let directory = open_or_create_private_dir(&temp_dir.path().join("private")).unwrap();

    let error = directory
        .write_file_noclobber("artifact.pack:stream", b"signed pack")
        .unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
}
