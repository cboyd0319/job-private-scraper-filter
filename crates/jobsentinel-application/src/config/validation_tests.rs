//! Additional validation tests for comprehensive config validation

#[cfg(test)]
mod validation_tests {
    use crate::config::{
        types::*, validation::validate_config, ExternalAiProvider, ValidationError,
        ValidationErrors,
    };

    fn validation_error_fields(result: Result<(), Box<dyn std::error::Error>>) -> Vec<String> {
        let validation_errors = result
            .unwrap_err()
            .downcast::<ValidationErrors>()
            .expect("validation should return structured errors");

        validation_errors
            .errors()
            .iter()
            .flat_map(|error| match error {
                ValidationError::OutOfRange { field, .. }
                | ValidationError::InvalidValue { field, .. }
                | ValidationError::RequiredField { field, .. }
                | ValidationError::TooLong { field, .. }
                | ValidationError::TooManyElements { field, .. }
                | ValidationError::InvalidUrl { field, .. }
                | ValidationError::InvalidEmail { field, .. }
                | ValidationError::EmptyString { field } => vec![field.clone()],
                ValidationError::InconsistentValues { field1, field2, .. } => {
                    vec![field1.clone(), field2.clone()]
                }
            })
            .collect()
    }

    fn create_minimal_valid_config() -> Config {
        crate::test_support::minimal_test_config()
    }

