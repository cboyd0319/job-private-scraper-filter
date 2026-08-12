# Idea Index

This is the ranked index for v3 planning. It consolidates ideas from the v3
planning docs into implementation-sized entries and scores each by expected
benefit and effort.

Use this file to decide what gets promoted into an exec plan first. Keep the
source docs for detail, rationale, references, and release bars.

## Scoring

Benefit:

- 5: Transformative for users or required for the v3 architecture.
- 4: High-value workflow improvement or major competitive differentiator.
- 3: Useful feature that improves completion, quality, or trust.
- 2: Helpful polish, specialization, or developer leverage.
- 1: Nice-to-have or narrow value.

Effort:

- 1: Small docs, copy, config, or isolated UI work.
- 2: Focused feature or validation work.
- 3: Multi-surface feature with tests and docs.
- 4: Cross-cutting architecture, migration, or platform work.
- 5: Major refactor, research program, or high-risk multi-platform system.

Priority:

- Now: likely v3 foundation or early prototype.
- Next: high value after foundations exist.
- Later: valuable, but dependent on other primitives.
- Moonshot: high upside, needs research before product commitment.
- Cut: should not be built under the current product boundaries.

## Top-Level Ranked Themes

| Rank | Idea | Benefit | Effort | Priority | Primary docs |
| --- | --- | --- | --- | --- | --- |
| 1 | Opportunity case files | 5 | 5 | Now | North Star, Architecture Bets, Idea Catalog |
| 2 | Local event ledger | 5 | 4 | Now | Architecture Bets, Compatibility |
| 3 | Source graph and policy ledger | 5 | 4 | Now | Architecture Bets, Source Strategy |
| 4 | Qwen3 evidence engine | 5 | 5 | Now | Local Intelligence, Research Agenda |
| 5 | Browser companion protocol | 5 | 5 | Now | Architecture Bets, Source Strategy |
| 6 | Privacy receipts and data boundaries | 5 | 3 | Now | Architecture Bets, Security |
| 7 | V3 compatibility and rollback boundary | 5 | 4 | Now | Compatibility, Architecture Bets |
| 8 | Model governance and eval lab | 5 | 4 | Now | Architecture Bets, Evaluation |
| 9 | Commercial superiority benchmark | 5 | 2 | Now | Commercial Superiority Bar, Evaluation |
| 10 | Security Doctor and security UX primitives | 5 | 4 | Now | Security, Architecture Bets |
| 11 | Edition-aware runtime and Essentials package | 5 | 4 | Now | Editions, Evaluation |
| 12 | Regional runtime boundary for UK, EU, and India | 5 | 4 | Now | Regional Framework, Architecture Bets |
| 13 | Downloadable pack runtime | 5 | 5 | Now | Downloadable Agents, Architecture Bets |
| 14 | Local vector store contract | 5 | 4 | Now | Architecture Bets, Local Intelligence |
| 15 | Agent runtime with approval gates | 5 | 5 | Now | Architecture Bets, Local Intelligence |
| 16 | Native OS adapter layer | 4 | 4 | Next | Native OS, Architecture Bets |
| 17 | Hiring firewall | 5 | 4 | Next | Frontier Concepts, Security |
| 18 | Campaign simulator | 4 | 3 | Next | Idea Catalog, Frontier Concepts |
| 19 | Application cockpit | 4 | 4 | Next | Moonshots, Commercial Bar |
| 20 | Signed content-addressed pack registry | 4 | 4 | Next | Downloadable Agents, Moonshots |
| 21 | Employer intelligence layer | 5 | 4 | Now | Employer Intelligence, Source Strategy, Commercial Bar |

## Architecture And Data Primitives

