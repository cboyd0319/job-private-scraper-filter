# Employer Intelligence

Last updated: 2026-06-21.

Employer intelligence is the v3 plan for helping users understand a company
before they spend time applying, interviewing, or negotiating. It should answer
plain user questions with sourced evidence, not create an opaque company rating
or a review-site clone.

This is candidate-side intelligence. It exists to protect user time, privacy,
salary floors, and safety.

## Product Goal

JobSentinel should help a user answer:

- Is this employer real and reachable through an official domain?
- Is this specific role visible on an official employer or ATS source?
- Does the role show useful pay information?
- Has the same role been reposted, changed, closed, or duplicated?
- Are there recent official signals that may affect hiring risk?
- Does this employer have public history relevant to visa sponsorship?
- What happened the last time I applied to or interviewed with this employer?
- What should I ask the recruiter, hiring manager, or interviewer?
- What do I still not know?

The answer should always separate:

- observed fact
- user-owned history
- source confidence
- uncertainty
- next action

## Source Hierarchy

Use a source hierarchy instead of treating every company signal equally.

| Tier | Source class | Examples | V3 handling |
| --- | --- | --- | --- |
| 1 | User-owned local data | Applications, notes, outcomes, interview dates, offers, contacts, saved recruiter claims | Local-only and highest relevance for this user. Never shared by default. |
| 2 | First-party employer sources | Company domain, careers page, ATS tenant, official job detail, official press or investor page | Preferred source for role existence, company identity, and apply route. |
| 3 | Official public registries and datasets | SEC EDGAR, WARN/state layoff sources, DOL OFLC data, USCIS H-1B data, Companies House, EU business registers, India company datasets | Evidence with provenance, date, jurisdiction, and coverage warning. |
| 4 | Public job and structured-data sources | Greenhouse, Lever, Ashby, Workday, SmartRecruiters, schema.org JobPosting, sitemap, RSS, public boards | Use source graph policy, rate limits, fixtures, and freshness metadata. |
| 5 | User-provided third-party context | User-pasted links, user-written notes, salary/review-site observations, recruiter screenshots the user chooses to summarize | Store as user notes or references, not as harvested background data. |

This hierarchy keeps v3 useful without overclaiming. A user note that says
"recruiter said remote is allowed" is valuable, but it is not the same as a
written job posting. An old H-1B record can show historical sponsorship
activity, but it cannot prove current sponsorship for this role.

## Data Dimensions

### 1. Employer Identity

Track:

- canonical employer name
- aliases and brands
- official domains
- careers page URLs
- ATS tenant identifiers
- known parent or subsidiary relationships when sourced
- region and jurisdiction hints
- source confidence and last verified date

User value:

- avoids fake-company and impersonation traps
- improves duplicate detection across boards
- lets company watchlists work across ATS migrations and branded subsidiaries

### 2. Role Existence And Hiring Reality

Track:

- first seen and last seen
- official employer or ATS source status
- public-board copies
- repost lineage
- closed or unavailable status
- title and location changes over time
- apply-link destination and token-scrubbed canonical URL
- source confidence

User value:

- shows when a role is official enough to spend time on
- distinguishes an active employer posting from a copied or stale listing
- gives ghost-job review stronger evidence without claiming employer intent

### 3. Pay Clarity

Track:

- listed range
- currency
- pay period
- location basis
- hourly, salary, contract, commission, equity, or mixed compensation
- range width
- source of pay data
- whether the range satisfies the user's saved floor
- policy pack and review date if legal guidance is shown

User value:

- flags missing or unusably broad ranges
- protects salary floors before the user invests effort
- supports offer review with written-versus-verbal separation

### 4. Stability And Business Risk Signals

Use official or first-party sources where possible:

- SEC company facts, filings, and submissions for public companies
- official investor pages and press releases when the employer publishes them
- WARN guidance and state layoff data where available
- official company register status in supported regions
- user-owned outcome history such as repeated no-response loops

Do not present these as financial advice or employer verdicts. Show them as
"signals to review" with dates and links.

User value:

- helps users ask better questions
- prevents surprise when a company has recent restructuring signals
- supports safer offer decisions without pretending to predict business health

### 5. Work Authorization And Sponsorship Context

Track historical public records separately from current role promises:

- DOL OFLC disclosure records
- USCIS H-1B employer data where available
- user-entered recruiter statements
- job-post text about sponsorship, authorization, or relocation

User value:

