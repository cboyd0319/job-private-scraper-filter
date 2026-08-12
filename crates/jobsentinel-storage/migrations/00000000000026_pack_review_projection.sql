-- Persists immutable signed pack-review facts and per-release artifact-cleanup
-- truth so management remains accurate across crashes, retries, and restaging.

ALTER TABLE v3_pack_releases
ADD COLUMN artifact_cleanup_pending INTEGER NOT NULL DEFAULT 0
CHECK(artifact_cleanup_pending IN (0, 1));

UPDATE v3_pack_releases
SET artifact_cleanup_pending = 1
WHERE lifecycle_state = 'removed';

CREATE INDEX idx_pack_release_cleanup_pending
    ON v3_pack_releases(
        artifact_cleanup_pending, publisher_key_id, pack_id, release_sequence
    );

CREATE TABLE pack_release_reviews (
    publisher_key_id TEXT NOT NULL,
    pack_id TEXT NOT NULL,
    release_sequence INTEGER NOT NULL CHECK(release_sequence > 0),
    publisher_name TEXT NOT NULL CHECK(
        length(CAST(publisher_name AS BLOB)) BETWEEN 1 AND 128
    ),
    license TEXT NOT NULL CHECK(
        length(CAST(license AS BLOB)) BETWEEN 1 AND 128
    ),
    minimum_app_version TEXT NOT NULL CHECK(
        length(CAST(minimum_app_version AS BLOB)) BETWEEN 1 AND 64
    ),
    maximum_app_version TEXT NOT NULL CHECK(
        length(CAST(maximum_app_version AS BLOB)) BETWEEN 1 AND 64
    ),
    payload_bytes INTEGER NOT NULL CHECK(
        payload_bytes BETWEEN 0 AND 3145728
    ),
    fixture_summary TEXT NOT NULL CHECK(
        length(CAST(fixture_summary AS BLOB)) BETWEEN 1 AND 4096
    ),
    privacy_labels_json TEXT NOT NULL CHECK(
        json_valid(privacy_labels_json)
        AND json_type(privacy_labels_json) = 'array'
        AND privacy_labels_json = json(privacy_labels_json)
    ),
    data_categories_json TEXT NOT NULL CHECK(
        json_valid(data_categories_json)
        AND json_type(data_categories_json) = 'array'
        AND data_categories_json = json(data_categories_json)
    ),
    task_kinds_json TEXT NOT NULL CHECK(
        json_valid(task_kinds_json)
        AND json_type(task_kinds_json) = 'array'
        AND task_kinds_json = json(task_kinds_json)
    ),
    actions_json TEXT NOT NULL CHECK(
        json_valid(actions_json)
        AND json_type(actions_json) = 'array'
        AND actions_json = json(actions_json)
    ),
    approval_gates_json TEXT NOT NULL CHECK(
        json_valid(approval_gates_json)
        AND json_type(approval_gates_json) = 'array'
        AND approval_gates_json = json(approval_gates_json)
    ),
    gateway_policy_id TEXT CHECK(
        gateway_policy_id IS NULL
        OR (
            length(CAST(gateway_policy_id AS BLOB)) BETWEEN 1 AND 128
            AND gateway_policy_id NOT GLOB '*[^-A-Za-z0-9._:]*'
        )
    ),
    external_destinations_json TEXT NOT NULL CHECK(
        json_valid(external_destinations_json)
        AND json_type(external_destinations_json) = 'array'
        AND external_destinations_json = json(external_destinations_json)
    ),
    PRIMARY KEY(publisher_key_id, pack_id, release_sequence),
    FOREIGN KEY(publisher_key_id, pack_id, release_sequence)
        REFERENCES v3_pack_releases(
            publisher_key_id, pack_id, release_sequence
        ) ON DELETE RESTRICT
);

