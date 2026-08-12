# Job Seeker Platform Research

Last updated: 2026-06-21.

This is the final v3 planning research pass on what strong job search platforms
do well, where they still fail candidates, and what JobSentinel should become
if it is serious about leveling the field for people who cannot pay for premium
help.

The goal is not to copy commercial tools. The goal is to make JobSentinel the
best candidate-side platform for finding real, relevant, fairly compensated
work while keeping private career data under user control.

## Research Posture

There is no single "best" job search platform. The market is split into narrow
tools:

- job boards and aggregators with large inventory;
- professional networks with paywalled reach and insights;
- trackers that reduce spreadsheet work;
- resume scanners that score documents against postings;
- browser helpers that reduce copy and form work;
- public employment services that add guidance, CV help, regional context, and
  sometimes human support.

That means v3 should be evaluated by user outcome, not feature count. A good
candidate platform helps the user answer:

- Which roles are real enough to spend time on?
- Which roles fit my evidence, constraints, pay floor, location, schedule, and
  seniority?
- What exactly should I do next?
- What data did I share, and what stayed private?
- How do I keep going without wasting effort or paying for an advantage?

## What Leading Platforms Prove

### Paid Networks Sell Useful Reach

LinkedIn Premium Career advertises paid job-post insights, AI-assisted
profile/message help, advanced search, profile visibility, learning content, and
InMail-based outreach. LinkedIn also claims Premium users are more likely to
hear back and get hired, based on LinkedIn-owned data.

Product lesson:

- Outreach and competitive insight matter.
- V3 cannot reproduce proprietary network reach, but it can give free users a
  local warm-path planner: saved contacts, companies, alumni, referral asks,
  recruiter notes, follow-up prompts, and evidence-backed outreach templates.
- Vendor performance claims should be treated as benchmark inputs, not proof.

### Trackers Reduce Spreadsheet Work, But Often Stop Too Early

Teal and Huntr show that job seekers value saved jobs, notes, application
statuses, reminders, contacts, resume variants, browser clipping, and templates.
Their free tiers are useful, but advanced AI, unlimited tailored documents,
advanced scoring, or unlimited tracking can be paid-tier features.

Product lesson:

- Tracking is table stakes.
- V3 should turn every job into an opportunity case file with source history,
  fit, risk, resume evidence, application packet, follow-up, interview, offer,
  outcome, and privacy receipts.
- JobSentinel should compete on zero cost for core workflows, exportability,
  privacy, and full-funnel continuity.

### Resume Scanners Prove Demand For Preparation Feedback

Jobscan's public material centers on match reports, missing skills, formatting
advice, LinkedIn optimization, scan history, and premium unlimited scanning.
The free tier is useful but limited.

Product lesson:

- Users want a clear answer before they apply.
- V3 should replace opaque or keyword-heavy scores with requirement-level
  evidence: hard requirement satisfied, weak evidence, missing evidence,
  unsupported keyword edit, ATS readability issue, and "why this did not match."
- The strongest free advantage is truthful preparation tied to the user's own
  evidence.

### Public Job Surfaces Need Source Intelligence

Google's JobPosting structured data guidance shows that public job content can
be made machine-readable for job search experiences. GOV.UK, EURES, and India's
National Career Service show another model: public or public-adjacent systems
combine search with advice, CV guidance, counselling, regional context, and
human support.

Product lesson:

- V3 should not treat job boards as the only source of truth.
- Official APIs, employer careers pages, public structured data, government
  services, regional portals, and user-visible browser capture should all feed a
  source graph with policy and confidence metadata.
- Region packs should model public services and local conventions, not only
  translate labels.

## What Job Seekers Need Most

### Time Protection

People do not only need more listings. They need fewer bad decisions. Existing
JobSentinel research already identifies application fatigue, source quality,
search-lane health, and long quiet periods as major workflow risks.

V3 requirements:

- rank opportunities by evidence, source confidence, user constraints, and
  next-action value;
- detect duplicates, reposts, stale listings, weak postings, and source drift;
- show stop rules when a lane produces effort without outcomes;
- keep weekly reviews calm and practical, not shame-based.

### Trust Protection

FTC guidance says job scam reports nearly tripled from 2020 to 2024 and
reported losses rose from $90 million to $501 million. The FTC also warns about
task scams, payment requests, suspicious messaging, and fake employment agency
patterns.

