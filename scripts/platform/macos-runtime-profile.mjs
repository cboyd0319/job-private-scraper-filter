import { readFileSync, readdirSync } from "node:fs";
import { basename, join, relative } from "node:path";

const essentialsProfile = "essentials";
const strongerLocalProfile = "stronger-local";
const strongerLocalNativeCommands = [
  "download_ml_model",
  "cancel_ml_model_download",
  "remove_ml_models",
  "get_ml_status",
  "semantic_match_skills",
  "match_resume_semantic",
];

function getArgValue(args, name) {
  const exactIndex = args.indexOf(name);
  if (exactIndex >= 0) {
    return args[exactIndex + 1];
  }

  const prefixed = args.find((arg) => arg.startsWith(`${name}=`));
  return prefixed ? prefixed.slice(name.length + 1) : undefined;
}

function hasArg(args, name) {
  return args.some((arg) => arg === name || arg.startsWith(`${name}=`));
}

export function normalizeMacosRuntimeProfile(runtimeProfile) {
  const profile = runtimeProfile ?? essentialsProfile;
  if (profile !== essentialsProfile && profile !== strongerLocalProfile) {
    throw new Error(`Unsupported macOS runtime profile: ${profile}. Expected essentials or stronger-local.`);
  }
  return profile;
}

export function getMacosRuntimeProfile(args) {
  const requestedProfile = getArgValue(args, "--runtime-profile");
  if (hasArg(args, "--runtime-profile") && !requestedProfile) {
    throw new Error("macOS runtime profile requires a value.");
  }
  return normalizeMacosRuntimeProfile(requestedProfile);
}

function withoutRuntimeProfileArgs(args) {
  const result = [];

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--runtime-profile") {
      index += 1;
    } else if (!arg.startsWith("--runtime-profile=")) {
      result.push(arg);
    }
  }

  return result;
}

function hasFeatureActivation(args, tokenMatches) {
  if (args.includes("--all-features")) {
    return true;
  }

  const valueHasFeature = (value) =>
    value.split(/[\s,]+/).some(tokenMatches);

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg.startsWith("--features=") || arg.startsWith("-f=") || arg.startsWith("-F=")) {
      if (valueHasFeature(arg.slice(arg.indexOf("=") + 1))) {
        return true;
      }
      continue;
    }
    const attachedShortValue =
      arg.match(/^-[dv]*f=?(.+)$/)?.[1] ?? arg.match(/^-[qrv]*F=?(.+)$/)?.[1];
    if (attachedShortValue) {
      if (valueHasFeature(attachedShortValue)) {
        return true;
      }
      continue;
    }
    if (arg !== "--features" && arg !== "-f" && arg !== "-F") {
      continue;
    }

    for (let valueIndex = index + 1; valueIndex < args.length; valueIndex += 1) {
      const value = args[valueIndex];
      if (value.startsWith("-")) {
        break;
      }
      if (valueHasFeature(value)) {
        return true;
      }
    }
  }

  return false;
}

function hasAnyEmbeddedMlActivation(args) {
  return hasFeatureActivation(args, (token) => token.split("/").at(-1) === "embedded-ml");
}

function hasRootEmbeddedMlActivation(args) {
  return hasFeatureActivation(
    args,
    (token) => token === "embedded-ml" || token === "jobsentinel/embedded-ml",
  );
}

export function macosRuntimeProfileTauriArgs(args) {
  const runtimeProfile = getMacosRuntimeProfile(args);
  const tauriArgs = withoutRuntimeProfileArgs(args);

  if (runtimeProfile === essentialsProfile && hasAnyEmbeddedMlActivation(tauriArgs)) {
    throw new Error("Essentials macOS builds cannot enable embedded-ml. Use --runtime-profile stronger-local.");
  }
  if (runtimeProfile === strongerLocalProfile && !hasRootEmbeddedMlActivation(tauriArgs)) {
    tauriArgs.push("--features", "embedded-ml");
  }

  return tauriArgs;
}

export function runtimeProfileArtifactViolations(dmgPath, runtimeProfile) {
  const profile = normalizeMacosRuntimeProfile(runtimeProfile);
  if (profile === strongerLocalProfile && !basename(dmgPath).includes("_stronger-local_")) {
    return ["stronger-local artifact name must contain _stronger-local_"];
  }
  return [];
}

export function assertMacosRuntimeProfileArtifact(dmgPath, runtimeProfile) {
  const violations = runtimeProfileArtifactViolations(dmgPath, runtimeProfile);
  if (violations.length > 0) {
    throw new Error(`Runtime profile artifact check failed:\n- ${violations.join("\n- ")}`);
  }
}

export function runtimeProfileCommandViolations(binaryContents, runtimeProfile) {
  const profile = normalizeMacosRuntimeProfile(runtimeProfile);
  const commands = strongerLocalNativeCommands.filter((command) => binaryContents.includes(command));

  if (profile === strongerLocalProfile) {
    return strongerLocalNativeCommands
      .filter((command) => !commands.includes(command))
      .map((command) => `stronger-local native command missing: ${command}`);
  }

  return commands.map(
    (command) => `Essentials binary unexpectedly contains stronger-local native command: ${command}`,
  );
}

export function modelPayloadFiles(appPath) {
  const payloadFiles = [];

  function visit(path) {
    for (const entry of readdirSync(path, { withFileTypes: true })) {
      const entryPath = join(path, entry.name);
      if (entry.isDirectory()) {
        visit(entryPath);
      } else if (entry.name.toLowerCase().endsWith(".safetensors")) {
        payloadFiles.push(relative(appPath, entryPath));
      }
    }
  }

  visit(appPath);
  return payloadFiles.sort();
}

export function verifyMacosRuntimeProfile(appPath, executable, runtimeProfile) {
  const profile = normalizeMacosRuntimeProfile(runtimeProfile);
  console.log(`Scanning native commands: ${executable}`);
  const commandViolations = runtimeProfileCommandViolations(readFileSync(executable), profile);
  if (commandViolations.length > 0) {
    throw new Error(`Runtime profile check failed:\n- ${commandViolations.join("\n- ")}`);
  }

  const payloadFiles = modelPayloadFiles(appPath);
  if (payloadFiles.length > 0) {
    throw new Error(`Bundled model payload files are forbidden:\n- ${payloadFiles.join("\n- ")}`);
  }

  console.log(`Runtime profile verified: ${profile}.`);
  console.log("Model payload absence verified: no .safetensors files in app bundle.");
}
