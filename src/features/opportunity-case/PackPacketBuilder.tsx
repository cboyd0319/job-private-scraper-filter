/** Previews and runs one installed Packet Builder under an exact local approval. */

import { useEffect, useRef, useState } from "react";
import { safeInvoke } from "../../platform/tauri";
import { Button } from "../../ui/Button";
import {
  parsePackManagementReviews,
  type PackManagementReview,
} from "../../shared/packManagementProjection";

type Task = {
  runId: string;
  approvalReference: string;
  plan: Array<{ action: string; label: string }>;
  maxDurationSeconds: number;
};
type Packet = {
  jobTitle: string;
  company: string;
  resumeName: string;
  evidenceRequirements: string[];
  reviewedClaims: Array<{ reviewed_text: string }>;
  attachmentChecklist: string[];
  questionsToReview: string[];
  screeningAnswersRequireManualReview: true;
  finalSubmissionByUser: true;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function nonEmptyStrings(value: unknown): value is string[] {
  return (
    Array.isArray(value) &&
    value.every(
      (item) =>
        typeof item === "string" && item.length > 0 && item.length <= 8192,
    )
  );
}

function parsePacket(value: unknown): Packet {
  if (!isRecord(value)) throw new Error("invalid packet");
  if (
    ![value.jobTitle, value.company, value.resumeName].every(
      (item) =>
        typeof item === "string" && item.length > 0 && item.length <= 512,
    ) ||
    !isRecord(value.evidenceReview) ||
    !Array.isArray(value.evidenceReview.requirements) ||
    value.evidenceReview.requirements.some(
      (requirement) =>
        !isRecord(requirement) ||
        typeof requirement.requirement !== "string" ||
        requirement.requirement.length === 0 ||
        requirement.requirement.length > 8192,
    ) ||
    !Array.isArray(value.reviewedClaims) ||
    value.reviewedClaims.some(
      (claim) =>
        !isRecord(claim) ||
        typeof claim.reviewed_text !== "string" ||
        claim.reviewed_text.length === 0 ||
        claim.reviewed_text.length > 8192,
    ) ||
    !nonEmptyStrings(value.attachmentChecklist) ||
    !nonEmptyStrings(value.questionsToReview) ||
    value.screeningAnswersRequireManualReview !== true ||
    value.finalSubmissionByUser !== true
  )
    throw new Error("invalid packet");
  return {
    ...(value as unknown as Omit<Packet, "evidenceRequirements">),
    evidenceRequirements: value.evidenceReview.requirements.map((requirement) =>
      String((requirement as Record<string, unknown>).requirement),
    ),
  };
}

const PACKET_ACTIONS = [
  "read_selected_case_file",
  "read_selected_resume_evidence",
  "read_public_job_posting",
  "create_draft_application_packet",
  "write_local_event",
];

const TASK_ID =
  /^pack-task:[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const APPROVAL_ID =
  /^pack-task-approval:[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const RECEIPT_ID =
  /^pack-task-receipt:[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

function parsePreparation(value: unknown): { task: Task; packet: Packet } {
  if (!isRecord(value) || !isRecord(value.task))
    throw new Error("invalid preparation");
  const task = value.task;
  const actions = Array.isArray(task.plan)
    ? task.plan.map((step) => (isRecord(step) ? step.action : null))
    : [];
  if (
    typeof task.runId !== "string" ||
    !TASK_ID.test(task.runId) ||
    typeof task.approvalReference !== "string" ||
    !APPROVAL_ID.test(task.approvalReference) ||
    task.taskKind !== "draft_packet" ||
    JSON.stringify(actions) !== JSON.stringify(PACKET_ACTIONS) ||
    !Array.isArray(task.plan) ||
    task.plan.some(
      (step) =>
        !isRecord(step) ||
        typeof step.label !== "string" ||
        step.label.length === 0 ||
        step.label.length > 120,
    ) ||
    JSON.stringify(task.privacyLabels) !==
      JSON.stringify(["local_only", "sensitive"]) ||
    JSON.stringify(task.dataCategories) !==
      JSON.stringify(["public_job_posting", "resume_evidence"]) ||
    !Number.isSafeInteger(task.maxDurationSeconds) ||
    Number(task.maxDurationSeconds) <= 0 ||
    Number(task.maxDurationSeconds) > 60 ||
    !Number.isSafeInteger(task.maxOutputBytes) ||
    Number(task.maxOutputBytes) <= 0 ||
    Number(task.maxOutputBytes) > 512 * 1024
  )
    throw new Error("invalid preparation");
  return { task: task as unknown as Task, packet: parsePacket(value.packet) };
}

function findPacketPack(value: unknown): PackManagementReview | null {
  const packs = parsePackManagementReviews(value).filter(
    (pack) =>
      pack.state === "ready" &&
      pack.currentRelease.purpose === "draft_application_packet",
  );
  return packs.length === 1 ? (packs[0] ?? null) : null;
}

export function PackPacketBuilder({ jobHash }: { jobHash: string }) {
  const [pack, setPack] = useState<PackManagementReview | null>(null);
  const [resumeId, setResumeId] = useState<number | null>(null);
  const [task, setTask] = useState<Task | null>(null);
  const [packet, setPacket] = useState<Packet | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [completed, setCompleted] = useState(false);
  const [availability, setAvailability] = useState<
    "loading" | "ready" | "no_resume" | "absent" | "error"
  >("loading");
  const [reload, setReload] = useState(0);
  const interaction = useRef(0);
  const pendingRun = useRef<string | null>(null);
  const reviewRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const interactionId = ++interaction.current;
    setPack(null);
    setResumeId(null);
    setTask(null);
    setPacket(null);
    setBusy(false);
    setError(null);
    setCompleted(false);
    setAvailability("loading");
    void Promise.all([
      safeInvoke<unknown>("list_pack_management", undefined, {
        logContext: "Find Packet Builder pack",
      }),
      safeInvoke<unknown>("get_active_resume", undefined, {
        logContext: "Find Packet Builder resume",
      }),
    ])
      .then(([packs, resume]) => {
        if (interaction.current !== interactionId) return;
        if (resume === null) {
          setAvailability("no_resume");
          return;
        }
        if (
          !isRecord(resume) ||
          !Number.isSafeInteger(resume.id) ||
          Number(resume.id) <= 0
        )
          throw new Error("invalid active resume");
        const found = findPacketPack(packs);
        setPack(found);
        setResumeId(Number(resume.id));
        setAvailability(found ? "ready" : "absent");
      })
      .catch(() => {
        if (interaction.current === interactionId) setAvailability("error");
      });
    return () => {
      interaction.current += 1;
      const runId = pendingRun.current;
      pendingRun.current = null;
      if (runId)
        void safeInvoke(
          "cancel_reviewed_pack_task",
          { runId },
          {
            logContext: "Cancel Packet Builder pack task",
          },
        ).catch(() => undefined);
    };
  }, [jobHash, reload]);

  useEffect(() => {
    if (task) reviewRef.current?.focus();
  }, [task]);

  const prepare = async () => {
    if (!pack || !resumeId || busy) return;
    const interactionId = interaction.current;
    setBusy(true);
    setError(null);
    try {
      const prepared = parsePreparation(
        await safeInvoke(
          "prepare_packet_builder",
          {
            publisherKeyId: pack.publisherKeyId,
            packId: pack.packId,
            expectedGeneration: pack.generation,
            jobHash,
            resumeId,
          },
          { logContext: "Prepare Packet Builder pack task" },
        ),
      );
      if (interaction.current !== interactionId) {
        await safeInvoke(
          "cancel_reviewed_pack_task",
          { runId: prepared.task.runId },
          {
            logContext: "Cancel Packet Builder pack task",
          },
        );
        return;
      }
      pendingRun.current = prepared.task.runId;
      setTask(prepared.task);
      setPacket(prepared.packet);
    } catch {
      if (interaction.current === interactionId)
        setError("Packet Builder could not prepare a current local draft.");
    } finally {
      if (interaction.current === interactionId) setBusy(false);
    }
  };

  const execute = async () => {
    if (!task || !resumeId || busy || pendingRun.current !== task.runId) return;
    const interactionId = interaction.current;
    setBusy(true);
    setError(null);
    try {
      const result = await safeInvoke<unknown>(
        "execute_packet_builder",
        {
          runId: task.runId,
          approvalReference: task.approvalReference,
          jobHash,
          resumeId,
        },
        { logContext: "Run Packet Builder pack task" },
      );
      if (
        !isRecord(result) ||
        typeof result.receiptId !== "string" ||
        !RECEIPT_ID.test(result.receiptId)
      )
        throw new Error("invalid result");
      const completedPacket = parsePacket(result.packet);
      if (interaction.current === interactionId) {
        pendingRun.current = null;
        setTask(null);
        setPacket(completedPacket);
        setCompleted(true);
      }
    } catch {
      if (interaction.current === interactionId) {
        pendingRun.current = null;
        setTask(null);
        setPacket(null);
        setError(
          "Packet Builder stopped safely. Refresh the case before trying again.",
        );
      }
    } finally {
      if (interaction.current === interactionId) setBusy(false);
    }
  };

  return (
    <section className="rounded-lg border border-surface-200 p-3 dark:border-surface-700">
      <h5 className="font-semibold text-surface-900 dark:text-white">
        Packet Builder pack
      </h5>
      <p>
        Creates a local draft from this case and the active saved resume.
        Nothing is sent or submitted.
      </p>
      {availability === "loading" ? (
        <p role="status">Checking local packet tools...</p>
      ) : null}
      {availability === "no_resume" ? (
        <p>Choose an active saved resume before preparing a packet.</p>
      ) : null}
      {availability === "absent" ? (
        <p>
          No active Packet Builder pack is available. Review signed packs in
          Settings.
        </p>
      ) : null}
      {availability === "error" ? (
        <div role="alert">
          <p>Packet Builder availability could not be checked.</p>
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
        <p role="alert" className="mt-2 text-danger">
          {error}
        </p>
      ) : null}
      {completed ? (
        <p role="status" className="mt-2 text-success">
          Local draft packet is ready.
        </p>
      ) : null}
      {!packet && pack && resumeId ? (
        <Button
          className="mt-3"
          variant="secondary"
          size="sm"
          loading={busy}
          loadingText="Preparing…"
          onClick={() => void prepare()}
        >
          Prepare Packet Builder
        </Button>
      ) : packet ? (
        <div className="mt-3 space-y-3">
          <p className="font-medium">
            {packet.jobTitle} at {packet.company}, using {packet.resumeName}
          </p>
          <PacketList
            title="Evidence review"
            values={packet.evidenceRequirements}
          />
          <PacketList
            title="Reviewed claims"
            values={packet.reviewedClaims.map((claim) => claim.reviewed_text)}
          />
          <PacketList
            title="Attachment checklist"
            values={packet.attachmentChecklist}
          />
          <PacketList
            title="Questions to review"
            values={packet.questionsToReview}
          />
          <p>
            Screening answers require manual review. Final submission stays with
            you.
          </p>
          {task ? (
            <div
              ref={reviewRef}
              tabIndex={-1}
              aria-label="Packet Builder approval plan"
              className="rounded-md border border-warning/40 bg-warning/10 p-3"
            >
              <p className="font-medium">Review this exact local plan</p>
              <ul className="mt-2 list-disc pl-5">
                {task.plan.map((step) => (
                  <li key={step.action}>{step.label}</li>
                ))}
              </ul>
              <Button
                className="mt-3"
                size="sm"
                loading={busy}
                loadingText="Building…"
                onClick={() => void execute()}
              >
                Approve local packet
              </Button>
            </div>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}

function PacketList({ title, values }: { title: string; values: string[] }) {
  return (
    <div>
      <p className="font-medium">{title}</p>
      {values.length > 0 ? (
        <ul className="list-disc pl-5">
          {values.map((value) => (
            <li key={value}>{value}</li>
          ))}
        </ul>
      ) : (
        <p>None.</p>
      )}
    </div>
  );
}
