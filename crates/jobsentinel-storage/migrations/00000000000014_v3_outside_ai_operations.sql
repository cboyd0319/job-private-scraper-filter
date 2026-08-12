CREATE TABLE v3_outside_ai_operations (
    operation_id TEXT PRIMARY KEY CHECK(
        length(CAST(operation_id AS BLOB)) BETWEEN 1 AND 128
        AND operation_id NOT GLOB '*[^-A-Za-z0-9._:]*'
        AND operation_id GLOB 'outside-ai:*'
    ),
    approval_reference TEXT NOT NULL UNIQUE CHECK(
        length(CAST(approval_reference AS BLOB)) BETWEEN 1 AND 128
        AND approval_reference NOT GLOB '*[^-A-Za-z0-9._:]*'
        AND approval_reference GLOB 'outside-ai-approval:*'
    ),
    feature_id TEXT NOT NULL CHECK(feature_id = 'job-description-summary'),
    provider_id TEXT NOT NULL CHECK(
        length(CAST(provider_id AS BLOB)) BETWEEN 1 AND 128
        AND provider_id NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    destination TEXT NOT NULL CHECK(
        length(CAST(destination AS BLOB)) BETWEEN 1 AND 2048
        AND destination GLOB 'https://*'
        AND instr(destination, '@') = 0
        AND instr(destination, '?') = 0
        AND instr(destination, '#') = 0
        AND instr(destination, char(9)) = 0
        AND instr(destination, char(10)) = 0
        AND instr(destination, char(13)) = 0
        AND instr(destination, ' ') = 0
    ),
    request_sha256 TEXT NOT NULL CHECK(
        length(request_sha256) = 64
        AND request_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    labels_json TEXT NOT NULL CHECK(
        json_valid(labels_json)
        AND json_type(labels_json) = 'array'
        AND json_array_length(labels_json) BETWEEN 1 AND 4
    ),
    data_categories_json TEXT NOT NULL CHECK(
        json_valid(data_categories_json)
        AND json_type(data_categories_json) = 'array'
        AND json_array_length(data_categories_json) BETWEEN 1 AND 6
    ),
    gateway_policy_revision TEXT NOT NULL CHECK(
        gateway_policy_revision = 'jobsentinel.external-ai-gateway.v1'
    ),
    status TEXT NOT NULL CHECK(status IN (
        'pending', 'started', 'succeeded', 'failed', 'ambiguous', 'cancelled'
    )),
    receipt_recorded INTEGER NOT NULL DEFAULT 0 CHECK(receipt_recorded IN (0, 1)),
    created_at TEXT NOT NULL CHECK(strftime('%s', created_at) IS NOT NULL),
    expires_at TEXT NOT NULL CHECK(
        strftime('%s', expires_at) IS NOT NULL
        AND julianday(expires_at) > julianday(created_at)
    ),
    started_at TEXT CHECK(
        started_at IS NULL OR strftime('%s', started_at) IS NOT NULL
    ),
    completed_at TEXT CHECK(
        completed_at IS NULL OR strftime('%s', completed_at) IS NOT NULL
    )
);

CREATE TRIGGER v3_outside_ai_insert_is_pending
BEFORE INSERT ON v3_outside_ai_operations
WHEN NEW.status <> 'pending'
  OR NEW.receipt_recorded <> 0
  OR NEW.started_at IS NOT NULL
  OR NEW.completed_at IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'outside AI review must begin pending');
END;

CREATE TRIGGER v3_outside_ai_validate_labels
BEFORE INSERT ON v3_outside_ai_operations
WHEN NEW.labels_json <> '["external_ai_optional","public_data_only"]'
BEGIN
    SELECT RAISE(ABORT, 'invalid outside AI privacy labels');
END;

CREATE TRIGGER v3_outside_ai_validate_categories
BEFORE INSERT ON v3_outside_ai_operations
WHEN NEW.data_categories_json <> '["public_job_posting"]'
BEGIN
    SELECT RAISE(ABORT, 'invalid outside AI data categories');
END;

CREATE TRIGGER v3_outside_ai_context_is_immutable
BEFORE UPDATE ON v3_outside_ai_operations
WHEN OLD.operation_id IS NOT NEW.operation_id
  OR OLD.approval_reference IS NOT NEW.approval_reference
  OR OLD.feature_id IS NOT NEW.feature_id
  OR OLD.provider_id IS NOT NEW.provider_id
  OR OLD.destination IS NOT NEW.destination
  OR OLD.request_sha256 IS NOT NEW.request_sha256
  OR OLD.labels_json IS NOT NEW.labels_json
  OR OLD.data_categories_json IS NOT NEW.data_categories_json
  OR OLD.gateway_policy_revision IS NOT NEW.gateway_policy_revision
  OR OLD.created_at IS NOT NEW.created_at
  OR OLD.expires_at IS NOT NEW.expires_at
BEGIN
    SELECT RAISE(ABORT, 'outside AI review context is immutable');
END;

CREATE TRIGGER v3_outside_ai_status_transition
BEFORE UPDATE ON v3_outside_ai_operations
WHEN NOT (
    (
        OLD.status = 'pending'
        AND NEW.status = 'started'
        AND OLD.receipt_recorded = NEW.receipt_recorded
        AND NEW.started_at IS NOT NULL
        AND NEW.completed_at IS NULL
        AND julianday(NEW.started_at) >= julianday(NEW.created_at)
        AND julianday(NEW.started_at) <= julianday(NEW.expires_at)
    )
    OR (
        OLD.status = 'pending'
        AND NEW.status = 'cancelled'
        AND OLD.receipt_recorded = NEW.receipt_recorded
        AND NEW.started_at IS NULL
        AND NEW.completed_at IS NOT NULL
        AND julianday(NEW.completed_at) >= julianday(NEW.created_at)
    )
    OR (
        OLD.status = 'started'
        AND NEW.status IN ('succeeded', 'failed', 'ambiguous')
        AND OLD.receipt_recorded = NEW.receipt_recorded
        AND NEW.started_at IS OLD.started_at
        AND NEW.completed_at IS NOT NULL
        AND julianday(NEW.completed_at) >= julianday(NEW.started_at)
        AND (NEW.status <> 'succeeded' OR NEW.receipt_recorded = 1)
    )
    OR (
        OLD.status = 'started'
        AND NEW.status = OLD.status
        AND OLD.receipt_recorded = 0
        AND NEW.receipt_recorded = 1
        AND NEW.started_at IS OLD.started_at
        AND NEW.completed_at IS NULL
    )
)
BEGIN
    SELECT RAISE(ABORT, 'invalid outside AI lifecycle transition');
END;

CREATE TRIGGER v3_outside_ai_no_delete
BEFORE DELETE ON v3_outside_ai_operations
BEGIN
    SELECT RAISE(ABORT, 'outside AI lifecycle is append-only');
END;

CREATE TRIGGER v3_privacy_receipt_no_update
BEFORE UPDATE ON v3_privacy_receipts
BEGIN
    SELECT RAISE(ABORT, 'privacy receipt audit metadata is immutable');
END;

CREATE TRIGGER v3_privacy_receipt_no_delete
BEFORE DELETE ON v3_privacy_receipts
BEGIN
    SELECT RAISE(ABORT, 'privacy receipt audit metadata is append-only');
END;

CREATE TRIGGER v3_privacy_receipt_exact_shape
BEFORE INSERT ON v3_privacy_receipts
WHEN EXISTS (
    SELECT 1
    FROM json_each(NEW.receipt_json)
    WHERE key NOT IN (
        'schema',
        'receipt_id',
        'task_id',
        'pack_id',
        'labels',
        'data_categories',
        'stored_locally',
        'data_left_device',
        'external_destination',
        'gateway_policy_id',
        'approval_reference',
        'delete_or_revoke_action'
    )
)
BEGIN
    SELECT RAISE(ABORT, 'privacy receipt contains unsupported metadata');
END;

CREATE TRIGGER v3_privacy_receipt_local_has_no_approval
BEFORE INSERT ON v3_privacy_receipts
WHEN NEW.data_left_device = 0
AND COALESCE(
    json_type(NEW.receipt_json, '$.approval_reference'),
    'null'
) <> 'null'
BEGIN
    SELECT RAISE(ABORT, 'local privacy receipt cannot reference external approval');
END;

CREATE TRIGGER v3_external_receipt_matches_started_operation
BEFORE INSERT ON v3_privacy_receipts
WHEN NEW.data_left_device = 1
AND NOT EXISTS (
    SELECT 1
    FROM v3_outside_ai_operations
    WHERE status = 'started'
      AND receipt_recorded = 0
      AND operation_id = json_extract(NEW.receipt_json, '$.task_id')
      AND approval_reference =
          json_extract(NEW.receipt_json, '$.approval_reference')
      AND destination =
          json_extract(NEW.receipt_json, '$.external_destination')
      AND gateway_policy_revision =
          json_extract(NEW.receipt_json, '$.gateway_policy_id')
      AND labels_json = json_extract(NEW.receipt_json, '$.labels')
      AND data_categories_json =
          json_extract(NEW.receipt_json, '$.data_categories')
)
BEGIN
    SELECT RAISE(ABORT, 'external privacy receipt does not match a started operation');
END;

CREATE TRIGGER v3_external_receipt_consumes_approval
AFTER INSERT ON v3_privacy_receipts
WHEN NEW.data_left_device = 1
BEGIN
    UPDATE v3_outside_ai_operations
    SET receipt_recorded = 1
    WHERE operation_id = json_extract(NEW.receipt_json, '$.task_id')
      AND approval_reference =
          json_extract(NEW.receipt_json, '$.approval_reference');
END;

UPDATE v3_compatibility_metadata
SET migration_version = 14
WHERE singleton = 1;
