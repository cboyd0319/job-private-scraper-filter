# Resume Match

**Compare a resume with a job posting locally, then decide where careful
tailoring is worth the time.**

Resume matching helps job seekers see how their resume lines up with a role
without sending the resume to a cloud service by default. It is not a promise
that an employer will respond, and it is not a tool for deceptive resume tricks.
It is a local, advisory signal for readability, fit, and preparation.

![Resume Match interface](../images/resume-matcher.png)

## Privacy Labels

| Workflow | Label | Default behavior |
| --- | --- | --- |
| Resume add and parsing | Local only, Sensitive | Resume text and file details stay local. |
| Readable text preview | Local only, Sensitive | The user can explicitly open and copy a bounded preview of text JobSentinel read from the selected resume. |
| Resume library | Local only, Sensitive | Resume versions stay on this device. |
| Skill review and edits | Local only, Sensitive | User edits stay local. |
| Reviewed-skill sorting preference | Local only, Sensitive | A user can explicitly use reviewed local skills as one job-sorting signal. |
| Resume/job fit review | Local only, Sensitive | Resume data is compared with saved job data locally. |
| Job posting text | Public-data only | Job descriptions are public or user-saved posting content. |
| Scanned resume PDFs | Local only, Sensitive | If enabled, JobSentinel tries to read scanned resume text on this device. |

External AI is not required for resume matching.

Local semantic matching refuses resume skills, job requirements, queries, or
candidates that contain shared instruction-override phrases or the invisible
format controls U+200B, U+2060 through U+2064, or U+FEFF. Other Unicode format
controls are ignored during phrase inspection, so bidirectional text, Persian
joiners, and emoji sequences remain valid but cannot conceal an override phrase.
Rejection stops before local model dispatch and returns a content-free
review-required error. It does not log, persist, or send the rejected value.
Security work such as "prompt injection testing" also remains valid input.

Requirement diagnostics are rebuilt locally from one database snapshot of the
current case-file job description, saved resume text, saved skills, resume
revision, job revision, and reviewed case evidence. Direct, Strong, Partial, and
Implied states require exact same-case UserConfirmed citations. Missing results
cannot carry citations. The in-memory result exposes only its case identifier,
job revision, requirement, importance, bounded match state, opaque evidence
IDs, hard-constraint status, and a bounded Partial, Implied, or Missing "why
not" reason. It does not trust caller-supplied match state, recommendation text,
importance, blockers, or citations, and it performs no write or external AI
call.

## What It Helps With

- **Resume readability**: Confirm and copy the text JobSentinel can read from
  the selected resume.
- **Readable structure review**: Flag missing top contact details, missing
  standard section headings, table-like extracted text, hidden instructions,
  and other practical readability risks.
- **Fit review**: Compare resume skills, experience, and education with a job
  posting.
- **Truthful tailoring**: See which real experience might be worth making more
  visible.
- **Requirement inventory**: Separate must-have requirements, high-value terms,
  supporting terms, and words that should not be forced into the resume.
- **Knockout consistency review**: Notice when required form answers should be
  supported by visible resume evidence, such as licenses, work authorization,
  location, required tools, or years of experience.
- **Gap awareness**: Notice missing or weakly represented requirements before
  spending time on a role.
- **Resume safety review**: Flag prompt-injection-like instructions, hidden
  text, and invisible characters with a plain **Safety check** label before
  the resume is used.
- **Reviewed-skill sorting**: After reviewing saved skills, choose whether
  those local skills should help sort jobs alongside titles, search words,
  salary, location, and company preferences.
- **Multiple resumes**: Keep different resume versions for different kinds of
  work.
- **Broad career coverage**: Recognize skills from technical and non-technical
  fields, including operations, healthcare, education, sales, marketing,
  customer support, finance, legal, creative, data, and security work.

## Everyday Workflow

1. Open **Resume Match**.
2. Paste the job post.
3. If you already have an active saved resume, JobSentinel selects it for the
   review automatically. Choose **Choose or Add Resume** only when you need to
   add, change, or refresh the selected resume.
4. Choose **See what JobSentinel read** if you want to inspect the readable
   resume text before using match results.
5. Use **Import from resume app** only if another resume app gave you export
   text. Copied resume details stay local and can be restored during the same
   app session if you leave Resume Match to add a saved resume.
6. Click **Review Match**.
7. Review suggested skills and add anything important that was missed.
8. Choose **Use these skills to sort jobs** if the reviewed local skills should
   influence job sorting.
