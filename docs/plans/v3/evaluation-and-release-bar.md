# Evaluation And Release Bar

V3 should not be considered ready because the ideas are exciting. It should be
ready only when the product works across real job-search workflows, source
classes, resumes, local models, browser capture paths, privacy boundaries, and
platforms.

## Release Bar Summary

Before v3 ships, JobSentinel should prove:

- A novice user can set up a search and understand the main workflow.
- A novice user can install, launch, finish first-run setup, and reach useful
  job-search help without accounts, external AI, or model downloads.
- The dashboard gives useful next actions without requiring spreadsheet work.
- Source checks and browser companion flows handle public and restricted
  boundaries correctly.
- Resume/job matching explanations are evidence-backed and eval-tested.
- Application packet and tracker flows preserve final user review.
- External AI remains optional, previewed, approved, and locally logged.
- Sensitive data stays local unless the user explicitly chooses otherwise.
- Windows 11+, macOS, and Linux are verified for the major workflows.
- Essentials package works on modest hardware without large model downloads.
- V3 migration, backup, compatible rollback, and unsupported-newer-data handling
  are verified.
- UK, EU, and India starter framework paths are verified if shipped in v3.
- OS-native model and OCR helpers are verified as reviewable suggestions with
  deterministic fallbacks.
- Commercial-superiority workflows beat tracked alternatives on defined
  metrics for at least the main install, import, match, apply, track, interview,
  offer, recovery, and export paths.

## End-To-End User Scenarios

Create manual and automated verification scenarios for:

- first-run setup without accounts
- install, update, repair, and rollback paths
- saved search setup
- official source check
- employer careers discovery
- public ATS import
- restricted-source browser session
- Browser Import or browser companion visible import
- native deep link, notification, file association, and permission fallback
- duplicate or repost detection
- job risk and ghost-job review
- resume import and parsing
- resume-to-job match
- application packet preparation
- application tracking state changes
- reminder and follow-up
- interview prep
- offer and pay review
- external AI public-job summary
- external AI sensitive-data blocked path
- safe support report
- export and backup
- edition selection and Essentials first-run path
- security doctor, consent revocation, privacy receipt, and repair flows
- regional starter setup for UK, EU, or India when a region pack ships
- OCR-assisted visible import from user-selected content
- OS-native micro-assist fallback when the platform feature is unavailable
- commercial benchmark demo against tracker, resume-scanner, browser-copilot,
  and auto-apply product classes

Each scenario needs success criteria, expected privacy boundary, and evidence
captured outside private user data.

## Source Verification

For every native source adapter:

- source manifest exists
- source class is declared
- rate limit is declared
- restricted status and rationale are declared when needed
- fixtures cover list and detail parsing
- parser returns canonical records
- URL validation is covered
- response size limits are covered
- safe error messages are covered
- source health appears in UI

For restricted sources:

- warning appears before sign-in or restricted action
- source session is user-started
- no session material is stored
- no hidden browser state is read
- no scheduled background refresh occurs
- imported records require local review
- privacy reminder and close controls work

## Browser Companion Verification

If v3 includes an extension or side panel, verify:

- pairing is user-approved
- token or key material rotates
- command scope is enforced
- browser origin is recorded
- revocation works
- extension cannot read local database directly
- extension cannot access vault material
- visible-page import works
- hidden/background page import is rejected
- copied text and URL token scrubbing work
- review queue prevents silent durable records
- Chrome, Edge, Firefox, and Safari strategy is documented

## ML And Ranking Verification

For local intelligence:

- model lock validates exact revisions and hashes
- model doctor handles missing, stale, and mismatched models
- cache paths are portable and user-visible
- fallback behavior is clear
- dense retrieval and reranker behavior pass eval fixtures
- Qwen3 requirement-level retrieval and reranking pass hard-negative fixtures
- hard blockers cap otherwise strong matches
- exact skill matches do not over-expand related skills
- seniority mismatch is explained
- "why not matched" works
- adversarial posting text is handled as untrusted input
- fairness counterfactuals are included
- latency and memory are measured on modest-hardware profiles
- OS-native local model helpers are measured against deterministic and Qwen3
  alternatives for accepted task types
