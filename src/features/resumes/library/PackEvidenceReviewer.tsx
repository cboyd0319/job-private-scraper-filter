/** Runs one installed Evidence Reviewer only after its exact local plan is approved. */

import { useEffect, useRef, useState } from "react";
import { safeInvoke } from "../../../platform/tauri";
import { Button } from "../../../ui/Button";
import {
  parsePackManagementReviews,
  type PackManagementReview,
} from "../../../shared/packManagementProjection";

type SavedMatchIdentity = { job_hash: string; resume_id: number };
type ReviewStep = { action: string; label: string };
type TaskReview = {
  runId: string;
  approvalReference: string;
  taskKind: "evidence_review";
  plan: ReviewStep[];
  privacyLabels: ["local_only", "sensitive"];
  dataCategories: ["resume_evidence"];
  maxDurationSeconds: number;
  maxOutputBytes: number;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

const TASK_ID =
  /^pack-task:[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const APPROVAL_ID =
  /^pack-task-approval:[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const RECEIPT_ID =
  /^pack-task-receipt:[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

function parseTaskReview(value: unknown): TaskReview {
  if (!isRecord(value)) throw new Error("invalid task review");
  const planActions = Array.isArray(value.plan)
    ? value.plan.map((step) => (isRecord(step) ? step.action : null))
    : [];
  const validPlan =
    Array.isArray(value.plan) &&
    value.plan.every(
      (step) =>
        isRecord(step) &&
        ["read_selected_resume_evidence", "write_local_event"].includes(
          String(step.action),
        ) &&
        typeof step.label === "string" &&
        step.label.length > 0 &&
        step.label.length <= 120,
    );
  if (
    typeof value.runId !== "string" ||
    !TASK_ID.test(value.runId) ||
    typeof value.approvalReference !== "string" ||
    !APPROVAL_ID.test(value.approvalReference) ||
    value.taskKind !== "evidence_review" ||
    !validPlan ||
    JSON.stringify(planActions) !==
      JSON.stringify(["read_selected_resume_evidence", "write_local_event"]) ||
    JSON.stringify(value.privacyLabels) !==
      JSON.stringify(["local_only", "sensitive"]) ||
    JSON.stringify(value.dataCategories) !==
      JSON.stringify(["resume_evidence"]) ||
    !Number.isSafeInteger(value.maxDurationSeconds) ||
    Number(value.maxDurationSeconds) <= 0 ||
    Number(value.maxDurationSeconds) > 30 ||
    !Number.isSafeInteger(value.maxOutputBytes) ||
    Number(value.maxOutputBytes) <= 0 ||
    Number(value.maxOutputBytes) > 256 * 1024
  )
    throw new Error("invalid task review");
  return value as unknown as TaskReview;
}

function completedRequirementCount(value: unknown): number {
  if (
    !isRecord(value) ||
    typeof value.receiptId !== "string" ||
    !RECEIPT_ID.test(value.receiptId) ||
    !isRecord(value.review) ||
    !Array.isArray(value.review.requirements) ||
    value.review.requirements.length > 256 ||
    value.review.requirements.some(
      (requirement) =>
        !isRecord(requirement) ||
        typeof requirement.requirement !== "string" ||
        requirement.requirement.length === 0 ||
        requirement.requirement.length > 8192,
    )
  ) {
    throw new Error("invalid task result");
  }
  return value.review.requirements.length;
}

function reviewerPack(value: unknown): PackManagementReview | null {
  const matches = parsePackManagementReviews(value).filter(
    (pack) =>
      pack.state === "ready" &&
      pack.currentRelease.purpose === "resume_evidence_review",
  );
  return matches.length === 1 ? (matches[0] ?? null) : null;
}

export function PackEvidenceReviewer({ match }: { match: SavedMatchIdentity }) {
  const [pack, setPack] = useState<PackManagementReview | null>(null);
  const [task, setTask] = useState<TaskReview | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [completedCount, setCompletedCount] = useState<number | null>(null);
  const [availability, setAvailability] = useState<
    "loading" | "ready" | "absent" | "error"
  >("loading");
  const [reload, setReload] = useState(0);
  const pendingRun = useRef<string | null>(null);
  const reviewRef = useRef<HTMLDivElement | null>(null);
  const interaction = useRef(0);
  const matchKey = `${match.resume_id}:${match.job_hash}`;

  useEffect(() => {
    const interactionId = ++interaction.current;
    let current = true;
    setPack(null);
    setTask(null);
    setBusy(false);
    setError(null);
    setCompletedCount(null);
    setAvailability("loading");
    void safeInvoke<unknown>("list_pack_management", undefined, {
      logContext: "Find Evidence Reviewer pack",
    })
      .then((value) => {
        if (current && interaction.current === interactionId) {
          const found = reviewerPack(value);
          setPack(found);
          setAvailability(found ? "ready" : "absent");
        }
      })
      .catch(() => {
        if (current) {
          setPack(null);
          setAvailability("error");
        }
      });
    return () => {
      current = false;
      interaction.current += 1;
      const runId = pendingRun.current;
      pendingRun.current = null;
      if (runId) {
        void safeInvoke(
          "cancel_reviewed_pack_task",
          { runId },
          {
            logContext: "Cancel Evidence Reviewer pack task",
          },
        ).catch(() => undefined);
      }
    };
  }, [matchKey, reload]);

  useEffect(() => {
    if (task) reviewRef.current?.focus();
  }, [task]);

  const prepare = async () => {
    if (!pack || busy) return;
    const interactionId = interaction.current;
    setBusy(true);
    setError(null);
    try {
      const prepared = parseTaskReview(
        await safeInvoke(
          "prepare_evidence_reviewer",
          {
            publisherKeyId: pack.publisherKeyId,
            packId: pack.packId,
            expectedGeneration: pack.generation,
            jobHash: match.job_hash,
            resumeId: match.resume_id,
          },
          { logContext: "Prepare Evidence Reviewer pack task" },
        ),
      );
      if (interaction.current === interactionId) {
        pendingRun.current = prepared.runId;
        setTask(prepared);
      } else {
        await safeInvoke(
          "cancel_reviewed_pack_task",
          { runId: prepared.runId },
          {
            logContext: "Cancel Evidence Reviewer pack task",
          },
        );
      }
    } catch {
      if (interaction.current === interactionId) {
        setError("Evidence Reviewer could not prepare a current local review.");
      }
    } finally {
      if (interaction.current === interactionId) setBusy(false);
    }
  };

  const execute = async () => {
    if (!task || busy || pendingRun.current !== task.runId) return;
    const interactionId = interaction.current;
    setBusy(true);
    setError(null);
    try {
      const result = await safeInvoke(
        "execute_evidence_reviewer",
        {
          runId: task.runId,
          approvalReference: task.approvalReference,
          jobHash: match.job_hash,
          resumeId: match.resume_id,
        },
        { logContext: "Run Evidence Reviewer pack task" },
      );
      if (interaction.current === interactionId) {
        pendingRun.current = null;
        setTask(null);
        setCompletedCount(completedRequirementCount(result));
      }
    } catch {
      if (interaction.current === interactionId) {
        pendingRun.current = null;
        setTask(null);
        setError(
          "Evidence Reviewer stopped safely. Refresh current evidence before trying again.",
        );
      }
    } finally {
      if (interaction.current === interactionId) setBusy(false);
    }
  };

  return (
    <section className="mt-6 border-t border-surface-200 pt-4 dark:border-surface-700">
      <h3 className="font-medium text-surface-900 dark:text-white">
        Evidence Reviewer pack
      </h3>
      <p className="mt-1 text-sm text-surface-600 dark:text-surface-300">
        Runs locally against this exact saved match. It cannot use the network,
        credentials, or outside AI.
      </p>
      {availability === "loading" ? (
        <p role="status">Checking local pack availability...</p>
      ) : null}
      {availability === "absent" ? (
        <p>
          No active Evidence Reviewer pack is available. Review signed packs in
          Settings.
        </p>
      ) : null}
      {availability === "error" ? (
        <div role="alert">
          <p>Evidence Reviewer availability could not be checked.</p>
          <Button
            size="sm"
            variant="secondary"
            onClick={() => setReload((value) => value + 1)}
          >
            Retry availability
          </Button>
        </div>
      ) : null}
      {error ? (
        <p role="alert" className="mt-3 text-sm text-red-700 dark:text-red-300">
          {error}
        </p>
      ) : null}
      {completedCount !== null ? (
        <p
          role="status"
          className="mt-3 text-sm text-green-700 dark:text-green-300"
        >
          Evidence Reviewer completed locally for {completedCount}{" "}
          {completedCount === 1 ? "requirement" : "requirements"}.
        </p>
      ) : null}
      {!task && pack ? (
        <Button
          className="mt-3"
          variant="secondary"
          loading={busy}
          loadingText="Preparing…"
          onClick={() => void prepare()}
        >
          Prepare Evidence Reviewer
        </Button>
      ) : task ? (
        <div
          ref={reviewRef}
          tabIndex={-1}
          aria-label="Evidence Reviewer approval plan"
          className="mt-3 rounded-md border border-warning/40 bg-warning/10 p-3"
        >
          <p className="text-sm font-medium text-surface-800 dark:text-surface-100">
            Review this exact plan
          </p>
          <ul className="mt-2 list-disc space-y-1 pl-5 text-sm text-surface-700 dark:text-surface-200">
            {task.plan.map((step) => (
              <li key={`${step.action}:${step.label}`}>{step.label}</li>
            ))}
          </ul>
          <p className="mt-2 text-xs text-surface-600 dark:text-surface-300">
            Privacy: Local only, Sensitive. Data: Resume evidence. Maximum{" "}
            {task.maxDurationSeconds} seconds.
          </p>
          <Button
            className="mt-3"
            loading={busy}
            loadingText="Running…"
            onClick={() => void execute()}
          >
            Run Evidence Reviewer
          </Button>
        </div>
      ) : null}
    </section>
  );
}
