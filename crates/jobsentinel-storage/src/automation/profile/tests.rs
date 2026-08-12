use super::*;
use crate::test_support::migrated_pool;

fn profile_input() -> ApplicationProfileInput {
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

#[tokio::test]
async fn test_create_profile() {
    let pool = migrated_pool().await;
    let manager = ProfileManager::new(pool);

    let input = ApplicationProfileInput {
        phone: Some("+1234567890".to_string()),
        linkedin_url: Some("https://linkedin.com/in/jordanlee".to_string()),
        ..profile_input()
    };

    let profile_id = manager.upsert_profile(&input).await.unwrap();
    assert!(profile_id > 0);

    let profile = manager.get_profile().await.unwrap();
    assert!(profile.is_some());
    let profile = profile.unwrap();
    assert_eq!(profile.full_name, "Jordan Lee");
    assert_eq!(profile.email, "jordan@example.com");
    assert!(profile.us_work_authorized);
}

#[tokio::test]
async fn test_update_profile() {
    let pool = migrated_pool().await;
    let manager = ProfileManager::new(pool);

    let input1 = profile_input();

    let id1 = manager.upsert_profile(&input1).await.unwrap();

    // Update
    let input2 = ApplicationProfileInput {
        full_name: "Sam Rivera".to_string(),
        email: "sam@example.com".to_string(),
        ..input1
    };

    let id2 = manager.upsert_profile(&input2).await.unwrap();

    // Should be same ID (update, not insert)
    assert_eq!(id1, id2);

    let profile = manager.get_profile().await.unwrap().unwrap();
    assert_eq!(profile.full_name, "Sam Rivera");
    assert_eq!(profile.email, "sam@example.com");
}

#[tokio::test]
async fn test_update_profile_preserves_resume_file_without_explicit_change() {
    let pool = migrated_pool().await;
    let manager = ProfileManager::new(pool);

    let input1 = ApplicationProfileInput {
        resume_file_path: Some("<local-private-resume>".to_string()),
        ..profile_input()
    };

    manager.upsert_profile(&input1).await.unwrap();

    let input2 = ApplicationProfileInput {
        email: "jordan.updated@example.com".to_string(),
        resume_file_path: None,
        clear_resume_file: None,
        ..input1
    };

    manager.upsert_profile(&input2).await.unwrap();

    let profile = manager.get_profile().await.unwrap().unwrap();
    assert_eq!(
        profile.resume_file_path,
        Some("<local-private-resume>".to_string())
    );
}

#[tokio::test]
async fn test_update_profile_replaces_and_clears_resume_file_explicitly() {
    let pool = migrated_pool().await;
    let manager = ProfileManager::new(pool);

    let input1 = ApplicationProfileInput {
        resume_file_path: Some("<local-private-resume>".to_string()),
        ..profile_input()
    };

    manager.upsert_profile(&input1).await.unwrap();

    let input2 = ApplicationProfileInput {
        resume_file_path: Some("C:\\Users\\Jordan\\Desktop\\new-resume.docx".to_string()),
        ..input1.clone()
    };
    manager.upsert_profile(&input2).await.unwrap();

    let profile = manager.get_profile().await.unwrap().unwrap();
    assert_eq!(
        profile.resume_file_path,
        Some("C:\\Users\\Jordan\\Desktop\\new-resume.docx".to_string())
    );

    let input3 = ApplicationProfileInput {
        resume_file_path: None,
        clear_resume_file: Some(true),
        ..input1
    };
    manager.upsert_profile(&input3).await.unwrap();

    let profile = manager.get_profile().await.unwrap().unwrap();
    assert_eq!(profile.resume_file_path, None);
}

#[test]
fn test_screening_question_matching_treats_symbols_as_literal_text() {
    assert!(screening_question_matches(
        "Security+",
        "Do you have a Security+ certification?"
    ));
    assert!(!screening_question_matches(
        "Security+",
        "Do you have a security clearance?"
    ));
}

#[test]
fn test_screening_question_matching_handles_plain_words_and_legacy_defaults() {
    assert!(screening_question_matches(
        "US citizen",
        "Are you a U.S. citizen?"
    ));
    assert!(screening_question_matches(
        "background check",
        "Can you complete a background check?"
    ));
    assert!(screening_question_matches(
        "(?i)authorized.*work.*(united states|us|usa)",
        "Are you authorized to work in the US?"
    ));
}

#[test]
fn test_screening_question_matching_handles_plain_quick_add_aliases() {
    assert!(screening_question_matches(
        "work authorization",
        "Are you legally authorized to work in the United States?"
    ));
    assert!(screening_question_matches(
        "physical requirements",
        "Are you able to lift 50 pounds safely?"
    ));
    assert!(screening_question_matches(
        "education",
        "Do you have a bachelor's degree or equivalent education?"
    ));
    assert!(screening_question_matches(
        "availability",
        "Can you work weekends and rotating shifts?"
    ));
    assert!(screening_question_matches(
        "reliable transportation",
        "Do you have access to a reliable vehicle for client visits?"
    ));
    assert!(screening_question_matches(
        "driver's license",
        "Do you have a valid driver license?"
    ));
    assert!(screening_question_matches(
        "salary",
        "What is your expected compensation?"
    ));
    assert!(!screening_question_matches(
        "certification",
        "Do you have a driver's license?"
    ));
}

#[tokio::test]
async fn test_screening_answers() {
    let pool = migrated_pool().await;
    let manager = ProfileManager::new(pool);

    manager
        .upsert_screening_answer(
            "(?i)authorized.*work.*us",
            "Yes",
            "yes_no",
            Some("US work authorization"),
        )
        .await
        .unwrap();

    let answers = manager.get_screening_answers().await.unwrap();
    assert!(answers.len() >= 1); // At least 1 (plus default ones from migration)

    // Test pattern matching
    let answer = manager
        .find_answer_for_question("Are you authorized to work in the US?")
        .await
        .unwrap();
    assert_eq!(answer, Some("Yes".to_string()));
}

#[tokio::test]
async fn protected_screening_answers_remain_saved_but_are_not_found() {
    let pool = migrated_pool().await;
    let manager = ProfileManager::new(pool);

    manager
        .upsert_screening_answer("veteran status", "Yes", "yes_no", None)
        .await
        .unwrap();
    manager
        .upsert_screening_answer("pronouns", "they/them", "text", None)
        .await
        .unwrap();

    assert!(manager
        .get_screening_answers()
        .await
        .unwrap()
        .iter()
        .any(|answer| answer.question_pattern == "veteran status" && answer.answer == "Yes"));
    assert_eq!(
        manager
            .find_answer_for_question("Do you identify as a protected veteran?")
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        manager
            .find_answer_for_question("What are your pronouns?")
            .await
            .unwrap(),
        None
    );
}

#[tokio::test]
async fn test_screening_answer_legacy_boolean_type_is_normalized() {
    let pool = migrated_pool().await;
    let manager = ProfileManager::new(pool);

    manager
        .upsert_screening_answer("(?i)authorized.*work", "Yes", "boolean", None)
        .await
        .unwrap();

    let answers = manager.get_screening_answers().await.unwrap();
    let answer = answers
        .iter()
        .find(|answer| answer.question_pattern == "(?i)authorized.*work")
        .unwrap();

    assert_eq!(answer.answer_type.as_deref(), Some("yes_no"));
}

#[tokio::test]
async fn test_screening_answer_invalid_type_is_rejected() {
    let pool = migrated_pool().await;
    let manager = ProfileManager::new(pool);

    let result = manager
        .upsert_screening_answer("(?i)invalid.*type", "Yes", "checkbox", None)
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("checkbox"));
}
