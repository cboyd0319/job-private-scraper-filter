# Local Job-Search Data

**Keep job-search history, saved searches, templates, reminders, and safe
support reports under user control.**

JobSentinel treats job-search data as sensitive. Notes, templates, searches,
interview prep, notification choices, salary context, and problem reports can
reveal employment status, urgency, career goals, location constraints, and
private circumstances. Core data workflows stay local by default. On macOS and
Linux, JobSentinel keeps its app-owned data and settings directories private to
the current account and keeps local database files owner-only.

## Privacy Labels

| Workflow | Label | Default behavior |
| --- | --- | --- |
| Cover letter templates | Local only, Sensitive | Saved locally and never submitted automatically. |
| Interview prep and follow-ups | Local only, Sensitive | Checklist state and reminders stay local. |
| Saved searches | Local only, Sensitive | Search names, words, filters, and history stay local. |
| Notification preferences | Local only, Sensitive | Preferences stay local; external channels are used only if the user turns them on. |
| Safe support reports | Local only, Sensitive | Reports are sanitized before copy or save. |
| Local-data backups | Local only, Sensitive | Backup files stay on the user's device and leave saved connection details out. |
| Reviewed plaintext exports | Local only, Sensitive | Exports require a separate review, leave managed credentials and paths out, and never overwrite an existing file. |
| Storage health and cleanup | Local only, Aggregate | Inspection reports only health and byte totals; cleanup preserves application records and works offline. |
| Offline recovery and repair | Local only, Sensitive | Recovery starts without primary storage or credentials, exposes only bounded local state, and labels connectivity before an action. |
| Platform health | Local only, Aggregate | Inspection and app-owned Unix permission repair stay local; ambiguous links and Windows permission state require manual review. |
| Location detection | Sensitive | Public-IP lookup happens only after explicit user action. |

External AI is not required for user-data management. If an outside-AI send is
blocked because the details are not on JobSentinel's reviewed sharing list, the
user-facing error should describe reviewed details, not payloads, fields, or
classification internals.

## What Stays Local

- Cover letter templates and generated drafts.
- Interview prep checklists and follow-up reminders.
- Saved searches, search history, and notification preferences.
- Problem history and sanitized safe support reports.
- Local-only UI state such as theme, onboarding completion, temporary recovery
  hints, and sanitized app-error records.

External notifications such as Slack, Discord, Teams, email, or other channels
are optional. They are used only after the user configures them.

## Everyday Workflows

### Cover Letter Templates

Templates help users avoid starting from a blank page. JobSentinel can fill
known placeholders such as company name, job title, hiring contact, department,
location, and listed pay range.

The user always reviews the result. JobSentinel does not submit the letter.
The templates page shows a **Cover Letter Review** checklist backed by
`src/shared/coverLetterReviewTaxonomy.ts` so users replace blanks, verify
claims against their resume or records, and check job-specific wording before
copying or sending.

### Interview Prep

Interview prep keeps logistics, company notes, role-specific questions,
follow-up reminders, and thank-you status near the application. Checklists are
for preparation, not performance scoring.

After an interview, an explicit local debrief save keeps signal strength,
questions asked, concerns, promised next steps, and an optional follow-up
deadline with that interview. Saving a debrief does not change application
status, calculate a hiring probability, notify anyone, or send data.

### Saved Searches

Saved searches store repeatable searches such as:

- "Office manager remote"
- "Clinic scheduler part-time"
- "Warehouse supervisor Denver"
- "Design assistant entry-level"

Saved searches should make return visits easier for non-technical users. The UI
should not require users to understand query syntax, filters, or scoring math.
First-run starting paths include office and administration, retail and
hospitality, trades and field service, healthcare, education, customer support,
sales, finance, operations, creative, legal, data, security, and software work.
When a starting path is selected, setup previews sample job titles and search
words as editable suggestions before the search setup is saved. Setup persists
those ranking, source, and alert settings; it does not create a dashboard Saved
Search filter snapshot. Users create those snapshots explicitly from Dashboard.
Software, security, and data paths can suggest tech-heavy job sources, but
first-run setup keeps them off until the user checks those sources in review.
Non-technical searches do not suggest the retired SimplyHired scheduled
adapter. Users can add reviewed active sources later or use user-opened search
links, Browser Import, and manual entry. Pasted-URL import for the four retired
domains stops before transport.

