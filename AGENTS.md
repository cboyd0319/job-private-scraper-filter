<!-- Defines the canonical repository-wide operating contract for JobSentinel contributors and agents. -->

# JobSentinel Agent Contract

JobSentinel is a privacy-first desktop job-search application for technical and
non-technical job seekers. The frontend is React 19, TypeScript, Vite, and
Tailwind CSS. The desktop and Rust owner layers use Tauri 2, Rust 2021, Tokio,
SQLite, and SQLx offline mode. Runtime versions are owned by `.nvmrc`,
`rust-toolchain.toml`, `package.json`, and the lockfiles.

## Startup Workflow

Before implementation:

1. Confirm the repository root with `git rev-parse --show-toplevel`.
2. Read `docs/harness/current-status.md`, then `scripts/harness/state/feature-list.json`.
3. Review `git log -5 --oneline` and `git status --short --branch`.
4. Run `./init.sh` on macOS or Linux, or `pwsh -File ./init.ps1` on Windows.
5. Repair a failed baseline before adding scope.
6. Work only on the single `active` feature in `scripts/harness/state/feature-list.json`.

First run and every dependency refresh use the same init command. It performs
locked project dependency synchronization, read-only environment diagnosis,
harness validation, and the baseline application smoke test. It does not install
global tools, enable hooks, start services, or read secrets. Start the desktop app
with `npm run tauri:dev`.

## Non-Negotiable Boundaries

- User data stays local unless the user explicitly configures an external
  channel. External AI is optional, disabled by default, and routed through the
  privacy-first AI gateway.
- Preserve credential safety, explicit user review, URL validation, scraper rate
  limits, accessibility, data-loss handling, and Windows 11, macOS 26+, and Linux
  behavior.
- Preserve unrelated user changes. Never commit, publish, release, change cloud
  state, or mutate user-level configuration without explicit authority.
- Use repo-relative paths and structured APIs. Do not commit secrets or local
  home paths.
- Every maintained hand-authored file that supports comments starts with a one- or two-line native description
  of its exact responsibility, after required shebang, front matter, or format directives.
- `scripts/harness/contracts/repository-structure.json` covers every maintained text file. New or
  changed files must stay within their scope; exceptions need an owner, measured
  baseline, reason, and removal trigger.

## Canonical Owners

- Current state: `docs/harness/current-status.md`
- Work selection and pass transitions: `scripts/harness/state/feature-list.json`
- Initialization: `init.sh`, `init.ps1`, and `scripts/harness/init.mjs`
- Harness ownership and retired paths: `scripts/harness/contracts/harness.json`
- Verification routing: `scripts/harness/plan.mjs`
- Source roots, units, and file-size policy: `scripts/harness/contracts/repository-structure.json`
- Rust graph, technology owners, and retired architecture paths:
  `scripts/harness/contracts/architecture.json`
- Detailed operating model: `docs/harness/README.md`
- Human architecture projection: `docs/architecture/repository.md`
- Product behavior: `docs/features/` and `docs/harness/product-sense.md`
- Reliability: `docs/harness/reliability.md`
- Security: `docs/security/README.md`
- Workflow: `docs/harness/change-contract.md` and
  `docs/harness/verification-matrix.md`
- Tests and commands: `docs/developer/TESTING.md`
- Change contracts: `docs/harness/change-contract.md`
- UI and copy: `docs/design/design-system.md`, `docs/design/design-spec.md`, and
  `docs/style-guide/README.md`

Closest feature documentation under `docs/features/` owns user-visible behavior.
Detailed plans under `docs/plans/` may expand the active feature but cannot
replace canonical state.

## Verification

- Plan the diff: `npm run harness:plan -- --since <valid-ref>`
- Harness and state: `npm run harness:check`
- File sizes: `npm run lint:file-size`
- Frontend lint and type check: `npm run lint` and `npm run typecheck`
- Frontend tests: `npm run test:run`
- Baseline smoke: `npm run test:smoke`
- Runtime journey: `npm run test:e2e:smoke`
- Script contracts: `npm run test:scripts`
- Architecture: `npm run lint:architecture`
- Rust: `cargo fmt --all -- --check`, `cargo clippy --workspace -- -D warnings`,
  and `cargo test --workspace`
- Full local gate: `npm run verify:full`

Use the smallest lane that can disprove the claim. Shared contracts, releases,
broad refactors, and uncertain routing require the full lane. Record commands,
exit status, platform, relevant result, and caveat in evidence outside startup
state.

## Definition Of Done

- Acceptance behavior is proven at every applicable verification level.
- `docs/harness/current-status.md` and `scripts/harness/state/feature-list.json` agree with fresh evidence. Only
  verification can move a feature to irreversible `passing`.
- Blocked work names the blocker and next trigger. Skipped required checks remain
  gaps, not passes.
- Temporary artifacts are removed, the working tree is understood, and the
  standard init and start paths remain runnable for a fresh session.