| ID | Idea | Benefit | Effort | Priority | Notes |
| --- | --- | --- | --- | --- | --- |
| A01 | Rust core runtime as the trust boundary | 5 | 3 | Now | Needed before moving policy or platform work. |
| A02 | Opportunity case file id and workflow model | 5 | 5 | Now | Converges job, resume, source, tracker, interview, and offer state. |
| A03 | Local event ledger with derived views | 5 | 4 | Now | Enables learning, rollback, audit, and dashboard history. |
| A04 | Local career graph | 5 | 5 | Now | Shared evidence layer for matching, resumes, interviews, and offers. |
| A05 | Source graph and source policy ledger | 5 | 4 | Now | Required for safe source expansion and source packs. |
| A06 | Browser companion protocol | 5 | 5 | Now | Replaces fragile one-off import flows. |
| A07 | Agent workflow schema | 5 | 4 | Now | Build before unrestricted agent behavior is considered. |
| A08 | Model manifest, lockfile, and cache verifier | 5 | 4 | Now | Required for governed Qwen3 and future local models. |
| A09 | Eval lab for matching, source, and UI changes | 5 | 4 | Now | Prevents impressive ideas from shipping unverified. |
| A10 | Privacy receipt model | 5 | 3 | Now | Makes local-first trust visible. |
| A11 | Pack manifest schema | 5 | 4 | Now | Foundation for skills, agents, sources, roles, and regions. |
| A12 | Edition manifest and runtime profiles | 5 | 3 | Now | Keeps Essentials, Standard, Pro Local, and Developer coherent. |
| A13 | Compatibility manifest | 5 | 3 | Now | Defines v3 data, pack, source, model, and export contracts. |
| A14 | Native OS adapter layer | 4 | 4 | Next | Keeps platform integrations behind typed Rust commands. |
| A15 | Security UX service for consent, receipts, and repair | 5 | 4 | Now | Reduces prompts without weakening safety. |
| A16 | Regional manifest and taxonomy bridge contracts | 5 | 4 | Now | Lets UK, EU, and India grow through packs. |
| A17 | Sandboxed dynamic adapter runtime | 4 | 5 | Later | Research only until sandbox denial tests pass. |
| A18 | Vector provenance and stale-vector detector | 5 | 4 | Now | Prevents mixing incompatible model and chunker outputs. |
| A19 | Contract-derived documentation and validation | 4 | 4 | Next | Consolidates E09; generate only from stable schemas where it removes maintained duplication, without a generic code-generation framework. |
| A20 | Local runtime kernel for typed jobs | 5 | 5 | Later | Gate 1 defaults to existing application and Tokio owners; promote a new kernel only if evidence proves those owners insufficient. |
| A21 | Graph-native storage strategy | 4 | 5 | Later | Valuable after case files and event ledger are proven. |

## Core User Workflows

| ID | Idea | Benefit | Effort | Priority | Notes |
| --- | --- | --- | --- | --- | --- |
| U01 | First-run "get me to value" flow | 5 | 3 | Now | Must beat commercial install and setup friction. |
| U02 | Daily mission board | 5 | 3 | Now | Replaces tracker blank-state with next actions. |
| U03 | Weekly review | 4 | 3 | Next | Shows source yield, interview rate, pay misses, and stale roles. |
| U04 | Fatigue-aware pacing | 4 | 3 | Next | Helps users avoid low-value application volume. |
| U05 | Search lane health and stop rules | 4 | 3 | Next | Turns outcome data into better strategy. |
| U06 | Campaign simulator | 4 | 3 | Next | Compares strategy changes before users spend effort. |
| U07 | Application runway view | 3 | 2 | Next | Shows strength and volume of active opportunities. |
| U08 | "What changed since yesterday" summary | 4 | 3 | Next | Makes repeat use obvious and useful. |
| U09 | Smart career campaign inbox | 4 | 4 | Next | Extend the accepted campaign loop after the event ledger, case files, and source graph stabilize. |
| U10 | Low-pressure or overwhelmed mode | 4 | 3 | Next | Important for stressed and nontechnical users. |
| U11 | Case-file timeline | 5 | 3 | Now | The visible face of the event ledger. |
| U12 | Evidence wall | 5 | 4 | Now | Core differentiator versus opaque match scores. |
| U13 | Decision summary | 4 | 3 | Now | Turns analysis into apply, maybe, skip, or research. |
| U14 | "Why not this job?" panel | 5 | 3 | Now | Explains blockers and missing evidence. |
| U15 | Duplicate and repost lineage | 4 | 3 | Next | Saves time and improves ghost-job review. |
| U16 | Company relationship view | 3 | 3 | Later | Higher value after contacts and outcomes exist. |
| U17 | Full campaign operating model | 5 | 5 | Now | Milestone 6 owns the core outcome; advanced campaign features remain in train 12A. |
| U18 | Career strategy simulator | 4 | 4 | Later | Strong differentiator, but depends on outcomes and labels. |
| U19 | Local career compiler | 5 | 5 | Next | Consolidates X03 into one reviewed campaign-planning outcome built from user evidence, goals, and constraints. |
| U20 | Employer dossier | 5 | 4 | Now | Company identity, official sources, pay, risk, and user-owned outcome context. |
| U21 | Employer question planner | 4 | 3 | Next | Turns employer signals into recruiter, interview, and offer questions. |

