/** Composes Dashboard-owned modal and overlay surfaces. */

import { Suspense, type ReactNode } from "react";
import { Button } from "../../../ui/Button";
import { FocusTrap } from "../../../ui/FocusTrap";
import { JobImportModal } from "./JobImportModal";
import { PanelSkeleton } from "../../../ui/LoadingFallbacks";
import { Modal, ModalFooter } from "../../../ui/Modal";
import ComponentErrorBoundary from "../errors/ComponentErrorBoundary";
import ModalErrorBoundary from "../errors/ModalErrorBoundary";
import { CheckCircleIcon } from "./DashboardIcons";
import type { DuplicateGroup } from "../types";
import { DuplicateGroupCard } from "./DuplicateGroupCard";
import type { CompanyResearchTarget, RenderCompanyResearch } from "../../../shared/companyResearch";

export function DashboardSettingsPanel({
  children,
  onClose,
}: {
  children: ReactNode;
  onClose: () => void;
}) {
  return (
    <ModalErrorBoundary
      modalName="settings"
      onClose={onClose}
    >
      <Suspense fallback={<PanelSkeleton />}>{children}</Suspense>
    </ModalErrorBoundary>
  );
}

export function DashboardDuplicateGroupsModal({
  isOpen,
  duplicateGroups,
  onClose,
  onMerge,
  onMergeAll,
}: {
  isOpen: boolean;
  duplicateGroups: DuplicateGroup[];
  onClose: () => void;
  onMerge: (primaryId: number, jobIds: number[]) => void;
  onMergeAll: () => void;
}) {
  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title="Possible Repeated Jobs"
    >
      <div className="space-y-4">
        {duplicateGroups.length === 0 ? (
          <div className="text-center py-8">
            <CheckCircleIcon className="w-12 h-12 text-green-500 mx-auto mb-3" />
            <p className="text-surface-600 dark:text-surface-400">
              No likely repeated postings found.
            </p>
          </div>
        ) : (
          <>
            <p className="text-sm text-surface-600 dark:text-surface-400">
              Found {duplicateGroups.length} possible repeat groups. These are
              similar saved postings, not proof that multiple sources confirmed
              the same job. Review before hiding extras.
            </p>
            <div className="space-y-4 max-h-96 overflow-y-auto">
              {duplicateGroups.map((group) => (
                <DuplicateGroupCard
                  key={group.primary_id}
                  group={group}
                  onMerge={onMerge}
                />
              ))}
            </div>
            <ModalFooter>
              <Button
                variant="secondary"
                onClick={onClose}
              >
                Close
              </Button>
              <Button onClick={onMergeAll}>
                Hide Extras ({duplicateGroups.length})
              </Button>
            </ModalFooter>
          </>
        )}
      </div>
    </Modal>
  );
}

export function DashboardLinkedInWorkbenchModal({
  isOpen,
  onClose,
  workbench,
}: {
  isOpen: boolean;
  onClose: () => void;
  workbench?: ReactNode;
}) {
  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title="LinkedIn Workbench"
      description="Use LinkedIn yourself, then save the jobs and actions you choose in JobSentinel."
      size="xl"
    >
      <Suspense fallback={<PanelSkeleton />}>{workbench}</Suspense>
    </Modal>
  );
}

export function DashboardCompanyResearchOverlay({
  researchTarget,
  renderCompanyResearch,
  onClose,
}: {
  researchTarget: CompanyResearchTarget | null;
  renderCompanyResearch?: RenderCompanyResearch;
  onClose: () => void;
}) {
  if (!researchTarget || !renderCompanyResearch) {
    return null;
  }

  return (
    <div
      className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
      onClick={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
      onKeyDown={(event) => {
        if (event.key === "Escape") onClose();
      }}
      role="dialog"
      aria-modal="true"
      aria-label={`Company research for ${researchTarget.companyName}`}
    >
      <FocusTrap>
        <ComponentErrorBoundary
          componentName="CompanyResearchPanel"
          fallback={() => (
            <div className="p-6 text-center">
              <p className="text-red-600 dark:text-red-400 mb-4">
                Could not load company research
              </p>
              <Button onClick={onClose}>
                Close
              </Button>
            </div>
          )}
        >
          <Suspense fallback={<PanelSkeleton />}>
            {renderCompanyResearch({
              companyName: researchTarget.companyName,
              jobHash: researchTarget.jobHash,
              onClose,
            })}
          </Suspense>
        </ComponentErrorBoundary>
      </FocusTrap>
    </div>
  );
}

export function DashboardImportJobModal({
  isOpen,
  onClose,
  onImportSuccess,
}: {
  isOpen: boolean;
  onClose: () => void;
  onImportSuccess: () => void;
}) {
  return (
    <JobImportModal
      isOpen={isOpen}
      onClose={onClose}
      onImportSuccess={onImportSuccess}
    />
  );
}
