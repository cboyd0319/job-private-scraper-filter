import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

const ideaIndexPath = "docs/plans/v3/idea-index.md";
const masterPlanPath = "docs/plans/v3/master-exec-plan.md";
const horizons = ["Now", "Next", "Later", "Moonshot"];
const releaseTruthPaths = new Set([
  "README.md",
  "CHANGELOG.md",
  "docs/README.md",
  "docs/ROADMAP.md",
  "docs/features/capabilities.md",
  "docs/releases/v2.9.5.md",
]);
const staleReleaseTruth =
  /2\.9\.5.{0,120}(?:source candidate|not published|unpublished)|v2\.9\.1.{0,160}(?:current public|latest verified|remains the latest)|not uploaded or published|release execution still requires explicit authorization/i;

function read(root, path) {
  const fullPath = join(root, path);
  return existsSync(fullPath) ? readFileSync(fullPath, "utf8") : "";
}

function tableRows(text) {
  return text.split(/\r?\n/).flatMap((line) => {
    if (!line.trim().startsWith("|")) return [];
    return [line.trim().slice(1, -1).split("|").map((cell) => cell.trim())];
  });
}

function section(text, heading, nextHeading) {
  const start = text.indexOf(heading);
  if (start === -1) return "";
  const end = nextHeading ? text.indexOf(nextHeading, start + heading.length) : -1;
  return text.slice(start, end === -1 ? text.length : end);
}

function ids(text) {
  return text.match(/\b[A-Z]\d{2}\b/g) ?? [];
}

function collectIndex(text, violations) {
  const index = new Map();
  for (const cells of tableRows(text)) {
    if (!/^[A-Z]\d{2}$/.test(cells[0] ?? "")) continue;
    const [id] = cells;
    const horizon = cells[4];
    if (index.has(id)) violations.push(`v3 idea index contains duplicate ID: ${id}`);
    if (!horizons.includes(horizon)) violations.push(`v3 idea index has unknown horizon for ${id}: ${String(horizon)}`);
    index.set(id, horizon);
  }
  return index;
}

function collectDispositions(text) {
  const dispositions = [];
  const acceptedNow = section(text, "### V3.0 Accepted", "### V3.x Accepted After V3.0");
  for (const cells of tableRows(acceptedNow)) {
    if (/^\d+$/.test(cells[0] ?? "")) {
      dispositions.push(...ids(cells[1] ?? "").map((id) => [id, "Now"]));
    }
  }
  const acceptedNext = section(text, "### V3.x Accepted After V3.0", "### Explicitly Deferred Beyond V3");
  for (const cells of tableRows(acceptedNext)) {
    if (/^12[A-Z]$/.test(cells[0] ?? "")) {
      dispositions.push(...ids(cells[2] ?? "").map((id) => [id, "Next"]));
    }
  }
  const deferred = section(text, "### Explicitly Deferred Beyond V3", "### Retired");
  for (const horizon of ["Later", "Moonshot"]) {
    const marker = `- \`${horizon}\`:`;
    const start = deferred.indexOf(marker);
    const tail = start === -1 ? "" : deferred.slice(start + marker.length);
    const end = tail.indexOf("\n- ");
    dispositions.push(...ids(end === -1 ? tail : tail.slice(0, end)).map((id) => [id, horizon]));
  }
  return dispositions;
}

export function collectV3IdeaDispositionViolations(root) {
  const violations = [];
  const indexText = read(root, ideaIndexPath);
  const planText = read(root, masterPlanPath);
  if (!indexText) return [`missing v3 idea index: ${ideaIndexPath}`];
  if (!planText) return [`missing v3 master plan: ${masterPlanPath}`];

  const index = collectIndex(indexText, violations);
  const dispositions = collectDispositions(planText);
  const seen = new Map();
  for (const [id, horizon] of dispositions) {
    if (seen.has(id)) violations.push(`v3 idea disposition contains duplicate ID: ${id}`);
    seen.set(id, horizon);
    if (!index.has(id)) violations.push(`v3 idea disposition contains unknown ID: ${id}`);
    else if (index.get(id) !== horizon) {
      violations.push(`v3 idea ${id} is ${String(index.get(id))} but disposed as ${horizon}`);
    }
  }
  for (const id of index.keys()) {
    if (!seen.has(id)) violations.push(`v3 idea disposition is missing ID: ${id}`);
  }

  const normalized = planText.replace(/\s+/g, " ");
  const claimedCounts = normalized.match(
    /all (\d+) `Now` ideas.*all (\d+) `Next` ideas.*all (\d+) `Later` and (\d+) `Moonshot` ideas/i,
  );
  for (const [position, horizon] of horizons.entries()) {
    const expected = [...index.values()].filter((value) => value === horizon).length;
    if (!claimedCounts || Number(claimedCounts[position + 1]) !== expected) {
      violations.push(`v3 scope count for ${horizon} must be ${expected}`);
    }
  }
  return [...new Set(violations)].sort();
}

export function hasStaleV3ReleaseTruth(root, path) {
  if (!releaseTruthPaths.has(path)) return false;
  return staleReleaseTruth.test(read(root, path).replace(/\s+/g, " "));
}

export function collectV3PlanViolations(root) {
  const violations = collectV3IdeaDispositionViolations(root);
  const planIndex = read(root, "docs/plans/index.json");
  try {
    if (JSON.parse(planIndex).active_plan !== masterPlanPath) {
      violations.push(`docs/plans/index.json must select ${masterPlanPath}`);
    }
  } catch {
    violations.push("docs/plans/index.json must contain a valid active plan pointer");
  }
  if (!read(root, "docs/plans/README.md").includes("[V3 master execution plan](v3/master-exec-plan.md)")) {
    violations.push("docs/plans/README.md must link the v3 master execution plan");
  }
  if (!read(root, "docs/plans/v3/README.md").includes("[V3 Master Execution Plan](master-exec-plan.md)")) {
    violations.push("docs/plans/v3/README.md must link the v3 master execution plan");
  }
  for (const path of releaseTruthPaths) {
    if (hasStaleV3ReleaseTruth(root, path)) violations.push(`replace stale v2.9.5 release truth: ${path}`);
  }
  return [...new Set(violations)].sort();
}
