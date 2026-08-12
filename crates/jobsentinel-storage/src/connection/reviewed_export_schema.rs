//! Defines the explicit include and exclude contract for reviewed plaintext exports.

use std::collections::BTreeSet;

pub(super) struct ExportTable {
    pub(super) section: &'static str,
    pub(super) table: &'static str,
    pub(super) columns: &'static str,
    pub(super) url_columns: &'static str,
    pub(super) url_or_text_columns: &'static str,
    pub(super) json_url_columns: &'static str,
    pub(super) protected: bool,
    pub(super) protected_json: bool,
    pub(super) filter: &'static str,
}

macro_rules! table {
    ($section:literal, $table:literal, $columns:literal) => {
        ExportTable {
            section: $section,
            table: $table,
            columns: $columns,
            url_columns: "",
            url_or_text_columns: "",
            json_url_columns: "",
            protected: false,
            protected_json: false,
            filter: "",
        }
    };
    ($section:literal, $table:literal, $columns:literal, urls = $urls:literal) => {
        ExportTable {
            section: $section,
            table: $table,
            columns: $columns,
            url_columns: $urls,
            url_or_text_columns: "",
            json_url_columns: "",
            protected: false,
            protected_json: false,
            filter: "",
        }
    };
    ($section:literal, $table:literal, $columns:literal, maybe_urls = $urls:literal) => {
        ExportTable {
            section: $section,
            table: $table,
            columns: $columns,
            url_columns: "",
            url_or_text_columns: $urls,
            json_url_columns: "",
            protected: false,
            protected_json: false,
            filter: "",
        }
    };
    ($section:literal, $table:literal, $columns:literal, json_urls = $urls:literal, protected_json) => {
        ExportTable {
            section: $section,
            table: $table,
            columns: $columns,
            url_columns: "",
            url_or_text_columns: "",
            json_url_columns: $urls,
            protected: false,
            protected_json: true,
            filter: "",
        }
    };
    ($section:literal, $table:literal, $columns:literal, protected) => {
        ExportTable {
            section: $section,
            table: $table,
            columns: $columns,
            url_columns: "",
            url_or_text_columns: "",
            json_url_columns: "",
            protected: true,
            protected_json: false,
            filter: "",
        }
    };
    ($section:literal, $table:literal, $columns:literal, filter = $filter:literal) => {
        ExportTable {
            section: $section,
            table: $table,
            columns: $columns,
            url_columns: "",
            url_or_text_columns: "",
            json_url_columns: "",
            protected: false,
            protected_json: false,
            filter: $filter,
        }
    };
}

