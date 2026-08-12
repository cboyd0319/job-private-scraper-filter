<!-- Records notable JobSentinel changes by release. -->

# Changelog

All notable changes to JobSentinel will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Current LinkedIn source policy: Starting with `2.9.0`, legacy LinkedIn
cookie/session storage and automatic account-backed monitoring are disabled.
Historical entries below may describe removed experiments. The current
supported path is user-opened search links plus user-clicked Browser Import.

## [Unreleased]

### Changed

- Development source metadata and the compiled signed-pack runtime now identify
  `3.0.0`. This source line is not published; `v2.9.5` remains the latest
  public release.

## [2.9.5] - 2026-07-18

Published release for the full repository refactor. The release workflow and
public asset verification completed successfully.

### Changed

- **Repository ownership** - Reorganized the frontend, Rust backend, scripts,
  tests, docs, platform tooling, and release tooling under explicit owners with
  private implementation leaves and bounded facades.
- **Cargo workspace** - Added a root explicit-member virtual workspace with
  inherited package metadata, exact dependency policy, shared lint policy, a
  Tauri-free core crate, and a thin private Tauri shell.
- **Maintainability contract** - Enforced deterministic member and test
  discovery, compiled Rust source ownership, modules before crates, and file
  caps for production, scripts, tests, and maintained documents.
- **Database startup integrity** - Existing encrypted databases now require a
  verified pre-migration snapshot. Successful migrations run quick,
  foreign-key, and scheduled full integrity checks before startup completes.
- **Reviewed job import** - Moved fetching, parsing, staged review state, and
  atomic insertion into the core owner. Confirmation saves the exact reviewed
  record without a second fetch.
- **Private domain owners** - Narrowed scraper, resume, notification, salary,
  market, automation, credential, and database APIs to the contracts used by
  the app.
- **Repository cleanup** - Removed orphan source, unused dependencies, obsolete
  compatibility paths, transition-only sensors, empty directories, and unused
  generated mobile and store icons.
- **Release process** - Release publication now uses manual dispatch from an
  existing version tag instead of publication as a side effect of pushing a
  tag. It also adds a hosted `windows-linux` path for releases with locally
  built macOS assets and scopes manual public verification so Windows/Linux
  checks do not spend a macOS runner.
- **Release closeout docs** - Closed `v2.9.1` as published and verified
  history across maintained docs, active plans, validation ledgers, and release
  process guidance.

### Fixed

- **Credential test prompts** - Default workspace tests no longer access the
  live operating-system credential store. Live roundtrips require explicit
  opt-in.
- **Occupied development ports** - Browser test and Browser Import setup paths
  recover from occupied default loopback ports by selecting an available local
  port.
- **Stale repository guidance** - Architecture, contributor, testing, SQLx,
  security, platform, release, harness, and ownership docs now follow the final
  workspace.

## [2.9.1] - 2026-06-22

Maintenance and repo cleanup release. This patch keeps the `2.9.0` user-facing
behavior intact while making the repository easier to verify and maintain.

### Changed

- **Dependency maintenance** - Refreshed direct npm and Rust crate pins to
  current stable versions and kept package, Tauri, and Cargo metadata aligned
  at `2.9.1`.
- **Maintainability cleanup** - Replaced custom lazy statics with standard lazy
  initialization where safe, narrowed Clippy allowances, split oversized tests
  and mocks, and moved source-health checks into smaller focused modules.
- **Docs and release state** - Updated maintained docs so `v2.9.1` is clearly a
  maintenance line without claiming public assets before this patch is cut.
- **Regression validation** - Added a v2.9.1 regression validation ledger and
  recorded local browser, frontend, backend, build, docs, bloat, harness, and
  isolated macOS native startup evidence.

### Fixed

- **Repo clutter and file-size drift** - Removed local disposable artifacts,
  split near-budget maintained files, and removed obsolete file-size
  exceptions after the split work.
- **Release-readiness expectations** - Updated release-readiness tests and
  active planning docs to target `2.9.1` metadata without claiming public
  assets before publication.
