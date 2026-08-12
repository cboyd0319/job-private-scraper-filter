import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ComponentProps } from "react";
import { describe, expect, it, vi } from "vitest";
import { ExternalAiReviewDialog } from "./ExternalAiReviewDialog";
import type { ExternalAiRequest } from "../../../shared/externalAi/externalAiTypes";

const publicRequest: ExternalAiRequest = {
  feature: "job-description-summary",
  sourceJobId: 42,
  labels: ["External AI optional", "Public-data only"],
  dataCategories: ["job_posting", "public_metadata"],
  payload: {
    title: "Operations Manager",
    company: "Example Co",
    description: "Lead scheduling and vendor coordination.",
  },
  redactedPayload: {
    title: "Operations Manager",
    company: "Example Co",
    description: "Lead scheduling and vendor coordination.",
  },
  previewShown: false,
  userApproved: false,
};

function renderDialog(
  request: ExternalAiRequest | null = publicRequest,
  props: Partial<ComponentProps<typeof ExternalAiReviewDialog>> = {},
) {
  const onCancel = vi.fn();
  const onApprove = vi.fn();

  render(
    <ExternalAiReviewDialog
      isOpen
      providerLabel="OpenAI"
      request={request}
      allowSensitivePayloads={false}
      onCancel={onCancel}
      onApprove={onApprove}
      {...props}
    />,
  );

  return { onCancel, onApprove };
}

describe("ExternalAiReviewDialog", () => {
  it("cancels without approving a request", async () => {
    const user = userEvent.setup();
    const { onCancel, onApprove } = renderDialog();

    await user.click(screen.getByRole("button", { name: "Cancel" }));

    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onApprove).not.toHaveBeenCalled();
  });

  it("allows cancellation while an approved request is in flight", async () => {
    const user = userEvent.setup();
    const onApprove = vi.fn(() => new Promise<void>(() => undefined));
    const { onCancel } = renderDialog(publicRequest, { onApprove });

    await user.click(
      screen.getByRole("button", { name: "Send Reviewed Details" }),
    );
    const cancelButton = screen.getByRole("button", { name: "Cancel" });

    expect(cancelButton).toBeEnabled();
    await user.click(cancelButton);
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("approves removing a stored public field", async () => {
    const user = userEvent.setup();
    const { onApprove } = renderDialog();
    const textarea = screen.getByLabelText("Details to send");

    fireEvent.change(textarea, {
      target: {
        value: JSON.stringify(
          {
            title: "Operations Manager",
            company: "Example Co",
          },
          null,
          2,
        ),
      },
    });
    await user.click(
      screen.getByRole("button", { name: "Send Reviewed Details" }),
    );

    expect(onApprove).toHaveBeenCalledWith(
      expect.objectContaining({
        previewShown: true,
        userApproved: true,
        redactedPayload: {
          title: "Operations Manager",
          company: "Example Co",
        },
      }),
    );
  });

  it("blocks adding or rewriting a stored public value", async () => {
    const user = userEvent.setup();
    const { onApprove } = renderDialog();

    fireEvent.change(screen.getByLabelText("Details to send"), {
      target: {
        value: JSON.stringify({
          title: "Operations Manager",
          company: "Example Co",
          description: "I am a protected veteran.",
        }),
      },
    });
    expect(
      screen.getByText(/adding or rewriting values is blocked/i),
    ).toBeInTheDocument();

    await user.click(
      screen.getByRole("button", { name: "Send Reviewed Details" }),
    );
    expect(onApprove).not.toHaveBeenCalled();
  });

  it("blocks invalid edited details", async () => {
    const user = userEvent.setup();
    const { onApprove } = renderDialog();

    fireEvent.change(screen.getByLabelText("Details to send"), {
      target: { value: "{" },
    });

    expect(
      screen.getByText("Fix the formatting before sending."),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Send Reviewed Details" }),
    ).toBeDisabled();

    await user.click(
      screen.getByRole("button", { name: "Send Reviewed Details" }),
    );
    expect(onApprove).not.toHaveBeenCalled();
  });

  it("blocks a duplicate send after an ambiguous provider outcome", async () => {
    const user = userEvent.setup();
    const onApprove = vi.fn().mockRejectedValue(
      new Error(
        "The Outside AI outcome is unknown. Check durable activity. Do not retry.",
      ),
    );
    renderDialog(publicRequest, { onApprove });

    const sendButton = screen.getByRole("button", {
      name: "Send Reviewed Details",
    });
    await user.click(sendButton);

    expect(await screen.findByText(/Do not retry/)).toBeInTheDocument();
    expect(sendButton).toBeDisabled();
    await user.click(sendButton);
    expect(onApprove).toHaveBeenCalledTimes(1);
  });

  it("requires explicit confirmation for private details", async () => {
    const user = userEvent.setup();
    const privateRequest: ExternalAiRequest = {
      ...publicRequest,
      labels: ["External AI optional", "Sensitive"],
      dataCategories: ["job_posting", "resume"],
      payload: {
        title: "Operations Manager",
        resumeText: "Private resume evidence",
      },
      redactedPayload: {
        title: "Operations Manager",
        resumeText: "Private resume evidence",
      },
    };
    const { onApprove } = renderDialog(privateRequest, {
      allowSensitivePayloads: true,
    });

    const sendButton = screen.getByRole("button", {
      name: "Send Reviewed Details",
    });
    expect(sendButton).toBeDisabled();

    await user.click(
      screen.getByRole("checkbox", {
        name: /I reviewed these private details/i,
      }),
    );
    await user.click(sendButton);

    expect(onApprove).toHaveBeenCalledWith(
      expect.objectContaining({
        explicitlyIncludedSensitiveData: true,
      }),
    );
  });

  it("explains when private details are off in settings", () => {
    const privateRequest: ExternalAiRequest = {
      ...publicRequest,
      labels: ["External AI optional", "Sensitive"],
      dataCategories: ["resume"],
      payload: { resumeText: "Private resume evidence" },
      redactedPayload: { resumeText: "Private resume evidence" },
    };

    renderDialog(privateRequest);

    expect(
      screen.getByText(/Private details are off in Settings/i),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Send Reviewed Details" }),
    ).toBeDisabled();
  });
});
