/** Verifies bookmarklet authorization rules in the repository bloat check. */

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";
import { checkRepoBloat } from "../../checks/repo-bloat.mjs";

function writeFixtureFile(root, path, content = "") {
  const fullPath = join(root, path);
  mkdirSync(dirname(fullPath), { recursive: true });
  writeFileSync(fullPath, content, "utf8");
}

function withGitFixture(callback) {
  const root = mkdtempSync(join(tmpdir(), "jobsentinel-repo-bloat-bookmarklet-auth-"));

  try {
    execFileSync("git", ["init", "--quiet"], { cwd: root });
    callback(root);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

test("checkRepoBloat rejects unauthenticated bookmarklet imports", () => {
  withGitFixture((root) => {
    writeFixtureFile(root, "package.json", "{}\n");
    writeFixtureFile(
      root,
      "crates/jobsentinel-assistance/src/bookmarklet/server.rs",
      [
        'if request.starts_with("POST /api/bookmarklet/import") {',
        "    handle_import_request(&request, database).await",
        "} else if request.starts_with(\"OPTIONS\") {",
        '    ("OK".to_string(), "text/plain".to_string())',
        "}",
        "",
      ].join("\n"),
    );

    execFileSync("git", ["add", "package.json", "crates/jobsentinel-assistance/src/bookmarklet/server.rs"], {
      cwd: root,
    });

    const violations = checkRepoBloat(root);

    assert.ok(
      violations.includes(
        "require structured bookmarklet pairing authorization: crates/jobsentinel-assistance/src/bookmarklet/server.rs",
      ),
      violations.join("\n"),
    );
  });
});

test("checkRepoBloat rejects bookmarklet code without pairing and origin binding", () => {
  withGitFixture((root) => {
    writeFixtureFile(root, "package.json", "{}\n");
    writeFixtureFile(
      root,
      "src-tauri/src/ipc/bookmarklet.rs",
      [
        'const TEMPLATE: &str = r#"javascript:(function(){',
        "document.createElement('iframe');",
        "fetch('http://localhost:4321/api/bookmarklet/import',{method:'POST'});",
        '})();"#;',
        "",
      ].join("\n"),
    );

    execFileSync("git", ["add", "package.json", "src-tauri/src/ipc/bookmarklet.rs"], {
      cwd: root,
    });

    const violations = checkRepoBloat(root);

    assert.ok(
      violations.includes(
        "bind bookmarklet code to structured pairing and origin: src-tauri/src/ipc/bookmarklet.rs",
      ),
      violations.join("\n"),
    );
  });
});

test("checkRepoBloat rejects non-atomic active pairing consumption", () => {
  withGitFixture((root) => {
    writeFixtureFile(root, "package.json", "{}\n");
    writeFixtureFile(
      root,
      "crates/jobsentinel-assistance/src/bookmarklet/server/pairing_state.rs",
      [
        "fn consume_active_pairing(active: &State, request: &Request) {",
        "  active.write().unwrap().as_mut().unwrap().authorize(request);",
        "}",
        "",
      ].join("\n"),
    );

    execFileSync(
      "git",
      [
        "add",
        "package.json",
        "crates/jobsentinel-assistance/src/bookmarklet/server/pairing_state.rs",
      ],
      { cwd: root },
    );

    const violations = checkRepoBloat(root);

    assert.ok(
      violations.includes(
        "make bookmarklet pairing atomic and one-use: crates/jobsentinel-assistance/src/bookmarklet/server/pairing_state.rs",
      ),
      violations.join("\n"),
    );
  });
});

test("checkRepoBloat accepts the repository-owned bookmarklet authorization boundary", () => {
  withGitFixture((root) => {
    writeFixtureFile(root, "package.json", "{}\n");
    writeFixtureFile(
      root,
      "crates/jobsentinel-assistance/src/bookmarklet/server/imports.rs",
      [
        "#[serde(deny_unknown_fields)]",
        "struct BookmarkletImportEnvelope {}",
        "async fn handle_import_request(repository: &Repository, grant: &Grant) {",
        "  consume_active_pairing();",
        "  repository.authorize_browser_action(grant).await;",
        "}",
        "",
      ].join("\n"),
    );

    execFileSync(
      "git",
      [
        "add",
        "package.json",
        "crates/jobsentinel-assistance/src/bookmarklet/server/imports.rs",
      ],
      { cwd: root },
    );

    const violations = checkRepoBloat(root);

    assert.ok(
      !violations.includes(
        "require structured bookmarklet pairing authorization: crates/jobsentinel-assistance/src/bookmarklet/server/imports.rs",
      ),
      violations.join("\n"),
    );
  });
});
