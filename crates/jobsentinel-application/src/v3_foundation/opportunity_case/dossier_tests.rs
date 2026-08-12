//! Proves employer dossiers expose sourced facts and explicit unknowns without verdicts.

use chrono::{NaiveDate, TimeZone, Utc};

use super::dossier::{
    EmployerApplicationChannel, EmployerDossierNextAction, EmployerDossierUncertainty,
    EmployerIdentityStatus, EmployerPayClarity, EmployerRoleStatus, EmployerSourceStatus,
};
use super::open_opportunity_case_at;
use crate::{
    test_support::test_job,
    v3_source_governance::{install_greenhouse, install_lever},
};
use jobsentinel_domain::{v3_manifests::SourceClass, v3_source_manifest::SalaryCoverage};
use jobsentinel_storage::Database;

#[tokio::test]
async fn dossier_separates_current_public_ats_evidence_from_unverified_employer_identity() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    install_greenhouse(&database).await.unwrap();
    let mut job = test_job("dossier-current", "Care Coordinator", "Community Care");
    job.source = "greenhouse".to_string();
    job.url = "https://job-boards.greenhouse.io/community-care/jobs/123".to_string();
    job.first_seen = Some(Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap());
    job.last_seen = Utc.with_ymd_and_hms(2026, 8, 10, 0, 0, 0).unwrap();
    job.times_seen = 4;
    job.repost_count = 2;
    job.salary_min = Some(100_000);
    job.salary_max = Some(140_000);
    job.currency = Some("USD".to_string());
    database.insert_job_if_new(&job).await.unwrap();

    let snapshot = open_opportunity_case_at(
        &database,
        &job.hash,
        60,
        NaiveDate::from_ymd_opt(2026, 8, 12).unwrap(),
    )
    .await
    .unwrap();
    let dossier = snapshot.employer_dossier;

    assert_eq!(dossier.employer.name, "Community Care");
    assert_eq!(
        dossier.employer.identity_status,
        EmployerIdentityStatus::UnverifiedSavedName
    );
    assert_eq!(dossier.employer.official_domain, None);
    assert_eq!(
        dossier.employer.posting_domain.as_deref(),
        Some("job-boards.greenhouse.io")
    );
    assert_eq!(dossier.role.status, EmployerRoleStatus::LastObserved);
    assert_eq!(dossier.role.times_seen, 4);
    assert_eq!(dossier.role.repost_count, 2);
    assert_eq!(dossier.source.status, EmployerSourceStatus::Current);
    assert_eq!(dossier.source.source_class, Some(SourceClass::PublicAts));
    assert_eq!(
        dossier.source.verified_on,
        Some(NaiveDate::from_ymd_opt(2026, 8, 12).unwrap())
    );
    assert_eq!(
        dossier.source.expires_on,
        Some(NaiveDate::from_ymd_opt(2026, 9, 11).unwrap())
    );
    assert_eq!(dossier.source.salary_coverage, Some(SalaryCoverage::None));
    assert_eq!(dossier.pay.clarity, EmployerPayClarity::RangeListed);
    assert_eq!(dossier.pay.currency.as_deref(), Some("USD"));
    assert_eq!(
        dossier.application_channel,
        EmployerApplicationChannel::PublicAts
    );
    assert!(dossier
        .uncertainty
        .contains(&EmployerDossierUncertainty::OfficialDomainUnknown));
    assert!(dossier
        .uncertainty
        .contains(&EmployerDossierUncertainty::RoleNotLiveChecked));
    assert_eq!(
        dossier.next_action,
        EmployerDossierNextAction::OpenSavedPosting
    );
    let serialized = serde_json::to_string(&dossier).unwrap();
    for forbidden in [
        "employer_score",
        "hiring_probability",
        "fraud",
        "scam",
        "legal_compliance",
    ] {
        assert!(!serialized.contains(forbidden), "invented {forbidden}");
    }
}

