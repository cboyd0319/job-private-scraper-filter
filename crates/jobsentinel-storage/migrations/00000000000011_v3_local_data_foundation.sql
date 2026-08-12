-- Local v3 data foundation. This migration is additive and keeps the legacy
-- application event ledger unchanged as read-only historical data for v3.

CREATE TABLE opportunity_case_files (
    case_file_id TEXT PRIMARY KEY
        CHECK(length(CAST(case_file_id AS BLOB)) BETWEEN 1 AND 128),
    job_hash TEXT UNIQUE REFERENCES jobs(hash) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    CHECK(job_hash IS NULL OR length(CAST(job_hash AS BLOB)) BETWEEN 1 AND 128)
);

CREATE TABLE v3_job_events (
    event_id TEXT PRIMARY KEY
        CHECK(
            length(CAST(event_id AS BLOB)) BETWEEN 1 AND 128
            AND event_id NOT GLOB '*[^-A-Za-z0-9._:]*'
        ),
    case_file_id TEXT NOT NULL
        REFERENCES opportunity_case_files(case_file_id) ON DELETE CASCADE,
    event_kind TEXT NOT NULL CHECK(event_kind IN (
        'case_created',
        'status_changed',
        'evidence_linked',
        'source_checked',
        'privacy_receipt_recorded',
        'source_policy_changed',
        'recovery_recorded'
    )),
    origin TEXT NOT NULL CHECK(origin IN ('user', 'system', 'source', 'migration')),
    user_action INTEGER NOT NULL CHECK(user_action IN (0, 1)),
    local_only INTEGER NOT NULL CHECK(local_only = 1),
    sensitive INTEGER NOT NULL CHECK(sensitive = 1),
    metadata_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    CHECK((origin = 'user') = (user_action = 1)),
    CHECK(json_valid(metadata_json)),
    CHECK(json_type(metadata_json) = 'object'),
    CHECK(length(CAST(metadata_json AS BLOB)) <= 2048),
    CHECK(
        (event_kind = 'case_created'
            AND json_extract(metadata_json, '$.kind') = 'empty')
        OR (event_kind = 'status_changed'
            AND json_extract(metadata_json, '$.kind') = 'status_transition')
        OR (event_kind IN (
                'evidence_linked',
                'privacy_receipt_recorded',
                'source_policy_changed'
            )
            AND json_extract(metadata_json, '$.kind') = 'local_reference')
        OR (event_kind = 'source_checked'
            AND json_extract(metadata_json, '$.kind') = 'source_outcome')
        OR (event_kind = 'recovery_recorded'
            AND json_extract(metadata_json, '$.kind') = 'recovery_outcome')
    ),
    CHECK(
        CASE json_extract(metadata_json, '$.kind')
            WHEN 'empty' THEN
                json_remove(metadata_json, '$.kind') = '{}'
            WHEN 'status_transition' THEN
                json_extract(metadata_json, '$.to') IN (
                    'saved', 'reviewing', 'preparing', 'applied',
                    'interviewing', 'offer', 'closed'
                )
                AND (
                    json_type(metadata_json, '$.from') = 'null'
                    OR json_extract(metadata_json, '$.from') IN (
                        'saved', 'reviewing', 'preparing', 'applied',
                        'interviewing', 'offer', 'closed'
                    )
                )
                AND json_remove(metadata_json, '$.kind', '$.from', '$.to') = '{}'
            WHEN 'local_reference' THEN
                json_type(metadata_json, '$.reference_id') = 'text'
                AND length(CAST(json_extract(
                    metadata_json, '$.reference_id'
                ) AS BLOB)) BETWEEN 1 AND 128
                AND json_extract(metadata_json, '$.reference_id')
                    NOT GLOB '*[^-A-Za-z0-9._:]*'
                AND json_remove(
                    metadata_json, '$.kind', '$.reference_id'
                ) = '{}'
            WHEN 'source_outcome' THEN
                json_type(metadata_json, '$.source_id') = 'text'
                AND length(CAST(json_extract(
                    metadata_json, '$.source_id'
                ) AS BLOB)) BETWEEN 1 AND 128
                AND json_extract(metadata_json, '$.source_id')
                    NOT GLOB '*[^-A-Za-z0-9._:]*'
                AND json_extract(metadata_json, '$.outcome') IN (
                    'success', 'failure', 'timeout', 'cancelled'
                )
                AND json_type(metadata_json, '$.item_count') = 'integer'
                AND json_extract(metadata_json, '$.item_count') BETWEEN 0 AND 10000
                AND json_type(
                    metadata_json, '$.connectivity_required'
                ) IN ('true', 'false')
                AND json_remove(
                    metadata_json,
                    '$.kind',
                    '$.source_id',
                    '$.outcome',
                    '$.item_count',
                    '$.connectivity_required'
                ) = '{}'
            WHEN 'recovery_outcome' THEN
                json_extract(metadata_json, '$.outcome') IN (
                    'retried', 'restored', 'failed'
                )
                AND json_remove(metadata_json, '$.kind', '$.outcome') = '{}'
            ELSE 0
        END
    )
);

