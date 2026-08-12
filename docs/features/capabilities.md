<!-- Maps JobSentinel's current user-facing capabilities, implementation boundaries, and release status. -->

# Features And Capabilities

Last reviewed: 2026-07-22.

This page is the single maintained map of what JobSentinel can do in the
current source line. Development metadata targets the unreleased `3.0.0` line.
The published `v2.9.5` release remains the latest public product boundary.
Detailed behavior stays in the linked feature, security, architecture,
research, and harness docs.

JobSentinel is a local-first job-search workspace. It helps users find,
review, compare, track, and prepare for jobs while keeping sensitive
job-search data under user control. Core workflows work without a hosted
account, telemetry, cloud sync, or external AI provider.

## Feature Highlights

| Highlight | Why it matters |
| --- | --- |
| Ghost-job review | Helps users avoid wasting energy on stale, reposted, weak-source, scam-like, or low-confidence roles. |
| Restricted-source Workbench | Lets users work from sign-in-backed job sites through visible, user-driven, local review flows without storing session material. |
| Resume Match and Builder | Connects job requirements to concrete resume evidence, readability checks, and truthful draft improvements. |
| Application Assist | Keeps repeated profile details, screening questions, attachments, and final submission under user review. |
| Pay Protection | Keeps salary floors, listed ranges, written offers, verbal numbers, total compensation, commute, relocation, and deadline pressure visible. |
| Local Qwen3 matching | Uses governed Qwen3 embeddings plus bounded Qwen3 reranking in embedded-ML builds, then blends exact skill, BM25, blocker, seniority, and evidence signals. |
| Agent Skills | Packages repeatable job-search, resume, outreach, interview, and offer workflows for agent-capable tools. |
| Optional external AI gateway | Supports provider choice and request approval without making cloud AI required for core workflows. |

## Under-The-Hood Highlights

| Mechanic | Current behavior |
| --- | --- |
| Source taxonomy and routing | Public feeds, employer career pages, ATS families, regional boards, public boards, restricted sources, search links, Browser Import, and manual paths are treated as distinct source classes. |
| Restricted-source Workbench | Sign-in-backed sources stay user-started and visible, with native review, local ledger actions, user-entered details, and no page capture or stored session material. |
| Evidence-bounded resume matching | Resume help connects requirements to concrete local resume evidence, readability signals, hard gaps, and truthful draft suggestions. |
| Local model governance | `crates/jobsentinel-local-ai/models.lock.toml` controls model identity, revision, hashes, size, license, backend compatibility, instructions, thresholds, and stale-vector rules. |
| Hybrid ranking | Dense retrieval, BM25, exact skill taxonomy hits, required coverage, seniority, blocker caps, reranker scores, and provenance are scored separately. |
| Privacy-first AI gateway | Provider setup supports OpenAI, Anthropic, Google Gemini, GitHub Copilot, and custom HTTPS providers without making external AI required. |
| Secure local operations | Saved secrets use the local vault, safe support reports stay review-first, and release assets carry checksums, SBOMs, and attestations. |
| Agent Skills packaging | Job search, resume, outreach, interview, and offer/pay skills ship as validated ZIP and tar.gz archives with checksums. |

## Status

| Item | Current state |
| ---- | ------------- |
| Source status | `3.0.0` is the current development source version and is unreleased |
| Public release | `v2.9.5` is the latest verified GitHub release with Windows, macOS, Linux, and Agent Skills assets |
| Primary platforms | Windows 11+, macOS, and Linux |
| Default data model | Local SQLite and local settings |
| Telemetry | None |
| External AI | Optional, disabled by default, routed through the privacy-first AI gateway |
| Restricted job sources | User-gated, local, review-first paths only |
| Final application submission | Always user-controlled outside JobSentinel |

Use [current status](../harness/current-status.md) for current release gates and
[the README](../../README.md) for public download status.

## Capability Map