### Notification Preferences

Notification settings control which saved searches and job sources can create
alerts. Plain labels should describe alert destinations and fit level
instead of service internals.
First-run desktop alerts start off and are saved only after the user turns them
on. External alert channels stay optional and are configured later in Settings.
First-run profile suggestions do not set a salary floor or narrow work-location
modes without user choice. Location starts broad across remote, hybrid, and
on-site, and pay stays unset unless the user enters a floor. First-run pay can
be entered as yearly or hourly; hourly pay is converted to an annual floor for
local pay comparisons while the review keeps the hourly meaning visible.

Users can skip setup for the current session without saving settings or starting
source work. The wizard states that setup returns next time. A failed save keeps
every reviewed choice visible and offers a bounded retry before entering the app.

### Safe Support Reports

When something breaks, users can choose **Copy Safe Support Report** or **Save
Safe Support Report** from Settings, App Problem History, crash recovery, or
page error recovery. Reports are local by default and should avoid full private
details.

Safe support reports can include high-level app state, feature names, timestamps,
sanitized error categories, redacted settings summaries, and fixed Privacy
Doctor identifiers, states, and next actions. The report excludes renderer
error messages and arbitrary error context. Final sanitization removes complete
absolute paths and all trailing filename text from their lines, including
Windows drive, UNC, mounted, temporary, and home paths. Reports should not
include full notes, resumes, full
search text, salary floors, secrets, private paths, cookies, connection links,
tokens, raw field names such as `url`, provider or model names, or full
application history.

Privacy Doctor is passive and offline. It inspects local storage, structural
portable-backup history, credential-vault metadata, configured external-AI
safeguards, Browser Import code state, governed source state, and the
no-telemetry product contract. It does not read secrets or test Keychain.
A recorded backup creation does not prove the file still exists, and system
credential storage is reported as unchecked until the user performs an action
that needs it.

App Problem History hides crash details from the visible list. If JobSentinel
help asks for more detail, users can copy or save a safe support report and
review it before sharing.

### Local-Data Backups

Settings includes **Backup Settings** and **Restore Settings** actions for a
private local-data backup file. The file includes settings, saved searches, and
cover letter templates. It does not include saved connection details, passwords,
tokens, cookies, browser sessions, or local database files.

After restoring a backup, users review settings and choose **Save Changes**.
Saved connection details must be added again if the restored settings use an
external alert channel or source that needs them.

Backup and recovery review shows the same plain-language coverage to the app and
the user:

- Settings backup: settings, saved searches, and cover letter templates.
- Encrypted portable backup: local jobs, applications, resumes, notes,
  reminders, and history for full recovery on another device.
- Neither backup includes saved connection details, passwords, tokens, cookies,
  browser sessions, or safe support reports.
- Full local recovery verifies compatibility and stages the replacement before
  startup promotes it. The previous compatible database remains available for
  rollback until recovery finishes.
- A dropped encrypted backup is first copied into private app-owned staging,
  then requires its passphrase and an explicit stage action. It never restores
  data during the current session.
- Recovery can inspect, stage, cancel, and clean incomplete restore work without
  network access. Corrupt primary data is preserved independently before a
  staged replacement is published.
- Copy or save a safe support report before full local recovery.

### Offline Recovery And Repair

If primary startup fails, JobSentinel opens a local recovery surface instead of
starting the normal app. This fallback uses temporary in-memory services and
does not initialize the primary database, production credential service,
scheduler, normal command surface, or tray search.

The recovery surface can work without connectivity to:

- preserve malformed configuration as a new private file before resetting it;
- inspect bounded storage, Privacy Doctor, platform-health, and queued local
  work state;
- create an encrypted portable backup from readable primary storage;
- stage, inspect, cancel, or clean an encrypted restore;
- preserve and clear a malformed restore marker;
- create a reviewed plaintext export or safe support report; and
- inspect or repair app-owned Unix file permissions.

