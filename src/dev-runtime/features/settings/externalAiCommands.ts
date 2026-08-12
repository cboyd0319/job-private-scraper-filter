import { getArg, getStringArg } from "../../mocks/handlers/commandHelpers";
import type { MockJob } from "../../mocks/handlers/types";
import type { MockExternalAiState } from "../../mocks/runtimeState";

export interface MockExternalAiCommandResult {
  handled: boolean;
  value: unknown;
}

interface MockExternalAiPrepareResponse {
  approvalId: string;
  providerId: string;
  destination: string;
  model: string;
  fieldCount: number;
}

interface MockExternalAiActivityEntry {
  providerId: string;
  destination: string;
  status: "succeeded" | "cancelled";
  createdAt: string;
  completedAt: string | null;
}

const MOCK_OUTSIDE_AI_DESTINATION = "https://mock.invalid/outside-ai";
const MOCK_OUTSIDE_AI_MODEL = "mock-local-development";
const REVIEW_UNAVAILABLE =
  "This Outside AI request has no current matching review. Review it again before sending.";
const PREPARATION_FAILED = "Reviewed details could not be prepared.";
const STORED_JOB_UNAVAILABLE = "The stored public job could not be verified.";
const PUBLIC_JOB_FIELDS_UNCHANGED =
  "Outside AI details must be unchanged fields from the stored public job. Remove fields instead of adding or rewriting them.";
const SUPPORTED_PROVIDERS = [
  "open_ai",
  "anthropic",
  "google_gemini",
  "github_copilot",
  "custom",
] as const;
const PUBLIC_DATA_CATEGORIES = ["job_posting", "public_metadata"] as const;

interface MockExternalAiCommandState extends MockExternalAiState {
  jobs: readonly MockJob[];
}

interface MockPreparedExternalAiRequest extends Record<string, unknown> {
  sourceJobId: number;
  payload: Record<string, unknown>;
}

export function handleMockExternalAiCommand(
  command: string,
  args: Record<string, unknown> | undefined,
  state: MockExternalAiCommandState,
): MockExternalAiCommandResult {
  switch (command) {
    case "prepare_external_ai_request":
      return handled(prepareMockExternalAiRequest(args, state));

    case "send_external_ai_request":
      return handled(sendMockExternalAiRequest(args, state));

    case "cancel_external_ai_request":
      cancelMockExternalAiRequest(args, state);
      return handled({ outcome: "cancelled" });

    case "list_external_ai_activity":
      return handled(getMockExternalAiActivity());

    default:
      return { handled: false, value: undefined };
  }
}

function handled(value: unknown): MockExternalAiCommandResult {
  return { handled: true, value };
}

function prepareMockExternalAiRequest(
  args: Record<string, unknown> | undefined,
  state: MockExternalAiCommandState,
): MockExternalAiPrepareResponse {
  const request = getPreparedRequest(args, state.jobs);
  const preparedRequest = JSON.parse(JSON.stringify(request)) as Record<
    string,
    unknown
  >;
  const payload = preparedRequest.payload;
  const providerId =
    typeof preparedRequest.provider === "string" && preparedRequest.provider
      ? preparedRequest.provider
      : "open_ai";
  const approvalId = `outside-ai-approval:${globalThis.crypto.randomUUID()}`;
  state.pendingExternalAiApproval = {
    approvalId,
    request: preparedRequest,
    providerId,
    model: MOCK_OUTSIDE_AI_MODEL,
  };
  return {
    approvalId,
    providerId,
    destination: MOCK_OUTSIDE_AI_DESTINATION,
    model: MOCK_OUTSIDE_AI_MODEL,
    fieldCount:
      payload && typeof payload === "object" && !Array.isArray(payload)
        ? Object.keys(payload).length
        : 0,
  };
}

function sendMockExternalAiRequest(
  args: Record<string, unknown> | undefined,
  state: MockExternalAiState,
): { text: string; provider: string; model: string } {
  const approval = requireCurrentMockExternalAiApproval(args, state, true);
  return {
    text: "Mock outside AI summary: review the original posting before using this summary.",
    provider: approval.providerId,
    model: approval.model,
  };
}

function cancelMockExternalAiRequest(
  args: Record<string, unknown> | undefined,
  state: MockExternalAiState,
): void {
  requireCurrentMockExternalAiApproval(args, state, false);
  state.pendingExternalAiApproval = null;
}

