use super::*;
use crate::application::automation::{
    AnswerSource, AnswerStatistics, AnswerSuggestion, ApplicationProfile, ModificationExample,
    ScreeningAnswer,
};

fn profile_with_resume_path(path: Option<String>) -> ApplicationProfile {
    ApplicationProfile {
        id: 1,
        full_name: "Jordan Lee".to_string(),
        email: "jordan@example.com".to_string(),
        phone: None,
        linkedin_url: None,
        github_url: None,
        portfolio_url: None,
        website_url: None,
        default_resume_id: None,
        resume_file_path: path,
        default_cover_letter_template: None,
        us_work_authorized: true,
        requires_sponsorship: false,
        max_applications_per_day: 10,
        require_manual_approval: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

#[test]
fn application_profile_response_redacts_resume_file_path() {
    let response = ApplicationProfileResponse::from(profile_with_resume_path(Some(
        "private/client-resume.pdf".to_string(),
    )));

    let json = serde_json::to_string(&response).unwrap();
    assert!(response.has_resume_file);
    assert_eq!(
        response.resume_file_name,
        Some("client-resume.pdf".to_string())
    );
    assert!(json.contains("client-resume.pdf"));
    assert!(!json.contains("private/client-resume.pdf"));
    assert!(!json.contains("resumeFilePath"));
}

#[test]
fn application_profile_response_omits_unused_backend_metadata() {
    let response = ApplicationProfileResponse::from(profile_with_resume_path(Some(
        "managed/application-resumes/7d9d16a1-2e5d-4b32-9eb2-bfbffb4ee871--jordan-resume.pdf"
            .to_string(),
    )));

    let json = serde_json::to_value(&response).unwrap();
    let object = json.as_object().unwrap();

    assert!(!object.contains_key("id"));
    assert!(!object.contains_key("defaultResumeId"));
    assert!(!object.contains_key("defaultCoverLetterTemplate"));
    assert!(!object.contains_key("createdAt"));
    assert!(!object.contains_key("updatedAt"));
}

#[test]
fn application_profile_response_handles_windows_resume_paths() {
    let response = ApplicationProfileResponse::from(profile_with_resume_path(Some(
        "Windows\\Desktop\\resume.docx".to_string(),
    )));

    let json = serde_json::to_string(&response).unwrap();
    assert_eq!(response.resume_file_name, Some("resume.docx".to_string()));
    assert!(!json.contains("Windows\\\\Desktop"));
}

fn screening_answer(pattern: &str, answer: &str) -> ScreeningAnswer {
    ScreeningAnswer {
        id: 1,
        question_pattern: pattern.to_string(),
        answer: answer.to_string(),
        answer_type: Some("select".to_string()),
        notes: Some("private note".to_string()),
        times_used: 0,
        times_modified: 0,
        confidence_score: 1.0,
        last_used_at: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

#[test]
fn application_preview_excludes_raw_user_controlled_answers() {
    let previews = application_screening_answer_previews(vec![
        screening_answer("protected veteran status", "Decline to answer"),
        screening_answer("Hispanic or Latino", "Decline to answer"),
        screening_answer("pronouns", "they/them"),
        screening_answer("work authorization", "Yes"),
    ]);

    let json = serde_json::to_string(&previews).unwrap();
    assert_eq!(previews.len(), 1);
    assert!(json.contains("work authorization"));
    assert!(json.contains("Yes"));
    assert!(!json.contains("protected veteran"));
    assert!(!json.contains("Hispanic"));
    assert!(!json.contains("pronouns"));
    assert!(!json.contains("they/them"));
    assert!(!json.contains("Decline to answer"));
    assert!(!json.contains("private note"));
}

fn valid_profile_input() -> ApplicationProfileInput {
    ApplicationProfileInput {
        full_name: "Jordan Lee".to_string(),
        email: "jordan@example.com".to_string(),
        phone: None,
        linkedin_url: None,
        github_url: None,
        portfolio_url: None,
        website_url: None,
        default_resume_id: None,
        resume_file_path: None,
        resume_file_token: None,
        clear_resume_file: None,
        default_cover_letter_template: None,
        us_work_authorized: true,
        requires_sponsorship: false,
        max_applications_per_day: 10,
        require_manual_approval: true,
    }
}

#[test]
fn application_profile_resume_input_rejects_renderer_file_paths() {
    let managed_dir = tempfile::tempdir().unwrap();
    let input = ApplicationProfileInput {
        resume_file_path: Some("private/resume.pdf".to_string()),
        ..valid_profile_input()
    };

    let err = prepare_application_profile_resume_input(input, managed_dir.path()).unwrap_err();

    assert!(err.contains("Choose"));
    assert!(err.contains("resume"));
}

#[test]
fn application_profile_resume_input_accepts_managed_tokens() {
    let managed_dir = tempfile::tempdir().unwrap();
    let token = "7d9d16a1-2e5d-4b32-9eb2-bfbffb4ee871--new-resume.docx";
    let managed_resume = managed_dir.path().join(token);
    std::fs::write(&managed_resume, b"resume").unwrap();
    let input = ApplicationProfileInput {
        resume_file_token: Some(token.to_string()),
        ..valid_profile_input()
    };

    let prepared = prepare_application_profile_resume_input(input, managed_dir.path()).unwrap();

    assert_eq!(
        prepared.resume_file_path,
        Some(managed_resume.to_string_lossy().to_string())
    );
    assert_eq!(prepared.resume_file_token, None);
}

#[test]
fn application_profile_resume_path_rejects_existing_unmanaged_paths() {
    let managed_dir = tempfile::tempdir().unwrap();
    let outside_dir = tempfile::tempdir().unwrap();
    let outside_resume = outside_dir.path().join("resume.pdf");
    std::fs::write(&outside_resume, b"resume").unwrap();

    let err = trusted_application_resume_path(
        Some(outside_resume.to_string_lossy().as_ref()),
        managed_dir.path(),
    )
    .unwrap_err();

    let err = err.to_ascii_lowercase();
    assert!(err.contains("choose"));
    assert!(err.contains("resume"));
}

#[test]
fn application_profile_response_shows_resume_name_without_token_prefix() {
    let response = ApplicationProfileResponse::from(profile_with_resume_path(Some(
        "managed/application-resumes/7d9d16a1-2e5d-4b32-9eb2-bfbffb4ee871--jordan-resume.pdf"
            .to_string(),
    )));

    let json = serde_json::to_string(&response).unwrap();

    assert_eq!(
        response.resume_file_name,
        Some("jordan-resume.pdf".to_string())
    );
    assert!(!json.contains("7d9d16a1"));
    assert!(!json.contains("managed/application-resumes"));
}

#[test]
fn answer_statistics_response_omits_raw_answer_history() {
    let modified_at = chrono::DateTime::parse_from_rfc3339("2026-06-01T12:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let stats = AnswerStatistics {
        pattern: "(?i)salary|compensation".to_string(),
        answer: "My salary floor is 120000".to_string(),
        times_used: 3,
        times_modified: 1,
        modification_rate: 1.0 / 3.0,
        confidence_score: 0.75,
        last_used_at: Some(modified_at),
        created_at: modified_at,
        recent_modifications: vec![ModificationExample {
            original_answer: "Expected salary 110000".to_string(),
            modified_to: "Expected salary 120000".to_string(),
            question_text: "What salary range do you need?".to_string(),
            modified_at,
        }],
    };

    let response = AnswerStatisticsResponse::from(stats);
    let json = serde_json::to_string(&response).unwrap();

    assert_eq!(response.times_used, 3);
    assert_eq!(response.recent_modifications[0].before_chars, 22);
    assert_eq!(response.recent_modifications[0].after_chars, 22);
    assert_eq!(response.recent_modifications[0].question_chars, 30);
    assert!(!json.contains("salary|compensation"));
    assert!(!json.contains("salary floor"));
    assert!(!json.contains("Expected salary"));
    assert!(!json.contains("What salary range"));
    assert!(!json.contains("originalAnswer"));
    assert!(!json.contains("modifiedTo"));
    assert!(!json.contains("questionText"));
}

#[test]
fn answer_suggestion_source_omits_raw_patterns_and_history_questions() {
    let manual = AnswerSuggestion {
        answer: "Yes".to_string(),
        confidence: 0.9,
        source: AnswerSource::Manual {
            pattern: "(?i)authorized.*work".to_string(),
            answer_id: 7,
        },
        times_used: 2,
        times_modified: 0,
        last_used_days_ago: Some(1),
        modification_rate: 0.0,
    };
    let historical = AnswerSuggestion {
        answer: "Expected salary 120000".to_string(),
        confidence: 0.6,
        source: AnswerSource::Historical {
            original_question: "What salary range do you need?".to_string(),
        },
        times_used: 1,
        times_modified: 0,
        last_used_days_ago: None,
        modification_rate: 0.0,
    };

    let manual_json = serde_json::to_string(&AnswerSuggestionResponse::from(manual)).unwrap();
    let historical_json =
        serde_json::to_string(&AnswerSuggestionResponse::from(historical)).unwrap();

    assert!(manual_json.contains("\"type\":\"manual\""));
    assert!(manual_json.contains("\"answerId\":7"));
    assert!(!manual_json.contains("authorized"));
    assert!(!manual_json.contains("pattern"));
    assert!(historical_json.contains("\"type\":\"historical\""));
    assert!(!historical_json.contains("originalQuestion"));
    assert!(!historical_json.contains("What salary range"));
}