9. Open job details from the dashboard to see recent resume fit reviews.
10. Use skills found in both places and skills to review as evidence for a
   decision:
   tailor carefully, save for later, ask a question, or skip.

JobSentinel should explain fit in plain language. A person should not need to
understand parsing, scoring, or employer screening systems to use the result.
JobSentinel should also make clear that local fit and readability results are
diagnostics, not predictions of how a specific employer will screen or respond.

## How To Read Fit Results

| Signal | Meaning | Use it for |
| --- | --- | --- |
| Overall fit | Combined fit signal from skills, experience, and education. | Decide whether the role deserves more attention. |
| Skills fit | Resume skills that appear relevant to the posting. | Find real strengths to make clearer. |
| Experience fit | Years or level signals found in the posting and resume. | Notice lower-title, lower-pay, or stretch-role risk. |
| Education fit | Degree or credential signals found in the posting and resume. | Spot requirements that may need explanation. |
| Skills to review | Posting requirements not clearly represented in the resume. | Decide whether to revise, ask, learn, or skip. |
| Required, preferred, or nice-to-have wording to review | Missing job-post language grouped by importance. | Start with required evidence before reviewing nice-to-have or other wording. |

Low fit does not mean "do not apply." It means "review fit before spending
extra time." Strong fit does not guarantee a response. It means the resume and
posting share stronger visible evidence.

Resume quality and role fit are separate. A clear resume can still be a poor
fit for a role, and a plausible role fit can still need format cleanup before
submission.

## Responsible Use

Resume matching must stay candidate-side and honest:

- Do not fabricate qualifications.
- Do not hide keywords.
- Do not stuff resumes with unrelated terms.
- Do not prompt-inject resumes.
- Do not present the fit estimate as an employer decision.
- Do not encourage users to apply to roles that violate their salary floor,
  location constraints, schedule needs, or other must-haves.

The right goal is application readability and truthful fit, not manipulating
employer screening systems.

## Local Matching Model

The current local matcher:

- extracts readable text from PDF, DOCX, TXT, Markdown, and HTML resumes;
- keeps selected resume uploads local and rejects files over 10 MB before
  copying them into managed local storage;
- routes an explicitly confirmed dropped resume through the same validation,
  managed storage, and Resume-page review path without exposing its source path;
- shows the resume format and whether readable text is available before review;
- provides an explicit local preview of readable resume text without returning
  the saved file path;
- shows a plain preview checklist for found text, employer-format priority, and
  whether the visible preview is complete, clipped, or unavailable;
- reminds users that important details such as contact details, roles, skills,
  and credentials should be selectable text instead of photos, logos, or
  image-only sections;
- tells users to follow employer file instructions first when readable text is
  missing, then suggests readable PDF, DOCX, TXT, Markdown, or HTML when no
  format is named;
- tells users when a PDF may be scanned or image-only because local parsing
  could not find selectable text;
- uses the same employer-instructions-first guidance inside the empty
  readable-text preview;
- checks the extracted text for common application-readability risks such as
  missing top contact details, missing standard section headings, table-like
  text, hidden instruction-like content, CSS-like white or tiny text markers,
  HTML comments, metadata tags, and work bullets that read like keyword lists,
  generic filler, or mix ownership with exposure-only wording instead of plain
  work evidence;
- checks HTML resume source for table layouts, multi-column CSS, risky
  decorative fonts, custom web-font dependencies, and very small font sizes
  before treating the resume as safe for submission;
- treats **Career Break**, **Career Pause**, and caregiving labels as readable
  resume headings instead of structural mistakes;
- treats **Volunteer Experience**, **Community Involvement**, and **Military
  Service** as readable resume headings and experience evidence instead of
  structural mistakes;
- treats **Employment History**, **Work History**, and **Professional History**
  as readable resume headings and experience evidence instead of structural
  mistakes;
- treats **Relevant Experience**, **Selected Experience**, and
  **Additional Experience** as readable resume headings and experience evidence
  instead of structural mistakes;
- treats **Academic Background**, **Academic History**, and
  **Education Background** as readable resume headings and education evidence
  instead of structural mistakes;
- treats **Licenses & Certifications**, **Licenses and Certifications**, and
  **Certifications and Licenses** as readable resume headings and credential
  evidence instead of structural mistakes;