## Source, Browser, And Import

| ID | Idea | Benefit | Effort | Priority | Notes |
| --- | --- | --- | --- | --- | --- |
| S01 | Official API source checks | 5 | 3 | Now | Greenhouse, Lever, USAJobs, RemoteOK, and similar public paths. |
| S02 | Public ATS and careers page adapters | 5 | 4 | Now | Workday, Ashby, SmartRecruiters, iCIMS, Phenom, Oracle, Taleo, and others. |
| S03 | Employer-careers discovery | 5 | 4 | Now | Maps arbitrary company pages to the safest source path. |
| S04 | ATS tenant discovery | 4 | 4 | Next | Converts ATS research into source graph coverage. |
| S05 | Public structured-data reader | 4 | 3 | Next | JSON-LD, microdata, RSS, Atom, sitemap, and JobPosting data. |
| S06 | Regional source packs | 5 | 4 | Now | Starter UK, EU, India, and future market expansion. |
| S07 | Role-family source packs | 4 | 3 | Next | Security, healthcare, trades, content, sales, education, and more. |
| S08 | Source coverage map | 4 | 2 | Now | Explains native, browser, manual, and under-review paths. |
| S09 | Source reliability score | 4 | 3 | Next | Exposes stale, noisy, blocked, or low-salary-coverage sources. |
| S10 | Company watchlists | 4 | 3 | Next | High-value employer monitoring for public sources. |
| S11 | Source simulator | 4 | 3 | Now | Required to validate parser drift and source packs. |
| S12 | Public source automation | 5 | 3 | Now | Low-friction public checks with limits and fixtures. |
| S13 | Restricted-source Workbench | 5 | 4 | Now | Helpful LinkedIn-style support without hidden account-backed monitoring. |
| S14 | Browser side panel or companion extension | 5 | 5 | Now | Main friction remover for visible user-controlled capture. |
| S15 | Visible-page overlay | 4 | 4 | Next | Marks cards with fit, risk, duplicate, and pay cues. |
| S16 | Visible-page capture | 5 | 4 | Now | Saves visible cards and details without hidden state. |
| S17 | Smart paste | 4 | 2 | Now | Turns pasted text, links, and screenshots into reviewable drafts. |
| S18 | OCR-assisted visible import | 4 | 4 | Next | Useful for rendered pages, screenshots, and PDFs with review. |
| S19 | One-click "I just applied" capture | 5 | 3 | Now | Directly attacks spreadsheet-like tracking friction. |
| S20 | Visible learning mode | 4 | 4 | Later | Learns only from JobSentinel-side actions and approved imports. |
| S21 | Cross-browser strategy | 4 | 4 | Next | Chrome, Edge, Firefox, Safari, and bookmarklet fallback. |
| S22 | Secure loopback pairing | 5 | 4 | Now | Required before browser companion can ship. |
| S23 | Application cockpit | 4 | 4 | Next | Combines browser, case file, resume packet, and helper side by side. |
| S24 | Universal visible-page import | 4 | 4 | Next | Makes unsupported sources useful. |
| S25 | User-controlled watch mode | 3 | 4 | Later | Only for visible user-controlled sessions. |
| S26 | Scraper technology evaluation | 3 | 2 | Now | Gate robust parser tools against product boundaries. |
| S27 | Employer intelligence source graph | 5 | 4 | Now | Maps employer identities, domains, ATS tenants, official registries, and provenance. |
| S28 | Employer official-data packs | 4 | 4 | Next | SEC, WARN, OFLC, H-1B, Companies House, EU, India, and similar public registries. |

