<!-- Defines supported job-source behavior, governance, and privacy boundaries. -->

# Job Sources

JobSentinel favors official-source job monitoring and public job sources
checked within clear limits. Source access must preserve user privacy, source
boundaries, rate limits, and review-first workflows.

The product stance is help-first informed consent. If a source carries account,
terms, or source-policy risk, JobSentinel should explain that risk plainly and
let the user choose a secure local path when privacy and security guardrails can
still hold. Block only when the requested path would capture secrets, persist
sessions, bypass platform controls, hide background access, violate URL or rate
limits, or create unsafe external side effects.

Restricted job boards are supported only through explicit user-controlled
paths: search links, manual entry, pasted individual job links, Browser Import,
or scheduled checks that show a source-specific warning first. Before risky
source use, JobSentinel must explain in plain language that some sites have
rules about automated tools and that misusing tools can affect the user's
account or create legal/privacy risk. The user can continue only after
acknowledging that warning. JobSentinel does not encourage terms violations,
collect restricted-site login details, save session cookies, bypass human
checks, call private systems, or read restricted pages in hidden background
access.

Restricted authenticated sites have a stricter rule. If JobSentinel opens a
sign-in page for LinkedIn or a similar source, the warning must appear before
that page opens, the action must be started by the user in that moment, no auth
tokens, session cookies, browser storage, or authorization headers may be saved,
and JobSentinel must not reuse that sign-in state later for offline or scheduled
collection. For manual sessions where JobSentinel does not inspect or automate
the site, use a visible privacy reminder instead of forcing the user to stop.
Hard expiry is reserved for any future restricted-source feature that reads or
automates restricted content.

Use the same privacy-reminder pattern for any browser or webview opened by
JobSentinel. The user should be able to close the session or keep going. Do not
force a timeout only because the browser has been open for a while.

Reviewed official hiring sources are low-friction sources. Greenhouse, Lever,
and similar public posting sources should use normal source opt-in, safe
messages, and quality checks, not restricted-source acknowledgements or
authenticated-session time gates.

Do not confuse restricted public boards with authenticated sessions. A job board
can be technically public and unauthenticated while still prohibiting or
restricting automated access. Local user consent never overrides provider
policy. Built In, Dice, SimplyHired, and Glassdoor scheduled automation is
disabled after current first-party policy review. Pasted-URL import for those
domains also stops before transport. Dice's official MCP remains review-required
until privacy, schema, and pacing contracts are reviewed.

The warning must be prominent in the UI and public docs because users should
not need to understand source internals to make an informed choice. The secure
path is the easy path: official feeds first, then user-opened and reviewed
paths. Automation remains disabled whenever source policy does not authorize it.

Legacy acknowledgement booleans remain loadable only for config compatibility.
They are always cleared and cannot authorize transport. Any future source
consent must be local, append-only, and bound to the exact source, operation,
warning, behavior revision, current policy, destination, data categories, and
minimized request fingerprint.

## Source Model

| Category | Sources |
| -------- | ------- |
| Scheduled job checks | Greenhouse, Lever, RemoteOK, WeWorkRemotely, and community hiring posts |
| Source awaiting reviewed access | USAJobs stops before credential access until approved-use evidence is current |
| Disabled legacy sources | JobsWithGPT, Built In, Dice HTML, SimplyHired, and Glassdoor settings remain loadable but cannot authorize transport |
| Source-check helpers | Active scheduled sources plus locally skipped Indeed, Wellfound, and ZipRecruiter availability checks |
| Company careers discovery | Employer careers pages that JobSentinel can classify before choosing a safe source path |
| User-opened search links | LinkedIn, Y Combinator Jobs, and other destination links opened by the user |
| Preferred expansion path | Official company career pages and public hiring-platform sources such as Greenhouse, Lever, Ashby, Workable, SmartRecruiters, and USAJobs |

## Veteran And U.S. Public-Service References

The machine-readable reviewed index is
`crates/jobsentinel-domain/src/fixtures/v3_veteran_public_service_index_v1.json`.
It is intentionally incomplete. Inclusion means only that JobSentinel has
recorded a bounded use for the reference. It does not mean complete coverage,
current employer hiring, veteran-friendly status, civilian equivalence,
qualification, preference, protected veteran status, or federal eligibility.