- can review the active saved resume against a pasted job post without copying
  structured resume details into the page;
- loads the active saved resume on page open so the user can paste a job post
  and review fit without knowing any technical setup steps;
- shows the selected resume format and readable-text status before match review
  without exposing saved file paths or raw resume text;
- shows a Resume Fit evidence-status label, such as checking must-haves first,
  mixed evidence, not enough job detail, or clearer evidence, so the score is
  treated as local evidence review instead of an employer prediction;
- keeps the Resume Fit evidence-status label at mixed evidence when required
  job-post wording is missing, partial, or only implied, unless the row is a
  hard-requirement review that needs a **Check must-haves first** status;
- tells users in the Resume Fit card that the result is local evidence review,
  not a hiring prediction or a promise about employer systems;
- treats malformed local score values as unavailable, while keeping words
  found, words to review, hard requirements, and next actions visible;
- shows readable format and details-included signals in a separate
  **Resume Quality** section instead of mixing them into role-fit review;
- identifies skills and job-post terms across broad career categories,
  including healthcare, education, service, operations, trades, legal, finance,
  and government or administrative work;
- lets reviewed local skills influence job sorting only after the user turns
  that preference on;
- compares resume evidence with saved job-posting text;
- returns fit, experience, education, matched-skill, and missing-skill
  signals;
- preserves whether missing job-post language came from required, preferred,
  or nice-to-have or other role-language context;
- labels the third missing-word bucket as **Nice-to-Have or Other to Review**
  so optional job-post language is not mistaken for a hard requirement;
- reminds users in the job words overview not to force words they cannot
  support with real work, training, or credentials;
- adds requirement-review rows for recognized local job-post keywords with
  visible evidence, needs-support, check-wording, and not-found states;
- labels readable resume evidence under an Experience role with a present-date
  marker as current role evidence, and resets that label when a later past-role
  date range appears;
- labels structured or readable Experience evidence from roles ending in the
  current or previous calendar year as recent role evidence;
- treats visible current-role or recent-role evidence hits as stronger than the
  same single hit in an older role, without discarding older role evidence;
- treats work or project evidence with visible metrics such as percentages,
  counts, or dollar amounts as stronger local evidence than a bare keyword;
- treats work or project evidence with scope such as work across teams,
  departments, locations, sites, regions, markets, or service lines as stronger
  local evidence than a bare keyword;
- treats work or project evidence with ownership or management verbs tied to
  workflows, processes, programs, operations, intake, cases, systems, or tools
  as stronger local evidence than a bare keyword;
- treats work or project evidence with clear task verbs tied to concrete
  requests, appointments, records, orders, cases, tickets, reports, files,
  forms, calls, emails, inquiries, intake, follow-ups, tasks, or schedules as
  stronger local evidence than a bare keyword;
- flags recognized missing or weakly supported hard requirements such as
  authorization, location, citizenship, schedule, availability, commute or
  transportation, travel, years of experience, language fluency, background or
  drug screening, minimum age or legal work age, driving record, auto or car
  insurance, insured vehicle, physical demands, license, certification, degree,
  or clearance and limits the fit label until the user verifies the
  requirement;
- treats explicit **degree or equivalent experience** wording, including
  equivalent combinations of education and experience, as
  experience-compatible evidence instead of a missing exact-degree hard cap;
- treats recognized required seniority language, such as senior-level
  experience, as a local experience constraint and checks for visible role,
  leadership, or enough-years evidence before raising the fit label;
- warns that lower-title or fewer-years evidence may not satisfy a higher
  seniority requirement, without telling users to round up titles or years;
- treats **shift lead**, **crew lead**, **lead worker**, and **lead
  experience** as lead-level evidence for required lead constraints;
- treats **supervisor experience**, **supervisory experience**,
  **team supervision**, **supervising staff**, **supervised staff**,
  **managed a team**, and **managed staff** as management-experience evidence
  for required supervisor or management constraints;
- treats **US citizenship**, **U.S. citizenship**, **US citizen**, and
  **U.S. citizen** as the same local citizenship evidence without treating
  generic work authorization as citizenship, and reports citizenship as its own
  hard-review category;
- treats **work authorization** and **authorized to work** as the same local
  work-eligibility evidence;
- treats **security clearance** and **clearance** as the same local clearance
  evidence;
- treats bilingual and fluency wording for known human languages, including
  Spanish, French, Mandarin, Cantonese, Arabic, Portuguese, German, Japanese,
  and Korean, as the same local language evidence for required language
  constraints;