- helps users decide whether to ask sponsorship questions early
- avoids treating a company with past filings as a guaranteed sponsor
- supports regional work-authorization packs without hard-coded assumptions

### 6. Process Quality And User Outcomes

Track only local user-owned history:

- applications submitted
- resume variant used
- interview stages
- response time
- recruiter and contact notes
- offer and rejection outcomes
- ghosting and stale follow-up events
- user sentiment or lessons learned, if the user records them

User value:

- turns personal experience into reusable strategy
- helps users decide whether to reapply, follow up, or deprioritize
- avoids a central review database that would create privacy and moderation risk

### 7. Scam And Impersonation Safety

Use FTC, FBI, and local evidence patterns to support safe next steps:

- unofficial or mismatched domain
- suspicious payment or equipment checks
- early requests for sensitive identity data
- interviews moved to unusual channels without an official trail
- pressure to act before verification
- job not found on official employer sources

Do not collapse every weak posting into "scam." Keep separate labels:

- needs official-source verification
- possible impersonation
- possible scam sign
- stale or low-confidence posting
- poor fit or low value

## Product Surfaces

### Employer Dossier

A dossier is the user-facing company page inside JobSentinel.

It should include:

- employer identity and aliases
- official domains and careers pages
- active saved jobs
- official-source status
- pay clarity summary
- recent source changes
- user-owned application history
- known contacts and outreach notes
- interview and offer history
- public registry signals with dates
- risk and uncertainty section
- next questions to ask

The dossier should not show a single "company score." A score would hide too
many assumptions. Use evidence cards and next actions instead.

### Employer Watchlist

The watchlist is for public and official sources:

- user chooses target employers
- JobSentinel checks approved public source paths
- restricted or account-backed sources stay browser-companion only
- changed jobs, new roles, closed roles, and pay changes become local events
- source failures show plain repair actions

### Company Relationship Timeline

The timeline should answer "what has happened with this employer?"

Events include:

- job first seen
- saved
- applied
- follow-up created
- interview scheduled
- recruiter note added
- offer received
- rejection received
- role closed
- role reposted
- official-source mismatch found

This turns employer intelligence into action history, not a static research
page.

### Job Detail Employer Context

Every job detail page should show a small employer context panel:

- official employer source found or missing
- previous user history with this employer
- pay clarity status
- duplicate or repost lineage
- sponsorship context if relevant to the user
- recent official public signals when available
- "questions to ask" suggestions

### Interview And Offer Context

Interview and offer workflows should use the dossier to generate:

- recruiter questions
- hiring-manager questions
- stability and role-priority questions
- pay and location questions
- written-versus-verbal confirmation prompts
- sponsorship or work-authorization questions when relevant
- offer-risk notes tied to dated evidence

## Architecture Notes

Employer intelligence should be built on the v3 source graph and event ledger,
not a separate data island.

Candidate entities:

- `employers`
- `employer_aliases`
- `employer_domains`
- `employer_source_links`
- `employer_dossier_signals`
- `employer_user_events`
- `employer_policy_records`
- `employer_source_snapshots`
- `employer_question_prompts`

Every non-user signal needs provenance:

- source class
- source URL or dataset id
- retrieved date
- observed date when different
- jurisdiction
- confidence
- expiration or refresh rule
- license or policy note
- parser or adapter version
- user-visible explanation

Derived views should be rebuilt from the ledger where practical. Do not make
the dossier the only copy of evidence.

## Privacy And Security Boundaries

Employer intelligence must preserve Rule 0:

- Keep user applications, notes, contacts, salary floors, interviews, and offers
  local by default.
- Do not upload user outcome history to build community employer ratings.
- Do not harvest review sites, salary sites, or restricted account-backed pages
  in the background.
- Do not store restricted-source cookies, headers, browser storage, or session
  material.
- Do not present legal, financial, immigration, or employment advice.
- Do not label an employer as fraudulent without evidence and plain uncertainty.
- Do not expose private company interests through hidden network calls.

Public lookups still disclose the queried employer to the destination source.
The UI should make this plain when a user runs a direct lookup. For common
official datasets, v3 should evaluate downloaded or bundled public-data packs
where licensing, size, update cadence, and privacy make sense.

## Regional Starter Framework

V3 should start with a U.S. path and create a framework for UK, EU, and India.
The framework should avoid pretending full global coverage exists.

### United States

Starter sources:

