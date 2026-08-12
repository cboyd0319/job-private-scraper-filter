<!-- Defines the shipped employer dossier, its evidence boundaries, and its safe research guidance. -->

# Employer Intelligence

JobSentinel provides a local employer dossier for each saved opportunity. It
helps users decide what to verify before spending more time on a role. It is an
evidence view, not an employer rating, fraud decision, legal conclusion, or
hiring prediction.

## Where It Appears

The dossier is part of the job-scoped Opportunity Case. The dashboard and an
interview record can open the same dossier when they have the saved job hash.
The older name-only company directory is not an evidence source.

Opening the dossier may create or reuse the job's local Opportunity Case. It
does not contact a website. It reads the saved job, the installed source
manifest and policy, and count-only local history. A visible saved-posting link
is a separate browser action. Following it discloses the URL request to that
destination.

## Evidence Shown

The minimum dossier keeps these facts separate:

- The saved employer name is an exact local label, not a canonical identity.
- The official employer domain remains unknown unless direct reviewed evidence
  exists. A Greenhouse or Lever host is a public ATS domain, not the employer's
  official domain.
- Role history shows first and last observation dates, repeated sightings, and
  repost count. A saved observation does not prove the role is still open.
- Pay clarity reports a valid saved range, one valid bound, no listed pay, or
  unusable saved pay. It does not compare against a private salary floor or make
  a legal-compliance claim.
- The application channel describes the recorded source class only.
- Local history contains counts for saved jobs, applications, interviews,
  offers, and terminal outcomes matching the exact saved employer name.
- Source details show the reviewed source identity, source class, observation
  date, manifest verification and expiry dates, confidence, policy and review
  references, parser version, and known coverage limitations.

Notes, contacts, recruiter details, interview text, resume content, private
salary floors, offer amounts, and event payloads do not enter the dossier.

## Freshness And Disagreement

Source freshness comes from the installed manifest's `verified_on` date plus
its `max_age_days`. Current, stale, review-required, policy-disabled,
policy-changed, review-expired, unavailable, and missing states remain distinct.
Staleness never deletes a dated posting observation or local history.

Evidence with different meanings is not merged into one conclusion. A current
source policy does not make a saved role current. A public ATS posting does not
verify the employer's official domain. Exact-name local history does not prove
that differently named employers are the same company. These limitations stay
visible as uncertainty and direct the user to verify the role and domain.

## Safety Guidance

JobSentinel keeps possible scam cues separate from ordinary posting age or
repost evidence. The dossier does not label an employer fraudulent. Users are
prompted to verify the role on an employer-controlled careers page, avoid fees
or equipment-payment requests, and withhold sensitive information until the
employer and role are confirmed.

The dashboard may show a separate possible-scam cue derived from the saved
posting text. That cue remains a verification prompt, not a verdict.

## Offline And Failure Behavior

The dossier is useful offline because it performs no refresh. Dated local
observations and aggregate history remain visible when a source record is stale
or unavailable. Missing, malformed, contradictory, or unsafe native projections
fail closed to the Opportunity Case's retryable generic error. Only exact
credential-free Greenhouse and Lever HTTPS posting links without query or
fragment data can be rendered.

The view uses semantic headings, lists, definition terms, source dates, and
status text. It does not depend on color. Long links wrap, the modal remains
keyboard accessible, and the layout must not overflow at narrow widths.

## Current Source Boundary

The shipped live-first minimum reuses already saved Greenhouse and Lever public
ATS observations and their reviewed manifests. It does not perform background
employer discovery, scrape review or salary sites, use account-backed sessions,
or query public registries. SEC, DOL, USCIS, WARN, Companies House, EU, India,
and other datasets require their own policy, license, fixture, parser, and
freshness review before they can become dossier sources.
