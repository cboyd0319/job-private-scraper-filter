use super::saved_match_debugger::{
    load_saved_match_context, map_saved_match_confirmation_error, SavedMatchContext,
};
use super::{
    confirm_military_transition_review, prepare_military_transition_review, FoundationError,
    MilitaryOccupationReviewDraft, MilitaryOccupationSuggestion, MilitaryTransitionWording,
};
use chrono::{DateTime, Duration, Utc};
use jobsentinel_domain::{
    v3_evaluation_inputs::MilitaryBranch,
    v3_foundation::{CaseFileEventInput, CaseFileEventKind, EventMetadata, EventOrigin},
    v3_manifests::PrivacyLabel,
    ResumeEvidenceCitation,
};
use jobsentinel_storage::{v3_foundation::SavedMatchEvidenceConfirmation, Database};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use uuid::Uuid;

const MAX_PENDING_MILITARY_TRANSITION_REVIEWS: usize = 20;
const PENDING_MILITARY_TRANSITION_REVIEW_LIFETIME: Duration = Duration::minutes(30);

/// Closed user-confirmation choices for military evidence absent from match debugging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SavedMatchMilitaryEvidenceKind {
    MilitaryService,
    CurrentClearance,
}

/// Renderer-safe civilian wording confirmed from one consumed local review session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SavedMatchMilitaryTransitionConfirmation {
    civilian_role: String,
    civilian_responsibilities: Vec<String>,
    credential_wording: Vec<String>,
    user_confirmed_current_clearance: Option<String>,
    boundary: &'static str,
    clearance_currentness: &'static str,
    military_civilian_equivalence: &'static str,
}

impl SavedMatchMilitaryTransitionConfirmation {
    #[must_use]
    pub fn civilian_role(&self) -> &str {
        &self.civilian_role
    }

    #[must_use]
    pub fn civilian_responsibilities(&self) -> &[String] {
        &self.civilian_responsibilities
    }

    #[must_use]
    pub fn credential_wording(&self) -> &[String] {
        &self.credential_wording
    }

    #[must_use]
    pub fn user_confirmed_current_clearance(&self) -> Option<&str> {
        self.user_confirmed_current_clearance.as_deref()
    }
}

/// Move-only review prepared from one current saved match and its confirmed resume evidence.
pub(super) struct SavedMatchMilitaryTransitionReview {
    job_hash: String,
    resume_id: i64,
    review: MilitaryOccupationReviewDraft,
}

impl SavedMatchMilitaryTransitionReview {
    pub(super) fn review_id(&self) -> &str {
        self.review.review_id()
    }

    fn same_saved_match(&self, other: &Self) -> bool {
        self.job_hash == other.job_hash && self.resume_id == other.resume_id
    }
}

/// Explicitly confirm the canonical military or clearance field for one saved match.
pub async fn confirm_saved_match_military_evidence(
    database: &Database,
    job_hash: &str,
    resume_id: i64,
    kind: SavedMatchMilitaryEvidenceKind,
) -> Result<bool, FoundationError> {
    let context = load_saved_match_context(database, job_hash, resume_id).await?;
    let field_path = evidence_field_path(kind);
    let citation = ResumeEvidenceCitation::for_field(context.snapshot(), field_path)
        .ok_or(FoundationError::Conflict)?;
    let current = load_saved_match_context(database, job_hash, resume_id).await?;
    if !context.matches(&current) {
        return Err(FoundationError::Conflict);
    }
    database
        .confirm_saved_match_evidence(&SavedMatchEvidenceConfirmation {
            job_hash: current.job_hash().to_string(),
            resume_id: current.resume_id(),
            saved_match_id: current.saved_match_id(),
            expected_job_revision: current.job_revision().to_string(),
            expected_resume_snapshot: current.snapshot().clone(),
            expected_skills: current.skills().to_vec(),
            citation,
        })
        .await
        .map_err(map_saved_match_confirmation_error)
}

/// Prepare a move-only review from current saved-match context and already confirmed evidence.
pub(super) async fn prepare_saved_match_military_transition_review(
    database: &Database,
    job_hash: &str,
    resume_id: i64,
    branch: MilitaryBranch,
    wording: MilitaryTransitionWording,
) -> Result<SavedMatchMilitaryTransitionReview, FoundationError> {
    let context = load_saved_match_context(database, job_hash, resume_id).await?;
    let (case_file_id, confirmed_evidence_ids) = confirmed_context(database, &context).await?;
    let citation = required_citation(&context, &confirmed_evidence_ids, "military_info")?;
    let clearance_citation = wording
        .current_clearance
        .as_ref()
        .map(|_| required_citation(&context, &confirmed_evidence_ids, "clearance"))
        .transpose()?;
    let review = prepare_military_transition_review(
        database,
        &case_file_id,
        branch,
        wording,
        context.snapshot(),
        &citation,
        clearance_citation.as_ref(),
        Utc::now().date_naive(),
    )
    .await?;
    Ok(SavedMatchMilitaryTransitionReview {
        job_hash: context.job_hash().to_string(),
        resume_id: context.resume_id(),
        review,
    })
}

pub(super) async fn confirm_saved_match_military_transition_review(
    database: &Database,
    review: SavedMatchMilitaryTransitionReview,
    expected_review_id: &str,
) -> Result<MilitaryOccupationSuggestion, FoundationError> {
    if review.review_id() != expected_review_id {
        return Err(FoundationError::Conflict);
    }
    let receipt = CaseFileEventInput {
        case_file_id: review.review.case_file_id().to_string(),
        kind: CaseFileEventKind::PrivacyReceiptRecorded,
        origin: EventOrigin::User,
        user_action: true,
        privacy_labels: [PrivacyLabel::LocalOnly, PrivacyLabel::Sensitive],
        metadata: EventMetadata::LocalReference {
            reference_id: expected_review_id.to_string(),
        },
    };
    confirm_military_transition_review(database, review.review, &receipt).await
}

