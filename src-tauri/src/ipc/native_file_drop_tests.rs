//! Proves native drop staging rejects path races and preserves bounded app-owned copies.

use super::{
    open_regular_source, pack::read_bounded_pack_artifact, read_bounded_job_text,
    NativeFileDropState,
};
use std::path::Path;

fn state_in(temp: &tempfile::TempDir) -> NativeFileDropState {
    NativeFileDropState::with_staging_dir(temp.path().join("native-file-drops"))
}

fn stage_file(state: &NativeFileDropState, path: &Path) -> String {
    state
        .stage_paths(&[path.to_path_buf()])
        .unwrap()
        .drop_id
        .unwrap()
}

#[test]
fn stages_exactly_one_regular_file_with_a_sanitized_payload() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("Care Coordinator résumé?.txt");
    std::fs::write(&source, "case notes").unwrap();
    let payload = state_in(&temp).stage_paths(&[source]).unwrap();
    let json = serde_json::to_string(&payload).unwrap();
    assert!(payload.drop_id.is_some());
    assert_eq!(payload.name.as_deref(), Some("Care-Coordinator-r-sum.txt"));
    assert_eq!(payload.error, None);
    assert!(json.contains("dropId"));
    assert!(json.contains("Care-Coordinator-r-sum.txt"));
    assert!(!json.contains("Care Coordinator"));
    assert!(!json.contains(temp.path().to_string_lossy().as_ref()));
}

#[test]
fn rejects_directories_and_multiple_paths_without_retaining_a_candidate() {
    let temp = tempfile::tempdir().unwrap();
    let first = temp.path().join("first.txt");
    let second = temp.path().join("second.txt");
    std::fs::write(&first, "one").unwrap();
    std::fs::write(&second, "two").unwrap();
    let state = state_in(&temp);
    for paths in [
        [temp.path().to_path_buf()].as_slice(),
        [first, second].as_slice(),
    ] {
        let payload = state.stage_paths(paths).unwrap();
        assert_eq!(payload.drop_id, None);
        assert_eq!(payload.name, None);
        assert_eq!(payload.error.as_deref(), Some("Drop one regular file."));
    }
    assert!(state.current("not-a-token").is_err());
}

#[cfg(unix)]
#[test]
fn rejects_symlinks_without_revealing_the_path() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.txt");
    let link = temp.path().join("private-link.txt");
    std::fs::write(&source, "case notes").unwrap();
    symlink(&source, &link).unwrap();
    let json = serde_json::to_string(&state_in(&temp).stage_paths(&[link]).unwrap()).unwrap();
    assert!(!json.contains("private-link"));
    assert!(!json.contains(temp.path().to_string_lossy().as_ref()));
}

#[cfg(unix)]
#[test]
fn source_open_does_not_follow_a_check_to_open_symlink_replacement() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.txt");
    let replacement = temp.path().join("replacement.txt");
    std::fs::write(&source, "expected").unwrap();
    std::fs::write(&replacement, "private replacement").unwrap();
    assert!(std::fs::symlink_metadata(&source).unwrap().is_file());
    std::fs::remove_file(&source).unwrap();
    symlink(&replacement, &source).unwrap();

    assert!(open_regular_source(&source).is_err());
}

#[test]
fn tokens_are_replaced_bound_and_cleanup_only_unshared_staged_copies() {
    let temp = tempfile::tempdir().unwrap();
    let first = temp.path().join("first.pdf");
    let second = temp.path().join("second.pdf");
    std::fs::write(&first, "one").unwrap();
    std::fs::write(&second, "two").unwrap();
    let state = state_in(&temp);
    let first_token = stage_file(&state, &first);
    let first_staged = state.current(&first_token).unwrap();
    let first_path = first_staged.path().to_path_buf();
    drop(first_staged);
    let second_token = stage_file(&state, &second);
    assert!(!first_path.exists());
    assert!(state.current(&first_token).is_err());
    assert!(state.discard(&first_token).is_err());
    let second_path = state.current(&second_token).unwrap().path().to_path_buf();
    state.discard(&second_token).unwrap();
    assert!(!second_path.exists());
    assert!(state.discard(&second_token).is_err());
}

#[test]
fn post_success_cleanup_does_not_replace_a_success_with_a_stale_token_error() {
    let temp = tempfile::tempdir().unwrap();
    let first = temp.path().join("first.txt");
    let second = temp.path().join("second.txt");
    std::fs::write(&first, "one").unwrap();
    std::fs::write(&second, "two").unwrap();
    let state = state_in(&temp);
    let first_token = stage_file(&state, &first);
    let second_token = stage_file(&state, &second);
    state.discard_after_success(&first_token);
    assert!(state.current(&second_token).is_ok());
}

#[test]
fn staged_copy_cannot_change_after_the_external_source_is_replaced() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("care-coordinator.txt");
    std::fs::write(&source, "original file").unwrap();
    let state = state_in(&temp);
    let token = stage_file(&state, &source);
    std::fs::write(&source, "replaced external file").unwrap();
    let staged = state.current(&token).unwrap();
    assert_eq!(std::fs::read(staged.path()).unwrap(), b"original file");
    assert_ne!(staged.path(), source);
}

