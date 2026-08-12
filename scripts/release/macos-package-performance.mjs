import { spawnSync } from "node:child_process";

function runCapture(command, args) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  if (result.status !== 0) {
    throw new Error(`${command} failed with status ${result.status}: ${result.stderr || result.stdout}`);
  }
  return result.stdout;
}

export function parseLsappinfoCoalition(output, mainPid) {
  const mainBlock = output
    .split(/(?=^\s*\d+\) ")/m)
    .find((block) => new RegExp(`^\\s*pid = ${mainPid}(?:\\s|$)`, "m").test(block));
  const asn = mainBlock?.match(/\b(ASN:0x[0-9a-f]+-0x[0-9a-f]+:)/i)?.[1];
  const members = mainBlock
    ?.match(/^\s*coalition:\s+\d+\s+\{\s*([^}]*)\}/m)?.[1]
    ?.match(/\d+/g)
    ?.map(Number) ?? [];
  const pids = [...new Set([mainPid, ...members])]
    .filter((pid) => Number.isSafeInteger(pid) && pid > 0)
    .sort((left, right) => left - right);
  return { asn, pids };
}

export function parsePsRssSample(output, selectedPids) {
  const selected = new Set(selectedPids);
  const observed = new Set();
  let rssKib = 0;

  for (const line of output.split("\n")) {
    const match = line.trim().match(/^(\d+)\s+(\d+)$/);
    if (!match) continue;
    const pid = Number.parseInt(match[1], 10);
    const rss = Number.parseInt(match[2], 10);
    if (!selected.has(pid) || observed.has(pid) || !Number.isSafeInteger(rss)) continue;
    observed.add(pid);
    rssKib += rss;
  }

  return { processCount: observed.size, rssKib };
}

export function sampleMacosAppRss(mainPid) {
  let coalition = { asn: undefined, pids: [mainPid] };
  try {
    coalition = parseLsappinfoCoalition(runCapture("lsappinfo", ["list"]), mainPid);
  } catch {
    // Main-process RSS remains measurable if LaunchServices metadata is unavailable.
  }
  return {
    ...coalition,
    ...parsePsRssSample(
      runCapture("ps", ["-o", "pid=,rss=", "-p", coalition.pids.join(",")]),
      coalition.pids,
    ),
  };
}

export function createLaunchRssMetrics() {
  return {
    lastSampleAt: undefined,
    maxProcessCount: 0,
    maxSampleGapMs: 0,
    observedMaxRssKib: 0,
    sampleCount: 0,
  };
}

export function recordLaunchRssSample(metrics, sample, sampledAt, startedAt) {
  if (sample.processCount > 0 && sample.rssKib > 0) {
    metrics.maxSampleGapMs = Math.max(
      metrics.maxSampleGapMs,
      Math.ceil(sampledAt - (metrics.lastSampleAt ?? startedAt)),
    );
    metrics.lastSampleAt = sampledAt;
    metrics.sampleCount += 1;
  }
  metrics.maxProcessCount = Math.max(metrics.maxProcessCount, sample.processCount);
  metrics.observedMaxRssKib = Math.max(metrics.observedMaxRssKib, sample.rssKib);
}

export function buildMacosCoalitionKillArgs(asn, hard) {
  return ["kill", "-coalition", ...(hard ? ["-hard"] : []), asn];
}

function readProcessCommands(pids) {
  if (pids.length === 0) return new Map();
  const result = spawnSync("ps", ["-o", "pid=,comm=", "-p", pids.join(",")], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  if (result.status === 1) return new Map();
  if (result.status !== 0) {
    throw new Error(`ps failed with status ${result.status}: ${result.stderr || result.stdout}`);
  }
  return new Map(result.stdout.split("\n").flatMap((line) => {
    const match = line.match(/^\s*(\d+)\s+(.+?)\s*$/);
    return match ? [[Number.parseInt(match[1], 10), match[2]]] : [];
  }));
}

function matchingProcessPids(identities) {
  const current = readProcessCommands([...identities.keys()]);
  return [...identities].flatMap(([pid, command]) => current.get(pid) === command ? [pid] : []);
}

async function waitForProcessExit(identities, timeoutMs) {
  const deadline = performance.now() + timeoutMs;
  while (matchingProcessPids(identities).length > 0 && performance.now() < deadline) {
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  return matchingProcessPids(identities);
}

function exactCommandPattern(command) {
  return `^${command.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}$`;
}

function signalApp(asn, mainCommand, hard) {
  const command = asn ? "lsappinfo" : "pkill";
  const args = asn
    ? buildMacosCoalitionKillArgs(asn, hard)
    : [hard ? "-KILL" : "-TERM", "-f", exactCommandPattern(mainCommand)];
  const result = spawnSync(command, args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
  if (result.status !== 0 && result.status !== 1) {
    throw new Error(`${command} failed with status ${result.status}: ${result.stderr || result.stdout}`);
  }
}

export async function stopMacosAppCoalition(mainPid) {
  let coalition = { asn: undefined, pids: [mainPid] };
  try {
    coalition = parseLsappinfoCoalition(runCapture("lsappinfo", ["list"]), mainPid);
  } catch {
    // The exact main executable remains a safe fallback shutdown identity.
  }
  const identities = readProcessCommands(coalition.pids);
  const mainCommand = identities.get(mainPid);
  if (!mainCommand && !coalition.asn) return;

  signalApp(coalition.asn, mainCommand, false);
  let remaining = await waitForProcessExit(identities, 2000);
  if (remaining.length === 0) return;
  signalApp(coalition.asn, mainCommand, true);
  remaining = await waitForProcessExit(identities, 1000);
  if (remaining.length > 0) {
    throw new Error(`Launch smoke could not stop scoped app processes: ${remaining.join(", ")}`);
  }
}
