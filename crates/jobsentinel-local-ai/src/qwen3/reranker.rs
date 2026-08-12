use std::cmp::Ordering;
use std::path::PathBuf;

use anyhow::{Context, Result};
use candle_core::DType;
use tokenizers::Tokenizer;

use crate::manifest::ModelSpec;
use crate::model::ModelManager;
use crate::qwen3::config::{Qwen3Config, Qwen3LogitScoreConfig};
use crate::qwen3::model::Qwen3Model;
use crate::qwen3::pooling::last_token_pool;
use crate::qwen3::tokenization::{format_rerank_inputs, tokenize_rerank_inputs};
use crate::qwen3::{DEFAULT_RERANKER_MAX_PAIRS, QWEN3_RERANKER_CANDLE_BACKEND};
use crate::runtime::{
    RerankCandidate, RerankQuery, RerankScore, RerankerBackend, RuntimeCompatibility,
};
use crate::MlError;

pub struct Qwen3RerankerBackend {
    model: Qwen3Model,
    tokenizer: Tokenizer,
    compatibility: RuntimeCompatibility,
    max_tokens: usize,
    max_pairs: usize,
    true_token_id: usize,
    false_token_id: usize,
}

impl Qwen3RerankerBackend {
    pub fn new(app_data_dir: PathBuf) -> Result<Self> {
        let manager = ModelManager::new(app_data_dir);
        let spec = ModelManager::default_reranker_model_spec()?;
        Self::from_manager(&manager, spec)
    }

    pub(crate) fn from_manager(manager: &ModelManager, spec: ModelSpec) -> Result<Self> {
        if spec.backend != QWEN3_RERANKER_CANDLE_BACKEND {
            anyhow::bail!("Qwen3 reranker requires qwen3-reranker-candle model lock backend");
        }

        if !manager.is_model_downloaded_for(&spec) {
            return Err(MlError::ModelNotDownloaded(
                "Qwen3 reranker model is not downloaded".to_string(),
            )
            .into());
        }

        let device = ModelManager::get_device()?;
        let tokenizer = manager.load_tokenizer_for_spec(&spec)?;
        let config: Qwen3Config = manager.load_model_json_for_spec(&spec, "config.json")?;
        let logit_config: Qwen3LogitScoreConfig =
            manager.load_model_json_for_spec(&spec, "1_LogitScore/config.json")?;
        let vb = manager.load_model_for_spec(&spec, &device)?;
        let model = Qwen3Model::load(config, vb.pp("model"))
            .context("failed to load Qwen3 reranker model")?;

        let compatibility =
            RuntimeCompatibility::from_spec(&spec, QWEN3_RERANKER_CANDLE_BACKEND, None);
        compatibility.validate_for_model(&spec)?;

        Ok(Self {
            model,
            tokenizer,
            compatibility,
            max_tokens: spec.max_tokens,
            max_pairs: DEFAULT_RERANKER_MAX_PAIRS,
            true_token_id: logit_config.true_token_id,
            false_token_id: logit_config.false_token_id,
        })
    }

    fn score_candidates(
        &self,
        query: &RerankQuery,
        candidates: &[RerankCandidate],
    ) -> Result<Vec<RerankScore>> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        let bounded_candidates: Vec<&RerankCandidate> =
            candidates.iter().take(self.max_pairs).collect();
        let texts = format_rerank_inputs(query, &bounded_candidates);
        let batch = tokenize_rerank_inputs(
            &self.tokenizer,
            &texts,
            self.max_tokens,
            self.model.device(),
        )?;
        let hidden = self
            .model
            .forward(&batch.input_ids, &batch.attention_mask)
            .context("Qwen3 reranker inference failed")?;
        let pooled = last_token_pool(&hidden, &batch.attention_mask)?;
        let score_vector = self
            .model
            .token_embedding(self.true_token_id)?
            .sub(&self.model.token_embedding(self.false_token_id)?)?
            .reshape((self.model.hidden_size(), 1))?;
        let raw_scores = pooled
            .matmul(&score_vector)?
            .to_dtype(DType::F32)
            .context("failed to convert Qwen3 reranker scores to f32")?
            .to_vec2::<f32>()
            .context("failed to read Qwen3 reranker scores")?;

        let mut scores = build_rerank_scores(&bounded_candidates, &raw_scores)?;

        scores.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.candidate_id.cmp(&right.candidate_id))
        });
        for (index, score) in scores.iter_mut().enumerate() {
            score.rank = index + 1;
        }

        Ok(scores)
    }
}

fn build_rerank_scores(
    candidates: &[&RerankCandidate],
    raw_scores: &[Vec<f32>],
) -> Result<Vec<RerankScore>> {
    if candidates.len() != raw_scores.len() {
        anyhow::bail!("invalid Qwen3 reranker output");
    }
    candidates
        .iter()
        .zip(raw_scores)
        .map(|(candidate, row)| {
            if row.len() != 1 || !row[0].is_finite() {
                anyhow::bail!("invalid Qwen3 reranker output");
            }
            Ok(RerankScore {
                candidate_id: candidate.id.clone(),
                score: row[0],
                rank: 0,
            })
        })
        .collect()
}

impl RerankerBackend for Qwen3RerankerBackend {
    fn model_id(&self) -> &str {
        &self.compatibility.model_id
    }

    fn compatibility(&self) -> &RuntimeCompatibility {
        &self.compatibility
    }

    fn rerank(
        &self,
        query: &RerankQuery,
        candidates: &[RerankCandidate],
    ) -> Result<Vec<RerankScore>> {
        self.score_candidates(query, candidates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reranker_rows_require_exact_cardinality_one_finite_scalar_each() {
        let candidates = [
            RerankCandidate {
                id: "a".to_string(),
                text: "first".to_string(),
                metadata: serde_json::Value::Null,
            },
            RerankCandidate {
                id: "b".to_string(),
                text: "second".to_string(),
                metadata: serde_json::Value::Null,
            },
        ];
        let candidates = candidates.iter().collect::<Vec<_>>();

        assert!(build_rerank_scores(&candidates, &[vec![0.1]]).is_err());
        assert!(build_rerank_scores(&candidates, &[vec![0.1], vec![0.2], vec![0.3]]).is_err());
        assert!(build_rerank_scores(&candidates, &[vec![], vec![0.2]]).is_err());
        assert!(build_rerank_scores(&candidates, &[vec![0.1, 0.2], vec![0.3]]).is_err());
        assert!(build_rerank_scores(&candidates, &[vec![f32::NAN], vec![0.2]]).is_err());
        assert_eq!(
            build_rerank_scores(&candidates, &[vec![0.1], vec![0.2]])
                .unwrap()
                .len(),
            2
        );
    }
}
