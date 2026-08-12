<!-- Defines the maintained release, signing, publication, and rollback contract. -->

# Release Contract

Release execution is separate from normal development and requires explicit user
authority. GitHub-hosted release, publication, and public-verification workflows
are manual. Their existence does not authorize dispatch, signing, upload,
attestation, or publication.

## Preconditions

Before tagging, building release assets, uploading, or publishing:

1. Close or explicitly rescope the active work in `docs/harness/current-status.md` and
   `scripts/harness/state/feature-list.json`.
2. Confirm one source revision and exact `X.Y.Z` version across Cargo, npm,
   Tauri, lockfiles, changelog, and `docs/releases/vX.Y.Z.md`.
3. Obtain explicit authority for every external mutation: tag push, release
   creation, asset deletion, upload, publication, signing, or notarization.
4. Use only local or explicitly authorized hosted or native runners. Workflow
   configuration is not evidence; retain the actual run and artifact evidence.

## Signed Pack Releases

Pack signing is local-only. Production private keys stay concealed in the
dedicated 1Password Developer Environment and must never be copied into the
repository, a command argument, a key file, or a dotenv file. The compiled
public identities and exact authority ceilings in
`crates/jobsentinel-application/src/pack_runtime/trust.rs` are authoritative.

| Pack identity | Publisher key ID | Concealed 1Password variable |
| --- | --- | --- |
| Source | `jobsentinel-source-publisher-v1` | `JOBSENTINEL_SOURCE_PRIVATE_KEY_PKCS8_B64` |
| Skill | `jobsentinel-skill-publisher-v1` | `JOBSENTINEL_SKILL_PRIVATE_KEY_PKCS8_B64` |
| Evaluation | `jobsentinel-evaluation-publisher-v1` | `JOBSENTINEL_EVALUATION_PRIVATE_KEY_PKCS8_B64` |
| Evidence Reviewer | `jobsentinel-evidence-reviewer-publisher-v1` | `JOBSENTINEL_EVIDENCE_REVIEWER_PRIVATE_KEY_PKCS8_B64` |
| Packet Builder | `jobsentinel-packet-builder-publisher-v1` | `JOBSENTINEL_PACKET_BUILDER_PRIVATE_KEY_PKCS8_B64` |

Set `JOBSENTINEL_1PASSWORD_ENVIRONMENT_ID` only in the operator's local shell.
Then sign an exact release JSON file with the matching ID, concealed-variable
name, and raw public-key hex from the compiled registry:

```bash
op run --environment "$JOBSENTINEL_1PASSWORD_ENVIRONMENT_ID" -- \
  npm run release:sign-pack -- \
  --release release.json \
  --publisher-key-id jobsentinel-source-publisher-v1 \
  --expected-public-key-hex <approved-raw-public-key-hex> \
  --key-env JOBSENTINEL_SOURCE_PRIVATE_KEY_PKCS8_B64 \
  --out source-release.jspack
```

PowerShell uses the same child-process route with
`--environment $env:JOBSENTINEL_1PASSWORD_ENVIRONMENT_ID`. The signer preserves
the exact UTF-8 release bytes, checks that the concealed private key derives the
approved public key, self-verifies the Ed25519 signature, and exclusively
creates a private-mode output file. It never overwrites an existing envelope.

Rotation always creates a new publisher key ID and receives a separate ceiling
review before registry inclusion. Omitting a publisher from the compiled
registry means revoked: startup reconciliation quarantines its active and
rollback artifacts as `trust_revoked`. Do not delete an old public identity
until the intended revocation or rotation boundary is approved. A signer run
does not install, activate, publish, upload, or otherwise trust the result.

## Local Release Gate

Run from the repository root:

```bash
./init.sh
npm run release:check-version -- vX.Y.Z
npm run release:check-env
npm run release:readiness -- --version vX.Y.Z
npm run release:check-deps
npm run verify:full
npm run lint:sqlx
npm run test:e2e:all
node scripts/dev/run-cargo.mjs clippy --workspace --all-features -- -D warnings
node scripts/dev/run-cargo.mjs test --workspace --all-features
```

Online advisory checks are required when network access is available:

```bash
npm audit --audit-level=moderate
cargo deny check advisories bans licenses sources
```

Record an unavailable host, tool, credential, or network check as a gap. Do not
convert a skipped gate into a pass.

## Native Platform Assets

Build every advertised platform from the same source revision.

### macOS

