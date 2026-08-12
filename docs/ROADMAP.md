<!-- Summarizes JobSentinel's development direction and routes implementation work to canonical plans. -->

# JobSentinel Developer Roadmap

This file is the developer-facing companion to the public
[root roadmap](../ROADMAP.md). Keep the root roadmap focused on the six product
pillars. Keep this file focused on implementation state, active plans,
verification, and contribution routing.

Use `package.json` for the current release package version.

Do not use this file as a changelog. Current release detail belongs in
[CHANGELOG.md](../CHANGELOG.md). Historical release snapshots belong in the
[release notes index](releases/README.md) and completed plans.

## Current product priorities

The public roadmap organizes work around six design pillars:

1. Ghost-job and stale-posting detection.
2. Pay equity and salary-floor protection.
3. Long-term unemployment support.
4. Bias-aware application strategy.
5. Protective, non-cheerleader UX.
6. Privacy-first local control.

Active implementation planning is indexed in [docs/plans](plans/README.md).
Major-release exploration for the next generation of JobSentinel lives in
[v3 planning](plans/v3/README.md).
The published `v2.9.5` release retains its `2.9.5` source metadata. Development
source metadata is now the unreleased `3.0.0` line.
Every non-trivial change should update the relevant feature doc, active plan,
or tech-debt item before it is committed.

## Current implementation snapshot

Use fresh command output when exact counts matter. This snapshot records the
current maintained surface at the time of the latest docs sweep.

| Area | Current state |
| ---- | ------------- |
| Desktop app | Tauri 2, React 19, TypeScript, Rust 2021 |
| Storage | Local SQLite with SQLx offline mode |
| Source monitoring | 12 scheduled source adapters plus user-opened search links |
| Source status | 15 source-status checks with plain help output |
| Backend surface | 254 registered Tauri commands |
| Privacy posture | Local-first, no telemetry, external channels user-configured |
| External AI posture | Optional, disabled by default, routed through `src/shared/externalAi/` |
| Safe support reports | Reports can be copied or saved locally, reviewed, and shared only when the user chooses help |

## Working product surfaces

| Surface | Maintained docs |
| ------- | --------------- |
| Setup and search intent | [Quick start](user/QUICK_START.md), [current status](harness/current-status.md) |
| Job monitoring and source status | [Source checks](features/job-sources.md), [source status](features/job-source-status.md) |
| Ghost and stale-posting review | [Ghost detection](features/ghost-detection.md), [current status](harness/current-status.md) |
| Salary and pay protection | [Salary support](features/pay-protection.md), [pay-equity research](research/pay-equity.md) |
| Resume builder and fit review | [Resume builder](features/resume-builder.md), [resume match](features/resume-matcher.md) |
| Application tracking | [Application tracking](features/application-tracking.md) |
| Application Assist | [Application Assist](features/application-assist.md) |
| Saved data and recovery | [User data management](features/user-data-management.md), [privacy policy](../PRIVACY.md) |
| Optional external AI | [AI gateway](security/privacy-first-ai-gateway.md), [Responsible AI](../RESPONSIBLE_AI.md) |

## Source policy

JobSentinel favors official sources, public application-platform postings, public feeds, and
user-opened browser links.

- Source checks must wait within each source's limits and show plain source
  status.
- Some sites only allow user-opened searches. Use search links for those sites.
- No source check may get around access controls, solve CAPTCHAs, reuse session
  cookies, rotate proxies to evade controls, or ignore explicit source blocks.
- New source work needs a feature doc update, fixture or parser coverage, and
  source-boundary review.

## Privacy and AI boundary

Rule 0: user privacy and security are non-negotiable.

- Core workflows must work locally.
- External AI is optional and disabled by default.
- All external AI requests must go through the AI gateway.
- Sensitive data needs minimization, preview, redaction, cancellation, approval,
  and local metadata logging before any provider call.
- Research and grant evaluation use public postings and synthetic candidate
  profiles by default.

See [PRIVACY.md](../PRIVACY.md),
[RESPONSIBLE_AI.md](../RESPONSIBLE_AI.md), and
[privacy-first AI gateway](security/privacy-first-ai-gateway.md).

## Current and completed plans

Current active plan docs are part of the repo goal:

- [Current status](harness/current-status.md)
- [V3 workstream selection](plans/v3/README.md)
- [Completed repository post-cleanup corrections](plans/completed/repository-post-cleanup-corrections.md)
- [Completed repository post-cleanup review](plans/completed/repository-post-cleanup-review.md)
- [Completed repository residual cleanup](plans/completed/repository-residual-cleanup.md)
- [Completed full repository refactor](plans/completed/full-repository-refactor.md)
- [Completed full repository refactor and v2.9.5 readiness](plans/completed/repository-architecture-reorganization.md)

Superseded long-running plan history is archived under
[Archived Plans](plans/README.md#archived-plans).

Move a plan to the completed-plan area listed in
[docs/plans](plans/README.md) only after the plan's success criteria and
verification evidence are current.

## Technical debt tracking

Use [tech-debt-tracker.md](plans/tech-debt-tracker.md) for current technical
debt. Do not bury new debt in old version sections.

Current recurring themes:

- Keep stale docs and release claims out of maintained docs.
- Keep source-boundary and privacy checks current.
- Keep Playwright and E2E suites fast enough for routine local use.
- Keep broad-audience examples visible across setup, docs, mocks, and tests.
- Keep frontend and backend error paths free of secrets, local paths, raw
  resumes, private notes, and unrelated application history.

## Optional local ML

The `embedded-ml` Cargo feature is optional and local-only. It is not required
for core workflows. When enabled by a developer build, it can use local model
files for semantic matching experiments and should fall back to deterministic
matching when unavailable. Settings includes **Local Match Check** so users can
see the current local matching mode without sending resume or job text anywhere.

Do not confuse embedded local ML with external AI. External AI remains disabled
by default and gateway-bound.

See [Embedded ML feature notes](developer/LOCAL_SEMANTIC_MATCHING.md) for
developer details.

## Verification map

Use the smallest relevant check first, then broaden when risk increases.

```bash
npm run harness:check
npm run lint:bloat
npm run lint:tests
npm run lint:docs
npm run lint
npm run test:run
npm run test:e2e:smoke
npm run build
npm run tauri:build:macos # macOS package check, on macOS only
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

Full release verification also includes the broader Playwright matrix,
package-build checks, and current dependency/security scans.

## Update rules

- Keep exact version, test-count, release, and command-count claims current or
  remove them.
- Prefer links to maintained docs over duplicated setup or API detail.
- Add new user-facing behavior to a feature doc.
- Add new broad work to an active plan or completed plan.
- Add recurring drift or known debt to the tech-debt tracker.
- Preserve Rule 0 and the local-first architecture in every roadmap edit.