- **Startup keyboard shortcut readiness** - Installed app-wide shortcuts before
  the visible shell is interactive so keyboard help, command palette, and
  navigation shortcuts do not miss early key presses during startup.

## [2.9.0] - 2026-06-20

Job-search readiness release for the full local-first workspace. This release helps users discover, judge, tailor, track, and negotiate job opportunities while keeping sensitive job-search data local by default.

### Added

- **Downloadable Agent Skills** - Added spec-compliant job-search, resume, application, outreach, interview, tracking, posting-risk, and offer/pay skills under `skills/`.
- **Browser Import workflow** - Kept LinkedIn-compatible job capture on user-opened pages and user-clicked import, with no session-cookie storage, token replay, background monitoring, result-list crawling, or account automation.
- **Under-the-hood release mechanics** - Public docs now call out the local architecture that powers the product: source taxonomy and routing, restricted-source Workbench, evidence-bounded resume matching, privacy-first AI gateway, local vault and safe reports, Agent Skills packaging, release supply-chain checks, and local semantic matching.
- **Qwen3 local matching architecture** - Embedded-ML builds can use governed Qwen3-Embedding-0.6B retrieval plus bounded Qwen3-Reranker-0.6B reranking, blended with exact skill, BM25, blocker, seniority, evidence, and provenance signals. `crates/jobsentinel-local-ai/models.lock.toml` pins model identity, revisions, hashes, sizes, licenses, backends, instruction profiles, thresholds, and stale-vector rules.

### Changed

- **Release metadata** - Bumped npm, Cargo, and Tauri package metadata to `2.9.0`.
- **Release documentation** - Rebuilt the README as a scannable product front door, refreshed the docs hub, capabilities page, release notes, quick start, release process, macOS development notes, active release plans, risk register, and public wiki posture for `2.9.0`.
- **Release-readiness harness** - Updated the release readiness check so it verifies the durable public-download contract instead of stale pre-release wording, and added focused regression coverage for checksum guidance.
- **Release supply chain** - Consolidated CI/release workflows now generate release SBOM manifests, require provenance and SBOM attestations, and verify public macOS supply-chain evidence.
- **No-account platform release path** - Hosted releases can now publish an explicitly `_unsigned` Windows MSI and NSIS setup EXE when signing credentials are unavailable, while preserving the signed Windows path, no-account macOS path, Linux package checks, checksums, SBOMs, and attestations.
- **Local macOS release evidence** - Rebuilt the current-source no-account universal DMG, verified checksum, metadata, universal architecture, signature, mounted and installed launch smoke, visible window, and private isolated-data creation, then staged macOS SBOM and Agent Skills archive checksums locally.
- **Dependency posture** - Release dependency checks now require exact latest stable direct pins, current package-manager and tool baselines, lockfile freshness, and pinned GitHub Actions.
- **Latest stable package pins** - Refreshed `lint-staged` to `17.0.8` after the release dependency freshness gate detected upstream drift.
- **Local model integrity** - Optional embedded ML downloads now use an exact Hugging Face revision and verify SHA-256 checksums before cache status or model loading succeeds.
- **Browser Import hardening** - The generated browser button now sends through
  clean transient browser APIs instead of host-page `window.fetch` or
  `JSON.stringify` while keeping one-use local tokens.
- **macOS dependency surface** - Removed unused direct WebKit and cookie
  extraction dependencies from the Tauri crate while keeping native
  user-presence protection for the local credential vault key.
- **Company research privacy** - Company research suggestions now use
  memory-only caching and clear the legacy localStorage cache key.
- **Onboarding accessibility** - The tour overlay now behaves as a modal dialog
  with focus handling, Escape close, and viewport-clamped placement.
- **Public wiki posture** - Synced public `Home.md` and `Capabilities.md` wiki pages with the current `2.9.0` release-line, macOS, LinkedIn, security, Agent Skills, and under-the-hood architecture posture.

### Fixed

