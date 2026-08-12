/** Verifies Dashboard overlay composition and exact research context. */

import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import {
  DashboardCompanyResearchOverlay,
  DashboardLinkedInWorkbenchModal,
} from "./DashboardOverlays";

describe("Dashboard overlays", () => {
  it("renders the app-composed LinkedIn workbench in its modal", () => {
    render(
      <DashboardLinkedInWorkbenchModal
        isOpen
        onClose={vi.fn()}
        workbench={<div>Composed LinkedIn workbench</div>}
      />,
    );

    expect(
      screen.getByRole("heading", { name: "LinkedIn Workbench" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Composed LinkedIn workbench")).toBeInTheDocument();
  });

  it("renders app-composed company research for the selected company", () => {
    const onClose = vi.fn();
    const renderCompanyResearch = vi.fn(({ companyName }) => (
      <div>Research for {companyName}</div>
    ));

    render(
      <DashboardCompanyResearchOverlay
        researchTarget={{ companyName: "CareBridge Services", jobHash: "job-carebridge" }}
        renderCompanyResearch={renderCompanyResearch}
        onClose={onClose}
      />,
    );

    expect(
      screen.getByRole("dialog", {
        name: "Company research for CareBridge Services",
      }),
    ).toBeInTheDocument();
    expect(screen.getByText("Research for CareBridge Services")).toBeInTheDocument();
    expect(renderCompanyResearch).toHaveBeenCalledWith({
      companyName: "CareBridge Services",
      jobHash: "job-carebridge",
      onClose,
    });
  });
});