## Resume, Matching, And Local Intelligence

| ID | Idea | Benefit | Effort | Priority | Notes |
| --- | --- | --- | --- | --- | --- |
| M01 | Application packet builder | 5 | 4 | Now | Competes against resume tools and tracker workflows. |
| M02 | Resume evidence graph | 5 | 4 | Now | Basis for truthful tailoring and Qwen3 evidence. |
| M03 | Requirement-by-requirement review | 5 | 4 | Now | Strongest product differentiator for apply decisions. |
| M04 | ATS readability lab | 4 | 4 | Next | PDF, DOCX, Markdown, JSON Resume, and common ATS parsing. |
| M05 | Regional CV profiles | 4 | 3 | Now | U.S., UK, Europass, India, public-sector, academic, trades. |
| M06 | Resume variant manager | 4 | 3 | Next | Tracks which resume version was used and what happened. |
| M07 | Bullet repair workspace | 4 | 3 | Next | Improves clarity without inventing claims. |
| M08 | Screening answer bank | 4 | 3 | Next | Exact-question provenance and reviewed answers. |
| M09 | Local application form rehearsal | 4 | 4 | Next | Reduces form risk before user opens real site. |
| M10 | Attachment checklist | 3 | 2 | Now | Low effort, immediate application-quality win. |
| M11 | Requirement-level matching | 5 | 4 | Now | Every hard and preferred requirement gets evidence. |
| M12 | Qwen3 reranker calibration | 5 | 4 | Now | Prevents one-size thresholds across query types. |
| M13 | Qwen3 reranker fine-tuning path | 4 | 5 | Later | Valuable after enough labels and hard negatives exist. |
| M14 | Learning-to-rank layer | 4 | 4 | Later | Use explainable features before full model fine-tuning. |
| M15 | Preference profiles | 4 | 3 | Next | Safe, stretch, pay-first, remote-first, and similar modes. |
| M16 | "Why did this not match?" diagnosis | 5 | 3 | Now | Better than opaque commercial scores. |
| M17 | Counterfactual fairness tests | 4 | 4 | Next | Needed before ranking claims are trusted. |
| M18 | Prompt-injection and adversarial posting guard | 5 | 4 | Now | Protects model, agent, and external AI paths. |
| M19 | Model diagnostics and Model Doctor | 5 | 3 | Now | Reduces failed setup and stale model confusion. |
| M20 | OS-native micro-assist | 3 | 3 | Next | Bounded local summaries, extraction, and accessibility copy. |
| M21 | Regional matching fixtures | 4 | 3 | Now | Prevents U.S.-centric matching behavior. |
| M22 | Label and feedback capture | 5 | 3 | Now | Required for learning and commercial benchmark proof. |
| M23 | Hard-negative mining | 5 | 4 | Next | Highest-return quality improvement loop. |
| M24 | Embedding fine-tuning | 3 | 5 | Later | Only if retrieval recall is poor. |
| M25 | Contextual bandits | 3 | 5 | Moonshot | Needs substantial local feedback history. |
| M26 | Open candidate-side matching benchmark | 4 | 4 | Next | Consolidates X04; publish only after internal evaluation sets and evidence contracts stabilize. |
| M27 | Local match debugger | 4 | 3 | Now | Developer and user trust tool for matching decisions. |

## Agents, Skills, And Automation

