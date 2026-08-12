export const FEATURE_PRIVACY_LABELS = [
  "Local only",
  "External AI optional",
  "External AI required",
  "Sensitive",
  "Public-data only",
] as const;

export type FeaturePrivacyLabel = (typeof FEATURE_PRIVACY_LABELS)[number];

export type ExternalAiProvider =
  | "none"
  | "open_ai"
  | "anthropic"
  | "google_gemini"
  | "github_copilot"
  | "custom";

export type ExternalAiDataCategory =
  | "job_posting"
  | "public_metadata"
  | "resume"
  | "salary_floor"
  | "private_notes"
  | "application_history"
  | "career_goals"
  | "location_preferences"
  | "full_database";

export interface ExternalAiSettings {
  enabled: boolean;
  provider: ExternalAiProvider;
  requirePayloadPreview: boolean;
  allowSensitivePayloads: boolean;
  redaction: {
    enabled: boolean;
  };
  logRequestsLocally: boolean;
}

export const DEFAULT_EXTERNAL_AI_SETTINGS: ExternalAiSettings = {
  enabled: false,
  provider: "none",
  requirePayloadPreview: true,
  allowSensitivePayloads: false,
  redaction: {
    enabled: true,
  },
  logRequestsLocally: true,
};

export interface ExternalAiRequest {
  feature: string;
  sourceJobId: number;
  labels: FeaturePrivacyLabel[];
  dataCategories: ExternalAiDataCategory[];
  payload: Record<string, unknown>;
  redactedPayload?: Record<string, unknown>;
  previewShown: boolean;
  userApproved: boolean;
  explicitlyIncludedSensitiveData?: boolean;
}

export interface PreparedExternalAiRequest {
  feature: string;
  sourceJobId: number;
  provider: Exclude<ExternalAiProvider, "none">;
  labels: FeaturePrivacyLabel[];
  dataCategories: ExternalAiDataCategory[];
  payload: Record<string, unknown>;
  previewShown: boolean;
  userApproved: boolean;
  explicitlyIncludedSensitiveData?: boolean;
}

export interface ExternalAiResponse {
  text: string;
  raw?: unknown;
}

export interface ExternalAiRequestLog {
  feature: string;
  provider: Exclude<ExternalAiProvider, "none">;
  timestamp: string;
  labels: FeaturePrivacyLabel[];
  dataCategories: ExternalAiDataCategory[];
}

export interface ExternalAiTransport {
  send(request: PreparedExternalAiRequest): Promise<ExternalAiResponse>;
}

export interface ExternalAiGateway {
  send(request: ExternalAiRequest): Promise<ExternalAiResponse>;
}

export type ExternalAiRequestLogger = (
  entry: ExternalAiRequestLog,
) => void | Promise<void>;

export type ExternalAiGatewayErrorCode =
  | "external_ai_disabled"
  | "provider_not_selected"
  | "transport_missing"
  | "payload_preview_required"
  | "user_approval_required"
  | "redacted_payload_required"
  | "unclassified_payload_key"
  | "external_ai_prompt_injection_blocked"
  | "full_database_blocked"
  | "sensitive_payload_blocked"
  | "public_data_only_violation";

export class ExternalAiGatewayError extends Error {
  constructor(
    public readonly code: ExternalAiGatewayErrorCode,
    message: string,
  ) {
    super(message);
    this.name = "ExternalAiGatewayError";
  }
}
