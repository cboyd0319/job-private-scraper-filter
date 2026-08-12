import { useCallback, useEffect, useState } from "react";
import { useToast } from "../../../shared/toast/useToast";
import { safeInvoke, safeInvokeWithToast } from "../../../platform/tauri";
import { getSafeErrorToastCopy } from "../../../shared/errorReporting/safeToastCopy";
import {
  DEFAULT_SKILL_STRENGTH,
  getSkillStrengthLabel,
  isResumeMatchingEnabled,
  optionalTrimmedText,
  optionalYearsValue,
  type MatchResult,
  type NewSkill,
  type ResumeData,
  type ResumeMatchingPreference,
  type ResumeMatchFeedback,
  type ResumeMatchFeedbackLabel,
  type ResumeTextPreview,
  type SkillUpdate,
  type UserSkill,
} from "./resumePageModel";

export function useResumeLibraryController() {
  const [resume, setResume] = useState<ResumeData | null>(null);
  const [allResumes, setAllResumes] = useState<ResumeData[]>([]);
  const [skills, setSkills] = useState<UserSkill[]>([]);
  const [recentMatches, setRecentMatches] = useState<MatchResult[]>([]);
  const [loading, setLoading] = useState(true);
  const [uploading, setUploading] = useState(false);
  const [editingSkillId, setEditingSkillId] = useState<number | null>(null);
  const [showAddSkill, setShowAddSkill] = useState(false);
  const [showResumeLibrary, setShowResumeLibrary] = useState(false);
  const [showTextPreview, setShowTextPreview] = useState(false);
  const [textPreview, setTextPreview] = useState<ResumeTextPreview | null>(null);
  const [textPreviewLoading, setTextPreviewLoading] = useState(false);
  const [resumeMatchingEnabled, setResumeMatchingEnabled] = useState(false);
  const [resumeMatchingLoading, setResumeMatchingLoading] = useState(false);
  const [savingFeedback, setSavingFeedback] = useState(false);
  const [categoryFilter, setCategoryFilter] = useState<string | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<{
    type: 'resume' | 'skill';
    id: number;
    name: string;
  } | null>(null);
  const toast = useToast();

  // Form state for editing/adding skills
  const [editForm, setEditForm] = useState<SkillUpdate>({});
  const [newSkillForm, setNewSkillForm] = useState<NewSkill>({
    skill_name: "",
    proficiency_level: DEFAULT_SKILL_STRENGTH,
  });

  useEffect(() => {
    let cancelled = false;

    const fetchData = async () => {
      try {
        setLoading(true);
        const [resumeData, resumesData, preferenceData] = await Promise.all([
          safeInvoke<ResumeData | null>("get_active_resume", {}, { logContext: "Load active resume" }),
          safeInvoke<ResumeData[]>("list_all_resumes", {}, { logContext: "List all resumes" }),
          safeInvoke<ResumeMatchingPreference | null>(
            "get_resume_matching_preference",
            {},
            { logContext: "Load resume sorting preference" },
          ),
        ]);

        if (cancelled) return;

        setResume(resumeData);
        setAllResumes(resumesData);
        setResumeMatchingEnabled(isResumeMatchingEnabled(preferenceData));

        if (resumeData) {
          const [skillsData, matchesData] = await Promise.all([
            safeInvoke<UserSkill[]>("get_user_skills", { resumeId: resumeData.id }, { logContext: "Load user skills" }),
            safeInvoke<MatchResult[]>("get_recent_matches", { resumeId: resumeData.id, limit: 10 }, { logContext: "Load recent matches" }),
          ]);

          if (cancelled) return;

          setSkills(skillsData);
          setRecentMatches(matchesData);
        } else {
          setSkills([]);
          setRecentMatches([]);
        }
      } catch (error: unknown) {
        if (cancelled) return;
        const safeError = getSafeErrorToastCopy(error, {
          fallbackTitle: "Could not load resume",
        });
        toast.error(safeError.title, safeError.message);
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    };

    fetchData();

    return () => {
      cancelled = true;
    };
  }, [toast]);

  // Refetch data after operations
  const refetchData = useCallback(async () => {
    try {
      setLoading(true);
      const [resumeData, resumesData, preferenceData] = await Promise.all([
        safeInvoke<ResumeData | null>("get_active_resume", {}, { logContext: "Refetch active resume" }),
        safeInvoke<ResumeData[]>("list_all_resumes", {}, { logContext: "Refetch all resumes" }),
        safeInvoke<ResumeMatchingPreference | null>(
          "get_resume_matching_preference",
          {},
          { logContext: "Refetch resume sorting preference" },
        ),
      ]);
      setResume(resumeData);
      setAllResumes(resumesData);
      setResumeMatchingEnabled(isResumeMatchingEnabled(preferenceData));

      if (resumeData) {
        const [skillsData, matchesData] = await Promise.all([
          safeInvoke<UserSkill[]>("get_user_skills", { resumeId: resumeData.id }, { logContext: "Refetch user skills" }),
          safeInvoke<MatchResult[]>("get_recent_matches", { resumeId: resumeData.id, limit: 10 }, { logContext: "Refetch recent matches" }),
        ]);
        setSkills(skillsData);
        setRecentMatches(matchesData);
      } else {
        setSkills([]);
        setRecentMatches([]);
      }
    } catch (error: unknown) {
      const safeError = getSafeErrorToastCopy(error, {
        fallbackTitle: "Could not load resume",
      });
      toast.error(safeError.title, safeError.message);
    } finally {
      setLoading(false);
    }
  }, [toast]);

  const handleUploadResume = async () => {
    try {
      setUploading(true);
      const resumeId = await safeInvokeWithToast<number | null>("select_and_upload_resume", undefined, toast, {
        logContext: "Add resume"
      });
      if (!resumeId) return;
      toast.success("Resume added", "Your resume is ready for local review.");
      refetchData();
    } catch {
      // Error already logged and shown to user
    } finally {
      setUploading(false);
    }
  };

  const handleImportJsonResume = async () => {
    try {
      setUploading(true);
      const resumeId = await safeInvokeWithToast<number | null>("select_and_import_json_resume", undefined, toast, {
        logContext: "Import structured resume data"
      });
      if (!resumeId) return;
      toast.success("Resume imported", "Your resume is ready for local review.");
      refetchData();
    } catch {
      // Error already logged and shown to user
    } finally {
      setUploading(false);
    }
  };

  const handleSetActiveResume = async (resumeId: number) => {
    try {
      await safeInvokeWithToast("set_active_resume", { resumeId }, toast, {
        logContext: "Set active resume"
      });
      toast.success("Resume activated", "Switched to selected resume");
      setShowResumeLibrary(false);
      refetchData();
    } catch {
      // Error already logged and shown to user
    }
  };

  const handleDeleteResume = async (resumeId: number) => {
    try {
      await safeInvokeWithToast("delete_resume", { resumeId }, toast, {
        logContext: "Delete resume"
      });
      toast.success("Resume deleted", "Resume and associated data removed");
      refetchData();
    } catch {
      // Error already logged and shown to user
    } finally {
      setDeleteConfirm(null);
    }
  };

  const confirmDeleteResume = (r: ResumeData) => {
    setDeleteConfirm({ type: 'resume', id: r.id, name: r.name });
  };

  const handlePreviewResumeText = async () => {
    if (!resume) {
      return;
    }

    try {
      setTextPreviewLoading(true);
      const preview = await safeInvoke<ResumeTextPreview>(
        "get_resume_text_preview",
        { resumeId: resume.id },
        { logContext: "Preview resume text" },
      );
      setTextPreview(preview);
      setShowTextPreview(true);
    } catch (error: unknown) {
      const safeError = getSafeErrorToastCopy(error, {
        fallbackTitle: "Could not show resume text",
      });
      toast.error(safeError.title, safeError.message);
    } finally {
      setTextPreviewLoading(false);
    }
  };

  const handleCopyResumeText = async () => {
    if (!textPreview?.text_preview) {
      return;
    }

    if (!navigator.clipboard?.writeText) {
      toast.error("Could not copy text", "Select the text and copy it manually.");
      return;
    }

    try {
      await navigator.clipboard.writeText(textPreview.text_preview);
      toast.success("Text copied", "Readable resume text copied to your clipboard.");
    } catch {
      toast.error("Could not copy text", "Select the text and copy it manually.");
    }
  };

  const handleSetResumeMatching = async (enabled: boolean) => {
    if (enabled && (!resume || skills.length === 0)) {
      toast.error("Review skills first", "Add or review at least one skill before using it to sort jobs.");
      return;
    }

    try {
      setResumeMatchingLoading(true);
      const preference = await safeInvoke<ResumeMatchingPreference>(
        "set_resume_matching_enabled",
        { enabled },
        { logContext: enabled ? "Use resume skills for job sorting" : "Stop using resume skills for job sorting" },
      );
      setResumeMatchingEnabled(preference.enabled);
      if (preference.enabled) {
        toast.success(
          "Resume skills will help sort jobs",
          "JobSentinel will use these reviewed local skills in job sorting.",
        );
      } else {
        toast.success(
          "Resume skills paused",
          "JobSentinel will sort jobs from your saved titles, words, salary, location, and company preferences.",
        );
      }
    } catch (error: unknown) {
      const safeError = getSafeErrorToastCopy(error, {
        fallbackTitle: "Could not update resume sorting",
      });
      toast.error(safeError.title, safeError.message);
    } finally {
      setResumeMatchingLoading(false);
    }
  };

  const handleUpdateSkill = async (skillId: number) => {
    const skillName = editForm.skill_name?.trim() ?? "";
    if (!skillName) {
      toast.error("Name the skill", "Add a skill name, then save again.");
      return;
    }

    const updates: SkillUpdate = {
      skill_name: skillName,
      skill_category: optionalTrimmedText(editForm.skill_category),
      proficiency_level: optionalTrimmedText(editForm.proficiency_level),
      years_experience: optionalYearsValue(editForm.years_experience),
    };

    try {
      await safeInvokeWithToast("update_user_skill", { skillId, updates }, toast, {
        logContext: "Update user skill"
      });
      toast.success("Skill updated", "Your skill has been updated");
      setEditingSkillId(null);
      setEditForm({});
      refetchData();
    } catch {
      // Error already logged and shown to user
    }
  };

  const handleDeleteSkill = async (skillId: number) => {
    try {
      await safeInvokeWithToast("delete_user_skill", { skillId }, toast, {
        logContext: "Delete user skill"
      });
      toast.success("Skill deleted", "Skill removed from your resume");
      refetchData();
    } catch {
      // Error already logged and shown to user
    } finally {
      setDeleteConfirm(null);
    }
  };

  const confirmDeleteSkill = (skill: UserSkill) => {
    setDeleteConfirm({ type: 'skill', id: skill.id, name: skill.skill_name });
  };

  const handleAddSkill = async () => {
    const skillName = newSkillForm.skill_name.trim();
    if (!resume || !skillName) {
      toast.error("Name the skill", "Add a skill name, then save again.");
      return;
    }

    try {
      await safeInvokeWithToast("add_user_skill", {
        resumeId: resume.id,
        skill: {
          ...newSkillForm,
          skill_name: skillName,
          skill_category: optionalTrimmedText(newSkillForm.skill_category) ?? undefined,
          proficiency_level: optionalTrimmedText(newSkillForm.proficiency_level) ?? undefined,
          years_experience: optionalYearsValue(newSkillForm.years_experience) ?? undefined,
        },
      }, toast, {
        logContext: "Add user skill"
      });
      toast.success("Skill added", `Added "${skillName}" to your skills`);
      setShowAddSkill(false);
      setNewSkillForm({ skill_name: "", proficiency_level: DEFAULT_SKILL_STRENGTH });
      refetchData();
    } catch {
      // Error already logged and shown to user
    }
  };

  const startEditingSkill = (skill: UserSkill) => {
    setEditingSkillId(skill.id);
    setEditForm({
      skill_name: skill.skill_name,
      skill_category: skill.skill_category || undefined,
      proficiency_level: skill.proficiency_level
        ? getSkillStrengthLabel(skill.proficiency_level)
        : DEFAULT_SKILL_STRENGTH,
      years_experience: skill.years_experience ?? undefined,
    });
  };

  const handleMatchFeedback = async (
    match: MatchResult,
    label: ResumeMatchFeedbackLabel,
  ) => {
    const nextLabel = match.feedback?.label === label ? null : label;
    try {
      setSavingFeedback(true);
      const feedback = await safeInvokeWithToast<ResumeMatchFeedback | null>(
        "set_resume_match_feedback",
        { matchId: match.id, label: nextLabel },
        toast,
        {
          logContext: nextLabel
            ? "Save resume match feedback"
            : "Clear resume match feedback",
        },
      );
      setRecentMatches((matches) =>
        matches.map((value) => (value.id === match.id ? { ...value, feedback } : value)),
      );
    } catch {
      // Error already logged and shown to user
    } finally {
      setSavingFeedback(false);
    }
  };

  return {
    resumeState: {
      allResumes,
      deleteConfirm,
      loading,
      recentMatches,
      resume,
      resumeMatchingEnabled,
      resumeMatchingLoading,
      savingFeedback,
      showResumeLibrary,
      uploading,
    },
    skillState: { categoryFilter, editForm, editingSkillId, newSkillForm, showAddSkill, skills },
    previewState: { showTextPreview, textPreview, textPreviewLoading },
    setters: {
      setCategoryFilter,
      setDeleteConfirm,
      setEditForm,
      setEditingSkillId,
      setNewSkillForm,
      setShowAddSkill,
      setShowResumeLibrary,
      setShowTextPreview,
    },
    skillActions: {
      confirmDeleteSkill,
      handleAddSkill,
      handleDeleteSkill,
      handleUpdateSkill,
      startEditingSkill,
    },
    resumeActions: {
      confirmDeleteResume,
      handleCopyResumeText,
      handleDeleteResume,
      handleImportJsonResume,
      handlePreviewResumeText,
      handleSetActiveResume,
      handleSetResumeMatching,
      handleUploadResume,
      handleMatchFeedback,
    },
  };
}
