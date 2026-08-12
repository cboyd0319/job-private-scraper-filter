#!/usr/bin/env node
// Creates a signed pack envelope from exact release bytes and an injected Ed25519 private key.

import {
  createPrivateKey,
  createPublicKey,
  sign,
  timingSafeEqual,
  verify,
} from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { TextDecoder } from "node:util";

export const MAX_SIGNED_PACK_BYTES = 4 * 1024 * 1024;

const envelopeSchema = "jobsentinel.v3.signed-pack-envelope.v1";
const signingDomain = Buffer.from("jobsentinel.pack-envelope.v1\0", "utf8");
const publicKeyPrefix = Buffer.from("302a300506032b6570032100", "hex");
const identifierPattern = /^[A-Za-z0-9._:-]{1,128}$/;
const publicKeyPattern = /^[a-f0-9]{64}$/;
const environmentNamePattern = /^[A-Za-z_][A-Za-z0-9_]*$/;
const valueOptions = new Map([
  ["--release", "releasePath"],
  ["--publisher-key-id", "publisherKeyId"],
  ["--expected-public-key-hex", "expectedPublicKeyHex"],
  ["--out", "outputPath"],
  ["--key-env", "keyEnvironmentVariable"],
]);

export function createSignedPackEnvelope({
  releaseBytes,
  publisherKeyId,
  expectedPublicKeyHex,
  privateKeyPkcs8Base64,
}) {
  if (!Buffer.isBuffer(releaseBytes) || releaseBytes.length > MAX_SIGNED_PACK_BYTES) {
    throw new Error("Invalid release bytes");
  }
  if (!identifierPattern.test(publisherKeyId)) {
    throw new Error("Invalid publisher key ID");
  }
  if (!publicKeyPattern.test(expectedPublicKeyHex)) {
    throw new Error("Invalid expected public key");
  }

  const releaseText = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(
    releaseBytes,
  );
  if (!Buffer.from(releaseText, "utf8").equals(releaseBytes)) {
    throw new Error("Release bytes are not canonical UTF-8");
  }
  JSON.parse(releaseText);
  const privateKeyBytes = decodeBase64(privateKeyPkcs8Base64);
  try {
    const privateKey = createPrivateKey({ key: privateKeyBytes, format: "der", type: "pkcs8" });
    if (privateKey.asymmetricKeyType !== "ed25519") {
      throw new Error("Invalid signing key type");
    }
    const publicKey = createPublicKey(privateKey);
    const publicKeyDer = publicKey.export({ format: "der", type: "spki" });
    if (
      publicKeyDer.length !== publicKeyPrefix.length + 32 ||
      !publicKeyDer.subarray(0, publicKeyPrefix.length).equals(publicKeyPrefix)
    ) {
      throw new Error("Invalid signing public key");
    }
    const derivedPublicKey = publicKeyDer.subarray(publicKeyPrefix.length);
    const expectedPublicKey = Buffer.from(expectedPublicKeyHex, "hex");
    if (!timingSafeEqual(derivedPublicKey, expectedPublicKey)) {
      throw new Error("Signing identity mismatch");
    }

    const signingBytes = Buffer.concat([
      signingDomain,
      lengthBytes(Buffer.byteLength(publisherKeyId)),
      Buffer.from(publisherKeyId, "utf8"),
      lengthBytes(releaseBytes.length),
      releaseBytes,
    ]);
    const signature = sign(null, signingBytes, privateKey);
    if (signature.length !== 64 || !verify(null, signingBytes, publicKey, signature)) {
      throw new Error("Signing self-verification failed");
    }

    const envelope = Buffer.from(
      JSON.stringify({
        schema: envelopeSchema,
        publisher_key_id: publisherKeyId,
        signed_release: releaseText,
        signature: signature.toString("hex"),
      }),
      "utf8",
    );
    if (envelope.length > MAX_SIGNED_PACK_BYTES) {
      throw new Error("Signed pack exceeds the byte limit");
    }
    return envelope;
  } finally {
    privateKeyBytes.fill(0);
  }
}

export function parseArgs(args) {
  const values = {};
  const seen = new Set();
  let keyFromStdin = false;

  for (let index = 0; index < args.length; index += 1) {
    const argument = args[index];
    if (argument === "--key-stdin") {
      if (seen.has(argument)) throw new Error("Invalid signing arguments");
      seen.add(argument);
      keyFromStdin = true;
      continue;
    }

    const separator = argument.indexOf("=");
    const option = separator === -1 ? argument : argument.slice(0, separator);
    const property = valueOptions.get(option);
    if (!property || seen.has(option)) throw new Error("Invalid signing arguments");
    seen.add(option);
    const value = separator === -1 ? args[++index] : argument.slice(separator + 1);
    if (!value || value.startsWith("--")) throw new Error("Invalid signing arguments");
    values[property] = value;
  }

  const required = ["releasePath", "publisherKeyId", "expectedPublicKeyHex", "outputPath"];
  if (required.some((property) => !values[property])) {
    throw new Error("Invalid signing arguments");
  }
  const keyEnvironmentVariable = values.keyEnvironmentVariable;
  if (
    Boolean(keyEnvironmentVariable) === keyFromStdin ||
    (keyEnvironmentVariable && !environmentNamePattern.test(keyEnvironmentVariable))
  ) {
    throw new Error("Invalid signing key source");
  }

  return {
    releasePath: values.releasePath,
    publisherKeyId: values.publisherKeyId,
    expectedPublicKeyHex: values.expectedPublicKeyHex,
    outputPath: values.outputPath,
    keyEnvironmentVariable,
    keyFromStdin,
  };
}

export function main({ args = process.argv.slice(2), environment = process.env } = {}) {
  const options = parseArgs(args);
  const privateKeyPkcs8Base64 = options.keyFromStdin
    ? readFileSync(0, "utf8").trim()
    : environment[options.keyEnvironmentVariable];
  if (!privateKeyPkcs8Base64) throw new Error("Signing key is unavailable");
  if (!options.keyFromStdin) delete environment[options.keyEnvironmentVariable];

  const envelope = createSignedPackEnvelope({
    releaseBytes: readFileSync(options.releasePath),
    publisherKeyId: options.publisherKeyId,
    expectedPublicKeyHex: options.expectedPublicKeyHex,
    privateKeyPkcs8Base64,
  });
  writeFileSync(options.outputPath, envelope, { flag: "wx", mode: 0o600 });
}

function decodeBase64(value) {
  const normalized = typeof value === "string" ? value.trim() : "";
  if (
    normalized.length === 0 ||
    !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(normalized)
  ) {
    throw new Error("Invalid signing key encoding");
  }
  const bytes = Buffer.from(normalized, "base64");
  if (bytes.toString("base64") !== normalized) {
    bytes.fill(0);
    throw new Error("Invalid signing key encoding");
  }
  return bytes;
}

function lengthBytes(value) {
  const bytes = Buffer.alloc(8);
  bytes.writeBigUInt64LE(BigInt(value));
  return bytes;
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    main();
  } catch {
    console.error("Pack signing failed.");
    process.exitCode = 1;
  }
}
