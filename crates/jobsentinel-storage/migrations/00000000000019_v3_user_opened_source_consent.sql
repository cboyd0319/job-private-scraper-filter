DROP TRIGGER v3_source_consent_no_replace;
DROP TRIGGER v3_source_consent_validate_categories;
DROP TRIGGER v3_source_consent_chain_is_current;
DROP TRIGGER v3_source_consent_grant_matches_policy;
DROP TRIGGER v3_source_consent_revoke_matches_grant;
DROP TRIGGER v3_source_consent_no_update;
DROP TRIGGER v3_source_consent_no_delete;
DROP INDEX idx_v3_source_consent_latest;

ALTER TABLE v3_source_consent_events
RENAME TO v3_source_consent_events_v18;

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
    operation TEXT NOT NULL CHECK(operation IN (
        'scheduled_check',
        'restricted_workbench'
    )),
    warning_version INTEGER NOT NULL CHECK(warning_version > 0),
    behavior_revision INTEGER NOT NULL CHECK(behavior_revision > 0),
    policy_ref TEXT NOT NULL CHECK(
        length(CAST(policy_ref AS BLOB)) BETWEEN 1 AND 128
        AND policy_ref NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    policy_revision INTEGER NOT NULL CHECK(policy_revision > 0),
    source_class TEXT NOT NULL CHECK(source_class IN (
        'restricted_public_scheduled',
        'restricted_user_opened'
    )),
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

INSERT INTO v3_source_consent_events (
    sequence, event_id, previous_event_id, source_id, operation,
    warning_version, behavior_revision, policy_ref, policy_revision,
    source_class, data_categories_json, destination_sha256,
    request_sha256, decision, recorded_at
)
SELECT
    sequence, event_id, previous_event_id, source_id, operation,
    warning_version, behavior_revision, policy_ref, policy_revision,
    source_class, data_categories_json, destination_sha256,
    request_sha256, decision, recorded_at
FROM v3_source_consent_events_v18
ORDER BY sequence;

DROP TABLE v3_source_consent_events_v18;

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
       OR (
           NEW.operation = 'scheduled_check'
           AND value NOT IN (
               'public_job_posting',
               'career_goals',
               'location_preferences'
           )
       )
       OR (
           NEW.operation = 'restricted_workbench'
           AND value NOT IN (
               'public_job_posting',
               'application_history',
               'career_goals'
           )
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
      AND user_review_required = 1
      AND (
          (
              NEW.operation = 'scheduled_check'
              AND source_class = 'restricted_public_scheduled'
              AND access = 'scheduled_public'
          )
          OR (
              NEW.operation = 'restricted_workbench'
              AND source_class = 'restricted_user_opened'
              AND access = 'user_opened'
          )
      )
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
SET migration_version = 19
WHERE singleton = 1;