| Area | What JobSentinel helps with | Default data boundary | Detailed doc |
| ---- | --------------------------- | --------------------- | ------------ |
| Setup and saved searches | Start from plain job goals, locations, pay floors, work modes, and reviewed source choices | Local only, sensitive | [Quick Start](../user/QUICK_START.md), [local data](user-data-management.md) |
| Job discovery and source checks | Check approved public job sources, official feeds, and employer career pages within source limits | Local storage; source requests only after user configuration or opt-in | [Job Sources](job-sources.md), [Job Source Status](job-source-status.md) |
| Company careers discovery | Classify employer career pages and route them to native source support, Browser Import, pasted links, or manual entry | Local classification, then user-approved source action | [Job Sources](job-sources.md) |
| Restricted-source Workbench | Help users work from user-opened restricted sites with acknowledgement, visible-page import, local ledger actions, and no stored auth material | Local only after user action; no cookies, tokens, browser storage, or hidden page state | [Job Sources](job-sources.md), [Browser Import Button](browser-import.md) |
| Browser Import | Send visible jobs from a user-opened browser page into a local review queue | Local one-use import detail; reviewed records stay local | [Browser Import Button](browser-import.md) |
| Dashboard and job cards | Show fit, source, pay, freshness, posting-risk, reminders, and next actions | Local job records and public posting details | [Fit Review](smart-scoring.md), [Ghost Detection](ghost-detection.md), [Pay Protection](pay-protection.md) |
| Fit review | Sort and explain jobs using titles, work words, salary, location, company preferences, freshness, and optionally reviewed resume skills | Local only | [Fit Review](smart-scoring.md), [Remote And Work-Mode Matching](remote-preference-scoring.md), [Synonym Matching](synonym-matching.md) |
| Ghost-job and posting-risk review | Flag stale, reposted, weak-source, unclear, scam-like, and low-confidence postings for user review | Local heuristic review from public posting and local source history | [Ghost Detection](ghost-detection.md) |
| Hiring trends | Summarize local job pool patterns by role, skill, company, location, remote mode, listed pay, and source activity | Local aggregates from local jobs | [Hiring Trends](hiring-trends.md) |
| Pay protection | Compare listed pay, salary floors, pay-transparency cues, written offers, total compensation, commute, relocation, and deadline pressure | Salary floor and offer notes stay local | [Pay Protection](pay-protection.md) |
| Resume library | Add and manage resume versions without exposing full local paths to app screens | Local only, sensitive | [Resume Match](resume-matcher.md), [Resume Builder](resume-builder.md) |
| Resume Match | Parse readable resume text, review application readability, compare a resume with a job post, show gaps, and support truthful tailoring | Local by default; no external AI required | [Resume Match](resume-matcher.md) |
| Resume Builder | Build, preview, and export readable resumes as PDF, DOCX, and JSON Resume | Local only | [Resume Builder](resume-builder.md) |
| JSON Resume import and export | Use portable structured resume data from compatible resume tools | Local only | [Resume Data Import](json-resume-import.md), [Resume Builder](resume-builder.md) |
| Application Assist | Prepare profile details in a visible browser, review hard screening questions, and keep final submission manual | Local only; selected resume file stays local and manual to attach | [Application Assist](application-assist.md) |
| Application tracking | Track opportunities, statuses, notes, contacts, follow-ups, interviews, offers, and no-response review | Local only, sensitive | [Application Tracking](application-tracking.md) |
| Notifications | Send optional desktop, email, Slack, Discord, Teams, or Telegram alerts after user configuration | Desktop local; external channels receive only alert details the user enabled | [Notifications](notifications.md), [Saved Secrets](saved-secrets.md) |
| Safe support and backups | Create sanitized support reports, local settings backups, and recovery guidance | Local by default; user reviews before sharing | [Local Job-Search Data](user-data-management.md), [Privacy](../../PRIVACY.md) |
| Saved secrets | Store alert passwords, connection links, and source access codes in a local encrypted vault | Local vault protected by the operating system password store or passphrase mode | [Saved Secrets](saved-secrets.md), [Keyring](../security/KEYRING.md) |
| External AI gateway | Let users configure OpenAI, Anthropic, Google Gemini, GitHub Copilot, or custom HTTPS providers for approved optional actions | Disabled by default; preview, edit, cancel, approval, redaction, and metadata-only logs | [Privacy-first AI Gateway](../security/privacy-first-ai-gateway.md), [Responsible AI](../../RESPONSIBLE_AI.md) |
| Local semantic matching | Optional Qwen3-Embedding-0.6B retrieval, Qwen3-Reranker-0.6B bounded reranking, exact skill/BM25 scoring, blocker caps, evidence labels, governed model manifests, and deterministic fallback | Local only when enabled; no provider required | [Local Semantic Matching](../developer/LOCAL_SEMANTIC_MATCHING.md) |
| Agent Skills | Downloadable skill packages for job search planning, posting-risk review, resume tailoring, application review, tracking, outreach, interview prep, and offer/pay review | Skill files are static local content | [Agent Skills](../../skills/README.md) |

