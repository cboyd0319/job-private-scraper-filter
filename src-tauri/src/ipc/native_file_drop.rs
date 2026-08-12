//! Owns bounded native file-drop staging and routes reviewed imports to typed owners.

use crate::bootstrap::{AppState, StartupRecoveryState};
use crate::ipc::import::stage_smart_paste_text;
use crate::ipc::recovery::{stage_portable_restore_from_path, PortableRestoreActionResult};
use crate::ipc::resume::resume_file_commands::import_selected_resume_from_path;
use crate::ipc::resume_file_names::safe_resume_file_stem;
use jobsentinel_application::{pack_runtime::MAX_PACK_ARTIFACT_BYTES, JobImportPreview};
use serde::Serialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use uuid::Uuid;
use zeroize::Zeroizing;

const DROP_ONE_REGULAR_FILE_ERROR: &str = "Drop one regular file.";
const DROPPED_FILE_UNAVAILABLE_ERROR: &str =
    "This dropped file is no longer available. Drop it again.";
const DROPPED_JOB_TEXT_ERROR: &str = "Drop a UTF-8 job text file within the Smart Paste limit.";
const MAX_DROPPED_JOB_TEXT_BYTES: u64 = 200_000;
const NATIVE_DROP_STAGING_DIR: &str = "native-file-drops";
const STAGED_FILE_PREFIX: &str = "native-drop-";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeFileDropPayload {
    pub drop_id: Option<String>,
    pub name: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug)]
struct AppOwnedStagedFile {
    path: PathBuf,
}

impl Drop for AppOwnedStagedFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StagedNativeFileDrop {
    file: Arc<AppOwnedStagedFile>,
    name: String,
}

impl StagedNativeFileDrop {
    fn path(&self) -> &Path {
        &self.file.path
    }
}

pub(crate) struct NativeFileDropState {
    staging_dir: PathBuf,
    reservation: AtomicU64,
    candidate: Mutex<Option<(String, StagedNativeFileDrop)>>,
}

impl Default for NativeFileDropState {
    fn default() -> Self {
        Self::with_staging_dir(crate::desktop::get_data_dir().join(NATIVE_DROP_STAGING_DIR))
    }
}

impl NativeFileDropState {
    pub(crate) fn with_staging_dir(staging_dir: PathBuf) -> Self {
        cleanup_stale_staged_files(&staging_dir);
        Self {
            staging_dir,
            reservation: AtomicU64::default(),
            candidate: Mutex::default(),
        }
    }

    #[cfg(test)]
    pub(crate) fn stage_paths(&self, paths: &[PathBuf]) -> Option<NativeFileDropPayload> {
        self.stage_reserved_paths(self.reserve_drop(), paths)
    }

    pub(crate) fn reserve_drop(&self) -> u64 {
        let reservation = self.reservation.fetch_add(1, Ordering::SeqCst) + 1;
        *self
            .candidate
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = None;
        reservation
    }

    pub(crate) fn stage_reserved_paths(
        &self,
        reservation: u64,
        paths: &[PathBuf],
    ) -> Option<NativeFileDropPayload> {
        if !self.is_current_reservation(reservation) {
            return None;
        }
        let Some(path) = paths.first().filter(|_| paths.len() == 1) else {
            return Some(invalid_drop_payload());
        };
        let drop_id = Uuid::new_v4().to_string();
        let name = sanitized_file_name(path);
        let Ok(Some(file)) = self.copy_source_to_staging(path, &drop_id, reservation) else {
            if !self.is_current_reservation(reservation) {
                return None;
            }
            return Some(invalid_drop_payload());
        };
        let mut candidate = self
            .candidate
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if !self.is_current_reservation(reservation) {
            return None;
        }
        *candidate = Some((
            drop_id.clone(),
            StagedNativeFileDrop {
                file: Arc::new(file),
                name: name.clone(),
            },
        ));
        Some(NativeFileDropPayload {
            drop_id: Some(drop_id),
            name: Some(name),
            error: None,
        })
    }

    pub(crate) fn current(&self, drop_id: &str) -> Result<StagedNativeFileDrop, String> {
        let candidate = self
            .candidate
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let Some((current_id, staged)) = candidate.as_ref() else {
            return Err(DROPPED_FILE_UNAVAILABLE_ERROR.to_string());
        };
        if current_id != drop_id.trim() {
            return Err(DROPPED_FILE_UNAVAILABLE_ERROR.to_string());
        }
        ensure_regular_file(staged.path())?;
        Ok(staged.clone())
    }

    pub(crate) fn discard(&self, drop_id: &str) -> Result<(), String> {
        let mut candidate = self
            .candidate
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let Some((current_id, _)) = candidate.as_ref() else {
            return Err(DROPPED_FILE_UNAVAILABLE_ERROR.to_string());
        };
        if current_id != drop_id.trim() {
            return Err(DROPPED_FILE_UNAVAILABLE_ERROR.to_string());
        }
        *candidate = None;
        Ok(())
    }

    pub(crate) fn discard_after_success(&self, drop_id: &str) {
        let _ = self.discard(drop_id);
    }

