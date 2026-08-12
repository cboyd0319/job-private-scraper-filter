CREATE TABLE v3_source_manifests (
    source_id TEXT PRIMARY KEY CHECK(
        length(CAST(source_id AS BLOB)) BETWEEN 1 AND 128
        AND source_id NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    policy_ref TEXT NOT NULL CHECK(
        length(CAST(policy_ref AS BLOB)) BETWEEN 1 AND 128
        AND policy_ref NOT GLOB '*[^-A-Za-z0-9._:]*'
    ),
    policy_revision INTEGER NOT NULL CHECK(policy_revision > 0),
    manifest_json TEXT NOT NULL CHECK(
        length(CAST(manifest_json AS BLOB)) BETWEEN 1 AND 65536
        AND json_valid(manifest_json)
        AND json_type(manifest_json) = 'object'
        AND COALESCE(
            json_extract(manifest_json, '$.schema'),
            ''
        ) = 'jobsentinel.v3.source-manifest.v1'
        AND COALESCE(json_extract(manifest_json, '$.source_id'), '') = source_id
        AND COALESCE(json_extract(manifest_json, '$.policy_ref'), '') = policy_ref
        AND COALESCE(
            json_extract(manifest_json, '$.policy_revision'),
            0
        ) = policy_revision
    ),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(source_id, policy_revision)
        REFERENCES v3_source_policy_ledger(source_id, revision)
);

CREATE TRIGGER v3_source_manifest_requires_current_policy
BEFORE INSERT ON v3_source_manifests
WHEN NOT EXISTS (
    SELECT 1
    FROM v3_source_policies
    WHERE source_id = NEW.source_id
      AND policy_ref = NEW.policy_ref
      AND revision = NEW.policy_revision
)
BEGIN
    SELECT RAISE(ABORT, 'source manifest must match the current policy');
END;

CREATE TRIGGER v3_source_manifest_same_revision_is_immutable
BEFORE INSERT ON v3_source_manifests
WHEN EXISTS (
    SELECT 1
    FROM v3_source_manifests
    WHERE source_id = NEW.source_id
      AND policy_revision = NEW.policy_revision
      AND (
          policy_ref <> NEW.policy_ref
          OR manifest_json <> NEW.manifest_json
      )
)
BEGIN
    SELECT RAISE(ABORT, 'source manifest revision is immutable');
END;

CREATE TRIGGER v3_source_manifest_update_requires_current_policy
BEFORE UPDATE ON v3_source_manifests
WHEN NOT EXISTS (
    SELECT 1
    FROM v3_source_policies
    WHERE source_id = NEW.source_id
      AND policy_ref = NEW.policy_ref
      AND revision = NEW.policy_revision
)
BEGIN
    SELECT RAISE(ABORT, 'source manifest must match the current policy');
END;

CREATE TRIGGER v3_source_manifest_revision_must_increase
BEFORE UPDATE ON v3_source_manifests
WHEN NEW.policy_revision <= OLD.policy_revision
BEGIN
    SELECT RAISE(ABORT, 'source manifest revision must increase');
END;

CREATE TRIGGER v3_source_manifest_identity_is_stable
BEFORE UPDATE ON v3_source_manifests
WHEN NEW.source_id <> OLD.source_id
BEGIN
    SELECT RAISE(ABORT, 'source manifest identity cannot change');
END;

CREATE TRIGGER v3_source_manifest_no_delete
BEFORE DELETE ON v3_source_manifests
BEGIN
    SELECT RAISE(ABORT, 'source manifest must be disabled through source policy');
END;

UPDATE v3_compatibility_metadata
SET migration_version = 15
WHERE singleton = 1;
