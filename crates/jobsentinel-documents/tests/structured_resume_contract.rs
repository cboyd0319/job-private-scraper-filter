use jobsentinel_documents::{
    AtsAnalyzer, ResumeAnalysisInput, ResumeExporter, StructuredResume, TemplateId,
    TemplateRenderer,
};

const STRUCTURED_RESUME_FIXTURE: &str = include_str!("fixtures/structured_resume.json");

#[test]
fn structured_resume_fixture_round_trips_without_field_drift() {
    let resume: StructuredResume = serde_json::from_str(STRUCTURED_RESUME_FIXTURE).unwrap();
    let serialized = serde_json::to_value(&resume).unwrap();
    let fixture: serde_json::Value = serde_json::from_str(STRUCTURED_RESUME_FIXTURE).unwrap();

    assert_eq!(serialized, fixture);
    assert_eq!(
        serde_json::from_value::<StructuredResume>(serialized).unwrap(),
        resume
    );
}

#[test]
fn template_id_supports_rendering_ids_and_legacy_export_aliases() {
    let cases = [
        ("classic", TemplateId::Classic, "\"Classic\""),
        ("modern", TemplateId::Modern, "\"Modern\""),
        ("technical", TemplateId::Technical, "\"Technical\""),
        ("executive", TemplateId::Executive, "\"Executive\""),
        ("military", TemplateId::Military, "\"Military\""),
        ("professional", TemplateId::Professional, "\"Professional\""),
        ("traditional", TemplateId::Traditional, "\"Traditional\""),
    ];

    for (input, expected, serialized) in cases {
        assert_eq!(input.parse::<TemplateId>().unwrap(), expected);
        assert_eq!(expected.as_str(), input);
        assert_eq!(serde_json::to_string(&expected).unwrap(), serialized);
        assert_eq!(
            serde_json::from_str::<TemplateId>(serialized).unwrap(),
            expected
        );
    }

    assert_eq!(TemplateId::Professional.rendering_id(), TemplateId::Classic);
    assert_eq!(
        TemplateId::Traditional.rendering_id(),
        TemplateId::Executive
    );
    assert!("unknown".parse::<TemplateId>().is_err());
}

#[test]
fn renderer_and_html_export_share_the_canonical_model() {
    let resume: StructuredResume = serde_json::from_str(STRUCTURED_RESUME_FIXTURE).unwrap();

    assert_eq!(
        ResumeExporter::export_html(&resume, TemplateId::Professional),
        TemplateRenderer::render_html(&resume, TemplateId::Classic)
    );
}

#[test]
fn exporters_accept_the_canonical_model() {
    let resume: StructuredResume = serde_json::from_str(STRUCTURED_RESUME_FIXTURE).unwrap();

    assert!(ResumeExporter::export_text(&resume).contains("Jordan Lee"));
    assert!(ResumeExporter::export_docx(&resume, TemplateId::Professional).is_ok());
}

#[test]
fn analysis_accepts_the_canonical_model_with_boundary_fields() {
    let resume: StructuredResume = serde_json::from_str(STRUCTURED_RESUME_FIXTURE).unwrap();
    let input = ResumeAnalysisInput {
        resume,
        custom_sections: Default::default(),
        evidence_snapshot: None,
    };

    assert!(AtsAnalyzer::analyze_format(&input).completeness_score > 0.0);
}
