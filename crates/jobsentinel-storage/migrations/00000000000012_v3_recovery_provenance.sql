CREATE TABLE v3_recovery_operations (
    operation_id TEXT PRIMARY KEY NOT NULL
        CHECK(length(operation_id) BETWEEN 1 AND 128)
        CHECK(operation_id NOT GLOB '*[^A-Za-z0-9._:-]*'),
    operation_kind TEXT NOT NULL
        CHECK(operation_kind IN (
            'backup', 'export', 'restore', 'rollback', 'repair', 'cleanup'
        )),
    outcome TEXT NOT NULL
        CHECK(outcome IN ('started', 'succeeded', 'failed', 'cancelled')),
    source_migration_sequence INTEGER NOT NULL
        CHECK(source_migration_sequence >= 0),
    target_migration_sequence INTEGER NOT NULL
        CHECK(target_migration_sequence >= 0),
    error_kind TEXT
        CHECK(error_kind IS NULL OR error_kind IN (
            'database', 'decode', 'encode', 'io', 'pool_closed',
            'pool_timed_out', 'protocol', 'row_not_found', 'tls',
            'type_not_found', 'column', 'unknown'
        )),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    CHECK(
        (outcome = 'started' AND completed_at IS NULL AND error_kind IS NULL)
        OR
        (outcome = 'succeeded' AND completed_at IS NOT NULL AND error_kind IS NULL)
        OR
        (outcome IN ('failed', 'cancelled') AND completed_at IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_v3_recovery_operations_created
    ON v3_recovery_operations(created_at DESC, operation_id DESC);

UPDATE v3_compatibility_metadata
SET migration_version = 12
WHERE singleton = 1;
