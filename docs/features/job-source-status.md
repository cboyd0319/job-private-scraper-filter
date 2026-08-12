<!-- Defines user-visible job-source availability and safe recovery guidance. -->

# Job Source Status

Job source status helps users see whether job sources are working, having
trouble, unavailable, or turned off. The dashboard turns source failures into
plain-language status and safe next steps.

## Scope

| Area | Current behavior |
| ---- | ---------------- |
| Scheduled source status | Tracks Greenhouse, Lever, RemoteOK, WeWorkRemotely, startup and tech job posts, disabled JobsWithGPT, and review-required USAJobs |
| Source-check coverage | Includes active scheduled sources plus locally skipped Indeed, Wellfound, and ZipRecruiter availability checks |
| User-opened search links and Workbench | LinkedIn and similar destination links are opened by the user, with local-only Workbench actions and no background monitoring |
| Restricted-site gate | User-directed actions explain site rules; provider policy prohibitions cannot be overridden locally |
| Saved access details | Tracks user-approved external channels where applicable; LinkedIn login details are not collected |
| Support reports | Safe support reports can be copied or saved locally, reviewed, and shared only when the user chooses |

## User States

| Status | Meaning | User-facing action |
| ------ | ------- | ------------------ |
| Working | Source recently worked | Keep using this source |
| Having trouble | Some recent checks failed | Try again later or use search links if urgent |
| Not working | Repeated recent checks failed | Prefer other sources or open official company pages |
| Off | User or policy turned source off | Turn on only if useful and allowed |
| Not checked | No recent check data | Check this job site now or wait for the next scheduled check |

## Dashboard Surface

The Settings source-status view should show:

- Summary counts for working, having-trouble, not-working, off, and not-checked
  sources.
- One row per source with status, plain recent-result label, time needed, jobs
  found, last checked, whether JobSentinel can read jobs, and what to do next.
- Run history for recent attempts.
- Job-site check buttons for known supported sources.
- Job-site checks use the current saved settings in the running app; users do
  not need to close and reopen JobSentinel after changing source settings.
- Safe support report actions when a user needs help.

## Source Policy

Source status must follow the same rules for job sources:

- Prefer official feeds, public feeds, and official employer or application
  platform postings.
- Check sites politely and avoid reading more page data than needed.
- Do not add hidden job-site checks.
- Do not collect restricted-site session credentials.
- Do not get around human checks or platform controls.
- Do not imply JobSentinel condones violating any source terms.
- Disable scheduled access when provider policy does not authorize automation.
  Local acknowledgement cannot override that boundary.
- Do not include raw credentials, cookies, private notes, resumes, salary floors,
  or application history in health errors or support reports.
- JobsWithGPT scheduled contact remains disabled while its provider endpoint
  and usage-policy review is unresolved. A saved exact-payload approval cannot
  authorize contact by itself.
- If a configured feed is later reviewed and enabled, disclose that it receives
  only saved job titles, location, remote preference, and result limit. It must
  stay off unless both provider governance and the exact local payload approval
  are current.
  The latest approved contact can be shown locally as contact time, website
  contacted, count-only request categories, and outcome. Do not store raw
  titles, raw location, private notes, resumes, salary floors, application
  history, or full source links in that contact history.
  Settings also labels sensitive data that was not sent, including resume text,
  salary floors, private notes, application history, and full source links.

LinkedIn and other restricted boards are intentionally handled through
user-directed or explicitly acknowledged paths. They should not appear as a
credential-renewal prompt, hidden background source, or job-site check that runs
without the user's explicit action.

The LinkedIn Workbench is available from Dashboard quick actions and Settings.
It records only user-confirmed local events such as applied, saved, tracking,
rejected, interview, follow-up, reminder, notes, or not interested. It does not
collect LinkedIn login details, read page content, inspect network traffic,
save browser storage, or run scheduled LinkedIn checks.

Built In, Dice HTML, SimplyHired, and Glassdoor scheduled checks are retired.
Their health rows stay absent, direct legacy check identifiers stop locally,
and stale config flags cannot restore transport. The status surface should
point to a user-opened search link, employer career page, Browser Import, or
manual entry. Pasted-URL import for these domains also stops before transport.
Do not expose raw source errors, cookies, tokens, full query links, or private
profile details while explaining the recovery path.