| ID | Idea | Benefit | Effort | Priority | Notes |
| --- | --- | --- | --- | --- | --- |
| G01 | Local skill runner | 5 | 4 | Now | Turns v2.9 downloadable skills into product workflows. |
| G02 | Search Planner agent | 4 | 3 | Next | Builds lanes, sources, stop rules, and weekly review. |
| G03 | Source Analyst agent | 4 | 3 | Next | Explains source health, duplicates, salary coverage, and policy state. |
| G04 | Posting Risk Reviewer agent | 4 | 3 | Next | Ghost-job, scam, stale, and weak-source review. |
| G05 | Resume Evidence Reviewer agent | 5 | 3 | Now | Maps requirements to evidence. |
| G06 | Application Packet Builder agent | 5 | 3 | Now | Prepares reviewed materials without final submit. |
| G07 | Browser Companion Assistant | 4 | 4 | Next | Suggests local actions beside visible pages. |
| G08 | Interview Coach agent | 4 | 3 | Next | Uses case file and resume evidence. |
| G09 | Negotiation Analyst agent | 4 | 3 | Next | Reviews written offer, verbal claims, costs, and deadlines. |
| G10 | Privacy Reviewer agent | 4 | 3 | Next | Explains data use before sensitive actions. |
| G11 | Human-in-the-loop task plans | 5 | 3 | Now | Required control model for every agent. |
| G12 | "Prepare this job" action | 5 | 3 | Now | High-value workflow shortcut. |
| G13 | "Replan from tracker" action | 4 | 3 | Next | Uses real outcomes to change search strategy. |
| G14 | Local MCP server | 4 | 4 | Later | Useful after scopes, receipts, and pairing exist. |
| G15 | Provider routing across external AI and local endpoints | 4 | 4 | Next | Must remain behind privacy-first gateway. |
| G16 | Agent failure-mode view | 4 | 2 | Now | Makes automation understandable. |
| G17 | Downloadable agent packs | 5 | 4 | Now | Structured agents with permissions and approval gates. |
| G18 | Expanded skills with handoff metadata | 4 | 3 | Now | Builds on v2.9 skill investment. |
| G19 | Eval packs for agent and skill quality | 4 | 3 | Now | Required before downloadable workflows scale. |

## Interview, Offer, And Career Outcomes

| ID | Idea | Benefit | Effort | Priority | Notes |
| --- | --- | --- | --- | --- | --- |
| O01 | Interview room | 4 | 4 | Next | Turns job and evidence into prep modes. |
| O02 | Story bank | 4 | 3 | Next | Reusable interview evidence. |
| O03 | Mock interview mode | 3 | 4 | Later | Useful, but model quality and UX need validation. |
| O04 | Work-sample planner | 3 | 3 | Later | Valuable for role families with exercises. |
| O05 | Offer cockpit | 4 | 4 | Next | Extends usefulness beyond application tracking. |
| O06 | Negotiation packet | 4 | 3 | Next | Facts, ask, fallback, questions, and decline script. |
| O07 | Post-interview debrief | 4 | 2 | Now | Low effort, high tracker and prep value. |
| O08 | Financial runway planner | 3 | 4 | Later | Sensitive, optional, and not financial advice. |
| O09 | Referral and networking graph | 4 | 4 | Later | High value after contacts and privacy labels exist. |
| O10 | Warm-path detection | 4 | 4 | Later | Depends on user-entered contacts and company history. |
| O11 | Scam response workflows | 4 | 3 | Next | Gives safe next steps beyond warnings. |
| O12 | Employer context panel | 4 | 3 | Next | Uses dossier signals for interview prep, recruiter questions, and offer risk. |

## Security, Privacy, And Governance

| ID | Idea | Benefit | Effort | Priority | Notes |
| --- | --- | --- | --- | --- | --- |
| P01 | Privacy Doctor | 5 | 3 | Now | Checks telemetry, AI, vault, backup, browser, and sources. |
| P02 | Encrypted backup and restore | 5 | 4 | Now | Required before v3 compatibility promises. |
| P03 | Multi-profile vaults | 4 | 4 | Later | Useful for shared machines and career transitions. |
| P04 | Safe support bundle | 5 | 3 | Now | Helps debug without leaking private data. |
| P05 | Extension security dashboard | 4 | 3 | Next | Shows pairings, permissions, last activity, and revoke controls. |
| P06 | Local policy ledger | 5 | 3 | Now | Source warnings, consent, and restricted rationale. |
| P07 | Smart consent | 5 | 3 | Now | Reduces repeat prompts safely. |
| P08 | Native trust integration | 4 | 4 | Next | LocalAuthentication, Windows Hello, Secret Service, and fallbacks. |
| P09 | Recovery and repair center | 5 | 4 | Now | Reset vault, rotate pairings, verify models, restore backup. |
| P10 | Prompt-injection guard | 5 | 4 | Now | Mandatory for untrusted job and resume text. |
| P11 | Malicious pack quarantine | 5 | 4 | Now | Required before packs scale. |
| P12 | AI governance mapping | 3 | 3 | Next | NIST AI RMF, EU AI Act concepts, and public docs. |
| P13 | Passkeys and device trust | 3 | 4 | Later | Useful for pairing and sensitive approvals. |
| P14 | Candidate data rights toolkit | 4 | 3 | Next | Export, delete, support, migration, and provider request receipts. |
| P15 | Privacy-preserving source intelligence mesh | 3 | 5 | Moonshot | Only aggregate source health, never private campaign data. |