    #[test]
    fn test_salary_target_less_than_floor_fails() {
        let mut config = create_minimal_valid_config();
        config.salary_floor_usd = 150000;
        config.salary_target_usd = Some(100000);

        let result = validate_config(&config);
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains("salary_target_usd"));
    }

    #[test]
    fn test_salary_target_equal_to_floor_passes() {
        let mut config = create_minimal_valid_config();
        config.salary_floor_usd = 150000;
        config.salary_target_usd = Some(150000);

        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_no_location_types_enabled_fails() {
        let mut config = create_minimal_valid_config();
        config.location_preferences.allow_remote = false;
        config.location_preferences.allow_hybrid = false;
        config.location_preferences.allow_onsite = false;

        let result = validate_config(&config);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("at least one of allow_remote"));
    }

    #[test]
    fn test_usajobs_pay_grade_consistency() {
        let mut config = create_minimal_valid_config();
        config.usajobs.enabled = true;
        config.usajobs.email = "test@example.com".to_string();
        config.usajobs.pay_grade_min = Some(10);
        config.usajobs.pay_grade_max = Some(5);

        let result = validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("pay_grade"));
    }

    #[test]
    fn test_usajobs_pay_grade_out_of_range() {
        let mut config = create_minimal_valid_config();
        config.usajobs.enabled = true;
        config.usajobs.email = "test@example.com".to_string();
        config.usajobs.pay_grade_min = Some(16); // GS grades are 1-15

        let result = validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_usajobs_date_posted_out_of_range() {
        let mut config = create_minimal_valid_config();
        config.usajobs.enabled = true;
        config.usajobs.email = "test@example.com".to_string();
        config.usajobs.date_posted_days = 61; // Max is 60

        let result = validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn retired_yc_startup_config_stays_loadable() {
        let mut config = create_minimal_valid_config();
        config.yc_startup.enabled = true;

        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_discord_invalid_user_id_format() {
        let mut config = create_minimal_valid_config();
        config.alerts.discord.enabled = true;
        config.alerts.discord.user_id_to_mention = Some("not_a_number".to_string());

        let result = validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("numeric"));
    }

    #[test]
    fn test_discord_valid_user_id_passes() {
        let mut config = create_minimal_valid_config();
        config.alerts.discord.enabled = true;
        config.alerts.discord.user_id_to_mention = Some("123456789012345678".to_string());

        // Should pass (discord validation doesn't block other errors)
        // Note: webhook_url validation happens at runtime when fetching from keyring
        let result = validate_config(&config);
        // May still fail on other fields, but not on user_id format
        if let Err(e) = result {
            assert!(!e.to_string().contains("user_id"));
        }
    }

    #[test]
    fn test_external_ai_requires_enabled_provider_and_preview_guards() {
        let mut config = create_minimal_valid_config();
        config.external_ai.enabled = true;
        config.external_ai.require_payload_preview = false;
        config.external_ai.redaction.enabled = false;

        let result = validate_config(&config);

        assert!(result.is_err());
        let fields = validation_error_fields(result);
        assert!(fields.contains(&"external_ai.enabled_providers".to_string()));
        assert!(fields.contains(&"external_ai.provider".to_string()));
        assert!(fields.contains(&"external_ai.require_payload_preview".to_string()));
        assert!(fields.contains(&"external_ai.redaction.enabled".to_string()));
    }

    #[test]
    fn test_external_ai_custom_provider_requires_public_https_endpoint() {
        for endpoint in [
            "http://127.0.0.1:9000/model",
            "https://provider.example/model?candidate=private",
            "https://provider.example/model#private",
        ] {
            let mut config = create_minimal_valid_config();
            config.external_ai.enabled = true;
            config.external_ai.provider = ExternalAiProvider::Custom;
            config.external_ai.enabled_providers = vec![ExternalAiProvider::Custom];
            config.external_ai.provider_order = vec![ExternalAiProvider::Custom];
            config.external_ai.custom_endpoint = endpoint.to_string();

            let result = validate_config(&config);

            assert!(result.is_err());
            let fields = validation_error_fields(result);
            assert!(fields.contains(&"external_ai.custom_endpoint".to_string()));
        }
    }

    #[test]
    fn test_company_lists_too_large() {
        let mut config = create_minimal_valid_config();
        config.preferred_companies = (0..501).map(|i| format!("Company {}", i)).collect();

        let result = validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Too many"));
    }

    #[test]
    fn test_invalid_jobswithgpt_endpoint() {
        let mut config = create_minimal_valid_config();
        config.jobswithgpt_endpoint = "not a url".to_string();

        let result = validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_jobswithgpt_endpoint_blocks_private_networks() {
        for endpoint in [
            "http://api.jobswithgpt.com/mcp",
            "http://localhost:3000/mcp",
            "http://127.0.0.1:3000/mcp",
            "http://10.0.0.5/mcp",
            "http://192.168.1.5/mcp",
            "http://[::1]/mcp",
            "http://127.0.0.1.nip.io/mcp",
            "http://192.168.1.5.sslip.io/mcp",
            "http://internal.jobs.local/mcp",
            "https://user:pass@api.jobswithgpt.com/mcp",
        ] {
            let mut config = create_minimal_valid_config();
            config.jobswithgpt_endpoint = endpoint.to_string();

            let result = validate_config(&config);
            assert!(result.is_err(), "{endpoint}");
        }
    }

    #[test]
    fn test_empty_jobswithgpt_endpoint() {
        // Empty endpoint is valid — it disables JobsWithGPT scraping
        let mut config = create_minimal_valid_config();
        config.jobswithgpt_endpoint = "".to_string();

        let result = validate_config(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_all_scrapers_disabled_passes() {
        let config = create_minimal_valid_config();
        // All scrapers disabled by default
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_scraper_limit_zero_fails() {
        let mut config = create_minimal_valid_config();
        config.remoteok.enabled = true;
        config.remoteok.limit = 0;

        let result = validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_scraper_limit_too_large_fails() {
        let mut config = create_minimal_valid_config();
        config.weworkremotely.enabled = true;
        config.weworkremotely.limit = 1001;

        let result = validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_validation_errors_collected() {
        let mut config = create_minimal_valid_config();
        // Create multiple errors
        config.salary_floor_usd = -1000; // Error 1: negative salary
        config.immediate_alert_threshold = 5.0; // Error 2: threshold > 1.0
        config.scraping_interval_hours = 0; // Error 3: interval < 1
        config.title_allowlist.push("".to_string()); // Error 4: empty title

        let result = validate_config(&config);
        assert!(result.is_err());

        let err_str = result.unwrap_err().to_string();
        // Should mention multiple errors
        assert!(
            err_str.contains("validation failed") || err_str.contains("negative"),
            "Expected error message to indicate validation failure"
        );
    }

    #[test]
    fn retired_source_fields_remain_loadable_but_inert() {
        let mut config = create_minimal_valid_config();
        config.builtin.enabled = true;
        config.builtin.limit = 0;
        config.dice.enabled = true;
        config.dice.query = "".to_string();
        config.simplyhired.enabled = true;
        config.simplyhired.query = "".to_string();
        config.glassdoor.enabled = true;
        config.glassdoor.query = "".to_string();

        let result = validate_config(&config);
        assert!(result.is_ok());
    }
}
