<!-- Routes readers to JobSentinel's maintained user, developer, security, research, and planning documentation. -->

# JobSentinel Documentation

This directory is the working documentation system for JobSentinel. The root
[README](../README.md) is the public project front door; this page routes
users, contributors, reviewers, and coding agents to maintained docs.

JobSentinel is an open-source, local-first job-search assistant for finding
real, relevant, fairly compensated work while keeping sensitive job-search data
under user control. It is designed for technical and non-technical jobs and for
users who should not need terminal commands, GitHub knowledge, or debugging
skill to get value.

## Project Rules

- **Rule 0:** user privacy and security cannot be circumvented.
- Core workflows must work locally without a hosted account, telemetry, cloud
  sync, or external AI provider.
- External AI is optional, disabled by default, and must go through the
  documented [privacy-first AI gateway](security/privacy-first-ai-gateway.md).
- Sensitive job-search data stays local unless the user explicitly chooses,
  previews, and approves what will be sent.
- JobSentinel must not send applications on the user's behalf, use deceptive
  resume tactics, manipulate ATS systems, hide restricted-source collection,
  solve CAPTCHAs, store restricted-site auth material, or evade platform
  controls.

## Start Here

| Reader | First docs | Use them for |
| --- | --- | --- |
| Job seeker | [Quick Start](user/QUICK_START.md) | Install, set up, and get help without technical setup. |
| Support request | [Safe support reports](features/user-data-management.md) | Save or copy a safe support report locally, review it, and share it only if you want help. |
| Design reviewer | [Design System](design/design-system.md), [Design Spec](design/design-spec.md), [Style Guide](style-guide/README.md), [Frontend Testing](developer/FRONTEND_TESTING.md) | Review visual direction, UI tone, accessibility checks, and responsive expectations. |
| Privacy reviewer | [Privacy](../PRIVACY.md), [Responsible AI](../RESPONSIBLE_AI.md), [AI gateway](security/privacy-first-ai-gateway.md) | Review data boundaries and external-AI gates. |
| Research or grant reviewer | [Research notes](research/README.md), [public roadmap](../ROADMAP.md), [v3 planning](plans/v3/README.md), [current status](harness/current-status.md) | Review evidence, evaluation boundaries, product pillars, and major-release ideas. |
| Contributor | [Getting Started](developer/GETTING_STARTED.md), [Architecture](developer/ARCHITECTURE.md), [Testing](developer/TESTING.md) | Build, understand, and verify the app. |
| Coding agent | [AGENTS.md](../AGENTS.md), [Harness](harness/README.md), [Verification Matrix](harness/verification-matrix.md) | Follow repo operating rules and required checks. |

The maintained external source index lives in
[References and External Sources](references.md).

## Current State

- Development package metadata version: `3.0.0`. This source line is unreleased.
- Current public release: `v2.9.5`, published on 2026-07-18 UTC at
  `https://github.com/cboyd0319/JobSentinel/releases/tag/v2.9.5`.
- Release assets are produced from manual release workflow dispatches on GitHub
  version tags, or from verified local platform builds, and verified after
  publication with checksum, SBOM, attestation, and platform package checks.
- Current macOS full-public-readiness is 94%; no-account path completion is
  100% at the 94% public-readiness ceiling. The no-account universal DMG path
  covers checksum, metadata, architecture, launch, install, and isolated-data
  smoke gates. Zero-friction Gatekeeper-ready public distribution still
  requires Apple Developer Program materials, Developer ID signing,
  notarization, stapling, and signed-artifact verification.
- Verified local build plus manual upload remains a supported release path when
  the same version, harness, package, checksum, SBOM, and public-artifact
  verification gates pass. Local macOS build and upload is the preferred
  no-account release path when hosted macOS proof is not needed; hosted
  cross-platform release builds remain available by manual dispatch from a
  release tag.
