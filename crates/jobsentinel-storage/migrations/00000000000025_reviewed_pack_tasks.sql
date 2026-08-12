-- One approved local task is an append-only, single-use binding to one exact
-- ready pack release. Context is deliberately stored as typed scalar fields.

CREATE TABLE pack_task_runs (
    run_id TEXT PRIMARY KEY CHECK(
        length(CAST(run_id AS BLOB)) BETWEEN 1 AND 128
        AND run_id NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    approval_reference TEXT NOT NULL CHECK(
        length(CAST(approval_reference AS BLOB)) BETWEEN 1 AND 128
        AND approval_reference NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    publisher_key_id TEXT NOT NULL CHECK(
        length(CAST(publisher_key_id AS BLOB)) BETWEEN 1 AND 128
        AND publisher_key_id NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    pack_id TEXT NOT NULL CHECK(
        length(CAST(pack_id AS BLOB)) BETWEEN 1 AND 128
        AND pack_id NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    release_sequence INTEGER NOT NULL CHECK(release_sequence > 0),
    signed_release_sha256 TEXT NOT NULL CHECK(
        length(CAST(signed_release_sha256 AS BLOB)) = 64
        AND signed_release_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    stream_generation INTEGER NOT NULL CHECK(stream_generation >= 0),
    task_kind TEXT NOT NULL CHECK(task_kind IN ('evidence_review', 'draft_packet')),
    task_id TEXT NOT NULL CHECK(
        length(CAST(task_id AS BLOB)) BETWEEN 1 AND 128
        AND task_id NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    input_sha256 TEXT NOT NULL CHECK(
        length(CAST(input_sha256 AS BLOB)) = 64
        AND input_sha256 NOT GLOB '*[^0-9a-f]*'
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
    status TEXT NOT NULL CHECK(status IN (
        'pending', 'started', 'succeeded', 'failed', 'cancelled'
    )),
    receipt_id TEXT UNIQUE REFERENCES v3_privacy_receipts(receipt_id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL CHECK(datetime(created_at) IS NOT NULL),
    expires_at TEXT NOT NULL CHECK(
        datetime(expires_at) IS NOT NULL
        AND datetime(expires_at) > datetime(created_at)
    ),
    started_at TEXT CHECK(started_at IS NULL OR datetime(started_at) IS NOT NULL),
    completed_at TEXT CHECK(completed_at IS NULL OR datetime(completed_at) IS NOT NULL),
    UNIQUE(approval_reference),
    CHECK(
        (started_at IS NULL OR datetime(started_at) >= datetime(created_at))
        AND (completed_at IS NULL OR datetime(completed_at) >= datetime(created_at))
        AND (started_at IS NULL OR completed_at IS NULL
            OR datetime(completed_at) >= datetime(started_at))
    ),
    CHECK(
        (status = 'pending' AND receipt_id IS NULL AND started_at IS NULL AND completed_at IS NULL)
        OR (status = 'started' AND receipt_id IS NULL AND started_at IS NOT NULL AND completed_at IS NULL)
        OR (status = 'succeeded' AND receipt_id IS NOT NULL AND started_at IS NOT NULL AND completed_at IS NOT NULL)
        OR (status IN ('failed', 'cancelled') AND receipt_id IS NULL AND completed_at IS NOT NULL)
    )
);

CREATE INDEX idx_pack_task_runs_reconciliation
    ON pack_task_runs(status, expires_at);

CREATE TRIGGER pack_task_runs_canonical_context
BEFORE INSERT ON pack_task_runs
WHEN NEW.status <> 'pending'
    OR EXISTS (
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
        SELECT 1 FROM json_each(NEW.privacy_labels_json) AS current
        JOIN json_each(NEW.privacy_labels_json) AS previous
          ON CAST(previous.key AS INTEGER) < CAST(current.key AS INTEGER)
        WHERE previous.value >= current.value
    )
    OR EXISTS (
        SELECT 1 FROM json_each(NEW.data_categories_json) AS current
        JOIN json_each(NEW.data_categories_json) AS previous
          ON CAST(previous.key AS INTEGER) < CAST(current.key AS INTEGER)
        WHERE previous.value >= current.value
    )
    OR json_array_length(NEW.privacy_labels_json) = 0
    OR json_array_length(NEW.data_categories_json) = 0
BEGIN
    SELECT RAISE(ABORT, 'pack task context is invalid');
END;

CREATE TRIGGER pack_task_runs_context_immutable
BEFORE UPDATE ON pack_task_runs
WHEN NEW.run_id <> OLD.run_id
    OR NEW.approval_reference <> OLD.approval_reference
    OR NEW.publisher_key_id <> OLD.publisher_key_id
    OR NEW.pack_id <> OLD.pack_id
    OR NEW.release_sequence <> OLD.release_sequence
    OR NEW.signed_release_sha256 <> OLD.signed_release_sha256
    OR NEW.stream_generation <> OLD.stream_generation
    OR NEW.task_kind <> OLD.task_kind
    OR NEW.task_id <> OLD.task_id
    OR NEW.input_sha256 <> OLD.input_sha256
    OR NEW.privacy_labels_json <> OLD.privacy_labels_json
    OR NEW.data_categories_json <> OLD.data_categories_json
    OR NEW.created_at <> OLD.created_at
    OR NEW.expires_at <> OLD.expires_at
BEGIN
    SELECT RAISE(ABORT, 'pack task context is immutable');
END;

CREATE TRIGGER pack_task_runs_valid_transition
BEFORE UPDATE OF status, receipt_id, started_at, completed_at ON pack_task_runs
WHEN NOT (
    (OLD.status = 'pending' AND NEW.status = 'started'
        AND NEW.receipt_id IS NULL AND NEW.started_at IS NOT NULL AND NEW.completed_at IS NULL)
    OR (OLD.status = 'pending' AND NEW.status = 'cancelled'
        AND NEW.receipt_id IS NULL AND NEW.completed_at IS NOT NULL)
    OR (OLD.status = 'started' AND NEW.status = 'succeeded'
        AND NEW.receipt_id IS NOT NULL AND NEW.completed_at IS NOT NULL)
    OR (OLD.status = 'started' AND NEW.status = 'failed'
        AND NEW.receipt_id IS NULL AND NEW.completed_at IS NOT NULL)
)
BEGIN
    SELECT RAISE(ABORT, 'pack task transition is invalid');
END;

CREATE TRIGGER pack_task_runs_start_requires_exact_ready_pack
BEFORE UPDATE OF status ON pack_task_runs
WHEN OLD.status = 'pending' AND NEW.status = 'started'
AND NOT EXISTS (
    SELECT 1
    FROM v3_pack_streams AS stream
    JOIN v3_pack_releases AS release
      ON release.publisher_key_id = stream.publisher_key_id
     AND release.pack_id = stream.pack_id
     AND release.release_sequence = stream.active_release_sequence
    JOIN v3_pack_publishers AS publisher
      ON publisher.publisher_key_id = stream.publisher_key_id
    WHERE stream.publisher_key_id = NEW.publisher_key_id
      AND stream.pack_id = NEW.pack_id
      AND stream.generation = NEW.stream_generation
      AND stream.availability = 'ready'
      AND stream.active_release_sequence = NEW.release_sequence
      AND release.signed_release_sha256 = NEW.signed_release_sha256
      AND release.lifecycle_state = 'ready'
      AND release.self_tested_at IS NOT NULL
      AND publisher.trust_state = 'trusted'
      AND datetime('now') < datetime(NEW.expires_at)
)
BEGIN
    SELECT RAISE(ABORT, 'pack task pack state is no longer ready');
END;

CREATE TRIGGER pack_task_runs_complete_requires_exact_ready_pack
BEFORE UPDATE OF status ON pack_task_runs
WHEN OLD.status = 'started' AND NEW.status = 'succeeded'
AND NOT EXISTS (
    SELECT 1
    FROM v3_pack_streams AS stream
    JOIN v3_pack_releases AS release
      ON release.publisher_key_id = stream.publisher_key_id
     AND release.pack_id = stream.pack_id
     AND release.release_sequence = stream.active_release_sequence
    JOIN v3_pack_publishers AS publisher
      ON publisher.publisher_key_id = stream.publisher_key_id
    WHERE stream.publisher_key_id = NEW.publisher_key_id
      AND stream.pack_id = NEW.pack_id
      AND stream.generation = NEW.stream_generation
      AND stream.availability = 'ready'
      AND stream.active_release_sequence = NEW.release_sequence
      AND release.signed_release_sha256 = NEW.signed_release_sha256
      AND release.lifecycle_state = 'ready'
      AND release.self_tested_at IS NOT NULL
      AND publisher.trust_state = 'trusted'
      AND datetime('now') < datetime(NEW.expires_at)
)
BEGIN
    SELECT RAISE(ABORT, 'pack task pack state is no longer ready');
END;

CREATE TRIGGER pack_task_runs_no_delete
BEFORE DELETE ON pack_task_runs
BEGIN
    SELECT RAISE(ABORT, 'pack task history is append-only');
END;

UPDATE v3_compatibility_metadata
SET migration_version = 25
WHERE singleton = 1;
