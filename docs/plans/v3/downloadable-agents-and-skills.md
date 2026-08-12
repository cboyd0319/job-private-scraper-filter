<!-- Defines JobSentinel's signed downloadable agent, skill, pack, and evaluation product boundary. -->

# Downloadable Agents And Skills

V2.9 ships downloadable Agent Skills. V3 should turn that into a broader
downloadable ecosystem: skills, agents, workflow packs, source packs, role
packs, rubrics, eval packs, and safe local tools that extend JobSentinel without
weakening privacy or turning the app into an unreviewed plugin host.

## Product Direction

V3 should support:

- downloadable skills
- downloadable agents
- workflow packs
- role and industry packs
- regional source packs
- resume and interview rubric packs
- negotiation packs
- source adapter packs
- eval and benchmark packs
- local template packs
- safe support and diagnostics packs
- regional starter packs for UK, EU, India, and future markets
- OS integration helper packs where platform support is explicit

The user-facing idea is simple:

```text
Add job-search help for this role, region, source, or workflow.
```

The implementation must stay structured, signed, inspectable, and revocable.

## Package Types

| Pack type | Purpose |
| --- | --- |
| Skill pack | Static Agent Skills, prompts, checklists, references, and templates that follow the skills spec. |
| Agent pack | A declared local workflow agent with inputs, outputs, privacy labels, approval gates, and allowed actions. |
| Workflow pack | Multi-step guided flows such as weekly review, apply packet, interview prep, or offer review. |
| Role pack | Role-specific titles, skills, rubrics, examples, source suggestions, interview questions, and resume guidance. |
| Regional pack | Country, state, city, currency, visa, pay, public-sector, and regional source guidance. |
| Source pack | Source manifest, policy note, rate limit, selectors or JSON paths, fixtures, and parser contracts. |
| Rubric pack | Resume, cover letter, outreach, interview story, negotiation, or application quality rubric. |
| Eval pack | Synthetic fixtures, hard negatives, counterfactuals, expected rankings, and parser fixtures. |
| Template pack | Resumes, cover notes, outreach messages, follow-up notes, tracker views, and export formats. |
| OS helper pack | Platform-specific helper metadata for shortcuts, file types, safe metadata, and permission copy. |

## Downloadable Agents

Agents should be workflow definitions, not arbitrary code.

An agent pack should declare:

- display name
- description
- supported JobSentinel version
- inputs
- outputs
- privacy labels
- allowed data categories
- allowed actions
- external AI allowance
- local model requirements
- approval gates
- failure states
- uninstall behavior
- test fixtures

Candidate v3 agents:

- Search Planner
- Source Analyst
- Posting Risk Reviewer
- Resume Evidence Reviewer
- Application Packet Builder
- Browser Companion Assistant
- Networking Coach
- Interview Coach
- Offer Negotiation Analyst
- Privacy Reviewer
- Migration Assistant
- Essentials Setup Assistant

Agents can prepare, explain, and queue work. They must not submit final
applications, bypass source controls, read hidden restricted-source state, or
send sensitive data outside the device without a reviewed gateway path.

## Expanded Skills

V3 skills should go beyond static worksheets:

- deeper references and rubrics
- role-specific examples
- handoff metadata between skills
- validated inputs and outputs
- `agents/openai.yaml` and future distribution metadata where allowed
- local-only and external-AI variants
- compact instructions plus deeper references
- eval examples for skill quality
- screenshots or assets where helpful

Expanded skill areas:

- source discovery
- search strategy
- resume evidence mapping
- ATS readability
- application packet review
- screening answer provenance
- networking outreach
- recruiter communication
- interview story quality
- work-sample prep
- salary and offer review
- career transition planning
- public-sector applications
- trades and apprenticeship search
- healthcare credentialed roles
- education and school-district search
- shared-computer workflow

## Marketplace Or Registry

V3 can support a local-first registry without requiring hosted accounts.

Explore:

- curated built-in packs
- user-installed local pack files
- optional remote index
- content-addressed package identity
- offline pack bundles for shared-computer or limited-internet environments
- signed manifests
- checksums
- publisher identity
- compatibility line
- privacy labels
- permission summary
- source policy summary
- install review
- update review
- rollback
- uninstall
- pack health diagnostics

The registry should not require telemetry or user accounts.

## Pack Quarantine And Self-Test

New or updated packs should enter quarantine until local checks pass:

- manifest schema validation
- signature and checksum validation
- compatibility-range validation
- privacy-label validation
- permission validation
- fixture tests for source, parser, eval, and workflow packs
- size budget checks
- banned-file and executable-code checks
- source policy and region metadata checks when relevant

The Milestone 5 contract verifies the manifest SHA-256 against the exact payload
bytes before a consumer can accept them. Matching bytes establish integrity
only, not publisher identity, trust, safe content, or permission to execute.

Users should see plain states:

- "Ready"
- "Needs review"
- "Quarantined"
- "Update available"
- "Failed self-test"
- "Removed"

## Trust And Signing

Every downloadable package needs:

- manifest version
- package id
- package type
- publisher
- version
- compatibility range
- checksum
- signature
- license
- privacy labels
- permissions
- data categories
- external destinations
- test fixture summary

V3 should reject or quarantine packs that:

- are unsigned when policy requires signing
- request unknown permissions
- declare broad filesystem or shell access
- include unreviewed executable code
- lack source policy metadata
- lack fixtures for parser behavior
- conflict with Rule 0

## Runtime Permissions

Pack permissions should be narrow:

- read selected case file
- read selected resume evidence
- read public job posting fields
- create draft local note
- create draft application packet
- create reminder
- open browser link
- request source check
- request external AI through gateway
- write local event

Avoid:

- raw database access
- vault access
- unrestricted filesystem access
- unrestricted shell access
- hidden browser access
- network access outside declared source or provider paths
- final application submit

## Pack UI

Users should see:

- what the pack helps with
- who published it
- what data it can read
- what actions it can take
- whether it uses external AI
- whether it needs a model download
- whether it adds sources
- how to disable or remove it
- when it was last verified

Use plain-language labels:

- "Can read selected job only"
- "Can create draft notes"
- "Can ask before sending to external AI"
- "Cannot submit applications"
- "Can be removed anytime"

## Developer And Community Workflow

V3 should make pack contribution possible:

- schema docs
- templates
- local validator
- fixture runner
- security checklist
- privacy-label checklist
- signing guide
- example packs
- CI checks
- package-size caps
- compatibility tests
- local pack playground that runs fixtures against synthetic data only
- regional pack templates for sources, taxonomy bridges, pay formats, and CV
  profiles

Keep the barrier low for useful non-code packs while keeping executable
behavior heavily constrained.

## Evaluation

Downloadable agents and skills need quality checks:

- Does the pack solve a real user task?
- Does it use plain language?
- Does it preserve Rule 0?
- Are inputs and outputs clear?
- Are examples realistic?
- Are handoffs useful?
- Are fixtures included where needed?
- Can it run in Essentials?
- Does it degrade gracefully without external AI?
- Can the user uninstall it cleanly?

## Release Bar

V3 should not ship downloadable agents until:

- pack manifest schema is stable
- permissions are enforced in Rust
- install review UI exists
- uninstall and rollback work
- signatures and checksums are verified
- privacy labels are validated
- agent actions are approval-gated
- external AI calls route through the gateway
- source packs require source policy metadata
- eval packs can run locally
- pack failures produce safe user-facing errors
