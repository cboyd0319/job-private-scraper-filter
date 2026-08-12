use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Opaque reference to one exact field in a local resume snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResumeEvidenceCitation {
    pub evidence_id: String,
    pub source_revision: String,
    pub field_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResumeEvidenceSnapshot {
    pub source_id: String,
    pub revision: String,
}

impl ResumeEvidenceCitation {
    pub fn for_field(snapshot: &ResumeEvidenceSnapshot, field_path: &str) -> Option<Self> {
        if !valid_snapshot_value(&snapshot.source_id)
            || !valid_snapshot_value(&snapshot.revision)
            || !valid_field_path(field_path)
        {
            return None;
        }

        let source_revision = hex::encode(Sha256::digest(
            format!(
                "resume_snapshot_v1\0{}\0{}",
                snapshot.source_id, snapshot.revision
            )
            .as_bytes(),
        ));
        let evidence_id = hex::encode(Sha256::digest(
            format!("resume_evidence_v1\0{field_path}\0{source_revision}").as_bytes(),
        ));
        Some(Self {
            evidence_id,
            source_revision,
            field_path: field_path.to_string(),
        })
    }
}

fn valid_snapshot_value(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}

fn valid_field_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.split('.').all(|segment| {
            !segment.is_empty()
                && segment
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '_')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn citation_is_stable_opaque_and_revision_bound() {
        let first_snapshot = ResumeEvidenceSnapshot {
            source_id: "resume-1".to_string(),
            revision: "revision-1".to_string(),
        };
        let edited_snapshot = ResumeEvidenceSnapshot {
            source_id: "resume-1".to_string(),
            revision: "revision-2".to_string(),
        };
        let first =
            ResumeEvidenceCitation::for_field(&first_snapshot, "experience.0.achievements.0")
                .unwrap();
        let repeated =
            ResumeEvidenceCitation::for_field(&first_snapshot, "experience.0.achievements.0")
                .unwrap();
        let edited =
            ResumeEvidenceCitation::for_field(&edited_snapshot, "experience.0.achievements.0")
                .unwrap();

        assert_eq!(first, repeated);
        assert_ne!(first.source_revision, edited.source_revision);
        assert_ne!(first.evidence_id, edited.evidence_id);
        assert!(!serde_json::to_string(&first)
            .unwrap()
            .contains("revision-1"));
    }

    #[test]
    fn identical_fields_in_different_resumes_do_not_collide() {
        let first = ResumeEvidenceCitation::for_field(
            &ResumeEvidenceSnapshot {
                source_id: "resume-1".to_string(),
                revision: "revision-1".to_string(),
            },
            "summary",
        )
        .unwrap();
        let second = ResumeEvidenceCitation::for_field(
            &ResumeEvidenceSnapshot {
                source_id: "resume-2".to_string(),
                revision: "revision-1".to_string(),
            },
            "summary",
        )
        .unwrap();

        assert_ne!(first.source_revision, second.source_revision);
        assert_ne!(first.evidence_id, second.evidence_id);
    }

    #[test]
    fn citation_rejects_missing_snapshot_identity_and_filesystem_paths() {
        let missing = ResumeEvidenceSnapshot {
            source_id: String::new(),
            revision: String::new(),
        };
        let current = ResumeEvidenceSnapshot {
            source_id: "resume-1".to_string(),
            revision: "revision-1".to_string(),
        };

        assert!(ResumeEvidenceCitation::for_field(&missing, "summary").is_none());
        assert!(ResumeEvidenceCitation::for_field(&current, "/Users/name/resume").is_none());
        assert!(ResumeEvidenceCitation::for_field(&current, "experience..title").is_none());
    }
}