JobSentinel currently has no Apple Developer Account.
A zero-friction public macOS DMG cannot be produced under that constraint. A
public no-account DMG must be visibly labeled `_no-account_`, use a universal
`x86_64,arm64` binary,
include its `.dmg.sha256`, and include the generated macOS SBOM and manifest. It
is not Developer ID signed, notarized, stapled, or Gatekeeper-ready.

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
JOBSENTINEL_MACOS_NO_ACCOUNT=true npm run tauri:build:macos -- --target universal-apple-darwin
npm run tauri:verify:macos -- \
  --dmg target/universal-apple-darwin/release/bundle/dmg/JobSentinel_X.Y.Z_no-account_universal.dmg \
  --expected-architectures x86_64,arm64 \
  --expected-bundle-id com.jobsentinel.main \
  --expected-product-name JobSentinel \
  --expected-version X.Y.Z \
  --expected-icon-file icon.icns \
  --expected-minimum-system-version 13.0 \
  --launch-smoke \
  --install-smoke \
  --require-checksum
```

Developer ID mode requires separately authorized signing and notarization
credentials. Keep certificate, password, App Store Connect key, and temporary
keychain material outside the repository. Require Gatekeeper verification before
calling that artifact ready.

Do not publish a Mac package without its checksum.
Do not publish a macOS package as zero-friction or Gatekeeper-ready until
`--require-gatekeeper` passes.

### Windows

Build on Windows 11 or an explicitly selected Windows VM:

```powershell
npm run tauri build
Get-AuthenticodeSignature target/release/bundle/msi/JobSentinel_*.msi
Get-AuthenticodeSignature target/release/bundle/nsis/JobSentinel_*.exe
```

Public Windows MSI and NSIS setup upload is signed with locally supplied Windows
signing secrets or visibly renamed with `_unsigned`. Both paths require matching
SHA-256 sidecars. Signing material must remain outside the repository and must be
removed from temporary stores after verification.

Windows signing secrets stay outside the repository and require explicit use.

### Linux

Build on the supported Linux target:

```bash
APPIMAGE_EXTRACT_AND_RUN=1 npx --no-install tauri build --target x86_64-unknown-linux-gnu
file target/x86_64-unknown-linux-gnu/release/bundle/appimage/*.AppImage
dpkg-deb --info target/x86_64-unknown-linux-gnu/release/bundle/deb/*.deb
dpkg-deb --contents target/x86_64-unknown-linux-gnu/release/bundle/deb/*.deb >/dev/null
```

Public Linux upload is blocked unless the advertised set contains exactly one
non-empty AppImage and one non-empty Debian package, versioned filenames, and
matching SHA-256 sidecars.

## SBOM And Staging

Generate release assets outside tracked source paths:

```bash
npm run release:skills -- --version X.Y.Z --out-dir release-assets/public
npm run release:sbom -- \
  --platform <windows|macos|linux> \
  --version X.Y.Z \
  --out-dir release-assets/public \
  --checksums-out release-assets/attestation-subjects.sha256 \
  --require-artifacts
```

Inspect the exact staged asset list. Reject duplicate installers, stale versions,
missing checksums, mismatched digests, incomplete SBOM manifests, or artifacts
from a different revision.

Upload `JobSentinel-X.Y.Z-agent-skills.tar.gz`,
`JobSentinel-X.Y.Z-agent-skills.zip`, and both `.sha256` sidecars. Upload only
after the local archive verifier passes.

## Publication And Public Verification

Tagging, pushing, creating a GitHub Release, deleting assets, uploading, and
publishing are external mutations and need explicit authority at execution time.
The authorized operator may use `gh` after reviewing the exact tag and asset
list. Do not dispatch a workflow as a release shortcut without that authority.

After publication, verify the downloadable assets locally:

```bash
npm run release:verify:public -- --tag vX.Y.Z --platforms windows,macos,linux
npm run tauri:verify:macos:latest -- --tag vX.Y.Z --no-require-supply-chain
```

The no-supply-chain flag is required for locally built assets because they have
no GitHub Actions provenance. Never claim hosted provenance, SLSA attestation,
or GitHub-generated SBOM attestation for local builds.

## Rollback

- Source changes roll back through a reviewed Git revert.
- Published assets are replaced only with explicit authority and the same
  version/revision contract.
- Database migrations never roll back by deleting user data. Use a forward
  repair or a verified backup restore.
- Preserve failed local artifacts outside tracked paths until the failure is
  understood, then remove them before completion.
