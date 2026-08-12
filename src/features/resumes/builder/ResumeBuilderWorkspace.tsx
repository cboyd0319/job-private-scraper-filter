import type { Dispatch, SetStateAction } from "react";
import { AtsLiveScorePanel } from "./AtsLiveScorePanel";
import { Card } from "../../../ui/Card";
import ContactStep from "./steps/ContactStep";
import EducationStep from "./steps/EducationStep";
import ExperienceStep from "./steps/ExperienceStep";
import SkillsStep from "./steps/SkillsStep";
import SummaryStep from "./steps/SummaryStep";
import { ResumeBuilderExportStep } from "./ResumeBuilderExportStep";
import { ResumeBuilderJobContextCard } from "./ResumeBuilderJobContextCard";
import { ResumeBuilderNavigation } from "./ResumeBuilderNavigation";
import { ResumeBuilderPreviewStep } from "./ResumeBuilderPreviewStep";
import type {
  ATSAnalysis,
  ContactInfo,
  Education,
  Experience,
  ResumeData,
  SkillEntry,
  Template,
  TemplateId,
} from "./resumeBuilderData";

interface ResumeBuilderWorkspaceProps {
  atsAnalysis: ATSAnalysis | null;
  contact: ContactInfo;
  currentStep: number;
  educations: Education[];
  experiences: Experience[];
  exporting: boolean;
  hasJobContext: boolean;
  importingSkills: boolean;
  newSkill: SkillEntry;
  previewHtml: string;
  resumeData: ResumeData | null;
  saving: boolean;
  selectedTemplate: TemplateId;
  showContactValidation: boolean;
  skills: SkillEntry[];
  summary: string;
  templates: Template[];
  onAddEducationClick: () => void;
  onAddExperienceClick: () => void;
  onAddSkill: () => void;
  onDeleteEducationClick: (education: Education) => void;
  onDeleteExperienceClick: (experience: Experience) => void;
  onDeleteSkill: (index: number, skillName: string) => void;
  onExport: () => void;
  onExportDocx: () => void;
  onExportJson: () => void;
  onExportPdf: () => void;
  onImportSkills: () => void;
  onNext: () => void;
  onPrevious: () => void;
  onSelectTemplate: (templateId: TemplateId) => void;
  setContact: Dispatch<SetStateAction<ContactInfo>>;
  setNewSkill: Dispatch<SetStateAction<SkillEntry>>;
  setSummary: Dispatch<SetStateAction<string>>;
}

export function ResumeBuilderWorkspace({
  atsAnalysis,
  contact,
  currentStep,
  educations,
  experiences,
  exporting,
  hasJobContext,
  importingSkills,
  newSkill,
  previewHtml,
  resumeData,
  saving,
  selectedTemplate,
  showContactValidation,
  skills,
  summary,
  templates,
  onAddEducationClick,
  onAddExperienceClick,
  onAddSkill,
  onDeleteEducationClick,
  onDeleteExperienceClick,
  onDeleteSkill,
  onExport,
  onExportDocx,
  onExportJson,
  onExportPdf,
  onImportSkills,
  onNext,
  onPrevious,
  onSelectTemplate,
  setContact,
  setNewSkill,
  setSummary,
}: ResumeBuilderWorkspaceProps) {
  return (
    <main className="max-w-7xl mx-auto px-6 py-8">
      <div className="grid grid-cols-1 lg:grid-cols-4 gap-6">
        <div className="lg:col-span-3">
          <Card>
            {currentStep === 1 && (
              <ContactStep
                contact={contact}
                setContact={setContact}
                showRequiredError={showContactValidation}
              />
            )}

            {currentStep === 2 && (
              <SummaryStep summary={summary} setSummary={setSummary} />
            )}

            {currentStep === 3 && (
              <ExperienceStep
                experiences={experiences}
                onAddClick={onAddExperienceClick}
                onDeleteClick={onDeleteExperienceClick}
              />
            )}

            {currentStep === 4 && (
              <EducationStep
                educations={educations}
                onAddClick={onAddEducationClick}
                onDeleteClick={onDeleteEducationClick}
              />
            )}

            {currentStep === 5 && (
              <SkillsStep
                skills={skills}
                newSkill={newSkill}
                setNewSkill={setNewSkill}
                onAddSkill={onAddSkill}
                onDeleteSkill={onDeleteSkill}
                onImportSkills={onImportSkills}
                importingSkills={importingSkills}
              />
            )}

            {currentStep === 6 && (
              <ResumeBuilderPreviewStep
                templates={templates}
                selectedTemplate={selectedTemplate}
                previewHtml={previewHtml}
                atsAnalysis={atsAnalysis}
                onSelectTemplate={onSelectTemplate}
              />
            )}

            {currentStep === 7 && (
              <ResumeBuilderExportStep
                exporting={exporting}
                onExportDocx={onExportDocx}
                onExportJson={onExportJson}
                onExportPdf={onExportPdf}
              />
            )}

            <ResumeBuilderNavigation
              currentStep={currentStep}
              exporting={exporting}
              saving={saving}
              onExport={onExport}
              onNext={onNext}
              onPrevious={onPrevious}
            />
          </Card>
        </div>

        <div className="lg:col-span-1 space-y-4">
          <AtsLiveScorePanel
            resumeData={
              contact.name && resumeData
                ? {
                    ...resumeData,
                    contact,
                    summary,
                    experience: experiences,
                    education: educations,
                    skills,
                  }
                : null
            }
            currentStep={currentStep}
            debounceMs={1500}
            showFullAnalysis={true}
          />

          <ResumeBuilderJobContextCard visible={hasJobContext} />
        </div>
      </div>
    </main>
  );
}
