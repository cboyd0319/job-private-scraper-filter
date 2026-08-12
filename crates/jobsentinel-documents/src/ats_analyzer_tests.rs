use super::*;
use std::collections::HashMap;

#[path = "ats_analyzer_tests/bullet_improvements.rs"]
mod bullet_improvements;
#[path = "ats_analyzer_tests/business_requirement_equivalences.rs"]
mod business_requirement_equivalences;
#[path = "ats_analyzer_tests/credential_requirement_equivalences.rs"]
mod credential_requirement_equivalences;
#[path = "ats_analyzer_tests/credential_summary_tests.rs"]
mod credential_summary_tests;
#[path = "ats_analyzer_tests/current_experience_evidence.rs"]
mod current_experience_evidence;
#[path = "ats_analyzer_tests/degree_requirement_equivalences.rs"]
mod degree_requirement_equivalences;
#[path = "ats_analyzer_tests/evidence_citations.rs"]
mod evidence_citations;
#[path = "ats_analyzer_tests/experience_requirement_constraints.rs"]
mod experience_requirement_constraints;
#[path = "ats_analyzer_tests/format_safety_tests.rs"]
mod format_safety_tests;
#[path = "ats_analyzer_tests/screening_requirement_constraints.rs"]
mod screening_requirement_constraints;
#[path = "ats_analyzer_tests/service_healthcare_requirement_equivalences.rs"]
mod service_healthcare_requirement_equivalences;
#[path = "ats_analyzer_tests/work_arrangement_constraints.rs"]
mod work_arrangement_constraints;

fn skill_category(name: &str, category: &str) -> ResumeSkillCategory {
    ResumeSkillCategory {
        name: category.to_string(),
        skills: vec![ResumeSkill {
            name: name.to_string(),
            proficiency: None,
            years_experience: None,
        }],
    }
}

fn sample_resume() -> ResumeAnalysisInput {
    ResumeAnalysisInput {
        resume: StructuredResume {
            personal: ResumePersonalInfo {
                name: "Jordan Lee".to_string(),
                email: "jordan@example.com".to_string(),
                phone: Some("555-1234".to_string()),
                location: Some("Portland, OR".to_string()),
                linkedin: Some("linkedin.com/in/jordan-lee".to_string()),
                github: None,
                website: None,
            },
            summary: Some(
                "Program operations lead with 5 years of client services and intake scheduling"
                    .to_string(),
            ),
            experience: vec![ResumeExperience {
                title: "Program Operations Lead".to_string(),
                company: "Harbor Community Services".to_string(),
                location: Some("Portland, OR".to_string()),
                start_date: "Jan 2020".to_string(),
                end_date: None,
                achievements: vec![
                    "Led client intake scheduling across three service teams".to_string(),
                    "Improved case documentation accuracy by 40%".to_string(),
                ],
                is_current: true,
            }],
            skills: vec![
                skill_category("Case management", "Client Services"),
                skill_category("Scheduling", "Operations"),
                skill_category("CRM", "Tools"),
                skill_category("Reporting", "Operations"),
                skill_category("Compliance", "Quality"),
                skill_category("Excel", "Tools"),
            ],
            education: vec![ResumeEducation {
                degree: "BA Public Administration".to_string(),
                institution: "State University".to_string(),
                field_of_study: None,
                location: Some("Portland, OR".to_string()),
                graduation_date: Some("2018".to_string()),
                gpa: Some("3.8".to_string()),
                honors: vec![],
            }],
            certifications: vec![],
            projects: vec![],
            clearance: None,
            military_info: None,
        },
        custom_sections: HashMap::new(),
        evidence_snapshot: None,
    }
}

