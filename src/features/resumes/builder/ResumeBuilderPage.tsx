import { useState, useCallback } from "react";
import { useToast } from "../../../shared/toast/useToast";
import { safeInvoke, safeInvokeWithToast } from "../../../platform/tauri";
import { getSafeErrorToastCopy } from "../../../shared/errorReporting/safeToastCopy";
import { hasStoredResumeJobContext } from "../shared/resumeJobContext";
import { mapSkillProficiencyLevel } from "../shared/resumeSkillUiTaxonomy";
import {
  canProceedResumeBuilderStep,
  getResumeBuilderStepValidationMessage,
} from "./resumeBuilderValidation";
import { ResumeBuilderDeleteModal, type ResumeBuilderDeleteTarget } from "./ResumeBuilderDeleteModal";
import { ResumeBuilderHeader } from "./ResumeBuilderHeader";
import {
  ResumeBuilderInitializationError,
  ResumeBuilderLoadingState,
} from "./ResumeBuilderStartupStates";
import { useResumeBuilderInitialization } from "./useResumeBuilderInitialization";
import { useResumeBuilderExports } from "./useResumeBuilderExports";
import { ResumeBuilderProgress } from "./ResumeBuilderProgress";
import { ResumeBuilderEducationModal } from "./ResumeBuilderEducationModal";
import { ResumeBuilderExperienceModal } from "./ResumeBuilderExperienceModal";
import { ResumeBuilderWorkspace } from "./ResumeBuilderWorkspace";
import {
  STEPS,
  type ContactInfo,
  type Education,
  type Experience,
  type Resume,
  type ResumeData,
  type SkillEntry,
  type TemplateId,
  type UserSkill,
} from "./resumeBuilderData";

