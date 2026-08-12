use super::resume_match_debugger_commands::{
    is_saved_match_debugger_opaque_id, validate_saved_match_debugger_args,
    validate_saved_match_packet_args,
};
use super::*;
use crate::application::resume::{
    ProfessionMatchingProfile, RegionalMatchingProfile, ResumeMatchingProfile,
};
use crate::application::v3_foundation::SavedMatchDebugger;

#[test]
fn explicit_matching_profile_reaches_the_application_owner() {
    let mut resume = ResumeAnalysisInput::default();
    resume.resume.summary = Some("Led program evaluation.".to_string());
    let profile = ResumeMatchingProfile {
        profession: ProfessionMatchingProfile::Operations,
        region: RegionalMatchingProfile::UnitedKingdom,
    };

    let result = analyze_resume_for_job(
        resume,
        "Required: programme evaluation".to_string(),
        Some(profile),
    )
    .unwrap();

    assert_eq!(result.matching_profile, Some(profile));
    assert_eq!(result.keyword_matches[0].keyword, "program evaluation");
}

#[test]
fn resume_match_feedback_is_closed_and_content_free() {
    assert_eq!(
        serde_json::from_str::<ResumeMatchFeedbackLabel>("\"not_relevant\"").unwrap(),
        ResumeMatchFeedbackLabel::NotRelevant
    );
    assert!(serde_json::from_str::<ResumeMatchFeedbackLabel>("\"maybe\"").is_err());

    let value = serde_json::to_value(ResumeMatchFeedback {
        match_id: 42,
        label: ResumeMatchFeedbackLabel::Useful,
        recorded_at: Utc::now(),
    })
    .unwrap();
    let mut keys = value
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    keys.sort();

    assert_eq!(keys, ["label", "match_id", "recorded_at"]);
}

#[test]
fn saved_match_debugger_arguments_require_a_saved_job_and_positive_resume() {
    assert_eq!(validate_saved_match_debugger_args("saved-job", 7), Ok(()));
    assert!(validate_saved_match_debugger_args("", 7).is_err());
    assert!(validate_saved_match_debugger_args("saved-job", 0).is_err());
    assert!(validate_saved_match_debugger_args(&"a".repeat(129), 7).is_err());
    assert!(is_saved_match_debugger_opaque_id(&"a".repeat(64)));
    assert!(!is_saved_match_debugger_opaque_id(&"a".repeat(63)));
    assert!(!is_saved_match_debugger_opaque_id(&"g".repeat(64)));
}

#[test]
fn saved_match_debugger_response_remains_renderer_serializable() {
    fn assert_serializable<T: serde::Serialize>() {}

    assert_serializable::<SavedMatchDebugger>();
}

#[test]
fn saved_match_packet_arguments_require_reviewed_text_and_current_evidence() {
    let evidence_id = "a".repeat(64);
    assert_eq!(
        validate_saved_match_packet_args("Reviewed scheduling claim", &[evidence_id.clone()]),
        Ok(())
    );
    assert!(validate_saved_match_packet_args("", &[evidence_id.clone()]).is_err());
    assert!(validate_saved_match_packet_args("Reviewed scheduling claim", &[]).is_err());
    assert!(validate_saved_match_packet_args(
        "Reviewed scheduling claim",
        &[evidence_id.clone(), evidence_id.clone()],
    )
    .is_err());
    assert!(
        validate_saved_match_packet_args("Reviewed scheduling claim", &["g".repeat(64)]).is_err()
    );
    assert!(validate_saved_match_packet_args(&"x".repeat(8_193), &[evidence_id]).is_err());
}

#[test]
fn resume_summary_serialization_omits_file_path_and_parsed_text() {
    let now = Utc::now();
    let resume = Resume {
        id: 42,
        name: "Private Resume.pdf".to_string(),
        file_path: "<local-private-resume>".to_string(),
        parsed_text: Some("secret resume body".to_string()),
        is_active: true,
        created_at: now,
        updated_at: now,
    };

    let serialized = serde_json::to_string(&ResumeSummary::from(resume)).unwrap();

    assert!(serialized.contains("Private Resume.pdf"));
    assert!(!serialized.contains("file_path"));
    assert!(!serialized.contains("<local-private-resume>"));
    assert!(!serialized.contains("parsed_text"));
    assert!(!serialized.contains("secret resume body"));
}

#[test]
fn resume_summary_includes_sanitized_import_status() {
    let now = Utc::now();
    let resume = Resume {
        id: 42,
        name: "Private Resume.pdf".to_string(),
        file_path: "<local-private-resume>".to_string(),
        parsed_text: Some("Care coordinator\nScheduling".to_string()),
        is_active: true,
        created_at: now,
        updated_at: now,
    };

    let summary = ResumeSummary::from(resume);
    let serialized = serde_json::to_string(&summary).unwrap();

    assert!(serialized.contains("\"format_label\":\"PDF\""));
    assert!(serialized.contains("\"has_readable_text\":true"));
    assert!(serialized.contains("\"readable_text_chars\":27"));
    assert!(!serialized.contains("<local-private-resume>"));
    assert!(!serialized.contains("Care coordinator"));
}

#[test]
fn resume_text_preview_serialization_omits_file_path() {
    let now = Utc::now();
    let resume = Resume {
        id: 42,
        name: "Private Resume.pdf".to_string(),
        file_path: "<local-private-resume>".to_string(),
        parsed_text: Some("Care coordinator\nScheduling\nCase management".to_string()),
        is_active: true,
        created_at: now,
        updated_at: now,
    };

    let preview = ResumeTextPreview::from(resume);
    let serialized = serde_json::to_string(&preview).unwrap();

    assert!(serialized.contains("\"resume_id\":42"));
    assert!(serialized.contains("Care coordinator"));
    assert!(!serialized.contains("file_path"));
    assert!(!serialized.contains("<local-private-resume>"));
    assert!(!serialized.contains("parsed_text"));
}

