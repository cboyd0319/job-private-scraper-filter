# JobSentinel V3 Planning

JobSentinel v3.0 is the next major-release planning horizon. Everything in the
v2.9 line should be treated as stabilization, minor feature polish, and bug
fixes unless a v3 discovery exposes a release-critical issue.

This directory is intentionally expansive. V3 can include a full refactor,
paper-driven model changes, new source architecture, new browser companion
workflows, a local agent runtime, and a different product shape. Rust remains
the base of JobSentinel and the primary local trust boundary. The point of this
package is to think beyond "make v2 better" and identify what would make
JobSentinel genuinely impressive.

The [V3 Master Execution Plan](master-exec-plan.md) is the sole owner for v3
scope, sequence, progress, decisions, verification, and handoff. The documents
below are references. They do not independently authorize implementation.

## Baseline

V2.9 provides the foundation:

- Local-first desktop storage with no hosted account or telemetry requirement.
- Public and official-source job checks, employer-careers discovery, Browser
  Import, and restricted-source Workbench flows.
- Ghost-job, posting-risk, pay-protection, resume, application-assist, and
  application-tracking workflows.
- Optional external AI through the privacy-first AI gateway.
- Local semantic matching with governed Qwen3 embedding and reranking models in
  embedded-ML builds.
- Downloadable Agent Skills for job search, resume, outreach, interview, and
  offer workflows.

See [Features And Capabilities](../../features/capabilities.md) for the
maintained v2.9 map.

## V3 Target

V3 should move from "JobSentinel helps me organize and analyze jobs" to
"JobSentinel runs my private job-search operating system with me."

That means:

- Manual data entry becomes the fallback, not the normal path.
- Every job becomes an opportunity case file with source, fit, risk, resume,
  application, interview, contact, follow-up, offer, and outcome history.
- Every recommendation is explainable through visible evidence and uncertainty.
- Every sensitive action is local, previewed, reversible, and user controlled.
- Every source path has a clear policy class, rate limit, and privacy boundary.
- Every automation removes user burden without hiding what happened.
- Users with modest hardware get a dedicated Essentials package that stays fast,
  local, and useful without large model downloads.
- V3 is the easiest release to install, set up, recover, and use.

V3.0.0 is the first supported compatibility baseline. JobSentinel makes no
forward-migration, rollback, or contract-compatibility promise for databases,
settings, APIs, packs, or artifacts created before v3.0.0. Starting with v3,
JobSentinel should lock the long-term architecture, define stable data and
config contracts, and support documented rollback paths within the v3
compatibility line.

## Non-Negotiables

The planning scope is wide, but these product constraints still define the
trusted shape of JobSentinel:

- User privacy and security are Rule 0.
- Core workflows must remain useful without cloud AI, telemetry, hosted sync, or
  a JobSentinel account.
- External AI remains optional, disabled by default, previewed, approved, and
  routed through the privacy-first AI gateway.
- Restricted authenticated sources must not store cookies, auth headers,
  browser storage, or session tokens.
- JobSentinel must not run hidden restricted-source monitoring.
- JobSentinel must not bypass human checks, platform controls, or final user
  review.
- Resume help must remain truthful and evidence-backed.
- Final application submission stays user-controlled.
- Rust remains the core runtime for durable local business logic, source
  adapters, storage, model governance, privacy boundaries, and security checks.
- Windows 11+, macOS, and Linux remain first-class targets.

V3 should maximize automation inside these boundaries. The goal is fewer
burdens for the user, not hidden behavior.

## Documents

