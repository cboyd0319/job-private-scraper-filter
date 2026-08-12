# Testing Guide

Complete guide to testing in JobSentinel.

---

## Table of Contents

- [Overview](#overview)
- [Running Tests](#running-tests)
- [Test Organization](#test-organization)
- [Testing Patterns](#testing-patterns)
- [Writing New Tests](#writing-new-tests)
- [Test Coverage](#test-coverage)
- [End-to-End Tests](#end-to-end-tests)
- [Local Verification](#local-verification)

---

## Overview

JobSentinel uses a comprehensive testing strategy across all layers:

- **Unit Tests**: Test individual functions and modules in isolation
- **Integration Tests**: Test interaction between modules
- **End-to-End Tests**: Test complete workflows with Playwright

### Testing Philosophy

> **"If it's hard to test, the design is probably wrong."**

We prioritize:

1. **Fast, deterministic tests** (<100ms each)
2. **Clear test names** describing behavior, conditions, and outcomes
3. **Isolated tests** using in-memory databases and mocks
4. **Comprehensive coverage** of happy paths, edge cases, and failure modes

---

## Running Tests

### Run All Tests

```bash
cargo test --workspace
```

### Run Tests in a Specific Module

```bash
# Test only config module
cargo test -p jobsentinel-application config

# Test only database module
cargo test -p jobsentinel-storage

# Test only Slack notifications
cargo test -p jobsentinel-notifications slack
```

### Run a Specific Test

```bash
cargo test -p jobsentinel-storage test_negative_salary_floor_fails
```

### Run Tests with Output

```bash
cargo test --workspace -- --nocapture
```

### Run Tests in Parallel (Default)

```bash
cargo test --workspace -- --test-threads=4
```

### Run Tests Sequentially

```bash
cargo test --workspace -- --test-threads=1
```

---

## Test Organization

### Directory Structure

```text
crates/jobsentinel-*/
- src/: owner implementation and adjacent tests
- tests/: owner-specific integration test crates where needed
src-tauri/
- src/commands/: private Tauri RPC handlers and command tests
```

**Note**: Large Rust modules keep most tests in separate `tests.rs` files or
feature-specific test subdirectories. This improves code organization and keeps
module files easier to maintain under the current
[harness file-size policy](../harness/README.md), enforced by
`scripts/harness/contracts/repository-structure.json` through `npm run lint:bloat`.

Repeated Rust setup belongs to a crate-local `test_support` owner. Keep
scenario-specific mutations and assertions in each test, and use production
migrations for database tests unless deliberate schema isolation is the subject
under test. Cross-crate fixtures must be exposed only through an explicit
`test-support` feature.

### Test Coverage Map

Test counts change as features move. Use fresh command output as source of truth.

| Area | Coverage |
| ---- | -------- |
| `jobsentinel-application/config` | Validation and defaults |
| `jobsentinel-storage` | CRUD, queries, analytics, integrity checks |
| `jobsentinel-notifications` | Slack, Discord, Teams, Telegram, email |
| `commands` | Tauri RPC handler behavior |
| `core/scoring` | Scoring config and score calculations |
| `core/scheduler` | Lifecycle, workers, shutdown |
| `core/scrapers/*` | Parser behavior, rate limiting, error handling |
| `core/ats` | Application tracking |
| `core/resume` | Parser, matcher, builder, export |
| `core/salary` | Benchmarks, prediction, offer comparison |
| `core/market_intelligence` | Trends, alerts, analytics |
| `src/**/*.test.*` | App composition, feature owners, shared services, UI primitives, and development mocks |
| `tests/e2e/playwright/*` | Browser-level user flows |

---

## Testing Patterns

### Pattern 1: In-Memory Database Tests

**Use for**: Testing database operations in isolation

```rust
#[tokio::test]
async fn test_upsert_job() {
    // Create in-memory database
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();

    let job = create_test_job("hash123", "Test Job", 0.95);

    let id = db.upsert_job(&job).await.unwrap();
    assert!(id > 0);

    // Database is automatically cleaned up when dropped
}
```

**Benefits:**

- Fast (no disk I/O)
- Isolated (no shared state)
- Auto-cleanup

### Pattern 2: Temporary Files/Directories

**Use for**: Testing file operations

```rust
#[test]
fn test_save_and_load_config() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.json");

    let config = create_valid_config();
    config.save(&config_path).expect("Failed to save");

    let loaded = Config::load(&config_path).expect("Failed to load");
    assert_eq!(loaded.salary_floor_usd, config.salary_floor_usd);

    // TempDir is automatically cleaned up when dropped
}
```

**Benefits:**

- Isolated filesystem operations
- No cleanup code needed
- Works across all OSes

### Pattern 3: Helper Functions

**Use for**: Reducing test boilerplate

```rust
/// Helper to create a valid test config
fn create_valid_config() -> Config {
    Config {
        title_allowlist: vec!["Program Analyst".to_string()],
        salary_floor_usd: 85000,
        // ... other fields
    }
}

#[test]
fn test_something() {
    let config = create_valid_config();
    // Test logic here
}
```

**Benefits:**

- DRY (Don't Repeat Yourself)
- Easy to update test data
- Self-documenting

### Pattern 4: Testing Validation Logic

**Use for**: Testing input validation

```rust
#[test]
fn test_negative_salary_floor_fails() {
    let mut config = create_valid_config();
    config.salary_floor_usd = -1000;

    let result = config.validate();
    assert!(result.is_err(), "Negative salary should fail");
    assert!(result.unwrap_err().to_string().contains("negative"));
}
```

**Benefits:**

- Clear intent
- Tests error messages
- Documents validation rules

### Pattern 5: Boundary Testing

**Use for**: Testing edge cases at boundaries

```rust
#[test]
fn test_salary_floor_at_boundary_passes() {
    let mut config = create_valid_config();
    config.salary_floor_usd = 10_000_000; // Exactly at $10M limit

    assert!(config.validate().is_ok());
}
```

**Benefits:**

- Catches off-by-one errors
- Documents boundaries
- High bug-finding potential

---

## Writing New Tests

### 1. Test Naming Convention

Use descriptive names that explain:

- **What** is being tested
- **Under what conditions**
- **What the expected outcome is**

```rust
// Good: descriptive behavior and expected outcome
#[test]
fn test_negative_salary_floor_fails() { ... }

#[test]
fn test_upsert_job_increments_times_seen() { ... }

// Bad: unclear behavior and expected outcome
#[test]
fn test1() { ... }

#[test]
fn test_config() { ... }
```

### 2. Test Structure (Arrange-Act-Assert)

```rust
#[test]
fn test_example() {
    // Arrange: Set up test data
    let db = Database::connect_memory().await.unwrap();
    let job = create_test_job("hash", "Title", 0.9);

    // Act: Perform the operation
    let result = db.upsert_job(&job).await;

    // Assert: Verify expectations
    assert!(result.is_ok());
    let id = result.unwrap();
    assert!(id > 0);
}
```

### 3. Testing Async Code

Use `#[tokio::test]` for async tests:

```rust
#[tokio::test]
async fn test_async_operation() {
    let db = Database::connect_memory().await.unwrap();
    // Async operations...
}
```

### 4. Testing Error Cases

Always test both success and failure:

```rust
#[test]
fn test_valid_webhook_url_passes() {
    let result = validate_webhook_url("https://hooks.slack.com/services/...");
    assert!(result.is_ok());
}

#[test]
fn test_invalid_webhook_url_fails() {
    let result = validate_webhook_url("https://evil.com/webhook");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("hooks.slack.com"));
}
```

---

## Test Coverage

### Current Coverage Summary

As of the latest test suite addition:

- **Config validation**: 100% of validation rules tested
- **Database operations**: All CRUD operations + edge cases
- **Slack notifications**: Webhook validation + message formatting
- **Tauri commands**: All RPC endpoints tested
- **Scraper hashing**: Comprehensive hash computation tests

### Checking Coverage (Manual)

Run tests with coverage flags:

```bash
# Install tarpaulin
cargo install cargo-tarpaulin --version 0.37.0 --locked

# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage
```

Open `coverage/index.html` to view detailed coverage.

### Coverage Goals

| Category            | Target | Status         |
| ------------------- | ------ | -------------- |
| Core business logic | 90%+   | Achieved       |
| Database layer      | 85%+   | Achieved       |
| Configuration       | 100%   | Achieved       |
| Tauri commands      | 80%+   | Achieved       |
| Scrapers            | 70%+   | In progress    |
| Platform-specific   | 60%+   | Achieved       |

---

## Local Verification

Hosted CI is absent under the named `pre-alpha-private-no-ci` exception. Local
commands are the only integration feedback during private pre-alpha development,
so retain the actual command, result, platform, and caveat.

Run `npm run harness:plan -- --since <valid-ref>` for a targeted lane or
`npm run verify:full` for the complete local suite. Ignored or live integration
tests remain opt-in and use targeted commands such as
`cargo test -p jobsentinel-sources scrapers::live_tests -- --ignored --nocapture`.

Credential-store roundtrips are opt-in because macOS Keychain and equivalent
stores can prompt for user approval. Default credential tests stay
non-interactive. Run live keyring checks only when you are ready for OS prompts:

```bash
JOBSENTINEL_LIVE_KEYRING_TESTS=1 cargo test -p jobsentinel --lib credential_integration_tests -- --nocapture
```

Repository npm Rust-verification wrappers remove this opt-in from child
environments even if the parent shell set it. Use the explicit raw Cargo command
above only for attended live credential verification.

For the no-CI exception and separately authorized release automation, see
[CI_CD.md](./CI_CD.md).

### Pre-commit Hook

The tracked Husky hook delegates to `scripts/harness/pre-commit.mjs`. It runs
secret scanning and staged-file checks only after a developer explicitly enables
Husky. The standard initializer disables dependency lifecycle scripts, so it
does not install hooks. Use `HUSKY=0` or `git commit --no-verify` only for
recovery, then run the applicable local verification before relying on that
commit.

---

## Best Practices

### Do

- Write tests FIRST when fixing bugs (TDD)
- Test edge cases and boundary conditions
- Use descriptive test names
- Keep tests fast (<100ms each)
- Use helper functions to reduce boilerplate
- Clean up resources automatically (use RAII)
- Test both success and failure paths

### Do Not

- Don't write flaky tests (use deterministic data)
- Don't test implementation details
- Don't share state between tests
- Don't skip tests (fix or remove them)
- Don't write tests without assertions
- Don't commit failing tests

---

## Debugging Failed Tests

### 1. Run with Output

```bash
cargo test test_name -- --nocapture
```

### 2. Run Single Test

```bash
cargo test test_specific_function -- --exact
```

### 3. Enable Logging

```bash
RUST_LOG=debug cargo test
```

### 4. Use `dbg!()` Macro

```rust
#[test]
fn test_example() {
    let result = calculate_something();
    dbg!(&result); // Prints value with file/line info
    assert_eq!(result, expected);
}
```

---

## End-to-End Tests

E2E tests use Playwright against the Vite mock dev server to test complete
browser workflows.

The `test:e2e*` npm scripts run Playwright through
`scripts/dev/run-playwright.mjs`. That wrapper keeps local and hosted-release output readable
on current Node versions by removing conflicting color environment settings and
silencing the known upstream `DEP0205` deprecation warning from Playwright or
Tailwind internals. It also selects an available loopback port and never reuses
an existing server by default. It does not silence test failures or application
warnings.

### Running E2E Tests

Run `npm run doctor:e2e` when setting up Playwright locally or when browser
tests fail before app code loads. It makes Playwright Chromium launch a hard
readiness gate and prints the install command if the browser is missing.

```bash
# Check local E2E readiness
npm run doctor:e2e

# Run local Chromium functional E2E tests
npm run test:e2e

# Run full cross-browser E2E tests
npm run test:e2e:all

# Run full cross-browser E2E tests with the maintained four-minute budget
npm run test:e2e:all:budget

# Run in interactive UI mode
npm run test:e2e:ui
```

Playwright uses port 5173 when it is available and otherwise asks the operating
system for an available loopback port. Set `PLAYWRIGHT_PORT` only when a fixed
test port is required; the wrapper fails clearly if that port is occupied.
Set `PLAYWRIGHT_REUSE_EXISTING_SERVER=1` only when the target is a verified
JobSentinel mock server. The installed Tauri GUI does not use a web-server port;
the development GUI uses port 5173. Browser Import is separate: it prefers its
saved port, which defaults to 4321, and selects and persists an available
loopback port if another process already owns that port.

### E2E Test Coverage

| Page            | Test File                | Tests                                          |
| --------------- | ------------------------ | ---------------------------------------------- |
| Dashboard       | `app.spec.ts`, `job-search-filtering.spec.ts` | Load, nav, search, filters, job cards |
| Settings        | `settings-save-load.spec.ts` | Modal nav, basic preferences, advanced notifications, credentials, ghost detection, save/load |
| Applications    | `application-tracking.spec.ts` | Kanban, cards, drag/drop, detail modal, status and notes, reminders |
| Hiring Trends   | `hiring-trends.spec.ts` | Tabs, snapshot, charts, locations, alert read state |
| Resume          | `resume-upload-matching.spec.ts` | No-resume state, active resume, skill CRUD, library switching, match results |
| Resume Builder  | `resume-builder.spec.ts` | Draft init, contact/summary validation, experience, education, skills/import, preview/template, PDF/DOCX/JSON export |
| Keyboard        | `keyboard-navigation.spec.ts` | Page shortcuts, command palette, help modal, search focus, focus trap, skip link |
| Application Assist | `application-assist.spec.ts` | Settings stats, profile validation/save/load, screening answers, human-review guardrails |

Documentation screenshots live in `screenshots.spec.ts` and run only through
`npm run docs:screenshots`.

### Writing E2E Tests

```typescript
import { expect, test } from "@playwright/test";

test.describe("Feature", () => {
  test("does something", async ({ page }) => {
    await page.goto("/");

    const button = page.getByRole("button", { name: "Search Now" });
    await button.click();

    await expect(button).toBeVisible();
  });
});
```

See [tests/README.md](../../tests/README.md) for full documentation.

---

## Resources

- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [tokio Testing](https://tokio.rs/tokio/topics/testing)
- [tempfile Crate](https://docs.rs/tempfile/)
- [SQLx Testing](https://github.com/launchbadge/sqlx#testing)
- [Playwright documentation](https://playwright.dev/docs/intro)
