use super::integrity::verify_model_file_checksum;
use super::status::MODEL_CACHE_METADATA_FILE;
use super::{
    ModelCacheMetadata, ModelManager, DEFAULT_EMBEDDING_MODEL_LOCK_MISSING,
    DEFAULT_RERANKER_MODEL_LOCK_MISSING, RUNTIME_MODEL_LOCK_MISSING,
};
use crate::manifest::{load_model_manifest, model_lock_hash, ModelManifest, ModelSpec};
use crate::MlError;
use anyhow::{Context, Result};
use chrono::Utc;
use futures_util::{Stream, StreamExt};
use hf_hub::{split_id, HFClientBuilder};
use jobsentinel_security::path_label_for_logging;
use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const MODEL_CACHE_PATH_UNSAFE: &str = "local model cache path is unsafe";

impl ModelManager {
    /// Download the legacy runtime embedding model from Hugging Face Hub.
    pub async fn download_model(&self) -> Result<PathBuf> {
        let manifest = load_model_manifest()?;
        let spec = manifest
            .legacy_runtime_embedding()
            .ok_or_else(|| MlError::ModelLoadFailed(RUNTIME_MODEL_LOCK_MISSING.to_string()))?;
        self.download_model_spec(&manifest, spec, None, 0, locked_bytes(spec))
            .await
    }

    pub async fn download_model_by_id(&self, model_id: &str) -> Result<PathBuf> {
        let manifest = load_model_manifest()?;
        let spec = manifest.model(model_id).ok_or_else(|| {
            MlError::ModelLoadFailed("requested model lock entry missing".to_string())
        })?;
        self.download_model_spec(&manifest, spec, None, 0, locked_bytes(spec))
            .await
    }

    pub async fn download_default_semantic_models(&self) -> Result<Vec<PathBuf>> {
        self.download_default_semantic_models_inner(None).await
    }

    pub async fn download_default_semantic_models_with_progress(
        &self,
        report: Arc<dyn Fn(u64, u64) + Send + Sync>,
    ) -> Result<Vec<PathBuf>> {
        self.download_default_semantic_models_inner(Some(report))
            .await
    }

    async fn download_default_semantic_models_inner(
        &self,
        report: Option<Arc<dyn Fn(u64, u64) + Send + Sync>>,
    ) -> Result<Vec<PathBuf>> {
        let manifest = load_model_manifest()?;
        let embedding = manifest.default_embedding().ok_or_else(|| {
            MlError::ModelLoadFailed(DEFAULT_EMBEDDING_MODEL_LOCK_MISSING.to_string())
        })?;
        let reranker = manifest.default_reranker().ok_or_else(|| {
            MlError::ModelLoadFailed(DEFAULT_RERANKER_MODEL_LOCK_MISSING.to_string())
        })?;
        let total_bytes = locked_bytes(embedding) + locked_bytes(reranker);
        if let Some(report) = &report {
            report(0, total_bytes);
        }

        let mut paths = Vec::with_capacity(2);
        let mut completed_bytes = 0;
        for spec in [embedding, reranker] {
            if self.is_model_downloaded_for(spec) {
                paths.push(self.model_cache_dir(spec));
            } else {
                paths.push(
                    self.download_model_spec(
                        &manifest,
                        spec,
                        report.as_ref(),
                        completed_bytes,
                        total_bytes,
                    )
                    .await?,
                );
            }
            completed_bytes += locked_bytes(spec);
            if let Some(report) = &report {
                report(completed_bytes.min(total_bytes), total_bytes);
            }
        }

        Ok(paths)
    }