Queued local work reports only a count and fixed capacity. It never exposes
queued URLs or other content, and stale entries expire locally.

Restore and repair operations fail closed on symlinks, hard links, non-regular
files, identity changes, unreadable state, and a live database owner. Invalid
configuration, corrupt-primary quarantine, and rollback publication use
independent copies so later writes through another path cannot change the
preserved recovery artifact.

On Windows, permission state is reported as unchecked and repair guidance is
manual. Package repair is guidance only: an already downloaded, verified
installer can be used offline; obtaining another installer is labeled as
requiring connectivity before the user acts. JobSentinel does not run a shell,
elevate, download, or install a package from this recovery surface.

### Reviewed Plaintext Export

Reviewed export provides a documented JSON Lines file for leaving JobSentinel
without copying its private database format. Review happens before any file is
written and names each selected category and record count. The export is
created entirely offline, uses a fixed field allowlist, ends with a completion
record, and refuses database tables this version does not understand. The
completion record detects incomplete writes; it is not a signature or
tamper-proof seal.

JobSentinel-managed credentials, encryption keys, authentication state,
dedicated app-managed file paths and connection-link fields, diagnostic caches,
and reproducible public datasets are never included. Legacy job, profile,
interview-location, and structured resume-draft links are cleaned before
writing. Protected application answers, clearance fields, military service,
veteran status, and disability information inside structured drafts are
excluded by default and require a separate explicit selection. Review shows
application-answer and structured resume-draft record counts separately.

User-authored resumes, descriptions, notes, cover letters, and similar text are
copied as written. They can contain accidentally pasted passwords, tokens,
private links, protected facts, or other sensitive details that JobSentinel
cannot identify reliably without damaging the user's records. Review the
plaintext artifact before sharing it. The destination must remain private.
JobSentinel does not overwrite an existing destination.

### Storage Health And Cleanup

Storage inspection works without network access and reports only whether the
local database is healthy, the aggregate number of already-free bytes, the
write-ahead log size when a database file exists, and whether SQLite supports
incremental page reclamation. It does not expose record contents, table counts,
credentials, or private file paths.

Cleanup first checks database and relationship integrity. If either check
fails, cleanup makes no change and directs recovery to a compatible backup.
For a healthy database, cleanup checkpoints SQLite's write-ahead log and asks
SQLite to reclaim at most 100 already-free pages when incremental vacuum is
enabled. It does not delete application records, rebuild indexes or search
data, contact a service, inspect credentials, or remove backups, exports,
restore data, model files, or other artifacts. A busy write-ahead log is a
reported failure rather than a reason to force another process or reader out.
Each attempted cleanup after the health check records only structural local
provenance and a sanitized error category.

## Older Local Data

Users who had older browser-saved templates or searches may see a migration
prompt. The migration moves supported local data into the current local store.
If no prompt appears, make a safe support report first, then close and reopen
JobSentinel and check Settings.

## Backups And Deletion

- Delete templates and saved searches carefully; deleted items may not be
  recoverable inside the app yet.
- Use Settings backup before moving devices or making larger search changes.
  The backup covers settings, saved searches, and cover letter templates.
- Use safe support reports before changing more data if something looks wrong.
- Use encrypted portable backup for full local recovery. It is separate from
  Settings restore because it can replace jobs, applications, resumes, notes,
  reminders, and history.
- Use reviewed plaintext export when a readable, application-independent copy
  is needed. Treat the resulting JSON Lines file as sensitive.

## When Something Does Not Work

| Problem | Plain next step |
| --- | --- |
| Older templates did not appear | Make a safe support report first, then close and reopen JobSentinel and check Settings for a migration prompt. |
| A saved search is missing | Open Saved Searches, then check recent search history. |
| Alerts feel too noisy | Raise alert selectivity or narrow the saved search. |
| Alerts miss expected jobs | Lower alert selectivity or check whether the source is enabled. |
| A template was deleted | Stop editing, make a safe support report, and check whether another copy exists. |
