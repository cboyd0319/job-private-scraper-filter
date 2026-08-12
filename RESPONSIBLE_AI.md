# Responsible AI

JobSentinel uses AI-adjacent and automation-adjacent features only to help job
seekers understand, organize, and protect their own search. It is candidate-side
software, not an employer screening system.

External AI is optional, disabled by default, and routed through one AI gateway.
Core workflows must work locally. Sensitive job-search data must stay local
unless the user opts in, reviews the exact request details, and approves sending
them.

Rule 0: user privacy and security are non-negotiable. AI features must never
weaken local-first storage, opt-in, request review, redaction, approval,
credential safety, source boundaries, or human review. If an AI feature cannot
meet that bar, it must not ship.

## Boundaries

JobSentinel does not:

- Submit application spam or bulk application batches.
- Encourage deceptive resume optimization.
- Use hidden keyword stuffing.
- Support resume prompt injection.
- Manipulate employer screening systems.
- Solve CAPTCHAs.
- Collect from restricted sites or evade platform controls.
- Upload the user's entire local database.
- Silently send resumes, notes, salary floors, or application history.
- Collect private candidate data for research without informed consent.
- Invent skills, credentials, employers, titles, dates, offers, or legal claims.
- Infer protected-class traits for scoring or salary guidance.
- Make final employment decisions.
- Present fit estimates as hiring guarantees.

JobSentinel supports:

- Candidate-side explainability.
- Readable application materials for employer screening systems.
- Ghost-job risk explanations.
- Truthful application readability.
- Salary transparency analysis.
- Pay-equity support grounded in role scope, range evidence, and user facts.
- Privacy-preserving job-search workflows.
- Human review before any form is filled or submitted.

## External AI Use

AI may help with:

- Summarizing job descriptions.
- Extracting structured job-posting fields.
- Explaining job fit from user-selected data.
- Explaining ghost-job or stale-posting risk.
- Improving truthful application readability.
- Checking salary transparency.
- Preparing compensation questions.
- Generating documentation or synthetic test fixtures.

Every external AI request needs opt-in, only the needed details, a clear review
screen before anything leaves the device, user approval, an exact single-use
backend approval, and a durable metadata-only privacy receipt. Sensitive details
need explicit selection by the user. Public-data-only requests must not include
private notes, application history, resumes, salary floors, or unrelated local
data.

## Hiring-system transparency

JobSentinel helps users see whether an application is readable, complete, and
truthful. It should explain what it found, what is uncertain, and what the user
can edit.

Acceptable guidance:

- Show parsed resume sections so users can catch formatting problems.
- Explain required, preferred, and nice-to-have job requirements.
- Suggest clearer wording only when it maps to user-confirmed experience.
- Warn about hidden text, stuffed keywords, prompt-like instructions, and
  image-only resumes.

Non-goals:

- Trick employer systems.
- Optimize for a guessed hidden model.
- Hide content from human readers.
- Make claims that the user has not confirmed.

## Salary and negotiation guidance

Salary guidance must avoid false certainty and user blame. Pay gaps can reflect
salary opacity, prior-pay anchoring, bias, occupational segregation, unequal
access to referrals, unclear promotion criteria, and discretionary pay setting.

JobSentinel should:

- Treat salary floors as user-controlled walk-away numbers.
- Compare offers against range evidence, role scope, level, location, benefits,
  schedule, and promotion path.
- Warn when a target appears below credible market evidence.
- Help users redirect salary-history questions to role range and target pay.
- Keep legal and policy claims dated, sourced, and scoped.

## Ghost-job guidance

Ghost-job detection is risk guidance, not a claim that JobSentinel knows employer
intent. Explanations should use cautious labels such as stale, unverified,
closed, evergreen, or high risk.

JobSentinel should:

- Prefer company or official hiring-platform sources when available.
- Track first seen, last seen, reposts, closure signals, and source quality.
- Separate ghost-job risk from scam risk.
- Give users recovery actions: verify, save anyway, hide, mark stale, or undo.

## Human control

JobSentinel must keep the user in charge:

- The user chooses sources, filters, salary floors, and external channels.
- The user reviews application forms before anything is sent.
- The user can edit generated notes before using them.
- The user can export, delete, or stop using local data.
- The user can generate a sanitized report when something breaks.

AI outputs are advisory. Users remain in control.
