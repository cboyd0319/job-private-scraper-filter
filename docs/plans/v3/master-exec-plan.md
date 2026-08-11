<!-- Defines the ordered v3 implementation, verification, compatibility, and closure authority. -->

# JobSentinel V3 Master Execution Plan

Last updated: 2026-07-22.

This is the sole execution plan for the v3 major line. The other files in this
directory provide product, architecture, research, and evaluation detail. They
do not independently authorize implementation or add scope.

## Problem

The repository has a broad v3 strategy package, a ranked 230-item idea index,
one narrow foundation plan, public-roadmap commitments, research follow-ups,
open or stale debt records, and deferred release work. Those sources do not
form one ordered, implementation-ready plan. Starting work from any one of them
would risk building dependent features in the wrong order, freezing an unstable
compatibility contract, or silently losing unfinished commitments.

V3 needs one durable plan that:

- defines what ships in v3.0 and later v3.x releases;
- orders architecture, migration, product, security, evaluation, and release work by dependency;
- gives every maintained unfinished commitment one explicit disposition;
- keeps only one active feature at a time;
- proves user value and recovery, not only implementation completion.

## Scope

In scope:

- The complete v3.0 foundation and first user-visible vertical release.
- Ordered v3.x enhancement work after the v3.0 compatibility boundary ships.
- Every `Now` and `Next` item in [Idea Index](idea-index.md), consolidated into bounded workstreams.
- Unfinished commitments from maintained roadmaps, plans, feature docs,
  research docs, testing docs, security docs, release docs, and debt tracking.
- Military-transition and veteran workflows that preserve current resume,
  evidence, clearance, and protected-answer behavior while adding reviewed
  civilian-role and public-service paths.
- A clean v3.0.0 compatibility baseline, backup, restore, compatible v3
  rollback, and unsupported-newer-data handling, with no pre-v3 upgrade promise.
- Product, privacy, security, accessibility, performance, documentation, packaging, supply-chain, and release evidence.

Out of scope:

- `Later` and `Moonshot` ideas unless a recorded v3 discovery proves they are necessary for an accepted milestone.
- Ideas prohibited by the cut lines in [Idea Index](idea-index.md).
- Hidden restricted-source monitoring, stored restricted-session material,
  control bypasses, fabricated resume evidence, opaque hiring predictions, or
  final application submission without user review.
- Hosted accounts, telemetry, required external AI, or required large-model downloads for core workflows.
- Claiming externally blocked distribution guarantees without the required credentials and live evidence.

## Success Criteria

- Observable result: v3.0 ships one coherent local-first job-search system built
  around opportunity case files, explainable evidence, safe source workflows,
  recovery, and stable compatibility contracts.
- User-ease result: a nontechnical job seeker can install, reach a useful
  workflow, import or find a job, decide whether it is worth pursuing, prepare
  truthful materials, track progress, recover from failure, and export their
  data without a hosted account.
- Veteran result: a veteran can carry user-confirmed military service,
  occupations, credentials, and current clearance evidence into civilian
  workflows without invented claims or inferred eligibility, and retains
  control over voluntary or protected veteran-status answers.
- Offline result: installed core workflows, local records, recovery, imports,
  and update recovery remain usable or fail clearly when the network is
  unavailable.
- Scope result: all 105 `Now` ideas are covered by v3.0 milestones; all 82
  `Next` ideas are assigned to ordered v3.x trains; all 34 `Later` and 9
  `Moonshot` ideas are explicitly deferred; prohibited ideas remain retired.