## Local Match Intelligence

Embedded-ML builds can run the strongest matching path entirely on the user's
machine. JobSentinel uses a JobSentinel-owned model layer instead of letting a
convenience downloader own model identity or scoring semantics.

| Mechanic | Release behavior |
| --- | --- |
| Governed model lock | `crates/jobsentinel-local-ai/models.lock.toml` pins Qwen3 embedding, Qwen3 reranker, MiniLM fallback, revisions, hashes, sizes, licenses, backends, instructions, thresholds, and stale-vector rules. |
| Dense retrieval | Qwen3-Embedding-0.6B retrieves resume and job passages that are meaningfully related. |
| Lexical grounding | BM25 and exact skill taxonomy matches keep tools, credentials, role terms, and hard requirements concrete. |
| Bounded reranking | Qwen3-Reranker-0.6B reranks top candidates so direct evidence can beat keyword-only near misses. |
| Evidence and blockers | The hybrid scorer records dense, BM25, exact skill, required-coverage, seniority, reranker, blocker, and provenance signals. |
| Privacy fallback | If governed model files are unavailable, JobSentinel falls back to deterministic local matching and marks diagnostics accordingly. |

## Job Sources And Access

JobSentinel separates low-friction public sources from sources that need user
acknowledgement or sign-in-session boundaries.

| Source class | Examples | Current behavior |
| ------------ | -------- | ---------------- |
| Official hiring APIs and feeds | Greenhouse, Lever, RemoteOK, USAJobs, public employer feeds | Normal opt-in source checks with rate limits, safe errors, and local storage |
| Reviewed public job feed | WeWorkRemotely | Governed opt-in checks with a reviewed endpoint, pacing, fixtures, and safe errors |
| Retired restricted scheduled adapters | Built In, Dice HTML, SimplyHired, Glassdoor | Disabled after provider policy review; stale config cannot restore transport |
| Employer career pages | Fivetran, SpaceX, Google, Microsoft, Amazon, GitHub, OpenAI, Anthropic, and other company pages | Classify the platform first, then use native support, Browser Import, pasted link import, or manual entry |
| Reviewed ATS families | Greenhouse, Lever, Ashby, Workable, SmartRecruiters, Workday, iCIMS/Jibe, Breezy, JazzHR, Bullhorn, Eightfold, Jobvite, Teamtailor, Recruitee, Taleo, SAP SuccessFactors, Oracle Recruiting, Phenom, Radancy/TalentBrew | Taxonomy-backed discovery, with native scheduled support only after source-specific review and fixtures |
| Native source-adapter contracts from API research | Workday Candidate Experience listing JSON, Phenom widget refineSearch JSON, Radancy/TalentBrew static HTML | Parser and canonical-record contracts exist; live scheduling still needs tenant-specific policy, robots, rate-limit, endpoint-stability, and parser checks |
| Restricted authenticated sources | LinkedIn and similar sign-in-backed sources | User starts the session, sees the warning before sign-in, uses the site directly, and imports or logs only user-selected visible information |
| Search-link destinations | LinkedIn, USAJobs, Google Jobs Search, regional boards, company search pages | Open in the user's browser, then use a policy-permitted import, pasted details, or manual entry; LinkedIn page capture is blocked |

Restricted-source support is intentionally explicit. JobSentinel should make
the secure local path easy, but it must not store login details, session
cookies, browser storage, authorization headers, hidden page state, or network
traffic from restricted sources.

## Everyday User Flows

### New Search Setup

1. Choose the kind of work, location, work mode, and optional salary floor.
2. Review suggested titles, search words, and optional source checks.
3. Save the search locally.
4. Return to the dashboard to review jobs and source status.

### Daily Job Review

