use crate::application::resume::{ResumeMatcher, MAX_RESUME_FILE_BYTES};
use crate::bootstrap::AppState;
use crate::desktop;
use crate::desktop::path_label_for_logging;
use crate::ipc::errors::user_friendly_error;
use crate::ipc::resume_file_names::safe_resume_file_stem;
use std::path::{Path, PathBuf};
use tauri::State;
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

const MAX_JSON_RESUME_IMPORT_BYTES: u64 = 5 * 1024 * 1024;
const MANAGED_RESUME_UPLOAD_DIR: &str = "resume-uploads";
const SUPPORTED_RESUME_UPLOAD_EXTENSIONS: &[&str] = &["pdf", "docx", "txt", "md", "html", "htm"];

/// Select, copy, and parse a local resume without exposing source paths to renderer IPC.
#[tauri::command]
pub(crate) async fn select_and_upload_resume(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<i64>, String> {
    let Some(file_path) = app
        .dialog()
        .file()
        .add_filter("Resume", SUPPORTED_RESUME_UPLOAD_EXTENSIONS)
        .blocking_pick_file()
    else {
        return Ok(None);
    };

    let source_path = file_path
        .into_path()
        .map_err(|_| "Could not read the selected resume file.".to_string())?;
    let name = selected_resume_name(&source_path, "Resume");
    let resume_id = import_selected_resume_from_path(name, &source_path, state).await?;

    Ok(Some(resume_id))
}

pub(crate) async fn import_selected_resume_from_path(
    name: String,
    source_path: &Path,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let managed_path = copy_selected_resume_to_managed_storage(source_path)?;
    match upload_resume_from_managed_path(name, managed_path.clone(), state).await {
        Ok(resume_id) => Ok(resume_id),
        Err(error) => {
            delete_managed_resume_upload_file(
                Some(managed_path.to_string_lossy().as_ref()),
                &managed_resume_upload_dir(),
            )
            .ok();
            Err(error)
        }
    }
}

async fn upload_resume_from_managed_path(
    name: String,
    file_path: PathBuf,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    tracing::info!(
        name_chars = name.chars().count(),
        file_path = %path_label_for_logging(&file_path),
        "Command: upload_resume"
    );

    let matcher = state.database.resume_matcher();
    matcher
        .upload_resume(&name, &file_path.to_string_lossy())
        .await
        .map_err(|e| user_friendly_error("Failed to upload resume", e))
}

/// Import resume from JSON Resume format
#[tauri::command]
pub(crate) async fn import_json_resume(
    name: String,
    json_string: String,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    tracing::info!(
        name_chars = name.chars().count(),
        json_chars = json_string.chars().count(),
        "Command: import_json_resume"
    );

    let matcher = state.database.resume_matcher();
    matcher
        .import_json_resume(name, &json_string)
        .await
        .map_err(|e| user_friendly_error("Failed to import JSON Resume", e))
}

/// Select and import a JSON Resume file without exposing source paths to renderer IPC.
#[tauri::command]
pub(crate) async fn select_and_import_json_resume(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<i64>, String> {
    let Some(file_path) = app
        .dialog()
        .file()
        .add_filter("Resume App Export", &["json"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };

    let path = file_path
        .into_path()
        .map_err(|_| "Could not read the selected resume file.".to_string())?;
    let name = selected_resume_name(&path, "Resume");
    let resume_id = import_json_resume_from_selected_path(name, &path, state).await?;

    Ok(Some(resume_id))
}

async fn import_json_resume_from_selected_path(
    name: String,
    path: &Path,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    tracing::info!(
        name_chars = name.chars().count(),
        file_path = %path_label_for_logging(path),
        "Command: import_json_resume_file"
    );

    if !has_json_extension(path) {
        return Err(
            "Choose a structured resume file exported from JobSentinel or another resume tool."
                .to_string(),
        );
    }

    let metadata = tokio::fs::metadata(path).await.map_err(|_| {
        "JobSentinel could not read that resume file. Choose another structured resume file or check that the file is still available.".to_string()
    })?;

    if !metadata.is_file() {
        return Err(
            "Choose a structured resume file exported from JobSentinel or another resume tool."
                .to_string(),
        );
    }

    if metadata.len() > MAX_JSON_RESUME_IMPORT_BYTES {
        return Err(
            "That resume file is too large for import. Choose a smaller structured resume file."
                .to_string(),
        );
    }

    let json_string = tokio::fs::read_to_string(path).await.map_err(|_| {
        "JobSentinel could not read that resume file. Choose another structured resume file or check that the file is still available.".to_string()
    })?;

    tracing::info!(
        name_chars = name.chars().count(),
        json_chars = json_string.chars().count(),
        "Command: import_json_resume_file loaded local file"
    );

    let matcher = state.database.resume_matcher();
    matcher
        .import_json_resume(name, &json_string)
        .await
        .map_err(|e| user_friendly_error("Failed to import structured resume", e))
}

pub(super) fn has_json_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
}

pub(super) fn supported_resume_extension(path: &Path) -> Option<String> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    SUPPORTED_RESUME_UPLOAD_EXTENSIONS
        .contains(&extension.as_str())
        .then_some(extension)
}

fn managed_resume_upload_dir() -> PathBuf {
    desktop::get_data_dir().join(MANAGED_RESUME_UPLOAD_DIR)
}

fn selected_resume_name(path: &Path, fallback: &str) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| fallback.to_string())
}