#[test]
fn test_extract_job_keywords() {
    // Use double newlines to separate sections properly
    let job_desc = r#"
Required: case management, scheduling, CRM

Nice to have: compliance, Excel
        "#;

    let keywords = AtsAnalyzer::extract_job_keywords(job_desc);

    // Case management should be extracted as Required.
    assert!(keywords
        .iter()
        .any(|(k, i)| k == "case management" && *i == KeywordImportance::Required));
    // Compliance should be extracted as Preferred from "nice to have".
    assert!(keywords
        .iter()
        .any(|(k, i)| k == "compliance" && *i == KeywordImportance::Preferred));
}

#[test]
fn test_extract_job_keywords_stops_required_at_preferred_heading() {
    let job_desc = r#"
Required:
- case management
- scheduling
Preferred:
- salesforce
- compliance
        "#;

    let keywords = AtsAnalyzer::extract_job_keywords(job_desc);

    assert!(keywords
        .iter()
        .any(|(k, i)| k == "case management" && *i == KeywordImportance::Required));
    assert!(keywords
        .iter()
        .any(|(k, i)| k == "salesforce" && *i == KeywordImportance::Preferred));
    assert!(!keywords
        .iter()
        .any(|(k, i)| k == "salesforce" && *i == KeywordImportance::Required));
}

#[test]
fn test_analyze_for_job_high_match() {
    let resume = sample_resume();
    let job_desc = "Required: case management, scheduling, CRM";

    let result = AtsAnalyzer::analyze_for_job(&resume, job_desc);

    assert!(result.keyword_score > 80.0);
    assert!(!result.keyword_matches.is_empty());
}

#[test]
fn test_analyze_for_job_low_match() {
    let resume = sample_resume();
    let job_desc = "Required: Java, Spring Boot, AWS Lambda";

    let result = AtsAnalyzer::analyze_for_job(&resume, job_desc);

    assert!(result.keyword_score < 50.0);
    assert!(!result.missing_keywords.is_empty());
    assert!(result
        .suggestions
        .iter()
        .any(|s| s.category == SuggestionCategory::AddKeyword));
}

#[test]
fn test_analyze_for_job_with_unrecognized_post_never_scores_perfect() {
    let resume = sample_resume();
    let job_desc = "We are hiring a dependable teammate for a busy office.";

    let result = AtsAnalyzer::analyze_for_job(&resume, job_desc);

    assert_eq!(result.keyword_score, 0.0);
    assert!(result.overall_score < 100.0);
    assert!(result.format_issues.iter().any(|issue| {
        issue.severity == IssueSeverity::Info
            && issue
                .issue
                .contains("Not enough job-post detail recognized")
    }));
}

#[test]
fn test_analyze_text_for_job_uses_saved_resume_text_without_structured_json() {
    let resume_text = "Jordan led client intake scheduling and case documentation.";
    let skills = vec!["CRM".to_string()];
    let job_desc = "Required: case management, scheduling, CRM";

    let result = AtsAnalyzer::analyze_text_for_job(resume_text, &skills, job_desc);

    assert!(result
        .keyword_matches
        .iter()
        .any(|matched| matched.keyword == "scheduling"
            && matched.found_in.contains(&"resume text".to_string())));
    assert!(result.keyword_matches.iter().any(
        |matched| matched.keyword == "crm" && matched.found_in.contains(&"skills".to_string())
    ));
    assert!(result.missing_keyword_details.iter().any(|gap| {
        gap.keyword == "case management" && gap.importance == KeywordImportance::Required
    }));
}

#[test]
fn test_missing_keywords_keep_job_importance() {
    let resume = sample_resume();
    let job_desc = r#"
Required: Java

Preferred: Salesforce
        "#;

    let result = AtsAnalyzer::analyze_for_job(&resume, job_desc);

    let has_required_gap = result
        .missing_keyword_details
        .iter()
        .any(|gap| gap.keyword == "java" && gap.importance == KeywordImportance::Required);
    let has_preferred_gap = result
        .missing_keyword_details
        .iter()
        .any(|gap| gap.keyword == "salesforce" && gap.importance == KeywordImportance::Preferred);

    assert!(has_required_gap);
    assert!(has_preferred_gap);
}