## Ecosystem, Packs, And Distribution

| ID | Idea | Benefit | Effort | Priority | Notes |
| --- | --- | --- | --- | --- | --- |
| E01 | Download chooser | 5 | 2 | Now | Makes v3 easier to install than v2.9. |
| E02 | One-click update, rollback, and repair | 5 | 4 | Now | Must align with signing and platform rules. |
| E03 | First-run doctor | 5 | 3 | Now | Vault, storage, permissions, browser, model, and source checks. |
| E04 | Source-pack marketplace | 4 | 4 | Next | Signed packs with fixtures and policy metadata. |
| E05 | Content-addressed pack registry | 4 | 4 | Next | Trustable distribution without accounts or telemetry. |
| E06 | Role-pack marketplace | 4 | 3 | Next | Career-specific sources, rubrics, and taxonomies. |
| E07 | Skill-pack marketplace | 4 | 3 | Next | Builds on Agent Skills spec. |
| E08 | Local API and CLI | 4 | 4 | Later | Power-user and automation bridge after scopes exist. |
| E10 | Import bridges | 4 | 3 | Next | JSON Resume, bookmarks, CSV, calendar, email reminders, notes. |
| E11 | Export package without lock-in | 5 | 3 | Now | Commercial differentiator and privacy promise. |
| E12 | Storage cleanup center | 4 | 2 | Now | Model packs, caches, logs, exports, and stale vectors. |
| E13 | Pack quarantine and self-test | 5 | 4 | Now | Manifest, signature, privacy, permissions, fixtures, and size checks. |
| E14 | Pack playground | 3 | 3 | Next | Synthetic fixture testing for community packs. |
| E15 | Portable package | 4 | 4 | Next | Shared computers, workforce centers, and no-admin setups. |
| E16 | Offline pack bundles | 3 | 3 | Next | Extends the accepted pack architecture for regional access, shared computers, and limited connectivity. |

## Native OS And Platform Integration

| ID | Idea | Benefit | Effort | Priority | Notes |
| --- | --- | --- | --- | --- | --- |
| N01 | Deep links | 3 | 2 | Next | Case files, reminders, imports, and browser pairing. |
| N02 | File associations | 3 | 3 | Next | Resume, backup, source pack, model pack, and export files. |
| N03 | Native notifications with actions | 4 | 3 | Next | Follow-ups, interviews, offers, source failures. |
| N04 | Tray or menu bar helper | 3 | 4 | Later | Quick capture, source status, reminders, privacy lock. |
| N05 | Global shortcut for quick capture | 3 | 3 | Later | Disabled by default. |
| N06 | Clipboard import helper | 3 | 2 | Next | Explicit user action and token scrubbing. |
| N07 | Drag-and-drop import | 4 | 2 | Now | Milestone 6 owns resume, job, and backup import; Milestone 7 adds source packs after quarantine and installer controls. |
| N08 | Native print and PDF export | 3 | 3 | Next | Resumes, packets, and reports. |
| N09 | OS search metadata | 3 | 4 | Later | Safe metadata only, no private text by default. |
| N10 | Platform health diagnostics | 4 | 3 | Now | Vault, database, sources, browser, model cache, permissions. |
| N11 | macOS App Intents and Shortcuts | 3 | 4 | Later | Quick actions with reviewable scopes. |
| N12 | macOS Core Spotlight and Quick Look | 3 | 4 | Later | Safe metadata and previews only. |
| N13 | macOS Foundation Models | 3 | 4 | Later | Bounded micro-assist only. |
| N14 | macOS Vision OCR | 4 | 4 | Next | User-selected screenshots and documents. |
| N15 | Windows Hello, DPAPI, and Credential Locker | 4 | 4 | Next | Sensitive unlock and vault adapters. |
| N16 | Windows protocol activation and toasts | 3 | 3 | Next | Case files, imports, reminders, browser pairing. |
| N17 | Windows AI APIs and DirectML | 3 | 4 | Later | Bounded local helpers and acceleration research. |
| N18 | Windows OCR | 4 | 4 | Next | User-selected screenshots and documents. |
| N19 | Linux Secret Service and xdg-desktop-portal | 4 | 4 | Next | Vault, files, notifications, and permissions. |
| N20 | Platform-specific package repair | 5 | 4 | Now | Permissions, vault, browser, model cache, source packs. |