interface ResumeBuilderPageProps {
  onBack: () => void;
}
export default function ResumeBuilderPage({ onBack }: ResumeBuilderPageProps) {
  const [currentStep, setCurrentStep] = useState(1);
  const [resumeData, setResumeData] = useState<ResumeData | null>(null);
  const [saving, setSaving] = useState(false);
  const [showContactValidation, setShowContactValidation] = useState(false);
  const [selectedTemplate, setSelectedTemplate] = useState<TemplateId>("Modern");
  const [importingSkills, setImportingSkills] = useState(false);
  const [hasJobContext] = useState(() => hasStoredResumeJobContext());

  const [contact, setContact] = useState<ContactInfo>({
    name: "",
    email: "",
    phone: null,
    linkedin: null,
    github: null,
    location: null,
    website: null,
  });

  const [summary, setSummary] = useState("");

  const [experiences, setExperiences] = useState<Experience[]>([]);
  const [editingExperience, setEditingExperience] = useState<Experience | null>(null);
  const [showExperienceModal, setShowExperienceModal] = useState(false);

  const [educations, setEducations] = useState<Education[]>([]);
  const [editingEducation, setEditingEducation] = useState<Education | null>(null);
  const [showEducationModal, setShowEducationModal] = useState(false);

  const [skills, setSkills] = useState<SkillEntry[]>([]);
  const [deleteConfirm, setDeleteConfirm] = useState<ResumeBuilderDeleteTarget | null>(null);
  const [newSkill, setNewSkill] = useState<SkillEntry>({
    name: "",
    category: "",
    proficiency: null,
  });

  const toast = useToast();

  const {
    initializationError,
    initializeResume,
    loading,
    resumeId,
    setLoading,
    templates,
  } = useResumeBuilderInitialization();

  // Load resume data
  const loadResumeData = useCallback(async () => {
    if (!resumeId) return;

    try {
      const data = await safeInvoke<ResumeData | null>("get_resume_draft", { resumeId }, {
        logContext: "Load resume draft"
      });
      if (data) {
        setResumeData(data);
        setContact(data.contact);
        setSummary(data.summary);
        setExperiences(data.experience);
        setEducations(data.education);
        setSkills(data.skills);
      }
    } catch (error: unknown) {
      const safeError = getSafeErrorToastCopy(error, {
        fallbackTitle: "Couldn't load your resume",
        fallbackMessage:
          "Resume details did not load. Copy a safe support report if this keeps happening, then create a new resume only if you need to keep working.",
      });
      toast.error(safeError.title, safeError.message);
    }
  }, [resumeId, toast]);

  // Auto-save on step change
  const saveCurrentStep = useCallback(async () => {
    if (!resumeId) return;

    try {
      setSaving(true);

      switch (currentStep) {
        case 1:
          await safeInvoke("update_resume_contact", { resumeId, contact }, {
            logContext: "Update resume contact"
          });
          break;
        case 2:
          await safeInvoke("update_resume_summary", { resumeId, summary }, {
            logContext: "Update resume summary"
          });
          break;
        case 5:
          await safeInvoke("set_resume_skills", { resumeId, skills }, {
            logContext: "Set resume skills"
          });
          break;
      }

      await loadResumeData();
    } catch (error: unknown) {
      const safeError = getSafeErrorToastCopy(error, {
        fallbackTitle: "Your changes weren't saved",
        fallbackMessage:
          "The resume section couldn't be saved. Copy your work to a safe place and try again.",
      });
      toast.error(safeError.title, safeError.message);
      throw error;
    } finally {
      setSaving(false);
    }
  }, [resumeId, currentStep, contact, summary, skills, loadResumeData, toast]);

  // Navigation handlers
  const handleNext = async () => {
    if (!canProceed()) {
      if (currentStep === 1) setShowContactValidation(true);
      toast.warning("Add missing resume details", getValidationMessage());
      return;
    }

    try {
      await saveCurrentStep();
      if (currentStep === 1) setShowContactValidation(false);
      setCurrentStep((prev) => Math.min(prev + 1, STEPS.length));
    } catch {
      // Error already shown by saveCurrentStep
    }
  };

  const handlePrevious = () => {
    setCurrentStep((prev) => Math.max(prev - 1, 1));
  };

  // Validation using lookup pattern (better performance than switch)
  const canProceed = useCallback((): boolean => {
    return canProceedResumeBuilderStep(currentStep, {
      contact,
      summary,
      experiences,
      educations,
      skills,
    });
  }, [currentStep, contact, summary, experiences, educations, skills]);

  // Validation messages using lookup pattern
  const getValidationMessage = useCallback((): string => {
    return getResumeBuilderStepValidationMessage(currentStep, {
      contact,
      summary,
      experiences,
      educations,
      skills,
    });
  }, [currentStep, contact, summary, experiences, educations, skills]);

  // Experience handlers
  const openExperienceModal = () => {
    setEditingExperience({
      id: 0,
      title: "",
      company: "",
      location: null,
      start_date: "",
      end_date: null,
      achievements: [],
    });
    setShowExperienceModal(true);
  };

  const handleAddExperience = async () => {
    if (!resumeId || !editingExperience) return;

    try {
      const id = await safeInvokeWithToast<number>("add_resume_experience", {
        resumeId,
        experience: editingExperience,
      }, toast, {
        logContext: "Add resume experience"
      });

      setExperiences([...experiences, { ...editingExperience, id }]);
      setShowExperienceModal(false);
      setEditingExperience(null);
      toast.success("Experience added", "");
    } catch {
      // Error already logged and shown to user
    }
  };

  const handleDeleteExperience = async (experienceId: number) => {
    if (!resumeId) return;

    try {
      await safeInvokeWithToast("delete_resume_experience", { resumeId, experienceId }, toast, {
        logContext: "Delete resume experience"
      });
      setExperiences(experiences.filter((e) => e.id !== experienceId));
      toast.success("Experience removed", "");
    } catch {
      // Error already logged and shown to user
    } finally {
      setDeleteConfirm(null);
    }
  };

  const confirmDeleteExperience = (exp: Experience) => {
    setDeleteConfirm({ type: 'experience', id: exp.id, name: `${exp.title} at ${exp.company}` });
  };

  // Education handlers
  const openEducationModal = () => {
    setEditingEducation({
      id: 0,
      degree: "",
      institution: "",
      location: null,
      graduation_date: null,
      gpa: null,
      honors: [],
    });
    setShowEducationModal(true);
  };

  const handleAddEducation = async () => {
    if (!resumeId || !editingEducation) return;

    try {
      const id = await safeInvokeWithToast<number>("add_resume_education", {
        resumeId,
        education: editingEducation,
      }, toast, {
        logContext: "Add resume education"
      });

      setEducations([...educations, { ...editingEducation, id }]);
      setShowEducationModal(false);
      setEditingEducation(null);
      toast.success("Education added", "");
    } catch {
      // Error already logged and shown to user
    }
  };

  const handleDeleteEducation = async (educationId: number) => {
    if (!resumeId) return;

    try {
      await safeInvokeWithToast("delete_resume_education", { resumeId, educationId }, toast, {
        logContext: "Delete resume education"
      });
      setEducations(educations.filter((e) => e.id !== educationId));
      toast.success("Education removed", "");
    } catch {
      // Error already logged and shown to user
    } finally {
      setDeleteConfirm(null);
    }
  };

  const confirmDeleteEducation = (edu: Education) => {
    setDeleteConfirm({ type: 'education', id: edu.id, name: `${edu.degree} at ${edu.institution}` });
  };

  // Skills handlers
  const handleAddSkill = () => {
    if (!newSkill.name || !newSkill.category) {
      toast.warning("Add skill details", "Add a skill name and category, then add the skill.");
      return;
    }

    setSkills([...skills, newSkill]);
    setNewSkill({ name: "", category: "", proficiency: null });
  };

  const handleDeleteSkill = (index: number) => {
    setSkills(skills.filter((_, i) => i !== index));
    setDeleteConfirm(null);
  };

  const confirmDeleteSkill = (index: number, skillName: string) => {
    setDeleteConfirm({ type: 'skill', id: index, name: skillName });
  };

  const handleImportSkills = async () => {
    try {
      setImportingSkills(true);

      // Get active resume
      const activeResume = await safeInvoke<Resume>("get_active_resume", {}, {
        logContext: "Get active resume for skill import"
      });
      if (!activeResume) {
        toast.warning("No resume added", "Add a resume in Resume Match first");
        return;
      }

      // Get skills from resume
      const userSkills = await safeInvoke<UserSkill[]>("get_user_skills", {
        resumeId: activeResume.id,
      }, {
        logContext: "Get user skills for import"
      });

      if (userSkills.length === 0) {
        toast.info("No skills found", "Add and review a resume in Resume Match first");
        return;
      }

      // Map UserSkill to SkillEntry format
      const importedSkills: SkillEntry[] = userSkills.map((skill) => ({
        name: skill.skill_name,
        category: skill.skill_category || "General",
        proficiency: mapSkillProficiencyLevel(skill.proficiency_level),
      }));

      // Merge with existing skills (avoid duplicates)
      const existingSkillNames = new Set(skills.map((s) => s.name.toLowerCase()));
      const newSkills = importedSkills.filter(
        (skill) => !existingSkillNames.has(skill.name.toLowerCase())
      );

      if (newSkills.length === 0) {
        toast.info("All skills already added", "No new skills to import");
        return;
      }

      setSkills([...skills, ...newSkills]);
      toast.success(`Imported ${newSkills.length} skills`, "Skills added from resume");
    } catch (error: unknown) {
      const safeError = getSafeErrorToastCopy(error, {
        fallbackTitle: "Could not import skills",
      });
      toast.error(safeError.title, safeError.message);
    } finally {
      setImportingSkills(false);
    }
  };

  const {
    atsAnalysis,
    exporting,
    handleExport,
    handleExportDocx,
    handleExportJson,
    handleExportPdf,
    previewHtml,
  } = useResumeBuilderExports({
    contactName: contact.name,
    currentStep,
    resumeData,
    resumeId,
    selectedTemplate,
    setLoading,
  });

  if (loading && !resumeId) return <ResumeBuilderLoadingState />;

  if (initializationError && !resumeId) {
    return (
      <ResumeBuilderInitializationError
        loading={loading}
        onBack={onBack}
        onRetry={initializeResume}
      />
    );
  }

  return (
    <div className="min-h-screen bg-surface-50 dark:bg-surface-900">
      <ResumeBuilderHeader currentStep={currentStep} onBack={onBack} />

      <ResumeBuilderProgress currentStep={currentStep} />

      <ResumeBuilderWorkspace
        atsAnalysis={atsAnalysis}
        contact={contact}
        currentStep={currentStep}
        educations={educations}
        experiences={experiences}
        exporting={exporting}
        hasJobContext={hasJobContext}
        importingSkills={importingSkills}
        newSkill={newSkill}
        previewHtml={previewHtml}
        resumeData={resumeData}
        saving={saving}
        selectedTemplate={selectedTemplate}
        showContactValidation={showContactValidation}
        skills={skills}
        summary={summary}
        templates={templates}
        onAddEducationClick={openEducationModal}
        onAddExperienceClick={openExperienceModal}
        onAddSkill={handleAddSkill}
        onDeleteEducationClick={confirmDeleteEducation}
        onDeleteExperienceClick={confirmDeleteExperience}
        onDeleteSkill={confirmDeleteSkill}
        onExport={handleExport}
        onExportDocx={handleExportDocx}
        onExportJson={handleExportJson}
        onExportPdf={handleExportPdf}
        onImportSkills={handleImportSkills}
        onNext={handleNext}
        onPrevious={handlePrevious}
        onSelectTemplate={setSelectedTemplate}
        setContact={setContact}
        setNewSkill={setNewSkill}
        setSummary={setSummary}
      />

      <ResumeBuilderExperienceModal
        experience={editingExperience}
        isOpen={showExperienceModal}
        onAdd={handleAddExperience}
        onChange={setEditingExperience}
        onClose={() => setShowExperienceModal(false)}
      />

      <ResumeBuilderEducationModal
        education={editingEducation}
        isOpen={showEducationModal}
        onAdd={handleAddEducation}
        onChange={setEditingEducation}
        onClose={() => setShowEducationModal(false)}
      />

      <ResumeBuilderDeleteModal
        target={deleteConfirm}
        onClose={() => setDeleteConfirm(null)}
        onConfirm={(target) => {
          if (target.type === "experience") {
            void handleDeleteExperience(target.id);
          } else if (target.type === "education") {
            void handleDeleteEducation(target.id);
          } else {
            handleDeleteSkill(target.id);
          }
        }}
      />
    </div>
  );
}
