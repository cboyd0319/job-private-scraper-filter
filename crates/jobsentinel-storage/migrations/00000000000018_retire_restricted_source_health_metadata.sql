DELETE FROM scraper_config
WHERE scraper_name IN ('builtin', 'dice', 'simplyhired', 'glassdoor');

CREATE TRIGGER retired_restricted_source_health_metadata_stays_absent
BEFORE INSERT ON scraper_config
WHEN NEW.scraper_name IN ('builtin', 'dice', 'simplyhired', 'glassdoor')
BEGIN
    SELECT RAISE(ABORT, 'restricted source automation retired after provider policy review');
END;

CREATE TRIGGER active_source_health_metadata_cannot_be_renamed_to_retired
BEFORE UPDATE OF scraper_name ON scraper_config
WHEN NEW.scraper_name IN ('builtin', 'dice', 'simplyhired', 'glassdoor')
BEGIN
    SELECT RAISE(ABORT, 'restricted source automation retired after provider policy review');
END;

UPDATE v3_compatibility_metadata
SET migration_version = 18
WHERE singleton = 1;