CREATE INDEX idx_v3_job_events_case_time
    ON v3_job_events(case_file_id, created_at, event_id);

CREATE TABLE career_graph_links (
    link_id TEXT PRIMARY KEY CHECK(
        length(CAST(link_id AS BLOB)) BETWEEN 1 AND 128
        AND link_id NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    subject_id TEXT NOT NULL CHECK(
        length(CAST(subject_id AS BLOB)) BETWEEN 1 AND 128
        AND subject_id NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    relation TEXT NOT NULL CHECK(relation IN (
        'alias', 'broader', 'narrower', 'related', 'confusable',
        'evidence', 'requirement', 'blocker', 'outcome'
    )),
    object_id TEXT NOT NULL CHECK(
        length(CAST(object_id AS BLOB)) BETWEEN 1 AND 128
        AND object_id NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    provenance TEXT NOT NULL CHECK(provenance IN (
        'user_confirmed', 'imported', 'public_source',
        'model_suggestion', 'migration'
    )),
    provenance_ref TEXT CHECK(
        provenance_ref IS NULL
        OR (
            length(CAST(provenance_ref AS BLOB)) BETWEEN 1 AND 128
            AND provenance_ref NOT GLOB '*[^-A-Za-z0-9._:]*'
        )
    ),
    created_at TEXT NOT NULL,
    CHECK(subject_id <> object_id),
    CHECK(
        (provenance IN ('user_confirmed', 'migration')
            AND provenance_ref IS NULL)
        OR (provenance IN ('imported', 'public_source', 'model_suggestion')
            AND provenance_ref IS NOT NULL)
    ),
    UNIQUE(subject_id, relation, object_id)
);

CREATE TABLE source_graph_links (
    link_id TEXT PRIMARY KEY CHECK(
        length(CAST(link_id AS BLOB)) BETWEEN 1 AND 128
        AND link_id NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    source_id TEXT NOT NULL CHECK(
        length(CAST(source_id AS BLOB)) BETWEEN 1 AND 128
        AND source_id NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    relation TEXT NOT NULL CHECK(relation IN ('related', 'policy', 'lineage')),
    related_id TEXT NOT NULL CHECK(
        length(CAST(related_id AS BLOB)) BETWEEN 1 AND 128
        AND related_id NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    provenance TEXT NOT NULL CHECK(provenance IN (
        'user_confirmed', 'imported', 'public_source',
        'model_suggestion', 'migration'
    )),
    provenance_ref TEXT CHECK(
        provenance_ref IS NULL
        OR (
            length(CAST(provenance_ref AS BLOB)) BETWEEN 1 AND 128
            AND provenance_ref NOT GLOB '*[^-A-Za-z0-9._:]*'
        )
    ),
    created_at TEXT NOT NULL,
    CHECK(source_id <> related_id),
    CHECK(
        (provenance IN ('user_confirmed', 'migration')
            AND provenance_ref IS NULL)
        OR (provenance IN ('imported', 'public_source', 'model_suggestion')
            AND provenance_ref IS NOT NULL)
    ),
    UNIQUE(source_id, relation, related_id)
);

CREATE TABLE v3_privacy_receipts (
    receipt_id TEXT PRIMARY KEY CHECK(
        length(CAST(receipt_id AS BLOB)) BETWEEN 1 AND 128
        AND receipt_id NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    schema TEXT NOT NULL CHECK(schema = 'jobsentinel.v3.privacy-receipt.v1'),
    receipt_json TEXT NOT NULL,
    stored_locally INTEGER NOT NULL CHECK(stored_locally = 1),
    data_left_device INTEGER NOT NULL CHECK(data_left_device IN (0, 1)),
    created_at TEXT NOT NULL,
    CHECK(length(CAST(receipt_json AS BLOB)) <= 8192),
    CHECK(json_valid(receipt_json)),
    CHECK(json_type(receipt_json) = 'object'),
    CHECK(COALESCE(json_extract(receipt_json, '$.schema'), '') = schema),
    CHECK(COALESCE(json_extract(receipt_json, '$.receipt_id'), '') = receipt_id),
    CHECK(
        COALESCE(json_type(receipt_json, '$.task_id'), 'missing') = 'text'
        AND length(CAST(
            json_extract(receipt_json, '$.task_id') AS BLOB
        )) BETWEEN 1 AND 128
        AND json_extract(receipt_json, '$.task_id')
            NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    CHECK(
        COALESCE(json_type(receipt_json, '$.pack_id'), 'missing') IN ('null', 'text')
        AND (
            json_type(receipt_json, '$.pack_id') = 'null'
            OR (
                length(CAST(
                    json_extract(receipt_json, '$.pack_id') AS BLOB
                )) BETWEEN 1 AND 128
                AND json_extract(receipt_json, '$.pack_id')
                    NOT GLOB '*[^-A-Za-z0-9._:]*'
            )
        )
    ),
    CHECK(
        COALESCE(
            json_type(receipt_json, '$.approval_reference'),
            'missing'
        ) IN ('null', 'text')
        AND (
            json_type(receipt_json, '$.approval_reference') = 'null'
            OR (
                length(CAST(
                    json_extract(receipt_json, '$.approval_reference') AS BLOB
                )) BETWEEN 1 AND 128
                AND json_extract(receipt_json, '$.approval_reference')
                    NOT GLOB '*[^-A-Za-z0-9._:]*'
            )
        )
    ),
    CHECK(
        COALESCE(
            json_type(receipt_json, '$.delete_or_revoke_action'),
            'missing'
        ) = 'text'
        AND length(CAST(json_extract(
            receipt_json, '$.delete_or_revoke_action'
        ) AS BLOB)) BETWEEN 1 AND 256
    ),
    CHECK(
        COALESCE(json_type(receipt_json, '$.labels'), 'missing') = 'array'
        AND json_array_length(receipt_json, '$.labels') > 0
    ),
    CHECK(
        COALESCE(
            json_type(receipt_json, '$.data_categories'),
            'missing'
        ) = 'array'
        AND json_array_length(receipt_json, '$.data_categories') > 0
    ),
    CHECK(json_extract(receipt_json, '$.stored_locally') IS stored_locally),
    CHECK(json_extract(receipt_json, '$.data_left_device') IS data_left_device)
);

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
    )
);

CREATE TABLE v3_compatibility_metadata (
    singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
    schema TEXT NOT NULL CHECK(schema = 'jobsentinel.v3.compatibility.v1'),
    compatibility_line INTEGER NOT NULL CHECK(compatibility_line = 3),
    database_schema INTEGER NOT NULL CHECK(database_schema = 2),
    migration_version INTEGER NOT NULL CHECK(migration_version >= 11)
);

INSERT INTO v3_compatibility_metadata (
    singleton, schema, compatibility_line, database_schema, migration_version
) VALUES (1, 'jobsentinel.v3.compatibility.v1', 3, 2, 11);
