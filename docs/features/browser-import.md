<!-- Explains Browser Import setup, review, privacy, and recovery behavior. -->

# Browser Import Button

The browser import button adds the visible job posting you choose to a local
JobSentinel review list. You decide what to save. It is optional. You can still
add jobs from inside JobSentinel without setting this up.

## What It Does

- Adds jobs from official career pages and trusted public job pages when the
  page shows enough job details.
- Uses the browser page you already opened.
- Offers a separate **I Just Applied** button that creates an applied draft from
  the visible title, company, and page address.
- Sends the job only to the JobSentinel app running on your computer.
- Keeps the review list and saved jobs local.

## When To Use It

Use the browser import button when:

- You find a job while browsing outside JobSentinel.
- A job source does not load inside JobSentinel.
- You want to save one visible job without copying details by hand.

Some sites have rules that prohibit third-party page capture. JobSentinel
blocks those domains before adding anything to the review queue. LinkedIn is
currently blocked based on its reviewed user agreement and automation policy.
Use the LinkedIn Workbench to enter or paste only the details you choose.

## Set It Up

1. Open JobSentinel Settings.
2. Find **Install Browser Button**.
3. Turn on **Browser Import**.
4. Enter the public HTTPS address of the individual job page.
5. Click **Copy Browser Button**, or **Copy I Just Applied Button** when you
   want an applied draft.
6. Review the native confirmation showing the minimized page address and exact
   site. Cancel if either is unexpected.
7. Create a new browser bookmark.
8. Name it **Import to JobSentinel** or **I Just Applied**.
9. Paste the copied text where the bookmark stores the page address.
10. Save it to your bookmarks bar.

If another app is using the saved button setup number, JobSentinel chooses an
available local number and saves it for next time. Settings tells you when this
happens. Copy a fresh browser button before importing.

Each copied button works once for the confirmed site and expires after about
ten minutes. Copy a fresh button for every import, after changing the button
setup number, or after closing and reopening JobSentinel.

Your browser may ask whether the confirmed site can connect to devices or apps
on your local network. This permission is required for that site to reach the
JobSentinel receiver on your computer. Grant it only when the displayed site
matches the site you just confirmed in JobSentinel.

## Review And Save Jobs

1. Open a supported public job posting in your browser.
2. Use the **Import to JobSentinel** bookmark.
3. Wait for the confirmation message.
4. Open JobSentinel Settings and find **Jobs waiting for review**.
5. Check each job, then click **Save Job** or **Skip**.

If jobs are missing, copy the browser button again and retry. If some details
are missing, save the job and edit it afterward, or skip it and add it manually.

## Review Applied Drafts

The **I Just Applied** button uses its own exact one-use permission. It reads
only the visible title, visible company, and public page address. It never reads
the description or structured page data.

An applied draft enters the same local review list. Missing title or company
fields are labeled before you save. **Save Applied Draft** creates or updates
the local application record with Applied status. **Skip** discards the pending
draft. LinkedIn and Y Combinator pages remain blocked from this browser action;
use the LinkedIn Workbench or manual entry instead.

## Where It Works Best

The browser import button works best on:

- Company career pages.
- Company application pages.
- Trusted public pages that show the full job description, employer name, and
  location.
- Pages you opened yourself in your browser.

Some job search result pages may not include enough job details. Some large job
boards also do not allow this kind of save. JobSentinel does not get around those
limits; use JobSentinel's search link or add the job manually when import does
not work.

## Troubleshooting

### Cannot Connect To JobSentinel

- Make sure JobSentinel is open.
- Turn on **Browser Import** in Settings.
- Leave **Button setup number** unchanged unless JobSentinel help instructions
  tell you otherwise. JobSentinel handles a number already in use.
- Copy a fresh browser button if you already attempted an import, restarted
  JobSentinel, or waited about ten minutes.
- If your firewall asks, allow connections for JobSentinel.
- If the browser denies local network access, allow it for the confirmed site
  in the browser's site settings, then copy a fresh browser button.

### Job Already Exists

JobSentinel found the same job in your saved jobs. Open your jobs list and
search for the company or title.

### Job Did Not Appear For Review

- Make sure you are on an individual job page at the site confirmed by
  JobSentinel.
- Try opening the job on the company career site.
- Some job sites block browser buttons from sending to apps on your computer.
  If that happens, copy the job link and use **Import Job from Link** inside
  JobSentinel.
- Add the job manually if the page does not include enough details.

### Missing Details

Some sites hide details or load them after the page opens. Save the job only if
the review looks useful, then fill in any blank details in JobSentinel.

## Privacy

- JobSentinel creates the browser button on your computer.
- The renderer sends only the job page address to Rust. Rust validates and
  minimizes it before a native confirmation.
- A private pairing secret stays in Rust memory and the copied one-use button.
  It never enters renderer responses, configuration, logs, pending review
  records, durable storage, or safe support reports.
- The pairing is bound to the confirmed public HTTPS origin, works for one
  request, and expires after about ten minutes. Starting, stopping, or replacing
  Browser Import revokes the active pairing.
- The local receiver requires the browser `Origin` header, the paired origin,
  and every submitted job URL to agree.
- The button sends only to the JobSentinel app running on your computer.
- The button checks restricted sites and its approved origin before reading the
  page. It reads visible posting fields only, not cookies, authorization
  headers, browser storage, hidden page state, screenshots, or network traffic.
- If copying fails, the previous pairing remains unchanged.
- Job data stays local unless you choose to share it.
- LinkedIn page capture is blocked before data enters the local review queue.
- Rust rechecks the installed source policy and manifest before queueing and
  again before saving. If authority changes, the item remains pending.
- Browser Import jobs are not durable saved jobs until you click **Save Job** in
  the review list. Applied drafts are not durable until you click
  **Save Applied Draft**.
- Safe support reports must redact the browser button details and saved jobs
  details.
