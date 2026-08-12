use super::*;
use jobsentinel_domain::ResumeEvidenceSnapshot;

fn evidence_resume(bullet: &str, revision: &str) -> ResumeAnalysisInput {
    let mut resume = sample_resume();
    resume.evidence_snapshot = Some(ResumeEvidenceSnapshot {
        source_id: "resume-1".to_string(),
        revision: revision.to_string(),
    });
    resume.resume.summary = None;
    resume.resume.skills.clear();
    resume.resume.experience[0].achievements = vec![bullet.to_string()];
    resume.resume.clearance = Some("User-confirmed security clearance".to_string());
    resume.resume.military_info = Some("Army 25B with user-confirmed logistics duties".to_string());
    resume
}

#[test]
fn structured_requirements_cite_exact_stable_fields_without_raw_content() {
    let resume = evidence_resume("Coordinated client scheduling", "revision-1");
    let first = AtsAnalyzer::analyze_for_job(&resume, "Required: scheduling, security clearance");
    let second = AtsAnalyzer::analyze_for_job(&resume, "Required: scheduling, security clearance");
    let scheduling = first
        .requirement_reviews
        .iter()
        .find(|review| review.keyword == "scheduling")
        .expect("scheduling review");
    let repeated = second
        .requirement_reviews
        .iter()
        .find(|review| review.keyword == "scheduling")
        .expect("repeated scheduling review");

    assert_eq!(scheduling.evidence_citations, repeated.evidence_citations);
    assert_eq!(
        scheduling.evidence_citations[0].field_path,
        "experience.0.achievements.0"
    );
    assert!(!serde_json::to_string(&scheduling.evidence_citations)
        .unwrap()
        .contains("Coordinated client scheduling"));

    let clearance = first
        .requirement_reviews
        .iter()
        .find(|review| review.keyword == "security clearance")
        .expect("clearance review");
    assert_eq!(clearance.match_state, RequirementMatchState::Implied);
    assert_eq!(clearance.evidence_citations[0].field_path, "clearance");
}

#[test]
fn user_confirmed_military_evidence_is_cited_without_inventing_equivalence() {
    let result = AtsAnalyzer::analyze_for_job(
        &evidence_resume("Coordinated client scheduling", "revision-1"),
        "Required: logistics",
    );
    let military = result
        .requirement_reviews
        .iter()
        .find(|review| review.keyword == "logistics")
        .expect("military occupation review");

    assert_ne!(military.match_state, RequirementMatchState::Missing);
    assert_eq!(military.evidence_citations[0].field_path, "military_info");
}

#[test]
fn editing_cited_content_changes_its_revision_and_identity() {
    let first = AtsAnalyzer::analyze_for_job(
        &evidence_resume("Coordinated client scheduling", "revision-1"),
        "Required: scheduling",
    );
    let edited = AtsAnalyzer::analyze_for_job(
        &evidence_resume("Improved scheduling operations", "revision-2"),
        "Required: scheduling",
    );
    let first_citation = &first.requirement_reviews[0].evidence_citations[0];
    let edited_citation = &edited.requirement_reviews[0].evidence_citations[0];

    assert_ne!(
        first_citation.source_revision,
        edited_citation.source_revision
    );
    assert_ne!(first_citation.evidence_id, edited_citation.evidence_id);
}

#[test]
fn plain_text_and_skills_matches_have_local_citations() {
    let result = AtsAnalyzer::analyze_text_for_job_with_snapshot(
        "Jordan Lee\n\nExperience\nCoordinated client scheduling",
        &["CRM".to_string()],
        "Required: scheduling, CRM\n\nPreferred: Salesforce",
        &ResumeEvidenceSnapshot {
            source_id: "resume-2".to_string(),
            revision: "revision-1".to_string(),
        },
    );
    let scheduling = result
        .requirement_reviews
        .iter()
        .find(|review| review.keyword == "scheduling")
        .expect("scheduling review");
    let crm = result
        .requirement_reviews
        .iter()
        .find(|review| review.keyword == "crm")
        .expect("CRM review");
    let missing = result
        .requirement_reviews
        .iter()
        .find(|review| review.keyword == "salesforce")
        .expect("Salesforce review");

    assert_eq!(scheduling.evidence_citations[0].field_path, "resume_text.3");
    assert_eq!(crm.match_state, RequirementMatchState::Partial);
    assert_eq!(crm.evidence_citations[0].field_path, "skills.0");
    assert!(missing.evidence_citations.is_empty());
    assert!(result.requirement_reviews.iter().all(|review| {
        review.match_state == RequirementMatchState::Missing
            || !review.evidence_citations.is_empty()
    }));
}

#[test]
fn a_requirement_split_across_fields_is_not_matchable_evidence() {
    let mut resume = evidence_resume("Coordinated intake", "revision-1");
    resume.resume.experience[0].title = "Case".to_string();
    resume.resume.experience[0].company = "Management".to_string();

    let result = AtsAnalyzer::analyze_for_job(&resume, "Required: case management");
    let review = result
        .requirement_reviews
        .iter()
        .find(|review| review.keyword == "case management")
        .expect("case management review");

    assert_eq!(review.match_state, RequirementMatchState::Missing);
    assert!(review.evidence_citations.is_empty());
}
