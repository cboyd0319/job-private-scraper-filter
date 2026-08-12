# Source And Browser Strategy

V3 should dramatically improve discovery without collapsing source boundaries.
The easy path should be official, public, user-approved, and local. Restricted
or authenticated sources should be helpful only through visible user-controlled
flows.

## Source Classes

| Class | Examples | V3 direction |
| --- | --- | --- |
| Official APIs and feeds | Greenhouse, Lever, USAJobs, RemoteOK, employer feeds | Low-friction scheduled checks with rate limits and parser tests. |
| Public ATS and careers pages | Ashby, SmartRecruiters, Workday, iCIMS, Phenom, Oracle, Taleo, Radancy, employer pages | Source graph classification, tenant review, fixtures, and native adapters when stable. |
| Public boards and aggregators | BuiltIn, WeWorkRemotely, Dice, SimplyHired, Glassdoor, regional boards | User-approved source checks with source-specific warnings where needed. |
| Search destinations | Google Jobs Search, LinkedIn search links, regional search pages | Open in browser, then use a policy-permitted import or manual review. |
| Restricted authenticated sources | LinkedIn and similar account-backed pages | User-visible Workbench with current provider-policy limits. No stored session material or scheduled refresh. |
| User imports | Browser Import, pasted URLs, pasted text, local files, CSV | Review queue with source provenance and token scrubbing. |
| Community packs | Source packs, regional packs, role packs | Signed declarative packs with fixtures, source policy, and local review. |

## Source Graph

V3 should use a source graph instead of scattered source constants. Each source
record should include:

- canonical source id
- display name
- source class
- domain and endpoint patterns
- official documentation link when available
- auth requirement
- restricted status and reason
- source policy or terms review note
- robots review status where relevant
- rate limit
- parser type
- fixture paths
- supported fields
- salary coverage expectation
- last verified date
- risk notes

This lets JobSentinel explain why one source can run automatically, another
needs a warning, and another is browser-only.

## Official And Public Source Expansion

Highest-value v3 source work:

- Promote reviewed Workday CXS, Phenom widget, Radancy/TalentBrew, Oracle
  Fusion Candidate Experience, and Taleo contracts from research into reliable
  source adapters where tenant policy and fixtures pass.
- Add Ashby, SmartRecruiters, Workable, Teamtailor, Recruitee, Breezy, JazzHR,
  Jobvite, iCIMS/Jibe, Bullhorn, Eightfold, and SAP SuccessFactors review lanes.
- Add company-careers discovery that maps arbitrary company pages to native
  source support, Browser Import, pasted link import, or manual entry.
- Add public structured-data extraction from schema.org JobPosting, JSON-LD,
  microdata, sitemaps, RSS, and Atom feeds.
- Build regional source packs for countries, states, cities, public sector,
  universities, healthcare, trades, and remote-first communities.
- Add source-health scoring so bad parsers and stale boards become visible
  before they waste user time.
- Add UK, EU, and India starter source lanes with region manifests, pay and
  location fixtures, taxonomy bridges, and incomplete-coverage labels.

## Browser Companion

The Browser Import button should evolve into a browser companion:

- Side panel or extension UI with local JobSentinel actions.
- Page-level import for visible job cards and visible job details.
- Case-file drawer next to the source page.
- Local overlay for already-seen, duplicate, fit, pay, ghost-risk, and source
  cues after user enables it.
- One-click ledger actions: Save, Applied, Track, Reject, Follow up, Interview,
  Offer, Not interested, Reminder, Note.
- Review queue for every imported record before durable storage.
- Optional user-selected OCR import for rendered text when DOM extraction is
  unreliable, with the same visible-source and review boundaries as other
  imports.

The browser companion should not become a hidden scraper. It should capture
what the user intentionally exposes and approves only where current provider
policy permits page capture.

## Restricted Authenticated Sources

For LinkedIn-style sources:

- The user starts the flow from JobSentinel.
- JobSentinel shows short, plain, friendly acknowledgement copy.
- The user signs in directly with the source.
- JobSentinel does not store restricted-site cookies, session tokens,
  authorization headers, browser storage, hidden page state, screenshots, or
  network traffic.
- The user chooses import or a JobSentinel ledger action.
- Captured records remain suggestions until reviewed.
- Local analysis starts after the record exists in JobSentinel.
- Scheduled background refresh is not allowed for account-backed restricted
  pages.
- Current LinkedIn terms prohibit third-party page capture, browser plugins,
  and automated activity, so LinkedIn Browser Import is blocked. The Workbench
  accepts only user-entered or user-selected details after trusted native
  review.

V3 can still reduce friction inside those rules with one-click local actions,
duplicate detection, sanitized pasted details, and direct case-file handoff.

## Public Source Automation

Unauthenticated public sources should not receive the same friction as
restricted sign-in sessions. They still need:

- source review
- rate limits
- safe error handling
- response size caps
- parser fixtures
- URL validation
- source policy notes
- local storage boundaries

When source terms or access expectations are unclear, use warning-gated checks
instead of hidden behavior.

## Declarative Source Packs

V3 should explore source packs that contain:

- source manifest
- policy note
- rate limit
- endpoint patterns
- selectors or JSON paths
- field mappings
- test fixtures
- example URLs
- expected output schema

Default source packs should be declarative. Avoid arbitrary JavaScript in
consumer packs unless there is a reviewed sandbox, no sensitive data access, and
strong fixture coverage.

## Sandboxed Dynamic Adapters

Some public sites may need more than selectors and JSON paths. V3 can research
dynamic adapters, but only as a bounded runtime:

- Rust owns URL selection, rate limits, response size caps, storage, and source
  policy.
- The adapter receives fixture HTML or a fetched public response from Rust.
- No ambient filesystem, shell, network, credential, browser storage, or local
  database access.
- Execution has CPU, memory, time, and output-size limits.
- Fixture tests and pack signatures must pass before activation.
- Restricted authenticated sources remain visible user-controlled flows, not
  dynamic adapter targets.

Declarative packs remain the default.

## Scraper Technology Evaluation

Robust parser libraries and browser rendering can help with public, reviewed
sources. V3 should evaluate them against JobSentinel boundaries:

- Does JobSentinel still control URLs, frequency, response size, and storage?
- Does the library avoid credential/session capture?
- Does it avoid hidden restricted-source browsing?
- Does it avoid human-check solving, proxy rotation, fingerprint evasion, or
  platform-control bypass?
- Can it run on Windows 11+, macOS, and Linux?
- Can parsers be tested with saved fixtures?
- Can failures produce plain user messages?

The v3 priority is resilient source access through official APIs, reviewed
public pages, and user-visible browser capture. Platform-control evasion is not
a product foundation.

## Source UX

Nontechnical users should see:

- "Ready to check"
- "Needs your review"
- "Open in browser"
- "Source blocked or changed"
- "No salary data"
- "Looks stale"
- "Already imported"
- "Needs source pack update"

They should not need to understand ATS families, robots policy, JSON paths, or
rate limits to know what to do next.