- SEC EDGAR for public-company facts, filings, and submissions
- DOL OFLC disclosure data for labor certification and LCA context
- USCIS H-1B employer data where available
- WARN guidance plus state workforce layoff sources where available
- state salary-transparency policy packs with dated review
- official employer and ATS sources

### United Kingdom

Starter sources:

- Companies House API for company identity and status
- GOV.UK Find a job and public-sector source lanes
- UK regional vocabulary, salary periods, notice periods, and CV conventions
- starter company register provenance model

### European Union

Starter sources:

- EURES and ESCO-adjacent career context
- EU business register interconnection concepts
- pay transparency framework references where region packs support them
- multilingual title, location, salary, and CV fixtures

### India

Starter sources:

- National Career Service context
- India company master data where available
- CTC, LPA, monthly pay, notice period, and work-location patterns
- NCO and NSQF taxonomy bridges

## Metrics And Release Bar

Add employer intelligence to v3 evaluation:

- percent of saved jobs with an official employer or ATS source link
- percent of employers with verified domain or registry evidence
- time to answer "is this role official?"
- percent of employer context cards with source provenance
- pay clarity coverage and range-quality classification
- rate of suspicious roles with safe next steps
- rate of user-owned history reused in decisions
- false-positive rate for scam or impersonation labels
- number of external lookups needed for a dossier
- novice-user task completion for "research this employer"

## Cut Lines

Do not build:

- central employer reviews from user private data
- hidden collection from review sites or restricted sources
- background account-backed employer monitoring
- unsourced employer risk scores
- legal or financial verdicts
- internal hiring-probability claims
- private-data exchange with third-party salary or review services by default
- defamatory labels that exceed the evidence shown

## Gate 4 Operating Decisions

- Use a live-first, source-specific model. The minimum dossier combines local
  user history with user-approved official employer, careers-page, Greenhouse,
  or Lever evidence. It shows role status, pay clarity, provenance date,
  uncertainty, and a next action.
- Add an optional static source pack only when an official bulk dataset lacks a
  suitable live path and its license, size, cadence, provenance, and expiry
  have passed review. Do not build both modes for one source without evidence.
- Keep dataset packs out of Essentials defaults and independently removable.
- Keep review-site observations as dated local user notes. Never publish them
  as ratings or harvest restricted pages.
- Reuse each source manifest's `verified_on + max_age_days` freshness owner.
  Staleness blocks a refresh or marks evidence stale; it does not delete
  historical observations.
- Keep SEC, DOL OFLC, USCIS, WARN, Companies House, EU registry, and India
  company datasets as research candidates until each has source-specific
  policy, license, fixture, parser, and freshness evidence.

These decisions freeze delivery and ownership, not a release claim. Milestone 8
still must implement the dossier and prove source accuracy, conflicts,
freshness, safe uncertainty, and novice task completion.

## Sources

- [SEC EDGAR APIs](https://www.sec.gov/search-filings/edgar-application-programming-interfaces)
- [SEC company facts API](https://www.sec.gov/edgar/sec-api-documentation)
- [DOL WARN Act guidance](https://www.dol.gov/agencies/eta/layoffs/warn)
- [DOL OFLC performance data](https://www.dol.gov/agencies/eta/foreign-labor/performance)
- [USCIS H-1B Employer Data Hub files](https://www.uscis.gov/tools/reports-studies/h-1b-employer-data-hub-files)
- [FTC job scams guidance](https://consumer.ftc.gov/articles/job-scams)
- [FBI IC3 work-from-home scam advisory](https://www.ic3.gov/PSA/2024/PSA240604)
- [Companies House API](https://developer.company-information.service.gov.uk/)
- [EU Business Registers Interconnection System](https://e-justice.europa.eu/topics/registers-business-insolvency-land/business-registers-eu_en)
- [India company master data](https://www.data.gov.in/catalog/company-master-data)
- [OpenCorporates API reference](https://api.opencorporates.com/documentation/API-Reference)
- [Google JobPosting structured data](https://developers.google.com/search/docs/appearance/structured-data/job-posting)
- [JobSentinel source and browser strategy](source-and-browser-strategy.md)
- [JobSentinel job seeker platform research](job-seeker-platform-research.md)
- [JobSentinel job-site data sources research](../../research/job-site-data-sources.md)
- [JobSentinel ghost-jobs research](../../research/ghost-jobs.md)
- [JobSentinel pay-equity research](../../research/pay-equity.md)
