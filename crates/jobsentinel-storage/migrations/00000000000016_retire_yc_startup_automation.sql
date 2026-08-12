DELETE FROM scraper_config
WHERE scraper_name = 'yc_startup';

UPDATE v3_compatibility_metadata
SET migration_version = 16
WHERE singleton = 1;