## Accessibility, Regions, And Access Equity

| ID | Idea | Benefit | Effort | Priority | Notes |
| --- | --- | --- | --- | --- | --- |
| R01 | Plain-language setup | 5 | 2 | Now | Starts with user goal, not source configuration. |
| R02 | Accessibility-first UI checks | 5 | 3 | Now | Keyboard, screen reader, contrast, zoom, reduced motion. |
| R03 | Cognitive-load and overwhelmed-user review | 4 | 3 | Next | Helps stressed users complete workflows. |
| R04 | Regional vocabulary and pay normalization | 5 | 4 | Now | Work mode, currency, salary period, visa, relocation, public sector. |
| R05 | UK starter pack | 4 | 4 | Now | GOV.UK, UK SOC, UK pay and CV patterns. |
| R06 | EU starter pack | 4 | 4 | Now | EURES, ESCO, Europass, multilingual fixtures. |
| R07 | India starter pack | 4 | 4 | Now | NCS, NCO, NSQF, CTC/LPA/monthly pay patterns. |
| R08 | Career-change mode | 4 | 3 | Next | Values adjacent evidence and explains stretch gaps. |
| R09 | Returnship and caregiver mode | 3 | 3 | Next | Extend inclusive job-search support after focused research, careful copy review, and evaluation. |
| R10 | Veteran, military-transition, disability-aware, and early-career modes | 5 | 4 | Now | User-prioritized v3.0 scope; preserve current veteran support and add reviewed civilian-role and public-service paths without inferring protected status or eligibility. |
| R11 | Trades, healthcare, education, and public-sector modes | 4 | 4 | Next | Role-specific sources, credentials, and application flows. |
| R12 | Mobile companion without cloud lock-in | 3 | 5 | Later | Reminders, quick notes, and interview prep. |
| R13 | Shared-computer privacy mode | 4 | 3 | Next | Lock, cleanup, local-data visibility, safe support. |
| R14 | Offline-first mode | 5 | 3 | Now | Baseline behavior across recovery, daily workflow, sources, imports, and updates rather than a separately deferred mode. |
| R15 | Data-saver source mode | 3 | 3 | Next | Fewer pages and lower concurrency. |
| R16 | Battery-friendly mode | 3 | 2 | Next | Runtime profile and background task tuning. |
| R17 | Essentials-to-stronger-local-ML upgrade path | 5 | 3 | Now | Lets users grow without reinstalling. |

## Research And Moonshots