| Reference | Reviewed use | Runtime boundary |
| --------- | ------------ | ---------------- |
| [USAJOBS API](https://developer.usajobs.gov/guides/authentication) | Official federal job search | User-provided API key and registered email; secure local credential storage |
| [O*NET Military Crosswalk](https://www.onetcenter.org/crosswalks.html) | Government-sponsored occupation linkage suggestions | Manual review only; August 2024 source and [O*NET Web Services external-data license exclusions](https://services.onetcenter.org/help/license_data) recorded |
| [Department of Defense COOL](https://www.cool.osd.mil/research-military-occupations.htm) | Occupation and credential research | Manual review only |
| [OPM Veteran Job Seekers guidance](https://www.opm.gov/fedshirevets/veteran-job-seekers/vets/) | General federal veteran-hiring guidance | Manual review only; never an individual eligibility decision |
| [Department of Labor VEVRAA guidance](https://www.dol.gov/agencies/ofccp/faqs/vevraa) | General protected-status guidance | Manual review only; answers remain voluntary and user selected |
| [VetSec remote security employer list](https://github.com/VetSec/companies-hiring-security-remote) | Community employer-discovery seed | Manual review only; verify each official careers site |
| [VetSec resume prompt examples](https://github.com/VetSec/AI-ML/tree/main/resume/ChatGPT) | Evaluation reference | Manual review only; never an external-AI default or approval bypass |
| [MOS Directory](https://mos.directory/) | Community occupation comparison | Manual review only |
| [Military Money MOS lists](https://www.militarymoney.com/careers/mos-lists/) | Older commercial occupation comparison | Manual review only; page showed a September 30, 2022 update date |
| [COECCC service-to-sector mapping](https://coeccc.net/from-service-to-sector-mapping-military-skills-to-civilian-careers/) | California San Diego and Imperial regional workforce research | Manual review only |
| [Best Military Resume MOS chart](https://bestmilitaryresume.com/blog/career-transition/mos-to-civilian-job-chart-all-branches-2026) | Commercial occupation comparison | Manual review only |
| [Military Transition Toolkit](https://www.militarytransitiontoolkit.com/mos) | Commercial occupation comparison | Manual review only |

Occupation, credential, and sector mappings are suggestions only. JobSentinel
may use them to help a user find civilian language, but every resulting duty,
tool, credential, date, clearance, achievement, and qualification must come
from user-confirmed evidence. Service history, federal hiring eligibility, and
protected veteran status remain separate concepts. JobSentinel never derives
one from another.

## Boundaries

| Rule | Requirement |
| ---- | ----------- |
| Official source first | Prefer official posting sources, public feeds, and company or application-platform postings |
| Restricted-site user gate | Warn before user-directed restricted-site import or search-link open. Browser Import requires a native per-site confirmation and exact persisted pairing authority; no acknowledgement can override a provider automation prohibition |
| Technical auth classification | Keep public unauthenticated sources, local API-key sources, authenticated user-session sources, and unknown review-required sources distinct in the shared source taxonomy |
| Restricted-domain rationale | Every domain in `RESTRICTED_JOB_SOURCE_DOMAINS` must come from a structured record with a specific reason, category, and source reference; do not add undocumented domains |
| No secret capture or evasion | Do not add hidden data paths, session-cookie collection, human-check workarounds, or platform-control evasion |
| Local-first storage | Source results, run history, and notes stay local |
| Rate limits | Every source check must wait within that source's limits |
| Response size | JobSentinel stops reading very large responses; the current safety limit is 16 MiB |
| User control | Job-site search links open in the user's browser; restricted-site actions stay user-directed and policy-bound |

## Smart Paste

Smart Paste turns copied job text and an optional public HTTPS job link into a
local review draft. It does not fetch the page, use external AI, inspect the
clipboard automatically, or read screenshots. The user pastes the text,
reviews and edits title, company, location, and link, then confirms the exact
draft before local storage.

Pasted text and review edits are rejected before queueing if they contain
password, token, cookie, authorization, or session-like material. Incomplete
text remains an unsaved review draft until the user supplies the required
fields. Screenshot and OCR import remain outside the v3.0 Smart Paste boundary.

## User-Controlled LinkedIn Workbench

The LinkedIn-compatible flow is user-controlled activity capture, not a
scheduled scraper. It is reachable from Dashboard quick actions and Settings:

1. The user starts the LinkedIn session from JobSentinel.
2. JobSentinel shows a trusted native review with short, friendly copy about
   what it can help with. The exact review is stored in the append-only local
   source consent ledger.
3. The user signs in and uses LinkedIn directly.
4. JobSentinel shows local controls beside the browser: save, applied, track,
   rejected, interview, follow up, reminder, note, not interested, and paste
   details.
5. JobSentinel stores only user-entered or user-selected details and
   user-confirmed local records.
6. Local analysis runs after the record exists in JobSentinel.

`Log applied` should be a one-click action. If title, company, or link are not
known yet, create a draft application record with `Needs details` fields and
prompt the user to finish it later. Optional details should come after the
click, not before it.

LinkedIn prefill is allowed only from pasted job links, pasted details,
selected text the user sends to JobSentinel, or previously confirmed local
records. Current LinkedIn policy prohibits third-party page capture, so
JobSentinel blocks LinkedIn Browser Import before anything enters the review
queue. Do not prefill by reading network traffic, browser storage, hidden page
state, screenshots, or background pages. Prefilled values remain suggestions
until the user confirms them. Pasted Workbench notes must remove session-like
URL query fields, cookies, and token-like fields before local storage.
When a user pastes selected text into the Workbench, JobSentinel may fill the
suggestion fields immediately from that pasted text.

### Assistive Capture Model

JobSentinel should reduce spreadsheet-style work without becoming a hidden
scraper. The release-safe model is assistive capture:

- **Capture only where current provider policy permits it.** A user-clicked
  Browser Import action may read the visible posting on a reviewed supported
  page. Policy-blocked domains fail closed.
- **Queue observations locally.** Captured jobs become local records for review,
  scoring, ghost-job checks, reminders, and resume tailoring.
- **Keep applied logging narrow.** The generic **I Just Applied** browser action
  reads only the visible title, company, and public page address, then queues a
  local applied draft with missing details marked. It does not read the
  description or structured page data.
- **Keep actions explicit.** Applied, saved, tracking, rejected, interview,
  follow-up, reminder, note, and not-interested events must come from a
  JobSentinel control, a visible user-approved import, or another explicit user
  action.
- **Never read hidden state.** Do not inspect restricted-site cookies, browser
  storage, authorization headers, network traffic, hidden pages, screenshots, or
  background tabs.
- **Make capture visible and reversible.** Any future watch-and-learn mode must
  show that it is on, say what it records, keep data local, provide an off
  switch, and require user confirmation before durable application records are
  created.

The current Workbench learning control starts off. When the user turns it on,
JobSentinel keeps a small local list of the Workbench buttons the user clicks
plus the reviewed job title and company. It shows reviewable suggestions and
has a clear action for deleting the learned signals. It does not store notes,
full links, cookies, browser storage, screenshots, hidden fields, or network
details.

Broader interest learning should continue to use local JobSentinel signals
first: Browser Import captures, saved jobs, manual ledger events, search terms,
dismissed jobs, notes, profile preferences, and user ratings. For restricted
authenticated sites, JobSentinel may learn from user-clicked visible-page
captures and JobSentinel-side actions, but not from silent page observation.
Any watch-and-learn mode must be visibly on, explain what it records, stay
local, have an off switch, and never submit applications or create durable
application records without user confirmation.

Third-party scraping frameworks must be evaluated against these boundaries
before adoption. Libraries that add browser fingerprint impersonation, proxy
rotation, hidden browser sessions, challenge solving, persistent cookies,
automatic redirect following, uncapped response reads, or broad crawling do not
fit the scheduled source-check model. A helper library may be considered only
when JobSentinel still controls which pages are read, how often they are read,
how much page data is accepted, and what user-safe message appears when the
source blocks access.

## Company Careers Discovery

Many strong employer postings never appear cleanly on stock job boards. A user
should be able to paste or open an employer careers page, such as a company
careers URL, and have JobSentinel classify it before deciding what to do next.

The discovery order is:

1. Detect official hiring-source signals such as Greenhouse, Lever, Ashby,
   SmartRecruiters, Workable, Workday, Amazon Jobs, large employer career
   pages, iCIMS/Jibe, Breezy, JazzHR, Bullhorn, Eightfold, Jobvite,
   Teamtailor, Recruitee, Taleo, SAP SuccessFactors, Oracle Recruiting, or
   Phenom.
2. Normalize to a native scheduled source only when the public source path and
   source terms are reviewed.
3. Offer a user-opened search link, Browser Import, or manual entry when the
   employer page is custom, restricted, blocked, or still under review. Offer
   pasted job link import only when current provider policy permits an
   automated fetch.

Examples from the 2026-06-19 source pass:

| Employer page | Discovery result |
| ------------- | ---------------- |
| Fivetran careers | Greenhouse board `fivetran`; keep the employer job links shown by the source |
| Klaviyo, Faire, and Mindgruve careers | Greenhouse hiring-source paths verified |
| Primer AI Greenhouse board | Greenhouse board `primerai` on the current `job-boards.greenhouse.io` host |
| SpaceX careers | Custom page, but public Greenhouse board `spacex` exists behind it |
| OpenAI careers | Ashby hiring-source path behind the employer page |
| Anthropic careers | Greenhouse hiring-source path behind the employer page |
| Optiv careers | Workday hiring-source path |
| Amazon Jobs | Employer-owned public job-search path |
| GitHub careers | iCIMS/Jibe public career-site signals |
| Dell Technologies and Albertsons careers | Oracle Fusion Candidate Experience JSON validated where robots allow |
| Valero Energy and Enterprise Products Partners careers | Public Taleo career-section listings validated where robots allow |
| Sysco and Chevron careers | Radancy/TalentBrew public listing paths validated where robots allow |
| Remote First Jobs | Remote jobs source with attribution requirements |
| Large employer company pages | Open in the user browser while native source support is reviewed |
| Naukri, Bayt, and BDJobs | Regional candidate sources that need source-specific review before native checks |
| Google Jobs Search | User-opened discovery and visible-result import candidate, not scheduled search scraping |
| LinkedIn company jobs | User-gated restricted discovery only; no silent scheduled discovery or session capture |
| LinkedIn Jobs Tracker | User-gated restricted tracking only for saved or applied jobs; no silent scheduled discovery or session capture |

Additional local API research from the Fortune 100 source registry added public
API foundation and employer-career lanes to the shared taxonomy: Greenhouse,
Lever, SmartRecruiters, Workday CXS listing JSON, Phenom widget refineSearch
JSON, Radancy/TalentBrew public HTML fallback, Oracle Fusion Candidate
Experience JSON, and public Taleo career-section listings. The native Rust
source-adapter layer now carries canonical record and parser contracts for the
validated Workday CXS, Phenom widget, and Radancy/TalentBrew static HTML lanes,
but live scheduled fetching still requires each employer tenant to pass
JobSentinel's source-specific policy, robots, rate-limit, endpoint-stability,
and parser checks.

The same research also added long-tail ATS fingerprints for platforms such as
ClearCompany, Dayforce, Avature, JobDiva, CEIPAL, Crelate, TrackerRMS,
Vincere, ApplicantPro, ApplicantStack, Homerun, Manatal, Recruit CRM, Loxo,
HiBob, Factorial, JOIN, Polymer, and Recooty. Those long-tail entries are
source-intelligence and routing metadata, not native scrapers.

## How Job Checks Work

```text
Source the user turned on
  -> confirm the source is allowed
  -> wait within that source's limits
  -> read public job details
  -> remove duplicates
  -> store locally
  -> record safe status details
```

First-run setup can suggest job sources before the user saves the search setup, but it
contacts only sources the user checks in review. If no outside source is
selected, the review summary says only local search settings are saved. Source
checks do not receive resumes, private notes, saved answers, application
history, or unrelated profile details.

## Source Status And Help

The job source status view tracks source status without requiring users to
understand website internals, saved connection details, or logs.
For sources that can be enabled from saved search settings, status toggles must
update the saved source settings as well as source status. Source-check-only
helpers that need a company URL, feed approval, access code, or other setup
remain setup checks until the user completes that setup in Settings.

| Surface | Purpose |
| ------- | ------- |
| Summary stats | Healthy, degraded, down, disabled, and unknown source counts |
| Source table | Current status, plain recent-result label, jobs found, last checked, and next step |
| Check history | Recent source attempts with timing and sanitized issues |
| Source checks | Availability checks for known supported sources |
| Help steps | Plain-language next steps and sanitized support report help |

Source status must never leak credentials, raw cookies, full links containing
sensitive parameters, private notes, salary floors, resumes, or application
history.

## Source Check Pace

Representative source pacing:

| Source | Check pace | Access pattern |
| ------ | ------------- | -------------- |
| Greenhouse | Paced, at most 1,000/hour | Official public Job Board API |
| Lever | Paced, at most 1,000/hour | Official public Postings API |
| USAJobs | High | Official source with user-provided access code |
| RemoteOK | Medium | Public job feed |
| Who Is Hiring community thread | Medium | Public community posts through Algolia HN Search |
| WeWorkRemotely | Moderate | Public feed/page |
| JobsWithGPT | Disabled | Provider endpoint and usage-policy review required |
| Built In, Dice HTML, SimplyHired, Glassdoor | Disabled | Provider policy does not authorize the retired adapters |

Checks that cannot operate within source boundaries should fail closed and
show a clear user-facing explanation.

YC Startup Jobs is user-opened only. A 2026-07-19 review of Y Combinator's
current Terms of Use found an explicit prohibition on scraping, robots, data
mining, and similar extraction. JobSentinel therefore does not schedule,
recommend, probe, fetch, import, or capture YC pages.

Source Status keeps Indeed, Wellfound, and ZipRecruiter connectivity helpers
locally skipped. Built In, Dice, SimplyHired, and Glassdoor health rows are
removed by migration and cannot be restored. A direct legacy check identifier
still stops locally with the provider-policy explanation. Restored or
hand-edited config cannot re-enable transport. Users can instead choose a
user-opened search link, Browser Import, an official employer page, or manual
entry. Pasted-URL import for these four retired domains also stops before
transport.

USAJOBS scheduled and connectivity checks are currently review-required and
stop before JobSentinel reads the saved access code. The current terms limit API
data to the registered consumer's approved use and prohibit derivative works.
JobSentinel has not established that every user's registration covers
normalized local persistence, so the manifest keeps automation fail-closed
until that approved-use boundary is represented by dated evidence. This is a
provider-policy boundary, not a legal conclusion.

Greenhouse and Lever scheduled and connectivity checks require exact persisted
Public ATS manifests and policies before network access. Each manifest binds
the public GET API prefix, reviewed synthetic parser fixture, hash-bound policy
review record, current policy revision, robots review, and a paced local
1,000-request-per-hour ceiling with burst one and no automatic retries. That
ceiling is JobSentinel policy, not a provider-published GET allowance.
Greenhouse uses only its documented public Job Board API. It no longer switches
to hosted-board HTML after an API failure or empty result. Lever uses only its
public Postings API and never submits an application. Normalized postings
remain local and are not sent to external model training. Missing, stale,
disabled, or drifted governance stops both the scheduled source and its fixed
connectivity check before network access.

All six governed API, feed, community, and public ATS manifests use the same
pure source simulator. It calls the production authorization contract, requires
every declared parser and policy fixture exactly once, hashes the supplied bytes,
and reports the action decision, manifest and review expiry, and risk-note
references. Missing, changed, extra, or duplicate fixtures produce the
`parser_drift` stop condition. Simulator checks use only committed synthetic
fixtures and do not contact a source.

RemoteOK scheduled and connectivity checks use the same exact persisted-policy
and manifest gate before network access. The reviewed first-party API notice
requires attribution and a followed link to the RemoteOK listing and prohibits
unapproved logo use. The reviewed robots policy allows ordinary public paths
with a one-second crawl delay, reserves model-training use, and excludes query
endpoints; named AI crawler groups also exclude user profiles. JobSentinel uses
only the exact `/api` feed for a local user-directed job search, never sends it
to model training, paces the authorized 500-request-per-hour rate without a
multi-request burst, and does not retry failed requests automatically.

We Work Remotely scheduled and connectivity checks use only its
[advertised public RSS feeds](https://weworkremotely.com/remote-job-rss-feed).
That page permits anyone to populate a remote-job feed when links are
attributed back to We Work Remotely. JobSentinel preserves the source label and
canonical WWR listing link, keeps normalized results local, and does not send
feed content to external model training. The separate partner API is not used.
Its token requirement and terms do not authorize JobSentinel's job-search
workflow. The exact persisted manifest and policy gate every RSS request, map
legacy category settings to reviewed feed URLs, reject all other categories,
pace one request per hour with burst one, and disable automatic retries. Policy
drift, unexpected access controls, attribution loss, parser drift, and stale
review stop the source before network access.

Who Is Hiring community checks use the
[Algolia HN Search API](https://hn.algolia.com/api) to locate only the monthly
thread posted by the `whoishiring` account. JobSentinel then reads the exact
numeric thread item and treats only its direct replies as job posts, so nested
discussion is not imported as listings. The exact persisted manifest and
policy gate the source action before either request, and the health check
validates both reviewed endpoints. Checks use a paced 500-request-per-hour
policy with burst one and no automatic retries. JobSentinel preserves canonical
community-comment links, keeps normalized results local, and does not export
the raw community corpus or send it to external model training. Schema drift,
service retirement, or lost attribution fails the current check without
storing mismatched records. Stale review or policy change stops the source
before network access, and malformed individual replies are skipped.

## Debug And Release Verification

Every source JobSentinel uses must have release evidence before JobSentinel
claims that source is ready:

- Native source checks need tests that prove JobSentinel can read expected job
  details, wait politely, handle no results or source trouble, and show safe
  status details.
- User-gated restricted paths need tests for warning visibility,
  acknowledgement, disabled actions before acknowledgement, and no credential
  or session-cookie capture.
- Live or manual source checks must be opt-in, low-volume, and recorded with
  date, source, platform, result, and user-safe recovery guidance.
- If a source blocks access, JobSentinel should show a plain next step:
  try later, use a search link, import one job from the browser, open the
  employer career page, or add the job manually.

## User-Approved Job-Source Feeds

JobsWithGPT scheduled contact is disabled. The legacy terms URL now redirects
to a differently named provider, and the reviewed first-party terms describe a
website but do not identify an exact feed endpoint, client authorization, or
request limits. An exact local payload approval cannot override that unresolved
provider-policy boundary. JobSentinel stops before writing a request attempt or
sending saved search preferences.

Settings may retain a configured endpoint, prior exact approval, and minimized
historical contact records locally so users can inspect or remove them. The
provider must not be re-enabled until a dated manifest verifies the exact
endpoint, permitted client behavior, pacing, and stop conditions. Re-enabling
would also require a current exact user approval for the saved job titles,
location, remote preference, and result limit. Any change would revoke that
approval. Do not send resumes, salary floors, private notes, application
history, screening answers, or unrelated profile details to a job-source feed.

For any reviewed feed, JobSentinel must write minimized request metadata before
the approved request. If that local audit write fails, nothing is sent. The
request is attempted once without automatic retries, and interrupted or
incomplete terminal records remain visible as uncertain attempts. Settings
shows the latest contact attempt as local status details only: attempt time,
website, count-only request categories, and outcome. The contact history must
not store raw titles, raw location, resumes, salary floors, private notes,
application history, or full source links.
The contact summary also names sensitive data that was not sent so users can
verify that resumes, salary floors, private notes, application history, and full
source links stayed out of the source request.

Source Status does not make a separate JobsWithGPT connectivity request.
Scheduled request history is the status owner, which avoids unreviewed or
repeated provider contact.

## Duplicate Handling

JobSentinel cleans posting links, titles, and locations before checking for
matching records.
This reduces duplicate postings across hiring-platform feeds, job boards,
social shares, and company pages.

User-imported job links are cleaned before preview, duplicate checks, and
storage. The importer removes embedded credentials, fragments, tracking
parameters, and sensitive query parameters while preserving public posting
identifiers needed to recognize the posting.
When the import preview includes posting pay, it labels it as listed pay so the
user treats it as source evidence to review rather than a guaranteed salary.
When the import preview does not include posting pay, it says listed pay is not
shown and asks the user to verify pay before tailoring.
When the posting includes a closing date, the import preview shows it before
the user saves the job. Previewed posting and closing dates preserve the source
date instead of shifting a day earlier in local time zones. If a source returns
a malformed posting or closing date, the preview shows **Date not shown**
instead of displaying a raw date-parsing error.

JobSentinel compares cleaned title, company, location, and link to spot
duplicates.

Normalization removes common tracking parameters, standardizes common title
abbreviations, and maps location aliases such as `SF`, `Remote US`, and
`work from home` to canonical forms.

## Privacy Labels

| Feature | Labels |
| ------- | ------ |
| Job tracking | Local only |
| Saved searches | Local only |
| Scheduled job checks | Local only, public-data only unless the user turns on an external access code such as USAJobs |
| Source status | Local only |
| User-opened search links | Local only, user-controlled browser action |
| Optional support report | Local only until user chooses copy, save, or GitHub issue flow; sanitized by default |

## Expansion Checklist

- Use official posting sources where available.
- Confirm source terms, robots policy, and practical access boundaries.
- Check sources politely and avoid reading more page data than needed.
- Keep new source checks inside JobSentinel's existing safety path so link
  checks, redirect handling, page-size limits, retry behavior, and safe support
  details stay consistent.
- Add source status and user-safe errors.
- Do not add hidden data paths, session-cookie collection, human-check workarounds,
  or evasion of platform controls.
