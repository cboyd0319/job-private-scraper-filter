/** Registers browser-development handlers for the desktop command surface. */

import {
  applyMockApplicationAssistCommand,
  applyMockApplicationsCommand,
  applyMockCoverLetterTemplateCommand,
  applyMockDashboardCommand,
  applyMockExternalAiCommand,
  applyMockInterviewCommand,
  applyMockJobImportCommand,
  applyMockLinkedInCommand,
  applyMockMarketCommand,
  applyMockNotificationCommand,
  applyMockOnboardingCommand,
  applyMockOpportunityCaseCommand,
  applyMockPackCommand,
  applyMockRecoveryCommand,
  applyMockResumeCommand,
  applyMockSalaryCommand,
  applyMockSavedSearchCommand,
  applyMockSearchLinksCommand,
  applyMockSettingsCommand,
  applyMockSourceHealthCommand,
  applyMockSupportCommand,
} from "./runtimeCommandAdapters";
import type { MockCommandAdapter } from "./runtimeCommandAdapters";

interface MockCommandGroup {
  commands: readonly string[];
  adapter: MockCommandAdapter;
}

const commandGroups: readonly MockCommandGroup[] = [
  {
    commands: [
      "get_jobs",
      "get_job",
      "hide_job",
      "unhide_job",
      "toggle_bookmark",
      "get_bookmarked_jobs",
      "set_job_notes",
      "mark_job_as_real",
      "mark_job_as_ghost",
      "get_job_notes",
      "get_statistics",
      "get_recent_jobs",
      "get_scraping_status",
      "search_jobs",
    ],
    adapter: applyMockDashboardCommand,
  },
  {
    commands: ["is_first_run", "complete_setup"],
    adapter: applyMockOnboardingCommand,
  },
  {
    commands: [
      "get_config",
      "get_dashboard_preferences",
      "get_resume_matching_preference",
      "set_resume_matching_enabled",
      "save_config",
      "get_credential_status",
      "has_credential",
      "store_credential",
      "get_credential_unlock_status",
      "enable_credential_passphrase",
      "unlock_credential_vault",
      "disable_credential_passphrase",
      "disconnect_linkedin",
      "linkedin_login",
      "get_linkedin_expiry_status",
      "detect_location",
      "get_ghost_config",
      "set_ghost_config",
      "reset_ghost_config",
      "validate_slack_webhook",
      "test_email_notification",
      "get_bookmarklet_config",
      "get_pending_bookmarklet_imports",
      "confirm_pending_bookmarklet_imports",
      "discard_pending_bookmarklet_imports",
      "copy_bookmarklet_code",
      "start_bookmarklet_server",
      "stop_bookmarklet_server",
      "set_bookmarklet_port",
      "get_semantic_matching_diagnostics",
      "download_ml_model",
      "cancel_ml_model_download",
      "remove_ml_models",
      "repair_semantic_matching_model_cache",
    ],
    adapter: applyMockSettingsCommand,
  },
  {
    commands: [
      "prepare_external_ai_request",
      "send_external_ai_request",
      "cancel_external_ai_request",
      "list_external_ai_activity",
    ],
    adapter: applyMockExternalAiCommand,
  },
  {
    commands: [
      "get_system_info",
      "get_config_summary",
      "get_debug_log_events",
      "generate_feedback_report",
      "sanitize_feedback_text",
      "get_feedback_filename",
      "save_feedback_file",
      "open_github_issues",
      "reveal_saved_feedback_file",
    ],
    adapter: applyMockSupportCommand,
  },
  {
    commands: [
      "get_local_recovery_report",
      "run_local_storage_cleanup",
      "repair_local_permissions",
      "repair_invalid_startup_config",
      "create_portable_backup",
      "stage_portable_restore",
      "get_staged_restore_status",
      "cancel_staged_restore",
      "create_reviewed_export",
    ],
    adapter: applyMockRecoveryCommand,
  },
  {
    commands: [
      "get_supported_sites",
      "get_sites_by_category_cmd",
      "generate_deep_links",
      "generate_deep_link",
      "open_deep_link",
    ],
    adapter: applyMockSearchLinksCommand,
  },
  {
    commands: [
      "preview_job_import",
      "confirm_job_import",
      "preview_smart_paste",
      "confirm_smart_paste",
    ],
    adapter: applyMockJobImportCommand,
  },
  {
    commands: [
      "get_linkedin_workbench_review_status",
      "review_linkedin_workbench",
      "revoke_linkedin_workbench_review",
      "record_linkedin_workbench_event",
    ],
    adapter: applyMockLinkedInCommand,
  },
  {
    commands: [
      "get_applications_kanban",
      "create_application",
      "update_application_status",
      "add_application_notes",
      "get_pending_reminders",
      "complete_reminder",
      "detect_ghosted_applications",
      "get_application_stats",
      "get_jobs_by_source",
      "get_salary_distribution",
      "get_upcoming_interviews",
      "get_past_interviews",
      "schedule_interview",
      "complete_interview",
      "delete_interview",
      "find_duplicates",
      "merge_duplicates",
    ],
    adapter: applyMockApplicationsCommand,
  },
  {
    commands: ["open_opportunity_case"],
    adapter: applyMockOpportunityCaseCommand,
  },
  {
    commands: [
      "list_pack_management",
      "disable_pack",
      "uninstall_pack",
      "retry_pack_cleanup",
    ],
    adapter: applyMockPackCommand,
  },
  {
    commands: [
      "get_active_resume",
      "list_all_resumes",
      "get_resume_text_preview",
      "set_active_resume",
      "select_and_upload_resume",
      "import_json_resume",
      "select_and_import_json_resume",
      "delete_resume",
      "get_user_skills",
      "add_user_skill",
      "update_user_skill",
      "delete_user_skill",
      "get_recent_matches",
      "set_resume_match_feedback",
      "get_saved_match_debugger",
      "confirm_saved_match_evidence",
      "list_saved_match_evidence_packets",
      "save_saved_match_evidence_packet",
      "confirm_saved_match_military_evidence",
      "prepare_saved_match_military_transition_review",
      "confirm_saved_match_military_transition_review",
      "match_resume_to_job",
      "create_resume_draft",
      "get_resume_draft",
      "update_resume_contact",
      "update_resume_summary",
      "add_resume_experience",
      "delete_resume_experience",
      "add_resume_education",
      "delete_resume_education",
      "set_resume_skills",
      "delete_resume_draft",
      "list_resume_templates",
      "render_resume_html",
      "analyze_resume_format",
      "analyze_resume_for_job",
      "analyze_active_resume_for_job",
      "get_ats_power_words",
      "improve_bullet_point",
      "export_resume_docx",
      "export_resume_html",
      "export_resume_text",
    ],
    adapter: applyMockResumeCommand,
  },
  {
    commands: [
      "predict_salary",
      "get_salary_benchmark",
      "generate_negotiation_script",
    ],
    adapter: applyMockSalaryCommand,
  },
  {
    commands: [
      "get_trending_skills",
      "get_active_companies",
      "get_hottest_locations",
      "get_market_alerts",
      "get_market_snapshot",
      "run_market_analysis",
      "mark_alert_read",
      "mark_all_alerts_read",
    ],
    adapter: applyMockMarketCommand,
  },
  {
    commands: [
      "has_application_profile",
      "get_application_profile_preview",
      "get_application_profile",
      "select_application_resume_file",
      "upsert_application_profile",
      "get_screening_answers",
      "get_application_screening_answer_previews",
      "upsert_screening_answer",
      "get_automation_stats",
      "detect_ats_platform",
      "fill_application_form",
      "is_browser_running",
      "close_automation_browser",
      "mark_attempt_submitted",
      "get_suggested_answers",
    ],
    adapter: applyMockApplicationAssistCommand,
  },
  {
    commands: [
      "get_health_summary",
      "get_scraper_health",
      "set_scraper_enabled",
      "get_scraper_runs",
      "get_latest_source_request",
      "run_scraper_smoke_test",
      "run_all_smoke_tests",
    ],
    adapter: applyMockSourceHealthCommand,
  },
  {
    commands: [
      "get_interview_prep_checklist",
      "save_interview_prep_item",
      "get_interview_followup",
      "save_interview_followup",
    ],
    adapter: applyMockInterviewCommand,
  },
  {
    commands: [
      "seed_default_templates",
      "list_cover_letter_templates",
      "get_cover_letter_template",
      "create_cover_letter_template",
      "update_cover_letter_template",
      "delete_cover_letter_template",
      "import_cover_letter_templates",
    ],
    adapter: applyMockCoverLetterTemplateCommand,
  },
  {
    commands: ["get_notification_preferences", "save_notification_preferences"],
    adapter: applyMockNotificationCommand,
  },
  {
    commands: [
      "get_search_history",
      "list_saved_searches",
      "create_saved_search",
      "use_saved_search",
      "delete_saved_search",
      "import_saved_searches",
      "add_search_history",
      "clear_search_history",
    ],
    adapter: applyMockSavedSearchCommand,
  },
];

function buildCommandRegistry(): ReadonlyMap<string, MockCommandAdapter> {
  const registry = new Map<string, MockCommandAdapter>();
  for (const group of commandGroups) {
    for (const command of group.commands) {
      if (registry.has(command)) {
        throw new Error(`Duplicate mock command registration: ${command}`);
      }
      registry.set(command, group.adapter);
    }
  }
  return registry;
}

const commandRegistry = buildCommandRegistry();

export const registeredMockCommands = Object.freeze([
  ...commandRegistry.keys(),
]);

export function invokeRegisteredMockCommand<T>(
  command: string,
  args?: Record<string, unknown>,
): T {
  return commandRegistry.get(command)?.(command, args) as T;
}