| Doc | Purpose |
| --- | --- |
| [V3 Master Execution Plan](master-exec-plan.md) | Sole active execution owner for the v3.0 and v3.x scope, backlog disposition, milestones, gates, verification, and release closure. |
| [North Star](north-star.md) | Product vision, strategic bets, and anti-goals. |
| [Idea Catalog](idea-catalog.md) | Broad v3 feature and moonshot inventory. |
| [Idea Index](idea-index.md) | Ranked benefit and effort index for v3 ideas and implementation sequencing. |
| [Moonshots And Refactors](moonshots-and-refactors.md) | Bigger swings, full-refactor options, and research-paper-driven architecture changes. |
| [Frontier Concepts](frontier-concepts.md) | Second-pass concepts that treat JobSentinel as candidate-side infrastructure, not just an app. |
| [Unexplored Frontiers](unexplored-frontiers.md) | Additional lanes for accessibility, governance, evidence wallets, sync, passkeys, mobile, networking, scams, and shared-computer use. |
| [Final Amplification Review](final-amplification-review.md) | Last "think bigger" pass across v3 ideas before implementation planning. |
| [Commercial Superiority Bar](commercial-superiority-bar.md) | Benchmark target for making v3 better than commercially similar products in the ways users feel. |
| [Job Seeker Platform Research](job-seeker-platform-research.md) | Evidence-backed investigation into leading platforms, job seeker needs, paid-tool gaps, and the v3 leveling-the-field mission. |
| [Employer Intelligence](employer-intelligence.md) | Candidate-side company dossiers, official-source research, source provenance, user-owned outcomes, and privacy boundaries for employer context. |
| [Native OS Integration](native-os-integration.md) | Advanced macOS, Windows, Linux, and Tauri integration ideas. |
| [Security With Less Friction](security-with-less-friction.md) | Ways to improve security while reducing prompts, confusion, and setup burden. |
| [Architecture Bets](architecture-bets.md) | System primitives that make the ideas coherent. |
| [Source And Browser Strategy](source-and-browser-strategy.md) | Job source expansion, source policy, and browser companion direction. |
| [Local Intelligence And Agents](local-intelligence-and-agents.md) | Local ML, agent workflows, model governance, feedback learning, and AI gateway evolution. |
| [Downloadable Agents And Skills](downloadable-agents-and-skills.md) | Expanded v3 ecosystem for skills, agents, workflow packs, role packs, source packs, and eval packs. |
| [Regional Readiness Framework](regional-readiness-framework.md) | Starter architecture for UK, EU, and India support without overpromising full global coverage. |
| [Editions And Packaging](editions-and-packaging.md) | V3 package strategy, including an Essentials package for modest hardware. |
| [Compatibility And Migration](compatibility-and-migration.md) | V3 as the first supported long-term compatibility and rollback boundary. |
| [Research Agenda](research-agenda.md) | Papers, prototypes, and evaluations that should inform v3 decisions. |
| [Evaluation And Release Bar](evaluation-and-release-bar.md) | How v3 ideas should be tested before they are considered release-ready. |

## Planning Rules

Use these rules when turning any idea in this directory into a concrete plan:

1. Start with user burden removed, not technology added.
2. Identify what data is read, where it is stored, and what leaves the device.
3. Name the source class and policy boundary before building a source adapter.
4. Prefer official APIs, public feeds, and user-visible browser capture.
5. Add eval fixtures before changing ranking or resume/job matching behavior.
6. Require visible user confirmation before durable application records,
   provider calls, sensitive exports, or external side effects.
7. Keep local deterministic fallbacks for AI-powered workflows.
8. Measure whether the feature helps nontechnical users finish real job-search
   tasks faster and with less confusion.
9. Keep install, first run, package selection, and recovery simpler than v2.9.

## Reference Baseline

Internal references:

- [Job Sources](../../features/job-sources.md)
- [Browser Import](../../features/browser-import.md)
- [Privacy-First AI Gateway](../../security/privacy-first-ai-gateway.md)
- [Local Semantic Matching](../../developer/LOCAL_SEMANTIC_MATCHING.md)
- [Semantic Resume-Job Matching Research](../../research/semantic-resume-job-matching.md)
- [Job Seeker Platform Research](job-seeker-platform-research.md)
- [Employer Intelligence](employer-intelligence.md)
- [Security Docs](../../security/README.md)

External references to keep in view during v3 design:

- [Agent Skills specification](https://agentskills.io/specification.md)
- [Model Context Protocol](https://modelcontextprotocol.io/)
- [OpenAI Agents guide](https://platform.openai.com/docs/guides/agents)
- [Greenhouse job board API](https://developers.greenhouse.io/job-board.html)
- [Lever postings API](https://github.com/lever/postings-api)
- [Google JobPosting structured data](https://developers.google.com/search/docs/appearance/structured-data/job-posting)
- [SEC EDGAR APIs](https://www.sec.gov/search-filings/edgar-application-programming-interfaces)
- [DOL OFLC performance data](https://www.dol.gov/agencies/eta/foreign-labor/performance)
- [Apple Foundation Models framework](https://developer.apple.com/documentation/foundationmodels)
- [Chrome extension side panel API](https://developer.chrome.com/docs/extensions/reference/api/sidePanel)
- [Windows AI APIs](https://learn.microsoft.com/en-us/windows/ai/apis/)
- [ESCO web-service API](https://esco.ec.europa.eu/en/use-esco/use-esco-services-api/esco-web-service-api)
