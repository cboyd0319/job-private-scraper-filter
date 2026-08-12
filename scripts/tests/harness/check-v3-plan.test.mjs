import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import test from "node:test";

import {
  collectV3IdeaDispositionViolations,
  collectV3PlanViolations,
  hasStaleV3ReleaseTruth,
} from "../../harness/checks/v3-plan.mjs";

function writeFixture(root, path, content) {
  const fullPath = join(root, path);
  mkdirSync(dirname(fullPath), { recursive: true });
  writeFileSync(fullPath, content, "utf8");
}

function withFixture(callback) {
  const root = mkdtempSync(join(tmpdir(), "jobsentinel-v3-plan-"));
  try {
    callback(root);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

test("v3 plan rejects missing and mismatched idea dispositions", () => {
  withFixture((root) => {
    writeFixture(
      root,
      "docs/plans/v3/idea-index.md",
      [
        "| A01 | Now idea | 5 | 3 | Now | Required. |",
        "| A02 | Next idea | 4 | 3 | Next | Useful. |",
        "| A03 | Later idea | 3 | 4 | Later | Dependent. |",
        "| A04 | Moonshot idea | 3 | 5 | Moonshot | Research. |",
      ].join("\n"),
    );
    writeFixture(
      root,
      "docs/plans/v3/master-exec-plan.md",
      [
        "all 1 `Now` ideas; all 1 `Next` ideas; all 1 `Later` and 1 `Moonshot` ideas",
        "### V3.0 Accepted",
        "| Milestone | Accepted `Now` IDs |",
        "| --- | --- |",
        "| 1 | A01, A02 |",
        "### V3.x Accepted After V3.0",
        "| Order | V3.x train | Accepted `Next` IDs | Dependency rule |",
        "| --- | --- | --- | --- |",
        "### Explicitly Deferred Beyond V3",
        "- `Later`: A03.",
        "- `Moonshot`:",
        "- Promotion trigger: evidence.",
      ].join("\n"),
    );

    assert.deepEqual(collectV3IdeaDispositionViolations(root), [
      "v3 idea A02 is Next but disposed as Now",
      "v3 idea disposition is missing ID: A04",
    ]);
  });
});

test("v3 docs audit rejects stale release-truth markers only in maintained owners", () => {
  withFixture((root) => {
    writeFixture(
      root,
      "docs/README.md",
      "The v2.9.5 source candidate is not published. v2.9.1 remains the latest verified release.",
    );
    writeFixture(
      root,
      "docs/plans/archive/history.md",
      "The v2.9.5 source candidate was not published at this point.",
    );

    assert.equal(hasStaleV3ReleaseTruth(root, "docs/README.md"), true);
    assert.equal(hasStaleV3ReleaseTruth(root, "docs/plans/archive/history.md"), false);
  });
});

test("v3 plan rejects duplicate, unknown, and count drift while the live plan passes", () => {
  withFixture((root) => {
    writeFixture(
      root,
      "docs/plans/v3/idea-index.md",
      [
        "| A01 | First | 5 | 3 | Now | Required. |",
        "| A01 | Duplicate | 5 | 3 | Now | Invalid. |",
        "| A02 | Unknown horizon | 3 | 3 | Experimental | Invalid. |",
      ].join("\n"),
    );
    writeFixture(
      root,
      "docs/plans/v3/master-exec-plan.md",
      [
        "all 0 `Now` ideas; all 0 `Next` ideas; all 0 `Later` and 0 `Moonshot` ideas",
        "### V3.0 Accepted",
        "| 1 | A01, A01, Z99 |",
        "### V3.x Accepted After V3.0",
        "### Explicitly Deferred Beyond V3",
        "- `Later`:",
        "- `Moonshot`:",
        "- Promotion trigger: evidence.",
      ].join("\n"),
    );

    const violations = collectV3IdeaDispositionViolations(root);
    for (const expected of [
      "v3 idea index contains duplicate ID: A01",
      "v3 idea index has unknown horizon for A02: Experimental",
      "v3 idea disposition contains duplicate ID: A01",
      "v3 idea disposition contains unknown ID: Z99",
      "v3 scope count for Now must be 1",
    ]) {
      assert.ok(violations.includes(expected), violations.join("\n"));
    }
  });

  assert.deepEqual(collectV3PlanViolations(resolve(".")), []);
});
