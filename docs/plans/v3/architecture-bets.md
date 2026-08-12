# Architecture Bets

V3 needs durable primitives so features do not become isolated screens. The
goal is not more abstraction for its own sake. The goal is one coherent local
system that can discover, reason, assist, and explain.

## Bet 0: Rust Core Runtime

V3 may refactor the architecture, data model, UI shell, browser companion, and
pack system, but Rust remains the base of JobSentinel.

Required properties:

- Durable business logic stays in Rust.
- Source adapters, URL validation, rate limits, policy checks, and parser
  contracts stay under Rust-owned boundaries.
- SQLite migrations, backups, compatibility checks, and rollback gates stay in
  Rust-owned code.
- Model governance, cache verification, Qwen3 runtime contracts, and matching
  diagnostics stay behind Rust-owned traits and commands.
- Browser companion, MCP, extension, and external AI entrypoints call into Rust
  validation before touching local data.

Why this matters:

- Rust gives JobSentinel a stable local trust boundary for privacy-sensitive
  workflows.
- Refactors should simplify architecture without moving security-critical
  behavior into renderer code or scripts.
- V3 compatibility contracts need a durable runtime owner.

## Bet 1: Local Event Ledger

Every user-approved job-search action should become a local event:

- source checked
- job discovered
- job imported
- job viewed
- saved
- hidden
- applied
- followed up
- interview scheduled
- offer received
- rejected
- withdrawn
- stale
- preference labeled
- resume version used

Why this matters:

- The dashboard can explain what changed.
- The tracker can show real funnel history.
- Learning-to-rank can use local outcomes.
- Support reports can summarize behavior without private text.
- V3 agents can plan from facts, not inferred state.

Implementation shape:

- Add a typed `job_events` table with event kind, local subject ids, timestamp,
  source, user action flag, privacy label, and optional sanitized metadata.
- Keep raw notes, resumes, and secrets out of event metadata.
- Derive state from events where practical, but keep denormalized summaries for
  fast UI.

## Bet 2: Opportunity Case File

Jobs should not be plain rows. Each opportunity should have a case file that
links:

- canonical job record
- source runs and imports
- duplicate and repost lineage
- company record
- fit and risk analysis
- resume evidence
- application packet
- tracking state
- contacts and follow-ups
- interview prep
- offer review
- outcome

Why this matters:

- Screens can converge around the job the user is deciding about.
- Agents can operate on a scoped case instead of the whole database.
- Users can see a complete history without hunting through tabs.

Implementation shape:

- Add a `case_file_id` concept that can attach to jobs, applications, imported
  browser records, and manually entered opportunities.
- Keep case files local and exportable.
- Make case files the main unit for v3 workflow orchestration.

## Bet 3: Local Career Graph

The career graph should model facts JobSentinel can reuse:

- roles and target titles
- skills and aliases
- evidence chunks from resumes and projects
- certifications and credentials
- seniority signals
- preferences and constraints
- source outcomes
- company relationships
- salary floors and offer constraints

Why this matters:

- Resume, matching, application, interview, and negotiation workflows can share
  context.
- The app can distinguish aliases, related skills, and dangerous near misses.
- Job recommendations can explain fit through evidence.

Implementation shape:

- Keep taxonomy files in `src/shared/` as the reviewed source of language.
- Add graph edges for alias, broader, narrower, related, confusable, evidence,
  requirement, blocker, and outcome.
- Store graph provenance so users can see whether a fact came from a resume,
  job posting, import, user preference, or model suggestion.

## Bet 4: Source Graph And Policy Ledger

Source handling should become explicit, inspectable infrastructure:

- source class
- official API or public page path
- auth requirement
- restricted status and reason
- source policy reference
- rate limit
- robots and endpoint review state
- parser confidence
- fixture coverage
- last successful run
- failure reason

Why this matters:

- V3 can expand sources without hardcoding fragile behavior.
- Users can see why a source is low-friction, warning-gated, or restricted.
- CI can reject source adapters without policy and fixture coverage.

Implementation shape:

- Extend shared source taxonomy into a source graph with machine-readable
  contracts.
- Require every restricted domain to carry reason, source class, and reference.
- Treat source packs as data plus fixtures, not arbitrary code.

## Bet 5: Browser Companion Protocol

Browser capture needs a real protocol, not a fragile one-off button.

Required protocol properties:

- local pairing initiated by the user
- loopback-only connection
- rotating token or key agreement
- scoped commands
- visible permission list
- no cookie, storage, auth header, hidden page, or network-traffic access
- page-origin binding
- explicit import or action event
- revocation and session history

Why this matters:

- It removes manual copy and paste work.
- It preserves the restricted-source boundary.
- It creates a path for Chrome, Edge, Firefox, Safari, and fallback browser
  workflows.

Implementation shape:

- Add `core/browser_companion` for pairing, permissions, message validation,
  import parsing, and audit events.
- Keep page-specific parsers deterministic and fixture-backed.
- Use browser-side side panels or extension UI for actions, while local storage
  stays in JobSentinel.

