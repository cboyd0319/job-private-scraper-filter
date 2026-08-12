# Application Tracking

**Keep each opportunity, follow-up, note, interview, and offer in one local
place.**

JobSentinel's application board is for anyone managing a job search, not only
engineers. It helps job seekers see what is active, what needs follow-up, what
has gone quiet, and which roles are worth more attention without turning the
search into a rejection counter.

Application history is sensitive. It can reveal employment status, job-search
urgency, salary expectations, location constraints, recruiter contacts, and
private notes. JobSentinel keeps this workflow local by default.

![Application Tracking - Kanban Board](../images/application-tracking.png)

## Privacy Labels

| Workflow | Label | Default behavior |
| --- | --- | --- |
| Application board | Local only, Sensitive | Records stay on this device. |
| Notes and contacts | Local only, Sensitive | Private notes and recruiter details stay local. |
| Follow-up reminders | Local only | Reminders are generated locally. |
| Interview tracking | Local only, Sensitive | Interview details stay local unless the user exports them. |
| Offer and pay notes | Local only, Sensitive | Salary floors and offer notes stay local. |
| External notifications | Sensitive | Slack, Discord, Teams, email alerts, or other channels are used only if the user turns them on. |

External AI is not required for application tracking.

## What It Helps With

- **Status clarity**: See applications by status so the next action is clear.
- **Follow-up timing**: Track when a reply, thank-you note, or check-in is due.
- **Quiet-period awareness**: Notice roles that have gone silent without
  treating silence as personal failure.
- **Contact memory**: Keep recruiter names, email addresses, and context beside
  the role.
- **Interview preparation**: Keep scheduled conversations, prep notes, and
  post-interview follow-ups attached to the application.
- **Offer comparison**: Track compensation and offer details with salary-floor
  context.
- **Search pacing**: Review active, stale, rejected, withdrawn, and offer-stage
  work without forcing every role to look equally urgent.

## Everyday Workflow

1. Open **Applications**.
2. Start with **Daily mission**. It shows three to seven concrete actions for
   specific reminders or opportunities, ordered by urgency and capped to keep
   the list usable.
3. Use each **After this** cue to move from the current review into application
   tracking, outreach, interview prep, offer review, source settings, or weekly
   replanning. Offer and source actions open those exact existing surfaces.
4. Review the board and pending follow-ups.
5. Move each card when status changes.
6. Open a card to add notes, contact details, salary information, or next steps.
7. A quiet-role action opens that opportunity for review. It does not silently
   move one or more applications to **No Response**.
8. Use interviews, reminders, local summaries, and explicit status changes to
   decide where to spend time next.
9. Use **Review this week's plan** to compare tracker evidence before changing
   lanes, sources, pacing, or stop rules.

JobSentinel should make this usable for a person who has never used a project
management tool. The board should answer plain questions:

- What should I do today?
- Which roles are waiting on me?
- Which roles are waiting on the employer?
- Which postings have gone quiet long enough that I should stop spending
  tailoring time on them?
- Which opportunities are worth extra preparation?

## Opportunity Case

The dashboard's explicit **Open case** action creates or reuses a local case for
the selected job. Its first summary combines the job and source state, stale or
repost signals, application and contact presence, interview counts, offer and
any terminal application outcome, evidence freshness, and a sanitized timeline.

Opening and reviewing a case works offline. Source refresh remains an explicit
connectivity action. The summary intentionally excludes raw notes, contact
values, resumes, packet text, event payloads, and full posting content; those
remain with their existing feature owners.

The first Milestone 6 case-file, daily-mission, evidence-wall, preparation,
debrief, and native file-drop slices are implemented. The case compares only
the active saved resume's exact saved match; it never silently substitutes
another resume. It
shows requirement-level evidence categories without resume text or opaque IDs, a
deterministic Apply, Maybe, Skip, or Research more summary, and plain "Why not
this job?" reasons. Missing, unconfirmed, stale, changed, military-section, or
required hard-constraint evidence stays reviewable rather than becoming a
qualification claim. Accepted offers are closed outcomes, not Apply
recommendations.