- **Auto-refresh privacy gate** - Background scraping no longer runs on
  startup or loop iterations unless auto-refresh is enabled.
- **Alert race protection** - Immediate-alert delivery now uses an atomic
  database claim so overlapping scrape cycles cannot duplicate the same alert.
- **JobsWithGPT transport** - Optional JobsWithGPT outbound endpoints now
  require HTTPS at config validation and runtime fetch validation.
- **Import deduplication** - Manual URL import and Browser Import now use the
  shared company, title, location, and URL hash contract used for job
  deduplication.
- **Application Assist validation** - Profile validation guidance stays visible
  after failed save and clears after the submitted errors are corrected.
- **Application Assist pay-history review** - Current-pay, previous-pay, and
  salary-history questions now get a separate review prompt that steers users
  toward role range and target-pay evidence instead of unsupported past-pay
  claims.
- **Hiring Trends QA** - Cross-browser Hiring Trends checks now target stable
  button semantics for location heatmap interactions.
- **Release test gate** - Rust integration fixtures now include the Browser
  Import port config field so the full `cargo test` release gate compiles and
  runs after config schema changes.
- **Browser automation script safety** - Application Assist dropdown filling
  now JSON-encodes selector and value literals before page evaluation, and the
  security harness rejects the old manual quote-escaping pattern.
- **Resume preview CSS filtering** - Resume Builder sanitization now decodes
  CSS escapes before blocking stylesheet imports, `@font-face`, `url(...)`, and
  `image-set(...)` resource loads.
- **Browser Import token checks** - Local Browser Import token matching now
  requires exact non-early-exit string comparison for header and body tokens.
- **Release SPDX attestation coverage** - Release SBOM attestations now target
  installable asset paths directly so Windows MSI, Windows setup EXE, macOS
  DMG, Linux AppImage, and Linux deb artifacts all receive SPDX attestations.

## [2.7.7] - 2026-06-06

Source version in `main`. Public no-account macOS assets are released and
verified for `v2.7.7`; latest full cross-platform public release remains
`v2.7.5` as of 2026-06-06 until Windows and Linux `2.7.7` assets are rebuilt
and verified.

### Fixed

- **Hiring Trends refresh** - Empty first-run and migrated local databases now
  refresh against the current jobs schema, creating a neutral empty snapshot
  instead of showing a generic app error.
- **Hiring Trends semantics** - Local market aggregates now use the current
  job `created_at` field, keep hidden or dismissed jobs as market evidence, and
  leave filled-job counts neutral until a source-backed closure signal exists.
- **Credential cleanup tests** - Credential integration coverage now keeps
  active notification credentials round-tripped when secure storage is
  available, accepts sanitized secure-storage denial when the OS keyring is
  locked, and verifies legacy LinkedIn cookie and expiry entries are removed.

### Documentation

- **Release state** - User and developer docs now distinguish the public
  `v2.7.7` no-account macOS package from the latest full cross-platform
  release, `v2.7.5`.
- **Release process** - Developer docs now document verified local build plus
  manual upload as a supported production path when the same release gates pass.

## [2.7.6] - 2026-06-06

### Fixed

- **Release verification** - Public macOS artifact verification now uses the
  workflow GitHub token so release-event checks do not fail on unauthenticated
  GitHub API rate limits.

## [2.7.5] - 2026-06-06

### Fixed

- **Application Assist save behavior** - Saving an unchanged valid profile now
  skips the backend write while still letting new or invalid profiles surface
  required-field guidance from the Save Profile control and keyboard shortcut.

## [2.7.4] - 2026-06-06

### Fixed

- **Application Assist validation** - Profile save validation now stays
  clickable immediately after field edits so required-field guidance appears
  reliably across Chromium and WebKit.

## [2.7.3] - 2026-06-06

### Fixed

- **macOS release packaging** - DMG verification now retries transient
  `hdiutil verify` resource-busy races after image creation while preserving
  hard checksum and disk-image failures.

## [2.7.2] - 2026-06-06