#[tokio::test]
async fn dossier_retains_stale_observation_and_marks_missing_source_and_unusable_pay() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    install_greenhouse(&database).await.unwrap();
    let mut stale = test_job("dossier-stale", "Analyst", "Example");
    stale.source = "greenhouse".to_string();
    stale.url = "https://job-boards.greenhouse.io/example/jobs/1".to_string();
    stale.salary_min = Some(140_000);
    stale.salary_max = Some(100_000);
    stale.currency = Some("USD".to_string());
    database.insert_job_if_new(&stale).await.unwrap();

    let stale_snapshot = open_opportunity_case_at(
        &database,
        &stale.hash,
        60,
        NaiveDate::from_ymd_opt(2026, 9, 12).unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(
        stale_snapshot.employer_dossier.source.status,
        EmployerSourceStatus::Stale
    );
    assert_eq!(
        stale_snapshot.employer_dossier.pay.clarity,
        EmployerPayClarity::Unusable
    );
    assert_eq!(
        stale_snapshot.employer_dossier.next_action,
        EmployerDossierNextAction::VerifyOnEmployerCareersPage
    );
    assert_eq!(
        stale_snapshot.employer_dossier.role.status,
        EmployerRoleStatus::LastObserved
    );

    let missing_database = Database::connect_memory().await.unwrap();
    missing_database.migrate().await.unwrap();
    let mut missing = test_job("dossier-missing", "Analyst", "Example");
    missing.source = "unreviewed-source".to_string();
    missing_database.insert_job_if_new(&missing).await.unwrap();
    let missing_snapshot = open_opportunity_case_at(
        &missing_database,
        &missing.hash,
        60,
        NaiveDate::from_ymd_opt(2026, 8, 12).unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(
        missing_snapshot.employer_dossier.source.status,
        EmployerSourceStatus::Missing
    );
    assert_eq!(
        missing_snapshot.employer_dossier.application_channel,
        EmployerApplicationChannel::Unknown
    );
    assert_eq!(
        missing_snapshot.employer_dossier.pay.clarity,
        EmployerPayClarity::NotListed
    );
}

#[tokio::test]
async fn dossier_emits_only_the_canonical_ats_link_without_query_or_fragment_data() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    install_greenhouse(&database).await.unwrap();
    let mut job = test_job("dossier-tokenized-link", "Analyst", "Example");
    job.source = "greenhouse".to_string();
    job.url =
        "https://job-boards.greenhouse.io/example/jobs/1?token=private#application".to_string();
    database.insert_job_if_new(&job).await.unwrap();

    let snapshot = open_opportunity_case_at(
        &database,
        &job.hash,
        60,
        NaiveDate::from_ymd_opt(2026, 8, 12).unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(
        snapshot.employer_dossier.role.posting_url.as_deref(),
        Some("https://job-boards.greenhouse.io/example/jobs/1")
    );
    assert_eq!(
        snapshot.employer_dossier.employer.posting_domain.as_deref(),
        Some("job-boards.greenhouse.io")
    );
    assert_eq!(
        snapshot.employer_dossier.next_action,
        EmployerDossierNextAction::OpenSavedPosting
    );
    assert!(!serde_json::to_string(&snapshot.employer_dossier)
        .unwrap()
        .contains("private"));
}

#[tokio::test]
async fn dossier_rejects_an_unreviewed_public_host_with_a_greenhouse_shaped_job_id() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    install_greenhouse(&database).await.unwrap();
    let mut job = test_job("dossier-unreviewed-host", "Account Executive", "Example");
    job.source = "greenhouse".to_string();
    job.url = "https://evil.example/careers/job?gh_jid=7687193003".to_string();
    database.insert_job_if_new(&job).await.unwrap();

    let snapshot = open_opportunity_case_at(
        &database,
        &job.hash,
        60,
        NaiveDate::from_ymd_opt(2026, 8, 12).unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(snapshot.employer_dossier.role.posting_url, None);
    assert_eq!(snapshot.employer_dossier.employer.posting_domain, None);
    assert_eq!(
        snapshot.employer_dossier.next_action,
        EmployerDossierNextAction::VerifyOnEmployerCareersPage
    );
}

#[tokio::test]
async fn dossier_rejects_reviewed_ats_hosts_on_unreviewed_ports() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    install_greenhouse(&database).await.unwrap();
    install_lever(&database).await.unwrap();

    for (hash, source, url) in [
        (
            "dossier-greenhouse-port",
            "greenhouse",
            "https://job-boards.greenhouse.io:8443/example/jobs/1",
        ),
        (
            "dossier-lever-port",
            "lever",
            "https://jobs.lever.co:8443/example/00000000-0000-4000-8000-000000000001",
        ),
    ] {
        let mut job = test_job(hash, "Analyst", "Example");
        job.source = source.to_string();
        job.url = url.to_string();
        database.insert_job_if_new(&job).await.unwrap();

        let snapshot = open_opportunity_case_at(
            &database,
            &job.hash,
            60,
            NaiveDate::from_ymd_opt(2026, 8, 12).unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(snapshot.employer_dossier.role.posting_url, None);
        assert_eq!(snapshot.employer_dossier.employer.posting_domain, None);
        assert_eq!(
            snapshot.employer_dossier.next_action,
            EmployerDossierNextAction::VerifyOnEmployerCareersPage
        );
    }
}
