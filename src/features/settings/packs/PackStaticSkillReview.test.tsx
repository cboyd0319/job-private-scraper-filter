/** Proves active static skills remain inert text with complete local resources. */

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "../../../platform/tauri";
import { PackStaticSkillReview } from "./PackStaticSkillReview";
import { pack, release } from "./packManagementTestData";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockInvoke = vi.mocked(invoke);

describe("PackStaticSkillReview", () => {
  beforeEach(() => mockInvoke.mockReset());

  it("re-verifies and displays static instructions and every resource as plain text", async () => {
    const user = userEvent.setup();
    const skillRelease = release({
      packType: "skill",
      purpose: "static_guidance",
    });
    const skillPack = pack({
      publisherKeyId: "jobsentinel-skill-publisher-v1",
      packId: "jobsentinel.skill.resume-review",
      currentRelease: skillRelease,
      releases: [skillRelease],
    });
    mockInvoke.mockResolvedValueOnce({
      skillName: "resume-review",
      skillMd: "# Resume review\n\n<button>Injected control</button>",
      resources: [
        { path: "references/rubric.md", content: "Rubric content" },
        { path: "assets/checklist.txt", content: "Checklist content" },
      ],
      handoff: { taskKind: "evidence_review", label: "Open Evidence Reviewer" },
    });

    render(<PackStaticSkillReview pack={skillPack} />);
    await user.click(screen.getByRole("button", { name: "Open static skill" }));

    expect(mockInvoke).toHaveBeenCalledWith("open_static_skill", {
      publisherKeyId: skillPack.publisherKeyId,
      packId: skillPack.packId,
      expectedGeneration: skillPack.generation,
    });
    expect(screen.getByText("references/rubric.md")).toBeInTheDocument();
    expect(screen.getByText("Rubric content")).toBeInTheDocument();
    expect(screen.getByText("Checklist content")).toBeInTheDocument();
    expect(
      screen.getByText(/<button>Injected control<\/button>/),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Injected control" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByText(
        /Check a saved match's Match Debugger for an eligible Evidence Reviewer pack/,
      ),
    ).toBeInTheDocument();
  });

  it("keeps verification failures generic and closes stale content", async () => {
    const user = userEvent.setup();
    mockInvoke.mockRejectedValueOnce(new Error("private artifact path"));
    render(
      <PackStaticSkillReview
        pack={pack({
          currentRelease: release({
            packType: "skill",
            purpose: "static_guidance",
          }),
        })}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Open static skill" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Static skill could not be verified. Refresh Packs and try again.",
    );
    expect(screen.queryByText("private artifact path")).not.toBeInTheDocument();
  });

  it("clears opened content when the durable pack generation changes", async () => {
    const user = userEvent.setup();
    const skillRelease = release({
      packType: "skill",
      purpose: "static_guidance",
    });
    const skillPack = pack({
      currentRelease: skillRelease,
      releases: [skillRelease],
    });
    mockInvoke.mockResolvedValueOnce({
      skillName: "resume-review",
      skillMd: "# Current instructions",
      resources: [],
      handoff: null,
    });
    const { rerender } = render(<PackStaticSkillReview pack={skillPack} />);
    await user.click(screen.getByRole("button", { name: "Open static skill" }));
    expect(
      await screen.findByText("# Current instructions"),
    ).toBeInTheDocument();

    rerender(<PackStaticSkillReview pack={{ ...skillPack, generation: 5 }} />);

    expect(
      screen.queryByText("# Current instructions"),
    ).not.toBeInTheDocument();
  });

  it("rejects unsafe or case-colliding resource paths before display", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValueOnce({
      skillName: "resume-review",
      skillMd: "# Current instructions",
      resources: [
        { path: "references/Rubric.md", content: "One" },
        { path: "references/rubric.md", content: "Two" },
      ],
      handoff: null,
    });
    render(
      <PackStaticSkillReview
        pack={pack({
          currentRelease: release({
            packType: "skill",
            purpose: "static_guidance",
          }),
        })}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Open static skill" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Static skill could not be verified. Refresh Packs and try again.",
    );
    expect(screen.queryByText("One")).not.toBeInTheDocument();
  });
});