V3 requirements:

- separate scam risk from stale or ghost-posting risk;
- preserve source provenance, first-seen date, last-seen date, repost lineage,
  domain checks, and user corrections;
- provide safe next steps for suspicious roles, not just a warning badge;
- never ask users to upload private resume data to a central service to get
  basic safety help.

### Pay And Cost Clarity

NWLC analysis found stronger pay disclosure where job-listing pay transparency
laws require ranges in postings. Pay laws are changing by state and locality, so
JobSentinel must avoid hard-coded legal claims and revalidate jurisdiction data
before shipping legal-specific copy.

V3 requirements:

- track salary range presence, range width, source, currency, pay period, and
  confidence;
- compare pay to user salary floor, commute, relocation, schedule, benefits,
  and total cost of work;
- label broad or vague ranges as low-quality evidence instead of treating them
  as useful pay transparency;
- show jurisdiction-sensitive guidance only from maintained policy packs with
  review dates and source links.

### Access For People Without Paid Tools

Premium career tools create a practical access gap: applicant insights, advanced
matching, unlimited tailored materials, scan history, and outreach tools are
often better for users who can pay. JobSentinel's free, local-first model is the
product advantage only if the free path is genuinely strong.

V3 requirements:

- no paid tier for core local features;
- no hosted account requirement for core value;
- no required telemetry or resume upload;
- no large model download required for Essentials;
- no hidden lock-in that prevents export, backup, or rollback;
- plain recovery when the user has an older computer, limited internet, or a
  shared machine.

### Digital And Accessibility Support

National Skills Coalition and the Federal Reserve Bank of Atlanta found broad
demand for digital skills, while many workers lack foundational digital skills.
JAN provides free, confidential job-seeking and accommodation guidance for
people with disabilities. Pew Research Center also shows workers are more
worried than hopeful about workplace AI, with lower- and middle-income workers
more likely to expect fewer job opportunities from AI.

V3 requirements:

- first-run setup must assume no technical vocabulary;
- every advanced feature needs plain-language mode and an expert mode boundary;
- keyboard, screen reader, high zoom, reduced motion, and low-distraction
  workflows are release criteria;
- disability-aware guidance should point users to trusted resources and avoid
  guessing sensitive traits;
- AI features should feel like reviewable help, not another opaque gate.

## V3 Platform Bar

The minimum bar for "best job search platform for everyone" is:

| Area | V3 bar |
| --- | --- |
| Inventory | Official APIs, employer pages, public structured data, regional portals, public boards, and visible browser capture. |
| Trust | Source confidence, scam risk, stale-posting risk, duplicate lineage, and privacy receipts. |
| Fit | Requirement-by-requirement evidence, blockers, seniority, pay, location, schedule, and user constraints. |
| Action | Daily mission board, one-click save/log applied, packet builder, reminders, interview prep, offer review, and weekly replan. |
| Equity | Free core features, Essentials package, portable package, plain-language setup, export, and no account requirement. |
| Privacy | Local data by default, external AI preview/approval, no restricted-source session storage, no telemetry requirement. |
| Recovery | Doctor, repair, backup, rollback, model cleanup, source cleanup, and safe support report. |
| Regions | UK, EU, and India starter packs with source, pay, CV, and terminology framework. |

## Research-Backed Product Opportunities

### 1. Candidate Advantage Lab

Create a recurring benchmark suite that compares JobSentinel against product
classes, not one named competitor.

Benchmark tasks:

- install and reach first useful action;
- save a visible job;
- import a resume;
- map requirements to evidence;
- identify a stale, duplicate, or suspicious posting;
- create a truthful application packet;
- log an application and follow-up;
- prepare for an interview;
- evaluate a written offer;
- export everything.

Record cost, required account, private-data egress, clicks, copy/paste steps,
evidence coverage, recovery path, and novice-user completion.

### 2. Source Truth Layer

Build a source graph that answers:

- Where did this role come from?
- Is there an official employer or ATS version?
- Has it been reposted or changed?
- Does it have usable salary evidence?
- Is this source public, restricted, browser-only, or under review?
- What can JobSentinel do automatically, and what requires user-visible action?

### 2A. Employer Intelligence Layer