#[test]
fn test_keyword_importance_ordering() {
    let resume = sample_resume();
    let job_desc = r#"
            Required: case management
            Preferred: reporting
            Compliance is also good
        "#;

    let result = AtsAnalyzer::analyze_for_job(&resume, job_desc);

    // Required keywords should come first
    if result.keyword_matches.len() > 1 {
        assert_eq!(
            result.keyword_matches[0].importance,
            KeywordImportance::Required
        );
    }
}

#[test]
fn test_format_issue_severity() {
    let mut resume = sample_resume();
    resume.resume.personal.email = String::new(); // Critical
    resume.resume.personal.phone = Some(String::new()); // Warning

    let result = AtsAnalyzer::analyze_format(&resume);

    let critical = result
        .format_issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Critical)
        .count();
    let warning = result
        .format_issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Warning)
        .count();

    assert!(critical > 0);
    assert!(warning > 0);
}

#[test]
fn test_completeness_score() {
    let resume = sample_resume();
    let result = AtsAnalyzer::analyze_format(&resume);

    // All sections filled
    assert_eq!(result.completeness_score, 100.0);

    // Remove some sections
    let mut incomplete = resume.clone();
    incomplete.resume.summary = Some(String::new());
    incomplete.resume.education.clear();

    let result2 = AtsAnalyzer::analyze_format(&incomplete);
    assert!(result2.completeness_score < 100.0);
}

#[test]
fn test_keyword_frequency_tracking() {
    let mut resume = sample_resume();
    resume.resume.summary =
        Some("Rust developer with Rust experience building Rust applications".to_string());

    let job_desc = "Required: Rust";
    let result = AtsAnalyzer::analyze_for_job(&resume, job_desc);

    let rust_match = result.keyword_matches.iter().find(|m| m.keyword == "rust");
    assert!(rust_match.is_some());
    assert!(rust_match.unwrap().frequency >= 3);
}

#[test]
fn test_keyword_matching_does_not_count_substrings_as_evidence() {
    let mut resume = sample_resume();
    resume.resume.summary = Some(
        "Customer success specialist with JavaScript dashboards and Salesforce reports".to_string(),
    );
    resume.resume.skills = vec![
        skill_category("JavaScript", "Tools"),
        skill_category("Salesforce", "Tools"),
    ];

    let result = AtsAnalyzer::analyze_for_job(&resume, "Required: Java, sales");

    assert!(!result
        .keyword_matches
        .iter()
        .any(|matched| matched.keyword == "java" || matched.keyword == "sales"));
    assert!(result
        .missing_keyword_details
        .iter()
        .any(|gap| gap.keyword == "java" && gap.importance == KeywordImportance::Required));
    assert!(result
        .missing_keyword_details
        .iter()
        .any(|gap| gap.keyword == "sales" && gap.importance == KeywordImportance::Required));
}

#[test]
fn test_long_bullet_points_detected() {
    let mut resume = sample_resume();
    resume.resume.experience[0].achievements = vec![
            "This is a very long bullet point that exceeds the recommended length for ATS systems and should be flagged as a formatting issue that needs to be addressed before submission".to_string(),
        ];

    let result = AtsAnalyzer::analyze_format(&resume);

    assert!(result
        .format_issues
        .iter()
        .any(|i| i.issue.contains("Long bullet point")));
}

#[test]
fn test_missing_start_date_detected() {
    let mut resume = sample_resume();
    resume.resume.experience[0].start_date = String::new();

    let result = AtsAnalyzer::analyze_format(&resume);

    assert!(result
        .format_issues
        .iter()
        .any(|i| i.issue.contains("Missing start date")));
}

#[test]
fn test_few_skills_warning() {
    let mut resume = sample_resume();
    resume.resume.skills = vec![
        skill_category("Rust", "Programming"),
        skill_category("Python", "Programming"),
    ];

    let result = AtsAnalyzer::analyze_format(&resume);

    assert!(result
        .format_issues
        .iter()
        .any(|i| i.issue.contains("Few skills")));
}

