-- A newer authenticated release restores an uninstalled stream to quarantine
-- while advancing its immutable downgrade high-water mark once.

DROP TRIGGER v3_pack_releases_advance_high_water;

CREATE TRIGGER v3_pack_releases_advance_high_water
AFTER INSERT ON v3_pack_releases
BEGIN
    UPDATE v3_pack_streams
    SET high_water_sequence = NEW.release_sequence,
        high_water_signed_release_sha256 = NEW.signed_release_sha256,
        availability = CASE
            WHEN availability = 'removed' THEN 'quarantined'
            ELSE availability
        END,
        generation = generation + 1,
        updated_at = NEW.updated_at
    WHERE publisher_key_id = NEW.publisher_key_id
      AND pack_id = NEW.pack_id;
END;

UPDATE v3_compatibility_metadata
SET migration_version = 24
WHERE singleton = 1;