fn evidence_field_path(kind: SavedMatchMilitaryEvidenceKind) -> &'static str {
    match kind {
        SavedMatchMilitaryEvidenceKind::MilitaryService => "military_info",
        SavedMatchMilitaryEvidenceKind::CurrentClearance => "clearance",
    }
}

async fn confirmed_context(
    database: &Database,
    context: &SavedMatchContext,
) -> Result<(String, Vec<String>), FoundationError> {
    let confirmed = database
        .read_saved_match_confirmed_evidence(context.job_hash(), context.resume_id())
        .await
        .map_err(map_saved_match_confirmation_error)?;
    let case_file_id = confirmed
        .case_file_id()
        .ok_or(FoundationError::Conflict)?
        .to_string();
    Ok((case_file_id, confirmed.evidence_ids().to_vec()))
}

fn required_citation(
    context: &SavedMatchContext,
    confirmed_evidence_ids: &[String],
    field_path: &str,
) -> Result<ResumeEvidenceCitation, FoundationError> {
    let citation = ResumeEvidenceCitation::for_field(context.snapshot(), field_path)
        .ok_or(FoundationError::Conflict)?;
    confirmed_evidence_ids
        .binary_search(&citation.evidence_id)
        .map(|_| citation)
        .map_err(|_| FoundationError::Conflict)
}

#[derive(Clone, Default)]
pub struct PendingMilitaryTransitionReviews {
    entries: Arc<RwLock<Vec<PendingMilitaryTransitionReview>>>,
}

struct PendingMilitaryTransitionReview {
    token: String,
    created_at: DateTime<Utc>,
    review: SavedMatchMilitaryTransitionReview,
}

impl PendingMilitaryTransitionReviews {
    pub(super) fn queue(
        &self,
        review: SavedMatchMilitaryTransitionReview,
        now: DateTime<Utc>,
    ) -> String {
        let mut entries = self.write_entries();
        retain_current(&mut entries, now);
        entries.retain(|entry| !entry.review.same_saved_match(&review));
        while entries.len() >= MAX_PENDING_MILITARY_TRANSITION_REVIEWS {
            entries.remove(0);
        }
        let token = Uuid::new_v4().to_string();
        entries.push(PendingMilitaryTransitionReview {
            token: token.clone(),
            created_at: now,
            review,
        });
        token
    }

    pub(super) fn take(
        &self,
        token: &str,
        now: DateTime<Utc>,
    ) -> Option<SavedMatchMilitaryTransitionReview> {
        let mut entries = self.write_entries();
        retain_current(&mut entries, now);
        entries
            .iter()
            .position(|entry| entry.token == token)
            .map(|index| entries.remove(index).review)
    }

    #[cfg(test)]
    pub(crate) fn current_count(&self, now: DateTime<Utc>) -> usize {
        let mut entries = self.write_entries();
        retain_current(&mut entries, now);
        entries.len()
    }

    #[cfg(test)]
    pub(crate) const fn capacity() -> usize {
        MAX_PENDING_MILITARY_TRANSITION_REVIEWS
    }

    fn write_entries(
        &self,
    ) -> std::sync::RwLockWriteGuard<'_, Vec<PendingMilitaryTransitionReview>> {
        match self.entries.write() {
            Ok(entries) => entries,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

fn retain_current(entries: &mut Vec<PendingMilitaryTransitionReview>, now: DateTime<Utc>) {
    entries.retain(|entry| now - entry.created_at < PENDING_MILITARY_TRANSITION_REVIEW_LIFETIME);
}

/// Store a current review and return its sole renderer-visible handle.
pub async fn prepare_pending_saved_match_military_transition_review(
    database: &Database,
    pending: &PendingMilitaryTransitionReviews,
    job_hash: &str,
    resume_id: i64,
    branch: MilitaryBranch,
    wording: MilitaryTransitionWording,
) -> Result<String, FoundationError> {
    let review = prepare_saved_match_military_transition_review(
        database, job_hash, resume_id, branch, wording,
    )
    .await?;
    Ok(pending.queue(review, Utc::now()))
}

/// Consume one opaque review handle and return only confirmed civilian wording.
pub async fn confirm_pending_saved_match_military_transition_review(
    database: &Database,
    pending: &PendingMilitaryTransitionReviews,
    token: &str,
) -> Result<SavedMatchMilitaryTransitionConfirmation, FoundationError> {
    let review = pending
        .take(token, Utc::now())
        .ok_or(FoundationError::Conflict)?;
    let review_id = review.review_id().to_string();
    let suggestion =
        confirm_saved_match_military_transition_review(database, review, &review_id).await?;
    Ok(SavedMatchMilitaryTransitionConfirmation {
        civilian_role: suggestion.civilian_role().to_string(),
        civilian_responsibilities: suggestion.civilian_responsibilities().to_vec(),
        credential_wording: suggestion.credential_wording().to_vec(),
        user_confirmed_current_clearance: suggestion
            .user_confirmed_current_clearance()
            .map(str::to_string),
        boundary: "suggestion_only",
        clearance_currentness: "not_verified",
        military_civilian_equivalence: "not_verified",
    })
}
