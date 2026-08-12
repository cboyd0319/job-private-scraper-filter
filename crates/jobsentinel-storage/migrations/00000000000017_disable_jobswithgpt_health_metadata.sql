UPDATE scraper_config
SET is_enabled = 0,
    updated_at = datetime('now')
WHERE scraper_name = 'jobswithgpt';

CREATE TRIGGER jobswithgpt_health_metadata_stays_disabled
BEFORE UPDATE OF is_enabled ON scraper_config
WHEN OLD.scraper_name = 'jobswithgpt' AND NEW.is_enabled IS NOT 0
BEGIN
    SELECT RAISE(ABORT, 'jobswithgpt provider review required');
END;

UPDATE v3_compatibility_metadata
SET migration_version = 17
WHERE singleton = 1;