Build candidate-side employer dossiers that connect source truth to company
context:

- official employer domains, careers pages, and ATS tenants;
- public registry and policy-pack signals with dates and jurisdiction;
- pay clarity, sponsorship context, scam checks, and source confidence;
- local user-owned application, interview, offer, and contact history;
- recruiter and interviewer questions based on known unknowns.

See [Employer Intelligence](employer-intelligence.md) for the full source
hierarchy, privacy boundary, data dimensions, and release bar.

### 3. No-Paywall Parity Pack

Identify features that commercial tools commonly gate and make local versions
first-class:

- applicant-fit insight through local requirement evidence;
- outreach planning through local contact and company context;
- resume scan history through local evidence records;
- advanced matching through governed local models and deterministic fallbacks;
- interview and follow-up templates through downloadable skills and agents.

This does not mean copying proprietary network access. It means giving the user
the best free candidate-side alternative where JobSentinel has legitimate local
data.

### 4. Public Service Bridge

Model regional and public-service workflows directly:

- GOV.UK and UK public-sector flows;
- EURES and Europass-adjacent CV, mobility, and legal/social-security context;
- India's National Career Service career counselling and vocational guidance
  model;
- U.S. public-sector, workforce-center, veteran, disability, apprenticeship,
  and trades paths.

The bridge should be source and guidance metadata, not a claim of full regional
coverage.

#### Veteran And Military-Transition Research Inputs

Use these sources as research inputs with explicit authority and freshness
labels. A listing or suggested civilian equivalent is never user evidence by
itself.

