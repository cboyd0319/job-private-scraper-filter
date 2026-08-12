# Getting Started with JobSentinel

**A private job-search assistant that keeps your data on your computer.**

This guide helps you install JobSentinel, create a first saved search, and know
where to get help if something does not work.

---

## Before We Start

You'll need to install JobSentinel on your computer. Here's the quick version:

**Is JobSentinel free?**

Yes. JobSentinel is free, will always stay free, and will always remain MIT
licensed. You can use it without a paid account, and people can use the code to
help more job seekers, including by building something better.

**Download the package or installer** (recommended)

1. Open the [JobSentinel latest download page](https://github.com/cboyd0319/JobSentinel/releases/latest).
2. Choose the package or installer for your computer:
   - **Windows installer**. If the latest Windows file includes `_unsigned`,
     expect a SmartScreen warning and verify the matching `.sha256` checksum
     for the downloaded `.msi` or setup `.exe` before opening it.
   - **Mac package** for Apple silicon and Intel Macs. Use the
     `_no-account_universal.dmg` from the latest release only when the same
     release also shows a matching `.dmg.sha256` checksum file.
   - **Linux installer** when present on the release
3. Install it:
   - Windows or Linux: double-click and follow the prompts.
   - Mac: open the `.dmg`, drag **JobSentinel** to **Applications**, then eject
     the disk image.
4. Open JobSentinel from your apps list or **Applications** folder.

JobSentinel does not silently update itself. When a newer release is available,
back up Settings first, download the new installer from GitHub Releases, verify
the matching `.sha256` checksum, then install it. See
[Updating Or Going Back](UPDATES.md) for the full update and rollback steps.

<details>
<summary><strong>First time on Mac? (Gatekeeper warning)</strong></summary>
<br>

macOS may show "JobSentinel can't be opened because Apple cannot check it for
malicious software." This is expected for no-account Mac packages because they
are not Developer ID signed and notarized yet.

Continue only if you downloaded JobSentinel from the latest download page
linked above, expected this file, and verified the `.dmg` against the matching
`.dmg.sha256` checksum from the same release. If you cannot verify the checksum,
do not bypass the warning; stop, delete the download, and use a fresh local
build or wait for a replacement package.

Advanced checksum check from the folder containing both downloads:

```bash
shasum -a 256 -c JobSentinel_*.dmg.sha256
```

**To continue after checking the download:**

1. Open **JobSentinel** from **Applications** once.
2. Dismiss the warning.
3. Go to **System Settings > Privacy & Security**.
4. Scroll down and click **"Open Anyway"** next to the JobSentinel message.
5. Click **Open** in the confirmation dialog.

This can happen because JobSentinel is a new open-source app. You only need to
approve it once.

</details>

<details>
<summary><strong>Windows showing a blue warning?</strong></summary>
<br>

Windows SmartScreen may show "Windows protected your PC" when an installer is
unsigned or not widely trusted yet.

Continue only if you downloaded JobSentinel from the latest download page
linked above, expected this file, and can verify the installer identity or
checksum from the same release. For an unsigned no-account release, the Windows
installer filename should include `_unsigned` and the same release should include
a matching `.sha256` checksum for that `.msi` or setup `.exe`. If you are not
sure, stop, delete the download, and use the latest signed installer when
available or build from source.

</details>

<details>
<summary><strong>For people changing JobSentinel</strong></summary>
<br>

Most people should download the installer and skip this section.

People changing the app can use the
[contributor setup guide](../developer/GETTING_STARTED.md).

</details>

---

## Step 1: Open JobSentinel

After installation:

- **Windows:** JobSentinel appears in your system tray (bottom-right corner)
- **macOS:** JobSentinel appears in your menu bar (top of screen)

Left-click the icon to open the dashboard.

---

## Step 2: Answer the first-run questions

The first time you open JobSentinel, setup asks a few plain questions.

### Question 1: What kind of work are you looking for?

Pick a starting area or choose **Build My Search**.

When you pick a starting area, JobSentinel shows suggested job titles and search
words. These are starting suggestions. You can edit job titles, search words,
pay, and location before saving.

### Question 2: What job titles do you want?

Type in job titles that match what you want. Examples:

- "Marketing Manager"
- "Registered Nurse"
- "Customer Success Manager"
- "Project Manager"
- "Bookkeeper"

**Tip:** You can add multiple titles. Specific titles help JobSentinel find
more relevant roles.

### Question 3: What work do you want more of?

Add skills, tools, duties, or strengths you want JobSentinel to notice.
Examples:

- "Customer support"
- "Scheduling"
- "Budgeting"
- "Patient care"
- "Project management"

If you have a saved resume, you can choose **Check saved resume skills**. Any
suggested skills stay local and are added only when you pick them. The final
review names skills added from the saved resume so you can remove any you do
not want before starting the search.

### Question 4: What work should JobSentinel avoid? (Optional)

Add words or phrases that should rank jobs lower. Examples:

- "Night shift"
- "Heavy travel"
- "Commission only"
- "Seasonal"

Skip this if nothing comes to mind.

### Question 5: What pay would make a job worth considering? (Optional)

Choose yearly or hourly pay, then add the minimum amount if you know it. Leave
it blank if you are not sure yet. Hourly setup pay is shown with its yearly
equivalent so pay-floor warnings still compare jobs consistently. Jobs without
listed pay, minimum-only pay, or "up to" pay stay visible and marked, and jobs
below your floor are warned about instead of silently hidden.

### Question 6: Where do you want to work?

Pick any combination:

- **Remote** - Work from anywhere
- **Hybrid** - Mix of office and home
- **On-site** - Full-time in the office

All three options start selected so your first search is broad. Uncheck any
work modes you do not want. JobSentinel will ask you to keep at least one
option selected, because a search with no work location cannot return useful
jobs.

### Question 7: Should fresh and verified jobs show first?

Pick how JobSentinel should handle older or hard-to-verify postings:

- **Fresh and verified first** - Warn earlier when a posting looks old,
  reposted, or hard to verify
- **Balanced** - Use normal posting-review alerts while keeping the list broad
- **Widest search** - Show more older postings and warn only when risk looks
  clearer

### Question 8: How many jobs do you want to review?

Pick how broad the first results and alerts should feel:

- **Smaller list** - Show fewer jobs and focus alerts on roles that most
  clearly fit your search
- **Balanced list** - Keep a manageable list without hiding useful roles
- **Broad discovery** - Show more possible roles, including adjacent ones that
  may still be worth a look

### Question 9: Want instant alerts? (Optional)

Desktop alerts start off. Turn them on if you want the easiest notification
path. They do not need another account.

Turn on **Quiet job-search mode** if you want desktop alerts without sound for
a more private or quieter search.

Email and chat alerts can be added later in Settings. Some apps ask you to
create a connection link so JobSentinel can send messages there. Skip this for
now and add it later if you want.

### Review your search

Before job checks start, JobSentinel shows the answers it will use to rank jobs:

- Jobs to look for
- Work to show more often
- Work to rank lower
- Location
- Freshness
- Review list
- Job sources
- Alerts
- Pay

If JobSentinel suggests outside job sources for the search, check only the
sources you want it to contact. Leave them unchecked to start with a local
saved search and add reviewed active sources later in Settings.

Change anything that looks wrong, then start finding jobs.

---

## Step 3: Review Jobs That Fit

JobSentinel is now checking the sources you allowed.

What JobSentinel does next:

- Every 2 hours, JobSentinel checks for new jobs.
- Each job is compared with your saved search.
- Roles that fit your search can trigger notifications if you set them up.
- Stale, reposted, or postings that need review are flagged with warnings.

### Your Dashboard

![Dashboard](../images/dashboard.png)

The dashboard shows:

- **Job List** - Every job found, sorted by closest fit
- **Posting Risk Filter** - Hide postings that need review first
- **Search Bar** - Find jobs by keyword, company, or location
- **Statistics** - See how many jobs fit your criteria

---

## Quick actions

### Manual Search

Don't want to wait 2 hours? Right-click the tray/menu bar icon and select
**"Search Now"** to check allowed job sources now.

### Bookmark Jobs

Found something interesting? Click the bookmark icon (or press `b`) to save it.

### Add Notes

Click the notes icon (or press `n`) on any job to add your own comments.

### Company Research

Click **Research company** to see local company details when JobSentinel has
them, such as industry, employer size, work mode, tools or systems, and links
to check official or public review pages.

---

## Optional keyboard shortcuts

You can use JobSentinel with the mouse. Keyboard shortcuts are optional.

Use `Cmd` on macOS and `Ctrl` on Windows or Linux.

| Key | What It Does                  |
| --- | ----------------------------- |
| `Cmd/Ctrl + K` | Open quick actions |
| `Cmd/Ctrl + 1-8` | Switch pages |
| `Cmd/Ctrl + ,` | Open settings |
| `Cmd/Ctrl + Z` | Undo the last action |
| `Cmd/Ctrl + Shift + Z` | Redo the last action |
| `/` | Jump to the search bar |
| `r` | Refresh the current view |
| `b` | Bookmark the selected job |
| `n` | Add notes to the selected job |
| `c` | View company details |
| `?` | Show all keyboard shortcuts |

Press `?` anytime to see the full list.

---

## Useful features

### Posting Review Alerts

JobSentinel flags stale, reposted, or hard-to-verify postings so you can
protect tailoring time:

- **Yellow** - Minor concerns (might be old)
- **Orange** - Multiple warning signs
- **Red** - Verify before tailoring

Use the posting-risk dropdown to hide these from your list or review them
separately.

### Resume Builder

Create professional resumes right inside JobSentinel:

1. Click **Resume Builder** in the sidebar
2. Answer the guided resume questions
3. Pick from 5 clean templates designed for readable applications
4. Export to PDF, Word (.docx), or JSON Resume (.json)

### Resume Match

Resume Match compares your resume with a job post and shows which evidence is
clear, missing, or worth rewriting. It does not promise hiring outcomes or trick
screening software:

1. Click **Resume Match** in the sidebar
2. Paste the job post
3. Choose or add a saved resume
4. Click **Review Match**
5. Get feedback on job-post evidence, readability, and truthful edits

If you use **Import from resume app**, copied resume details stay local and can
be restored during the same app session if you leave Resume Match to add a
saved resume.

### Application Assist

Prepare repeated application details while keeping each submission under your control:

1. Click **Prepare Form** on a job worth reviewing
2. JobSentinel opens the browser and prepares matching details
3. You review everything and submit it yourself only if the role still fits

**Important:** JobSentinel never submits applications without you.
See [Application Assist Guide](../features/application-assist.md) for setup.

---

## Changing Your Preferences

Want to adjust your job titles, salary, or location preferences?

1. Click the **Settings** icon in the sidebar
2. Update any field
3. Click **Save**

Your changes take effect immediately.

---

## Getting Notifications

### Easiest option: desktop alerts

Desktop alerts do not need another account.

1. Open Settings, choose **Sources & Alerts**, then find **Desktop
   Notifications**.
2. Turn desktop alerts on.
3. Save settings.

### Email

Email alerts are optional. Use them only if your email service gives you a
special password for apps or clear mail-sending instructions.

1. Open Settings, choose **Sources & Alerts**, then find **Email Alerts**.
2. Choose your email service when listed.
3. Follow the email-service steps shown in JobSentinel.
4. Save settings.

### Optional chat alerts

Chat alerts are helpful if you already use these tools. Skip them if you do
not.

#### Slack

1. Open Slack's alert-connection page from JobSentinel Settings.
2. Create a new connection link for your workspace.
3. Copy the connection link.
4. In JobSentinel, open Settings, choose **Sources & Alerts**, then paste it in
   **Slack Notifications**.

#### Discord

1. Right-click your Discord channel > Edit Channel > Integrations
2. Create a channel connection.
3. Copy the connection link.
4. In JobSentinel, open Settings, choose **Sources & Alerts**, then paste it in
   **Discord Notifications**.

#### Microsoft Teams

1. In your Teams channel, click More options > Workflows
2. Add a workflow that receives a Teams webhook request.
3. Copy the connection link.
4. In JobSentinel, open Settings, choose **Sources & Alerts**, then paste it in
   **Microsoft Teams Notifications**.

Existing Microsoft 365 connector links still work if Teams already created one
for your channel. For a new setup, use the Workflows link.

---

## Where's My Data?

Your saved job-search data stays on your computer by default. Core workflows do
not need a cloud account or a JobSentinel account. When you turn on job sources
or alerts, JobSentinel contacts only the sources or alert services you choose.

<details>
<summary><strong>Optional: where JobSentinel saves files</strong></summary>
<br>

You do not need to find app files for normal help. Use **Save Safe Support
Report** or **Copy Safe Support Report** instead. The locations below are for
extra support or your own inspection.

**Saved jobs and settings:** your local JobSentinel app data.

- Windows: saved in your local app data folder
- macOS: saved in your Application Support folder; JobSentinel keeps this
  folder private to your Mac account

**Saved passwords and notification details:** stored in your operating system's
secure vault.

- Windows: Credential Manager
- macOS: Keychain
- Linux: Secret Service

</details>

---

## When Something Does Not Work

### No Jobs Showing

1. Make sure your job titles aren't too specific
2. Try enabling more location options (Remote + Hybrid + Onsite)
3. If you set a pay amount, keep it at the lowest pay you would actually
   consider. Jobs without listed pay or with one-sided listed pay still stay
   visible and marked for review.
4. Right-click the tray icon > "Search Now" to check again now

### Alerts Do Not Arrive

1. Double-check your notification connection link in Settings
2. Make sure the notification channel is enabled
3. Test it by clicking **Test** in Settings

### Something Else Looks Wrong

1. Open Settings in JobSentinel.
2. Click **Save Safe Support Report** to create a local file, or click
   **Copy Safe Support Report** if you prefer to paste it.
3. Open **Send Feedback** if you want a report with replies.
4. Share the safe support report only if you want help. JobSentinel redacts
   known sensitive fields before copy or save, but review the report before
   sharing it.

Also include:

- What you were trying to do
- What happened instead
- Your OS version (Windows 11, macOS, etc.)

---

## Next Steps

Next useful steps:

- **Learn more** about [Ghost Detection](../features/ghost-detection.md)
- **Build your resume** with the [Resume Builder](../features/resume-builder.md)
- **Review applications faster** with [Application Assist](../features/application-assist.md)

---

**JobSentinel keeps practical job-search support local and under your control.**

*Your job-search data stays local unless you choose an external channel or
approve an external AI request.*