- Current active work is the v3 master execution plan after the verified
  `v2.9.5` release. Pending and unreleased work is tracked in
  [CHANGELOG.md](../CHANGELOG.md) and the
  [active plans index](plans/README.md#current-active-plans).
- Historical release notes are indexed in
  [docs/releases](releases/README.md), but this page should not be used as a
  release log.
- Product priorities live in the root [ROADMAP](../ROADMAP.md); developer
  routing lives in [docs/ROADMAP](ROADMAP.md).

## User Docs

| Need | Doc |
| --- | --- |
| Complete capabilities map | [Features And Capabilities](features/capabilities.md) |
| Install and first run | [Quick Start](user/QUICK_START.md) |
| Update or go back to an older version | [Updating Or Going Back](user/UPDATES.md) |
| Open job searches on outside sites | [Search Links](user/DEEP_LINKS.md) |
| Manage local data and safe support reports | [User Data Management](features/user-data-management.md) |
| Set up alerts | [Notifications](features/notifications.md) |
| Understand job source checks | [Job Source Status](features/job-source-status.md) |

## Feature Docs

| Area | Doc |
| --- | --- |
| Full capability map | [Features And Capabilities](features/capabilities.md) |
| Ghost-job and stale-posting protection | [Ghost Detection](features/ghost-detection.md) |
| Pay protection and salary transparency | [Pay Protection](features/pay-protection.md) |
| Resume fit review | [Resume Match](features/resume-matcher.md) |
| Resume writing | [Resume Builder](features/resume-builder.md) |
| Application board | [Application Tracking](features/application-tracking.md) |
| Review-first application form help | [Application Assist](features/application-assist.md) |
| Official-source job monitoring | [Job Sources](features/job-sources.md) |
| Browser import button | [Browser Import Button](features/browser-import.md) |
| Hiring trends | [Hiring Trends](features/hiring-trends.md) |
| Fit priorities | [Fit Review](features/smart-scoring.md) |
| Work-mode matching | [Remote Preference Matching](features/remote-preference-scoring.md) |
| Broad skill matching | [Synonym Matching](features/synonym-matching.md) |
| Structured resume import | [Resume Data Import](features/json-resume-import.md) |
| Saved secrets | [Saved Secrets](features/saved-secrets.md) |

## Design Docs

| Need | Doc |
| --- | --- |
| Design docs hub | [Design Docs](design/README.md) |
| Design-system source | [Design System](design/design-system.md) |
| Product design spec | [Design Spec](design/design-spec.md) |
| UI writing style | [Style Guide](style-guide/README.md) |
| Frontend verification | [Frontend Testing](developer/FRONTEND_TESTING.md) |

## Privacy, Security, And Responsible AI

| Topic | Doc |
| --- | --- |
| Privacy philosophy and data boundaries | [PRIVACY.md](../PRIVACY.md) |
| Responsible AI boundaries | [RESPONSIBLE_AI.md](../RESPONSIBLE_AI.md) |
| External-AI gateway architecture | [Privacy-first AI Gateway](security/privacy-first-ai-gateway.md) |
| Local secret storage | [Local Secret Vault And Keychain Integration](security/KEYRING.md) |
| Security reporting | [SECURITY.md](../SECURITY.md) |

## Research Docs

Research and evaluation use public job postings and synthetic candidate
profiles by default. Real user data requires explicit informed consent.

- [Research README](research/README.md)
- [Job-seeker behavior](research/job-seeker-behavior.md)
- [ATS transparency](research/ats-transparency.md)
- [Ghost jobs](research/ghost-jobs.md)
- [Job-site data sources](research/job-site-data-sources.md)
- [Pay equity](research/pay-equity.md)
- [Salary negotiation](research/salary-negotiation.md)
- [Semantic resume-job matching](research/semantic-resume-job-matching.md)

## Developer Docs

| Need | Doc |
| --- | --- |
| Local development setup | [Getting Started](developer/GETTING_STARTED.md) |
| Architecture map | [Architecture](developer/ARCHITECTURE.md) |
| Core module inventory | [Core Architecture](developer/ARCHITECTURE_CORE.md) |
| Stored configuration example | [config.example.json](examples/config.example.json) |
| Test strategy and commands | [Testing](developer/TESTING.md) |
| Contributing workflow | [Contributing](developer/CONTRIBUTING.md) |
| Release process | [Releasing](developer/RELEASING.md) |
| Linux packages | [Linux Build Guide](developer/LINUX_BUILD.md) |
| macOS notes | [macOS Development](developer/MACOS_DEVELOPMENT.md) |
| SQLite and SQLx setup | [SQLite Configuration](developer/sqlite-configuration.md) |
| Optional local semantic matching | [Local Semantic Matching](developer/LOCAL_SEMANTIC_MATCHING.md) |
| Error-handling patterns | [Error Handling](developer/ERROR_HANDLING.md) |
| Mutation testing | [Mutation Testing](developer/MUTATION_TESTING.md) |
| Browser E2E tests | [Tests README](../tests/README.md) |

## Harness And Plans

| Need | Doc |
| --- | --- |
| Agent operating model | [Harness README](harness/README.md) |
| Required checks | [Verification Matrix](harness/verification-matrix.md) |
| Change contracts | [Change Contract](harness/change-contract.md) |
| Exec plan format | [Exec Plans](exec-plans.md) |
| V3 major-release planning | [V3 Planning](plans/v3/README.md) |
| Active plans | [Active Plans](plans/README.md#current-active-plans) |
| Completed plans | [Completed Plans](plans/README.md#archived-plans) |
| Technical debt tracker | [Tech Debt Tracker](plans/tech-debt-tracker.md) |

## Maintenance Rules

- Update docs when behavior, setup, commands, architecture, security posture,
  privacy posture, release flow, or user-facing copy changes.
- Keep docs in one maintained home instead of duplicating release logs or old
  implementation notes.
- Prefer plain user language for user-facing docs and clear implementation
  contracts for developer docs.
- Run focused checks for the changed surface. Docs changes usually need:

```bash
npm run lint:docs
npm run lint:bloat
git diff --check
```

Broader changes may also require frontend tests, Rust checks, build checks, or
E2E tests from the [Verification Matrix](harness/verification-matrix.md).