    fn is_current_reservation(&self, reservation: u64) -> bool {
        self.reservation.load(Ordering::SeqCst) == reservation
    }

    fn with_current_payload<T>(
        &self,
        reservation: u64,
        payload: &NativeFileDropPayload,
        publish: impl FnOnce() -> T,
    ) -> Option<T> {
        let candidate = self
            .candidate
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let is_current = self.is_current_reservation(reservation)
            && match payload.drop_id.as_deref() {
                Some(drop_id) => candidate
                    .as_ref()
                    .is_some_and(|(current_id, _)| current_id == drop_id),
                None => candidate.is_none(),
            };
        is_current.then(publish)
    }

    fn copy_source_to_staging(
        &self,
        source_path: &Path,
        drop_id: &str,
        reservation: u64,
    ) -> Result<Option<AppOwnedStagedFile>, String> {
        prepare_staging_dir(&self.staging_dir)?;
        if !self.is_current_reservation(reservation) {
            return Ok(None);
        }
        let mut source = open_regular_source(source_path)?;
        if has_pack_extension(source_path) {
            match source.metadata() {
                Ok(metadata) if metadata.len() <= MAX_PACK_ARTIFACT_BYTES as u64 => {}
                _ => return Err(DROPPED_FILE_UNAVAILABLE_ERROR.to_string()),
            }
        }

        if !self.is_current_reservation(reservation) {
            return Ok(None);
        }
        let destination = self
            .staging_dir
            .join(staged_file_name(drop_id, source_path));
        let mut staged = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
            .map_err(|_| DROPPED_FILE_UNAVAILABLE_ERROR.to_string())?;
        let copied = if has_pack_extension(source_path) {
            std::io::copy(
                &mut source.take(MAX_PACK_ARTIFACT_BYTES as u64 + 1),
                &mut staged,
            )
        } else {
            std::io::copy(&mut source, &mut staged)
        };
        if !matches!(
            copied,
            Ok(bytes)
                if !has_pack_extension(source_path)
                    || bytes <= MAX_PACK_ARTIFACT_BYTES as u64
        ) {
            drop(staged);
            let _ = std::fs::remove_file(&destination);
            return Err(DROPPED_FILE_UNAVAILABLE_ERROR.to_string());
        }
        Ok(Some(AppOwnedStagedFile { path: destination }))
    }
}

pub(crate) fn capture_native_file_drop<R: Runtime>(app: &AppHandle<R>, paths: &[PathBuf]) {
    let reservation = {
        let Some(state) = app.try_state::<NativeFileDropState>() else {
            return;
        };
        state.reserve_drop()
    };
    let app = app.clone();
    let paths = paths.to_vec();
    tauri::async_runtime::spawn_blocking(move || {
        let Some(state) = app.try_state::<NativeFileDropState>() else {
            return;
        };
        let Some(payload) = state.stage_reserved_paths(reservation, &paths) else {
            return;
        };
        let emitted = state.with_current_payload(reservation, &payload, || {
            app.emit("native-file-drop", payload.clone())
        });
        if emitted.is_some_and(|result| result.is_err()) {
            tracing::warn!("Could not emit native file drop status");
        }
    });
}

#[tauri::command]
pub(crate) fn discard_native_file_drop(
    drop_id: String,
    native_file_drop: State<'_, NativeFileDropState>,
) -> Result<(), String> {
    native_file_drop.discard(&drop_id)
}

#[tauri::command]
pub(crate) async fn import_dropped_resume(
    drop_id: String,
    state: State<'_, AppState>,
    native_file_drop: State<'_, NativeFileDropState>,
) -> Result<i64, String> {
    let staged = native_file_drop.current(&drop_id)?;
    let resume_name = staged.name.clone();
    let resume_path = staged.path().to_path_buf();
    let resume_id = import_selected_resume_from_path(resume_name, &resume_path, state).await?;
    drop(staged);
    native_file_drop.discard_after_success(&drop_id);
    Ok(resume_id)
}

#[tauri::command]
pub(crate) async fn preview_dropped_job(
    drop_id: String,
    title: Option<String>,
    company: Option<String>,
    job_url: Option<String>,
    location: Option<String>,
    state: State<'_, AppState>,
    native_file_drop: State<'_, NativeFileDropState>,
) -> Result<JobImportPreview, String> {
    let staged = native_file_drop.current(&drop_id)?;
    let text = read_bounded_job_text(staged.path())?;
    stage_smart_paste_text(&state, &text, title, company, job_url, location).await
}

#[tauri::command]
pub(crate) async fn stage_dropped_portable_restore(
    drop_id: String,
    passphrase: String,
    state: State<'_, AppState>,
    recovery: State<'_, StartupRecoveryState>,
    native_file_drop: State<'_, NativeFileDropState>,
) -> Result<PortableRestoreActionResult, String> {
    let staged = native_file_drop.current(&drop_id)?;
    let passphrase = Zeroizing::new(passphrase);
    let result =
        stage_portable_restore_from_path(&state, &recovery, staged.path(), &passphrase).await?;
    native_file_drop.discard_after_success(&drop_id);
    Ok(result)
}