    async fn download_model_spec(
        &self,
        manifest: &ModelManifest,
        spec: &ModelSpec,
        report: Option<&Arc<dyn Fn(u64, u64) + Send + Sync>>,
        completed_before_model: u64,
        total_bytes: u64,
    ) -> Result<PathBuf> {
        tracing::info!(
            model_id = spec.id,
            revision = spec.revision,
            "Downloading model from HuggingFace Hub"
        );

        if std::env::var_os("HF_HUB_DISABLE_IMPLICIT_TOKEN").is_none_or(|value| value.is_empty()) {
            return Err(MlError::DownloadFailed(
                "Anonymous model download policy is not initialized".to_string(),
            )
            .into());
        }
        let model_dir = self.prepare_model_cache(spec)?;
        let xet_cache = self.reset_xet_transfer_cache()?;
        // The Xet runtime reads HF_XET_CACHE as UTF-8, so a non-UTF8 app path
        // would silently escape the governed cache; require an exact UTF-8 match.
        let xet_cache_policy = xet_cache.to_str();
        if xet_cache_policy.is_none()
            || std::env::var("HF_XET_CACHE").ok().as_deref() != xet_cache_policy
        {
            self.clear_xet_transfer_cache()?;
            return Err(MlError::DownloadFailed(
                "App-scoped model transfer cache policy is not initialized".to_string(),
            )
            .into());
        }
        let download_cache = model_dir.join(".download");
        ensure_directory(&download_cache)?;
        let client = HFClientBuilder::new()
            .endpoint("https://huggingface.co")
            .user_agent(concat!("JobSentinel/", env!("CARGO_PKG_VERSION")))
            .cache_dir(&download_cache)
            .build()
            .map_err(|_error| {
                MlError::DownloadFailed("Failed to initialize model download client".to_string())
            })?;
        let (owner, name) = split_id(&spec.repo);
        let repo = client.model(owner, name);

        let downloaded_at = Utc::now().to_rfc3339();
        let mut completed_before_file = completed_before_model;
        for file in spec.required_files() {
            let expected_file_bytes = file.size_bytes.ok_or_else(|| {
                MlError::DownloadFailed(format!(
                    "Required model file has no locked size: {}",
                    file.path
                ))
            })?;
            let target_path = self.prepare_model_file_path(spec, file)?;
            if verify_model_file_checksum(&target_path, &file.sha256).is_ok() {
                completed_before_file += expected_file_bytes;
                if let Some(report) = report {
                    report(completed_before_file.min(total_bytes), total_bytes);
                }
                continue;
            }
            tracing::info!(file = file.path, "Downloading required model file");

            let (mut staged_path, mut staged_file, mut resume_bytes) =
                self.prepare_staging_file(spec, file)?;
            if resume_bytes == expected_file_bytes {
                drop(staged_file);
                if verify_model_file_checksum(&staged_path, &file.sha256).is_ok() {
                    self.promote_staged_file(&staged_path, &target_path)?;
                    completed_before_file += expected_file_bytes;
                    if let Some(report) = report {
                        report(completed_before_file.min(total_bytes), total_bytes);
                    }
                    continue;
                }
                self.clear_staging_files(spec, file)?;
                (staged_path, staged_file, resume_bytes) = self.prepare_staging_file(spec, file)?;
            }

            if let Some(report) = report {
                report(
                    completed_before_file
                        .saturating_add(resume_bytes)
                        .min(total_bytes),
                    total_bytes,
                );
            }
            let remaining_bytes = expected_file_bytes.saturating_sub(resume_bytes);
            let streamed = repo
                .download_file_stream()
                .filename(file.path.clone())
                .revision(spec.revision.clone())
                .maybe_range((resume_bytes > 0).then_some(resume_bytes..expected_file_bytes))
                .send()
                .await;
            let (content_length, stream) = streamed.map_err(|_error| {
                MlError::DownloadFailed(format!(
                    "Failed to stream required model file: {}",
                    file.path
                ))
            })?;
            if content_length.is_some_and(|length| length != remaining_bytes) {
                drop(stream);
                drop(staged_file);
                self.clear_staging_files(spec, file)?;
                return Err(MlError::DownloadFailed(format!(
                    "Model download declared a size outside the lock: {}",
                    file.path
                ))
                .into());
            }
            let written = write_download_stream(
                stream,
                staged_file,
                &staged_path,
                remaining_bytes,
                completed_before_file.saturating_add(resume_bytes),
                total_bytes,
                report,
            )
            .await?;
            if written != remaining_bytes {
                self.clear_staging_files(spec, file)?;
                return Err(MlError::DownloadFailed(format!(
                    "Model download did not match the locked size: {}",
                    file.path
                ))
                .into());
            }

            if let Err(error) = verify_model_file_checksum(&staged_path, &file.sha256) {
                self.clear_staging_files(spec, file)?;
                return Err(error.context(format!(
                    "Downloaded model file failed integrity check: {}",
                    file.path
                )));
            }
            self.promote_staged_file(&staged_path, &target_path)?;
            completed_before_file += expected_file_bytes;
            if let Some(report) = report {
                report(completed_before_file.min(total_bytes), total_bytes);
            }
        }

        self.verify_model_cache(spec)?;
        let verified_at = Utc::now().to_rfc3339();
        self.write_cache_metadata(&model_dir, manifest, spec, downloaded_at, verified_at)?;
        drop(repo);
        drop(client);
        self.clear_xet_transfer_cache()?;
        ensure_directory(&download_cache)?;
        std::fs::remove_dir_all(download_cache)
            .context("Failed to remove completed model download cache")?;

        tracing::info!(
            model_dir = %path_label_for_logging(&model_dir),
            "Model downloaded successfully"
        );
        Ok(model_dir)
    }

