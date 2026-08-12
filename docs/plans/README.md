# Plans

This folder contains durable plans for release, feature, refactor, security, and
harness work.

## Structure

- `active/` - Work in progress.
- `archive/` - Preserved historical plan state that should not slow restarts.
- `completed/` - Finished plans with outcomes.
- `index.json` - Machine-readable active workstream index used by the harness
  score and session snapshot.
- `templates/` - Reusable plan and change-contract templates.
- `tech-debt-tracker.md` - Durable debt and harness drift.
- `v3/` - Major-release planning, moonshots, refactor options, and research
  agenda.

See [Exec Plans](../exec-plans.md) for required format.

## Current Active Plans

| Workstream | Document |
| ---------- | -------- |
| V3 master planning | [V3 master execution plan](v3/master-exec-plan.md) |

## Major-Release Planning

| Horizon | Document |
| ------- | -------- |
| V3 major line | [V3 master execution plan](v3/master-exec-plan.md) |
| V3 strategy and research | [V3 planning package](v3/README.md) |

## Plan Requirements

Each broad plan follows [the exec-plan template](templates/exec-plan-template.md):

1. **Problem** - What needs to change and why.
2. **Scope** - What is in and out.
3. **Success Criteria** - Observable result, user-ease result, and verification
   result.
4. **Audience And Ease** - Primary user, assumed technical knowledge, broad
   job-seeker fit, and support or recovery path.
5. **Risks** - Risks and mitigations.
6. **Orchestration** - Whether the work stays local or uses coordinated agents.
7. **Milestones** - Small checkable steps.
8. **Verification** - Exact commands and evidence required before completion.
9. **Progress, Discoveries, Decisions, Outcomes, and Handoff** - Durable state
   for review and restart.

## Archived Plans

| Version | Status | Document |
| ------- | ------ | -------- |
| Repository | Complete | [Post-cleanup corrections](completed/repository-post-cleanup-corrections.md) |
| Repository | Complete | [Post-cleanup review](completed/repository-post-cleanup-review.md) |
| Repository | Complete | [Residual cleanup](completed/repository-residual-cleanup.md) |
| Frontend | Complete | [DRY remediation](completed/frontend-dry-remediation.md) |
| Rust crates | Complete | [DRY remediation](completed/crates-dry-remediation.md) |
| Repository | Complete | [Full refactor](completed/full-repository-refactor.md), [ownership blueprint](completed/repository-refactor-blueprint.md) |
| v2.9.5 | Release complete | [Full repository refactor and readiness](completed/repository-architecture-reorganization.md) |
| Feedback workflow | Complete on main | [Beta feedback system](completed/beta-feedback-system.md) |
| v2.9.1 | Complete | [Maintenance and repo cleanup](completed/v2.9.1-maintenance-and-repo-cleanup.md) |
| v2.9.0 | Complete | [Completion and full-feature roadmap](completed/v2.9.0-completion-and-full-feature-roadmap.md) |
| v2.6.x | Complete on main | [Undo and redo wiring](completed/undo-redo-wiring.md) |
| v2.6.0 | Complete | [v2.6.0 UX improvements](completed/v2.6.0-ux-improvements.md) |
| Superseded active history | Archived | [Guided intake](archive/guided-job-search-intake-superseded-2026-06-04.md) |
| v2.5.0 | Complete | No formal plan. |
| v2.0.0 | Complete | No formal plan. |
| Release pipeline optimization | Deferred on 2026-07-13 | [Deferred plan](archive/release-pipeline-audit-and-optimization-deferred-2026-07-13.md) |
| v1.0.0 | Complete | No formal plan. |

Starting with the harness alignment work, broad changes should use
`docs/plans/active/` and move completed plans to `docs/plans/completed/`.
Historical progress rows can move to `docs/plans/archive/` when they are useful
for provenance but too noisy for active restart context.