**Prepare this job** builds a local workup from that safe case snapshot. It
keeps source freshness, evidence and reviewed-claim state, material selection,
screening answers, and final review explicit. It does not select attachments,
infer protected veteran status or eligibility, refresh a source, call AI, write
case state, send data, or submit an application. The user compares exact
employer wording with confirmed records and completes any employer-site action.
Protected veteran status, disability, race or ethnicity, gender, and other
voluntary sensitive personal questions remain local and manual-only. The case
does not prepare an answer; Application Assist leaves matching live-form fields
untouched and exposes only a generic manual-review topic.

The existing **Interview Schedule** owns the post-interview debrief. An explicit
local save records signal strength, questions asked, concerns, promised next
steps, and an optional follow-up deadline without scoring performance, changing
application status, or sending data. Incomplete interviews remain open after
their scheduled time so the debrief is reachable, and saved debriefs remain in
completed interview history without an arbitrary age cutoff.

The installed main window accepts one regular file drop at a time. Rust opens
the source once without following a final symlink or Windows reparse point,
copies its bytes into private app-owned staging, and sends the review surface
only an opaque token and sanitized file name. Replacement, cancel, completion,
and restart clean staged copies. JobSentinel does not subscribe to Tauri's
built-in raw-path event, grants the renderer no filesystem capability, and
never renders or logs the source path.

The user explicitly chooses the existing owner:

- **Add resume** reuses the local PDF, DOCX, text, Markdown, or HTML validation,
  managed copy, and Resume-page review path.
- **Review job posting** accepts bounded UTF-8 text, creates an editable Smart
  Paste draft, and requires **Save Job**. Duplicate drafts cannot be saved. It
  does not fetch a source, call AI, or create a durable job on drop.
- **Backup/Recovery** requires the backup passphrase and stages the encrypted
  restore for restart without changing the current session.

The existing Add resume, Import Job, and Backup/Recovery controls remain the
keyboard and non-drop paths. Source-pack drops remain with Milestone 7 because
they require the signed-pack quarantine and installer.

First-run setup saves reviewed search settings, allows a disclosed session-only
skip, and retains every choice for retry after a failed save. The case workflow
has browser coverage for empty, partial, duplicate, offline, failed-source, and
restored-data states at desktop and narrow widths without hidden refresh, merge,
status change, send, or submit work.

## Statuses

The application board supports the full job-search path:

| Status | Meaning |
| --- | --- |
| To Apply | Saved for review, not yet submitted. |
| Applied | Application sent or started. |
| Screening Call | Early recruiter or hiring-team contact. |
| Phone Interview | Phone or video conversation scheduled or completed. |
| Skills Interview | Skills, work-sample, portfolio, writing, trade, case, or assessment round. |
| Onsite Interview | Final, panel, site, or deeper-round conversation. |
| Offer Received | Employer made an offer. |
| Offer Accepted | User accepted an offer. |
| Offer Declined | User declined an offer. |
| Not Selected | Employer declined to continue. |
| No Response | No response after the quiet period you chose. |
| Withdrawn | User chose to stop pursuing the role. |

Stored status keys may use legacy internal names for compatibility. Visible
copy should stay broad enough for technical and non-technical job searches.

## Protective Follow-Up Model

Follow-up reminders should help the user spend energy carefully:

- A reminder suggests a next step; it does not send messages automatically.
- A quiet-period warning means "review this role" rather than "you failed."
- Selecting a mission action never changes an application status automatically.
- No-response review should help the user stop wasting time on stale or
  non-responsive roles.
- Recent applications with no contact note must not be moved to **No Response**
  until the quiet period has passed.
- Weekly summaries should favor useful decisions over motivation copy:
  continue sources that produce active employer or ATS pages, replies,
  interviews, referrals, or strong fit; adjust mixed sources; pause stale,
  duplicate, weak-source, low-fit, or below-floor lanes.
- Salary and offer notes should help users avoid accepting below-floor offers.

## Data Boundaries

- Application records, notes, contacts, salary details, and interview details
  stay local by default.
- External notification channels require user configuration.
- No application tracking feature should upload all local job-search data.
- Private notes, salary floors, resumes, and unrelated application history must
  not be sent to any external AI provider by default.
- Research and grant evaluation should use public postings and synthetic
  candidate profiles unless a real user gives explicit informed consent.
