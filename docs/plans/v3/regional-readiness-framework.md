# Regional Readiness Framework

V3 should try to become useful outside the United States without pretending it
can fully solve every country, language, source, form, labor rule, and hiring
custom in one release. The v3 goal is to create the framework and starter packs
for the UK, EU, and India so future releases can deepen coverage without another
architecture reset.

## Scope

V3 should provide:

- region manifest schema
- regional source-pack hooks
- regional taxonomy bridges
- currency, pay-period, and location normalization
- CV and resume format profiles
- region-aware plain-language warnings
- region-aware source policy notes
- starter UK, EU, and India research packs
- tests that prove a region can be added without changing core app code

V3 should not promise:

- complete job-board coverage in every country
- legal, immigration, tax, or financial advice
- automatic form completion for every regional application system
- high-quality non-English drafting without evals
- final v4-level localization coverage

## Region Manifest

Each region pack should declare:

| Field | Purpose |
| --- | --- |
| `region_id` | Stable identifier such as `uk`, `eu`, `india`, or `us-co`. |
| `countries` | ISO country codes covered by the pack. |
| `languages` | User-facing languages the pack can support. |
| `currency` | Default currency and supported alternates. |
| `pay_periods` | Hourly, daily, monthly, annual, contract, stipend, or not disclosed. |
| `location_rules` | City, postcode, region, remote, hybrid, onsite, relocation, and travel norms. |
| `work_authorization` | Labels and warnings, not eligibility decisions. |
| `source_classes` | Official, public, regional board, employer careers, restricted, or user import. |
| `cv_profiles` | Resume, CV, Europass, apprenticeship, public-sector, academic, or local formats. |
| `taxonomies` | Occupation and skills systems mapped by the pack. |
| `policy_notes` | Terms, privacy, source access, and restricted-source notes. |
| `eval_fixtures` | Regional jobs, CVs, pay formats, locations, and parser drift fixtures. |

## Taxonomy Bridge

Regional matching should not depend on U.S.-centric role titles.

V3 should support taxonomy bridges for:

- O*NET and U.S. SOC for U.S. roles.
- UK SOC 2020 for UK occupation grouping and regional role labels.
- ESCO for EU skills, competences, qualifications, and occupations.
- India NCO 2015 and NSQF/NOS concepts for Indian role and qualification
  alignment.
- JobSentinel canonical role families, seniority labels, and skill aliases.

The bridge should preserve provenance:

- source taxonomy
- source code or concept URI when available
- canonical JobSentinel concept
- confidence
- mapping type: exact, close, broader, narrower, or regional synonym
- pack version

## UK Starter Framework

V3 should make the UK possible through:

- GOV.UK Find a job as an official public source research target.
- UK SOC 2020 taxonomy mapping.
- UK pay-period parsing for annual salary, hourly wage, pro rata, day rate,
  contract rate, and apprenticeship wage patterns.
- UK location handling for country, county, city, postcode, remote, hybrid, and
  region names.
- CV profile guidance that does not assume a U.S. resume format.
- Region labels for sponsorship, right-to-work, and public-sector role language
  without making eligibility claims.

Starter deliverables:

- `uk` region manifest.
- UK source research entries.
- UK pay and location fixtures.
- UK CV import and export profile notes.
- UK occupational taxonomy bridge prototype.

## EU Starter Framework

V3 should make EU expansion possible through:

- EURES as an official EU job mobility research target.
- ESCO as the primary EU taxonomy bridge for skills, occupations, and
  qualifications.
- Europass CV profile support, including import/export research for structured
  Europass data where practical.
- Multilingual source and CV fixture strategy.
- Currency handling for euro and non-euro EU countries.
- Cross-border work mode, language requirement, and relocation labels.

Starter deliverables:

- `eu` region manifest.
- ESCO concept ingestion prototype.
- Europass CV profile notes and fixtures.
- EURES source research entries.
- Multilingual hard-negative matching fixtures.

## India Starter Framework

V3 should make India expansion possible through:

- National Career Service as an official Indian public job portal research
  target.
- India NCO 2015 taxonomy bridge.
- NSQF and National Occupational Standards research for qualification-heavy
  roles.
- Location parsing for city, state, metro, remote, hybrid, onsite, and
  relocation patterns.
- Pay parsing for annual CTC, monthly salary, stipend, LPA, and negotiable
  compensation patterns.
- Starter coverage for technical, public-sector, apprenticeship, healthcare,
  education, retail, and operations roles.

Starter deliverables:

- `india` region manifest.
- India source research entries.
- India pay and location fixtures.
- India taxonomy bridge prototype.
- CV/profile guidance notes that avoid U.S.-only assumptions.

## Regional UX

Users should not need to understand taxonomies.

V3 should expose:

- "I am searching in..." setup choice.
- region-specific source suggestions.
- region-specific pay parsing and display.
- region-specific CV or resume options.
- region-specific warnings in plain language.
- "This region pack is starter coverage" labels when coverage is incomplete.
- feedback button for missing sources, formats, or terms.

## Gate 4 Delivery Decisions

- Deliver optional user-selected region data as signed static `Region` packs.
  Static packs receive no executable action, credential, external AI, or
  network authority.
- Keep every live source behind its separate reviewed source manifest and exact
  operation grant. Installing a region pack cannot authorize a request.
- Keep region packs out of Essentials defaults and make them independently
  installable, disableable, removable, and upgradeable in Milestone 9.
- Treat `reviewed_on` as visible provenance for static taxonomy and CV guidance,
  not as source authorization or a universal expiry. Reuse each runtime source
  manifest's evidence-backed `max_age_days`; stale source policy stops network
  actions without deleting dated static guidance.
- Keep GOV.UK Find a job and UK SOC 2020, EURES, ESCO and Europass, and India's
  National Career Service, NCO 2015 and NSQF/NOS as research seeds until their
  regional fixtures and source-specific reviews pass.

These decisions do not label a region pack ready. Current manifests remain
English-only starter research metadata with incomplete-coverage labels.

## Regional Evaluation

Each region pack needs tests for:

- job title parsing
- company and location parsing
- pay and currency normalization
- work mode labels
- source policy class
- CV/resume format handling
- skill and occupation mapping
- Qwen3 matching against regional terminology
- missing-data behavior
- plain-language warning copy

V3 should not claim a region is ready unless the starter pack passes fixtures
and manual review.

## V4 Handoff

V3 should leave v4 with:

- stable region-pack schema
- starter UK, EU, and India manifests
- taxonomy bridge interfaces
- source research records
- fixture strategy
- region-specific UX patterns
- known coverage gaps

V4 can then deepen regional coverage without changing the core architecture.

## References

- [GOV.UK Find a job](https://www.gov.uk/find-a-job)
- [EURES](https://eures.europa.eu/)
- [ESCO web-service API](https://esco.ec.europa.eu/en/use-esco/use-esco-services-api/esco-web-service-api)
- [Europass CV](https://europass.europa.eu/en/create-europass-cv)
- [UK SOC 2020](https://www.ons.gov.uk/methodology/classificationsandstandards/standardoccupationalclassificationsoc/soc2020)
- [National Career Service India](https://www.ncs.gov.in/)
- [India NCO 2015](https://www.ncs.gov.in/documents/national%20classification%20of%20occupations%20_vol%20i-%202015.pdf)
- [India NSQF](https://ncvet.gov.in/national-skills-qualification-framework/nsqf-notification/)