- regional terminology fixtures pass before a region pack is labeled ready

Do not ship ranking changes without eval deltas and explanation review.

## Regional Pack Verification

For each region pack that ships:

- region manifest validates
- source classes are declared
- pay and currency normalization works
- location and work-mode parsing works
- CV or resume profile behavior is documented
- taxonomy bridge provenance is stored
- incomplete-coverage labels appear where appropriate
- source warnings use region-appropriate plain language
- Qwen3 and deterministic matching pass regional terminology fixtures
- region pack can be installed, disabled, removed, and upgraded independently

## Agent Workflow Verification

For every v3 agent workflow:

- declared inputs and outputs
- visible task plan
- privacy labels
- local-only path or external AI path
- approval points
- cancel path
- error state
- audit trail
- deterministic fallback where possible
- no final submission automation
- no hidden restricted-source access
- no unrestricted shell, file, or network access

Agents must be tested with malicious job postings, hidden instructions, missing
inputs, conflicting requirements, and low-confidence data.

## External AI Verification

For every provider-backed workflow:

- feature privacy label is registered
- provider is disabled by default
- preview is shown
- redaction is applied when enabled
- user can edit, cancel, or approve
- backend validates outgoing payload
- sensitive data is blocked unless feature-specific opt-in exists
- metadata-only request history is stored locally
- provider errors are safe and understandable
- local fallback exists or the feature is clearly labeled external-AI-required

No feature may bypass the gateway.

## Privacy And Security Verification

Run threat reviews for:

- browser companion loopback protocol
- local MCP server
- source packs
- model downloads
- downloadable pack registry
- sandboxed dynamic adapters
- OS-native model and OCR helpers
- external AI provider payloads
- safe support reports
- encrypted backup and restore
- multi-profile vaults
- import/export
- URL handling
- extension permissions

Required checks:

- secret scanning
- dependency and crate pin checks
- action pin checks
- file-size bloat checks
- URL validation tests
- CSP and XSS tests
- keyring and passphrase tests
- command execution boundaries
- source policy manifest validation
- privacy-label manifest validation
- privacy receipt validation
- consent reset and revocation tests
- Security Doctor repair-flow tests
- pack quarantine and self-test validation
- sandbox denial tests for filesystem, shell, credential, database, and ambient
  network access
- OCR provenance and review tests
- OS-native model fallback tests

## Migration And Rollback Verification

Because v3 is the first long-term compatibility boundary, verify:

- fresh v3 install
- v3.0.0 is the first supported compatibility baseline
- pre-v3 readers, migration shims, fixtures, and release claims are absent
- backup-before-v3.x migration
- failed v3.x migration retry
- v3.x backup restore
- compatible v3 patch rollback
- unsupported-newer-data detection
- source pack compatibility
- model pack compatibility
- settings compatibility
- browser companion permission reset when protocol changes
- safe support report when migration fails

## UX And Accessibility Verification

V3 must be manually verified across:

- keyboard navigation
- screen reader labels
- focus states
- loading states
- empty states
- error states
- narrow width
- high zoom
- reduced motion
- low contrast risk
- nontechnical copy
- first-run flow
- advanced settings hidden behind expert mode

Use real novice-user tasks, not only component snapshots.

## Cross-Platform Verification

Required platform checks:

- Windows 11 install, launch, vault, file import/export, browser companion,
  model cache, and source checks.
- macOS install, launch, vault, file import/export, browser companion,
  model cache, and source checks.
- Linux install, launch, vault or passphrase fallback, file import/export,
  browser companion strategy, model cache, and source checks.
- Essentials package install, launch, setup, source check, resume import,
  deterministic matching, tracker update, pay review, and support report on
  modest-hardware profiles.