| ID | Idea | Benefit | Effort | Priority | Notes |
| --- | --- | --- | --- | --- | --- |
| X01 | Candidate-side infrastructure standard | 5 | 5 | Moonshot | Local career data, packets, receipts, ledger, MCP, browser protocol. |
| X02 | Personal hiring firewall | 5 | 4 | Next | Protects users from low-quality or unsafe hiring systems. |
| X05 | Local evidence engine | 5 | 5 | Now | Qwen3 plus provenance, blockers, and explanations. |
| X06 | Source intelligence mesh | 3 | 5 | Moonshot | Source health only, no private data. |
| X07 | Candidate data rights toolkit | 4 | 3 | Next | Export, deletion report, support report, migration report. |
| X08 | Multi-device without cloud ownership | 4 | 5 | Later | Encrypted backup, LAN transfer, QR pairing, conflict review. |
| X09 | Evidence credential wallet | 4 | 4 | Later | Certifications, licenses, work samples, and proof without over-sharing. |
| X10 | Hiring web observatory | 4 | 4 | Later | ATS families, source health, salary transparency, duplicates. |
| X11 | Hiring-system transparency simulator | 3 | 5 | Moonshot | Simulates generic ATS stress without claiming employer internals. |
| X12 | Local career digital twin | 4 | 5 | Moonshot | Local model of evidence, goals, outcomes, and constraints. |
| X13 | Outcome-learning engine | 4 | 5 | Later | Needs event ledger and enough outcome data. |
| X14 | Privacy-preserving community intelligence | 3 | 5 | Moonshot | Only after privacy boundaries are proven. |
| X15 | Job-market strategy simulator | 4 | 4 | Later | Broader version of campaign simulator. |
| X16 | Local MCP server | 4 | 4 | Later | Useful after scopes and receipts exist. |
| X17 | JobSentinel CLI | 3 | 3 | Later | Doctor, import, model, source, pack, and debug commands. |
| X18 | Signed community pack registry | 4 | 4 | Next | Needs pack runtime and quarantine first. |
| X19 | Verifiable evidence wallet | 3 | 5 | Moonshot | Research before product commitment. |
| X20 | Zero-footprint ephemeral session | 3 | 5 | Moonshot | Useful for shared machines, but high platform risk. |
| X21 | Employer intelligence layer | 5 | 4 | Now | Candidate-side company dossiers without central review harvesting. |

## Commercial Benchmark Ideas

| ID | Idea | Benefit | Effort | Priority | Notes |
| --- | --- | --- | --- | --- | --- |
| C01 | Product-class benchmark matrix | 5 | 2 | Now | Trackers, resume scanners, browser copilots, auto-apply, boards, chatbots. |
| C02 | Competitive demo script | 5 | 2 | Now | Install, region, resume, search, browser, match, packet, track, offer, export. |
| C03 | Time-to-first-use metric | 5 | 2 | Now | Must beat commercial friction. |
| C04 | Copy/paste burden metric | 5 | 2 | Now | Measures visible import and applied logging. |
| C05 | Evidence coverage metric | 5 | 3 | Now | Percent of recommendations with cited evidence. |
| C06 | Recovery benchmark | 4 | 3 | Now | Failed source, model, update, vault, and pack states. |
| C07 | Export and lock-in benchmark | 4 | 2 | Now | V3 should beat hosted products on portability. |
| C08 | Modest-hardware benchmark | 4 | 3 | Now | Essentials must remain responsive. |
| C09 | Regional-readiness benchmark | 4 | 3 | Next | UK, EU, and India starter coverage. |
| C10 | Unsafe-automation cut line | 5 | 1 | Now | Do not compete through hidden scraping or final auto-submit. |
| C11 | No-paywall parity benchmark | 5 | 2 | Now | Prove core workflows are useful without paid tiers or hosted accounts. |
| C12 | Candidate advantage lab | 5 | 3 | Now | Compare product classes by cost, effort, evidence, privacy, recovery, and outcomes. |
| C13 | Source-truth benchmark | 5 | 3 | Now | Measures official-source confidence, duplicate lineage, stale risk, and safe next steps. |
| C14 | Trust and scam response benchmark | 5 | 3 | Now | Goes beyond warnings into verification and reporting guidance. |
| C15 | Pay clarity benchmark | 5 | 3 | Now | Measures salary evidence, range quality, pay-floor fit, and dated policy-pack coverage. |
| C16 | Employer intelligence benchmark | 5 | 3 | Now | Measures source provenance, company verification, public signals, and user outcome reuse. |

## Cut Lines

These ideas should not be promoted unless the product boundary changes:

| Idea | Reason |
| --- | --- |
| Hidden restricted-source monitoring | Conflicts with restricted-source boundary and user-visible control. |
| Stored restricted-source cookies or tokens | Conflicts with credential and session safety. |
| Background account-backed refresh | Conflicts with explicit user-driven restricted-source sessions. |
| Final application submission without user review | Conflicts with final user control. |
| Fake resume claims or unsupported keyword stuffing | Conflicts with truthful evidence-backed resume help. |
| Human-check solving, proxy rotation, or fingerprint evasion | Conflicts with platform-control boundaries. |
| Broad shell, filesystem, or network access for packs | Conflicts with Rule 0 and local security boundaries. |
| Opaque hiring-probability claims | Conflicts with evidence-based candidate decision support. |
