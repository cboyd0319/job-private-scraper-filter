import type { ExternalAiDataCategory, ExternalAiRequest } from "./externalAiTypes";

const sensitiveDataCategories = new Set<ExternalAiDataCategory>([
  "resume",
  "salary_floor",
  "private_notes",
  "application_history",
  "career_goals",
  "location_preferences",
]);

const sensitivePayloadKeys = new Set([
  "applicationHistory",
  "application_history",
  "careerGoals",
  "career_goals",
  "database",
  "fullDatabase",
  "full_database",
  "localDatabase",
  "notes",
  "privateNotes",
  "private_notes",
  "resume",
  "resumeText",
  "resume_text",
  "salaryFloor",
  "salary_floor",
]);

const classifiedPayloadKeysByCategory: Record<ExternalAiDataCategory, Set<string>> = {
  job_posting: new Set([
    "company",
    "description",
    "location",
    "title",
  ]),
  public_metadata: new Set(),
  resume: new Set(["resume", "resumeText", "resume_text"]),
  salary_floor: new Set(["salaryFloor", "salary_floor"]),
  private_notes: new Set(["notes", "privateNotes", "private_notes"]),
  application_history: new Set(["applicationHistory", "application_history"]),
  career_goals: new Set(["careerGoals", "career_goals"]),
  location_preferences: new Set(["locationPreferences", "location_preferences"]),
  full_database: new Set([
    "database",
    "fullDatabase",
    "full_database",
    "localDatabase",
  ]),
};

function collectClassifiedPayloadKeys(): Set<string> {
  const keys = new Set<string>();

  for (const category of Object.keys(
    classifiedPayloadKeysByCategory,
  ) as ExternalAiDataCategory[]) {
    for (const key of classifiedPayloadKeysByCategory[category]) {
      keys.add(key);
    }
  }

  return keys;
}

function collectPayloadKeys(value: unknown, keys = new Set<string>()): Set<string> {
  if (!value || typeof value !== "object") {
    return keys;
  }

  if (Array.isArray(value)) {
    for (const item of value) {
      collectPayloadKeys(item, keys);
    }
    return keys;
  }

  for (const [key, nestedValue] of Object.entries(value)) {
    keys.add(key);
    collectPayloadKeys(nestedValue, keys);
  }

  return keys;
}

export function hasSensitivePayloadKeys(
  payload: Record<string, unknown>,
): boolean {
  return [...collectPayloadKeys(payload)].some((key) =>
    sensitivePayloadKeys.has(key),
  );
}

export function findUnclassifiedPayloadKey(
  payload: Record<string, unknown>,
): string | undefined {
  const classifiedKeys = collectClassifiedPayloadKeys();

  return [...collectPayloadKeys(payload)].find((key) => !classifiedKeys.has(key));
}

export function hasSensitiveData(
  request: ExternalAiRequest,
  outgoingPayload: Record<string, unknown>,
): boolean {
  return (
    request.labels.includes("Sensitive") ||
    request.dataCategories.some((category) => sensitiveDataCategories.has(category)) ||
    hasSensitivePayloadKeys(outgoingPayload)
  );
}
