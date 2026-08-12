import { formatEvidenceSections } from "./atsAnalysisLabels";
import type { RequirementReview } from "./atsAnalysisContracts";

interface ResumeRequirementEvidenceProps {
  review: RequirementReview;
}

function itemNumber(value: string | undefined): number | null {
  if (!value || !/^\d+$/.test(value)) return null;
  const index = Number(value);
  return Number.isSafeInteger(index) && index < Number.MAX_SAFE_INTEGER
    ? index + 1
    : null;
}

function evidenceLocation(fieldPath: string): string {
  if (fieldPath === "summary") return "Professional summary";
  if (fieldPath === "clearance") {
    return "Clearance details, current status not verified";
  }
  if (fieldPath === "military_info") {
    return "Military service details, civilian equivalence not verified";
  }

  const [section, index, field, childIndex, childField] = fieldPath.split(".");
  const item = itemNumber(index);
  if (section === "resume_text" && item) return `Resume text, line ${item}`;
  if (!item) return "Saved resume field";

  if (section === "experience") {
    if (field === "achievements") {
      const bullet = itemNumber(childIndex);
      return bullet
        ? `Work experience ${item}, bullet ${bullet}`
        : `Work experience ${item}`;
    }
    if (field === "title") return `Work experience ${item}, job title`;
    if (field === "company") return `Work experience ${item}, employer`;
    return `Work experience ${item}`;
  }

  if (section === "skills") {
    const skill = field === "skills" ? itemNumber(childIndex) : null;
    return skill && childField === "name"
      ? `Skills group ${item}, item ${skill}`
      : `Skills list, item ${item}`;
  }

  if (section === "education") {
    const honor = field === "honors" ? itemNumber(childIndex) : null;
    if (honor) return `Education ${item}, honor ${honor}`;
    if (field === "degree") return `Education ${item}, degree`;
    if (field === "institution") return `Education ${item}, school`;
    if (field === "location") return `Education ${item}, location`;
    return `Education ${item}`;
  }

  if (section === "certifications") {
    if (field === "name") return `Certification ${item}, name`;
    if (field === "issuer") return `Certification ${item}, issuer`;
    return `Certification ${item}`;
  }

  if (section === "projects") {
    const technology = field === "technologies" ? itemNumber(childIndex) : null;
    if (technology) return `Project ${item}, technology ${technology}`;
    if (field === "name") return `Project ${item}, name`;
    if (field === "description") return `Project ${item}, description`;
    if (field === "url") return `Project ${item}, link`;
    return `Project ${item}`;
  }

  return "Saved resume field";
}

export function ResumeRequirementEvidence({
  review,
}: ResumeRequirementEvidenceProps) {
  const locations = [
    ...new Set(
      (review.evidence_citations ?? []).map((citation) =>
        evidenceLocation(citation.field_path),
      ),
    ),
  ];

  return (
    <div className="mt-2 space-y-2 text-xs text-surface-500 dark:text-surface-400">
      <p>
        {review.evidence_sections.length > 0
          ? `Found in: ${formatEvidenceSections(review.evidence_sections)}`
          : "No clear resume evidence found"}
      </p>
      {locations.length > 0 && (
        <>
          <ul
            aria-label={`Evidence locations for ${review.keyword}`}
            className="list-disc space-y-1 pl-5"
          >
            {locations.map((location) => (
              <li key={location}>{location}</li>
            ))}
          </ul>
          <p>These locations show matching wording, not verification of a claim.</p>
        </>
      )}
    </div>
  );
}
