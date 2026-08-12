-- Reviewed local packet claims retain only their opaque citations and the
-- revisions required to invalidate stale content. Resume text and field paths
-- remain outside this durable record.
CREATE TABLE v3_evidence_packets (
    packet_id TEXT PRIMARY KEY CHECK(
        length(CAST(packet_id AS BLOB)) = 36
        AND packet_id GLOB '????????-????-????-????-????????????'
        AND lower(packet_id) = packet_id
    ),
    case_file_id TEXT NOT NULL
        REFERENCES opportunity_case_files(case_file_id) ON DELETE RESTRICT,
    resume_id INTEGER NOT NULL CHECK(resume_id > 0),
    resume_revision TEXT NOT NULL CHECK(
        length(CAST(resume_revision AS BLOB)) BETWEEN 1 AND 128
    ),
    job_revision TEXT NOT NULL CHECK(
        length(CAST(job_revision AS BLOB)) BETWEEN 1 AND 128
    ),
    claim_id TEXT NOT NULL UNIQUE CHECK(
        length(CAST(claim_id AS BLOB)) = 36
        AND claim_id GLOB '????????-????-????-????-????????????'
        AND lower(claim_id) = claim_id
    ),
    reviewed_text TEXT NOT NULL CHECK(
        length(CAST(reviewed_text AS BLOB)) BETWEEN 1 AND 8192
        AND length(trim(reviewed_text)) > 0
    ),
    local_only INTEGER NOT NULL DEFAULT 1 CHECK(local_only = 1),
    sensitive INTEGER NOT NULL DEFAULT 1 CHECK(sensitive = 1),
    created_at TEXT NOT NULL
);

CREATE TABLE v3_evidence_packet_evidence (
    packet_id TEXT NOT NULL
        REFERENCES v3_evidence_packets(packet_id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL CHECK(ordinal BETWEEN 0 AND 31),
    evidence_id TEXT NOT NULL CHECK(
        length(CAST(evidence_id AS BLOB)) = 64
        AND evidence_id NOT GLOB '*[^0-9a-f]*'
    ),
    PRIMARY KEY(packet_id, ordinal),
    UNIQUE(packet_id, evidence_id)
);

CREATE TABLE v3_evidence_packet_boundaries (
    packet_id TEXT NOT NULL
        REFERENCES v3_evidence_packets(packet_id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL CHECK(ordinal BETWEEN 0 AND 1),
    boundary TEXT NOT NULL CHECK(boundary IN (
        'clearance_currentness_unverified',
        'military_civilian_equivalence_unverified'
    )),
    PRIMARY KEY(packet_id, ordinal),
    UNIQUE(packet_id, boundary)
);

CREATE INDEX idx_v3_evidence_packets_case_created
    ON v3_evidence_packets(case_file_id, created_at, packet_id);

UPDATE v3_compatibility_metadata
SET migration_version = 22
WHERE singleton = 1;