#[test]
fn test_suggestion_categories() {
    let resume = sample_resume();
    let job_desc = "Required: Java, Spring Boot, AWS";

    let result = AtsAnalyzer::analyze_for_job(&resume, job_desc);

    // Should have AddKeyword suggestions for missing required skills
    assert!(result
        .suggestions
        .iter()
        .any(|s| s.category == SuggestionCategory::AddKeyword));
}

#[test]
fn test_missing_keyword_suggestions_are_review_first() {
    let resume = sample_resume();
    let job_desc = "Required: Java";

    let result = AtsAnalyzer::analyze_for_job(&resume, job_desc);
    let suggestion = result
        .suggestions
        .iter()
        .find(|s| s.category == SuggestionCategory::AddKeyword)
        .expect("missing keyword suggestion");

    assert!(suggestion.suggestion.contains("Review whether"));
    assert!(suggestion.suggestion.contains("worth making visible"));
    assert!(suggestion.impact.contains("real evidence is visible"));
    assert!(!suggestion.suggestion.contains("Add '"));
    assert_ne!(suggestion.impact, "High");
}

#[test]
fn test_resume_format_suggestions_have_plain_impact_copy() {
    let mut resume = sample_resume();
    resume.resume.experience[0].achievements.clear();

    let result = AtsAnalyzer::analyze_format(&resume);
    let suggestion = result
        .suggestions
        .iter()
        .find(|s| s.category == SuggestionCategory::AddSection)
        .expect("add-section suggestion");

    assert!(suggestion.impact.contains("work evidence"));
    assert_ne!(suggestion.impact, "High");
}

#[test]
fn test_bullet_suggestions_are_review_first() {
    let mut resume = sample_resume();
    resume.resume.experience[0].achievements = vec!["Handled weekly client scheduling".to_string()];

    let result = AtsAnalyzer::analyze_format(&resume);
    let suggestion = result
        .suggestions
        .iter()
        .find(|s| s.category == SuggestionCategory::RewordBullet)
        .expect("bullet suggestion");

    assert!(suggestion.suggestion.contains("Review whether"));
    assert!(suggestion.suggestion.contains("clear action"));
    assert!(suggestion.impact.contains("easier to scan"));
    assert_ne!(suggestion.impact, "Medium");
}

#[test]
fn test_requirement_reviews_explain_direct_partial_and_missing_evidence() {
    let result = AtsAnalyzer::analyze_text_for_job(
        "Jordan Lee\njordan@example.com\n\nExperience\nLed client intake scheduling projects.",
        &["CRM".to_string()],
        "Required: scheduling, CRM\n\nPreferred: Salesforce",
    );

    let scheduling = result
        .requirement_reviews
        .iter()
        .find(|review| review.keyword == "scheduling")
        .expect("scheduling review");
    assert_eq!(scheduling.match_state, RequirementMatchState::Direct);
    assert!(scheduling
        .evidence_sections
        .contains(&"experience".to_string()));
    assert!(scheduling.recommendation.contains("visible evidence"));

    let crm = result
        .requirement_reviews
        .iter()
        .find(|review| review.keyword == "crm")
        .expect("crm review");
    assert_eq!(crm.match_state, RequirementMatchState::Partial);
    assert!(crm.evidence_sections.contains(&"skills".to_string()));
    assert!(crm.recommendation.contains("supporting evidence"));

    let salesforce = result
        .requirement_reviews
        .iter()
        .find(|review| review.keyword == "salesforce")
        .expect("salesforce review");
    assert_eq!(salesforce.match_state, RequirementMatchState::Missing);
    assert!(salesforce.recommendation.contains("Only add it if true"));
}

#[path = "ats_analyzer_tests/matching_profile_tests.rs"]
mod matching_profile_tests;
