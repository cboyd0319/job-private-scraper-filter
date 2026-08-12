# Job-Site Data Sources Research

This brief condenses research and source-governance guidance for job monitoring.

## Key signals

- Public ATS feeds and official employer pages are better sources for freshness
  and closure checks than copied aggregator listings.
- Source terms, robots rules, access controls, privacy limits, and rate limits
  are product constraints.
- Single-user local monitoring is different from operating a public job board or
  reselling job data, but it still needs careful source review.
- Provenance matters. Users need to know where a job came from and how current
  it may be.

## Reviewed source decisions

### Y Combinator Jobs

Reviewed 2026-07-19 against the current
[Y Combinator Terms of Use](https://www.ycombinator.com/legal/#terms-of-use).
The terms prohibit scraping, data mining, robots, and similar gathering or
extraction methods. JobSentinel retires its YC HTML adapter, scheduled checks,
connectivity probe, and Settings recommendation. The user-opened YC search link
remains available. URL import and Browser Import fail closed before fetching or
extracting YC pages. Reconsider automated access only if Y Combinator publishes
an official feed or grants written authorization.

### Built In, Dice, SimplyHired, and Glassdoor

Reviewed 2026-07-19 against each provider's current Terms and robots rules:
[Built In](https://builtin.com/community-terms-of-use),
[Dice](https://www.dice.com/about/terms-and-conditions),
[Indeed terms covering SimplyHired](https://www.indeed.com/legal?hl=en_US), and
[Glassdoor](https://www.glassdoor.com/about/terms/). The reviewed terms do not
authorize the retired automated adapters or pasted-URL fetching. JobSentinel
blocks those paths before transport and ignores legacy local acknowledgements.
User-opened search links, visible Browser Import, and manual entry remain local,
user-controlled paths. Dice's official MCP is a separate review-required
candidate, not a substitute for the retired HTML adapter.

Current LinkedIn evidence was reviewed on 2026-07-19 from the
[LinkedIn User Agreement](https://www.linkedin.com/legal/user-agreement) and
[LinkedIn automated-activity guidance](https://www.linkedin.com/help/linkedin/answer/a1340567).
The agreement effective November 3, 2025 and the current guidance prohibit
unauthorized scraping, copying, browser plugins, and automated activity.
JobSentinel therefore blocks LinkedIn Browser Import, scheduled access, and
hidden page access. User-opened navigation and the local Workbench remain
available after trusted native review, with no session-material storage.

## Product implications

- Prefer official-source job monitoring where public feeds or employer pages are
  allowed.
- Keep collection low-volume, bounded, cached, and tied to user intent.
- Make restricted-source terms and account-risk warnings prominent before any
  risky action. JobSentinel does not encourage terms violations, but if users
  choose to proceed, the safe path must be local, explicit, reviewed, and
  harder to misuse than an unreviewed script or fork.
- For restricted authenticated sources such as LinkedIn, do not store auth
  tokens, session cookies, browser storage, authorization headers, or equivalent
  sign-in material. Every use must be user-initiated and show a plain-language
  site-rules and privacy warning before any sign-in screen. When JobSentinel is
  not inspecting or automating the restricted site, use a visible privacy
  reminder instead of forcing the user to stop.
- Do not collect candidate profiles, recruiter profiles, employee lists, social
  graph data, login-only pages, or restricted content.
- Track source health and source confidence in plain language.
- Record source, fetch date, canonical URL, and closure evidence.
- Add a company-careers discovery layer before adding custom scrapers. When a
  user provides or discovers an employer careers page, first detect whether it
  maps to a public hiring platform, official feed, or restricted board, then
  route to the safest available path.
- Keep source candidates, platform families, regional boards, access models,
  and career-profile coverage centralized in
  `src/shared/jobSourceDiscoveryTaxonomy.ts` so source discovery can grow
  without hard-coding source lists in UI components or docs-only notes.
- Keep parser resilience inside the existing source-adapter boundaries. The
  Scrapling Rust core can parse already-fetched HTML, but adopting it is not
  justified while JobSentinel already pins `scraper` and `quick-xml`; Scrapling
  fetch, browser, and spider features are out of scope because they add
  fingerprint impersonation, cookies, proxy rotation, or browser automation.
- Prefer deterministic parser failures over heuristic field guesses. If a page
  changes enough that a selector is uncertain, the user should see a clear
  source-changed fallback instead of a silently misclassified company, title,
  pay, or location field.
- Prefer structured parsing over string splitting for source formats that
  already have stable structure, such as JSON-LD script tags and RSS item
  fields.

## Scrapling Rust evaluation

Scrapling was reviewed on 2026-06-18 against JobSentinel's source-access
requirements, using the local `scrapling-rs` checkout, docs.rs crate docs, and
the upstream Rust and Python project docs. A secondary implementation guide was
also checked for its summary of fetcher modes, adaptive selectors, proxy use,
and browser-resource limits.

| Area | Finding | JobSentinel decision |
| ---- | ------- | -------------------- |
| Core parser | `scrapling` adds pseudo-element selectors, text helpers, HTML-to-Markdown, and adaptive element relocation. It also defaults to SQLite-backed storage through `rusqlite`. | Do not adopt for v2.9.0. Reconsider only if a source needs adaptive parsing of already-fetched HTML and the dependency can be added with default features off. |
| HTTP fetcher | `scrapling-fetch` defaults to browser TLS impersonation, stealth headers, redirect following, retries, cookies, proxy support, and whole-body reads. | Do not route source checks through it. JobSentinel requires the existing fetch funnel for URL resolution, no-redirect defaults, request-target checks, sanitized logging, and 16 MiB decoded body caps. |
| Browser automation | `scrapling-browser` uses Playwright and includes stealth sessions, cookie injection, proxy rotation, WebRTC/canvas fingerprint controls, and Cloudflare challenge handling. | Do not adopt. These capabilities conflict with the no hidden automation, no session-cookie collection, and no platform-control workaround boundaries. |
| Spider engine | `scrapling-spider` provides crawling, checkpointing, response cache, robots handling, blocked-response retries, and multi-session fetch backends. | Do not adopt. JobSentinel should keep checks bounded to user-enabled source adapters, official feeds, and source-specific rate limits instead of adding crawler infrastructure. |

The practical value for current adapters is low because JobSentinel already
extracts from official APIs, RSS feeds, JSON-LD scripts, and source-specific
structured payloads. If future work borrows ideas from Scrapling, borrow only
parser ergonomics or selector tests. Do not borrow stealth, proxy, cookie,
challenge-solving, broad crawling, or uncapped fetch behavior.

2026-06-19 recheck: `cargo search`, `cargo info`, and local checkout `0d61c3e`
show the Rust crates are still `0.2.0`, MIT licensed, and require Rust `1.85`.
The local Python Scrapling checkout advertises parser relocation, stealth
fetchers, Cloudflare challenge solving, proxy rotation, persistent sessions,
and broad spiders. The Rust port mirrors that split: `scrapling` is the core
parser, while `scrapling-fetch`, `scrapling-browser`, and `scrapling-spider`
carry transport, browser, anti-bot, proxy, cookie, and crawl behavior. A local
Cargo spike with `scrapling = { version = "=0.2.0", default-features = false }`
compiled, but it still added a second parser stack through `scraper 0.22.0`,
extra `html5ever` versions, `html2md`, text helpers, and URL/encoding helpers
alongside JobSentinel's pinned `scraper 0.27.0`.

Decision: do not replace JobSentinel's scraper stack wholesale for v2.9.0.
Parser-only Scrapling remains an allowed experiment only when a real source
fixture proves the current parser fails and Scrapling succeeds. If added later,
pin exactly to the latest stable crate, disable default features unless
SQLite-backed adaptive storage is explicitly justified, route already-fetched
HTML through the existing source adapter boundary, and keep JobSentinel's
current fetch funnel, redirect policy, URL validation, body caps, sanitized
logging, and source-specific pacing. Do not add `scrapling-fetch`,
`scrapling-browser`, or `scrapling-spider` for native source checks.

Additional Rust scraping references supplied on 2026-06-19 did not change that
decision. `scrapling-fetch` depends on `wreq` release-candidate crates with
cookie, compression, SOCKS, and browser-emulation features; the local lockfile
pulls in BoringSSL-related crates and native build tooling. Stygian's published
Rust crates are explicitly anti-detection, proxy-rotation, graph-extraction,
and browser-automation oriented. General Rust scraping guides are useful for
API/HTML parsing patterns, but JobSentinel's desktop release path needs fewer
native build surfaces, not more. Keep `reqwest` with rustls and the existing
`scraper`/`quick-xml` parsers unless a failing, source-specific fixture proves
otherwise.

Adaptive selector relocation is also a poor default for durable job records.
It needs a prior selector fingerprint, which new users do not have, and its
best-match behavior can be wrong in ways that look plausible. For scheduled
source checks, a blocked source or changed page must return a structured
source-status message and a manual or Browser Import fallback rather than
guessing job fields.

## Source-Mining Pass

The 2026-06-19 mining pass reviewed local JobSpy, old job-aggregator examples,
and public job scraper or automation projects. Keep useful source intelligence,
but do not import unsafe transport habits.

Useful deltas:

- Add BDJobs to the shared source taxonomy for Bangladesh coverage.
- Promote Naukri and Bayt to restricted candidate sources because source-mined
  projects show account-free data shapes worth reviewing.
- Treat Google Jobs as a candidate visible-result or user-opened import helper,
  not a scheduled search scraper.
- Add per-source capability notes before native adapters. Some sources cannot
  combine recency, remote, job-type, and easy-apply filters in one request, so
  JobSentinel should explain unsupported filter combinations instead of showing
  confusing empty results.
- Reuse normalization ideas for listed pay, salary interval, salary source,
  job type, remote signal, skills, experience ranges, company ratings, and
  direct employer apply links.
- Prioritize user-defined career portals: a user should be able to paste a
  company career URL and let JobSentinel detect Greenhouse, Lever, Ashby,
  Workday, or another reviewed hiring platform.

Patterns to avoid:

- Do not copy embedded third-party API keys, app tokens, static mobile headers,
  cookies, session state, or authorization material from other projects.
- Do not add proxy rotation, TLS/browser impersonation, challenge solving,
  cookie injection, hidden authenticated reads, or broad crawlers as default
  JobSentinel source behavior.
- Do not use "undetected" browser wrappers, challenge solvers, proxy-rotation
  SDKs, copied mobile-app identities, or pasted cURL/session replays to make a
  source pass. Treat those as evidence that the source needs a safer fallback,
  not as implementation requirements.
- Do not send resumes, private notes, salary floors, saved answers, or
  application history to external AI or hosted scraping services unless the
  user explicitly configures that path through the privacy-first AI gateway.

## Open evaluation ideas

- Maintain a source registry with permission notes, rate limits, and data shape.
- Test source adapters against robots and source-specific stop conditions.
- Compare freshness between official ATS pages and third-party copies.
- Review each new source against privacy, security, and responsible-AI docs.

## Source Governance Matrix

| Access model | Technical access | Preferred use | Examples | User agreement |
| ------------ | ---------------- | ------------- | -------- | -------------- |
| Official API or feed | Public unauthenticated or local API key | Native scheduled source | USAJobs, Adzuna, Reed, Remotive, official RSS or JSON feeds | Normal source opt-in; API key or source setup plus any required attribution or rate limit when needed |
| Public ATS postings | Public unauthenticated | Native scheduled source | Greenhouse, Lever, Ashby, Workable, SmartRecruiters, Recruitee, Personio | Normal source opt-in, no restricted-source acknowledgement unless the source is reclassified after review |
| Public community or remote source | Public unauthenticated | Native scheduled source with conservative limits and attribution where required | Hacker News hiring posts, We Work Remotely, RemoteOK, Remote First Jobs | Normal source opt-in |
| Policy-blocked website | Public page whose current terms prohibit extraction | User-opened search link only | YC job listings | No automated fetch, import, capture, probe, or scheduled action |
| Policy-blocked scheduled board | Public page whose current terms do not authorize the retired automation | User-opened search link, visible Browser Import, or manual entry | Built In, Dice HTML, SimplyHired, Glassdoor | No scheduled check, connectivity probe, pasted-URL fetch, or local-consent override |
| Employer-owned web API | Public unauthenticated when reviewed | Company discovery first, then native adapter after fixtures prove the endpoint is stable | Workday CXS tenants, Amazon Jobs, Google Careers, Microsoft Careers, GitHub iCIMS/Jibe, Tesla Careers | Normal source opt-in when public; keep Browser Import or manual entry fallback if the endpoint is unstable or blocked in the user's environment |
| Employer career system | Unknown until reviewed | Company discovery first, then classify into public API, public page import, restricted, or manual | Best Choice Products, Champion Petfoods, Ascend Wellness Holdings, Yourgi Pet, AC Lion, ForceBrands, Berri Organics, Renovation Brands, and other direct employer pages | Treat as review-required until source terms, structure, and rate limits are recorded |
| Restricted public board | Public unauthenticated with terms/account-risk warning | Search link, pasted individual job link, Browser Import, or explicitly acknowledged scheduled check where current policy permits | Indeed, Monster, ZipRecruiter, Naukri, Shine, Foundit, CV-Library, Totaljobs, Wellfound, ClearanceJobs | Prominent warning and explicit local acknowledgement before the risky action; no sign-in-session rules unless a sign-in session is opened |
| Restricted authenticated source | Authenticated user session | User-initiated interactive use within current provider policy | LinkedIn search, LinkedIn Jobs Tracker, FlexJobs, Upwork, Freelancer, Toptal, any future account-backed restricted source | Warning before sign-in, fresh user initiation, no auth/session/browser-storage persistence, no background collection, and no page capture where provider policy prohibits it |
| Unknown or changing source | Unknown review required | Manual entry or search link until reviewed | New country or niche boards | Treat as restricted until source terms, robots policy, rate limits, and practical access are reviewed |

## Company-Careers Discovery Examples

The 2026-06-19 source pass found that employer pages can hide a safe public ATS
source behind custom JavaScript. JobSentinel should make that discovery easy for
non-technical users instead of expecting them to know the ATS vendor or board
token.

| Employer URL | Observed source family | Recommended JobSentinel path |
| ------------ | ---------------------- | ---------------------------- |
| `https://www.fivetran.com/careers#jobs` | Greenhouse public board API, board `fivetran`; canonical job URLs point back to Fivetran careers pages | Detect and normalize to Greenhouse native source; preserve Greenhouse `absolute_url` |
| `https://job-boards.greenhouse.io/primerai` | Current Greenhouse hosted board, board `primerai` | Accept current `job-boards.greenhouse.io` host and use Greenhouse API first |
| `https://www.spacex.com/careers` | Custom Angular employer page with public Greenhouse board `spacex` | Detect Greenhouse board and use native Greenhouse API instead of scraping the custom frontend |
| Klaviyo, Faire, and Mindgruve careers | Verified Greenhouse public board tokens from direct board API probes | Treat as Greenhouse native sources when the user adds these employers |
| `https://www.tesla.com/careers/search/?site=US` | Tesla employer-owned careers system; local direct fetch can be blocked by edge controls | Keep browser-open and manual import fallback until stable public fixtures are captured without bypassing controls |
| `https://builtin.com/jobs`, state/city filters such as `?state=California&country=USA&allLocations=true`, and `https://www.builtincolorado.com/jobs` | Policy-blocked Built In network with regional city job boards | Use a user-opened search link, visible Browser Import, or manual entry; do not fetch the pasted URL or schedule collection |
| `https://www.linkedin.com/company/fivetran/jobs/` and search-results URLs with `keywords`, `geoId`, `f_TPR`, or `f_AL` filters | Restricted LinkedIn jobs and company jobs pages | User-opened navigation after trusted review; use manual or sanitized user-selected Workbench details, with no Browser Import, hidden access, or session-like storage |
| `https://www.linkedin.com/jobs-tracker/?stage=applied` | Restricted LinkedIn Jobs Tracker for user-reviewed jobs | User-opened navigation and local Workbench logging only; no page capture, broad background discovery, login capture, session-cookie storage, or hidden access |
| LinkedIn Jobs home anchors for Preferences, Job tracker, and My Career Insights | Restricted LinkedIn navigation surfaces | User-opened navigation only; use these to help a user reach the right LinkedIn area, not as stored source-query or session state |
| `https://www.google.com/about/careers/applications/jobs/results?hl=en_US` | Google employer-owned careers web app with public job-search surfaces | Add a source-specific adapter only after a stable public fixture is reviewed; keep user-opened search fallback |
| `https://www.yahooinc.com/careers/` | Yahoo custom career site with server-rendered search pages | User-opened employer search or source-specific adapter after endpoint and terms review |
| `https://www.ibm.com/careers/search` | IBM proprietary career search with additional career-domain links | User-opened employer search until a stable public endpoint is reviewed |
| `https://careers.microsoft.com/v2/global/en/home.html` and `https://apply.careers.microsoft.com/careers` | Microsoft employer-owned careers web app backed by Eightfold surfaces | Treat as public employer web API candidate; native adapter only after Eightfold endpoint and terms review |
| `https://optiv.wd5.myworkdayjobs.com/Optiv_Careers` | Workday CXS public jobs endpoint; direct probe returned published job JSON | Add Workday adapter with tenant fixtures, capped request bodies, and source-specific rate limits |
| `https://www.amazon.jobs/en/` | Amazon employer-owned public JSON search and recommendations endpoints | Add Amazon Jobs adapter candidate; normalize relative job paths |
| `https://www.github.careers/careers-home` | GitHub employer-owned iCIMS/Jibe career site | Add iCIMS/Jibe detection and adapter fixtures before scheduled checks |
| `https://openai.com/careers/search/` | OpenAI employer page with Ashby public job posting API behind application links | Add Ashby adapter and preserve OpenAI canonical job URLs when present |
| `https://www.anthropic.com/careers/jobs` | Anthropic employer page with Greenhouse public board links and API coverage | Normalize to Greenhouse native source |
| `https://remotefirstjobs.com/api/search-jobs` and legacy `https://jobscollider.com/api/search-jobs` | Remote First Jobs documented public remote-jobs API; legacy JobsCollider API redirects to it, and a live probe returned cybersecurity jobs with salary fields on 2026-06-19 | Add candidate native remote feed with source attribution and backlink requirements |
| Old aggregator examples using Adzuna, The Muse, Randstad, GitHub Jobs, and USCIS H-1B files | Useful source-family reminder, but GitHub Jobs is historical and unavailable from current probes | Keep Adzuna and The Muse as optional credentialed APIs, avoid GitHub Jobs as a live source, and use H-1B employer files as optional sponsorship/employer context rather than job postings |

Candidate platform families for the discovery registry include Greenhouse,
Workday, SmartRecruiters, Lever, Ashby, Breezy, JazzHR, Bullhorn, Workable,
Teamtailor, Recruitee, Jobvite, iCIMS, Taleo, Eightfold, SAP SuccessFactors,
Oracle Recruiting, Phenom, Personio, Comeet, Jobylon, Rippling, Zoho Recruit,
Freshteam, Pinpoint, JobScore, and employer-owned custom systems. Greenhouse,
Lever, Ashby, Recruitee, Personio, and SmartRecruiters are the first native
expansion candidates because they expose public posting APIs or feeds. Workday,
Eightfold, Bullhorn, JazzHR, Rippling, Zoho Recruit, Freshteam, Pinpoint, and
other enterprise or account-scoped systems need source-specific proof before
scheduled checks ship.

Regional and local boards are first-class source-discovery candidates, not
one-off hard-coded hostnames. This includes Built In city pages, state workforce
job banks, city and county career pages, local chambers of commerce, economic
development boards, local newspaper job boards, industry associations, and
sector-specific local sources for retail, hospitality, trades, healthcare,
education, legal, creative, finance, and HR roles. Treat these sources as
review-required or restricted until the specific source terms, structure, and
rate limits are recorded.

Expansion work for UK, India, and other markets should add sources only after
this classification is recorded. New native adapters need fixtures, rate-limit
rules, user-safe errors, and source-status behavior before release. If a source
requires attribution, such as logo or "jobs by" language, the UI must include it
before the adapter is enabled.

2026-06-19 expanded source research synced the shared taxonomy with additional
UK, European, remote, and sector-specific sources. Taxonomy-only additions
include CV-Library, Totaljobs, StepStone, XING Jobs, Otta, Welcome to the
Jungle, Jobicy, The Muse, Deel Jobs, Remote.com Jobs, No Fluff Jobs, Just Join
IT, Tech Ladies, and Malt. CV-Library and Totaljobs now match existing
restricted-domain gates; Foundit Gulf and Indonesia now share the Foundit
taxonomy entry. Treat these additions as discovery/import coverage until a
specific adapter has parser fixtures, transport limits, attribution handling,
safe errors, source-status behavior, and release-ledger proof.