- treats minimum-age wording such as **18 years of age**, **years old**,
  **minimum age**, **age requirement**, and **legal work age** as a hard review
  item to verify before tailoring, without treating it as years of experience
  or telling users to add age to a resume;
- treats **background check**, **background screening**, **drug screen**,
  **drug test**, and **pre-employment screening** as local screening
  requirements to verify before tailoring;
- treats conservative local term equivalents, such as **CRM** and **customer
  relationship management**, and **customer service**, **customer support**,
  **client service**, **client services**, **client support**, **guest
  service**, **guest services**, **front desk**, **front-desk**, **reception**,
  and **receptionist**, as the same evidence without broad fuzzy matching;
- treats **case management** and **case coordination** as the same local
  evidence without treating broader case-documentation wording as enough by
  itself;
- treats **scheduling**, **calendar management**, and **appointment setting**
  as the same local evidence while still avoiding broad fuzzy matching;
- treats **onboarding**, **new hire orientation**, and **employee orientation**
  as the same local service, HR, or training evidence while still avoiding broad
  fuzzy matching;
- treats **training**, **trained**, **staff training**, **employee training**,
  and **team training** as the same local training evidence while still avoiding
  broad fuzzy matching;
- treats **quality assurance** and **QA** as the same local evidence while
  still avoiding broad fuzzy matching;
- treats **patient care** and **patient-care** as the same local evidence while
  still avoiding broad fuzzy matching;
- treats **student support** and **student services** as the same local education
  evidence while still avoiding broad fuzzy matching;
- treats **parent communication**, **family communication**, and
  **guardian communication** as the same local education evidence while still
  avoiding broad fuzzy matching;
- treats **medical record**, **medical records**, **medical-record**, and
  **medical-records** as the same local evidence while avoiding duplicate
  requirement rows for singular, plural, or hyphenated job wording;
- treats **care plan**, **care plans**, **care-plan**, and **care-plans** as
  the same local evidence while avoiding duplicate requirement rows for
  singular, plural, or hyphenated job wording;
- treats **vital sign**, **vital signs**, **vital-sign**, and **vital-signs**
  as the same local evidence while avoiding duplicate requirement rows for
  singular, plural, or hyphenated job wording;
- treats **medication administration** and **medication-administration** as the
  same local evidence while avoiding duplicate requirement rows for hyphenated
  job wording;
- treats **data entry** and **data-entry** as the same local evidence while
  still avoiding broad fuzzy matching;
- treats **data analysis**, **data analytics**, **data-analysis**,
  **data-analytics**, and **analytics** as the same local data evidence while
  still avoiding broad fuzzy matching;
- treats **bookkeeping** and **bookkeeper** as the same local finance evidence
  without broad fuzzy matching;
- treats **QuickBooks** and **QBO** as the same local finance evidence without
  broad acronym matching;
- treats **A/P** and **A/R** as shorthand for **accounts payable** and
  **accounts receivable** without broad acronym matching;
- treats **point of sale**, **POS system**, and **POS systems** as the same
  local retail/service evidence without matching bare POS initials;
- treats **budgeting** and **budget tracking** as the same local finance and
  operations evidence without broad fuzzy matching;
- treats **cashier** and **cash handling** as the same local retail/service
  evidence without broad fuzzy matching;
- treats **inventory**, **inventory control**, **inventory management**, **stock
  control**, **stock management**, and **stockroom** as the same local
  operations evidence without broad fuzzy matching;
- treats **logistics**, **shipping**, **receiving**, and **shipping and
  receiving** as the same local operations evidence without broad fuzzy
  matching;
- treats **procurement** and **purchasing** as the same local operations
  evidence without broad fuzzy matching;
- treats **vendor management** and **supplier management** as the same local
  operations evidence without broad fuzzy matching;
- treats **document review** and **document-review** as the same local evidence
  while avoiding duplicate requirement rows for hyphenated job wording;
- treats **records management** and **records-management** as the same local
  evidence while avoiding duplicate requirement rows for hyphenated job wording;
- treats **case files** and **case-files** as the same local evidence while
  avoiding duplicate requirement rows for hyphenated job wording;
- treats **legal research** and **legal-research** as the same local evidence
  while avoiding duplicate requirement rows for hyphenated job wording;
- treats **policy analysis** and **policy-analysis** as the same local evidence
  while avoiding duplicate requirement rows for hyphenated job wording;
