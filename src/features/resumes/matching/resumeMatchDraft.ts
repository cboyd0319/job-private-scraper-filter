import {
  readStorageValue,
  removeStorageValue,
  writeStorageValue,
} from "../../../shared/browserStorage";
import {
  isResumeMatchingProfile,
  type ResumeMatchingProfile,
} from "../shared/atsAnalysisContracts";
import type { AtsAnalysisResult } from "./resumeMatchModel";

export interface ResumeMatchDraft {
  jobDescription: string;
  resumeJson: string;
  analysisResult: AtsAnalysisResult | null;
  analysisInputSource: "active" | "copied" | null;
  matchingProfile: ResumeMatchingProfile | null;
  showAdvancedResumeImport: boolean;
  showComparison: boolean;
}

const STORAGE_KEY = "jobsentinel-resume-match-draft-v1";

export function readResumeMatchDraft(): ResumeMatchDraft | null {
  const raw = readStorageValue("session", STORAGE_KEY);
  if (!raw) return null;

  try {
    const parsed = JSON.parse(raw) as Partial<ResumeMatchDraft>;
    if (typeof parsed.jobDescription !== "string") return null;
    if (typeof parsed.resumeJson !== "string") return null;
    removeStorageValue("session", STORAGE_KEY);
    return {
      jobDescription: parsed.jobDescription,
      resumeJson: parsed.resumeJson,
      analysisResult: parsed.analysisResult ?? null,
      analysisInputSource:
        parsed.analysisInputSource === "active" ||
          parsed.analysisInputSource === "copied"
          ? parsed.analysisInputSource
          : null,
      matchingProfile: isResumeMatchingProfile(parsed.matchingProfile)
        ? parsed.matchingProfile
        : null,
      showAdvancedResumeImport: Boolean(parsed.showAdvancedResumeImport),
      showComparison: Boolean(parsed.showComparison),
    };
  } catch {
    removeStorageValue("session", STORAGE_KEY);
    return null;
  }
}

export function writeResumeMatchDraft(draft: ResumeMatchDraft): void {
  writeStorageValue("session", STORAGE_KEY, JSON.stringify(draft));
}