    fn prepare_model_cache(&self, spec: &ModelSpec) -> Result<PathBuf> {
        let model_root = self.cache_dir.join(&spec.id);
        let revision_root = model_root.join(&spec.revision);
        let cache_root = revision_root.join(model_lock_hash());
        for directory in [&self.cache_dir, &model_root, &revision_root, &cache_root] {
            ensure_directory(directory)?;
        }
        Ok(cache_root)
    }

    fn prepare_model_file_path(
        &self,
        spec: &ModelSpec,
        file: &crate::manifest::ModelFileSpec,
    ) -> Result<PathBuf> {
        let cache_root = self.prepare_model_cache(spec)?;
        prepare_relative_file(&cache_root, Path::new(&file.path))
    }

    fn prepare_staging_file(
        &self,
        spec: &ModelSpec,
        file: &crate::manifest::ModelFileSpec,
    ) -> Result<(PathBuf, File, u64)> {
        let expected_bytes = file
            .size_bytes
            .ok_or_else(|| anyhow::anyhow!("required model file has no locked size"))?;
        let paths = self.staging_file_paths(spec, file)?;
        let mut lengths = [0_u64; 2];
        for (index, path) in paths.iter().enumerate() {
            if let Ok(metadata) = std::fs::symlink_metadata(path) {
                if metadata.len() > expected_bytes {
                    std::fs::remove_file(path).context("Failed to reset model staging file")?;
                } else {
                    lengths[index] = metadata.len();
                }
            }
        }

        let source_index = usize::from(lengths[1] > lengths[0]);
        let destination_index = 1 - source_index;
        let resume_bytes = lengths[source_index];
        let destination = &paths[destination_index];
        if std::fs::symlink_metadata(destination).is_ok() {
            std::fs::remove_file(destination).context("Failed to reset model staging file")?;
        }
        let mut destination_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)
            .context("Failed to create model staging file")?;

        if resume_bytes > 0 {
            let source =
                File::open(&paths[source_index]).context("Failed to reopen model staging file")?;
            let copied = std::io::copy(&mut source.take(resume_bytes), &mut destination_file)
                .context("Failed to copy resumable model staging data")?;
            if copied != resume_bytes {
                anyhow::bail!("resumable model staging file changed during preparation");
            }
        }

        Ok((destination.clone(), destination_file, resume_bytes))
    }

    fn staging_file_paths(
        &self,
        spec: &ModelSpec,
        file: &crate::manifest::ModelFileSpec,
    ) -> Result<[PathBuf; 2]> {
        let staging_root = self.prepare_model_cache(spec)?.join(".download");
        ensure_directory(&staging_root)?;
        Ok([
            prepare_relative_file(&staging_root, Path::new(&format!("{}.partial", file.path)))?,
            prepare_relative_file(
                &staging_root,
                Path::new(&format!("{}.partial.next", file.path)),
            )?,
        ])
    }

    fn clear_staging_files(
        &self,
        spec: &ModelSpec,
        file: &crate::manifest::ModelFileSpec,
    ) -> Result<()> {
        for path in self.staging_file_paths(spec, file)? {
            if std::fs::symlink_metadata(&path).is_ok() {
                std::fs::remove_file(path).context("Failed to clear model staging file")?;
            }
        }
        Ok(())
    }

    fn promote_staged_file(&self, staged: &Path, target: &Path) -> Result<()> {
        ensure_regular_file_or_missing(staged)?;
        ensure_regular_file_or_missing(target)?;
        if std::fs::symlink_metadata(target).is_ok() {
            std::fs::remove_file(target).context("Failed to replace cached model file")?;
        }
        std::fs::rename(staged, target).context("Failed to activate verified model file")
    }

    fn write_cache_metadata(
        &self,
        model_dir: &Path,
        manifest: &ModelManifest,
        spec: &ModelSpec,
        downloaded_at: String,
        verified_at: String,
    ) -> Result<()> {
        let metadata = ModelCacheMetadata {
            manifest_version: manifest.manifest_version,
            manifest_hash: model_lock_hash(),
            model_id: spec.id.clone(),
            kind: format!("{:?}", spec.kind),
            repo: spec.repo.clone(),
            revision: spec.revision.clone(),
            source_url: spec.source_url.clone(),
            backend: spec.backend.clone(),
            license: spec.license.clone(),
            downloaded_at,
            verified_at,
        };
        let metadata_json = serde_json::to_vec_pretty(&metadata)
            .context("failed to serialize model cache metadata")?;
        let metadata_path = model_dir.join(MODEL_CACHE_METADATA_FILE);
        ensure_regular_file_or_missing(&metadata_path)?;
        std::fs::write(metadata_path, metadata_json)
            .context("failed to write model cache metadata")?;
        Ok(())
    }
}

