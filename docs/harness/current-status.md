<!-- Summarizes completed milestones, the active feature, and its next verified action. -->

# Current Status

Last updated: 2026-08-11

## Done

- Rust ownership is implemented across the declared crates. `jobsentinel-core`
  is deleted, storage hides its raw SQLx pool, and Tauri delegates product
  behavior through `jobsentinel-application`.
- Desktop, frontend, script, workflow, and maintained-file ownership match the executable contracts.
- Crate DRY remediation is complete. Maintained crate production duplication
  fell from 693 lines across 35 regions to zero, and crate test duplication fell
  from 2,184 lines across 79 regions to zero. The baselines are ratcheted to
  zero and the full local gate passed with structured completion evidence.
- Frontend DRY remediation is complete. Maintained production duplication fell
  from 778 lines across 38 regions to zero. Shared resume, score, dashboard,
  market, feedback, error, and desktop-adapter behavior now has canonical owners,
  and the full local gate passed.
- Residual cleanup is complete. Fixtures, file-size policy, records, dependencies, and Rust support have owners;
  maintained scopes have zero duplication, the full gate passed, and review recorded a bounded concerns verdict.
- The v2.9.5 GUI QA and release publication are complete. All 288 browser
  journeys and hosted release gates passed. The public release contains 20
  checksummed assets with SBOM and provenance validation. The no-account Mac
  package passed fresh-download installation and launch smoke verification.
- Gate 0 is approved. The comprehensive v3 master plan and its 230 canonical
  idea dispositions are the sole execution authority for the v3 major line.
- Milestone 0 reconciled release truth, debt, tests, dependencies, and release policy.
- Milestone 1 froze fail-closed contracts and evals, retained existing scheduler ownership, and passed native input.
- Milestone 2 passed the local data model, migration recovery, v2.9 preservation, and newer-data refusal.
- Milestone 3 passed encrypted portability, staged recovery, reviewed export, offline cleanup, Privacy Doctor,
  safe support, exact consent, governed Outside AI, platform health, and fail-closed publication.
- Milestone 4 passed Gate 3 with typed source governance, safe discovery and
  Workbench paths, one-use Browser Import, Smart Paste, and applied drafts.
- Milestone 5 passed local evidence, matching, model lifecycle, and Gate 4 decisions with later release proof explicit.
- Milestone 6 passed the offline case, daily workflow, reviewed native drops, protected answers,
  first-run choices, and desktop and narrow state matrix without hidden automation.

## In Progress

- Active feature: `v3-milestone-7-agent-pack-runtime`
- Status: `active`
- Current slice: Signed packs have bounded persistence, lifecycle, cleanup, reviewed execution, management, private
  evaluation scoring, and a verified static-skill reader behind caller-owned trust. Settings is read-only, and pack
  review exposes the key ID and immutable public-key fingerprint. Desktop startup owns the fixed platform app-data
  `pack-artifacts` path without creating it or loading trust. Database startup cancels expired reviewed-task approvals
  and fails interrupted started tasks before returning. Active-artifact reconciliation and skill UI remain unwired.
- Next action: Approve exact production key IDs, fingerprints, and ceilings. Then compile immutable trust and reconcile
  active artifacts at startup. Finish source-pack drop, lifecycle controls, product-target evaluation, reviewed local
  execution, and live platform proof.

## Deferred

- Hosted general CI remains absent under `pre-alpha-private-no-ci`.

Keep this current; put command history and long evidence under `docs/harness/evidence/`.