### Fixed

- **Release packaging CI** - macOS release cleanup now streams stale DMG assets
  directly so Bash 3 with `set -u` does not fail when no stale macOS assets
  exist. This keeps draft release uploads moving through the verified DMG and
  checksum path.

## [2.7.1] - 2026-06-05

### Added

- **Beta Feedback System** - Privacy-first feedback for beta testers
  - GitHub Issues integration (primary channel) with pre-filled templates
  - Google Drive upload (secondary channel) for users without GitHub
  - Full PII sanitization (paths, emails, webhooks, tokens, IPs)
  - Cross-platform support (macOS, Windows, Linux)
  - Debug log ring buffer (100 events) for diagnostics
  - 11 new Tauri commands: `open_github_issues`, `open_google_drive`, `reveal_file`,
    `get_system_info`, `get_config_summary`, `generate_feedback_report`,
    `get_debug_log_formatted`, `get_debug_log_events`, `clear_debug_log_cmd`,
    `get_feedback_filename`, `save_feedback_file`
- **GitHub Issue Templates** - Structured forms for bug reports, feature requests, questions
- **macOS deployment verification** - Strict package checks for whole app data tree
  permissions, installed-app launch smoke, checksums, bundle metadata,
  architectures, release asset labels, and public artifact verification.
- **Startup recovery** - Settings and first-run failures now expose safe support
  report copy/save actions instead of leaving users stuck.

### Changed

- **Harness Engineering alignment** - Added root `AGENTS.md`, structured harness docs,
  exec plan templates, tech debt tracker, and `npm run harness:check` for agent-facing
  documentation and workflow validation
- **macOS Universal Binary** - No-account universal `.dmg` packaging now covers both Intel and Apple Silicon Macs
  - Updated GitHub Actions release workflow to build universal binary
  - Installs both aarch64-apple-darwin and x86_64-apple-darwin Rust targets
  - Uses `--target universal-apple-darwin` flag for Tauri builds
  - Eliminates need for separate Intel and Apple Silicon downloads
- **macOS release workflow** - No-account DMGs are labeled before upload, stale
  Mac release assets are removed, API-key notarization secrets are materialized
  into owner-only temporary `.p8` files, and Developer ID builds require
  Gatekeeper verification.
- **Local data hardening** - macOS and Linux app-owned data/config directories
  and existing local database backups are tightened to owner-only permissions.
- **Settings modal behavior** - Settings and nested feedback dialogs share a
  scroll lock and focus-safe modal path.
- **Navigation** - Desktop navigation now stays as a compact icon rail so app
  content remains visible on macOS.

### Fixed

- **Feedback command wiring** - Frontend feedback actions now invoke registered Tauri commands
  for GitHub, Google Drive, file reveal, and native report saving.
- **macOS local data permissions** - Legacy `jobs.db`, WAL/SHM, and
  pre-migration backup files no longer remain group/world readable after app
  startup.
- **Settings recovery** - Startup and Settings failure states now let users copy
  or save safe support reports.
- **Notification tests** - Email and Slack test actions now use explicit button
  labels.
- **macOS release docs** - Stale public Mac assets are no longer presented as
  passing current data-permission gates.
- **Release packaging CI** - Windows release builds initialize Windows version
  metadata correctly, macOS release cleanup avoids Bash 4-only syntax, and
  asset uploads keep releases in draft until manual publication.
- **macOS visible-window smoke** - Package verification now fails if launch
  smoke starts a process without exposing a normal on-screen app window.

## Historical Changelog Archives

Older root changelog entries are split by release band to keep active release
history under the repository file-size contract.

- [v2.6.4 through v2.6.0](docs/releases/changelog-v2.6.md)
- [v2.5.5 through v2.5.0](docs/releases/changelog-v2.5.md)
- [v2.4.0 through v2.0.0](docs/releases/changelog-v2.0-to-v2.4.md)
- [v1.6.0 through v1.0.0-alpha](docs/releases/changelog-v1.md)
