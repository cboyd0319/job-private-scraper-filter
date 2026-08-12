use super::*;
use chrono::Utc;

fn make_test_profile() -> ApplicationProfile {
    ApplicationProfile {
        id: 1,
        full_name: "Jordan Lee".to_string(),
        email: "jordan@example.com".to_string(),
        phone: Some("+1234567890".to_string()),
        linkedin_url: Some("https://linkedin.com/in/jordanlee".to_string()),
        github_url: None,
        portfolio_url: None,
        website_url: None,
        default_resume_id: None,
        resume_file_path: None,
        default_cover_letter_template: None,
        us_work_authorized: true,
        requires_sponsorship: false,
        max_applications_per_day: 10,
        require_manual_approval: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn make_screening_answer(pattern: &str, answer: &str) -> ScreeningAnswer {
    ScreeningAnswer {
        id: 1,
        question_pattern: pattern.to_string(),
        answer: answer.to_string(),
        answer_type: Some("text".to_string()),
        notes: None,
        times_used: 0,
        times_modified: 0,
        confidence_score: 1.0,
        last_used_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[test]
fn test_get_greenhouse_selectors() {
    let selectors = FormFiller::get_field_selectors(&AtsPlatform::Greenhouse);
    assert!(selectors.contains_key(&FieldType::FirstName));
    assert!(selectors.contains_key(&FieldType::Email));
    assert!(selectors.contains_key(&FieldType::Resume));
}

#[test]
fn test_get_lever_selectors() {
    let selectors = FormFiller::get_field_selectors(&AtsPlatform::Lever);
    assert!(selectors.contains_key(&FieldType::FullName));
    assert!(selectors.contains_key(&FieldType::Email));
}

#[test]
fn test_get_unknown_selectors() {
    let selectors = FormFiller::get_field_selectors(&AtsPlatform::Unknown);
    assert!(selectors.is_empty());
}

#[test]
fn test_expanded_platforms_use_generic_review_first_selectors() {
    for platform in [
        AtsPlatform::SmartRecruiters,
        AtsPlatform::Workable,
        AtsPlatform::Recruitee,
        AtsPlatform::BreezyHr,
        AtsPlatform::JazzHr,
        AtsPlatform::Bullhorn,
        AtsPlatform::Jobvite,
        AtsPlatform::Teamtailor,
        AtsPlatform::SuccessFactors,
        AtsPlatform::OracleRecruiting,
        AtsPlatform::Personio,
        AtsPlatform::Eightfold,
    ] {
        let selectors = FormFiller::get_field_selectors(&platform);
        assert!(selectors.contains_key(&FieldType::Email), "{platform:?}");
        assert!(selectors.contains_key(&FieldType::Resume), "{platform:?}");
    }
}

#[test]
fn test_screening_answer_matching() {
    let profile = make_test_profile();
    let answers = vec![
        make_screening_answer("years of experience", "5"),
        make_screening_answer("authorized work US", "Yes"),
        make_screening_answer("salary", "120000"),
        make_screening_answer("work from home", "Yes, I prefer remote work"),
    ];

    let filler = FormFiller::new(profile, None).with_screening_answers(answers);

    // Test exact matches
    assert_eq!(
        filler.find_answer_for_question("How many years of experience do you have?"),
        Some("5".to_string())
    );
    assert_eq!(
        filler.find_answer_for_question("Are you authorized to work in the US?"),
        Some("Yes".to_string())
    );
    assert_eq!(
        filler.find_answer_for_question("What is your expected salary?"),
        Some("120000".to_string())
    );
    assert_eq!(
        filler.find_answer_for_question("Are you open to remote work from home?"),
        Some("Yes, I prefer remote work".to_string())
    );

    // Test non-matching question
    assert_eq!(
        filler.find_answer_for_question("What is your favorite color?"),
        None
    );
}

#[test]
fn test_screening_answer_matching_handles_plain_quick_add_aliases() {
    let profile = make_test_profile();
    let answers = vec![
        make_screening_answer("work authorization", "Yes"),
        make_screening_answer("physical requirements", "Can lift 50 pounds safely"),
        make_screening_answer("education", "Bachelor's degree"),
        make_screening_answer("availability", "Available weekends"),
        make_screening_answer("reliable transportation", "Yes"),
    ];

    let filler = FormFiller::new(profile, None).with_screening_answers(answers);

    assert_eq!(
        filler.find_answer_for_question("Are you legally authorized to work in the United States?"),
        Some("Yes".to_string())
    );
    assert_eq!(
        filler.find_answer_for_question("Are you able to lift 50 pounds safely?"),
        Some("Can lift 50 pounds safely".to_string())
    );
    assert_eq!(
        filler.find_answer_for_question("Do you have a bachelor's degree or equivalent education?"),
        Some("Bachelor's degree".to_string())
    );
    assert_eq!(
        filler.find_answer_for_question("Can you work weekends and rotating shifts?"),
        Some("Available weekends".to_string())
    );
    assert_eq!(
        filler.find_answer_for_question(
            "Do you have access to a reliable vehicle for client visits?"
        ),
        Some("Yes".to_string())
    );
}

#[test]
fn test_screening_answer_case_insensitive() {
    let profile = make_test_profile();
    let answers = vec![make_screening_answer("security clearance", "No")];

    let filler = FormFiller::new(profile, None).with_screening_answers(answers);

    // Should match regardless of case
    assert_eq!(
        filler.find_answer_for_question("Do you have a Security Clearance?"),
        Some("No".to_string())
    );
    assert_eq!(
        filler.find_answer_for_question("SECURITY CLEARANCE STATUS"),
        Some("No".to_string())
    );
}

#[test]
fn test_screening_answer_symbols_are_literal() {
    let profile = make_test_profile();
    let answers = vec![make_screening_answer("Security+", "Yes")];

    let filler = FormFiller::new(profile, None).with_screening_answers(answers);

    assert_eq!(
        filler.find_answer_for_question("Do you have a Security+ certification?"),
        Some("Yes".to_string())
    );
    assert_eq!(
        filler.find_answer_for_question("Do you have a security clearance?"),
        None
    );
}

#[test]
fn screening_field_label_does_not_echo_question_text() {
    assert_eq!(SCREENING_FIELD_LABEL, "screening:saved_answer");
    assert!(!SCREENING_FIELD_LABEL.contains("salary"));
    assert!(!SCREENING_FIELD_LABEL.contains("authorized"));
}

#[test]
fn screening_answer_review_topics_are_bounded() {
    assert_eq!(
        screening_answer_review_topic("work authorization"),
        Some("work authorization")
    );
    assert_eq!(
        screening_answer_review_topic("Bachelor's degree"),
        Some("education")
    );
    assert_eq!(
        screening_answer_review_topic("weekend availability"),
        Some("schedule or availability")
    );
    assert_eq!(screening_answer_review_topic("favorite color"), None);
}

#[test]
fn protected_screening_answers_are_never_returned_and_require_manual_review() {
    let filler = FormFiller::new(make_test_profile(), None).with_screening_answers(vec![
        make_screening_answer("veteran status", "Yes"),
        make_screening_answer("Hispanic or Latino", "Decline to answer"),
        make_screening_answer("pronouns", "they/them"),
    ]);

    for question in [
        "Do you identify as a protected veteran?",
        "Are you Hispanic or Latino?",
        "What are your pronouns?",
    ] {
        assert_eq!(filler.find_answer_for_question(question), None);
    }
}

#[test]
fn question_discovery_error_does_not_echo_browser_detail() {
    let error = question_discovery_error().to_string();

    assert_eq!(error, QUESTION_DISCOVERY_ERROR);
    assert!(!error.contains("https://"));
    assert!(!error.contains("token"));
    assert!(!error.contains("selector"));
    assert!(!error.contains("Jordan Lee"));
}
