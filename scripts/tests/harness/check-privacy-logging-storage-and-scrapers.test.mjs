import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";
import {
  hasBookmarkletCodeWithoutPairingBoundary,
  hasLinkedInLoginCookieReturn,
  hasManualBookmarkletJsonErrorResponses,
  hasMlRawErrorDisplay,
  hasMlRawLocalPathDoc,
  hasMlRawLocalPathExposure,
  hasRawBackupPathError,
  hasRawBookmarkletImportLogging,
  hasRawImportBookmarkletCommandErrorDetails,
  hasRawJobsWithGptDebug,
  hasRawLinkedInDebug,
  hasRawLocalPathLogging,
  hasRawPrivateQueryLogging,
  hasRawSchedulerJobContentLogging,
  hasRawSchedulerScoringPrivacyLeak,
  hasRawSchedulerScraperErrorDetails,
  hasRawScoringCacheJobHashLogging,
  hasRawScraperLoopErrorLogging,
  hasRawScraperUrlOrQueryLogging,
  hasRawUserDataPrivacyLogging,
  hasResidualCorePrivacyLeak,
  hasNonAtomicBookmarkletPairing,
  hasUnauthenticatedBookmarkletImports,
  hasUnboundedExternalResponseBodyRead,
} from "../../harness/checks/privacy-logging.mjs";

function writeFixtureFile(root, path, content = "") {
  const fullPath = join(root, path);
  mkdirSync(dirname(fullPath), { recursive: true });
  writeFileSync(fullPath, content, "utf8");
}