pub(super) const EXPORT_TABLES: &[ExportTable] = &[
    table!(
        "applications",
        "application_attempts",
        "id,job_hash,application_id,status,ats_platform,automation_duration_ms,user_approved,submitted_at,created_at"
    ),
    table!(
        "applications",
        "application_events",
        "id,application_id,event_type,event_data,created_at"
    ),
    table!(
        "profile",
        "application_profile",
        "id,full_name,email,phone,linkedin_url,github_url,portfolio_url,website_url,default_resume_id,default_cover_letter_template,max_applications_per_day,require_manual_approval,created_at,updated_at",
        urls = "linkedin_url,github_url,portfolio_url,website_url"
    ),
    table!(
        "protected_answers",
        "application_profile",
        "id,us_work_authorized,requires_sponsorship",
        protected
    ),
    table!(
        "applications",
        "application_reminders",
        "id,application_id,reminder_type,reminder_time,message,completed,completed_at,created_at"
    ),
    table!(
        "applications",
        "applications",
        "id,job_hash,status,applied_at,last_contact,next_followup,notes,resume_version_id,cover_letter_text,recruiter_name,recruiter_email,recruiter_phone,salary_expectation,created_at,updated_at"
    ),
    table!(
        "applications",
        "captcha_challenges",
        "id,application_attempt_id,challenge_type,solved,solved_at,created_at"
    ),
    table!(
        "campaign",
        "career_graph_links",
        "link_id,subject_id,relation,object_id,provenance,provenance_ref,created_at"
    ),
    table!(
        "resume",
        "cover_letter_templates",
        "id,name,content,category,created_at,updated_at"
    ),
    table!(
        "opportunities",
        "ghost_feedback",
        "id,job_id,user_verdict,created_at"
    ),
    table!(
        "applications",
        "interview_followups",
        "id,interview_id,thank_you_sent,sent_at"
    ),
    table!(
        "applications",
        "interview_prep_checklists",
        "id,interview_id,item_id,completed,completed_at"
    ),
    table!(
        "applications",
        "interviews",
        "id,application_id,interview_type,scheduled_at,duration_minutes,interviewer_names,location,preparation_notes,feedback_notes,completed,created_at,interviewer_name,interviewer_title,notes,outcome,updated_at,post_interview_notes",
        maybe_urls = "location"
    ),
    table!(
        "opportunities",
        "job_repost_history",
        "id,job_hash,company,title,source,first_seen,last_seen,repost_count"
    ),
    table!(
        "opportunities",
        "job_salary_predictions",
        "id,job_hash,predicted_min,predicted_max,predicted_median,confidence_score,prediction_method,data_points_used,created_at"
    ),
    table!(
        "opportunities",
        "job_skills",
        "id,job_hash,skill_name,is_required,skill_category,created_at"
    ),
    table!(
        "opportunities",
        "jobs",
        "id,hash,title,company,url,location,description,score,score_reasons,source,remote,salary_min,salary_max,currency,created_at,updated_at,last_seen,times_seen,immediate_alert_sent,included_in_digest,hidden,bookmarked,notes,ghost_score,ghost_reasons,first_seen,repost_count",
        urls = "url"
    ),
    table!(
        "opportunities",
        "market_alerts",
        "id,alert_type,title,description,severity,related_entity,related_entity_type,metric_value,metric_change_pct,is_read,created_at"
    ),
    table!(
        "applications",
        "negotiation_history",
        "id,offer_id,negotiation_round,initial_offer,counter_offer,final_offer,outcome,notes,created_at"
    ),
    table!(
        "applications",
        "negotiation_templates",
        "id,template_name,scenario,template_text,placeholders,is_default,created_at,updated_at"
    ),
    table!(
        "profile",
        "notification_preferences",
        "id,global_enabled,quiet_hours_enabled,quiet_hours_start,quiet_hours_end,source_configs,advanced_filters,updated_at"
    ),
    table!(
        "applications",
        "offers",
        "id,application_id,base_salary,currency,equity_shares,equity_percentage,signing_bonus,annual_bonus,benefits_summary,start_date,offer_received_at,offer_expires_at,accepted,decision_made_at,created_at"
    ),
    table!(
        "campaign",
        "opportunity_case_files",
        "case_file_id,job_hash,created_at"
    ),
    table!(
        "campaign",
        "v3_evidence_packets",
        "packet_id,case_file_id,resume_id,resume_revision,job_revision,claim_id,reviewed_text,local_only,sensitive,created_at"
    ),
    table!(
        "campaign",
        "v3_evidence_packet_evidence",
        "packet_id,ordinal,evidence_id"
    ),
    table!(
        "campaign",
        "v3_evidence_packet_boundaries",
        "packet_id,ordinal,boundary"
    ),
    table!(
        "resume",
        "resume_drafts",
        "id,data,created_at,updated_at",
        json_urls = "data",
        protected_json
    ),
    table!(
        "resume",
        "resume_job_matches",
        "id,resume_id,job_hash,overall_match_score,skills_match_score,experience_match_score,education_match_score,missing_skills,matching_skills,gap_analysis,created_at"
    ),
    table!(
        "resume",
        "resumes",
        "id,name,parsed_text,is_active,created_at,updated_at"
    ),
    table!(
        "opportunities",
        "saved_searches",
        "id,name,sort_by,score_filter,source_filter,remote_filter,bookmark_filter,notes_filter,posted_date_filter,salary_min_filter,salary_max_filter,ghost_filter,text_search,created_at,last_used_at"
    ),
    table!(
        "profile",
        "scoring_config",
        "id,skills_weight,salary_weight,location_weight,company_weight,recency_weight,updated_at"
    ),
    table!(
        "sources",
        "scraper_config",
        "scraper_name,display_name,is_enabled,requires_auth,auth_type,scraper_type,rate_limit_per_hour,selector_health,last_selector_check,created_at,updated_at"
    ),
    table!(
        "protected_answers",
        "screening_answer_history",
        "id,screening_answer_id,question_text,question_normalized,answer_filled,was_modified,modified_to,job_hash,application_attempt_id,created_at",
        protected
    ),
    table!(
        "protected_answers",
        "screening_answers",
        "id,question_pattern,answer,answer_type,notes,created_at,updated_at,times_used,times_modified,last_used_at,confidence_score",
        protected
    ),
    table!(
        "protected_answers",
        "screening_learned_answers",
        "id,question_pattern,source_question_texts,learned_answer,confidence_score,times_used,times_modified,last_used_at,created_at,updated_at",
        protected
    ),
    table!(
        "opportunities",
        "search_history",
        "id,query,use_count,last_used_at"
    ),
    table!(
        "sources",
        "source_graph_links",
        "link_id,source_id,relation,related_id,provenance,provenance_ref,created_at"
    ),
    table!(
        "sources",
        "v3_source_manifests",
        "source_id,policy_ref,policy_revision,manifest_json,created_at,updated_at"
    ),
    table!(
        "resume",
        "user_education",
        "id,resume_id,degree,field_of_study,institution,graduation_year,gpa,created_at"
    ),
    table!(
        "resume",
        "user_experience",
        "id,resume_id,company,title,start_date,end_date,description,is_current,created_at"
    ),
    table!(
        "applications",
        "user_reported_salaries",
        "id,job_title,company,location,base_salary,bonus,equity_value,total_compensation,years_of_experience,seniority_level,is_remote,is_verified,reported_at,created_at"
    ),
    table!(
        "resume",
        "user_skills",
        "id,resume_id,skill_name,skill_category,confidence_score,years_experience,proficiency_level,source,created_at"
    ),
    table!(
        "campaign",
        "v3_job_events",
        "event_id,case_file_id,event_kind,origin,user_action,local_only,sensitive,metadata_json,created_at"
    ),
    table!(
        "privacy_and_recovery",
        "v3_privacy_receipts",
        "receipt_id,schema,receipt_json,stored_locally,data_left_device,created_at"
    ),
    table!(
        "privacy_and_recovery",
        "v3_recovery_operations",
        "operation_id,operation_kind,outcome,source_migration_sequence,target_migration_sequence,error_kind,created_at,completed_at",
        filter = "operation_kind <> 'export'"
    ),
    table!(
        "sources",
        "v3_source_policies",
        "source_id,source_class,access,request_limit_per_hour,user_review_required,policy_ref,revision,restriction_reason_code,reviewed_at,created_at,updated_at"
    ),
];