fn invalid_drop_payload() -> NativeFileDropPayload {
    NativeFileDropPayload {
        drop_id: None,
        name: None,
        error: Some(DROP_ONE_REGULAR_FILE_ERROR.to_string()),
    }
}

fn ensure_regular_file(path: &Path) -> Result<(), String> {
    let metadata =
        std::fs::symlink_metadata(path).map_err(|_| DROPPED_FILE_UNAVAILABLE_ERROR.to_string())?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(DROPPED_FILE_UNAVAILABLE_ERROR.to_string());
    }
    Ok(())
}

fn open_regular_source(path: &Path) -> Result<std::fs::File, String> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    configure_no_follow(&mut options);
    let file = options
        .open(path)
        .map_err(|_| DROPPED_FILE_UNAVAILABLE_ERROR.to_string())?;
    let metadata = file
        .metadata()
        .map_err(|_| DROPPED_FILE_UNAVAILABLE_ERROR.to_string())?;
    if !metadata.is_file() || is_windows_reparse_point(&metadata) {
        return Err(DROPPED_FILE_UNAVAILABLE_ERROR.to_string());
    }
    Ok(file)
}

#[cfg(unix)]
fn configure_no_follow(options: &mut std::fs::OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;
    options.custom_flags(libc::O_NOFOLLOW);
}

#[cfg(windows)]
fn configure_no_follow(options: &mut std::fs::OpenOptions) {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
}

#[cfg(unix)]
fn is_windows_reparse_point(_: &std::fs::Metadata) -> bool {
    false
}

#[cfg(windows)]
fn is_windows_reparse_point(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

fn prepare_staging_dir(staging_dir: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(staging_dir) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            Err(DROPPED_FILE_UNAVAILABLE_ERROR.to_string())
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir_all(staging_dir)
                .map_err(|_| DROPPED_FILE_UNAVAILABLE_ERROR.to_string())?;
            let metadata = std::fs::symlink_metadata(staging_dir)
                .map_err(|_| DROPPED_FILE_UNAVAILABLE_ERROR.to_string())?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                Err(DROPPED_FILE_UNAVAILABLE_ERROR.to_string())
            } else {
                Ok(())
            }
        }
        Err(_) => Err(DROPPED_FILE_UNAVAILABLE_ERROR.to_string()),
    }
}

fn cleanup_stale_staged_files(staging_dir: &Path) {
    let Ok(metadata) = std::fs::symlink_metadata(staging_dir) else {
        return;
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return;
    }
    let Ok(entries) = std::fs::read_dir(staging_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            continue;
        };
        if !is_recognized_staged_file_name(file_name) {
            continue;
        }
        let path = entry.path();
        if std::fs::symlink_metadata(&path).is_ok_and(|entry| entry.is_file()) {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn staged_file_name(drop_id: &str, source_path: &Path) -> String {
    safe_extension(source_path).map_or_else(
        || format!("{STAGED_FILE_PREFIX}{drop_id}"),
        |extension| format!("{STAGED_FILE_PREFIX}{drop_id}.{extension}"),
    )
}

fn is_recognized_staged_file_name(file_name: &str) -> bool {
    let Some(token_and_extension) = file_name.strip_prefix(STAGED_FILE_PREFIX) else {
        return false;
    };
    let (token, extension) = token_and_extension
        .split_once('.')
        .map_or((token_and_extension, None), |(token, extension)| {
            (token, Some(extension))
        });
    Uuid::parse_str(token).is_ok()
        && extension.is_none_or(|extension| {
            !extension.is_empty()
                && extension.len() <= 10
                && extension
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
        })
}

fn sanitized_file_name(path: &Path) -> String {
    let stem = safe_resume_file_stem(path, "file");
    let extension = safe_extension(path);
    extension.map_or_else(|| stem.clone(), |extension| format!("{stem}.{extension}"))
}

fn safe_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.len() <= 10)
        .filter(|value| {
            value
                .chars()
                .all(|character| character.is_ascii_alphanumeric())
        })
        .map(str::to_ascii_lowercase)
}

pub(crate) fn read_bounded_job_text(path: &Path) -> Result<String, String> {
    ensure_regular_file(path)?;
    let metadata = std::fs::metadata(path).map_err(|_| DROPPED_JOB_TEXT_ERROR.to_string())?;
    if metadata.len() > MAX_DROPPED_JOB_TEXT_BYTES {
        return Err(DROPPED_JOB_TEXT_ERROR.to_string());
    }
    let bytes = std::fs::read(path).map_err(|_| DROPPED_JOB_TEXT_ERROR.to_string())?;
    if bytes.len() > MAX_DROPPED_JOB_TEXT_BYTES as usize {
        return Err(DROPPED_JOB_TEXT_ERROR.to_string());
    }
    let text = String::from_utf8(bytes).map_err(|_| DROPPED_JOB_TEXT_ERROR.to_string())?;
    if text.chars().count() > 50_000 {
        return Err(DROPPED_JOB_TEXT_ERROR.to_string());
    }
    Ok(text)
}

fn has_pack_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("jspack"))
}

pub(crate) mod pack;

#[cfg(test)]
#[path = "native_file_drop_tests.rs"]
mod tests;