#[test]
fn later_reservation_wins_when_its_copy_finishes_before_an_older_task_starts() {
    let temp = tempfile::tempdir().unwrap();
    let first = temp.path().join("first.txt");
    let second = temp.path().join("second.txt");
    std::fs::write(&first, "first").unwrap();
    std::fs::write(&second, "second").unwrap();
    let state = state_in(&temp);
    let first_reservation = state.reserve_drop();
    let second_reservation = state.reserve_drop();

    let second_payload = state
        .stage_reserved_paths(second_reservation, &[second])
        .unwrap();
    let second_token = second_payload.drop_id.clone().unwrap();

    assert!(state
        .stage_reserved_paths(first_reservation, &[first])
        .is_none());
    let mut published = false;
    assert!(state
        .with_current_payload(second_reservation, &second_payload, || published = true)
        .is_some());
    assert!(published);
    assert!(state
        .with_current_payload(first_reservation, &second_payload, || ())
        .is_none());
    assert_eq!(
        std::fs::read(state.current(&second_token).unwrap().path()).unwrap(),
        b"second"
    );
}

#[cfg(unix)]
#[test]
fn startup_cleanup_removes_only_recognized_regular_staged_files() {
    use std::os::unix::fs::symlink;
    use uuid::Uuid;
    let temp = tempfile::tempdir().unwrap();
    let staging = temp.path().join("native-file-drops");
    std::fs::create_dir(&staging).unwrap();
    let stale = staging.join(format!("native-drop-{}.pdf", Uuid::new_v4()));
    let target = temp.path().join("do-not-delete.txt");
    let stale_link = staging.join(format!("native-drop-{}.pdf", Uuid::new_v4()));
    std::fs::write(&stale, "stale").unwrap();
    std::fs::write(&target, "keep").unwrap();
    symlink(&target, &stale_link).unwrap();
    let _state = NativeFileDropState::with_staging_dir(staging);
    assert!(!stale.exists());
    assert!(stale_link.exists());
    assert_eq!(std::fs::read(&target).unwrap(), b"keep");
}

#[cfg(unix)]
#[test]
fn staging_directory_symlinks_are_not_followed() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("outside");
    let staging = temp.path().join("native-file-drops");
    let source = temp.path().join("resume.pdf");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(&source, "resume").unwrap();
    symlink(&target, &staging).unwrap();
    let payload = NativeFileDropState::with_staging_dir(staging)
        .stage_paths(&[source])
        .unwrap();
    assert_eq!(payload.error.as_deref(), Some("Drop one regular file."));
    assert!(std::fs::read_dir(target).unwrap().next().is_none());
}

#[test]
#[ignore = "manual local staging measurement"]
fn measures_a_64_mib_portable_backup_copy() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("portable.db");
    std::fs::write(&source, vec![0_u8; 64 * 1024 * 1024]).unwrap();
    let state = state_in(&temp);
    let started = std::time::Instant::now();
    let token = stage_file(&state, &source);
    eprintln!("64 MiB native drop staging: {:?}", started.elapsed());
    state.discard(&token).unwrap();
}

#[test]
fn job_text_requires_bounded_utf8_before_smart_paste_staging() {
    let temp = tempfile::tempdir().unwrap();
    let valid = temp.path().join("job.txt");
    let binary = temp.path().join("job.bin");
    let oversized = temp.path().join("large-job.txt");
    std::fs::write(&valid, "Care Coordinator\nExample Clinic").unwrap();
    std::fs::write(&binary, [0xff, 0xfe]).unwrap();
    std::fs::write(&oversized, "a".repeat(200_001)).unwrap();
    assert_eq!(
        read_bounded_job_text(&valid).unwrap(),
        "Care Coordinator\nExample Clinic"
    );
    assert!(read_bounded_job_text(&binary).is_err());
    assert!(read_bounded_job_text(&oversized).is_err());
}

#[test]
fn signed_pack_reads_are_bounded_before_verification() {
    let temp = tempfile::tempdir().unwrap();
    let valid = temp.path().join("valid.jspack");
    let oversized = temp.path().join("oversized.jspack");
    std::fs::write(&valid, b"{}\n").unwrap();
    std::fs::write(
        &oversized,
        vec![b' '; jobsentinel_application::pack_runtime::MAX_PACK_ARTIFACT_BYTES + 1],
    )
    .unwrap();

    assert_eq!(read_bounded_pack_artifact(&valid).unwrap(), b"{}\n");
    assert!(read_bounded_pack_artifact(&oversized).is_err());
}

#[test]
fn native_drop_commands_are_registered_without_a_filesystem_capability() {
    let registry = include_str!("registry.rs");
    let capability = include_str!("../../capabilities/default.json");
    for command in [
        "discard_native_file_drop",
        "import_dropped_resume",
        "preview_dropped_job",
        "stage_dropped_portable_restore",
        "stage_dropped_pack",
        "choose_and_stage_pack",
    ] {
        assert!(registry.contains(command));
    }
    assert!(!capability.contains("\"fs:"));
    assert!(!capability.contains("\"fs\""));
}