- Backlog result: every unfinished maintained-doc commitment listed in
  [Completion Inventory](#completion-inventory) is completed, deferred with a
  trigger, or retired with evidence.
- Compatibility result: fresh v3.0.0 install, failed v3.x migration recovery,
  backup restore, compatible v3 rollback, and newer-data refusal are proven
  without silent data loss. Pre-v3 compatibility surfaces are absent.
- Verification result: every milestone has focused evidence, every release cut
  passes the full applicable local and hosted gates, and final GUI QA covers
  every shipped action.

## Audience And Ease

- Primary user: technical and nontechnical job seekers, including military
  veterans, people transitioning to civilian employment, career changers, and
  people in long searches.
- Technical knowledge assumed: none for install, setup, normal workflows, recovery, update, export, or support.
- Broad job-seeker fit: examples, fixtures, taxonomies, pay models, credentials,
  and source packs must cover multiple job families and work arrangements.
- Support and recovery: every failure offers a plain next action, preserves
  local data, and provides a reviewable safe support report when useful.

## Product And Trust Contract

- Rust remains the durable local trust boundary. Tauri remains a thin desktop
  and IPC shell. React feature owners render typed application contracts.
- User data stays local unless the user explicitly configures and approves an external channel.
- External AI stays optional, disabled by default, previewed, editable,
  cancellable, redacted, approved, metadata-logged, and gateway-routed.
- Job postings, resumes, browser content, packs, and model output are untrusted inputs.
- Every recommendation names evidence, uncertainty, blockers, and the local data used.
- Every durable write, external send, restricted-source action, sensitive
  export, and final application action remains visible and user controlled.
- Veteran status, service history, clearances, and preference or benefit
  eligibility remain sensitive user-owned facts. JobSentinel does not infer
  them, choose protected answers, or expose them without explicit user action.
- Core workflows retain deterministic local fallbacks.
- Accessibility, data-loss handling, credential safety, source pacing, and
  platform parity are release requirements, not later polish.
- V3 contracts favor deletion and existing owners over new crates, dependencies, daemons, services, or plugin runtimes.

## Authority And Change Control

1. `scripts/harness/state/feature-list.json` owns the single active feature.
2. This file owns v3 sequence, scope, decisions, progress, and handoff.
3. [V3 Planning](README.md) indexes strategy and research references.
4. [Idea Index](idea-index.md) owns idea descriptions and priority labels.
5. Closest feature and security docs own shipped behavior.
6. A reference document cannot add release scope without a decision recorded here.
7. Activate one milestone or one bounded child feature at a time. Record its
   acceptance behavior, paths, verification, and rollback before code changes.
8. Scope changes require an updated disposition, dependency review, and user approval when they change the release boundary.
9. Only verification can move a milestone or child feature to `passing`.

## Completion Inventory

| Source | Unfinished or conflicting work | Disposition |
| --- | --- | --- |
| `README.md`, `docs/README.md`, `docs/ROADMAP.md`, `docs/features/capabilities.md`, `docs/releases/v2.9.5.md`, `CHANGELOG.md` | Maintained release records identify v2.9.5 as the current published release. | Milestone 0 corrected the stale claims and added a focused marker audit. |
| Root `ROADMAP.md` | Resume readability, source verification, repost lineage, pay protection, long-search support, bias-aware routes, protective UX, and local control remain planned. | Milestones 4 through 9 implement them; Milestone 11 evaluates them. |
| `docs/plans/tech-debt-tracker.md` DRY-002 | The small direct dependency review is complete. | Milestone 0 replaced direct `scopeguard` with existing `tempfile` ownership and retained three dependencies with evidence-backed rationales. |
| `docs/plans/tech-debt-tracker.md` DRY-003 | The live duplication gate reports zero across all six maintained scopes. | Milestone 0 closed the stale item with fresh evidence. |
| Deferred release-pipeline plan | Publication through `github.token` cannot trigger the separate `release.published` verifier, the current verifier is authenticated, and its macOS job lacks platform scoping. | Milestone 11 makes authenticated draft verification blocking before publication, restores platform scoping, and retains explicit unauthenticated post-publication verification. |
| `docs/harness/current-status.md` | Shared-history CI is deferred under an explicit user override. | Keep the exception visible. Milestone 11 requires a fresh user decision before v3 release and records the enforcement gap if retained. |
| `docs/developer/TESTING.md` | The stale generic future-work list is removed after reconciling it with live pipeline, property, mock-server, E2E, and contract coverage. | Milestones 1 and 11 add only measured performance budgets and risk-based mutation checks; generic benchmark and snapshot frameworks remain unapproved. |
| `tests/e2e/README.md` | Drag-and-drop relies on retries; native file-picker behavior lacks installed-app smoke coverage. | Milestone 1 makes drag-and-drop deterministic and adds a native smoke owner. |
| Research open evaluations | Ghost, pay, negotiation, behavior, source, resume, and screening-system studies contain unevaluated product questions. | Milestone 1 creates versioned eval sets; feature milestones add fixtures before implementation; Milestone 11 runs the accepted evaluations. |
| Resume feature and research docs | Broader evidence strength, synonym handling, recency, seniority, profession weighting, YAML import, static export, draft history, and a public-profile importer remain future work. | Core evidence and weighting ship in Milestone 5. Import/export extensions enter the v3.x train in Milestone 12. |
| Veteran and military-transition surfaces | The Military Transition resume template, military-service evidence, truthful clearance checks, protected veteran-status answer handling, and veteran-aware U.S. public-service research exist across maintained product, skill, and planning owners, while R10 was deferred. | Milestones 1, 4, 5, 6, 9, and 11 preserve, connect, expand, and verify them as accepted v3.0 behavior without inferring protected status, eligibility, or civilian equivalence. |
| Job-source research and feature docs | Source registry, permission notes, pacing, stop conditions, freshness comparison, and source review remain open. | Milestone 4 owns the source graph, policy ledger, fixtures, and browser protocol. |
| Employer-intelligence plan | Dataset mode, starter sources, Essentials size, manual observations, expiry, and minimum dossier remain open decisions. | Decision Gate 4 resolves them before Milestone 8 implementation. |
| Security docs | Future private-data AI, encrypted-secret backup, and stricter local-auth choices are conditional, not approved behavior. | Milestones 3 and 7 may promote only reviewed paths with focused threat models and local fallbacks. |
| Distribution-dependent platform work | Start-on-login, sandboxing, signing, notarization, and final native proof remain incomplete or externally constrained. | Design and fixture work stays late. Live platform and distribution proof is part of Milestone 11 only. |
| Completed and archived plans | Completed plans are historical. Milestone 0 resolved the release-pipeline plan's remaining decision. | Do not reopen historical checklists. Milestone 11 owns only the recorded release-pipeline corrections above. |
| Checklists and boundary language | Security, contribution, support, and skill checkboxes are reusable procedures; phrases such as “not yet submitted” describe state. | Not backlog. Keep them as verification or user guidance. |

The inventory covers maintained root state, roadmaps, plan references, feature
docs, research, developer and testing guidance, security guidance, and release
records. Completed or archived material remains historical unless it contains an
explicit unresolved commitment named above. New maintained commitments must add
or update an inventory row before implementation.

## Idea Disposition

The IDs below come from [Idea Index](idea-index.md). An accepted ID is a
requirement to satisfy, not necessarily a separate screen, table, or abstraction.

### V3.0 Accepted

| Milestone | Accepted `Now` IDs |
| --- | --- |
| 1 | A01, A07, A08, A09, A11, A12, A13, A16, A18, C01, C02, C03, C04, C05, C06, C07, C08, C10, C11, C12, C13, C14, C15, C16 |
| 2 | A02, A03, A04, A05, A10, A15 |
| 3 | P01, P02, P04, P06, P07, P09, E11, E12, N10, N20, R14 |
| 4 | A06, S01, S02, S03, S06, S08, S11, S12, S13, S14, S16, S17, S19, S22, S26, S27 |
| 5 | M01, M02, M03, M05, M10, M11, M12, M16, M18, M19, M21, M22, M27, X05 |
| 6 | U01, U02, U11, U12, U13, U14, U17, G12, O07, N07, R01, R02 |
| 7 | G01, G05, G06, G11, G16, G17, G18, G19, P10, P11, E13 |
| 8 | U20, X21 |
| 9 | R04, R05, R06, R07, R10, R17, E01, E03 |
| 10 | E02 |

### V3.x Accepted After V3.0

Milestone 12 uses this default dependency order. Each ID or tightly coupled
group remains a separately activated child feature.

| Order | V3.x train | Accepted `Next` IDs | Dependency rule |
| --- | --- | --- | --- |
| 12A | Campaign operating loop | U03, U04, U05, U06, U07, U08, U09, U10, U15, U19, U21 | Extends the core U17 outcome after stable v3.0 case-file and event evidence. |
| 12B | Source reach and reliability | S04, S05, S07, S09, S10, S15, S18, S21, S23, S24, S28 | Preserves Gate 3 policy, pacing, permission, and revocation contracts. |
| 12C | Application, interview, and offer quality | M04, M06, M07, M08, M09, M15, M17, M20, M23, O01, O02, O05, O06, O11, O12 | Uses frozen evidence and outcome contracts; never invents candidate evidence. |
| 12D | Privacy and governance | P05, P08, P12, P14 | Requires focused threat models and local fallbacks. |
| 12E | Safe automation | G02, G03, G04, G07, G08, G09, G10, G13, G15 | Cannot widen the v3.0 capability boundary without a major-version deferral. |
| 12F | Ecosystem and distribution | A19, E04, E05, E06, E07, E10, E14, E15 | Uses signed, compatible pack and edition contracts; contract-derived outputs require stable schemas and maintained-duplication evidence. |
| 12G | Native integration | A14, N01, N02, N03, N06, N08, N14, N15, N16, N18, N19 | Builds the adapter contract before individual integrations. |
| 12H | Regional and access expansion | E16, R03, R08, R09, R11, R13, R15, R16 | Extends manifests, offline bundles, inclusive modes, and fixtures without claiming unsupported completeness. |
| 12I | Frontier and benchmark | M26, X02, X07, X18, C09 | Requires stable internal evals and measured value, and cannot displace accepted safety or recovery work. |

### Explicitly Deferred Beyond V3

- `Later`: A17, A20, A21, E08, G14, M13, M14, M24,
  N04, N05, N09, N11, N12, N13, N17, O03, O04, O08, O09, O10, P03, P13,
  R12, S20, S25, U16, U18, X08, X09, X10, X13, X15, X16, X17.
- `Moonshot`: M25, P15, X01, X06, X11, X12, X14, X19, X20.
- Promotion trigger: a completed v3 discovery must prove user value, data and
  trust boundaries, architecture ownership, focused evals, rollback, and no
  displacement of accepted work.

### Retired

The following remain prohibited: hidden restricted-source monitoring, stored
restricted-session credentials, background account-backed refresh, final
submission without review, fabricated resume claims, control evasion, broad
pack privileges, and opaque hiring-probability claims.

## Constraints And Risks

| Risk | Required control |
| --- | --- |
| V3 freezes the wrong contract | Complete Milestone 1 decision records and compatibility fixtures before schema or protocol freeze. |
| Migration damages local data | Verified encrypted backup, integrity checks, retry, restore, and unsupported-newer-data refusal are blocking tests. |
| Event or graph systems duplicate private text | Typed event allowlists, metadata size limits, privacy labels, redaction tests, and no raw resume, note, credential, or provider payload fields. |
| Browser or source work exceeds safe access | Source class, policy reference, rate limit, visible user action, loopback pairing, revocation, and stop-condition tests precede adapters. |
| Model output becomes authoritative | Evidence citations, blocker caps, deterministic fallback, stale-vector detection, calibration, and manual explanation review. |
| Packs or agents widen privilege | Signed manifests, capability grants, quarantine, denial tests, approval gates, local logs, uninstall, and cleanup. |
| Veteran workflows stereotype service, overstate civilian equivalence, expose protected status, or infer eligibility | User-confirmed evidence, provenance for occupation and credential mappings, no fabricated claims, sensitive-field controls, current public-source review, diverse fixtures, and manual explanation review. |
| Scope becomes unfinishable | One active child feature, hard release cut lines, dependency gates, and promotion only through this plan. |
| Modest hardware is excluded | Essentials profile, no required model download, bounded memory and storage tests, and responsive fallback paths. |
| Documentation diverges | Closest feature docs and generated contracts update in the same milestone; stale release claims fail Milestone 0. |
| Platform-specific work consumes the stream early | Keep native distribution proof in the final release milestone unless an earlier contract test requires a fixture. |

## Orchestration

- The coordinating agent owns this plan, active state, integration, and final
  verification.
- Keep tightly coupled migration and contract work local.
- Delegate only bounded, independent work with disjoint file ownership when the
  user, platform, and active profile authorize it.
- Every delegated result receives local diff review and verification.
- Use a specification reviewer after each milestone plan and an adversarial
  reviewer for migrations, security boundaries, browser protocols, agents,
  packs, compatibility, and release claims.

## Decision Gates

| Gate | Decision required | Blocks |
| --- | --- | --- |
| 0 | User approves this master scope and Milestone 0 activation. | All implementation |
| 1 | Freeze v3 identifiers, privacy labels, manifest schemas, backup/export envelope, compatibility policy, vector backend, and executable-pack class. Confirm that existing application and Tokio owners can run typed jobs; approve A20 only if focused evidence proves an ownership or runtime gap. | Milestones 2 through 10 |
| 2 | Approve the minimum case-file, event, graph, receipt, and policy data model after migration prototypes. | Milestones 3 through 8 |
| 3 | Approve browser protocol, source classes, permission model, revocation, and replacement or coexistence of existing capture tools after threat-model tests. | Browser and source UI |
| 4 | Approve model thresholds, local runtime and provider matrix, setup defaults, employer datasets, regional delivery mode and sources, expiry, and Essentials footprint from eval evidence. | Milestones 5, 8, and 9 release claims |
| 5 | Approve edition names, component matrix, update and rollback UX, and v3.0 feature cut. | Milestones 10 and 11 |
| 6 | Approve final release after full evidence, known gaps, and external blockers are recorded. | Publication |

## Milestones

| Milestone | Required work | Owners and exit |
| --- | --- | --- |
| 0. Planning authority and repository truth | [x] Verify this file remains active in the plan index, v3 planning, plans hub, current status, and feature state.<br>[x] Correct stale v2.9.5 release records.<br>[x] Reconcile DRY-002 and close stale DRY-003 with live evidence.<br>[x] Reconcile testing future-work claims against existing coverage.<br>[x] Re-audit the deferred release-pipeline decision.<br>[x] Add a deterministic idea-disposition check and a focused docs-marker audit for inventory drift.<br>[x] Record Gate 0 and activate Milestone 1.<br>Verify: `npm run harness:check`, `npm run lint:docs`, `npm run lint:deps:why`, `npm run lint:dup -- --list`, `npm run lint:file-size`, and `git diff --check`. | Planning and harness owners.<br>Exit: one active plan, one active feature, no contradictory current-state docs, and no unowned open debt. |
| 1. Contract, evaluation, and quality baseline | [x] Define versioned Rust-owned schemas for compatibility, manifests, privacy receipts, agent tasks, packs, regions, editions, model provenance, and vector freshness.<br>[x] Add fixtures for current v2.9, malformed, unsupported-newer, and forward-compatible unknown input.<br>[x] Create public or synthetic eval sets for source truth, resume evidence, pay clarity, posting risk, employer context, accessibility, recovery, modest hardware, military-to-civilian evidence, protected veteran-status answers, and commercial comparison.<br>[x] Prove whether existing application and Tokio owners satisfy typed-job scheduling, cancellation, recovery, observability, and resource-bound requirements; keep A20 deferred unless the evidence shows a concrete gap.<br>[x] Make drag-and-drop E2E deterministic and add installed-app native file-picker smoke coverage.<br>[x] Set benchmark budgets and use mutation testing only where it can disprove high-risk test quality.<br>[x] Complete Gate 1. | `jobsentinel-domain`, security, local AI, platform, harness contracts, tests, and closest docs.<br>Exit: schemas and evals fail closed, are versioned, require no database migration, and do not add a runtime kernel without evidence. |
| 2. V3 local data foundation | [x] Write fail-first tests for case-file create/reuse, sanitized events, metadata limits, privacy receipts, source policy upsert, compatibility reads, failed migration, retry, and backup restore.<br>[x] Add append-only SQLx migrations for case files, typed events, career/source graph records, receipts, source policies, and compatibility metadata.<br>[x] Exclude raw resumes, notes, credentials, browser storage, provider payloads, and unrelated history from event metadata.<br>[x] Add application-layer operations without frontend commands.<br>[x] Prove fresh install and v2.9 fixture migration, then complete Gate 2. | Storage, domain, application, and `.sqlx/`.<br>Exit: the foundation is local, typed, bounded, migration-safe, and has no renderer surface. |
| 3. Recovery, portability, policy, and repair | [x] Implement encrypted backup and restore, migration provenance, compatible rollback, newer-data refusal, export without lock-in, and storage cleanup.<br>[x] Make recovery, local record access, queued local work, and repair guidance available without network access; identify actions that require connectivity before the user commits work.<br>[x] Build Privacy Doctor, safe support bundle, policy ledger, smart consent, and governed external-send audit.<br>[x] Build the remaining offline recovery and repair services.<br>[x] Keep secret export disabled unless a separate design passes threat review and explicit approval.<br>[x] Add platform-health and package-repair contracts with plain fallbacks. | Storage, security, credentials, application, platform, Settings, and user-data docs.<br>Exit: destructive and irreversible paths have backup, restore, cancel, offline, support, and test evidence. |
| 4. Source graph and browser companion | [x] Implement source identity, class, policy, rate limit, permission, fixture, confidence, freshness, lineage, and stop-condition records.<br>[x] Consolidate official APIs, public ATS pages, employer discovery, regional packs, restricted Workbench, visible capture, smart paste, and applied logging behind the graph.<br>[x] Add reviewed veteran and U.S. public-service source and guidance metadata with provenance, dates, eligibility boundaries, and no claim of complete coverage.<br>[x] Prototype secure loopback pairing, scoped permissions, revocation, replay protection, and no stored restricted-session material.<br>[x] Add a source simulator, policy fixtures, robots and terms review records, freshness comparison, and adversarial protocol tests.<br>[x] Complete Gate 3 before browser UI. | Sources, network, security, application, platform, browser assets, Settings, and source docs.<br>Exit: every source action is classified, paced, visible, revocable, and explainable. |
| 5. Local evidence, resume, and matching engine | [x] Build a provenance-aware evidence graph shared by resume, requirement, packet, and case-file workflows.<br>[x] Add requirement states, hard constraints, seniority, recency, profession and regional profiles, transparent blockers, and "why not" diagnostics.<br>[x] Preserve the Military Transition template and military-service evidence, then add reviewed mappings from user-confirmed military occupations, responsibilities, credentials, and current clearances to civilian wording without inventing equivalence or claims.<br>[x] Calibrate Qwen3 retrieval and reranking against frozen evals while preserving deterministic fallback and stale-vector repair.<br>[x] Guard jobs, resumes, models, and packs against prompt injection and poisoned input.<br>[x] Implement Model Doctor, match debugger, feedback capture, and evidence-bound packets.<br>[x] Complete Gate 4 model and data decisions. | Documents, intelligence, local AI, assistance, storage, resume UI, and matching docs.<br>Exit: every match claim is evidence-linked, bounded, reproducible, and useful without external AI. |
| 6. Opportunity case file and daily workflow | [x] Ship plain-language first run, one useful initial search path, and clear skip and recovery choices.<br>[x] Make the case file the shared view for job, source, risk, evidence, packet, application, interview, contact, offer, and outcome state.<br>[x] Deliver the core campaign operating model through the mission board, timeline, evidence wall, decision summary, "why not this job," "prepare this job," debrief, and Rust-owned drag-and-drop import for resumes, job postings, and encrypted backups; leave source-pack drop completion with its Milestone 7 quarantine and installer, and leave inbox, simulation, and adaptive campaign extensions to train 12A.<br>[x] Keep the local campaign, saved evidence, drafts, and review actions useful offline, with explicit stale and connectivity-required states for source actions.<br>[x] Preserve user control over voluntary or protected veteran-status answers, clearance claims, and eligibility questions throughout application review.<br>[x] Preserve focused feature ownership and typed application commands.<br>[x] Test empty, partial, duplicate, offline, failed-source, and restored-data states at desktop and narrow layouts. | Application, storage, assistance, `src/features/`, shared UI, and closest docs.<br>Exit: the primary v3 campaign works end to end without hidden automation or required connectivity. |
| 7. Agent and pack runtime | [ ] Implement local skill execution, reviewed task plans, bounded resume and packet agents, failure views, and eval packs.<br>[ ] Define signed manifests, capability grants, quarantine, self-test, install, update, disable, uninstall, cleanup, and source-pack drag-and-drop import.<br>[ ] Limit v3.0 executable packs to reviewed typed actions; keep generic script or dynamic adapter execution deferred until sandbox denial tests and an explicit future promotion pass.<br>[ ] Deny broad shell, filesystem, network, credential, and external-send access by default.<br>[ ] Test injection, malformed packs, signatures, downgrade, revocation, replay, partial install, and rollback.<br>[ ] Keep static Agent Skills compatible and external AI gateway-bound. | Application, security, AI, assistance, platform, `skills/`, pack UI, and security docs.<br>Exit: agents and packs cannot exceed visible user-approved capabilities. |
| 8. Employer, pay, and outcome intelligence | [x] Resolve all employer-intelligence open decisions at Gate 4.<br>[ ] Build the minimum employer dossier from official public sources, provenance, freshness, user-owned observations, and local outcomes.<br>[ ] Integrate source verification, posting history, pay clarity, scam response, application channel, interview context, and offer evidence without verdicts or central private reviews.<br>[ ] Revalidate volatile legal, policy, pay, and public-data claims before use. | Intelligence, sources, storage, salary, application, company research, and research docs.<br>Exit: guidance shows source, date, uncertainty, and safe next actions. |
| 9. Regions, access, editions, and first-run doctor | [ ] Define region manifests, taxonomy bridges, location and pay normalization, CV profiles, public-source fixtures, and starter packs.<br>[ ] Validate starter coverage without claiming regional completeness.<br>[ ] Implement Essentials and stronger-local profiles, download chooser, first-run doctor, model-free startup, and in-place model upgrade.<br>[ ] Ship the veteran and military-transition path across onboarding, resumes, matching, source guidance, application review, and case files; include disability-aware and early-career variants without treating veterans as one profile.<br>[ ] Cover role families, work modes, credentials, pay types, and accessibility needs in examples and tests.<br>[x] Complete Gate 4 regional and footprint decisions.<br>[ ] Complete Gate 5 edition decisions. | Domain, sources, documents, intelligence, local AI, platform, onboarding, Settings, packaging, and region docs.<br>Exit: modest hardware and model-free installs complete the core journey. |
| 10. Update, rollback, repair, and distribution design | [ ] Implement explicit update availability, package verification, compatible rollback, repair, and component cleanup without silent updates.<br>[ ] Keep the installed application and compatible local data usable when update checks, downloads, or online repair are unavailable.<br>[ ] Bind app, edition, pack, model, browser, region, export, and database compatibility metadata.<br>[ ] Test interrupted update, checksum failure, unsupported package, missing component, rollback restore, and offline recovery.<br>[ ] Freeze the v3.0 feature and component cut at Gate 5. | Application, platform, security, storage, release scripts, Settings, updater docs, and release docs.<br>Exit: update and repair are reversible, inspectable, offline-safe, and need no developer instructions. |
| 11. V3.0 integration, QA, and release | [ ] Run all end-to-end scenarios and commercial benchmarks in the evaluation bar.<br>[ ] Perform full GUI QA for every shipped action, recovery path, keyboard flow, accessibility state, responsive layout, and installed-app boundary.<br>[ ] Prove the veteran journey from military-service evidence and civilian-role review through source guidance, protected application answers, case-file tracking, and export.<br>[ ] Prove fresh v3.0.0 install, failure recovery, compatible v3 rollback, export/import, pack and model removal, and newer-data refusal.<br>[ ] Resolve release-pipeline and shared-history enforcement decisions; record any retained exception as a gap.<br>[ ] Update all front-door, product, security, migration, release, and support docs.<br>[ ] Verify assets, checksums, SBOMs, attestations, archives, labels, and public downloads.<br>[ ] Perform final live platform and distribution proof last, then complete Gate 6. | All affected owners.<br>Exit: every v3.0 success criterion and the [Evaluation And Release Bar](evaluation-and-release-bar.md) done definition has revision-bound evidence. |
| 12. V3.x enhancement trains | [ ] Activate one `Next` ID or tightly coupled group at a time.<br>[ ] Preserve v3 compatibility; defer breaking changes to the next major line.<br>[ ] Apply the same red-test, privacy-label, migration, docs, GUI QA, and release-evidence rules as v3.0.<br>[ ] Use the default train order; within an eligible train, sequence by measured user burden, not novelty.<br>[ ] Reorder trains only through a recorded dependency and user-value decision.<br>[ ] Re-score after each minor release without silently promoting `Later` or `Moonshot` work. | Closest canonical owners.<br>Exit: every accepted `Next` ID is shipped, moved with evidence, or retired by user-approved decision. |
| 13. V3 line closure | [ ] Confirm every accepted ID and inventory item has a final disposition.<br>[ ] Remove transition-only `v3` prefixes from ordinary files, modules, types, commands, and live schema names; retain version labels only in immutable compatibility identifiers where the version is part of the contract.<br>[ ] Delete pre-v3 compatibility readers, migration shims, fixtures, and release claims; make v3.0.0 the first supported compatibility baseline.<br>[ ] Audit Rust workspace and module organization against canonical crate ownership, Cargo workspace dependency and lint inheritance, compile boundaries, and repository file-size policy; split only on cohesive ownership boundaries.<br>[ ] Give every maintained hand-authored comment-capable file a one- or two-line native responsibility header; run `npm run lint:file-description -- --all` with zero gaps.<br>[ ] Move completed child plans and evidence to canonical completed owners.<br>[ ] Record compatibility guarantees, unsupported paths, retained exceptions, and the v4 backlog.<br>[ ] Audit docs, architecture, security, dependencies, duplication, migration, GUI, packages, and public artifacts. | Planning, harness, release, and all affected owners.<br>Exit: no transition-only v3 naming, pre-v3 compatibility machinery, or file-description gap remains; Rust code has explicit cohesive owners without boundary leakage, no active v3 work remains outside this plan, current state is compact, and the next major line starts from an explicit backlog. |

## Verification

Every child feature starts with:

```bash
./init.sh
npm run harness:plan -- --since <valid-ref>
```

Minimum planning and documentation gate:

```bash
npm run harness:check
npm run lint:docs
npm run lint:language
npm run lint:file-size
git diff --check
```

Frontend or desktop behavior gate:

```bash
npm run typecheck
npm run lint
npm run test:run
npm run test:e2e:smoke
```

Rust, migration, security, source, model, pack, or compatibility work adds the
focused crate tests and:

```bash
npm run lint:architecture
npm run lint:security
npm run lint:sqlx
npm run verify:rust
```

Milestone and release cuts run:

```bash
npm run verify:full
npm run test:e2e:all:budget
node scripts/dev/run-cargo.mjs test --workspace --all-features
```

Release cuts also run current package, public-artifact, supply-chain, migration,
rollback, and installed-app commands selected by the verification matrix.
Evidence records the command, revision, platform or fixture, exit status,
relevant result, and caveat.

## Progress

| Date | Status | Notes |
| --- | --- | --- |
| 2026-07-18 | Drafted | Consolidated v3 strategy, 233 indexed ideas, the foundation plan, maintained roadmap commitments, open debt, testing gaps, research evaluations, and deferred release work. Activated canonical routing and absorbed the foundation plan. Awaiting Gate 0 approval. |
| 2026-07-18 | Scope updated | User promoted all maintained veteran and military-transition work into v3.0. R10 moved to Milestone 9 with cross-milestone source, evidence, application-review, and release requirements. |
| 2026-07-18 | Priorities reconciled | Absorbed U17 and R14 into v3.0, moved the A20 build decision to Gate 1, promoted six outcomes to v3.x trains, and consolidated E09, X03, and X04 into their canonical IDs. |
| 2026-07-18 | Gate 0 approved | User authorized full execution of this plan on `feat/v3-major-line` through draft PR 329. Milestone 0 is the sole active feature. |
| 2026-07-18 | Milestone 0 complete | Corrected release truth, closed DRY-002 and DRY-003, reconciled testing claims, resolved the release-pipeline decision, added deterministic plan audits, and activated Milestone 1 with evidence at `469abb3f`. |
| 2026-07-19 | Milestone 1 complete | Froze fail-closed v3 contracts and synthetic evaluations, proved existing scheduler ownership, made drag input deterministic, passed isolated native picker smoke, retained measured budgets, deferred A20, and activated Milestone 2 with evidence at `5c8d23c7`. |
| 2026-07-19 | Milestone 2 complete | Added the typed local case, event, graph, receipt, source-policy, and compatibility foundation; proved fresh and v2.9 migration, failure, retry, snapshot restore, and newer-data refusal; completed Gate 2; and activated Milestone 3 with implementation at `69acdf3c`. |
| 2026-07-19 | Milestone 3 reviewed export complete | Added a fixed-schema reviewed JSON Lines export with no-clobber publication, structural inspection, managed-credential and private-path exclusion, separate protected-record review, and fail-closed URL sanitation at `3b4f635b`. Milestone 3 remains active. |
| 2026-07-19 | Milestone 3 safe storage cleanup complete | Added offline aggregate health inspection, integrity-first refusal, checked WAL cleanup, bounded native incremental vacuum, sanitized provenance, and new-database activation at `4304fbb5`. Milestone 3 remains active. |
| 2026-07-19 | Milestone 3 restricted scheduled-source consent complete | Added append-only source-policy history and exact durable consent for the four shipped restricted scheduled sources at `80218596`. Policy or request drift revokes, renderer and config booleans cannot authorize transport, and fixed restricted health probes remain unavailable pending distinct reviewed scope. Milestone 3 remains active. |
| 2026-07-19 | Milestone 3 Outside AI governance complete | Added backend-verified public-job provenance, trusted native confirmation, exact single-use approval, immutable receipt-bound lifecycle, no-retry transport, durable cancellation, explicit ambiguity, and bounded local recovery activity at `02465456`. Two adversarial reviewers returned PASS. Milestone 3 remains active. |
| 2026-07-19 | Milestone 3 complete | Added isolated offline startup recovery, bounded queued-work state, independent invalid-state preservation, encrypted restore controls, fail-closed publication and rollback, platform health, app-owned Unix permission repair, explicit package connectivity guidance, and Settings recovery controls at `aaa9f28c`. Focused product and security reviewers returned PASS. Activated Milestone 4. |
| 2026-07-19 | Milestone 4 connected source governed | Classified the legacy JobsWithGPT path, disabled scheduled contact before audit or transport, preserved local configuration and history, corrected source health through migration 17, and bound provider-review and parser fixtures at `8ef539ad`. Focused checks passed and adversarial correction review returned PASS. Milestone 4 remains active. |
| 2026-07-19 | Milestone 4 restricted scheduled sources retired | Bound current first-party policy evidence and disabled manifests for Built In, Dice HTML, SimplyHired, and Glassdoor; removed legacy adapters, UI controls, and health metadata; cleared stale config authority; blocked pasted-URL fetches before transport; and kept visible Browser Import as a separate reviewed path at `8a7d3439`. Migration 18 prevents health-row recreation, focused checks passed, and adversarial review returned SOUND. Milestone 4 remains active. |
| 2026-07-19 | Milestone 4 LinkedIn Workbench governed | Bound current LinkedIn policy evidence and a restricted user-opened manifest; moved review authority from renderer storage to exact append-only backend consent; enforced freshness, revocation, and pre-write credential rejection; separated navigation confirmation from durable consent; blocked LinkedIn Browser Import before DOM access; and required paired browser grants for browser-origin applied logging at `8baa2fea`. Focused checks passed and adversarial review returned APPROVE. Milestone 4 remains active. |
| 2026-07-19 | Milestone 4 employer discovery governed | Added a user-directed EmployerDiscovery manifest and real Schema.org fixture; canonicalized and hard-blocked pasted destinations before native review; required exact installed policy and manifest authorization before the single credential-free fetch and before storage; retained the nonsecret grant only with bounded in-memory pending work; and persisted the graph-owned source identity at `43675a11`. Focused checks passed and adversarial review returned APPROVE. Milestone 4 remains active. |
| 2026-07-19 | Milestone 4 starter region manifests added | Added dated, explicitly incomplete UK, EU, and India research manifests with exact regional scope, policy and provenance separation, real frozen evaluation references, current pay semantics, and no native-source or runtime-pack claim at `c5422e63`. Focused checks passed and adversarial review returned APPROVE. Milestone 4 remains active. |
| 2026-07-19 | Milestone 4 and Gate 3 complete | Replaced legacy Browser Import with origin-bound one-use pairing at `0104af1b`, then added local Smart Paste and a distinct reviewed applied-draft action at `88beaadc`. Exact grants, revocation, replay protection, restricted-domain blocking, credential rejection, simulator drift checks, and explicit save or discard paths passed focused verification. Adversarial review returned APPROVE. Activated Milestone 5. |
| 2026-07-20 | Milestone 5 vector storage added | Added the frozen `sqlite_blob_v1` owner at `d29cd573`. Exact current vectors round-trip as bounded little-endian values; stale, missing-model, malformed, dimension-invalid, and non-finite rows fail closed and are removed without an ABA race. Derived vectors remain local, stay out of reviewed plaintext export, and store no source text. Focused checks passed and adversarial review returned APPROVE. Milestone 5 remains active. |
| 2026-07-20 | Milestone 5 first vector integration added | Integrated one first-ordered resume-skill vector into the embedded semantic command at `915647f8`. Qwen3 rebuilds only the missing, stale, or invalid local vector; remaining vectors stay in memory. Exact resume-revision guards prevent late writes after edits or deletion, poisoned inputs stop before model or cache work, and MiniLM or model-free installs preserve existing local matching while purging the Qwen3 row. The visible Resume Match page is unchanged. Focused tests and adversarial correction review passed. Milestone 5 remains active. |
| 2026-07-20 | Milestone 5 first semantic evidence citation added | Bound the embedded semantic command's first positive claim to the exact revision-scoped local resume skill at `5b4c316d`. Missing, ambiguous, changed, or deleted evidence fails closed without partial output; the citation exposes no resume content, filesystem path, source identifier, or raw revision. Existing semantic fields remain top-level and the visible Resume Match page remains unchanged. Seven focused tests, production clippy, desktop compilation, and adversarial review passed. Milestone 5 remains active. |
| 2026-07-20 | Milestone 5 semantic evidence coverage completed | Replaced the singular citation boundary with one exact same-order citation for every positive embedded semantic claim at `0d38d46b`. Repeated use of one resume skill repeats its citation without losing cardinality, while any unmappable claim rejects the whole result. Seven focused tests, production clippy, desktop compilation, and adversarial re-review passed. The visible Resume Match page remains unchanged and Milestone 5 remains active. |
| 2026-07-20 | Milestone 5 active saved-resume analysis stabilized | Moved the visible active saved-resume analysis behind application ownership at `c59df6dd`. Exact before-and-after context rejects resume revision, content, file, bounded HTML source, ordered-skill, active-selection, and deletion changes without returning stale evidence. The Tauri command retains existing validation, result shape, and safe missing or unreadable guidance. Six application tests, 11 Tauri resume tests, 5 document citation tests, strict application and desktop clippy, and adversarial re-review passed. Milestone 5 remains active. |
| 2026-07-20 | Milestone 5 copied-resume evidence bound | Moved copied structured-resume analysis behind application ownership at `91a8a3ff`. Renderer-supplied evidence identity is discarded, canonical local content determines the opaque ephemeral revision, custom-section ordering is deterministic, and content over 10 MiB fails before analysis. Nothing is persisted or sent. Nine focused application tests, strict application and desktop clippy, desktop compilation, security and architecture checks, and adversarial review passed. Milestone 5 remains active. |
| 2026-07-20 | Milestone 5 veteran transition evidence bound | Bound proposed civilian wording to exact current saved-resume occupation, responsibility, credential, and current-clearance evidence at `e69063f9`. Preparation requires same-case user-confirmed citations and preserves every military source phrase beside its proposed wording. Confirmation atomically rechecks resume revision, full content hash, and evidence links before returning only the reviewed wording. O*NET and DoD COOL remain manual review resources, and no status, eligibility, verification, or equivalence is inferred. Eight application military tests, storage consistency tests, strict clippy, desktop compilation, security and architecture checks, and adversarial re-review passed. Milestone 5 remains active. |
| 2026-07-20 | Milestone 5 explicit matching profiles added | Added user-selected role evidence and regional spelling profiles at `6a75e007` for active and copied resume review. Role profiles only mark preferred evidence sections and break otherwise equal review ordering. Regional profiles recognize a bounded spelling set and can change recognized matches and scores without weakening hard constraints or treating different concepts as equivalent. Profiles are local, never inferred, explicitly incomplete, and locked during an in-flight review. Six document profile tests, application and Tauri owner tests, 18 UI and dev-runtime tests, strict clippy, harness, architecture, security, file-size, and adversarial re-review passed. Milestone 5 remains active. |
| 2026-07-20 | Milestone 5 matching profile evals frozen | Added the strict versioned synthetic profile baseline at `75e4a347`. All nine role profiles now have exact non-baseline review ordering with unchanged requirement states and zero score deltas. Bounded regional spelling cases exercise all four declared profiles with exact deltas, preserve a distinct-concept negative, and keep a missing required licence behind its hard-constraint cap. The documents library passed 318 tests with one private-path test ignored; production clippy, harness, file-size, formatting, diff validation, and adversarial re-review passed. This does not validate Qwen3 or region-pack completeness. Milestone 5 remains active. |
| 2026-07-20 | Gate 4 operating boundaries frozen | Froze model-free setup, the exact wired Qwen3 Candle pair, legacy-only MiniLM fallback, live-first employer evidence, no-authority static region packs, per-source expiry, and the Essentials component boundary at `d7f0c2a4`. Removed speculative matching-provider compatibility. Gate 4 remains open because production-wide calibration, an 8 GiB model-free performance report, installed setup and removal, employer and regional evals, and the platform matrix are not yet proven. All 75 default embedded local-AI tests passed with 10 explicit model tests ignored; all 151 domain tests, strict clippy, docs, security, harness, and adversarial review passed. |
| 2026-07-20 | Milestone 5 Qwen3 requirement calibration expanded | Expanded the frozen requirement hard-negative set from three to six synthetic pairs at `da5ce3df`. The new cases distinguish civilian RN licensure from Army combat-medic experience, people-management ownership from mentoring, and GAAP close ownership from accounts-payable processing without changing the `0.30` retrieval or `3.0` reranker threshold. The original three pairs retain full production, cached-vector, and abstention proof; the three new pairs freeze direct Qwen3 embedding and reranker behavior. Both opt-in pinned-model tests passed on one macOS 27 arm64 64 GiB host, all 77 default local-AI tests passed with 11 model tests ignored, strict clippy, architecture, security, harness, and adversarial re-review passed. Gate 4 remains open because six synthetic requirement pairs are not production-wide, cross-platform, modest-hardware, installed-setup, employer, or regional evidence. |
| 2026-07-20 | Milestone 5 macOS Essentials baseline measured | Added exact native launch-coalition measurement and bounded app-scoped cleanup at `8a321b55`. A model-free arm64 no-account DMG measured 14,224,509 bytes, its installed app measured 29,704,192 bytes, and three final-verifier launches reached a visible window in a median upper bound of 825 ms with a median aggregate observed maximum RSS of 240,480 KiB across six app-coalition processes. Mounted and copied-app smoke, private local data, empty stderr, model-free payload checks, 19 verifier tests, all 842 script tests, security, harness, and final adversarial review passed. Gate 4 remains open because this low-pressure 64 GiB host does not prove actual 8 GiB behavior, the complete installed Essentials journey, numeric release thresholds, or the platform matrix. |
| 2026-07-20 | Milestone 5 Linux 8 GiB Essentials journey measured | Reused the already-owned database during setup at `d442138f`, removing a redundant credential-backed reconnect and migration that blocked installed first-run completion. The exact source tree produced amd64 Debian and AppImage packages and installed the Debian package inside a controlled 8 GiB, zero-swap Ubuntu guest. Build peak was 4,027,662,336 bytes; the complete installed journey peak was 687,190,016 bytes; persistence relaunch peak was 717,479,936 bytes; and every memory-limit and OOM counter remained zero. Setup, an explicit Greenhouse source check, synthetic veteran resume import, deterministic match review, tracker update, pay abstention, a support report with zero tested private-marker matches, and restart persistence passed. Focused config tests, strict desktop clippy, formatting, AppImage model-payload inspection, and adversarial review passed. Gate 4 remains open for numeric thresholds, Windows 11 and macOS 26 installed evidence, stronger-local setup and removal, broader Qwen3 calibration, employer and regional evaluation, and signed release artifacts. |
| 2026-07-20 | Milestone 5 governed model setup and removal added | Added visible Settings setup, cancellation, and removal for the stronger-local Qwen3 pair at `8bd764c0`. Downloads require native confirmation naming host, size, license, and the local-only data boundary, then stream into app-owned staged files with locked sizes, checksum-verified promotion, resume, symlink rejection, a pinned anonymous Hugging Face endpoint, and an app-scoped Xet transfer cache that is reset before and cleared after each transfer. One lifecycle reservation serializes download, cancel, removal, and repair; cancellation works during confirmation, mid-transfer, on panel exit, and for orphaned downloads after a renderer reload. Removal deletes all cached data under the two governed model identities, including stale revisions and superseded lock layouts, and refuses on any symlink inside the governed tree. Progress events are byte-only and throttled. Built-in matching stays available throughout. 88 local-AI tests, focused desktop and 16 UI tests, strict production clippy, harness, docs, security checks, and adversarial review passed after one MAJOR and six smaller findings were fixed and re-review returned APPROVE. Milestone 5 remains active. |
| 2026-07-20 | Milestone 5 governed model lifecycle proven live | Ran the complete governed setup and removal against Hugging Face at `a2ec0549`. The opt-in lifecycle test downloaded the full 2,406,043,460-byte pinned Qwen3 pair into a fresh app-data directory in 145.96 seconds on the macOS 27 arm64 64 GiB host, verified every locked size and checksum, confirmed monotonic byte-only progress ending exactly at the locked total, confirmed transfer and staging cache cleanup, and removed both governed identities completely. A new fail-closed test proves download stops before any network client without the app-set anonymous policy, and opt-in downloading tests now set that same policy and run single-threaded. 89 default local-AI tests passed with 12 model tests ignored. This is direct production-path proof, not installed-app GUI, modest-hardware, or Windows or Linux evidence. Milestone 5 remains active. |
| 2026-07-20 | Gate 4 numeric thresholds frozen | Froze the Essentials numeric release thresholds in the evaluation and release bar from measured macOS arm64 and enforced 8 GiB Linux evidence: model-free package and installed sizes, a 2,000 ms installed visible-window upper bound, 1 GiB journey and relaunch memory peaks with zero limit events, a 512 MiB unconstrained macOS coalition RSS bound, and an exactly-zero model-payload package check. Thresholds bind only measured platforms; Windows 11 and macOS 26 must be measured before claiming them, and the stronger-local payload remains lock-owned at exactly 2,406,043,460 bytes. Gate 4 remains open for Windows 11 and macOS 26 installed evidence, installed-app GUI model setup proof, broader Qwen3 calibration, employer and regional evaluations, and signed release artifacts. |
| 2026-07-20 | Milestone 5 Qwen3 calibration broadened to ten pairs | Broadened the frozen requirement hard-negative set from six to ten synthetic pairs at `c93ac516`. The four new pairs freeze truthful credential and scope boundaries across trades, transportation, education, and retail management: a journeyman electrician license against apprentice helper work, a CDL-A against forklift certification, a secondary mathematics teaching credential against private tutoring, and store profit-and-loss ownership against shift-lead duties. Every reason credits the adjacent experience without devaluing it, and the retail pair records a genuine layered-design finding: its positive retrieves only 0.1285 above the dense threshold while the reranker separates it decisively. All three pinned calibration tests, including the untouched production-path baseline, passed together in 791.30 seconds on the verified macOS 27 arm64 cache at this exact revision. Default-suite contract and ordering tests now guard all ten queries, the calibration tests moved to a shared capture-then-assert helper in their own submodule, and 89 default tests, production clippy, formatting, harness, and adversarial review with two closed guard findings returned APPROVE. Gate 4 remains open for cross-platform and installed evidence, employer and regional evaluations, and signed release artifacts. |
| 2026-07-21 | Milestone 5 stronger-local macOS package and installed lifecycle proven | Added the explicit stronger-local no-account packaging and verification path through `b3b5d2fa`, then attached both model lifecycle confirmations to the installed main window at `fc165239`. The rebuilt arm64 DMG passed checksum, integrity, profile, command, no-model-payload, signature-integrity, mounted-launch, copied-install, private-data, and empty-stderr checks. In an isolated installed-app GUI journey, both governed Qwen3 groups reached 7/7 ready after the exact 2,406,043,460-byte download; confirmed removal returned them to 0/7 and left the governed model root empty. Focused Rust tests, formatting, diff validation, the final package verifier, and read-only review passed. Gate 4 remains open for exact Windows 11 and macOS 26 evidence, signed and notarized release artifacts, employer and regional evaluations, and the remaining Milestone 5 product surfaces. |
| 2026-07-21 | Milestone 5 and Gate 4 decisions complete | Added durable saved-match evidence, reviewed packet claims, the visible match debugger, and reviewed military-transition wording through `a450f883`. Focused owner tests, frontend contract tests, docs, security, architecture, and adversarial review passed. Gate 4 now approves the frozen model, setup, employer-data, regional-delivery, expiry, and Essentials decisions. Exact Windows 11 and macOS 26 behavior, signed and notarized artifacts, employer dossier evaluation, regional readiness, and the full platform matrix remain later release evidence under Milestones 8, 9, and 11; they are not decision-gate claims. Activated Milestone 6. |
| 2026-07-21 | Milestone 6 opportunity-case checkpoint | Added an explicit local case-open boundary and the first dashboard case summary with source risk, decision state, evidence freshness, and a sanitized timeline. Focused application, IPC validation, renderer, and mock-command tests cover the exposed boundary and UI states; direct storage aggregation coverage remains open. Milestone 6 remains active; the mission board, preparation and debrief, drag-and-drop import, and first-run state coverage remain open. |
| 2026-07-22 | Milestone 6 aggregation and daily-mission checkpoint | Added direct storage aggregation coverage and corrected status-only offer projection plus same-time timeline order. Replaced aggregate review categories with a deterministic three-to-seven-action daily mission that targets specific reminders and opportunities, remains useful with sparse or empty local state, opens quiet roles without bulk status mutation, and routes offer and source actions to their existing review surfaces. Focused Rust and renderer tests, affected-library clippy, type checking, lint, harness, and file-size checks passed. Milestone 6 remains active; the evidence wall, blocker explanations, preparation, debrief, drag-and-drop import, and first-run state coverage remain open. |
| 2026-07-22 | Milestone 6 evidence-wall checkpoint | Added a renderer-safe evidence wall for the active saved resume's exact job match, a deterministic Apply, Maybe, Skip, or Research more summary, and plain "Why not this job?" reasons. The case never substitutes an inactive resume, exposes only evidence categories and confirmation state, omits resume text and opaque IDs, keeps military-section evidence and required hard constraints behind review, treats accepted offers as closed outcomes, and remains read-only and offline. Focused application, saved-match serialization, renderer, and dev-runtime tests, Rust formatting and clippy, TypeScript, lint, harness, architecture, file-size, and docs checks passed. Milestone 6 remains active; preparation, debrief, drag-and-drop import, protected-answer review, first-run, and remaining responsive state coverage remain open. |
| 2026-07-22 | Milestone 6 preparation checkpoint | Added an in-case local preparation workup over the existing safe snapshot. It separates source freshness from refresh, keeps missing or changed evidence and reviewed claims in review, requires manual material selection, exact employer-question wording, and current records for factual claims, leaves voluntary protected veteran-status answers with the user, and never adds a command, write, network, AI, send, or submit action. Focused renderer state tests, TypeScript, lint, harness, architecture, file-size, and docs checks passed. Milestone 6 remains active; debrief, drag-and-drop import, the broader protected-answer review flow, first-run, and remaining workflow coverage remain open. |
| 2026-07-22 | Milestone 6 post-interview debrief checkpoint | Added an explicit local debrief to the existing Interview Schedule owner for signal strength, questions asked, concerns, promised next steps, and an optional follow-up deadline. Saving completes the selected interview atomically without changing application status, scoring performance, calculating hiring probability, calling AI, notifying anyone, or sending data. Overdue incomplete interviews remain open for debrief, completed history retains saved debriefs without an arbitrary age cutoff, the dev runtime follows production query rules, and reminder reads wait until completed history is opened. Focused renderer, dev-runtime, and Rust storage tests, affected-library clippy, formatting, TypeScript, lint, harness, architecture, file-size, docs, and script-contract checks passed. Milestone 6 remains active; drag-and-drop import, the broader protected-answer review flow, first-run, and remaining workflow state coverage remain open. |
| 2026-07-22 | Milestone 6 native file-drop checkpoint | Added one reviewed native main-window drop for resumes, UTF-8 job-posting text, and encrypted backups. Rust copies one regular source into private app-owned staging without following a final symlink or Windows reparse point, exposes only an opaque token and sanitized name, and routes explicit choices through the existing resume, Smart Paste, or staged-recovery owners. Replacement and completion races retain the newest drop, no drop writes a job or restores data automatically, and the renderer has no filesystem capability. Source-pack drops remain with Milestone 7's quarantine and installer. Focused Rust, renderer, startup, TypeScript, lint, harness, architecture, file-size, dependency, documentation, and script-contract checks passed; adversarial re-review returned SOUND after file-swap and stale-result races were fixed with fail-first tests. Windows reparse behavior is source-reviewed but not live-run on this macOS host. Milestone 6 remains active; protected-answer review, first-run, and remaining workflow-state coverage remain open. |
| 2026-07-22 | Milestone 6 protected-answer checkpoint | Added one shared classifier for voluntary or sensitive personal questions and applied it at saved-answer lookup, suggestions, learned and historical answers, usage recording, application preview, and live form preparation. Explicitly saved selections remain in the local answer bank, but protected records are not suggested, learned, previewed, or filled; the renderer receives only a bounded manual-review topic. Opportunity-case preparation covers protected veteran status, disability, race or ethnicity, gender, and other voluntary sensitive questions without adding storage or automation. Focused domain, assistance, storage, Tauri, renderer, dev-runtime, TypeScript, lint, harness, architecture, file-size, documentation, and script-contract checks passed. Adversarial re-review returned SOUND after EEO wording and dev-runtime parity blockers were fixed. Care Coordinator remains a civilian role and does not trigger protected handling. Milestone 6 remains active; first-run and remaining workflow-state coverage remain open. |
| 2026-07-22 | Milestone 6 complete | Added a truthful reviewed search setup with a disclosed session-only skip and retained-choice retry, exposed existing repost, posting-risk, contact-presence, and completed-interview state in the shared case, and proved empty, partial, duplicate, offline, failed-source, and restored-data behavior at desktop and narrow widths. Focused UI, Rust persistence, browser, type, lint, architecture, file-size, harness, and documentation checks passed; adversarial re-review returned SOUND. Source-pack installation remains Milestone 7, first-run doctor work remains Milestone 9, and installed release-platform proof remains Milestone 11. Activated Milestone 7. |
| 2026-07-22 | Milestone 7 signed release trust checkpoint | Added a bounded non-executing Ed25519 release verifier at `6b86b970` with publisher ceilings, compiled-runtime compatibility, publisher-qualified identity, exact payload integrity, strict JSON, and gateway-bound Outside AI. Focused domain and security checks passed. Installation, persistent trust, and execution remain open. |
| 2026-07-22 | Milestone 7 typed payload self-test checkpoint | Added a closed signed payload parser and non-executing self-tests for disabled source packs with exact fixtures, the compiled Evidence Reviewer and Application Packet Builder plans, static text-only Agent Skills, and complete synthetic local evaluation packs. Unsupported types, scripts, dynamic adapters, external destinations, host installation tasks, injected plan text, capability drift, fixture drift, partial evals, ambiguous static metadata, and filesystem-ambiguous resource paths fail closed. Fifteen new focused tests, production clippy, formatting, patch validation, Agent Skills compatibility, and hard file-size checks passed; adversarial re-review returned SOUND. Transactional quarantine, replay and downgrade state, rollback, revocation, native source-pack drop, UI, and execution remain open. |
| 2026-07-22 | Milestone 7 transactional staging checkpoint | Added a local SQLite release ledger, publisher trust identity, and logical-pack stream state. Externally immutable verified releases bind the authenticated release digest and verification-time public-key fingerprint. They stage quarantined under a publisher-qualified composite identity; known replay is inert, same-sequence equivocation is detected, and unseen releases below the durable high-water mark are rejected. Staging never activates or executes content. Focused migration, storage lifecycle, typed-payload, and signed-release tests passed; adversarial re-review returned SOUND. Self-test promotion, activation, rollback, revocation, uninstall, filesystem promotion, UI, and execution remain open. |
| 2026-07-22 | Milestone 7 pack lifecycle and artifact checkpoint | Added proof-bound self-test promotion, explicit generation-checked activation, disable and verified re-enable, retained-release rollback, publisher-wide irreversible revocation, uninstall tombstones, and retryable owned-artifact cleanup. Signed artifacts persist under a caller-supplied validated root through descriptor-relative Unix operations and held no-delete Windows directory handles; activation re-reads, re-verifies, and re-tests exact bytes. The tested but unwired reconciliation path leaves valid active packs unchanged, atomically falls back to a verified rollback, or quarantines invalid and missing active and rollback artifacts. Migration 24 restores a removed stream to quarantine only for a newer release. Focused domain, migration, storage, application, symlink-redirection, and production clippy checks passed on macOS. Adversarial review's initial filesystem `BLOCK` was fixed and the fresh verdict is `CONCERNS`, limited to production root binding and Windows live proof. Publisher-key provisioning, startup wiring, native source-pack drop, UI, typed execution, and final closure remain open. |
| 2026-07-22 | Milestone 7 reviewed packet execution checkpoint | Added the compiled Application Packet Builder execution at `28387b2c`. One exact reviewed approval produces only a deterministic local draft over the selected case, resume name, safe match debugger, reviewed evidence claims, attachment checklist, unresolved questions, and explicit manual-answer and final-submission boundaries. It has no browser, network, filesystem, credential, external-send, answer-fill, or submission path. A storage-owned digest covers every packet source before and after assembly, is the immutable approved task input, and is rechecked before and after atomic receipt insertion under an immediate SQLite transaction. Stale rendered or non-rendered state, a replacement current guard, output overflow, replay, and an in-transaction source mutation all fail without an audit commit. Fourteen focused storage tests, twenty-five application pack-runtime tests, production clippy, formatting, headers, harness, architecture, file-size hard limits, and adversarial re-review passed. Durable pack-management projection, production root and trust, startup wiring, native source-pack drop, UI, live platform proof, and final closure remain open. |
| 2026-07-22 | Milestone 7 durable pack-management checkpoint | Added immutable signed review projection and release-scoped cleanup truth at `1990a87b`. One typed renderer-safe view now includes every stream and release history, distinguishes needs-review from ready or disabled updates, preserves compatibility, publisher, license, privacy, capability, destination, fixture, quarantine, and cleanup facts, and exposes no artifact path, payload, envelope, signature, digest, task data, or receipt. Uninstall marks each exact removed release pending before filesystem work; deletion attempts every release, persists individual success, is idempotent under concurrent retries, and cannot delete a newer restaged artifact. Missing review facts block activation and reviewed execution until an exact freshly verified replay restores them. Three migration, twenty lifecycle, fourteen reviewed-task, twenty-eight application runtime, thirty-one signed-payload and static-skill tests, isolated SQLx metadata validation, production clippy, formatting, headers, harness, architecture, and hard file-size checks passed. Adversarial re-review returned SOUND and lean review returned APPROVE. Production root and trust, startup wiring, native source-pack drop, UI, live platform proof, and final closure remain open. |
| 2026-07-22 | Milestone 7 read-only pack review checkpoint | Added a read-only Settings pack view at `8c663ac1`. One renderer-safe Tauri command exposes the existing immutable management projection without artifact paths, envelopes, signatures, digests, payloads, task data, or receipts. The renderer rejects unknown signed facts, impossible capability combinations, contradictory lifecycle state, duplicate release sequences, and current/history mismatch before displaying anything. Users can inspect every current and retained release, including a pending update's exact publisher key, type, execution class, compatibility, privacy labels, data, tasks, actions, approval gates, gateway route, fixture summary, quarantine reason, and cleanup state. Raw backend failures remain local behind fixed retryable copy, browser-development command parity is explicit, and no install, activation, lifecycle, trust, execution, filesystem, network, credential, send, or submission authority was added. Twenty-seven focused renderer tests, the Rust IPC test, typecheck, frontend lint, production clippy, formatting, headers, harness, architecture, and hard file-size checks passed. The initial adversarial blocks for incomplete facts, permissive decoding, misleading management wording, and hidden update permissions were fixed; final review returned SOUND/PASS and lean acceptance review returned APPROVE. Production publisher trust, the v3.0.0 runtime cut, artifact-root binding, startup wiring, native source-pack drop, lifecycle controls, reviewed local skill and agent execution, live platform proof, and final closure remain open. |
| 2026-07-22 | Milestone 7 opaque evaluation scoring checkpoint | Added a private local observation scorer for self-tested synthetic evaluation packs at `f9f0a849`. The scorer retains the validated complete fixture set without exposing its content, requires exact known-case coverage, rejects missing and extra observations, and returns only one pass or fail result per case. Revision metadata is bounded to a safe identifier alphabet. Domain tests, doctests, production clippy, formatting, headers, harness, architecture, hard file-size checks, and patch validation passed. The initial adversarial review blocked public fixture exposure and an overstated execution API; both were removed, and fresh re-review returned SOUND. Actual product-target evaluation execution, production publisher trust, the v3.0.0 runtime cut, artifact-root binding, startup wiring, native source-pack drop, lifecycle controls, reviewed local skill and agent execution, live platform proof, and final closure remain open. |
| 2026-07-22 | Milestone 7 unreleased 3.0.0 runtime checkpoint | Cut development source metadata and the compiled signed-pack runtime to the unreleased 3.0.0 line at `8e928aaa`. Cargo workspace metadata, exact internal dependency constraints, npm and Tauri metadata, and both lockfiles move together. The production parser accepts an exact trusted 3.0.0 release, and application and storage pack tests no longer inject a test-only runtime version. The latest public release remains v2.9.5; no tag, workflow dispatch, updater, artifact, publication, or compatibility-closure change was made. The workspace locked compile, 184 domain tests and five doctests, 28 application pack tests, 20 storage lifecycle tests, 25 release-contract tests, affected-crate clippy, formatting, release metadata and static-readiness checks, SQLx offline validation, dependency, architecture, harness, documentation, header, file-size, secret, and patch checks passed. Initial adversarial review blocked stale source-truth documentation; after the correction, fresh re-review returned SOUND. Production publisher trust, artifact-root binding, startup wiring, native source-pack drop, lifecycle controls, product-target evaluation, reviewed local skill execution, live platform proof, pre-v3 compatibility removal, transition-name removal, and final closure remain open. |
| 2026-07-22 | Milestone 7 static Agent Skill reader checkpoint | Added a bounded application reader for Ready static skills. It requires the exact active generation, reloads and re-verifies the owned signed artifact against current caller-supplied trust, repeats the closed payload self-test, and returns only plain signed `SKILL.md` text, validated text resources, and advisory handoff metadata. Missing or invalid artifacts retain the existing quarantine behavior. The reader creates no task, receipt, execution, network, filesystem, AI, or automatic handoff authority. The fail-first application test failed on the absent reader before implementation; seven domain static-skill tests, three focused reader tests, all 31 application pack-runtime tests, strict affected-crate clippy, formatting, headers, architecture, hard file-size limits, and patch validation passed. Independent adversarial review returned SOUND. Production trust, artifact-root and startup binding, user-visible skill review, native source-pack drop, lifecycle controls, agent UI, product-target evaluation, live platform proof, and final closure remain open. |
| 2026-08-11 | Milestone 7 publisher fingerprint checkpoint | Corrected the read-only pack review contract at `0c698ffb` so the immutable SHA-256 fingerprint of the verified Ed25519 publisher key crosses storage and application ownership, is validated as exact lowercase 64-hex at the renderer boundary, and is visible beside the publisher key ID for current and retained releases. The raw public key, signatures, envelopes, payloads, artifact paths, task data, and receipts remain absent, and no lifecycle or execution authority was added. Fail-first storage, application, and renderer regressions drove the change; five storage management tests, three application management tests, fifteen renderer tests, affected-crate clippy, formatting, TypeScript, frontend lint, harness, architecture, security, file-size, and patch checks passed. Production key identities and ceilings still require approval before trust, artifact-root, or startup reconciliation wiring. |
| 2026-07-22 | V3 closure and compatibility boundary locked | User decision makes v3.0.0 the first supported compatibility baseline. Pre-v3 upgrade support is out of scope, and Milestone 13 must delete its readers, shims, fixtures, and claims. Closure must also remove transition-only `v3` prefixes from ordinary code and live schema names, retaining version labels only where they are immutable compatibility identifiers. Earlier v2.9 migration proof remains historical evidence, not a release requirement. |
| 2026-07-22 | Rust organization requirement locked | User made proper Rust workspace and module organization a release requirement. Cargo workspace ownership, shared dependency and lint inheritance, cohesive module boundaries, focused crate checks, and the repository file-size policy are blocking closure checks; extra crates and abstractions remain evidence-driven rather than size-driven. |
| 2026-07-22 | File responsibility descriptions locked | Every maintained hand-authored comment-capable file must start with a one- or two-line native description of its exact responsibility, after any required format directive. Changed files are enforced immediately; strict commentless formats and canonical non-hand-authored exclusions remain untouched, and Milestone 13 requires the all-files audit to report zero gaps. |

## Discoveries

- The v3 package is comprehensive strategy but did not previously have a
  comprehensive execution owner.
- The prior foundation plan is a valid storage slice but is not registered as
  active and covers only part of Milestone 2.
- The live duplication gate is zero while DRY-003 remains marked open.
- Veteran support already spans the Military Transition resume template,
  military-service evidence, truthful clearance checks, protected application
  answers, and public-service research; it was not one connected v3 outcome.
- Several maintained docs still describe the already-published v2.9.5 release
  as a candidate.
- Several testing “future improvements” already exist in source, so Milestone 0
  must reconcile docs before creating redundant infrastructure.
- The ranked index contains 105 `Now` and 82 `Next` ideas. They require
  consolidation into user outcomes, not 187 independent features.
- E09, X03, and X04 duplicated A19, U19, and M26. Their intent now remains
  under the canonical IDs rather than inflating the index and disposition list.
- Direct `scopeguard` ownership duplicated existing `tempfile` cleanup, while
  `hex`, `urlencoding`, and `basic-toml` avoid riskier custom implementations.
- Under [GitHub's token event rules](https://docs.github.com/en/actions/concepts/security/github_token),
  a release published with `github.token` cannot trigger the separate
  `release.published` verifier. That verifier also authenticates downloads and
  lacks its previously recorded macOS platform condition.
- The legacy JobsWithGPT terms URL now redirects to a renamed provider whose
  first-party terms do not identify an exact feed endpoint, client authorization,
  or pacing contract.
- Current Built In, Dice, SimplyHired, and Glassdoor provider terms do not
  authorize the retired scheduled or pasted-URL fetch paths. A local
  acknowledgement cannot override those provider boundaries.
- Pasted-URL import and visible Browser Import are different operations.
  Pasted-URL import performs an application-owned network fetch, while Browser
  Import receives user-selected information already visible in the user's
  browser. The four retired boards allow only the latter local review path;
  YC remains blocked for both operations.
- Current LinkedIn terms prohibit unauthorized scraping, copying, browser
  plugins, and automated activity. LinkedIn Browser Import must therefore stop
  before DOM access or localhost transport, even when the user starts it.
- The prior LinkedIn acknowledgement lived only in renderer storage, while
  native IPC could write local job and application records directly. Restricted
  Workbench authority must live in the backend and bind exact current policy,
  behavior, destination, request, and data categories.
- The generated Browser Import button currently produces one posting, not a
  visible-card batch. Maintained UI and feature docs must describe that current
  behavior while broader companion capture remains planned work.
- Deleted health metadata still needs defense in depth. A hostile restored row
  must remain inert in storage, native IPC, development mocks, and renderer
  controls.

## Decisions

- Treat v3 as a major line: v3.0 freezes durable contracts and ships the first
  coherent vertical product; v3.x delivers accepted dependent enhancements.
- Split N07 at its real trust boundary. Milestone 6 owns Rust-validated drops
  into the existing resume, reviewed job, and staged encrypted-backup flows.
  Milestone 7 completes source-pack dropping only after its signature,
  quarantine, self-test, rollback, and installer contracts exist. This keeps
  all four accepted N07 categories in v3.0 without inventing an unsafe
  pre-installer pack import.
- Record the user's full-execution instruction as Gate 0 approval and activate
  Milestone 0 before any product implementation.
- Accept `Now` and `Next` items, defer `Later` and `Moonshot` items, and retain
  all cut lines unless the user approves a future product-boundary change.
- Promote R10 into v3.0 and treat veteran and military-transition support as
  one cross-milestone outcome. Preserve current capabilities, require
  user-confirmed evidence and protected-answer control, and do not infer
  civilian equivalence, veteran status, clearance, or eligibility.
- Treat the core U17 campaign outcome and R14 offline resilience as v3.0
  baseline behavior. Keep advanced campaign capabilities in train 12A.
- Keep A20 deferred by default. Gate 1 may promote a new local runtime kernel
  only if focused evidence proves existing application and Tokio owners cannot
  meet typed-job requirements.
- Consolidate E09 into A19, X03 into U19, and X04 into M26. Implement
  contract-derived outputs only for stable schemas and publish an open matching
  benchmark only after internal evaluation contracts stabilize.
- Absorb the foundation plan into Milestone 2 and retire the separate file.
- Keep the existing crate graph unless a milestone proves an ownership gap that
  cannot be solved by an existing owner.
- Put platform-specific live distribution proof at the end of the release
  stream.
- At Milestone 11, make authenticated draft-asset verification block
  publication, restore verifier platform scoping, and keep unauthenticated
  public-download verification as an explicit post-publication closure check.
  Do not add a broader credential only to trigger another workflow.
- Freeze the Gate 1 compatibility line at v3, the vector backend at
  `sqlite_blob_v1`, and executable packs at static content or reviewed typed
  workflows. Backups are encrypted. Reviewed plaintext exports exclude
  JobSentinel-managed credentials and app-managed private paths, preserve
  user-authored text verbatim, and require explicit user review.
- Keep immediate notification delivery at one claimed attempt for now. A
  failed or ambiguous provider attempt remains visible and is not retried
  automatically; per-channel durable retry needs later storage design.
- Approve the Gate 2 minimum local data model. Keep legacy application events
  historical, write v3 history to a typed bounded ledger, use concrete graph
  links, require local and sensitive labels for protected records, and keep
  source policy monotonic.
- Treat the migration snapshot as same-device encrypted recovery for one exact
  transition. Portable backup, reviewed export, user-facing restore, rollback,
  cleanup, and repair remain Milestone 3 responsibilities.
- Milestone 3 owns append-only policy history and exact consent for already
  shipped restricted scheduled sources. Milestone 4 owns the complete source
  graph, new source operations, browser permissions, and any distinct reviewed
  health-check scope.
- Keep JobsWithGPT configuration, exact prior approval, and minimized contact
  history locally inspectable, but disable new contact until a dated provider
  review verifies the endpoint, authorization, pacing, and stop conditions.
- Supersede Milestone 3 scheduled consent for Built In, Dice, SimplyHired, and
  Glassdoor with Milestone 4 disabled policies after current provider review.
  Preserve append-only policy and consent history, but revoke remembered
  consent and clear runtime and saved enablement.
- Treat direct pasted-URL fetches as automation. Block the four retired board
  domains and their subdomains before transport. Keep visible Browser Import,
  user-opened search links, employer follow-through, and manual entry separate.
  Keep YC's stricter no-fetch and no-capture boundary.
- Treat Dice's official MCP as a separate review-required candidate. Do not
  scaffold it until exact schema, privacy, pacing, endpoint, and fixture
  contracts are approved.
- Treat LinkedIn as a restricted user-opened Workbench under current dated
  first-party evidence. Keep user-opened navigation behind a native confirmation
  that creates no durable consent. Keep Workbench writes behind exact append-only
  backend consent and fresh policy evidence.
- Block LinkedIn Browser Import before iframe creation, DOM access, or transport,
  and retain server rejection as defense in depth. A renderer acknowledgement
  cannot authorize capture.
- Keep in-app local ledger actions inside the exact RestrictedWorkbench
  capability. Require PairedBrowserGrant for browser-origin AppliedLogging and
  VisiblePageCapture. Generic Smart Paste and browser-origin applied logging
  are separate reviewed operations.
- Approve Gate 3 with coexistence, not replacement: scheduled public checks,
  retired restricted automation, employer discovery, restricted Workbench,
  visible Browser Import, Smart Paste, and applied logging retain distinct
  source classes, permissions, pacing, visibility, and exit controls.
- Accept the origin-bound bookmarklet companion as the S14 v3.0 minimum. A
  dedicated extension, side panel, and broader cross-browser packaging remain
  train 12B work; do not scaffold them in v3.0.
- Accept source identity, action, provenance, lineage, and governed employer
  discovery as the S27 Milestone 4 graph minimum. Employer dossiers and registry
  intelligence remain Milestone 8 work.
- Keep starter region manifests as dated research metadata. RegionalPackCheck
  remains unsupported and fail-closed until Milestone 9 and Gate 4.
- Treat the shared Schema.org source fixture as canonical normalized-output and
  hash-drift evidence. Smart Paste parsing and applied logging behavior use
  operation-specific runtime tests rather than an invented shared parser.
- Freeze the Gate 4 Essentials numeric thresholds in
  [Evaluation And Release Bar](evaluation-and-release-bar.md) from the measured
  macOS arm64 and enforced 8 GiB Linux evidence. The thresholds bind release
  claims only for measured platforms; Windows 11 and macOS 26 must be measured
  before claiming them, and the stronger-local payload stays lock-owned rather
  than threshold-owned.

## Outcomes

- Milestone 0 established one verified planning authority, accurate current
  release records, closed repository debt, and deterministic scope checks.
- Milestone 1 established fail-closed v3 contracts, privacy-safe evaluation
  fixtures, proven scheduler lifecycle ownership, deterministic drag input,
  installed-app native picker evidence, and a measured smoke budget.
- Milestone 2 established a typed local data foundation with bounded event
  metadata, protected privacy receipts, concrete provenance links, monotonic
  source policy, compatibility reads, atomic case reuse, and recovery-first
  migration behavior.
- Milestone 3 now has a reviewed, application-independent plaintext export
  whose fixed schema, private publication, structural completion, managed-secret
  exclusions, and protected-record controls passed focused and adversarial
  verification. It also has offline aggregate storage inspection and an
  integrity-first cleanup path that preserves records, reclaims only
  already-free SQLite pages, reports unsupported legacy modes honestly, and
  passed focused and adversarial verification. A local-only Privacy Doctor now
  reports bounded recovery and privacy states without reading secrets, and the
  safe support bundle preserves those typed states through final sanitization
  while removing arbitrary renderer details and complete private paths. The
  milestone also has append-only source-policy history and exact durable
  consent for shipped restricted scheduled sources. Material drift revokes,
  restored booleans cannot authorize transport, and fixed restricted health
  probes remain unavailable until a different exact scope is reviewed.
  Outside AI now accepts only unchanged stored public job fields after trusted
  native confirmation, consumes one exact approval, records an immutable bound
  receipt before one provider attempt, preserves cancellation and uncertain
  delivery as ambiguous, and exposes only bounded lifecycle recovery metadata
  in Settings. Primary startup failure now enters an isolated in-memory recovery
  service that skips primary storage, credentials, scheduled work, normal IPC,
  and tray search. Recovery exposes bounded queued-work and health state, stages
  or cancels encrypted restore, preserves invalid configuration and corrupt
  primary data independently, refuses linked or ambiguous filesystem entries,
  and labels package connectivity before user action. Platform repair is local
  and handle-verified on Unix and manual on Windows. Milestone 3 is passing.
- Milestone 4 now classifies JobsWithGPT as a disabled restricted scheduled
  source. Exact local approval cannot bypass the policy, source health cannot
  re-enable it, and the worker stops before request audit or transport.
- Milestone 4 now binds disabled manifests and current first-party evidence for
  Built In, Dice HTML, SimplyHired, and Glassdoor. Legacy scheduler adapters,
  live health helpers, onboarding and Settings controls, and source
  recommendations are removed. Migration 18 deletes their health rows and
  prevents recreation. Stale configuration and local acknowledgement cannot
  restore transport. Pasted-URL fetches stop before network access, while
  user-opened links, visible Browser Import, and manual entry remain explicit
  local alternatives.
- Milestone 4 now governs the LinkedIn Workbench with a current restricted
  user-opened manifest, exact backend-owned append-only consent, stale-policy
  pause, revocation, and pre-write structured credential rejection. LinkedIn
  Browser Import stops before page access, navigation confirmation grants no
  durable consent, and browser-origin applied logging cannot downgrade from a
  paired grant.
- Milestone 4 now governs user-directed employer discovery with a narrow
  EmployerDiscovery manifest. Pasted targets are canonicalized and hard-blocked
  before trusted native review, exact installed policy and manifest state gates
  both the single credential-free fetch and durable confirmation, and the
  nonsecret grant expires with bounded in-memory pending work.
- Milestone 4 now includes dated UK, EU, and India research manifests. They
  remain explicitly incomplete, English-only metadata with no RegionalPackCheck
  runtime action or claim that an official source is integrated.
- Milestone 4 now passes Gate 3. Browser Import uses native-confirmed,
  origin-bound, atomic one-use pairing and visible fields only. Smart Paste is
  local, editable, screenshot-free, and credential-filtered. Applied logging
  uses a distinct paired grant, captures only visible title, company, and page
  address, marks missing details, and requires local review before Applied
  status. Every supported source action has a typed class, permission, pacing,
  visible control, revocation or cancellation path, safe explanation, and
  focused evidence.
- Milestone 5 now keeps match claims behind current opaque evidence links,
  bounded requirement states, hard-constraint blockers, deterministic fallbacks,
  and explicit user confirmation. Recent saved matches expose plain why-not
  diagnostics, local feedback, reviewed durable packet claims, and military
  wording review without exporting military source evidence or asserting
  clearance currentness or civilian equivalence.
- Gate 4 decisions are approved. Deterministic matching remains the default;
  stronger-local uses only the exact governed Qwen3 pair; MiniLM is legacy-only;
  employer evidence stays live-first and source-specific; region data stays in
  non-authoritative static packs; source manifests own expiry; and Essentials
  remains model-free. Later platform, employer, region, and distribution proof
  limits release claims but does not reopen these decisions.

## Handoff

- Current state: Milestones 0 through 6 and Gates 0 through 4 are passing.
  Milestone 7 agent and pack runtime is the sole active feature. Signed release
  trust is committed at `6b86b970`; strict typed payload self-tests and the
  current lifecycle checkpoint adds publisher-qualified quarantine, inert exact
  replay, same-sequence equivocation detection, proof-bound activation and
  rollback, revocation, uninstall tombstones, private artifact persistence,
  cleanup retry, and a tested but unwired startup artifact reconciliation path.
  Reviewed evidence and draft-packet execution is committed through `28387b2c`; exact source state, active release,
  approval, expiry, bounds, receipt atomicity, and replay are enforced without browser, network, filesystem,
  credential, send, or submission authority. Durable pack-management projection and release-scoped cleanup truth are
  committed at `1990a87b`; every lifecycle and release-history state remains visible from immutable verified facts,
  and concurrent cleanup or later restaging cannot lose truth or delete a new artifact. The read-only Settings pack
  review is committed at `8c663ac1`; current and pending release permissions, failures, and cleanup truth remain
  inspectable without adding lifecycle or execution authority. Self-tested synthetic evaluation packs now retain their
  complete fixture set behind the private scorer committed at `f9f0a849`; exact observation coverage returns only
  per-case pass or fail results. Development source metadata and the compiled signed-pack runtime now identify the
  unreleased 3.0.0 line, and pack-runtime tests use the production parser. Ready static skills can now be reopened
  through the same verified artifact boundary for bounded plain-text review and an advisory-only handoff. Read-only pack
  review now exposes the immutable publisher public-key fingerprint beside the key ID and rejects malformed fingerprints.
  Product-target evaluation execution, production publisher trust, artifact-root binding, startup wiring, native
  source-pack drop,
  lifecycle controls, user-visible skill review, reviewed agent execution, and platform live proof remain open.
- Evidence: `docs/harness/evidence/v3-milestone-3-reviewed-export-2026-07-19.json`
  binds the reviewed-export slice at `3b4f635b`, and
  `docs/harness/evidence/v3-milestone-3-storage-cleanup-2026-07-19.json` binds
  offline storage cleanup at `4304fbb5`.
  `docs/harness/evidence/v3-milestone-3-privacy-doctor-support-2026-07-19.json`
  binds Privacy Doctor and safe support behavior at `154408e5`, and
  `docs/harness/evidence/v3-milestone-3-policy-consent-2026-07-19.json` binds
  restricted scheduled-source policy and consent at `80218596`, and
  `docs/harness/evidence/v3-milestone-3-outside-ai-governance-2026-07-19.json`
  binds the governed Outside AI lifecycle at `02465456`.
  `docs/harness/evidence/v3-milestone-3-offline-recovery-platform-repair-2026-07-19.json`
  binds final offline recovery and platform repair at `aaa9f28c`; prior
  milestone evidence remains authoritative for its completed scope. Milestone 4
  connected-source governance is bound at `8ef539ad`, and restricted scheduled
  source retirement is bound by
  `docs/harness/evidence/v3-milestone-4-restricted-source-retirement-2026-07-19.json`
  at `8a7d3439`. LinkedIn Workbench governance is bound by
  `docs/harness/evidence/v3-milestone-4-linkedin-workbench-governance-2026-07-19.json`
  at `8baa2fea`. Reviewed employer discovery is bound by
  `docs/harness/evidence/v3-milestone-4-employer-discovery-governance-2026-07-19.json`
  at `43675a11`. Starter regional manifests are bound by
  `docs/harness/evidence/v3-milestone-4-starter-region-manifests-2026-07-19.json`
  at `c5422e63`. Gate 3 completion is bound by
  `docs/harness/evidence/v3-milestone-4-gate-3-2026-07-19.json` at
  `88beaadc`. The strict synthetic matching-profile baseline is versioned at
  `crates/jobsentinel-documents/src/eval_fixtures/matching_profiles_v1.json`
  and bound by `75e4a347`. The initial Gate 4 operating decisions are bound by
  `docs/harness/evidence/v3-milestone-5-gate-4-operating-decisions-2026-07-20.json`
  at `d7f0c2a4`. The six-pair Qwen3 requirement calibration and its retained
  limitations are bound by
  `docs/harness/evidence/v3-milestone-5-qwen-requirement-calibration-2026-07-20.json`
  at `da5ce3df`. The first model-free macOS arm64 package, launch, and observed
  memory baseline is bound by
  `docs/harness/evidence/v3-milestone-5-essentials-macos-footprint-2026-07-20.json`
  at `8a321b55`; its application artifact was built from `62405463`, with only
  verifier and verifier-test files changed between those revisions. The
  controlled 8 GiB Linux package, complete installed journey, setup correction,
  and persistence relaunch are bound by
  `docs/harness/evidence/v3-milestone-5-essentials-linux-8g-journey-2026-07-20.json`
  at `d442138f`. The live governed model setup and removal proof is bound by
  `docs/harness/evidence/v3-milestone-5-governed-model-lifecycle-2026-07-20.json`
  at `a2ec0549`. The ten-pair broadened Qwen3 calibration is bound by
  `docs/harness/evidence/v3-milestone-5-qwen-broadened-calibration-2026-07-20.json`
  at `c93ac516`. Milestone 5 and the superseding Gate 4 decision closure are
  bound by
  `docs/harness/evidence/v3-milestone-5-local-evidence-completion-2026-07-21.json`
  at `a450f883`. Milestone 6 completion is bound by
  `docs/harness/evidence/v3-milestone-6-opportunity-case-workflow-2026-07-22.json`
  at `48a47eaf`.
- Next step: approve the exact production publisher key IDs, public-key fingerprints, and capability ceilings. Then bind
  the production artifact root and approved trust, wire startup reconciliation, and add reviewed native source-pack drop
  without making staged content executable.
- Publication checkpoint: after each pushed checkpoint, keep draft PR 329's
  description aligned with the exact remote-head commit, implemented scope,
  focused verification, known gaps, and next planned work before pausing or
  advancing to the next slice.
- Open risks: scope remains large, contract freeze is irreversible within the
  v3 compatibility line, recovery and permission behavior still needs Windows
  11, macOS 26, and Linux release-matrix proof, installed recovery UI still
  needs keyboard and screen-reader evidence, and some final distribution
  evidence depends on external credentials.
