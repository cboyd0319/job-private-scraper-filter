import { validateExternalAiRequest } from "./requestValidation";
import {
  DEFAULT_EXTERNAL_AI_SETTINGS,
  ExternalAiGatewayError,
  type ExternalAiGateway,
  type ExternalAiRequest,
  type ExternalAiRequestLogger,
  type ExternalAiResponse,
  type ExternalAiSettings,
  type ExternalAiTransport,
  type PreparedExternalAiRequest,
} from "../externalAiTypes";

export function createExternalAiGateway(
  settings: ExternalAiSettings = DEFAULT_EXTERNAL_AI_SETTINGS,
  transport?: ExternalAiTransport,
  logRequest: ExternalAiRequestLogger = () => undefined,
): ExternalAiGateway {
  return {
    async send(request: ExternalAiRequest): Promise<ExternalAiResponse> {
      const provider = settings.provider;
      const outgoingPayload = validateExternalAiRequest(
        settings,
        provider,
        request,
        transport,
      );
      if (provider === "none") {
        throw new ExternalAiGatewayError(
          "provider_not_selected",
          "Choose the outside AI service before sending anything.",
        );
      }
      if (!transport) {
        throw new ExternalAiGatewayError(
          "transport_missing",
          "Outside AI sending is not set up.",
        );
      }

      const preparedRequest: PreparedExternalAiRequest = {
        feature: request.feature,
        sourceJobId: request.sourceJobId,
        provider,
        labels: request.labels,
        dataCategories: request.dataCategories,
        payload: outgoingPayload,
        previewShown: request.previewShown,
        userApproved: request.userApproved,
        explicitlyIncludedSensitiveData:
          request.explicitlyIncludedSensitiveData,
      };

      const response = await transport.send(preparedRequest);

      if (settings.logRequestsLocally) {
        await logRequest({
          feature: request.feature,
          provider,
          timestamp: new Date().toISOString(),
          labels: request.labels,
          dataCategories: request.dataCategories,
        });
      }

      return response;
    },
  };
}
