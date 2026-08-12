// Verifies deterministic Ed25519 pack-envelope signing and secret-safe CLI behavior.

import assert from "node:assert/strict";
import { createPrivateKey, createPublicKey, sign } from "node:crypto";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import test from "node:test";

import {
  MAX_SIGNED_PACK_BYTES,
  createSignedPackEnvelope,
  parseArgs,
} from "../../release/sign-pack-envelope.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const scriptPath = join(repoRoot, "scripts", "release", "sign-pack-envelope.mjs");
const publisherKeyId = "jobsentinel-test-publisher-v1";
const privateKeyDer = Buffer.concat([
  Buffer.from("302e020100300506032b657004220420", "hex"),
  Buffer.alloc(32, 7),
]);
const privateKeyBase64 = privateKeyDer.toString("base64");
const privateKey = createPrivateKey({ key: privateKeyDer, format: "der", type: "pkcs8" });
const publicKeyHex = createPublicKey(privateKey)
  .export({ format: "der", type: "spki" })
  .subarray(-32)
  .toString("hex");

test("createSignedPackEnvelope preserves exact release bytes and framing", () => {
  const release = Buffer.from('{\r\n  "release_id": "fixed"\r\n}', "utf8");
  const signingBytes = Buffer.concat([
    Buffer.from("jobsentinel.pack-envelope.v1\0", "utf8"),
    lengthBytes(Buffer.byteLength(publisherKeyId)),
    Buffer.from(publisherKeyId, "utf8"),
    lengthBytes(release.length),
    release,
  ]);
  const expectedSignature = sign(null, signingBytes, privateKey).toString("hex");

  const envelope = createSignedPackEnvelope({
    releaseBytes: release,
    publisherKeyId,
    expectedPublicKeyHex: publicKeyHex,
    privateKeyPkcs8Base64: privateKeyBase64,
  });

  assert.deepEqual(JSON.parse(envelope.toString("utf8")), {
    schema: "jobsentinel.v3.signed-pack-envelope.v1",
    publisher_key_id: publisherKeyId,
    signed_release: release.toString("utf8"),
    signature: expectedSignature,
  });
  assert.equal(envelope.includes(Buffer.from("\r\n")), false);
  assert.equal(JSON.parse(envelope).signed_release.includes("\r\n"), true);
});

test("parseArgs accepts exactly one non-argument key source", () => {
  const common = [
    "--release",
    "release.json",
    "--publisher-key-id",
    publisherKeyId,
    "--expected-public-key-hex",
    publicKeyHex,
    "--out",
    "pack.json",
  ];
  assert.deepEqual(parseArgs([...common, "--key-env", "TEST_SIGNING_KEY"]), {
    releasePath: "release.json",
    publisherKeyId,
    expectedPublicKeyHex: publicKeyHex,
    outputPath: "pack.json",
    keyEnvironmentVariable: "TEST_SIGNING_KEY",
    keyFromStdin: false,
  });
  assert.deepEqual(parseArgs([...common, "--key-stdin"]), {
    releasePath: "release.json",
    publisherKeyId,
    expectedPublicKeyHex: publicKeyHex,
    outputPath: "pack.json",
    keyEnvironmentVariable: undefined,
    keyFromStdin: true,
  });
  assert.throws(() => parseArgs([...common, "--key", privateKeyBase64]));
  assert.throws(() => parseArgs([...common, "--key-env", "ONE", "--key-stdin"]));
});