pub(super) fn safe_resume_upload_file_name(path: &Path) -> String {
    let safe_stem = safe_resume_file_stem(path, "resume");
    let extension = supported_resume_extension(path).unwrap_or_else(|| "resume".to_string());
    format!("{}--{safe_stem}.{extension}", Uuid::new_v4())
}

pub(super) fn validate_selected_resume(path: &Path) -> Result<(), String> {
    if supported_resume_extension(path).is_none() {
        return Err("Choose a PDF, DOCX, TXT, Markdown, or HTML resume file.".to_string());
    }

    let metadata = std::fs::symlink_metadata(path)
        .map_err(|_| "JobSentinel could not read that resume file.".to_string())?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("Choose a resume file, not a folder.".to_string());
    }

    if metadata.len() > MAX_RESUME_FILE_BYTES {
        return Err(
            "That resume file is too large for local review. Choose a file under 10 MB or export a smaller readable PDF, DOCX, TXT, Markdown, or HTML resume."
                .to_string(),
        );
    }

    Ok(())
}

fn copy_selected_resume_to_managed_storage(path: &Path) -> Result<PathBuf, String> {
    validate_selected_resume(path)?;
    let managed_dir = managed_resume_upload_dir();
    std::fs::create_dir_all(&managed_dir)
        .map_err(|_| "Could not prepare local resume storage.".to_string())?;
    let destination = managed_dir.join(safe_resume_upload_file_name(path));
    std::fs::copy(path, &destination)
        .map_err(|_| "Could not copy the selected resume file.".to_string())?;

    Ok(destination)
}

fn validate_managed_resume_upload_file_name(file_name: &str) -> bool {
    let Some((uuid_part, display_name)) = file_name.split_once("--") else {
        return false;
    };
    if Uuid::parse_str(uuid_part).is_err() || display_name.trim().is_empty() {
        return false;
    }
    if file_name.contains(['/', '\\', ':']) || file_name.contains("..") {
        return false;
    }

    supported_resume_extension(Path::new(display_name)).is_some()
}

pub(super) fn managed_resume_upload_cleanup_path(
    stored_path: Option<&str>,
    managed_dir: &Path,
) -> Option<PathBuf> {
    let stored_path = stored_path.map(str::trim).filter(|path| !path.is_empty())?;
    let stored_path = PathBuf::from(stored_path);
    let file_name = stored_path.file_name()?.to_str()?;
    if !validate_managed_resume_upload_file_name(file_name) {
        return None;
    }

    let canonical_dir = managed_dir.canonicalize().ok()?;
    let canonical_parent = stored_path.parent()?.canonicalize().ok()?;
    if canonical_parent != canonical_dir {
        return None;
    }

    let metadata = std::fs::symlink_metadata(&stored_path).ok()?;
    if metadata.is_file() || metadata.file_type().is_symlink() {
        Some(stored_path)
    } else {
        None
    }
}

fn delete_managed_resume_upload_file(
    stored_path: Option<&str>,
    managed_dir: &Path,
) -> Result<(), String> {
    let Some(path) = managed_resume_upload_cleanup_path(stored_path, managed_dir) else {
        return Ok(());
    };

    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(
            "Resume was removed, but JobSentinel could not remove its local file copy.".to_string(),
        ),
    }
}

/// Delete a resume
#[tauri::command]
pub(crate) async fn delete_resume(
    resume_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!("Command: delete_resume (id: {})", resume_id);

    let matcher = state.database.resume_matcher();
    delete_resume_with_file_cleanup(resume_id, &matcher, &managed_resume_upload_dir()).await
}

pub(super) async fn delete_resume_with_file_cleanup(
    resume_id: i64,
    matcher: &ResumeMatcher,
    managed_dir: &Path,
) -> Result<(), String> {
    let resume = matcher
        .get_resume(resume_id)
        .await
        .map_err(|e| user_friendly_error("Failed to delete resume", e))?;

    matcher
        .delete_resume(resume_id)
        .await
        .map_err(|e| user_friendly_error("Failed to delete resume", e))?;
    delete_managed_resume_upload_file(Some(&resume.file_path), managed_dir)
}
