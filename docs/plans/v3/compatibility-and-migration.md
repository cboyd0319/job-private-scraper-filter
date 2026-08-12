# Compatibility And Migration

V3.0.0 is the first JobSentinel release that makes backward compatibility a
product promise. Releases before v3.0.0 have no supported forward-migration,
rollback, or schema-compatibility guarantee. V3 is the line where architecture,
data contracts, package identity, and migration discipline become stable enough
for users to rely on.

Rust remains the base runtime for this compatibility line. Compatibility
contracts may expose CLI, browser companion, MCP, source-pack, model-pack, or UI
interfaces, but the durable validation, migration, storage, and privacy
boundaries should stay Rust-owned.

## Compatibility Boundary

| Release range | Compatibility posture |
| --- | --- |
| Before v3.0.0 | Unsupported as an upgrade source. No forward-migration, rollback, or durable plugin, source-pack, model-pack, database, settings, API, or artifact compatibility promise. |
| v3.0.0 and later v3.x | Stable long-term contracts inside the v3 compatibility line, documented migrations, backup-before-upgrade, and supported rollback where the data contract allows it. |
| Future major versions | Explicit migration plan from the previous long-term line, with compatibility breaks documented before release. |

## What V3 Must Stabilize

V3 should define durable contracts for:

- SQLite schema migration and downgrade policy
- app settings and user preferences
- saved searches
- source manifests and source packs
- restricted-source acknowledgement records
- browser companion protocol and permissions
- model lockfiles and model cache metadata
- vector provenance and stale-vector rules
- Agent Skill package expectations
- agent, workflow, role, regional, source, rubric, eval, template, and OS helper
  pack manifests
- external AI provider settings shape
- privacy labels and receipts
- safe support report schema
- backup and restore format
- import and export package format
- edition manifests and package metadata
- region manifests, taxonomy bridges, CV profiles, pay normalization, and
  location normalization contracts
- vector store provenance, freshness keys, and rebuild semantics

## Rollback Promise

Rollback support should be explicit and limited:

- V3 should support rolling back to the previous compatible v3 patch or minor
  release when no irreversible migration has occurred.
- If a migration cannot support rollback, JobSentinel must create a backup first
  and say that rollback will restore from backup rather than downgrade in place.
- Rollback should not promise compatibility with pre-v3 releases.
- Optional model packs and source packs should be removable independently of app
  rollback.
- The app should detect newer unsupported data and stop with recovery guidance
  instead of corrupting local data.

## Migration Rules

Migrations from v3.0.0 onward should:

- be versioned and deterministic
- run locally
- avoid sending data outside the device
- create a verified backup before destructive or irreversible changes
- record migration provenance
- keep user-visible recovery instructions
- include tests for fresh install, upgrade, failed migration, retry, backup
  restore, and unsupported-newer-data handling

## Pre-V3 Boundary

JobSentinel v3.0.0 does not read, migrate, restore, or roll back databases,
settings, APIs, packs, or artifacts created before v3.0.0. Release closure must
remove pre-v3 readers, migration shims, fixtures, and user-facing claims rather
than carry unsupported compatibility code into the long-term line.

## Architecture Implications

Because v3 becomes the compatibility boundary, major refactors should happen
before or during v3.0.0, not immediately after it. This includes:

- event ledger
- opportunity case files
- source graph
- browser companion protocol
- model governance contracts
- privacy receipts
- edition manifests
- pack manifests
- region manifests and taxonomy bridges
- vector store contract
- backup and export format

If a refactor is likely to require breaking data contracts, decide before
v3.0.0 ships. After v3.0.0, the bar for breaking changes should be much higher.

## Verification

Before v3.0.0 release:

- fresh install works
- v3.0.0 establishes the first supported data and settings baseline
- no pre-v3 compatibility reader, shim, fixture, or release claim remains
- v3 patch rollback works for a compatible migration
- unsupported-newer-data detection works
- source packs survive compatible upgrades
- model packs survive compatible upgrades
- regional packs survive compatible upgrades
- vector indexes detect stale or incompatible data and rebuild safely
- browser companion permission reset works when required
- backup export and restore work across Windows 11+, macOS, and Linux
