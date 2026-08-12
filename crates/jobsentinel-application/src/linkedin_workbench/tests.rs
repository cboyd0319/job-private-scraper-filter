use super::*;

async fn unreviewed_db() -> Database {
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    crate::v3_source_governance::install_linkedin_workbench(&db)
        .await
        .unwrap();
    db
}

async fn memory_db() -> Database {
    let db = unreviewed_db().await;
    remember_workbench_review(&db).await.unwrap();
    db
}

#[tokio::test]
async fn direct_record_without_backend_review_makes_no_durable_changes() {
    let db = unreviewed_db().await;

    assert!(record_event(
        &db,
        LinkedInWorkbenchEventInput {
            event_type: LinkedInWorkbenchEventType::Applied,
            title: Some("Security Engineer".to_string()),
            company: Some("Example Co".to_string()),
            url: Some("https://www.linkedin.com/jobs/view/123".to_string()),
            notes: Some("private note".to_string()),
        },
    )
    .await
    .is_err());
    assert!(db.get_recent_jobs(10).await.unwrap().is_empty());
    assert_eq!(
        db.application_tracker()
            .get_application_stats()
            .await
            .unwrap()
            .total,
        0
    );
}

#[tokio::test]
async fn revoked_or_drifted_workbench_review_fails_closed() {
    let db = memory_db().await;
    assert!(revoke_workbench_review(&db).await.unwrap());
    assert!(!workbench_review_is_current(&db).await.unwrap());
    assert!(record_event(
        &db,
        LinkedInWorkbenchEventInput {
            event_type: LinkedInWorkbenchEventType::Saved,
            title: None,
            company: None,
            url: None,
            notes: None,
        },
    )
    .await
    .is_err());

    remember_workbench_review(&db).await.unwrap();
    let mut drifted = crate::v3_source_governance::linkedin_workbench_policy().unwrap();
    drifted.revision += 1;
    db.upsert_source_policy(&drifted).await.unwrap();
    assert!(!workbench_review_is_current(&db).await.unwrap());
    assert!(record_event(
        &db,
        LinkedInWorkbenchEventInput {
            event_type: LinkedInWorkbenchEventType::Saved,
            title: None,
            company: None,
            url: None,
            notes: None,
        },
    )
    .await
    .is_err());
}