function withFixture(callback) {
  const root = mkdtempSync(join(tmpdir(), "jobsentinel-privacy-logging-"));
  try {
    callback(root);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

test("privacy logging rejects raw private query fields", () => {
  withFixture((root) => {
    writeFixtureFile(
      root,
      "src-tauri/src/ipc/jobs.rs",
      'tracing::debug!("search query: {}", query);',
    );
    assert.equal(hasRawPrivateQueryLogging(root, "src-tauri/src/ipc/jobs.rs"), true);
    assert.equal(hasRawPrivateQueryLogging(root, "crates/jobsentinel-storage/src/lib.rs"), false);
  });
});

test("privacy logging rejects raw user-data and scheduler logging", () => {
  withFixture((root) => {
    writeFixtureFile(
      root,
      "src-tauri/src/ipc/user_data.rs",
      'tracing::info!("Creating saved search: {}", name);',
    );
    writeFixtureFile(
      root,
      "crates/jobsentinel-application/src/scheduler/workers/persistence.rs",
      "let job_title = job.title.clone();",
    );
    writeFixtureFile(
      root,
      "crates/jobsentinel-application/src/scheduler/workers/scrapers.rs",
      "fail_run(db, scraper_run_id, &e.to_string()).await;",
    );
    assert.equal(hasRawUserDataPrivacyLogging(root, "src-tauri/src/ipc/user_data.rs"), true);
    assert.equal(
      hasRawSchedulerJobContentLogging(
        root,
        "crates/jobsentinel-application/src/scheduler/workers/persistence.rs",
      ),
      true,
    );
    assert.equal(
      hasRawSchedulerScraperErrorDetails(
        root,
        "crates/jobsentinel-application/src/scheduler/workers/scrapers.rs",
      ),
      true,
    );
    assert.equal(hasRawSchedulerScraperErrorDetails(root, "src-tauri/src/ipc/jobs.rs"), false);
  });
});

test("privacy logging rejects raw import and bookmarklet details", () => {
  withFixture((root) => {
    writeFixtureFile(
      root,
      "src-tauri/src/ipc/import.rs",
      'format!("Invalid URL: {}", e);',
    );
    writeFixtureFile(
      root,
      "crates/jobsentinel-assistance/src/bookmarklet/server.rs",
      [
        'tracing::info!(title = %title, company = %company, "imported");',
        'format!(r#"{{"error":"{}"}}"#, e);',
        "if body_has_valid_bookmarklet_token(&body, auth_token) { save_job(); }",
        'if request.starts_with("POST /api/bookmarklet/import") {',
        "  handle_import_request(&request, database).await",
        "}",
        "",
      ].join("\n"),
    );
    writeFixtureFile(
      root,
      "src-tauri/src/ipc/bookmarklet.rs",
      'const TEMPLATE: &str = r#"fetch("/api/bookmarklet/import")"#;',
    );
    assert.equal(
      hasRawImportBookmarkletCommandErrorDetails(root, "src-tauri/src/ipc/import.rs"),
      true,
    );
    assert.equal(
      hasRawBookmarkletImportLogging(
        root,
        "crates/jobsentinel-assistance/src/bookmarklet/server.rs",
      ),
      true,
    );
    assert.equal(
      hasManualBookmarkletJsonErrorResponses(
        root,
        "crates/jobsentinel-assistance/src/bookmarklet/server.rs",
      ),
      true,
    );
    assert.equal(
      hasUnauthenticatedBookmarkletImports(
        root,
        "crates/jobsentinel-assistance/src/bookmarklet/server.rs",
      ),
      true,
    );
    assert.equal(
      hasNonAtomicBookmarkletPairing(
        root,
        "crates/jobsentinel-assistance/src/bookmarklet/server.rs",
      ),
      true,
    );
    assert.equal(
      hasBookmarkletCodeWithoutPairingBoundary(
        root,
        "src-tauri/src/ipc/bookmarklet.rs",
      ),
      true,
    );
    assert.equal(hasRawBookmarkletImportLogging(root, "src-tauri/src/ipc/import.rs"), false);
  });
});

test("privacy logging rejects raw scheduler scoring and residual core leaks", () => {
  withFixture((root) => {
    writeFixtureFile(
      root,
      "crates/jobsentinel-application/src/scoring/cache.rs",
      "tracing::debug!(job_hash = %job_hash);",
    );
    writeFixtureFile(
      root,
      "crates/jobsentinel-application/src/scheduler/workers/scoring.rs",
      "tracing::warn!(error = %e, job_hash = %job.hash);",
    );
    writeFixtureFile(
      root,
      "crates/jobsentinel-application/src/config/io.rs",
      'format!("Invalid API key: {}", e);',
    );
    assert.equal(
      hasRawScoringCacheJobHashLogging(
        root,
        "crates/jobsentinel-application/src/scoring/cache.rs",
      ),
      true,
    );
    assert.equal(
      hasRawSchedulerScoringPrivacyLeak(
        root,
        "crates/jobsentinel-application/src/scheduler/workers/scoring.rs",
      ),
      true,
    );
    assert.equal(
      hasResidualCorePrivacyLeak(root, "crates/jobsentinel-application/src/config/io.rs"),
      true,
    );
    assert.equal(hasResidualCorePrivacyLeak(root, "src-tauri/src/ipc/import.rs"), false);
  });
});

test("privacy logging rejects raw scraper URL and query output", () => {
  withFixture((root) => {
    writeFixtureFile(
      root,
      "crates/jobsentinel-network/src/external_request.rs",
      'tracing::debug!("Fetching API for query: {}", query);',
    );
    assert.equal(
      hasRawScraperUrlOrQueryLogging(
        root,
        "crates/jobsentinel-network/src/external_request.rs",
      ),
      true,
    );
    assert.equal(
      hasRawScraperUrlOrQueryLogging(root, "crates/jobsentinel-sources/src/scrapers/mod.rs"),
      false,
    );
  });
});

test("privacy logging rejects raw scraper loop errors outside tests", () => {
  withFixture((root) => {
    writeFixtureFile(
      root,
      "crates/jobsentinel-sources/src/scrapers/greenhouse.rs",
      [
        'tracing::warn!("Failed to scrape {}: {}", company.name, e);',
        "#[cfg(test)]",
        "mod tests {",
        '  tracing::warn!("Failed to scrape {}: {}", company.name, e);',
        "}",
      ].join("\n"),
    );
    assert.equal(
      hasRawScraperLoopErrorLogging(
        root,
        "crates/jobsentinel-sources/src/scrapers/greenhouse.rs",
      ),
      true,
    );
    assert.equal(
      hasRawScraperLoopErrorLogging(root, "crates/jobsentinel-sources/src/scrapers/dice.rs"),
      false,
    );
  });
});

test("privacy logging rejects unbounded external response body reads", () => {
  withFixture((root) => {
    writeFixtureFile(
      root,
      "crates/jobsentinel-sources/src/scrapers/dice.rs",
      "let body = response.text().await?;",
    );
    writeFixtureFile(
      root,
      "crates/jobsentinel-network/src/body.rs",
      "let body = response.text().await?;",
    );
    assert.equal(
      hasUnboundedExternalResponseBodyRead(
        root,
        "crates/jobsentinel-sources/src/scrapers/dice.rs",
      ),
      true,
    );
    assert.equal(
      hasUnboundedExternalResponseBodyRead(root, "crates/jobsentinel-network/src/body.rs"),
      false,
    );
  });
});

test("privacy logging rejects raw local path logging", () => {
  withFixture((root) => {
    writeFixtureFile(
      root,
      "src-tauri/src/ipc/resume.rs",
      'tracing::error!("resume path: {}", path.display());',
    );
    assert.equal(hasRawLocalPathLogging(root, "src-tauri/src/ipc/resume.rs"), true);
    assert.equal(hasRawLocalPathLogging(root, "src-tauri/src/ipc/jobs.rs"), false);
  });
});

test("privacy logging rejects raw backup path errors", () => {
  withFixture((root) => {
    writeFixtureFile(
      root,
      "crates/jobsentinel-storage/src/connection/backups.rs",
      'return Err(format!("Backup file not found: {}", backup_path.display()));',
    );
    assert.equal(
      hasRawBackupPathError(root, "crates/jobsentinel-storage/src/connection/backups.rs"),
      true,
    );
    assert.equal(hasRawBackupPathError(root, "crates/jobsentinel-storage/src/connection.rs"), false);
  });
});

test("privacy logging rejects ML local path exposure", () => {
  withFixture((root) => {
    writeFixtureFile(root, "src-tauri/src/ipc/ml.rs", "pub model_path: PathBuf,\n");
    writeFixtureFile(root, "docs/developer/LOCAL_SEMANTIC_MATCHING.md", "model_path: string\n");
    assert.equal(hasMlRawLocalPathExposure(root, "src-tauri/src/ipc/ml.rs"), true);
    assert.equal(hasMlRawLocalPathExposure(root, "src-tauri/src/ipc/jobs.rs"), false);
    assert.equal(hasMlRawLocalPathDoc(root, "docs/developer/LOCAL_SEMANTIC_MATCHING.md"), true);
    assert.equal(hasMlRawLocalPathDoc(root, "docs/README.md"), false);
  });
});

test("privacy logging rejects raw ML error display", () => {
  withFixture((root) => {
    writeFixtureFile(
      root,
      "crates/jobsentinel-local-ai/src/lib.rs",
      '#[error("model loading failed: {0}")]\nstruct MlError;',
    );
    assert.equal(hasMlRawErrorDisplay(root, "crates/jobsentinel-local-ai/src/lib.rs"), true);
    assert.equal(hasMlRawErrorDisplay(root, "crates/jobsentinel-local-ai/src/model.rs"), false);
  });
});

test("privacy logging rejects sensitive Debug derives", () => {
  withFixture((root) => {
    writeFixtureFile(
      root,
      "crates/jobsentinel-sources/src/scrapers/jobswithgpt.rs",
      "#[derive(Debug)]\npub struct JobQuery;",
    );
    writeFixtureFile(
      root,
      "crates/jobsentinel-sources/src/scrapers/linkedin.rs",
      "#[derive(Clone, Debug)]\npub struct LinkedInScraper;",
    );
    assert.equal(
      hasRawJobsWithGptDebug(
        root,
        "crates/jobsentinel-sources/src/scrapers/jobswithgpt.rs",
      ),
      true,
    );
    assert.equal(
      hasRawLinkedInDebug(root, "crates/jobsentinel-sources/src/scrapers/linkedin.rs"),
      true,
    );
    assert.equal(
      hasRawJobsWithGptDebug(root, "crates/jobsentinel-sources/src/scrapers/dice.rs"),
      false,
    );
    assert.equal(
      hasRawLinkedInDebug(root, "crates/jobsentinel-sources/src/scrapers/dice.rs"),
      false,
    );
  });
});

test("privacy logging rejects LinkedIn cookie return", () => {
  withFixture((root) => {
    writeFixtureFile(
      root,
      "src-tauri/src/ipc/linkedin_auth.rs",
      "tx.send(cookie_result.map(|(cookie, expires)| cookie))?;",
    );
    assert.equal(
      hasLinkedInLoginCookieReturn(root, "src-tauri/src/ipc/linkedin_auth.rs"),
      true,
    );
    assert.equal(hasLinkedInLoginCookieReturn(root, "src-tauri/src/ipc/config.rs"), false);
  });
});