fn locked_bytes(spec: &ModelSpec) -> u64 {
    spec.required_files()
        .filter_map(|file| file.size_bytes)
        .sum()
}

async fn write_download_stream<S, B, E>(
    mut stream: S,
    mut destination: File,
    destination_path: &Path,
    max_bytes: u64,
    completed_before: u64,
    total_bytes: u64,
    report: Option<&Arc<dyn Fn(u64, u64) + Send + Sync>>,
) -> Result<u64>
where
    S: Stream<Item = std::result::Result<B, E>> + Unpin,
    B: AsRef<[u8]>,
{
    let mut written = 0_u64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_error| {
            MlError::DownloadFailed("Model download stream failed".to_string())
        })?;
        let chunk_bytes = u64::try_from(chunk.as_ref().len()).map_err(|_| {
            MlError::DownloadFailed("Model download chunk is too large".to_string())
        })?;
        let Some(next_written) = written
            .checked_add(chunk_bytes)
            .filter(|written| *written <= max_bytes)
        else {
            drop(destination);
            let _ = std::fs::remove_file(destination_path);
            return Err(MlError::DownloadFailed(
                "Model download exceeded the locked size".to_string(),
            )
            .into());
        };
        destination
            .write_all(chunk.as_ref())
            .context("Failed to write model staging data")?;
        written = next_written;
        if let Some(report) = report {
            report(
                completed_before.saturating_add(written).min(total_bytes),
                total_bytes,
            );
        }
    }
    destination
        .flush()
        .context("Failed to flush model staging data")?;
    Ok(written)
}

fn prepare_relative_file(root: &Path, relative: &Path) -> Result<PathBuf> {
    let mut target = root.to_path_buf();
    let mut components = relative.components().peekable();
    while let Some(component) = components.next() {
        let std::path::Component::Normal(component) = component else {
            anyhow::bail!(MODEL_CACHE_PATH_UNSAFE);
        };
        target.push(component);
        if components.peek().is_some() {
            ensure_directory(&target)?;
        } else {
            ensure_regular_file_or_missing(&target)?;
        }
    }
    if target == root {
        anyhow::bail!(MODEL_CACHE_PATH_UNSAFE);
    }
    Ok(target)
}

fn ensure_directory(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == ErrorKind::NotFound => {
            if let Err(error) = std::fs::create_dir(path) {
                if error.kind() != ErrorKind::AlreadyExists {
                    return Err(error).context("Failed to create local model cache directory");
                }
            }
        }
        Err(_) => anyhow::bail!(MODEL_CACHE_PATH_UNSAFE),
        Ok(_) => {}
    }
    let metadata =
        std::fs::symlink_metadata(path).map_err(|_| anyhow::anyhow!(MODEL_CACHE_PATH_UNSAFE))?;
    if !metadata.file_type().is_dir() {
        anyhow::bail!(MODEL_CACHE_PATH_UNSAFE);
    }
    Ok(())
}

fn ensure_regular_file_or_missing(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        _ => anyhow::bail!(MODEL_CACHE_PATH_UNSAFE),
    }
}

#[cfg(test)]
mod tests;
