import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";

function generatedBrowserButtonCode(
  action: "import" | "applied" = "import",
): string {
  const source = readFileSync(
    resolve(process.cwd(), "src-tauri/src/ipc/bookmarklet.rs"),
    "utf8",
  );
  const match = source.match(
    /const TEMPLATE: &str = r#"(javascript:[\s\S]*?)"#;/,
  );

  if (!match?.[1]) {
    throw new Error("Browser button template was not found");
  }

  return match[1]
    .replaceAll("__PORT__", "4321")
    .replaceAll("__ORIGIN__", JSON.stringify("https://jobs.example"))
    .replaceAll(
      "__DESCRIPTION_CAPTURE__",
      action === "applied"
        ? "var desc=null;"
        : `var desc=first('[class*="description"],[class*="desc"]');`,
    )
    .replaceAll(
      "__SUCCESS_MESSAGE__",
      JSON.stringify(
        action === "applied"
          ? "Applied draft sent. Return to JobSentinel to review missing details."
          : "Request sent. Return to JobSentinel and check the review list. Copy a fresh button before retrying or importing another job.",
      ),
    )
    .replaceAll(
      "__PAIRING__",
      JSON.stringify({
        protocol_version: 1,
        pairing_id: "pairing-1",
        client_id: "browser-import",
        source_id: "user-source-actions",
        policy_ref: "jobsentinel.source-policy.user-source-actions",
        policy_revision: 1,
        operation:
          action === "applied" ? "applied_logging" : "visible_page_capture",
        origin: "https://jobs.example",
        nonce: "nonce-1",
        token: "test-token",
      }),
    );
}

describe("generated Browser Import button", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    document.body.innerHTML = "";
  });

  it("blocks restricted domains for both actions before page access", () => {
    const alertSpy = vi.spyOn(window, "alert").mockImplementation(() => {});
    const createElementSpy = vi.spyOn(document, "createElement");
    const querySelectorAllSpy = vi.spyOn(document, "querySelectorAll");
    const fetchSpy = vi.spyOn(window, "fetch");
    for (const action of ["import", "applied"] as const) {
      const code = generatedBrowserButtonCode(action).replace(
        /^javascript:/,
        "",
      );
      const run = new Function("location", code);

      for (const hostname of [
        "linkedin.com",
        "www.linkedin.com",
        "jobs.linkedin.com",
        "ycombinator.com",
        "www.ycombinator.com",
      ]) {
        run({
          hostname,
          href: `https://${hostname}/jobs/`,
          pathname: "/jobs",
        });
      }
    }

    expect(alertSpy).toHaveBeenCalledTimes(10);
    expect(alertSpy).toHaveBeenLastCalledWith(
      "Browser Import is unavailable for this source",
    );
    expect(createElementSpy).not.toHaveBeenCalled();
    expect(querySelectorAllSpy).not.toHaveBeenCalled();
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it("blocks an unpaired origin before reading or transporting page data", () => {
    const alertSpy = vi.spyOn(window, "alert").mockImplementation(() => {});
    const createElementSpy = vi.spyOn(document, "createElement");
    const querySelectorAllSpy = vi.spyOn(document, "querySelectorAll");
    const fetchSpy = vi.spyOn(window, "fetch");
    const code = generatedBrowserButtonCode().replace(/^javascript:/, "");
    const run = new Function("location", code);

    run({
      hostname: "other.example",
      href: "https://other.example/jobs/1",
      origin: "https://other.example",
      pathname: "/jobs/1",
      protocol: "https:",
    });

    expect(alertSpy).toHaveBeenCalledWith(
      "This Browser Import button is paired with a different site. Copy a fresh button for this page.",
    );
    expect(createElementSpy).not.toHaveBeenCalled();
    expect(querySelectorAllSpy).not.toHaveBeenCalled();
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it("captures rendered fields without reading hidden or structured page state", async () => {
    const fetchSpy = vi.fn().mockResolvedValue({});
    const alertSpy = vi.fn();
    const parent = { removeChild: vi.fn() };
    const frame = {
      contentWindow: {
        fetch: fetchSpy,
        JSON,
        URL,
        Object,
        Element: {
          prototype: {
            getClientRects(this: { rendered: boolean }) {
              return this.rendered ? [{ width: 100, height: 20 }] : [];
            },
          },
        },
        Document: {
          prototype: {
            querySelectorAll(
              this: { querySelectorAll: (selector: string) => unknown[] },
              selector: string,
            ) {
              return this.querySelectorAll(selector);
            },
          },
        },
        HTMLElement: { prototype: {} },
        getComputedStyle(element: { rendered: boolean }) {
          return {
            display: element.rendered ? "block" : "none",
            visibility: "visible",
            opacity: "1",
          };
        },
      },
      parentNode: parent,
      setAttribute: vi.fn(),
      style: { display: "" },
    };
    Object.defineProperty(
      frame.contentWindow.HTMLElement.prototype,
      "innerText",
      {
        get(this: { innerText: string }) {
          return this.innerText;
        },
      },
    );
    const hiddenDescription = {
      rendered: false,
      innerText: "HIDDEN_PRIVATE_DESCRIPTION",
    };
    const visibleDescription = {
      rendered: true,
      innerText: "Public job description",
    };
    const querySelectorAll = vi.fn((selector: string) => {
      const fields: Record<string, unknown[]> = {
        h1: [{ rendered: true, innerText: "Public role" }],
        '[class*="company"],[class*="employer"]': [
          { rendered: true, innerText: "Public employer" },
        ],
        '[class*="description"],[class*="desc"]': [
          hiddenDescription,
          visibleDescription,
        ],
        'script[type="application/ld+json"]': [
          { textContent: "JSON_LD_PRIVATE_DESCRIPTION" },
        ],
      };
      return fields[selector] ?? [];
    });
    const documentStub = {
      createElement: vi.fn(() => frame),
      documentElement: { appendChild: vi.fn() },
      body: { appendChild: vi.fn() },
      querySelectorAll,
    };
    const locationStub = {
      hostname: "jobs.example",
      href: "https://jobs.example/roles/1",
      origin: "https://jobs.example",
      pathname: "/roles/1",
      protocol: "https:",
    };
    const code = generatedBrowserButtonCode().replace(/^javascript:/, "");
    const run = new Function("location", "document", "window", "alert", code);

    run(locationStub, documentStub, { location: locationStub }, alertSpy);
    await Promise.resolve();

    const payload = JSON.parse(fetchSpy.mock.calls[0]?.[1]?.body as string) as {
      job: { title: string; company: string; description: string };
    };
    expect(payload.job).toEqual({
      title: "Public role",
      company: "Public employer",
      description: "Public job description",
      url: "https://jobs.example/roles/1",
    });
    expect(fetchSpy.mock.calls[0]?.[1]).toMatchObject({
      mode: "no-cors",
      targetAddressSpace: "loopback",
    });
    expect(querySelectorAll).not.toHaveBeenCalledWith(
      'script[type="application/ld+json"]',
    );
    expect(fetchSpy.mock.calls[0]?.[1]?.body).not.toContain(
      "HIDDEN_PRIVATE_DESCRIPTION",
    );
    expect(fetchSpy.mock.calls[0]?.[1]?.body).not.toContain(
      "JSON_LD_PRIVATE_DESCRIPTION",
    );
    expect(alertSpy).toHaveBeenCalledWith(
      "Request sent. Return to JobSentinel and check the review list. Copy a fresh button before retrying or importing another job.",
    );
  });
});