function requireCurrentMockExternalAiApproval(
  args: Record<string, unknown> | undefined,
  state: MockExternalAiState,
  requireExactRequest: boolean,
): NonNullable<MockExternalAiState["pendingExternalAiApproval"]> {
  const approval = state.pendingExternalAiApproval;
  const approvalId = getStringArg(args, "approvalId")?.trim();
  if (!approval || !approvalId || approval.approvalId !== approvalId) {
    throw new Error(
      requireExactRequest
        ? REVIEW_UNAVAILABLE
        : "Outside AI approval could not be verified.",
    );
  }
  if (
    requireExactRequest &&
    !isExactPreparedRequest(getRequest(args), approval.request)
  ) {
    throw new Error(REVIEW_UNAVAILABLE);
  }
  if (requireExactRequest) state.pendingExternalAiApproval = null;
  return approval;
}

function getRequest(
  args: Record<string, unknown> | undefined,
): Record<string, unknown> {
  const request = getArg(args, "request");
  return request && typeof request === "object" && !Array.isArray(request)
    ? (request as Record<string, unknown>)
    : {};
}

function getPreparedRequest(
  args: Record<string, unknown> | undefined,
  jobs: readonly MockJob[],
): Record<string, unknown> {
  const request = getArg(args, "request");
  if (!isRecord(request) || !hasPreparedRequestShape(request)) {
    throw new Error(PREPARATION_FAILED);
  }
  const job = jobs.find(({ id }) => id === request.sourceJobId);
  if (!job) throw new Error(STORED_JOB_UNAVAILABLE);
  validatePublicJobPayload(request.payload, job);
  return request;
}

function hasPreparedRequestShape(
  request: Record<string, unknown>,
): request is MockPreparedExternalAiRequest {
  return (
    request.feature === "job-description-summary" &&
    typeof request.sourceJobId === "number" &&
    Number.isSafeInteger(request.sourceJobId) &&
    request.sourceJobId > 0 &&
    SUPPORTED_PROVIDERS.includes(
      request.provider as (typeof SUPPORTED_PROVIDERS)[number],
    ) &&
    hasExactPublicLabels(request.labels) &&
    hasPublicDataCategories(request.dataCategories) &&
    typeof request.previewShown === "boolean" &&
    typeof request.userApproved === "boolean" &&
    (request.explicitlyIncludedSensitiveData === undefined ||
      request.explicitlyIncludedSensitiveData === false) &&
    isRecord(request.payload)
  );
}

function hasExactPublicLabels(value: unknown): boolean {
  return (
    Array.isArray(value) &&
    value.length === 2 &&
    new Set(value).size === 2 &&
    value.includes("External AI optional") &&
    value.includes("Public-data only")
  );
}

function hasPublicDataCategories(value: unknown): boolean {
  return (
    Array.isArray(value) &&
    value.length > 0 &&
    new Set(value).size === value.length &&
    value.every((category) =>
      PUBLIC_DATA_CATEGORIES.includes(
        category as (typeof PUBLIC_DATA_CATEGORIES)[number],
      ),
    )
  );
}

function validatePublicJobPayload(
  payload: Record<string, unknown>,
  job: MockJob,
): void {
  const fields: Record<string, string> = {
    title: job.title,
    company: job.company,
  };
  for (const key of ["location", "description"] as const) {
    const value = job[key]?.trim();
    if (value) fields[key] = value;
  }
  const entries = Object.entries(payload);
  if (entries.length === 0) {
    throw new Error("Choose at least one stored public job field to send.");
  }
  if (entries.some(([key, value]) => fields[key] !== value)) {
    throw new Error(PUBLIC_JOB_FIELDS_UNCHANGED);
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function isExactPreparedRequest(
  request: Record<string, unknown>,
  preparedRequest: Record<string, unknown>,
): boolean {
  return JSON.stringify(request) === JSON.stringify(preparedRequest);
}

function getMockExternalAiActivity(): MockExternalAiActivityEntry[] {
  return [
    {
      providerId: "open_ai",
      destination: MOCK_OUTSIDE_AI_DESTINATION,
      status: "succeeded",
      createdAt: new Date(Date.now() - 86400000).toISOString(),
      completedAt: new Date(Date.now() - 86340000).toISOString(),
    },
    {
      providerId: "anthropic",
      destination: MOCK_OUTSIDE_AI_DESTINATION,
      status: "cancelled",
      createdAt: new Date(Date.now() - 172800000).toISOString(),
      completedAt: new Date(Date.now() - 172740000).toISOString(),
    },
  ];
}
