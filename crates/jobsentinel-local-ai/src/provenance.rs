use crate::manifest::{load_model_manifest, model_lock_hash};
use anyhow::{Context, Result};
use jobsentinel_domain::{v3_contracts::SchemaId, v3_manifests::ModelProvenance};

pub fn default_resume_embedding_provenance() -> Result<ModelProvenance> {
    let manifest = load_model_manifest()?;
    let model = manifest
        .default_embedding()
        .context("default embedding model is absent from the checked-in model lock")?;
    let dimension = u32::try_from(
        model
            .dimension
            .context("default embedding model dimension is missing")?,
    )
    .context("default embedding model dimension is invalid")?;
    let tokenizer = model
        .file("tokenizer.json")
        .context("default embedding tokenizer is absent from the checked-in model lock")?;
    let provenance = ModelProvenance {
        schema: SchemaId::ModelProvenanceV1,
        model_id: model.id.clone(),
        revision: model.revision.clone(),
        backend: model.backend.clone(),
        dimension,
        tokenizer_sha256: tokenizer.sha256.clone(),
        manifest_sha256: model_lock_hash(),
        instruction_profile: "resume_requirement_v1".to_string(),
        pooling: model.pooling.clone(),
        normalization: model.normalization.clone(),
    };
    provenance.validate().map_err(anyhow::Error::msg)?;
    Ok(provenance)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_resume_embedding_provenance_is_lock_owned() {
        let provenance = default_resume_embedding_provenance().unwrap();
        let manifest = load_model_manifest().unwrap();
        let model = manifest.default_embedding().unwrap();

        assert_eq!(provenance.model_id, model.id);
        assert_eq!(provenance.revision, model.revision);
        assert_eq!(provenance.backend, model.backend);
        assert_eq!(provenance.dimension, model.dimension.unwrap() as u32);
        assert_eq!(provenance.manifest_sha256, model_lock_hash());
        assert_eq!(provenance.instruction_profile, "resume_requirement_v1");
    }
}