#[tokio::test]
async fn review_status_distinguishes_consent_from_stale_policy_evidence() {
    let reviewed = memory_db().await;
    assert_eq!(
        workbench_review_status_on(
            &reviewed,
            chrono::NaiveDate::from_ymd_opt(2026, 8, 18).unwrap()
        )
        .await
        .unwrap(),
        LinkedInWorkbenchReviewStatus::Reviewed
    );
    assert_eq!(
        workbench_review_status_on(
            &reviewed,
            chrono::NaiveDate::from_ymd_opt(2026, 8, 19).unwrap()
        )
        .await
        .unwrap(),
        LinkedInWorkbenchReviewStatus::PolicyRefreshRequired
    );

    let unreviewed = unreviewed_db().await;
    assert!(remember_workbench_review_on(
        &unreviewed,
        chrono::NaiveDate::from_ymd_opt(2026, 8, 19).unwrap()
    )
    .await
    .is_err());
    assert!(unreviewed
        .source_consent_history("linkedin-workbench", 100)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn applied_event_creates_draft_application_without_hidden_browser_state() {
    let db = memory_db().await;

    let result = record_event(
        &db,
        LinkedInWorkbenchEventInput {
            event_type: LinkedInWorkbenchEventType::Applied,
            title: None,
            company: None,
            url: None,
            notes: Some("User clicked Easy Apply in LinkedIn.".to_string()),
        },
    )
    .await
    .unwrap();

    assert_eq!(result.status, "applied");
    assert!(result.needs_details);
    assert!(result.saved_as_bookmark);
    assert!(result.application_id.is_some());

    let app = db
        .application_tracker()
        .get_application(result.application_id.unwrap())
        .await
        .unwrap();
    assert_eq!(app.status, ApplicationStatus::Applied);
    assert!(app.applied_at.is_some());

    let job = db.get_job_by_hash(&result.job_hash).await.unwrap().unwrap();
    assert_eq!(job.source, SOURCE);
    assert_eq!(job.title, DEFAULT_TITLE);
    assert_eq!(job.company, DEFAULT_COMPANY);
    assert!(job.bookmarked);
    assert_eq!(
        job.notes.as_deref(),
        Some("User clicked Easy Apply in LinkedIn.")
    );
}

#[tokio::test]
async fn saved_event_uses_user_confirmed_url_and_does_not_create_application() {
    let db = memory_db().await;

    let result = record_event(
        &db,
        LinkedInWorkbenchEventInput {
            event_type: LinkedInWorkbenchEventType::Saved,
            title: Some("Staff Content Strategist".to_string()),
            company: Some("Example Co".to_string()),
            url: Some("https://www.linkedin.com/jobs/view/123?token=secret".to_string()),
            notes: None,
        },
    )
    .await
    .unwrap();

    assert_eq!(result.status, "saved");
    assert!(!result.needs_details);
    assert!(result.saved_as_bookmark);
    assert!(result.application_id.is_none());

    let job = db.get_job_by_hash(&result.job_hash).await.unwrap().unwrap();
    assert_eq!(job.title, "Staff Content Strategist");
    assert_eq!(job.company, "Example Co");
    assert_eq!(job.url, "https://www.linkedin.com/jobs/view/123");
    assert!(job.bookmarked);
}

#[tokio::test]
async fn not_interested_event_hides_local_draft() {
    let db = memory_db().await;

    let result = record_event(
        &db,
        LinkedInWorkbenchEventInput {
            event_type: LinkedInWorkbenchEventType::NotInterested,
            title: Some("Role to skip".to_string()),
            company: Some("Nope Co".to_string()),
            url: None,
            notes: None,
        },
    )
    .await
    .unwrap();

    assert!(result.hidden);
    let job = db.get_job_by_hash(&result.job_hash).await.unwrap().unwrap();
    assert!(job.hidden);
}

#[tokio::test]
async fn expanded_activity_events_use_local_application_ledger() {
    let db = memory_db().await;
    let tracker = db.application_tracker();

    let interview = record_event(
        &db,
        LinkedInWorkbenchEventInput {
            event_type: LinkedInWorkbenchEventType::Interview,
            title: Some("Support Lead".to_string()),
            company: Some("Example Co".to_string()),
            url: Some("https://www.linkedin.com/jobs/view/222".to_string()),
            notes: None,
        },
    )
    .await
    .unwrap();
    let interview_app = tracker
        .get_application(interview.application_id.expect("interview app"))
        .await
        .unwrap();
    assert_eq!(interview.status, "interview");
    assert_eq!(interview_app.status, ApplicationStatus::PhoneInterview);
    assert!(interview.saved_as_bookmark);

    let follow_up = record_event(
        &db,
        LinkedInWorkbenchEventInput {
            event_type: LinkedInWorkbenchEventType::FollowUp,
            title: Some("Support Lead".to_string()),
            company: Some("Example Co".to_string()),
            url: Some("https://www.linkedin.com/jobs/view/222?token=secret".to_string()),
            notes: Some("Sent a short follow-up.".to_string()),
        },
    )
    .await
    .unwrap();
    let follow_up_app = tracker
        .get_application(follow_up.application_id.expect("follow-up app"))
        .await
        .unwrap();
    assert_eq!(follow_up.status, "follow_up");
    assert!(follow_up_app.last_contact.is_some());

    let reminder = record_event(
        &db,
        LinkedInWorkbenchEventInput {
            event_type: LinkedInWorkbenchEventType::Reminder,
            title: Some("Support Lead".to_string()),
            company: Some("Example Co".to_string()),
            url: Some("https://www.linkedin.com/jobs/view/222".to_string()),
            notes: None,
        },
    )
    .await
    .unwrap();
    let reminder_application_id = reminder.application_id.expect("reminder app");
    let due_reminder_count = tracker
        .get_pending_reminders()
        .await
        .unwrap()
        .into_iter()
        .filter(|pending| pending.application_id == reminder_application_id)
        .count();
    assert_eq!(reminder.status, "reminder");
    assert!(reminder.saved_as_bookmark);
    assert_eq!(due_reminder_count, 0);

    let rejected = record_event(
        &db,
        LinkedInWorkbenchEventInput {
            event_type: LinkedInWorkbenchEventType::Rejected,
            title: Some("Support Lead".to_string()),
            company: Some("Example Co".to_string()),
            url: Some("https://www.linkedin.com/jobs/view/222".to_string()),
            notes: None,
        },
    )
    .await
    .unwrap();
    let rejected_app = tracker
        .get_application(rejected.application_id.expect("rejected app"))
        .await
        .unwrap();
    assert_eq!(rejected.status, "rejected");
    assert_eq!(rejected_app.status, ApplicationStatus::Rejected);
}

#[tokio::test]
async fn notes_remove_sensitive_url_fields_before_storage() {
    let db = memory_db().await;

    let result = record_event(
        &db,
        LinkedInWorkbenchEventInput {
            event_type: LinkedInWorkbenchEventType::Applied,
            title: Some("Principal Security Engineer".to_string()),
            company: Some("Example Co".to_string()),
            url: Some(
                "https://www.linkedin.com/jobs/view/123?currentJobId=123&token=secret".to_string(),
            ),
            notes: Some(
                "Saved from https://www.linkedin.com/jobs/view/123?token=secret".to_string(),
            ),
        },
    )
    .await
    .unwrap();

    let job = db.get_job_by_hash(&result.job_hash).await.unwrap().unwrap();
    assert_eq!(job.url, "https://www.linkedin.com/jobs/view/123");
    let notes = job.notes.unwrap_or_default();
    assert!(notes.contains("https://www.linkedin.com/jobs/view/123"));
    assert!(!notes.contains("token=secret"));
}

#[tokio::test]
async fn secret_bearing_notes_fail_before_any_durable_write() {
    for notes in [
        r#"{"access_token": "json-secret-value"}"#,
        "Authorization: Basic dXNlcjpwYXNz",
        "li_at = AQED-opaque-value",
        "Cookie: JSESSIONID=ajax:123456; bcookie=v=2",
        "Bearer eyJhbGciOi.fake.payload",
        r#"{"name":"li_at","value":"AQED-opaque-value"}"#,
        "oauth_token=oauth-secret-value",
        r#"{"id_token":"eyJhbGciOi.fake.payload"}"#,
        "client_secret=client-secret-value",
        "curl -H 'Authorization: Basic dXNlcjpwYXNz' https://example.com",
        r#"{"authorization":"Bearer eyJhbGciOi.fake.payload"}"#,
        "curl -H 'Cookie: li_at=AQED-opaque-value' https://example.com",
        r#"{"cookie":"li_at=AQED-opaque-value"}"#,
    ] {
        let db = memory_db().await;
        let result = record_event(
            &db,
            LinkedInWorkbenchEventInput {
                event_type: LinkedInWorkbenchEventType::Note,
                title: Some("Principal Security Engineer".to_string()),
                company: Some("Example Co".to_string()),
                url: Some("https://www.linkedin.com/jobs/view/123".to_string()),
                notes: Some(notes.to_string()),
            },
        )
        .await;

        assert_eq!(
            result.unwrap_err().to_string(),
            "LinkedIn Workbench text cannot contain session or credential material",
            "{notes}"
        );
        assert!(db.get_recent_jobs(10).await.unwrap().is_empty(), "{notes}");
    }
}

#[tokio::test]
async fn secret_bearing_title_or_company_fails_before_any_durable_write() {
    for (title, company) in [
        (
            "Authorization: Bearer eyJhbGciOi.fake.payload",
            "Example Co",
        ),
        (
            "Principal Security Engineer",
            r#"{"name":"JSESSIONID","value":"ajax:123456"}"#,
        ),
    ] {
        let db = memory_db().await;
        let result = record_event(
            &db,
            LinkedInWorkbenchEventInput {
                event_type: LinkedInWorkbenchEventType::Saved,
                title: Some(title.to_string()),
                company: Some(company.to_string()),
                url: Some("https://www.linkedin.com/jobs/view/123".to_string()),
                notes: None,
            },
        )
        .await;

        assert!(result.is_err(), "{title} / {company}");
        assert!(db.get_recent_jobs(10).await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn legitimate_clearance_and_interview_notes_are_preserved() {
    let db = memory_db().await;
    let notes = "Secret clearance: required. Work authorization: U.S. citizen. Interview session: Tuesday. Bearer of good news. Bearer authentication. Authorization: Basic knowledge.";

    let result = record_event(
        &db,
        LinkedInWorkbenchEventInput {
            event_type: LinkedInWorkbenchEventType::Note,
            title: Some("Principal Security Engineer".to_string()),
            company: Some("Example Co".to_string()),
            url: Some("https://www.linkedin.com/jobs/view/123".to_string()),
            notes: Some(notes.to_string()),
        },
    )
    .await
    .unwrap();

    assert_eq!(
        db.get_job_by_hash(&result.job_hash)
            .await
            .unwrap()
            .unwrap()
            .notes
            .as_deref(),
        Some(notes)
    );
}
