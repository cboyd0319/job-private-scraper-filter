-- Signed pack trust and lifecycle state remains local. Composite identities are
-- authoritative; release_id is display metadata and is never a database key.

CREATE TABLE v3_pack_publishers (
    publisher_key_id TEXT PRIMARY KEY CHECK(
        length(CAST(publisher_key_id AS BLOB)) BETWEEN 1 AND 128
        AND publisher_key_id NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    public_key_sha256 TEXT NOT NULL CHECK(
        length(CAST(public_key_sha256 AS BLOB)) = 64
        AND public_key_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    trust_state TEXT NOT NULL CHECK(trust_state IN ('trusted', 'revoked')),
    revoked_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK(
        (trust_state = 'trusted' AND revoked_at IS NULL)
        OR (trust_state = 'revoked' AND revoked_at IS NOT NULL)
    )
);

CREATE TABLE v3_pack_releases (
    publisher_key_id TEXT NOT NULL
        REFERENCES v3_pack_publishers(publisher_key_id) ON DELETE RESTRICT,
    pack_id TEXT NOT NULL CHECK(
        length(CAST(pack_id AS BLOB)) BETWEEN 1 AND 128
        AND pack_id NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    release_sequence INTEGER NOT NULL CHECK(release_sequence > 0),
    release_id TEXT NOT NULL CHECK(
        length(CAST(release_id AS BLOB)) BETWEEN 1 AND 128
        AND release_id NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    signed_release_sha256 TEXT NOT NULL CHECK(
        length(CAST(signed_release_sha256 AS BLOB)) = 64
        AND signed_release_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    payload_sha256 TEXT NOT NULL CHECK(
        length(CAST(payload_sha256 AS BLOB)) = 64
        AND payload_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    pack_version TEXT NOT NULL CHECK(
        length(CAST(pack_version AS BLOB)) BETWEEN 1 AND 64
    ),
    pack_type TEXT NOT NULL CHECK(pack_type IN (
        'skill', 'agent', 'workflow', 'source', 'evaluation'
    )),
    execution_class TEXT NOT NULL CHECK(execution_class IN (
        'static_content', 'reviewed_typed_workflow'
    )),
    lifecycle_state TEXT NOT NULL CHECK(lifecycle_state IN (
        'staged', 'self_tested', 'ready', 'quarantined', 'removed'
    )),
    quarantine_reason TEXT CHECK(quarantine_reason IN (
        'self_test_failed',
        'trust_revoked',
        'interrupted',
        'artifact_missing',
        'integrity_failed'
    )),
    self_tested_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY(publisher_key_id, pack_id, release_sequence),
    CHECK(
        (lifecycle_state = 'staged'
            AND quarantine_reason IS NULL
            AND self_tested_at IS NULL)
        OR (lifecycle_state IN ('self_tested', 'ready')
            AND quarantine_reason IS NULL
            AND self_tested_at IS NOT NULL)
        OR (lifecycle_state = 'quarantined'
            AND quarantine_reason IS NOT NULL)
        OR lifecycle_state = 'removed'
    )
);

CREATE TABLE v3_pack_streams (
    publisher_key_id TEXT NOT NULL
        REFERENCES v3_pack_publishers(publisher_key_id) ON DELETE RESTRICT,
    pack_id TEXT NOT NULL CHECK(
        length(CAST(pack_id AS BLOB)) BETWEEN 1 AND 128
        AND pack_id NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    high_water_sequence INTEGER NOT NULL DEFAULT 0
        CHECK(high_water_sequence >= 0),
    high_water_signed_release_sha256 TEXT CHECK(
        high_water_signed_release_sha256 IS NULL
        OR (
            length(CAST(high_water_signed_release_sha256 AS BLOB)) = 64
            AND high_water_signed_release_sha256 NOT GLOB '*[^0-9a-f]*'
        )
    ),
    active_release_sequence INTEGER,
    rollback_release_sequence INTEGER,
    availability TEXT NOT NULL CHECK(availability IN (
        'ready', 'disabled', 'quarantined', 'removed'
    )),
    generation INTEGER NOT NULL DEFAULT 0 CHECK(generation >= 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY(publisher_key_id, pack_id),
    FOREIGN KEY(publisher_key_id, pack_id, active_release_sequence)
        REFERENCES v3_pack_releases(
            publisher_key_id, pack_id, release_sequence
        ) ON DELETE RESTRICT,
    FOREIGN KEY(publisher_key_id, pack_id, rollback_release_sequence)
        REFERENCES v3_pack_releases(
            publisher_key_id, pack_id, release_sequence
        ) ON DELETE RESTRICT,
    CHECK(
        (high_water_sequence = 0
            AND high_water_signed_release_sha256 IS NULL)
        OR (high_water_sequence > 0
            AND high_water_signed_release_sha256 IS NOT NULL)
    ),
    CHECK(
        (availability IN ('ready', 'disabled')
            AND active_release_sequence IS NOT NULL)
        OR (availability IN ('quarantined', 'removed')
            AND active_release_sequence IS NULL
            AND rollback_release_sequence IS NULL)
    ),
    CHECK(
        active_release_sequence IS NULL
        OR active_release_sequence <= high_water_sequence
    ),
    CHECK(
        rollback_release_sequence IS NULL
        OR rollback_release_sequence <= high_water_sequence
    ),
    CHECK(
        active_release_sequence IS NULL
        OR rollback_release_sequence IS NULL
        OR active_release_sequence <> rollback_release_sequence
    )
);

CREATE INDEX idx_v3_pack_releases_lifecycle
    ON v3_pack_releases(
        lifecycle_state, publisher_key_id, pack_id, release_sequence
    );

CREATE TRIGGER v3_pack_publishers_no_delete
BEFORE DELETE ON v3_pack_publishers
BEGIN
    SELECT RAISE(ABORT, 'pack publisher history is immutable');
END;

CREATE TRIGGER v3_pack_publishers_identity_immutable
BEFORE UPDATE ON v3_pack_publishers
WHEN NEW.publisher_key_id <> OLD.publisher_key_id
    OR NEW.public_key_sha256 <> OLD.public_key_sha256
    OR NEW.created_at <> OLD.created_at
    OR (OLD.trust_state = 'revoked' AND NEW.trust_state <> 'revoked')
    OR (
        NEW.trust_state = 'revoked'
        AND EXISTS (
            SELECT 1
            FROM v3_pack_streams
            WHERE publisher_key_id = OLD.publisher_key_id
              AND availability IN ('ready', 'disabled')
        )
    )
BEGIN
    SELECT RAISE(ABORT, 'pack publisher update is invalid');
END;

CREATE TRIGGER v3_pack_releases_insert_is_new_high_water
BEFORE INSERT ON v3_pack_releases
WHEN NOT EXISTS (
    SELECT 1
    FROM v3_pack_streams
    WHERE publisher_key_id = NEW.publisher_key_id
      AND pack_id = NEW.pack_id
      AND NEW.release_sequence > high_water_sequence
)
BEGIN
    SELECT RAISE(ABORT, 'pack release is not newer');
END;

CREATE TRIGGER v3_pack_releases_advance_high_water
AFTER INSERT ON v3_pack_releases
BEGIN
    UPDATE v3_pack_streams
    SET high_water_sequence = NEW.release_sequence,
        high_water_signed_release_sha256 = NEW.signed_release_sha256,
        generation = generation + 1,
        updated_at = NEW.updated_at
    WHERE publisher_key_id = NEW.publisher_key_id
      AND pack_id = NEW.pack_id;
END;

CREATE TRIGGER v3_pack_releases_immutable_fields
BEFORE UPDATE ON v3_pack_releases
WHEN NEW.publisher_key_id <> OLD.publisher_key_id
    OR NEW.pack_id <> OLD.pack_id
    OR NEW.release_sequence <> OLD.release_sequence
    OR NEW.release_id <> OLD.release_id
    OR NEW.signed_release_sha256 <> OLD.signed_release_sha256
    OR NEW.payload_sha256 <> OLD.payload_sha256
    OR NEW.pack_version <> OLD.pack_version
    OR NEW.pack_type <> OLD.pack_type
    OR NEW.execution_class <> OLD.execution_class
    OR NEW.created_at <> OLD.created_at
BEGIN
    SELECT RAISE(ABORT, 'pack release identity is immutable');
END;

CREATE TRIGGER v3_pack_releases_state_transition
BEFORE UPDATE OF lifecycle_state ON v3_pack_releases
WHEN OLD.lifecycle_state <> NEW.lifecycle_state
    AND NOT (
        (OLD.lifecycle_state = 'staged'
            AND NEW.lifecycle_state IN (
                'self_tested', 'quarantined', 'removed'
            ))
        OR (OLD.lifecycle_state = 'self_tested'
            AND NEW.lifecycle_state IN ('ready', 'quarantined', 'removed'))
        OR (OLD.lifecycle_state = 'ready'
            AND NEW.lifecycle_state IN ('quarantined', 'removed'))
        OR (OLD.lifecycle_state = 'quarantined'
            AND NEW.lifecycle_state = 'removed')
    )
BEGIN
    SELECT RAISE(ABORT, 'pack release state transition is invalid');
END;

CREATE TRIGGER v3_pack_releases_referenced_state
BEFORE UPDATE OF lifecycle_state ON v3_pack_releases
WHEN NEW.lifecycle_state <> 'ready'
    AND EXISTS (
        SELECT 1
        FROM v3_pack_streams
        WHERE publisher_key_id = OLD.publisher_key_id
          AND pack_id = OLD.pack_id
          AND (
              active_release_sequence = OLD.release_sequence
              OR rollback_release_sequence = OLD.release_sequence
          )
    )
BEGIN
    SELECT RAISE(ABORT, 'referenced pack release must remain ready');
END;

CREATE TRIGGER v3_pack_releases_no_delete
BEFORE DELETE ON v3_pack_releases
BEGIN
    SELECT RAISE(ABORT, 'pack release history is immutable');
END;

CREATE TRIGGER v3_pack_streams_update_guard
BEFORE UPDATE ON v3_pack_streams
WHEN NEW.publisher_key_id <> OLD.publisher_key_id
    OR NEW.pack_id <> OLD.pack_id
    OR NEW.created_at <> OLD.created_at
    OR NEW.high_water_sequence < OLD.high_water_sequence
    OR (
        NEW.high_water_sequence = OLD.high_water_sequence
        AND NEW.high_water_signed_release_sha256
            IS NOT OLD.high_water_signed_release_sha256
    )
    OR NEW.generation <> OLD.generation + 1
    OR (
        NEW.high_water_sequence > OLD.high_water_sequence
        AND NOT EXISTS (
            SELECT 1
            FROM v3_pack_releases
            WHERE publisher_key_id = NEW.publisher_key_id
              AND pack_id = NEW.pack_id
              AND release_sequence = NEW.high_water_sequence
              AND signed_release_sha256
                    = NEW.high_water_signed_release_sha256
        )
    )
    OR (
        NEW.availability IN ('ready', 'disabled')
        AND NOT EXISTS (
            SELECT 1
            FROM v3_pack_publishers
            WHERE publisher_key_id = NEW.publisher_key_id
              AND trust_state = 'trusted'
        )
    )
    OR (
        NEW.availability IN ('ready', 'disabled')
        AND NOT EXISTS (
            SELECT 1
            FROM v3_pack_releases
            WHERE publisher_key_id = NEW.publisher_key_id
              AND pack_id = NEW.pack_id
              AND release_sequence = NEW.active_release_sequence
              AND lifecycle_state = 'ready'
        )
    )
    OR (
        NEW.rollback_release_sequence IS NOT NULL
        AND NOT EXISTS (
            SELECT 1
            FROM v3_pack_releases
            WHERE publisher_key_id = NEW.publisher_key_id
              AND pack_id = NEW.pack_id
              AND release_sequence = NEW.rollback_release_sequence
              AND lifecycle_state = 'ready'
        )
    )
BEGIN
    SELECT RAISE(ABORT, 'pack stream update is invalid');
END;

CREATE TRIGGER v3_pack_streams_no_delete
BEFORE DELETE ON v3_pack_streams
BEGIN
    SELECT RAISE(ABORT, 'pack stream history is immutable');
END;

UPDATE v3_compatibility_metadata
SET migration_version = 23
WHERE singleton = 1;
