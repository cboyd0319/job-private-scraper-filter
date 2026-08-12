import { Button } from "../../../ui/Button";
import { Card } from "../../../ui/Card";
import { Badge } from "../../../ui/Badge";
import { Modal, ModalFooter } from "../../../ui/Modal";
import { ResumeSkeleton } from "../../../ui/Skeleton";
import {
  getReadableTextBadgeVariant,
  getReadableTextDescription,
  getReadableTextLabel,
  getResumeFormatLabel,
  getSkillSourceLabel,
  getSkillStrengthColor,
} from "./resumePageModel";
import {
  BackIcon,
  DocumentIcon,
  FolderIcon,
  PlusIcon,
} from "./ResumeIcons";
import { useResumeLibraryController } from "./useResumeLibraryController";
import { ResumeLibraryDropdown } from "./ResumeLibraryDropdown";
import { ResumeEmptyState } from "./ResumeEmptyState";
import { ResumeRecentMatches } from "./ResumeRecentMatches";
import { ResumeTextPreviewModal } from "./ResumeTextPreviewModal";
import { ResumeSkillsManagementCard } from "./ResumeSkillsManagementCard";
import { ResumeSkillStrengthMix } from "./ResumeSkillStrengthMix";

interface ResumeLibraryPageProps {
  onBack: () => void;
}