#[test]
fn resume_text_preview_is_bounded_and_reports_truncation() {
    let now = Utc::now();
    let long_text = "Care ".repeat(MAX_RESUME_TEXT_PREVIEW_CHARS);
    let resume = Resume {
        id: 42,
        name: "Long Resume.pdf".to_string(),
        file_path: "<local-long-resume>".to_string(),
        parsed_text: Some(long_text),
        is_active: true,
        created_at: now,
        updated_at: now,
    };

    let preview = ResumeTextPreview::from(resume);

    assert_eq!(
        preview.text_preview.chars().count(),
        MAX_RESUME_TEXT_PREVIEW_CHARS
    );
    assert!(preview.text_chars > preview.text_preview.chars().count());
    assert!(preview.is_truncated);
    assert!(preview.has_text);
}

#[test]
fn json_resume_file_validation_requires_json_extension() {
    assert!(has_json_extension(Path::new("resume.JSON")));
    assert!(has_json_extension(Path::new(r"C:\Resume\resume.json")));
    assert!(!has_json_extension(Path::new("resume.pdf")));
    assert!(!has_json_extension(Path::new("resume")));
}

#[test]
fn resume_upload_extension_validation_accepts_supported_local_formats() {
    assert_eq!(
        supported_resume_extension(Path::new("resume.PDF")).as_deref(),
        Some("pdf")
    );
    assert_eq!(
        supported_resume_extension(Path::new("resume.DOCX")).as_deref(),
        Some("docx")
    );
    assert_eq!(
        supported_resume_extension(Path::new("resume.txt")).as_deref(),
        Some("txt")
    );
    assert_eq!(
        supported_resume_extension(Path::new("resume.md")).as_deref(),
        Some("md")
    );
    assert_eq!(
        supported_resume_extension(Path::new("resume.HTML")).as_deref(),
        Some("html")
    );
    assert_eq!(
        supported_resume_extension(Path::new("resume.htm")).as_deref(),
        Some("htm")
    );
    assert!(supported_resume_extension(Path::new("resume.doc")).is_none());
}

#[test]
fn safe_resume_upload_file_name_preserves_supported_extension() {
    let docx_name = safe_resume_upload_file_name(Path::new("Jordan Resume.docx"));
    let text_name = safe_resume_upload_file_name(Path::new("Jordan Resume.txt"));
    let html_name = safe_resume_upload_file_name(Path::new("Jordan Resume.html"));

    assert!(docx_name.ends_with("--Jordan-Resume.docx"));
    assert!(text_name.ends_with("--Jordan-Resume.txt"));
    assert!(html_name.ends_with("--Jordan-Resume.html"));
    assert!(!docx_name.contains(' '));
    assert!(!text_name.contains(' '));
    assert!(!html_name.contains(' '));
}

#[test]
fn managed_resume_upload_cleanup_deletes_only_managed_tokens() {
    let managed_dir = tempfile::tempdir().unwrap();
    let outside_dir = tempfile::tempdir().unwrap();
    let token = "7d9d16a1-2e5d-4b32-9eb2-bfbffb4ee871--Jordan-Resume.pdf";
    let managed_resume = managed_dir.path().join(token);
    let outside_resume = outside_dir.path().join(token);
    std::fs::write(&managed_resume, b"managed").unwrap();
    std::fs::write(&outside_resume, b"outside").unwrap();

    assert!(managed_resume_upload_cleanup_path(
        Some(outside_resume.to_string_lossy().as_ref()),
        managed_dir.path(),
    )
    .is_none());
    assert!(outside_resume.exists());

    let cleanup_path = managed_resume_upload_cleanup_path(
        Some(managed_resume.to_string_lossy().as_ref()),
        managed_dir.path(),
    )
    .expect("managed upload should be eligible for cleanup");

    std::fs::remove_file(cleanup_path).unwrap();
    assert!(!managed_resume.exists());
}

#[tokio::test]
async fn delete_resume_with_file_cleanup_removes_managed_upload_after_database_delete() {
    let database = crate::desktop::Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();

    let managed_dir = tempfile::tempdir().unwrap();
    let token = "7d9d16a1-2e5d-4b32-9eb2-bfbffb4ee871--Jordan-Resume.txt";
    let managed_resume = managed_dir.path().join(token);
    std::fs::write(&managed_resume, b"resume text").unwrap();

    let matcher = database.resume_matcher();
    let resume_id = matcher
        .upload_resume("Jordan Resume", managed_resume.to_string_lossy().as_ref())
        .await
        .unwrap();
    delete_resume_with_file_cleanup(resume_id, &matcher, managed_dir.path())
        .await
        .unwrap();

    assert!(matcher.get_resume(resume_id).await.is_err());
    assert!(!managed_resume.exists());
}

#[test]
fn selected_resume_validation_rejects_oversized_file_without_path_leak() {
    let temp_dir = tempfile::tempdir().unwrap();
    let resume_path = temp_dir.path().join("Private Large Resume.pdf");
    let file = std::fs::File::create(&resume_path).unwrap();
    file.set_len(MAX_RESUME_FILE_BYTES + 1).unwrap();

    let err = validate_selected_resume(&resume_path).unwrap_err();

    assert!(err.contains("too large"));
    assert!(!err.contains(temp_dir.path().to_string_lossy().as_ref()));
    assert!(!err.contains("Private Large Resume"));
}
