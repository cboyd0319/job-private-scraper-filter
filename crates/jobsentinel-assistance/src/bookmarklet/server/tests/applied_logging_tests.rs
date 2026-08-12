use super::*;

#[tokio::test]
async fn applied_logging_queues_a_local_draft_with_missing_details() {
    let database = bookmarklet_test_database().await;
    let (active_pairing, code) =
        bookmarklet_pairing_for_operation("https://example.com", SourceOperation::AppliedLogging);
    let pending_imports = bookmarklet_pending_imports();
    let body = bookmarklet_import_body(
        &code,
        serde_json::json!({"url": "https://example.com/jobs/1"}),
    );

    let (response, _) = handle_import_request(
        &bookmarklet_import_request(&body),
        &active_pairing,
        pending_imports.clone(),
        database.clone(),
    )
    .await;
    let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
    let previews = pending_bookmarklet_import_previews(&pending_imports);

    assert_eq!(parsed["success"], true, "{parsed}");
    assert_eq!(previews.len(), 1);
    let preview = serde_json::to_value(&previews[0]).unwrap();
    assert_eq!(preview["operation"], "applied_logging");
    assert_eq!(
        preview["missing_fields"],
        serde_json::json!(["title", "company"])
    );
    assert_eq!(stored_job_count(&database).await, 0);
}

#[tokio::test]
async fn confirming_an_applied_draft_rechecks_authority_and_marks_it_applied() {
    let database = bookmarklet_test_database().await;
    let (active_pairing, code) =
        bookmarklet_pairing_for_operation("https://example.com", SourceOperation::AppliedLogging);
    let pending_imports = bookmarklet_pending_imports();
    let body = bookmarklet_import_body(
        &code,
        serde_json::json!({"url": "https://example.com/jobs/1"}),
    );
    handle_import_request(
        &bookmarklet_import_request(&body),
        &active_pairing,
        pending_imports.clone(),
        database.clone(),
    )
    .await;
    let preview = pending_bookmarklet_import_previews(&pending_imports)
        .pop()
        .unwrap();

    database.set_authorization_allowed(false);
    let blocked =
        confirm_pending_bookmarklet_imports(&database, &pending_imports, &[preview.id.clone()])
            .await;

    assert_eq!(
        blocked.unwrap_err(),
        BOOKMARKLET_AUTHORIZATION_FAILURE_MESSAGE
    );
    assert_eq!(stored_job_count(&database).await, 0);

    database.set_authorization_allowed(true);
    let result =
        confirm_pending_bookmarklet_imports(&database, &pending_imports, &[preview.id]).await;
    let stored = database.jobs().remove(0);

    assert_eq!(result.unwrap().imported, 1);
    assert_eq!(stored.title, "Job title not added");
    assert_eq!(stored.company, "Company not added");
    assert!(database.is_applied(&stored.hash));
}

#[tokio::test]
async fn applied_logging_rejects_extra_page_state() {
    for extra in [
        serde_json::json!({"description": "hidden description"}),
        serde_json::json!({"location": "hidden location"}),
        serde_json::json!({"remote": true}),
        serde_json::json!({
            "@type": "JobPosting",
            "hiringOrganization": {"name": "Hidden organization"}
        }),
    ] {
        let database = bookmarklet_test_database().await;
        let (active_pairing, code) = bookmarklet_pairing_for_operation(
            "https://example.com",
            SourceOperation::AppliedLogging,
        );
        let pending_imports = bookmarklet_pending_imports();
        let mut job = serde_json::json!({
            "title": "Visible title",
            "company": "Visible company",
            "url": "https://example.com/jobs/1"
        });
        job.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        let body = bookmarklet_import_body(&code, job);

        let (response, _) = handle_import_request(
            &bookmarklet_import_request(&body),
            &active_pairing,
            pending_imports.clone(),
            database,
        )
        .await;
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();

        assert_eq!(parsed["error"], INVALID_BOOKMARKLET_PAYLOAD_MESSAGE);
        assert!(pending_bookmarklet_import_previews(&pending_imports).is_empty());
    }
}