- treats **grant administration** and **grant-administration** as the same local
  evidence while avoiding duplicate requirement rows for hyphenated job wording;
- treats **financial reconciliation** and **financial-reconciliation** as the
  same local evidence while avoiding duplicate requirement rows for hyphenated
  job wording;
- treats **billing** and **invoicing** as the same local finance evidence
  without broad fuzzy matching;
- treats **loan processing** and **loan-processing** as the same local evidence
  while avoiding duplicate requirement rows for hyphenated job wording;
- treats **onsite**, **on-site**, and **on site** as the same local location
  evidence for required location constraints;
- treats **remote work**, **remote role**, and related remote wording as the
  same local location evidence for required remote-work constraints;
- treats **reliable internet**, **high-speed internet**, **home office**,
  **quiet workspace**, and **dedicated workspace** as local evidence for
  required remote-work setup constraints;
- treats **hybrid work**, **hybrid role**, and **hybrid schedule** as the same
  local location evidence for required hybrid-work constraints;
- treats **relocation**, **relocate**, and **willing to relocate** as the same
  local location evidence for required relocation constraints;
- treats **reliable transportation** and **own transportation** as the same
  local location evidence for required commute or transportation constraints;
- treats **reliable vehicle**, **insured vehicle**, **proof of auto
  insurance**, **proof of insurance**, **auto insurance**, **car insurance**,
  and **vehicle insurance** as local vehicle or insurance evidence for required
  field-work constraints;
- treats **commute** and **commuting** as the same local commute evidence;
- treats **night shift**, **overnight shift**, **third shift**, and **3rd
  shift** as the same local schedule evidence for required night-shift
  constraints;
- treats **weekend availability**, **weekend shift**, and **weekend shifts** as
  the same local schedule evidence for required weekend constraints;
- treats **evening shift**, **second shift**, and **2nd shift** as the same
  local schedule evidence for required evening-shift constraints;
- treats **day shift**, **first shift**, and **1st shift** as the same local
  schedule evidence for required day-shift constraints;
- treats **overtime availability**, **overtime**, **overtime shift**, and
  **overtime shifts** as the same local schedule evidence for required
  overtime constraints;
- treats **holiday availability**, **holiday**, **holiday shift**, and
  **holiday shifts** as the same local schedule evidence for required holiday
  constraints;
- treats **availability** and **available** as the same local schedule evidence
  for required availability constraints;
- treats **full-time availability**, **full time**, **part-time availability**,
  and **part time** as local schedule evidence for required schedule
  constraints;
- treats lift-weight wording with the same number, such as **lift 50 lbs** and
  **lift 50 pounds**, as the same local physical-demand evidence;
- treats **stand for long periods** and **standing for long periods** as the
  same local physical-demand evidence;
- treats clear credential equivalents, such as **BLS** and **Basic Life
  Support**, as the same evidence without guessing unrelated credentials;
- treats **Security+** and **Security Plus** as the same local credential
  evidence;
- treats **CISSP** and **Certified Information Systems Security Professional**
  as the same local credential evidence;
- treats **driver's license**, **drivers license**, and **driver license** as
  the same local license evidence;
- treats **clean driving record**, **acceptable driving record**, **driving
  record**, **MVR**, and **motor vehicle record** as the same local driving
  screening evidence;
- treats **CDL**, **commercial driver's license**, **commercial drivers
  license**, and **commercial driver license** as the same local commercial
  driving-license evidence without adding a duplicate generic driver-license
  gap when the commercial requirement is matched;
- treats **RN**, **RN license**, **Registered Nurse**, and **Registered Nurse
  license** as the same local nursing-license evidence, including full
  Registered Nurse license wording in job posts;
- keeps short credential checks bounded to full terms, so **RN** is not found
  inside unrelated words such as **intern**;
- treats CNA, Certified Nursing Assistant, Certified Nurse Assistant, and
  Certified Nurse Aide as the same local credential evidence, and avoids a
  duplicate generic certification risk when the specific credential matches;
- treats LPN, Licensed Practical Nurse, LVN, and Licensed Vocational Nurse as
  the same local credential evidence while still telling users to verify the
  license before applying;
- treats PMP, Project Management Professional, PMP certification, and Project
  Management Professional certification as the same local credential evidence;
- treats food safety, food safety certification, ServSafe, and food handler,
  food-handler, food handler's, or food handlers certification, certificate,
  permit, or card wording as the same local credential evidence;
