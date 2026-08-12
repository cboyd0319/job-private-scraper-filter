use crate::config::types::Config;
use crate::config::validation::is_valid_email;
use crate::config::validation_error::{ValidationError, ValidationErrors};

/// Validate scraper configurations
pub(super) fn validate_scrapers(config: &Config, errors: &mut ValidationErrors) {
    const MAX_SCRAPER_LIMIT: usize = 1000;
    const MAX_QUERY_LENGTH: usize = 200;
    const MAX_LOCATION_LENGTH: usize = 100;
    const MAX_EMAIL_LENGTH: usize = 100;

    // Validate RemoteOK scraper
    if config.remoteok.enabled
        && (config.remoteok.limit == 0 || config.remoteok.limit > MAX_SCRAPER_LIMIT)
    {
        errors.add(ValidationError::out_of_range(
            "remoteok.limit",
            config.remoteok.limit,
            Some(1_usize),
            Some(MAX_SCRAPER_LIMIT),
        ));
    }

    // Validate WeWorkRemotely scraper
    if config.weworkremotely.enabled
        && (config.weworkremotely.limit == 0 || config.weworkremotely.limit > MAX_SCRAPER_LIMIT)
    {
        errors.add(ValidationError::out_of_range(
            "weworkremotely.limit",
            config.weworkremotely.limit,
            Some(1_usize),
            Some(MAX_SCRAPER_LIMIT),
        ));
    }

    // Validate HN Hiring scraper
    if config.hn_hiring.enabled
        && (config.hn_hiring.limit == 0 || config.hn_hiring.limit > MAX_SCRAPER_LIMIT)
    {
        errors.add(ValidationError::out_of_range(
            "hn_hiring.limit",
            config.hn_hiring.limit,
            Some(1_usize),
            Some(MAX_SCRAPER_LIMIT),
        ));
    }

    // Validate USAJobs scraper
    if config.usajobs.enabled {
        if config.usajobs.email.is_empty() {
            errors.add(ValidationError::required_field(
                "usajobs.email",
                "required when USAJobs scraper is enabled (used in User-Agent header)",
            ));
        } else if !is_valid_email(&config.usajobs.email) {
            errors.add(ValidationError::invalid_email(
                "usajobs.email",
                &config.usajobs.email,
            ));
        } else if config.usajobs.email.len() > MAX_EMAIL_LENGTH {
            errors.add(ValidationError::too_long(
                "usajobs.email",
                config.usajobs.email.len(),
                MAX_EMAIL_LENGTH,
            ));
        }

        if let Some(keywords) = &config.usajobs.keywords {
            if keywords.len() > MAX_QUERY_LENGTH {
                errors.add(ValidationError::too_long(
                    "usajobs.keywords",
                    keywords.len(),
                    MAX_QUERY_LENGTH,
                ));
            }
        }

        if let Some(location) = &config.usajobs.location {
            if location.len() > MAX_LOCATION_LENGTH {
                errors.add(ValidationError::too_long(
                    "usajobs.location",
                    location.len(),
                    MAX_LOCATION_LENGTH,
                ));
            }
        }

        // Validate GS pay grades (1-15)
        if let Some(grade) = config.usajobs.pay_grade_min {
            if !(1..=15).contains(&grade) {
                errors.add(ValidationError::out_of_range(
                    "usajobs.pay_grade_min",
                    grade,
                    Some(1_u8),
                    Some(15_u8),
                ));
            }
        }

        if let Some(grade) = config.usajobs.pay_grade_max {
            if !(1..=15).contains(&grade) {
                errors.add(ValidationError::out_of_range(
                    "usajobs.pay_grade_max",
                    grade,
                    Some(1_u8),
                    Some(15_u8),
                ));
            }
        }

        // Validate pay grade consistency
        if let (Some(min), Some(max)) = (config.usajobs.pay_grade_min, config.usajobs.pay_grade_max)
        {
            if min > max {
                errors.add(ValidationError::inconsistent_values(
                    "usajobs.pay_grade_min",
                    "usajobs.pay_grade_max",
                    format!("pay_grade_min ({}) must be <= pay_grade_max ({})", min, max),
                ));
            }
        }

        // Validate date_posted_days (1-60)
        if !(1..=60).contains(&config.usajobs.date_posted_days) {
            errors.add(ValidationError::out_of_range(
                "usajobs.date_posted_days",
                config.usajobs.date_posted_days,
                Some(1_u8),
                Some(60_u8),
            ));
        }

        if config.usajobs.limit == 0 || config.usajobs.limit > MAX_SCRAPER_LIMIT {
            errors.add(ValidationError::out_of_range(
                "usajobs.limit",
                config.usajobs.limit,
                Some(1_usize),
                Some(MAX_SCRAPER_LIMIT),
            ));
        }
    }
}
