# Editions And Packaging

V3 should not assume every job seeker has a high-end computer, stable
broadband, or room for multi-gigabyte model caches. JobSentinel needs a package
strategy that keeps the strongest experience available while making the easy,
reliable path work on modest hardware.

V3 should also be the easiest JobSentinel release to install and use. The
package strategy should reduce decisions, explain defaults, and get a user to a
useful local workflow quickly.

## Product Rule

Do not use deficit labels for this path. The user should never feel blamed for
their hardware. Use names like:

- JobSentinel Essentials
- Lightweight package
- Core local package
- Small install

The package should feel intentional, supported, and capable.

## Editions

| Edition | Target user | Default behavior |
| --- | --- | --- |
| Essentials | Older laptops, limited storage, limited memory, slower networks, shared family machines | Small install, deterministic local scoring, no large model download, public sources, Browser Import, resume checks, tracker, pay protection, and safe support. |
| Standard | Most users with current Windows, macOS, or Linux hardware | Full local workflows, optional governed model downloads, browser companion, source packs, external AI gateway, and diagnostics. |
| Pro Local | Users who want strongest local intelligence and can support larger model caches | Qwen3 embedding, Qwen3 reranking, eval tools, model doctor, larger indexes, and optional local LLM integrations. |
| Developer | Contributors and power users | Full diagnostics, source fixture lab, ML eval runner, CLI, pack validation, and debug tooling. |
| Portable | Shared computers, workforce centers, removable drives, and no-admin setups | Self-contained data location, explicit lock and cleanup, no account requirement, Essentials defaults, and clear platform limits. |

These are packaging and defaults, not locked feature classes. A user should be
able to start with Essentials and later enable stronger local ML or more source
packs.

## Essentials Package

Essentials should include:

- setup wizard
- saved searches
- official and public source checks that do not require heavyweight rendering
- Browser Import or browser companion fallback
- manual and pasted job import
- job dashboard
- ghost-job and posting-risk heuristics
- pay floor and pay-protection checks
- resume parser and readability checks
- deterministic resume and job matching fallback
- application tracker
- reminders
- safe support reports
- external AI gateway setup, disabled by default
- optional upgrade prompt for local model downloads
- starter regional pack support for UK, EU, and India manifests when installed
- optional portable-mode cleanup controls

Essentials should avoid by default:

- bundled multi-gigabyte models
- required GPU or accelerator paths
- heavy browser automation dependencies where a user-visible fallback exists
- background jobs that can make older machines feel stuck
- advanced settings unless expert mode is enabled

Gate 4 freezes the Essentials composition as the `Lighter` profile with
deterministic matching, public sources, and safe support. It has no automatic
or bundled model download, and region packs remain separately installed
options. This approves the component boundary, not a numeric footprint or
performance claim. A controlled zero-swap Linux amd64 guest now proves the
model-free installed journey under an enforced 8 GiB limit, and a revision-bound
macOS arm64 low-pressure baseline exists on a 64 GiB host. Approved numeric
thresholds and the Windows 11, macOS 26, and Linux platform package matrix remain
blocked.

## Runtime Profiles

V3 should expose a simple runtime profile without making users tune model
parameters:

| Profile | User-facing copy | Technical behavior |
| --- | --- | --- |
| Balanced | Best default for most computers | Uses local deterministic scoring and offers governed model setup when supported. |
| Lighter | Uses less memory and storage | Smaller batches, fewer background tasks, deterministic matching, no automatic model downloads. |
| Stronger local matching | Improves match explanations on capable machines | Enables Qwen3 embedding and reranking after model cache verification. |

The UI should say what changes in plain language:

- "Uses less storage."
- "Runs fewer checks at once."
- "Keeps matching local but simpler."
- "Download stronger local matching when you are ready."

## Packaging Requirements

- Build artifacts should identify edition, version, architecture, and platform.
- Essentials packages should be available for Windows 11+, macOS, and Linux.
- Checksums, SBOMs, and attestations should exist for every edition.
- V3 package metadata should declare compatibility line, edition, architecture,
  and rollback support.
- The main README should explain which package to download without technical
  jargon.
- The download page and README should recommend an edition in plain language.
- The app should detect missing optional components and offer clear setup, not
  fail with developer errors.
- Model downloads should be explicit, resumable, verified, and removable.
- Source packs and model packs should be independently removable to reclaim
  storage.
- Regional packs should be independently installable, removable, and visible in
  the edition chooser.
- Portable packages should keep data, models, packs, and logs in predictable
  user-visible locations and explain macOS, Windows, and Linux limitations.

## Upgrade Path

Essentials users should be able to upgrade in place:

1. Open Settings.
2. See what stronger local matching adds.
3. Review model size, license, and local-only behavior.
4. Download verified model files.
5. Run model doctor.
6. Rebuild stale vectors.
7. Fall back cleanly if the machine cannot support it.

No user should need to reinstall the whole app just to try stronger matching.

## Easy Install Requirements

- One public download path per platform with clear labels.
- Essentials recommended when the app cannot confidently assume stronger local
  matching will run comfortably.
- No hosted account required.
- No external AI provider required.
- No model download required before the first useful workflow.
- First-run setup creates at least one useful local search path.
- If permissions, vault, browser companion, source checks, or model downloads
  fail, the app gives plain recovery actions.
- Users can uninstall, update, roll back, or repair without reading developer
  docs.
- Shared-computer and portable users can lock, export, delete, or clean up local
  state from one visible place.

## Verification

Essentials needs its own release checks:

- fresh install on a modest Windows 11 machine profile
- fresh install on a modest macOS machine profile
- app launch without model files
- setup wizard
- first useful workflow completed without external accounts
- source checks
- Browser Import fallback
- resume import
- deterministic match
- tracker state changes
- pay protection
- safe support report
- model upgrade prompt
- model-free external AI disabled path
- storage cleanup
- regional pack install, disable, and remove
- portable/shared-computer cleanup
- v3-compatible rollback path

The key success metric is not raw benchmark speed. It is whether the app feels
responsive and useful on hardware that cannot comfortably run large local
models.