| Source | Research role | Required handling |
| --- | --- | --- |
| [O*NET Crosswalk Files](https://www.onetcenter.org/crosswalks.html) | Primary machine-readable military-to-civilian occupation crosswalk candidate. It combines Defense Manpower Data Center mappings with Department of Labor analysis and supplemental COOL data. | Pin the retrieved release, checksum it, retain attribution and license metadata, and show source date and mapping type. Revalidate before each release. |
| [DOD COOL Military Occupations Explorer](https://www.cool.osd.mil/research-military-occupations.htm) | Primary manual reference for military occupations and related credentials. | Automated access returned `403` during the 2026-07-18 review. Do not make this page a runtime dependency; use manual review and the downloadable COOL-derived O*NET data where permitted. |
| [MOS Directory](https://mos.directory/) | Independent six-branch directory covering MOS, Rating, AFSC, and SFSC codes, skills, certifications, and civilian role families. | Use for coverage comparison and plain-language UX research. Verify every mapping against official data because the site is not government-affiliated. |
| [Military Money MOS Lists](https://www.militarymoney.com/careers/mos-lists/) | Broad secondary MOS taxonomy and terminology reference. | The page was last updated in 2022. Use only for gap discovery and historical terminology; do not import mappings or current claims without primary-source confirmation. |
| [From Service to Sector](https://coeccc.net/from-service-to-sector-mapping-military-skills-to-civilian-careers/) | 2025 regional labor-market study showing a skills, duties, qualifications, wage, and demand mapping method. | Use its method and evaluation questions, not its San Diego and Imperial results as national claims. Revalidate labor-market data by region and date. |
| [Best Military Resume MOS-to-Civilian Chart](https://bestmilitaryresume.com/blog/career-transition/mos-to-civilian-job-chart-all-branches-2026) | Current commercial comparison for occupation translation, salary presentation, federal series, and resume workflow. | Treat mappings, salaries, testimonials, and product claims as untrusted comparison inputs. Verify them against O*NET, BLS, official federal data, and user evidence. |
| [Military Transition Toolkit MOS Translator](https://www.militarytransitiontoolkit.com/mos) | Independent comparison for six-branch navigation, career paths, certifications, salary display, and transition sequencing. | The site states that it is not government-affiliated. Use for coverage and UX comparison only; verify mappings, salary figures, eligibility, and benefit guidance independently. |
| [VetSec Remote Security Companies](https://github.com/VetSec/companies-hiring-security-remote) | Community-curated discovery seed for remote security employers. | The repository is a fork and warns that company remote posture changes. Verify every employer and opening against an official careers source before display; never represent the list as veteran-specific hiring proof. |
| [VetSec AI/ML Resume Guides](https://github.com/VetSec/AI-ML/tree/main/resume/ChatGPT) | Veteran-focused resume and cybersecurity-transition comparison material. | Use for adversarial and UX evaluation only. Do not copy its prompts into product behavior, infer tools or credentials, or send resume content to ChatGPT by default. JobSentinel requires user-confirmed evidence and its privacy-first AI gateway. |

Evaluation requirements:

- Normalize Army and Marine Corps MOS, Navy and Coast Guard Rating, Air Force
  AFSC, and Space Force SFSC identifiers without collapsing branch meaning.
- Present multiple plausible civilian paths with provenance, mapping type,
  date, uncertainty, and the user evidence needed to support each path.
- Never infer veteran status, service history, discharge character,
  preference or benefit eligibility, certifications, current clearance, or
  civilian-tool experience from an occupation code.
- Preserve exact user wording and evidence before proposing civilian-friendly
  language. A crosswalk can suggest investigation, not authorize a resume claim.
- Keep employer, salary, credential, federal-series, and policy data volatile.
  Refresh or clearly date it and provide a manual verification path.
- Keep resume content local by default. External AI remains optional,
  redacted, previewed, approved, and gateway-routed.
- Include fixtures for multiple branches, ranks, occupation-code variants,
  direct and skill-related mappings, no-match cases, stale sources, and users
  who do not want to disclose veteran status.

### 5. Safety Response Center

Move beyond "this may be a scam" into safe action:

- verify official domain;
- check if the job appears on the company careers page;
- identify suspicious payment, check, equipment, messaging, and identity
  requests;
- preserve evidence locally;
- show reporting links where appropriate;
- mark the role as avoid, verify, or proceed with caution.

## Evaluation Metrics To Add

Add these to v3 benchmark work:

- monthly cost to complete a serious search;
- number of core workflows blocked by a paid tier;
- number of workflows requiring account creation;
- private resume, notes, salary, or application history sent outside the device
  by default;
- time to first useful role decision;
- percent of recommendations with concrete evidence;
- percent of risky posts with safe next steps;
- user confusion rate during first-run setup;
- support success for modest hardware and limited internet;
- export completeness and rollback recovery;
- regional coverage confidence by source class, not only country label.

## Cut Lines

Do not use this research to justify:

- hidden restricted-source monitoring;
- stored restricted-source cookies, browser storage, auth headers, or session
  tokens;
- final application submission without user review;
- unsupported claims that JobSentinel predicts hiring outcomes;
- fake resume claims, hidden keywords, or keyword stuffing;
- fear-based copy that pressures the user;
- uploading private user data to measure community trends;
- hard-coded law lists without a dated policy source.

## Sources

- [FTC Job Scams](https://consumer.ftc.gov/all-scams/job-scams)
- [Pew Research Center: U.S. workers and future AI use](https://www.pewresearch.org/social-trends/2025/02/25/u-s-workers-are-more-worried-than-hopeful-about-future-ai-use-in-the-workplace/)
- [National Skills Coalition: Closing the Digital Skill Divide](https://nationalskillscoalition.org/resource/publications/closing-the-digital-skill-divide/)
- [LinkedIn Premium Career](https://premium.linkedin.com/careers/career)
- [Teal pricing](https://www.tealhq.com/pricing)
- [Huntr plan types and pricing](https://help.huntr.co/en/articles/10714568-plan-types-and-pricing)
- [Jobscan Premium overview](https://www.jobscan.co/video-jobscan-premium)
- [Google JobPosting structured data](https://developers.google.com/search/docs/appearance/structured-data/job-posting)
- [GOV.UK Find a job](https://www.gov.uk/find-a-job)
- [EURES services](https://eures.europa.eu/eures-services_en)
- [India Directorate General of Employment: National Career Service](https://dge.gov.in/ncs)
- [JAN Job Seeking](https://askjan.org/topics/Job_Seeking.cfm)
- [NWLC pay transparency analysis](https://nwlc.org/press-release/pay-transparency-laws-are-most-effective-when-they-require-salary-ranges-in-job-listings-nwlc-analysis-of-glassdoor-data-reveals/)
- [JobSentinel job-seeker behavior research](../../research/job-seeker-behavior.md)
- [JobSentinel ghost-jobs research](../../research/ghost-jobs.md)
- [JobSentinel pay-equity research](../../research/pay-equity.md)
