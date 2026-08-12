-- Local derived vectors use the frozen sqlite_blob_v1 storage format.
-- Source text is intentionally not stored in this table.
CREATE TABLE v3_local_vectors (
    subject_id TEXT PRIMARY KEY
        CHECK (
            typeof(subject_id) = 'text'
            AND length(CAST(subject_id AS BLOB)) BETWEEN 1 AND 128
        ),
    freshness_json TEXT NOT NULL
        CHECK (
            typeof(freshness_json) = 'text'
            AND length(CAST(freshness_json AS BLOB)) BETWEEN 2 AND 4096
        ),
    dimension INTEGER NOT NULL
        CHECK (typeof(dimension) = 'integer' AND dimension BETWEEN 1 AND 4096),
    vector_blob BLOB NOT NULL
        CHECK (
            typeof(vector_blob) = 'blob'
            AND length(vector_blob) = dimension * 4
        ),
    revision INTEGER NOT NULL DEFAULT 1
        CHECK (typeof(revision) = 'integer' AND revision > 0),
    updated_at TEXT NOT NULL CHECK (typeof(updated_at) = 'text')
);