## Bet 6: Agent Runtime With Approval Gates

V3 agents should be structured workflow runners, not autonomous background
actors.

Required runtime properties:

- task plan with visible steps
- declared inputs and outputs
- privacy labels for every data read
- local deterministic fallback when possible
- external AI preview and approval when used
- no shell or file access by default
- no final application submission
- no hidden source access
- local audit trail

Why this matters:

- JobSentinel can run skills directly.
- Complex workflows become easier for nontechnical users.
- Risky tasks stay bounded and reviewable.

Implementation shape:

- Add a small workflow schema before adding a general-purpose agent engine.
- Model actions such as read case file, score job, draft packet, ask user,
  open browser, record event, and export document.
- Keep provider-specific AI calls behind the existing gateway.

## Bet 7: Model Governance And Evaluation Lab

Local intelligence should remain governed by JobSentinel:

- checked-in model manifests and lockfiles
- exact revisions and hashes
- backend compatibility checks
- instruction profile versions
- stale-vector detection
- score thresholds by query type
- eval fixtures and hard negatives
- diagnostics visible to the user

Why this matters:

- Model quality can improve without hidden drift.
- V3 can swap backends without changing product semantics.
- Regression tests can catch ranking failures before release.

Implementation shape:

- Extend the current `core/ml` runtime rather than scattering model calls.
- Add an eval runner that measures retrieval, reranking, blockers, fairness
  counterfactuals, adversarial postings, and latency.
- Keep local ML optional for modest-hardware devices, but make setup clear.

## Bet 8: Privacy Receipts And Data Boundaries

Every sensitive workflow should be able to answer:

- What data did JobSentinel read?
- What did it store?
- Did anything leave the device?
- Which provider or source was contacted?
- What did the user approve?
- How can the user delete or export it?

Why this matters:

- Trust becomes visible.
- Support can debug without private data.
- External AI and browser companion features stay auditable.

Implementation shape:

- Add a reusable privacy receipt model.
- Attach receipts to external AI requests, browser imports, source checks,
  exports, backups, support reports, and agent tasks.
- Surface receipts in Settings and case files.

## Bet 9: Pluggable Packs With Signed Manifests

V3 can grow through packs if packs are constrained:

- source packs
- role taxonomy packs
- interview rubric packs
- resume rubric packs
- agent skill packs
- regional source packs
- company watchlist packs

Why this matters:

- JobSentinel can support more careers and regions without bloating core code.
- Community contribution becomes possible without arbitrary code execution.
- Users can choose packs relevant to their search.

Implementation shape:

- Use signed manifests, checksums, versioned schemas, and local install review.
- Prefer declarative selectors, taxonomies, prompts, rubrics, fixtures, and
  examples.
- Ban arbitrary scripts in default consumer packs unless they run in a reviewed
  sandbox with no sensitive data access.

## Bet 10: Edition-Aware Runtime

V3 should support the same product mission across different hardware profiles.

Required properties:

- Essentials package works without large model downloads.
- Optional model packs can be installed, verified, removed, and repaired.
- Runtime profiles control concurrency, batch sizes, background checks, cache
  size, and model use without exposing expert settings by default.
- Feature availability is explained in plain language.
- Core workflows never require high-end hardware.

Why this matters:

- Job seekers often use whatever machine they have.
- Local-first privacy should not become a privilege reserved for powerful
  computers.
- Diagnostics and packaging can be simpler when editions are explicit.

Implementation shape:

- Add an edition manifest for Essentials, Standard, Pro Local, and Developer.
- Keep deterministic matching and small-source checks available in every
  edition.
- Add upgrade and cleanup flows for optional model and source packs.

## Bet 11: Long-Term Compatibility Boundary

V3 should be the first release line with supported backward compatibility and
rollback expectations.

Required properties:

- Pre-v3 data, settings, APIs, packs, and artifacts are unsupported upgrade
  sources and carry no forward-migration or rollback promise.
- V3 data, settings, packs, model metadata, privacy receipts, and browser
  companion permissions get versioned contracts.
- Compatible v3 patch and minor releases support documented rollback when
  migrations are reversible.
- Irreversible migrations require backup-first restore guidance.
- Newer unsupported data is detected safely.

Why this matters:

- V3 is the moment to finalize long-term architecture.
- Users need trust that future updates will not strand their local job-search
  data.
- Packs, browser companion, model cache, and local agents all need stable
  compatibility contracts before broad adoption.

Implementation shape:

- Add a compatibility manifest for database, settings, source packs, model
  packs, browser protocol, AI gateway settings, and export formats.
- Add migration tests for fresh install, the v3.0.0 baseline, compatible v3
  rollback, failed migration retry, backup restore, and unsupported-newer-data
  detection.

## Bet 12: Native OS Adapter Layer

V3 should use native OS features through explicit platform adapters, not ad hoc
renderer calls.

Required properties:

- Rust-owned adapters for platform capabilities.
- Narrow Tauri permissions per window or WebView.
- Platform support matrix and fallback for every feature.
- Native permission prompts preceded by plain JobSentinel explanation.
- No raw private data in OS search, recent items, notification text, or logs
  unless the user explicitly chooses it.

Why this matters:

- Native OS features can make JobSentinel easier to use.
- They also expand privacy and permission risk if they are not controlled.

Implementation shape:

- Add `core/platform` submodules for deep links, notifications, file
  associations, vault unlock, local auth, OS search metadata, calendar/reminder
  exports, and package diagnostics.
- Keep all platform-specific behavior behind typed commands and tests.

## Bet 13: Security UX As Infrastructure

V3 should reduce friction by making security state reusable and visible.

Required properties:

- scoped consent records
- versioned acknowledgement records
- privacy receipts
- revocation controls
- Security Doctor diagnostics
- repair flows
- compact prompts for repeated low-risk actions
- full preview for sensitive or changed actions

Why this matters:

- More security prompts can make users ignore security.
- The right architecture can reduce repeated prompts without weakening controls.
- Users need recovery, not only warnings.

Implementation shape:

- Add Rust-owned consent, receipt, and security-diagnostic services.
- Make browser companion, MCP, external AI, source packs, model packs, backups,
  exports, and restricted-source flows use the same security UX primitives.

## Bet 14: Downloadable Pack Runtime

V3 should turn v2.9 downloadable skills into a broader extension ecosystem
without allowing arbitrary untrusted automation.

Required properties:

- pack manifest schema
- signed package verification
- compatibility range
- privacy labels
- permission summary
- install review
- update review
- uninstall and rollback
- fixture validation
- Rust-enforced action scopes

Why this matters:

- JobSentinel can grow across roles, regions, sources, and workflows.
- Community contributions become possible without weakening local trust.
- Agents and skills can be powerful while still bounded.

Implementation shape:

- Add a Rust-owned pack registry and validator.
- Support skill, agent, workflow, role, regional, source, rubric, eval, and
  template packs.
- Keep executable behavior off by default unless a sandbox and permission model
  are proven.

## Bet 15: Regional Runtime Boundary

V3 should make regions configurable without hard-coding every market into core
logic.

Required properties:

- region manifest schema
- source classes by region
- pay, currency, location, and work-mode normalization
- CV and resume format profiles
- taxonomy bridges for O*NET, UK SOC, ESCO, India NCO, and future systems
- region-specific eval fixtures
- incomplete-coverage labels when starter packs are not comprehensive

Why this matters:

- V3 can try to help users in the UK, EU, and India without pretending the
  product has full global maturity.
- Future region packs can improve coverage without changing Rust core.
- Matching, source warnings, and resume guidance stop assuming U.S. defaults.

Implementation shape:

- Add `RegionManifest`, `PayPeriod`, `LocationProfile`, `CvProfile`, and
  `TaxonomyBridge` contracts in Rust-owned shared model code.
- Treat regional packs as signed packs with fixtures and compatibility ranges.
- Keep legal, immigration, tax, and financial content as guidance labels, not
  advice or eligibility decisions.

## Bet 16: Sandboxed Adapter Runtime

Declarative source packs should stay the default, but some public sources may
need a carefully reviewed dynamic adapter path.

Required properties:

- no ambient filesystem access
- no shell or process execution
- no credential, vault, browser storage, or database access
- no arbitrary network calls
- CPU, memory, time, and output-size limits
- Rust-owned URL fetch, rate limit, source policy, and storage decisions
- fixture-only developer test mode
- pack signature and permission review before activation

Why this matters:

- The source ecosystem is too large and dynamic for selectors alone.
- A sandboxed runtime can prevent community packs from becoming unrestricted
  local code.
- The same boundary can support future eval, transform, and parser packs.

Implementation shape:

- Research WebAssembly Component Model, WASI, QuickJS, and Deno Core as
  candidate runtimes.
- Prefer WebAssembly or declarative transforms when possible.
- Keep restricted authenticated sources outside dynamic adapter scope.

## Bet 17: Local Vector Store Contract

The Qwen3 evidence engine needs a portable vector contract that survives model,
chunker, and pack changes.

Required properties:

- vector provenance for model id, revision, dimension, instruction profile,
  chunker version, normalizer version, and manifest hash
- stale-vector detection
- deterministic chunk ids
- separate dense, lexical, skill, blocker, and reranker signals
- embedded vector storage evaluation before introducing a separate service
- memory and disk budgets for Essentials and Standard packages

Why this matters:

- Mixing vector revisions silently weakens matching quality.
- Users need cleanup, rebuild, and repair flows for local model data.
- V3 compatibility must include model-derived data, not only SQL tables.

Implementation shape:

- Evaluate SQLite-compatible vector extensions such as sqlite-vec before
  adopting a heavier local service.
- Keep vector rebuild and doctor commands Rust-owned.
- Store every vector with a freshness key and model provenance.