pub(super) const EXCLUDED_TABLES: &[&str] = &[
    "_sqlx_migrations",
    "app_metadata",
    "ats_form_fields",
    "backup_log",
    "company_green_flags",
    "company_health_scores",
    "company_hiring_velocity",
    "company_profiles",
    "company_red_flags",
    "credential_health",
    "credential_key_wrapping",
    "crunchbase_data",
    "glassdoor_data",
    "h1b_salaries",
    "integrity_check_log",
    "jobs_fts",
    "jobs_fts_config",
    "jobs_fts_data",
    "jobs_fts_docsize",
    "jobs_fts_idx",
    "layoffs_data",
    "linkedin_headcount",
    "location_job_density",
    "market_snapshots",
    "news_sentiment",
    "pack_release_reviews",
    "pack_task_runs",
    "role_demand_trends",
    "salary_benchmarks",
    "salary_trends",
    "scraper_runs",
    "scraper_smoke_tests",
    "secret_vault",
    "skill_demand_trends",
    "source_request_log",
    "v3_compatibility_metadata",
    "v3_local_vectors",
    "v3_outside_ai_operations",
    "v3_pack_publishers",
    "v3_pack_releases",
    "v3_pack_streams",
    "v3_source_consent_events",
    "v3_source_policy_ledger",
];

pub(super) const EXCLUDED_DATA: &[&str] = &[
    "JobSentinel-managed credentials, encryption keys, and authentication storage",
    "dedicated application-managed file paths and connection-link fields",
    "operational diagnostics and cached public or derived datasets",
    "signed pack trust, lifecycle, review, and reviewed-task operation metadata",
];

pub(super) fn validate_export_tables() -> Result<(), sqlx::Error> {
    for table in EXPORT_TABLES {
        validate_table(table)?;
    }
    Ok(())
}

pub(super) fn count_query(table: &ExportTable) -> Result<String, sqlx::Error> {
    validate_table(table)?;
    Ok(format!(
        "SELECT COUNT(*) FROM main.\"{}\"{}",
        table.table,
        where_clause(table)
    ))
}

pub(super) fn export_query(table: &ExportTable) -> Result<String, sqlx::Error> {
    validate_table(table)?;
    let columns = table.columns.split(',').collect::<Vec<_>>();
    let projection = columns
        .iter()
        .map(|column| format!("'{column}', \"{column}\""))
        .collect::<Vec<_>>()
        .join(", ");
    Ok(format!(
        "SELECT json_object({projection}) FROM main.\"{}\"{} ORDER BY \"{}\"",
        table.table,
        where_clause(table),
        columns[0]
    ))
}

fn validate_table(table: &ExportTable) -> Result<(), sqlx::Error> {
    validate_identifier(table.section)?;
    validate_identifier(table.table)?;
    let columns = table.columns.split(',').collect::<BTreeSet<_>>();
    if columns.len() != table.columns.split(',').count() {
        return Err(schema_error());
    }
    for column in &columns {
        validate_identifier(column)?;
    }
    for list in [
        table.url_columns,
        table.url_or_text_columns,
        table.json_url_columns,
    ] {
        for column in list.split(',').filter(|value| !value.is_empty()) {
            validate_identifier(column)?;
            if !columns.contains(column) {
                return Err(schema_error());
            }
        }
    }
    let filter_is_safe = table.filter.is_empty()
        || (table.table == "v3_recovery_operations"
            && table.filter == "operation_kind <> 'export'");
    if !filter_is_safe {
        return Err(schema_error());
    }
    Ok(())
}

fn where_clause(table: &ExportTable) -> String {
    if table.filter.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", table.filter)
    }
}

fn validate_identifier(identifier: &str) -> Result<(), sqlx::Error> {
    if !identifier.is_empty()
        && identifier
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        Ok(())
    } else {
        Err(schema_error())
    }
}

fn schema_error() -> sqlx::Error {
    sqlx::Error::Protocol("Reviewed export schema is invalid".to_string())
}