test("CLI signs from environment or stdin without overwriting output", () => {
  const root = mkdtempSync(join(tmpdir(), "jobsentinel-pack-signing-"));
  const releasePath = join(root, "release.json");
  const firstOutput = join(root, "environment.jspack");
  const secondOutput = join(root, "stdin.jspack");
  writeFileSync(releasePath, '{\r\n  "release_id": "fixed"\r\n}', "utf8");
  const common = cliArgs(releasePath);

  const fromEnvironment = spawnSync(
    process.execPath,
    [scriptPath, ...common, "--key-env", "TEST_SIGNING_KEY", "--out", firstOutput],
    {
      env: { ...process.env, TEST_SIGNING_KEY: privateKeyBase64 },
      encoding: "utf8",
    },
  );
  assert.equal(fromEnvironment.status, 0, fromEnvironment.stderr);
  const firstBytes = readFileSync(firstOutput);

  const overwrite = spawnSync(
    process.execPath,
    [scriptPath, ...common, "--key-env", "TEST_SIGNING_KEY", "--out", firstOutput],
    {
      env: { ...process.env, TEST_SIGNING_KEY: privateKeyBase64 },
      encoding: "utf8",
    },
  );
  assert.equal(overwrite.status, 1);
  assert.deepEqual(readFileSync(firstOutput), firstBytes);

  const fromStdin = spawnSync(
    process.execPath,
    [scriptPath, ...common, "--key-stdin", "--out", secondOutput],
    { input: `${privateKeyBase64}\n`, encoding: "utf8" },
  );
  assert.equal(fromStdin.status, 0, fromStdin.stderr);
  assert.deepEqual(readFileSync(secondOutput), firstBytes);
});

test("CLI failures do not disclose keys or release contents", () => {
  const root = mkdtempSync(join(tmpdir(), "jobsentinel-pack-signing-secret-"));
  const releasePath = join(root, "release.json");
  const outputPath = join(root, "pack.jspack");
  const releaseMarker = "SENSITIVE_RELEASE_MARKER";
  const keyMarker = "SENSITIVE_KEY_MARKER";
  writeFileSync(releasePath, releaseMarker, "utf8");

  const result = spawnSync(
    process.execPath,
    [
      scriptPath,
      ...cliArgs(releasePath),
      "--key-env",
      "TEST_SIGNING_KEY",
      "--out",
      outputPath,
    ],
    { env: { ...process.env, TEST_SIGNING_KEY: keyMarker }, encoding: "utf8" },
  );

  assert.equal(result.status, 1);
  assert.equal(`${result.stdout}${result.stderr}`.includes(releaseMarker), false);
  assert.equal(`${result.stdout}${result.stderr}`.includes(keyMarker), false);
  assert.throws(() => readFileSync(outputPath));
});

test("signer rejects invalid identity and byte boundaries before output", () => {
  const base = {
    releaseBytes: Buffer.from("{}"),
    publisherKeyId,
    expectedPublicKeyHex: publicKeyHex,
    privateKeyPkcs8Base64: privateKeyBase64,
  };

  assert.throws(() =>
    createSignedPackEnvelope({ ...base, publisherKeyId: "x".repeat(129) }),
  );
  assert.throws(() =>
    createSignedPackEnvelope({
      ...base,
      expectedPublicKeyHex: publicKeyHex.toUpperCase(),
    }),
  );
  assert.throws(() =>
    createSignedPackEnvelope({
      ...base,
      releaseBytes: Buffer.from([0xff]),
    }),
  );
  assert.throws(() =>
    createSignedPackEnvelope({
      ...base,
      releaseBytes: Buffer.concat([Buffer.from([0xef, 0xbb, 0xbf]), Buffer.from("{}")]),
    }),
  );
  assert.throws(() =>
    createSignedPackEnvelope({
      ...base,
      releaseBytes: Buffer.alloc(MAX_SIGNED_PACK_BYTES, 0x61),
    }),
  );
});

function cliArgs(releasePath) {
  return [
    "--release",
    releasePath,
    "--publisher-key-id",
    publisherKeyId,
    "--expected-public-key-hex",
    publicKeyHex,
  ];
}

function lengthBytes(value) {
  const bytes = Buffer.alloc(8);
  bytes.writeBigUInt64LE(BigInt(value));
  return bytes;
}