1. Open the dashboard.
2. Review fit, source, pay, posting-risk, and freshness cues.
3. Save, hide, import, or track jobs.
4. Use Browser Import or pasted links for jobs found outside scheduled sources.
5. Open only the roles worth more attention for resume or application work.

### Resume And Application Prep

1. Add or build a resume locally.
2. Check readable text and application-readability issues.
3. Review the resume against a job posting.
4. Use evidence-backed suggestions only when they are true.
5. Use Application Assist to prepare repeated profile details.
6. Review the outside application and submit it yourself.

### Tracking And Follow-Up

1. Move opportunities through the application board.
2. Add notes, contact details, reminders, interviews, and offer information.
3. Use quiet-period and no-response review to stop wasting time on stale roles.
4. Replan sources and search lanes from tracker evidence.

### Offer And Pay Review

1. Compare listed pay with salary floors and written offer terms.
2. Separate verbal numbers from written offer details.
3. Review total compensation, commute, relocation, deadline pressure, and role level.
4. Draft local counter or decline notes only from confirmed facts.

## Privacy Labels

The machine-readable privacy label index is
[`docs/harness/feature-privacy-labels.json`](../harness/feature-privacy-labels.json).
Feature changes that add data flows, external calls, sensitive payloads, or
new provider behavior must update that index and pass `npm run harness:check`.

| Label | Meaning |
| ----- | ------- |
| Local only | No data leaves the device for that workflow. |
| Sensitive | The workflow may involve resumes, salary floors, private notes, application history, career goals, or location preferences. |
| Public-data only | The workflow uses public job-posting content or public metadata only. |
| External AI optional | The user may choose an external provider, but the local path remains available. |
| External AI required | The feature needs a provider call and must require explicit opt-in. No shipped user-facing feature should use this label without a new reviewed contract. |

## What External AI Can Do

External AI is optional and disabled by default. Users can configure OpenAI,
Anthropic, Google Gemini, GitHub Copilot, or a custom HTTPS provider. Provider
keys stay in the local secure vault. Users can configure more than one provider
and set preference order and model names.

The shipped provider-backed UI action remains public job-posting summary from a
job card. It uses unchanged stored public posting fields only, after preview,
optional field removal, cancel, redaction, backend validation, and trusted
native confirmation. The exact request needs an expiring, single-use backend
approval, a durable metadata-only lifecycle, and a bound privacy receipt before
provider transport. Settings shows a bounded projection of that durable
lifecycle without payloads, responses, credentials, or raw errors.

No private-data external AI feature ships in the v3 execution line. Any
future resume, salary-floor, private-note, or application-history send needs
feature-specific payload minimization, privacy labels, backend validation,
preview, edit, cancel, approval, redaction, and tests before it can be enabled.

## What JobSentinel Does Not Do

| Boundary | Reason |
| -------- | ------ |
| Submit final applications | The user must review and submit applications themselves. |
| Fabricate resume claims | Resume help must stay truthful and evidence-backed. |
| Hide keywords, add invisible text, or prompt-inject resumes | These tactics are deceptive and unsafe. |
| Guarantee employer response or hiring outcome | JobSentinel provides candidate-side decision support, not employer predictions. |
| Treat pay guidance as legal advice | Pay cues help users ask better questions and review written ranges. |
| Run hidden restricted-source monitoring | Restricted sources remain user-directed, and provider automation prohibitions cannot be overridden locally. |
| Store restricted-site cookies, tokens, browser storage, or auth headers | Credential and session capture is outside the product boundary. |
| Solve human checks or bypass platform controls | Source boundaries and user privacy remain non-negotiable. |
| Upload the local job database by default | Core workflows are local-first. |
| Send private resume, salary, notes, or application history to external AI by default | Sensitive sends require a future reviewed, optional, previewed, approved path. |

## Maintenance Rules

- Update this page when a user-facing capability, data boundary, source class,
  AI provider path, import flow, notification channel, resume workflow, or
  application workflow changes.
- Keep detailed implementation contracts in their feature docs and link them
  here.
- Keep privacy labels synchronized with
  `docs/harness/feature-privacy-labels.json`.
- Keep current release status synchronized with
  `docs/harness/current-status.md` and `scripts/harness/state/feature-list.json`.
- Run focused docs checks after changes:

```bash
npm run harness:check
npm run lint:docs
npm run lint:bloat
git diff --check
```