- treats first aid, first-aid, first aid certification, First Aid Certified,
  and first aid certificate wording as the same local credential evidence;
- treats forklift, forklift certification, forklift certified, forklift
  operator certification, and forklift license wording as the same local
  credential evidence;
- treats OSHA 10, OSHA10, OSHA 10 certification, and OSHA 10-hour wording as
  the same local credential evidence without treating OSHA 30 as equivalent;
- treats OSHA 30, OSHA30, OSHA 30 certification, and OSHA 30-hour wording as
  the same local credential evidence without treating OSHA 10 as equivalent;
- treats high school diploma, high-school diploma, high school degree,
  high-school degree, GED, high school equivalency, high-school equivalency,
  and General Education Development as the same local education evidence
  without guessing unrelated credentials;
- treats **associate's degree**, **associate degree**, **associates degree**,
  **Associate of Applied Science**, **Associate of Arts**, and **Associate of
  Science** as the same local education evidence;
- treats **bachelor's degree**, **baccalaureate degree**, **bachelor degree**,
  **bachelors degree**, **Bachelor of Applied Science**, **Bachelor of Arts**,
  **Bachelor of Business Administration**, **Bachelor of Education**,
  **Bachelor of Engineering**, **Bachelor of Fine Arts**, and **Bachelor of
  Science**, and **Bachelor of Social Work** as the same local education
  evidence for generic bachelor's requirements;
- treats **master's degree**, **master degree**, **masters degree**,
  **Master of Arts**, **Master of Business Administration**,
  **Master of Education**, **Master of Engineering**, **Master of Fine Arts**,
  **Master of Science**, and **Master of Social Work** as the same local
  education evidence for generic master's requirements;
- treats **PhD**, **doctorate**, **doctorate degree**, and **doctoral degree**
  as the same local education evidence;
- treats headings such as **Training**, **Credentials**, and **Certificates**
  as credential evidence and readable resume headings when reviewing readable
  resume text;
- turns the local review into plain next actions such as checking a hard
  requirement before tailoring, adding supporting evidence only if true, or
  keeping useful evidence visible;
- adds an interview-defense reminder to drafted alternative bullets so users
  check the problem, role, action, result, and evidence before using a clearer
  action verb;
- preserves vague user wording such as **was responsible for**, **worked on**,
  and **helped with** instead of turning it into unverified ownership or
  development claims;
- adds a healthcare and licensed-work evidence prompt to drafted alternative
  bullets when job details mention patient care, medication administration, or
  nursing credentials;
- adds a trades and field-service evidence prompt to drafted alternative
  bullets when job details mention maintenance, equipment repair, field
  service, forklifts, OSHA, work orders, installation, or trade work;
- adds a career-change evidence prompt to drafted alternative bullets when job
  details mention career transitions, returnships, return-to-work paths, or
  transferable experience;
- adds an early-career evidence prompt to drafted alternative bullets when job
  details mention entry-level, trainee, apprenticeship, internship, or new
  graduate paths;
- adds a regulated-work evidence prompt to drafted alternative bullets when job
  details mention legal, finance, government, records, policy, grants, or audit
  work;
- adds a service and operations evidence prompt to drafted alternative bullets
  when job details mention customer service, support, case management,
  scheduling, intake, or operations work;
- adds a technical and data evidence prompt to drafted alternative bullets when
  job details mention software, data analysis, machine learning, dashboards,
  APIs, or product work;
- adds a sales and marketing evidence prompt to drafted alternative bullets
  when job details mention sales, pipeline, account, retention, marketing,
  campaign, conversion, or revenue work;
- gives hard-requirement next actions category-specific honesty guidance,
  such as checking work authorization, clearance, licenses, education, years
  or level, minimum age or legal work age, background or drug screening,
  physical demands, location, schedule, availability, and travel before
  tailoring, and keeps hard-requirement review rows first even when the backend
  sends the row without a separate hard-risk item;
- shows those must-have checks in the live Resume Builder detail review when
  available, and shows a compact pre-detail warning in the live Resume Builder
  panel using plain category labels instead of backend risk names or score caps;
- keeps required and preferred job-post headings separate even when the posting
  uses ordinary single-line section breaks;
- treats job posts with too little recognized requirement detail as
  insufficient evidence instead of a perfect match;