- Native OS integrations for each platform that ships them, including fallback
  behavior when the platform feature is unavailable.
- OS-native model and OCR helpers for each platform that ships them, including
  unavailable-platform fallback.

If a platform lacks a feature, the UI must say so plainly and offer a fallback.

### Essentials Numeric Thresholds

These Gate 4 release thresholds are derived from measured evidence on the
macOS 27 arm64 host and the enforced 8 GiB zero-swap Linux guest recorded in
`docs/harness/evidence/`. They bind release claims only for measured
platforms; Windows 11 and macOS 26 must be measured before claiming them.

| Threshold | Limit | Measured basis |
| --- | --- | --- |
| Model-free macOS DMG size | <= 25,000,000 bytes | 14,224,509 bytes |
| Model-free installed macOS app size | <= 50,000,000 bytes | 29,704,192 bytes |
| Model-free Debian package size | <= 30,000,000 bytes | 16,181,176 bytes |
| Model-free AppImage size | <= 120,000,000 bytes | 89,344,504 bytes |
| Model-free installed Linux binary size | <= 60,000,000 bytes | 39,689,664 bytes |
| Installed first visible window, polling upper bound | <= 2,000 ms | 825 ms, 582 ms, 277 ms |
| Complete Essentials first-run journey peak memory on an enforced 8 GiB zero-swap profile | <= 1,073,741,824 bytes with zero memory-limit and OOM events | 687,190,016 bytes |
| Persistence relaunch peak memory on the same profile | <= 1,073,741,824 bytes | 717,479,936 bytes |
| Aggregate app-coalition observed maximum RSS on an unconstrained macOS host | <= 536,870,912 bytes | 246,251,520 bytes |
| Model-payload filenames inside model-free packages | exactly 0 matches | 0 matches |

The stronger-local download payload is not a threshold: the lock owns the
exact 2,406,043,460-byte required total for the governed Qwen3 pair, and
changing it requires a model-lock change with fresh calibration evidence.

## Triage Framework

Classify each v3 idea before implementation:

| Class | Meaning |
| --- | --- |
| Now | Strong user value, clear boundaries, fits existing architecture, testable. |
| Next | Valuable but needs a prototype, source review, eval data, or UX study. |
| Later | Valuable but depends on other v3 primitives. |
| Moonshot | Big upside, high uncertainty, needs research before product commitment. |
| Cut | Conflicts with Rule 0, user control, platform boundaries, or product trust. |

Every accepted idea needs:

- user problem
- data boundary
- source or provider boundary
- architecture owner
- tests
- manual verification path
- docs update path
- rollback or disable strategy

## Commercial Benchmark Verification

Before v3 ships, run a product benchmark against commercially similar tools.
Use current public docs and hands-on testing where feasible.

Benchmark categories:

- job tracker and CRM
- resume scanner and builder
- browser save, autofill, and copilot
- job board and source aggregator
- AI auto-apply
- general chatbot workflow

For each category, compare:

- account requirement
- local-first privacy posture
- install and first-use time
- visible job import effort
- application logging effort
- resume evidence quality
- match explanation quality
- source breadth
- recovery behavior
- export and lock-in
- accessibility and nontechnical usability
- modest-hardware behavior
- regional readiness

V3 does not need to match unsafe final-submit automation, but it must make the
safe user-controlled path faster, clearer, and more helpful.

## V3 Done Definition

V3 is not done until:

- All accepted v3 features are represented in maintained docs.
- Feature privacy labels cover every sensitive or external-AI-capable workflow.
- Source manifests cover every native source.
- Agent workflows have explicit approval gates.
- Browser companion flows pass security review.
- ML ranking changes pass eval and manual explanation review.
- Main README and wiki reflect the shipped v3 experience.
- Release assets, checksums, SBOMs, and attestations are produced.
- Manual UI verification covers every click and action in the shipped surface.
- Compatibility and rollback docs define v3.0.0 as the first supported baseline
  and make clear that pre-v3 upgrades are unsupported.