CREATE TRIGGER pack_release_reviews_canonical_values
BEFORE INSERT ON pack_release_reviews
WHEN EXISTS (
        SELECT 1 FROM json_each(NEW.privacy_labels_json)
        WHERE type <> 'text' OR value NOT IN (
            'local_only', 'external_ai_optional', 'sensitive', 'public_data_only'
        )
    )
    OR EXISTS (
        SELECT 1 FROM json_each(NEW.data_categories_json)
        WHERE type <> 'text' OR value NOT IN (
            'public_job_posting', 'resume_evidence', 'application_history',
            'career_goals', 'pay_preferences', 'location_preferences',
            'military_service', 'clearance_claim', 'protected_veteran_answer'
        )
    )
    OR EXISTS (
        SELECT 1 FROM json_each(NEW.task_kinds_json)
        WHERE type <> 'text' OR value NOT IN (
            'source_check', 'evidence_review', 'draft_packet',
            'backup', 'export', 'pack_install'
        )
    )
    OR EXISTS (
        SELECT 1 FROM json_each(NEW.actions_json)
        WHERE type <> 'text' OR value NOT IN (
            'read_selected_case_file', 'read_selected_resume_evidence',
            'read_public_job_posting', 'create_draft_local_note',
            'create_draft_application_packet', 'create_reminder',
            'open_browser_link', 'request_source_check',
            'request_external_ai', 'write_local_event'
        )
    )
    OR EXISTS (
        SELECT 1 FROM json_each(NEW.approval_gates_json)
        WHERE type <> 'text' OR value <> 'per_execution_review'
    )
    OR EXISTS (
        SELECT 1
        FROM json_each(NEW.privacy_labels_json) AS current
        JOIN json_each(NEW.privacy_labels_json) AS previous
          ON CAST(previous.key AS INTEGER) < CAST(current.key AS INTEGER)
        WHERE previous.value = current.value
    )
    OR EXISTS (
        SELECT 1
        FROM json_each(NEW.data_categories_json) AS current
        JOIN json_each(NEW.data_categories_json) AS previous
          ON CAST(previous.key AS INTEGER) < CAST(current.key AS INTEGER)
        WHERE previous.value = current.value
    )
    OR EXISTS (
        SELECT 1
        FROM json_each(NEW.task_kinds_json) AS current
        JOIN json_each(NEW.task_kinds_json) AS previous
          ON CAST(previous.key AS INTEGER) < CAST(current.key AS INTEGER)
        WHERE previous.value = current.value
    )
    OR EXISTS (
        SELECT 1
        FROM json_each(NEW.actions_json) AS current
        JOIN json_each(NEW.actions_json) AS previous
          ON CAST(previous.key AS INTEGER) < CAST(current.key AS INTEGER)
        WHERE previous.value = current.value
    )
    OR EXISTS (
        SELECT 1
        FROM json_each(NEW.approval_gates_json) AS current
        JOIN json_each(NEW.approval_gates_json) AS previous
          ON CAST(previous.key AS INTEGER) < CAST(current.key AS INTEGER)
        WHERE previous.value = current.value
    )
    OR EXISTS (
        SELECT 1 FROM json_each(NEW.external_destinations_json)
        WHERE type <> 'text'
            OR length(CAST(value AS BLOB)) NOT BETWEEN 1 AND 128
    )
    OR (
        EXISTS (
            SELECT 1 FROM json_each(NEW.actions_json)
            WHERE value = 'request_external_ai'
        )
        AND (
            NEW.gateway_policy_id IS NOT 'jobsentinel.external-ai-gateway.v1'
            OR NEW.external_destinations_json
                <> '["jobsentinel.external-ai-gateway.v1"]'
        )
    )
    OR (
        NOT EXISTS (
            SELECT 1 FROM json_each(NEW.actions_json)
            WHERE value = 'request_external_ai'
        )
        AND (
            NEW.gateway_policy_id IS NOT NULL
            OR NEW.external_destinations_json <> '[]'
        )
    )
BEGIN
    SELECT RAISE(ABORT, 'pack release review is invalid');
END;

CREATE TRIGGER pack_release_reviews_immutable
BEFORE UPDATE ON pack_release_reviews
BEGIN
    SELECT RAISE(ABORT, 'pack release review is immutable');
END;

CREATE TRIGGER pack_release_reviews_no_delete
BEFORE DELETE ON pack_release_reviews
BEGIN
    SELECT RAISE(ABORT, 'pack release review is immutable');
END;

CREATE TRIGGER pack_stream_actionable_requires_review
BEFORE UPDATE OF availability, active_release_sequence ON v3_pack_streams
WHEN NEW.availability IN ('ready', 'disabled')
    AND NOT EXISTS (
        SELECT 1 FROM pack_release_reviews
        WHERE publisher_key_id = NEW.publisher_key_id
          AND pack_id = NEW.pack_id
          AND release_sequence = NEW.active_release_sequence
    )
BEGIN
    SELECT RAISE(ABORT, 'pack release review is unavailable');
END;

CREATE TRIGGER pack_task_runs_create_requires_review
BEFORE INSERT ON pack_task_runs
WHEN NOT EXISTS (
    SELECT 1 FROM pack_release_reviews
    WHERE publisher_key_id = NEW.publisher_key_id
      AND pack_id = NEW.pack_id
      AND release_sequence = NEW.release_sequence
)
BEGIN
    SELECT RAISE(ABORT, 'pack release review is unavailable');
END;

CREATE TRIGGER pack_task_runs_transition_requires_review
BEFORE UPDATE OF status ON pack_task_runs
WHEN NEW.status IN ('started', 'succeeded')
    AND NOT EXISTS (
        SELECT 1 FROM pack_release_reviews
        WHERE publisher_key_id = NEW.publisher_key_id
          AND pack_id = NEW.pack_id
          AND release_sequence = NEW.release_sequence
    )
BEGIN
    SELECT RAISE(ABORT, 'pack release review is unavailable');
END;

CREATE TRIGGER pack_release_cleanup_truth
BEFORE UPDATE OF lifecycle_state, artifact_cleanup_pending ON v3_pack_releases
WHEN (
        OLD.lifecycle_state <> 'removed'
        AND NEW.lifecycle_state = 'removed'
        AND NEW.artifact_cleanup_pending <> 1
    )
    OR (
        NEW.artifact_cleanup_pending <> OLD.artifact_cleanup_pending
        AND NOT (
            OLD.artifact_cleanup_pending = 0
            AND NEW.artifact_cleanup_pending = 1
            AND OLD.lifecycle_state <> 'removed'
            AND NEW.lifecycle_state = 'removed'
        )
        AND NOT (
            OLD.artifact_cleanup_pending = 1
            AND NEW.artifact_cleanup_pending = 0
            AND OLD.lifecycle_state = 'removed'
            AND NEW.lifecycle_state = 'removed'
        )
    )
    OR (NEW.artifact_cleanup_pending = 1 AND NEW.lifecycle_state <> 'removed')
BEGIN
    SELECT RAISE(ABORT, 'pack release cleanup transition is invalid');
END;

UPDATE v3_compatibility_metadata
SET migration_version = 26
WHERE singleton = 1;