- stores results locally so recent job comparisons can be reviewed later.
- shows only the scoring signals actually used in each recent local result,
  and, when embedded local scoring applies a limit, shows its first stable
  evidence blocker as plain **Why not** and **Scoring sources** lines.
- lets the user mark a saved match **Useful** or **Not relevant** and clear the
  label. Feedback stays local, stores only the saved match ID, closed label, and
  time, and does not change ranking or retain extra resume or job content.

The v3 storage foundation keeps derived vectors in the local database as bounded
little-endian `f32` bytes with model and content provenance. It stores no source
text in the vector table and excludes the table from reviewed plaintext export.
A read returns bytes only when model availability, provenance, dimension, and
every value remain valid; otherwise it removes the row and requests a rebuild.
Encrypted portable backups include the local table. Row deletion is lifecycle
cleanup, not a secure-erasure claim.

The embedded semantic command caches only the first skill in the existing
deterministic skill order. It rebuilds that local vector when content or model
provenance changes and removes it when the skill set changes, the resume is
deleted, or the governed Qwen3 runtime is unavailable. Remaining skill
vectors are generated in memory for that request. MiniLM and model-free
installs keep their existing local matching behavior and never consume the
Qwen3 cache. The command returns one same-order citation for every positive
match, bound to the exact skill in the same revision-guarded local order. It
returns no partial result when any skill is missing or ambiguous, and returns
no result when the resume changes or is deleted during matching. Citations
contain no resume content, filesystem path, source identifier, or raw revision.
This command boundary is not yet the visible Resume Match page.

Settings calls the Model Doctor surface **Local Match Check**. It reports the
active local profile, privacy mode, fallback, model revision, backend, license,
dimension, required-file count, and cache integrity. Missing or incomplete
models keep built-in matching available. Integrity-invalid lock-owned caches
can be removed only after native review; stale vector provenance is rebuilt at
the application boundary rather than presented as current evidence.

The visible active saved-resume review is application-owned. It brackets the
existing local analysis with exact active-resume, revision, parsed-text, file,
bounded HTML-source, and ordered-skill reads. A change or deletion refuses the
whole result with a safe retry action instead of returning stale citations. HTML
format-source reads use one file handle and stop after the 10 MiB boundary plus
one sentinel byte. The result shape and visible review remain unchanged.

Copied structured-resume review is also application-owned. It discards any
renderer-supplied evidence identity, derives an opaque local revision from
canonical resume content, and stops before analysis when that content exceeds
10 MiB. The copied resume and its derived identity are not persisted.

The skill list is self-contained and deterministic. Same input should produce
the same local result. Optional OCR is available for scanned PDFs when the app
is built with OCR support and local OCR tools are installed.

Current import support covers PDF, DOCX, TXT, Markdown, and HTML. Private reference
examples reviewed during planning also showed RTF, ODT, EPUB, and archive
exports. Future importer work should use synthetic fixtures based on those
format patterns instead of committing private resume text.

The detailed research note for future resume assistance work is
[Resume Formatting And Application Readability, 2026](../research/resume-formatting-ats-2026.md).
That guidance locks in single-column readable structure, plain-text preview,
truthful keyword evidence, application-form consistency, ethical confidence,
profession-specific prompts, and score humility. Drafted bullet reminders now
include healthcare, regulated work, service and operations, technical and data,
sales and marketing, design and creative, education and academic, executive and
leadership, security, federal, trades and field-service, career-change, and
early-career evidence prompts.
The companion
[Resume Alignment Scoring](../research/resume-alignment-scoring.md) note locks
in transparent component rubrics, match states, evidence strength, hard
constraint caps, conservative synonym handling, recency and section placement,
and profession-specific weighting. Local requirement-state rows and recognized
hard-constraint caps have started. Section-placement review, seniority-level
constraint review, conservative `CRM` / `customer relationship management`, and
common credential equivalence have started. Copied structured resume details
can show current-role evidence labels, and readable saved-resume text can mark
bullets after a present-date role marker as current role evidence. Current-role
evidence now carries a small strength boost over the same single older-role
hit. Broader non-metric evidence strength beyond scope and responsibility
signals, synonym, broader recency weighting, deeper seniority alignment, and
profession-specific weighting remain future work.
Visible matched-word and requirement review surfaces, including live Resume
Builder tooltips and Resume Match rows, use plain evidence labels such as
current role experience, recent role experience, work experience, and skills
list instead of backend section names.
Requirement rows and live Resume Builder details can also show the matching
resume locations as keyboard-visible human labels. They never show citation
identifiers, source revisions, raw field paths, or excerpts. Clearance labels
do not verify current status, and military-service labels do not claim civilian
equivalence.

