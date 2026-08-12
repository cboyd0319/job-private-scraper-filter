import { useState } from "react";
import {
  RESUME_ROLE_FAMILY_TAXONOMY,
  type ResumeRoleFamilyId,
} from "../shared/resumeRoleFamilyTaxonomy";
import type {
  ProfessionMatchingProfile,
  RegionalMatchingProfile,
  ResumeMatchingProfile,
} from "../shared/atsAnalysisContracts";

interface ResumeMatchingProfileFieldsProps {
  disabled: boolean;
  initialProfile: ResumeMatchingProfile | null;
  onProfileChange: (profile: ResumeMatchingProfile | null) => void;
}

const REGION_OPTIONS: readonly {
  value: RegionalMatchingProfile;
  label: string;
}[] = [
  { value: "us", label: "United States" },
  { value: "uk", label: "United Kingdom" },
  { value: "eu", label: "European Union" },
  { value: "india", label: "India" },
];

function toProfession(
  roleFamily: ResumeRoleFamilyId,
): ProfessionMatchingProfile {
  return roleFamily === "early-career" ? "early_career" : roleFamily;
}

function toRoleFamily(
  profession: ProfessionMatchingProfile,
): ResumeRoleFamilyId {
  return profession === "early_career" ? "early-career" : profession;
}

export function ResumeMatchingProfileFields({
  disabled,
  initialProfile,
  onProfileChange,
}: ResumeMatchingProfileFieldsProps) {
  const [profession, setProfession] = useState<ProfessionMatchingProfile | "">(
    initialProfile?.profession ?? "",
  );
  const [region, setRegion] = useState<RegionalMatchingProfile | "">(
    initialProfile?.region ?? "",
  );

  const updateProfile = (
    nextProfession: ProfessionMatchingProfile | "",
    nextRegion: RegionalMatchingProfile | "",
  ) => {
    onProfileChange(
      nextProfession && nextRegion
        ? { profession: nextProfession, region: nextRegion }
        : null,
    );
  };

  return (
    <fieldset className="space-y-3 disabled:opacity-70" disabled={disabled}>
      <legend className="text-sm font-semibold text-surface-900 dark:text-surface-100">
        Optional local matching profile
      </legend>
      <p className="text-sm text-surface-600 dark:text-surface-300">
        Choose both. Role evidence focus changes only same-priority review
        order. Reviewed regional spellings can change recognized matches and
        scores.
      </p>
      <div className="grid gap-3 sm:grid-cols-2">
        <label className="text-sm text-surface-700 dark:text-surface-200">
          <span className="mb-1 block font-medium">Role evidence focus</span>
          <select
            value={profession ? toRoleFamily(profession) : ""}
            onChange={(event) => {
              const value = event.target.value as ResumeRoleFamilyId | "";
              const nextProfession = value ? toProfession(value) : "";
              setProfession(nextProfession);
              updateProfile(nextProfession, region);
            }}
            className="w-full rounded-lg border border-surface-200 bg-white px-3 py-2 text-surface-900 dark:border-surface-600 dark:bg-surface-700 dark:text-surface-100"
          >
            <option value="">No role profile</option>
            {RESUME_ROLE_FAMILY_TAXONOMY.map((family) => (
              <option key={family.id} value={family.id}>
                {family.label}
              </option>
            ))}
          </select>
        </label>
        <label className="text-sm text-surface-700 dark:text-surface-200">
          <span className="mb-1 block font-medium">Job market wording</span>
          <select
            value={region}
            onChange={(event) => {
              const nextRegion = event.target.value as
                | RegionalMatchingProfile
                | "";
              setRegion(nextRegion);
              updateProfile(profession, nextRegion);
            }}
            className="w-full rounded-lg border border-surface-200 bg-white px-3 py-2 text-surface-900 dark:border-surface-600 dark:bg-surface-700 dark:text-surface-100"
          >
            <option value="">No regional profile</option>
            {REGION_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
      </div>
      <p className="text-xs text-surface-500 dark:text-surface-400">
        This local starter aid does not claim complete regional terminology or
        employer coverage.
      </p>
    </fieldset>
  );
}