export default function ResumeLibraryPage({ onBack }: ResumeLibraryPageProps) {
  const {
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
    skillActions: {
      confirmDeleteSkill,
      handleAddSkill,
      handleDeleteSkill,
      handleUpdateSkill,
      startEditingSkill,
    },
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
  } = useResumeLibraryController();

  if (loading) {
    return <ResumeSkeleton />;
  }

  return (
    <div className="min-h-screen bg-surface-50 dark:bg-surface-900">
      {/* Header */}
      <header className="bg-white dark:bg-surface-800 border-b border-surface-100 dark:border-surface-700 sticky top-0 z-10">
        <div className="max-w-7xl mx-auto px-6 py-4">
          <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
            <div className="flex min-w-0 items-center gap-3">
              <button
                onClick={onBack}
                className="p-2 text-surface-500 hover:text-surface-700 dark:text-surface-400 dark:hover:text-surface-200 transition-colors"
                aria-label="Go back"
              >
                <BackIcon />
              </button>
              <div className="min-w-0">
                <h1 className="font-display text-display-md text-surface-900 dark:text-white">
                  Resume Match
                </h1>
                <p className="text-sm text-surface-500 dark:text-surface-400">
                  Local resume review and job fit evidence
                </p>
              </div>
            </div>
            <div className="flex w-full flex-col items-stretch gap-2 sm:w-auto sm:flex-row sm:items-center">
              {allResumes.length > 1 && (
                <Button
                  variant="secondary"
                  className="w-full sm:w-auto"
                  onClick={() => setShowResumeLibrary(!showResumeLibrary)}
                >
                  <FolderIcon className="w-4 h-4 mr-2" />
                  Library ({allResumes.length})
                </Button>
              )}
              <Button
                variant="secondary"
                className="w-full sm:w-auto"
                onClick={handleImportJsonResume}
                loading={uploading}
                loadingText="Importing..."
                title="Use a file exported from JobSentinel or another resume app"
              >
                Import from resume app
              </Button>
              <Button
                className="w-full sm:w-auto"
                onClick={handleUploadResume}
                loading={uploading}
                loadingText="Adding..."
              >
                {resume ? "Add New" : "Add Resume"}
              </Button>
            </div>
          </div>
        </div>
      </header>

      {showResumeLibrary && (
        <ResumeLibraryDropdown
          resumes={allResumes}
          onActivateResume={handleSetActiveResume}
          onDeleteResume={confirmDeleteResume}
        />
      )}

      <main className="max-w-7xl mx-auto p-6">
        {!resume ? (
          <ResumeEmptyState
            uploading={uploading}
            onImportJsonResume={handleImportJsonResume}
            onUploadResume={handleUploadResume}
          />
        ) : (
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
            {/* Resume Info */}
            <Card className="lg:col-span-1 dark:bg-surface-800">
              <h2 className="font-display text-display-sm text-surface-900 dark:text-white mb-4">
                Active Resume
              </h2>
              <div className="flex items-center gap-3 p-4 bg-surface-50 dark:bg-surface-700 rounded-lg mb-4">
                <div className="w-12 h-12 shrink-0 bg-sentinel-100 dark:bg-sentinel-900/30 rounded-lg flex items-center justify-center">
                  <DocumentIcon className="w-6 h-6 text-sentinel-600 dark:text-sentinel-400" />
                </div>
                <div className="min-w-0">
                  <p className="break-words [overflow-wrap:anywhere] font-medium text-surface-800 dark:text-surface-200">
                    {resume.name}
                  </p>
                  <p className="text-sm text-surface-500 dark:text-surface-400">
                    Added: {new Date(resume.created_at).toLocaleDateString("en-US")}
                  </p>
                </div>
              </div>
              <div
                data-testid="resume-import-status"
                className="mb-4 border-l-2 border-surface-200 dark:border-surface-700 pl-3 py-1"
              >
                <div className="flex flex-wrap items-center gap-2">
                  <Badge variant="surface" size="sm">
                    {getResumeFormatLabel(resume)}
                  </Badge>
                  <Badge variant={getReadableTextBadgeVariant(resume)} size="sm">
                    {getReadableTextLabel(resume)}
                  </Badge>
                </div>
                <p className="mt-2 text-xs text-surface-500 dark:text-surface-400">
                  {getReadableTextDescription(resume)}
                </p>
              </div>
              <Button
                variant="secondary"
                size="sm"
                className="w-full mb-4"
                onClick={handlePreviewResumeText}
                loading={textPreviewLoading}
                loadingText="Reading..."
              >
                See what JobSentinel read
              </Button>

              <div className="mb-4 rounded-lg border border-surface-200 dark:border-surface-700 bg-white dark:bg-surface-800 p-3">
                <div className="flex items-start justify-between gap-3 mb-2">
                  <div>
                    <h3 className="text-sm font-medium text-surface-800 dark:text-surface-200">
                      Resume Skills Sorting
                    </h3>
                    <p className="text-xs text-surface-500 dark:text-surface-400">
                      Use reviewed local skills as one signal when sorting jobs.
                    </p>
                  </div>
                  {resumeMatchingEnabled && (
                    <Badge variant="success" size="sm">
                      On
                    </Badge>
                  )}
                </div>
                {resumeMatchingEnabled ? (
                  <div className="space-y-3">
                    <p className="text-sm text-surface-600 dark:text-surface-300">
                      Resume skills are helping sort jobs.
                    </p>
                    <Button
                      variant="secondary"
                      size="sm"
                      className="w-full"
                      onClick={() => handleSetResumeMatching(false)}
                      loading={resumeMatchingLoading}
                      loadingText="Saving..."
                    >
                      Stop using resume skills
                    </Button>
                  </div>
                ) : skills.length > 0 ? (
                  <Button
                    size="sm"
                    className="w-full"
                    onClick={() => handleSetResumeMatching(true)}
                    loading={resumeMatchingLoading}
                    loadingText="Saving..."
                  >
                    Use these skills to sort jobs
                  </Button>
                ) : (
                  <p className="text-sm text-surface-500 dark:text-surface-400">
                    Review or add skills before using them to sort jobs.
                  </p>
                )}
              </div>

              <div className="mb-4">
                <div className="flex items-center justify-between mb-2">
                  <h3 className="text-sm font-medium text-surface-700 dark:text-surface-300">
                    Saved Skills ({skills.length})
                  </h3>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => setShowAddSkill(true)}
                  >
                    <PlusIcon className="w-4 h-4 mr-1" />
                    Add
                  </Button>
                </div>
                <div className="flex flex-wrap gap-2">
                  {skills.slice(0, 15).map((skill) => (
                    <Badge key={skill.id} variant={getSkillStrengthColor(skill.proficiency_level)}>
                      {skill.skill_name}
                      {skill.years_experience && ` • ${skill.years_experience}y`}
                      <span className="ml-1 text-xs opacity-70">
                        {getSkillSourceLabel(skill.source)}
                      </span>
                    </Badge>
                  ))}
                  {skills.length > 15 && (
                    <Badge variant="surface">+{skills.length - 15} more</Badge>
                  )}
                </div>
                <ResumeSkillStrengthMix skills={skills} />
              </div>
            </Card>

            <ResumeSkillsManagementCard
              skills={skills}
              categoryFilter={categoryFilter}
              setCategoryFilter={setCategoryFilter}
              showAddSkill={showAddSkill}
              setShowAddSkill={setShowAddSkill}
              newSkillForm={newSkillForm}
              setNewSkillForm={setNewSkillForm}
              editForm={editForm}
              setEditForm={setEditForm}
              editingSkillId={editingSkillId}
              setEditingSkillId={setEditingSkillId}
              onAddSkill={handleAddSkill}
              onUpdateSkill={handleUpdateSkill}
              onStartEditingSkill={startEditingSkill}
              onConfirmDeleteSkill={confirmDeleteSkill}
            />

            <ResumeRecentMatches
              matches={recentMatches}
              savingFeedback={savingFeedback}
              onFeedback={handleMatchFeedback}
            />
          </div>
        )}
      </main>

      <ResumeTextPreviewModal
        isOpen={showTextPreview}
        resume={resume}
        textPreview={textPreview}
        onClose={() => setShowTextPreview(false)}
        onCopyText={handleCopyResumeText}
      />

      {/* Delete Confirmation Modal */}
      <Modal
        isOpen={!!deleteConfirm}
        onClose={() => setDeleteConfirm(null)}
        title={`Delete ${deleteConfirm?.type === 'resume' ? 'Resume' : 'Skill'}?`}
      >
        <p className="text-surface-600 dark:text-surface-400 mb-4">
          Are you sure you want to delete <span className="font-medium text-surface-800 dark:text-surface-200">{deleteConfirm?.name}</span>?
          {deleteConfirm?.type === 'resume' && (
            <span className="block mt-2 text-sm text-red-600 dark:text-red-400">
              This will also remove all associated skills and match data.
            </span>
          )}
        </p>
        <ModalFooter>
          <Button variant="secondary" onClick={() => setDeleteConfirm(null)}>
            Cancel
          </Button>
          <Button
            variant="danger"
            onClick={() => {
              if (!deleteConfirm) return;
              if (deleteConfirm.type === 'resume') {
                handleDeleteResume(deleteConfirm.id);
              } else {
                handleDeleteSkill(deleteConfirm.id);
              }
            }}
          >
            Delete
          </Button>
        </ModalFooter>
      </Modal>
    </div>
  );
}