The Milestone 5 application contract prepares military-to-civilian wording as a
move-only, in-memory review draft. The occupation code, every described military
responsibility or credential, and any current-clearance label must appear in the
current bounded saved-resume text and have same-case, user-confirmed military
evidence. Current-clearance evidence uses its own citation. The draft preserves
each exact military source phrase beside the proposed civilian wording for
explicit review. Saved-match existence, job description, match ID, and skills
are checked only when preparing that review; they do not author its wording.
It returns only the reviewed civilian wording after a typed user-action receipt
names that exact draft and a fresh atomic read confirms the same resume
revision, content hash, and evidence links. A job or saved-match context change
cannot revise the prepared civilian wording; skills context cannot revise it
either, and any resulting resume revision or a changed evidence link fails
confirmation.

The dated O*NET occupation crosswalk and DoD COOL credential site remain manual
review resources. They do not author or verify the branch, occupation, civilian
role, responsibilities, credentials, clearance, veteran status, eligibility, or
equivalence. Missing, changed, deleted, ambiguous, unconfirmed, or near-match
source evidence fails closed. No review draft or suggestion is persisted,
logged, sent, or generated from a packaged crosswalk. The native move-only
draft and suggestion remain non-Serialize. IPC may serialize only an opaque
pending-review token and, after confirmation, a safe civilian projection:
role, responsibilities, credential wording, a user-confirmed clearance label,
and fixed `suggestion_only` / `not_verified` boundaries for clearance
currentness and military-to-civilian equivalence. It never serializes source-
evidence fields, raw resume text, citations, IDs, revisions, paths, or
resources.

Each recent saved match also offers **Inspect evidence**. The debugger shows
bounded requirement states, hard-constraint blockers, plain **Why not**
reasons, and opaque current citations. Users must confirm exact current
evidence before saving a reviewed packet claim. Packet claims persist only the
reviewed text, ordered opaque evidence links, and fixed uncertainty boundaries;
stale resume, skill, or job evidence refuses the write or reload.

Resume Match offers an optional local matching profile only after the user
selects both a role evidence focus and job-market wording. The role focus marks
preferred evidence sections and breaks ties only after requirement importance
and match state. It does not upgrade evidence, change a requirement state, or
change a score. The regional profile recognizes a small reviewed set of
`program` / `programme` and `license` / `licence` spellings, so it can change
recognized matches and scores. It does not treat different concepts as
equivalent, and a missing required licence remains a hard constraint.

United States, United Kingdom, European Union, and India are explicit starter
choices, not inferred locations. They do not claim complete terminology,
employer, occupation, or regional coverage. Unselected analysis keeps the
existing result contract without profile fields. Selected profiles reach both
active saved-resume and copied structured-resume analysis, stay in local
session state, and never trigger network access. The controls remain locked
while a review is running so an older profile result cannot appear under a
newer selection.

The versioned synthetic `matching-profiles-v1` evaluation fixture freezes this
limited behavior. All nine role profiles must produce an exact review order
that differs from unprofiled analysis while preserving requirement states and
zero keyword and overall score deltas. The bounded United Kingdom, European
Union, and India spelling cases produce exact positive match-score deltas; the
United States control remains unchanged. Separate cases preserve the
`project evaluation` / `program evaluation` distinction and the required
licence hard-constraint cap. These are regression values, not claims that
Qwen3, regional terminology, employers, occupations, sources, or a region pack
are complete or ready to ship.

Resume Match also shows a compact Role Coverage card backed by
`src/features/resumes/shared/resumeRoleFamilyTaxonomy.ts`. The shared contract covers technical,
content, operations, healthcare, service, trades, education, sales, and
early-career resume help, and tests verify each family maps to existing career
profiles plus analyzer evidence prompts.

## Boundaries

- Resume data stays local by default.
- External AI is not required.
- Any future external AI resume review must go through the AI gateway, require
  explicit opt-in, show the exact request contents, support redaction or
  cancellation, and log high-level request details locally.
- Resume matching should use public job-posting text plus the selected resume,
  not unrelated notes, salary floors, or application history.
- Research and evaluation should use synthetic resumes unless a real user gives
  explicit informed consent.
