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
- Current slice: Pack persistence, lifecycle, cleanup, reviewed execution, management facts, and private evaluation
  scoring exist behind caller-owned runtime inputs; Settings remains read-only. Source and runtime metadata identify
  unreleased 3.0.0, and exact compatible packs reach the production parser. A bounded application reader reopens a
  Ready static Agent Skill only after generation, trust, signature, artifact, and self-test checks, returning plain
  signed text, resources, and an advisory handoff without execution. Pack review now exposes and validates the immutable
  publisher public-key fingerprint as well as its key ID. Startup reconciliation and skill UI are unwired.
- Next action: Approve the exact production publisher key IDs, public-key fingerprints, and capability ceilings. Then
  bind the production artifact root and startup.
  Finish source-pack drop, lifecycle controls, product-target evaluation and reviewed local execution, and live
  platform proof.

## Deferred

- Hosted general CI remains absent under `pre-alpha-private-no-ci`.

Keep this current; put command history and long evidence under `docs/harness/evidence/`.
