ALTER TABLE v3_source_policies RENAME TO v3_source_policies_v12;

CREATE TABLE v3_source_policies (
    source_id TEXT PRIMARY KEY CHECK(
        length(CAST(source_id AS BLOB)) BETWEEN 1 AND 128
        AND source_id NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    source_class TEXT NOT NULL CHECK(source_class IN (
        'official_public_api',
        'public_ats',
        'public_employer_page',
        'regional_board',
        'restricted_public_scheduled',
        'user_import',
        'restricted_user_opened'
    )),
    access TEXT NOT NULL CHECK(access IN (
        'disabled', 'scheduled_public', 'user_opened'
    )),
    request_limit_per_hour INTEGER NOT NULL
        CHECK(request_limit_per_hour BETWEEN 0 AND 1000),
    user_review_required INTEGER NOT NULL CHECK(user_review_required IN (0, 1)),
    policy_ref TEXT NOT NULL CHECK(
        length(CAST(policy_ref AS BLOB)) BETWEEN 1 AND 128
        AND policy_ref NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    revision INTEGER NOT NULL CHECK(revision > 0),
    restriction_reason_code TEXT CHECK(
        restriction_reason_code IS NULL
        OR (
            length(CAST(restriction_reason_code AS BLOB)) BETWEEN 1 AND 128
            AND restriction_reason_code NOT GLOB '*[^-A-Za-z0-9._:]*'
        )
    ),
    reviewed_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK(access <> 'disabled' OR request_limit_per_hour = 0),
    CHECK(
        access <> 'scheduled_public'
        OR (
            request_limit_per_hour > 0
            AND source_class NOT IN ('restricted_user_opened', 'user_import')
        )
    ),
    CHECK(
        access <> 'user_opened'
        OR (request_limit_per_hour = 0 AND user_review_required = 1)
    ),
    CHECK(
        source_class <> 'restricted_user_opened'
        OR (
            access IN ('disabled', 'user_opened')
            AND restriction_reason_code IS NOT NULL
        )
    ),
    CHECK(
        source_class <> 'restricted_public_scheduled'
        OR (
            access IN ('disabled', 'scheduled_public')
            AND (access <> 'scheduled_public' OR user_review_required = 1)
            AND restriction_reason_code IS NOT NULL
        )
    )
);

INSERT INTO v3_source_policies
SELECT * FROM v3_source_policies_v12;

DROP TABLE v3_source_policies_v12;

CREATE TABLE v3_source_policy_ledger (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id TEXT NOT NULL CHECK(
        length(CAST(source_id AS BLOB)) BETWEEN 1 AND 128
        AND source_id NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    source_class TEXT NOT NULL CHECK(source_class IN (
        'official_public_api',
        'public_ats',
        'public_employer_page',
        'regional_board',
        'restricted_public_scheduled',
        'user_import',
        'restricted_user_opened'
    )),
    access TEXT NOT NULL CHECK(access IN (
        'disabled', 'scheduled_public', 'user_opened'
    )),
    request_limit_per_hour INTEGER NOT NULL
        CHECK(request_limit_per_hour BETWEEN 0 AND 1000),
    user_review_required INTEGER NOT NULL CHECK(user_review_required IN (0, 1)),
    policy_ref TEXT NOT NULL CHECK(
        length(CAST(policy_ref AS BLOB)) BETWEEN 1 AND 128
        AND policy_ref NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    revision INTEGER NOT NULL CHECK(revision > 0),
    restriction_reason_code TEXT CHECK(
        restriction_reason_code IS NULL
        OR (
            length(CAST(restriction_reason_code AS BLOB)) BETWEEN 1 AND 128
            AND restriction_reason_code NOT GLOB '*[^-A-Za-z0-9._:]*'
        )
    ),
    reviewed_at TEXT NOT NULL,
    recorded_at TEXT NOT NULL,
    CHECK(access <> 'disabled' OR request_limit_per_hour = 0),
    CHECK(
        access <> 'scheduled_public'
        OR (
            request_limit_per_hour > 0
            AND source_class NOT IN ('restricted_user_opened', 'user_import')
        )
    ),
    CHECK(
        access <> 'user_opened'
        OR (request_limit_per_hour = 0 AND user_review_required = 1)
    ),
    CHECK(
        source_class <> 'restricted_user_opened'
        OR (
            access IN ('disabled', 'user_opened')
            AND restriction_reason_code IS NOT NULL
        )
    ),
    CHECK(
        source_class <> 'restricted_public_scheduled'
        OR (
            access IN ('disabled', 'scheduled_public')
            AND (access <> 'scheduled_public' OR user_review_required = 1)
            AND restriction_reason_code IS NOT NULL
        )
    ),
    UNIQUE(source_id, revision)
);

INSERT INTO v3_source_policy_ledger (
    source_id,
    source_class,
    access,
    request_limit_per_hour,
    user_review_required,
    policy_ref,
    revision,
    restriction_reason_code,
    reviewed_at,
    recorded_at
)
SELECT
    source_id,
    source_class,
    access,
    request_limit_per_hour,
    user_review_required,
    policy_ref,
    revision,
    restriction_reason_code,
    reviewed_at,
    updated_at
FROM v3_source_policies;

CREATE TRIGGER v3_source_policy_revision_must_increase
BEFORE UPDATE ON v3_source_policies
WHEN NEW.revision <= OLD.revision
BEGIN
    SELECT RAISE(ABORT, 'source policy revision must increase');
END;

CREATE TRIGGER v3_source_policy_identity_is_stable
BEFORE UPDATE ON v3_source_policies
WHEN NEW.source_id <> OLD.source_id
BEGIN
    SELECT RAISE(ABORT, 'source policy identity cannot change');
END;

CREATE TRIGGER v3_source_policy_insert_must_advance
BEFORE INSERT ON v3_source_policies
WHEN NEW.revision <= COALESCE((
    SELECT MAX(revision)
    FROM v3_source_policy_ledger
    WHERE source_id = NEW.source_id
), 0)
BEGIN
    SELECT RAISE(ABORT, 'source policy revision must advance history');
END;

CREATE TRIGGER v3_source_policy_no_delete
BEFORE DELETE ON v3_source_policies
BEGIN
    SELECT RAISE(ABORT, 'source policy must be disabled, not deleted');
END;

CREATE TRIGGER v3_source_policy_ledger_matches_current
BEFORE INSERT ON v3_source_policy_ledger
WHEN NOT EXISTS (
    SELECT 1
    FROM v3_source_policies
    WHERE source_id = NEW.source_id
      AND source_class = NEW.source_class
      AND access = NEW.access
      AND request_limit_per_hour = NEW.request_limit_per_hour
      AND user_review_required = NEW.user_review_required
      AND policy_ref = NEW.policy_ref
      AND revision = NEW.revision
      AND restriction_reason_code IS NEW.restriction_reason_code
      AND reviewed_at = NEW.reviewed_at
)
BEGIN
    SELECT RAISE(ABORT, 'source policy history must match current policy');
END;

CREATE TRIGGER v3_source_policy_ledger_no_replace
BEFORE INSERT ON v3_source_policy_ledger
WHEN EXISTS (
    SELECT 1
    FROM v3_source_policy_ledger
    WHERE source_id = NEW.source_id
      AND revision = NEW.revision
)
BEGIN
    SELECT RAISE(ABORT, 'source policy history is append-only');
END;

CREATE TRIGGER v3_source_policy_ledger_after_insert
AFTER INSERT ON v3_source_policies
BEGIN
    INSERT INTO v3_source_policy_ledger (
        source_id,
        source_class,
        access,
        request_limit_per_hour,
        user_review_required,
        policy_ref,
        revision,
        restriction_reason_code,
        reviewed_at,
        recorded_at
    ) VALUES (
        NEW.source_id,
        NEW.source_class,
        NEW.access,
        NEW.request_limit_per_hour,
        NEW.user_review_required,
        NEW.policy_ref,
        NEW.revision,
        NEW.restriction_reason_code,
        NEW.reviewed_at,
        NEW.created_at
    );
END;

CREATE TRIGGER v3_source_policy_ledger_after_update
AFTER UPDATE ON v3_source_policies
BEGIN
    INSERT INTO v3_source_policy_ledger (
        source_id,
        source_class,
        access,
        request_limit_per_hour,
        user_review_required,
        policy_ref,
        revision,
        restriction_reason_code,
        reviewed_at,
        recorded_at
    ) VALUES (
        NEW.source_id,
        NEW.source_class,
        NEW.access,
        NEW.request_limit_per_hour,
        NEW.user_review_required,
        NEW.policy_ref,
        NEW.revision,
        NEW.restriction_reason_code,
        NEW.reviewed_at,
        NEW.updated_at
    );
END;

CREATE TRIGGER v3_source_policy_ledger_no_update
BEFORE UPDATE ON v3_source_policy_ledger
BEGIN
    SELECT RAISE(ABORT, 'source policy history is append-only');
END;

CREATE TRIGGER v3_source_policy_ledger_no_delete
BEFORE DELETE ON v3_source_policy_ledger
BEGIN
    SELECT RAISE(ABORT, 'source policy history is append-only');
END;

CREATE TABLE v3_source_consent_events (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL UNIQUE CHECK(
        length(CAST(event_id AS BLOB)) BETWEEN 1 AND 128
        AND event_id NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    previous_event_id TEXT CHECK(
        previous_event_id IS NULL
        OR (
            length(CAST(previous_event_id AS BLOB)) BETWEEN 1 AND 128
            AND previous_event_id NOT GLOB '*[^-A-Za-z0-9._:]*'
        )
    ),
    source_id TEXT NOT NULL CHECK(
        length(CAST(source_id AS BLOB)) BETWEEN 1 AND 128
        AND source_id NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    operation TEXT NOT NULL CHECK(operation = 'scheduled_check'),
    warning_version INTEGER NOT NULL CHECK(warning_version > 0),
    behavior_revision INTEGER NOT NULL CHECK(behavior_revision > 0),
    policy_ref TEXT NOT NULL CHECK(
        length(CAST(policy_ref AS BLOB)) BETWEEN 1 AND 128
        AND policy_ref NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    policy_revision INTEGER NOT NULL CHECK(policy_revision > 0),
    source_class TEXT NOT NULL CHECK(source_class = 'restricted_public_scheduled'),
    data_categories_json TEXT NOT NULL CHECK(
        length(CAST(data_categories_json AS BLOB)) <= 1024
        AND json_valid(data_categories_json)
        AND json_type(data_categories_json) = 'array'
        AND json_array_length(data_categories_json) BETWEEN 1 AND 16
    ),
    destination_sha256 TEXT NOT NULL CHECK(
        length(destination_sha256) = 64
        AND destination_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    request_sha256 TEXT NOT NULL CHECK(
        length(request_sha256) = 64
        AND request_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    decision TEXT NOT NULL CHECK(decision IN ('granted', 'revoked')),
    recorded_at TEXT NOT NULL,
    FOREIGN KEY (previous_event_id)
        REFERENCES v3_source_consent_events(event_id),
    FOREIGN KEY (source_id, policy_revision)
        REFERENCES v3_source_policy_ledger(source_id, revision)
);

CREATE INDEX idx_v3_source_consent_latest
ON v3_source_consent_events(source_id, operation, sequence DESC);

CREATE TRIGGER v3_source_consent_no_replace
BEFORE INSERT ON v3_source_consent_events
WHEN EXISTS (
    SELECT 1
    FROM v3_source_consent_events
    WHERE event_id = NEW.event_id
)
BEGIN
    SELECT RAISE(ABORT, 'source consent history is append-only');
END;

CREATE TRIGGER v3_source_consent_validate_categories
BEFORE INSERT ON v3_source_consent_events
WHEN EXISTS (
    SELECT 1
    FROM json_each(NEW.data_categories_json)
    WHERE type <> 'text'
       OR value NOT IN (
            'public_job_posting',
            'career_goals',
            'location_preferences'
       )
)
OR (
    SELECT COUNT(*)
    FROM json_each(NEW.data_categories_json)
) <> (
    SELECT COUNT(DISTINCT value)
    FROM json_each(NEW.data_categories_json)
)
BEGIN
    SELECT RAISE(ABORT, 'invalid source consent data categories');
END;

CREATE TRIGGER v3_source_consent_chain_is_current
BEFORE INSERT ON v3_source_consent_events
WHEN (
    SELECT event_id
    FROM v3_source_consent_events
    WHERE source_id = NEW.source_id
      AND operation = NEW.operation
    ORDER BY sequence DESC
    LIMIT 1
) IS NOT NEW.previous_event_id
BEGIN
    SELECT RAISE(ABORT, 'source consent state changed');
END;

CREATE TRIGGER v3_source_consent_grant_matches_policy
BEFORE INSERT ON v3_source_consent_events
WHEN NEW.decision = 'granted'
AND NOT EXISTS (
    SELECT 1
    FROM v3_source_policies
    WHERE source_id = NEW.source_id
      AND source_class = NEW.source_class
      AND policy_ref = NEW.policy_ref
      AND revision = NEW.policy_revision
      AND access = 'scheduled_public'
      AND user_review_required = 1
)
BEGIN
    SELECT RAISE(ABORT, 'source consent must match current policy');
END;

CREATE TRIGGER v3_source_consent_revoke_matches_grant
BEFORE INSERT ON v3_source_consent_events
WHEN NEW.decision = 'revoked'
AND NOT EXISTS (
    SELECT 1
    FROM v3_source_consent_events
    WHERE event_id = NEW.previous_event_id
      AND decision = 'granted'
      AND source_id = NEW.source_id
      AND operation = NEW.operation
      AND warning_version = NEW.warning_version
      AND behavior_revision = NEW.behavior_revision
      AND policy_ref = NEW.policy_ref
      AND policy_revision = NEW.policy_revision
      AND source_class = NEW.source_class
      AND data_categories_json = NEW.data_categories_json
      AND destination_sha256 = NEW.destination_sha256
      AND request_sha256 = NEW.request_sha256
)
BEGIN
    SELECT RAISE(ABORT, 'source consent revocation must match the latest grant');
END;

CREATE TRIGGER v3_source_consent_no_update
BEFORE UPDATE ON v3_source_consent_events
BEGIN
    SELECT RAISE(ABORT, 'source consent history is append-only');
END;

CREATE TRIGGER v3_source_consent_no_delete
BEFORE DELETE ON v3_source_consent_events
BEGIN
    SELECT RAISE(ABORT, 'source consent history is append-only');
END;

UPDATE v3_compatibility_metadata
SET migration_version = 13
WHERE singleton = 1;
