/** Re-verifies and displays an active static skill as inert local text. */

import { useEffect, useRef, useState } from "react";
import { invoke } from "../../../platform/tauri";
import { Button } from "../../../ui/Button";
import type { PackManagementReview } from "../../../shared/packManagementProjection";

type StaticSkill = {
  skillName: string;
  skillMd: string;
  resources: Array<{ path: string; content: string }>;
  handoff: {
    taskKind: "evidence_review" | "draft_packet";
    label: string;
  } | null;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

const RESOURCE_PATH =
  /^(assets|references)\/[A-Za-z0-9_-]+(?:\/[A-Za-z0-9_-]+)*(?:\.[A-Za-z0-9_-]+)*\.(csv|json|md|txt|yaml|yml)$/;
const WINDOWS_RESERVED = /^(con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)/i;

function validResourcePath(path: string): boolean {
  return (
    path.length <= 256 &&
    path.length > 0 &&
    path === path.trim() &&
    /^[\x20-\x7e]+$/.test(path) &&
    !path.includes("//") &&
    RESOURCE_PATH.test(path) &&
    path
      .split("/")
      .every(
        (part) =>
          !part.startsWith(".") &&
          !part.endsWith(".") &&
          !WINDOWS_RESERVED.test(part),
      )
  );
}

function utf8Bytes(value: string): number {
  return new TextEncoder().encode(value).byteLength;
}

function parseStaticSkill(value: unknown): StaticSkill {
  if (!isRecord(value)) throw new Error("invalid static skill");
  const resources = value.resources;
  const handoff = value.handoff;
  const paths = Array.isArray(resources)
    ? resources.map((resource) =>
        isRecord(resource) && typeof resource.path === "string"
          ? resource.path.toLocaleLowerCase("en-US")
          : "",
      )
    : [];
  if (
    typeof value.skillName !== "string" ||
    !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(value.skillName) ||
    value.skillName.length > 64 ||
    typeof value.skillMd !== "string" ||
    value.skillMd.length === 0 ||
    utf8Bytes(value.skillMd) > 256 * 1024 ||
    value.skillMd.split("\n").length > 500 ||
    !Array.isArray(resources) ||
    resources.length > 64 ||
    resources.some(
      (resource) =>
        !isRecord(resource) ||
        typeof resource.path !== "string" ||
        !validResourcePath(resource.path) ||
        typeof resource.content !== "string" ||
        resource.content.length === 0 ||
        utf8Bytes(resource.content) > 512 * 1024,
    ) ||
    new Set(paths).size !== paths.length ||
    utf8Bytes(value.skillMd) +
      resources.reduce(
        (total, resource) =>
          total +
          (isRecord(resource) && typeof resource.content === "string"
            ? utf8Bytes(resource.content)
            : 0),
        0,
      ) >
      4 * 1024 * 1024 ||
    (handoff !== null &&
      (!isRecord(handoff) ||
        !["evidence_review", "draft_packet"].includes(
          String(handoff.taskKind),
        ) ||
        typeof handoff.label !== "string" ||
        handoff.label.length === 0 ||
        handoff.label.length > 120))
  )
    throw new Error("invalid static skill");
  return value as unknown as StaticSkill;
}

export function PackStaticSkillReview({
  pack,
}: {
  pack: PackManagementReview;
}) {
  const [skill, setSkill] = useState<StaticSkill | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(false);
  const interaction = useRef(0);

  useEffect(() => {
    interaction.current += 1;
    setSkill(null);
    setLoading(false);
    setError(false);
  }, [pack.generation, pack.state]);

  const open = async () => {
    const interactionId = ++interaction.current;
    setLoading(true);
    setError(false);
    setSkill(null);
    try {
      const opened = parseStaticSkill(
        await invoke("open_static_skill", {
          publisherKeyId: pack.publisherKeyId,
          packId: pack.packId,
          expectedGeneration: pack.generation,
        }),
      );
      if (interaction.current === interactionId) setSkill(opened);
    } catch {
      if (interaction.current === interactionId) setError(true);
    } finally {
      if (interaction.current === interactionId) setLoading(false);
    }
  };

  return (
    <section className="mt-4 border-t border-surface-200 pt-4 dark:border-surface-700">
      <Button
        size="sm"
        variant="secondary"
        loading={loading}
        loadingText="Verifying..."
        onClick={() => void open()}
      >
        Open static skill
      </Button>
      {error ? (
        <p role="alert" className="mt-3 text-sm text-danger">
          Static skill could not be verified. Refresh Packs and try again.
        </p>
      ) : null}
      {skill ? (
        <div className="mt-3 space-y-3">
          <div>
            <h4 className="font-medium text-surface-900 dark:text-surface-100">
              {skill.skillName}
            </h4>
            <pre className="mt-2 max-h-80 overflow-auto whitespace-pre-wrap rounded-md bg-surface-50 p-3 text-sm text-surface-700 dark:bg-surface-800 dark:text-surface-200">
              {skill.skillMd}
            </pre>
          </div>
          {skill.resources.map((resource) => (
            <details key={resource.path}>
              <summary className="cursor-pointer text-sm font-medium text-sentinel-700 dark:text-sentinel-300">
                {resource.path}
              </summary>
              <pre className="mt-2 max-h-64 overflow-auto whitespace-pre-wrap rounded-md bg-surface-50 p-3 text-sm text-surface-700 dark:bg-surface-800 dark:text-surface-200">
                {resource.content}
              </pre>
            </details>
          ))}
          {skill.handoff ? (
            <p className="rounded-md bg-surface-50 p-3 text-sm text-surface-700 dark:bg-surface-800 dark:text-surface-200">
              Suggested next step: {skill.handoff.label}.{" "}
              {skill.handoff.taskKind === "evidence_review"
                ? "Check a saved match's Match Debugger for an eligible Evidence Reviewer pack."
                : "Check a reviewed opportunity case for an eligible Packet Builder pack."}
            </p>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}
