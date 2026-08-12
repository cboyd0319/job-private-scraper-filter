# JobSentinel E2E Tests

Comprehensive Playwright E2E tests for critical user flows in JobSentinel.

## Test Structure

### Page Objects (`page-objects/`)

Reusable page objects following the Page Object Model pattern:

- `BasePage.ts` - Base class with common functionality
- `DashboardPage.ts` - Job search and filtering
- `ResumePage.ts` - Resume state, skills, library, and match results
- `ApplicationsPage.ts` - Application tracking kanban
- `SettingsPage.ts` - Settings management
- `OneClickApplyPage.ts` - Application Assist settings, profile, and screening answers
- `ResumeBuilderPage.ts` - Resume Builder wizard
- `MarketIntelligencePage.ts` - Hiring Trends features
- `JobDetailPage.ts` - Job detail view and interactions

### Test Suites

1. **job-search-filtering.spec.ts**
   - Search functionality with keywords
   - Filter by location, salary, experience
   - Job card interactions (bookmark, view, apply)
   - Error handling

2. **resume-upload-matching.spec.ts**
   - Empty resume state and native import actions
   - Active resume, extracted skills, and match results
   - Skill add/edit/delete and category filtering
   - Resume library activation and deletion

3. **application-tracking.spec.ts**
   - Kanban board display
   - Application card details and status updates
   - Drag-and-drop status changes
   - Notes persistence, reminders, ghost detection, and toolbar dialogs

4. **settings-save-load.spec.ts**
   - Settings persistence across sessions
   - General, notifications, privacy settings
   - Auto-save and reset functionality
   - Validation and error handling

5. **keyboard-navigation.spec.ts**
   - Global keyboard shortcuts (Ctrl/Cmd+1-8)
   - Command palette (Ctrl/Cmd+K)
   - Keyboard help modal
   - Search focus, focus trap, and skip link behavior

6. **application-assist.spec.ts**
   - Settings stats and tabs
   - Application profile validation, save, and reload behavior
   - Screening answer add/edit validation and persistence
   - Human-review and manual-final-submit guardrails

7. **resume-builder.spec.ts**
   - Wizard navigation (7 steps)
   - Contact and summary validation
   - Experience, education, and skills management
   - Skill import from seeded active resume
   - Template selection, ATS preview, and DOCX export

8. **hiring-trends.spec.ts**
   - Overview tab with market metrics
   - Trend charts for skills and companies
   - Skills trends analysis
   - Company activity tracking
   - Location heatmap
   - Market alert read state

9. **job-interactions.spec.ts**
   - Bookmarking jobs
   - Adding notes to jobs
   - Application status tracking
   - Search and filtering
   - Match score display
   - Complete user flow

10. **app.spec.ts**
    - App shell smoke coverage
    - Theme toggle
    - Sidebar destination navigation
    - Responsive shell rendering

11. **screenshots.spec.ts**
    - Documentation screenshot capture
    - Excluded from normal E2E runs
    - Refreshes tracked docs images only through `npm run docs:screenshots`

## Running Tests

### Local Functional Tests

```bash
npm run test:e2e
```

Runs Chromium only and excludes documentation screenshot capture.
Local Playwright runs use 4 workers by default, reduced-motion browser settings,
and the line reporter. Override local concurrency with
`PLAYWRIGHT_WORKERS=<count>` when a machine is under load.

### Smoke Tests

```bash
npm run test:e2e:smoke
npm run test:e2e:smoke:budget
npm run test:e2e:smoke:all
```

Use smoke tests during routine frontend work. They cover app load, search,
keyboard focus, applications, settings, resume, Application Assist, resume
builder, market intelligence, and safe support report generation without
running every browser-flow assertion.
The budget command runs the Chromium smoke suite with Playwright JSON output
and fails when duration or test count exceeds the maintained budget.

### Full Cross-Browser Tests

```bash
npm run test:e2e:all
npm run test:e2e:all:budget
```

Runs Chromium and WebKit functional E2E tests. Use the budget command when
checking that the full suite has not drifted back into slow local feedback
loops. The maintained full-suite budget is four minutes.

### Last Failed Tests

```bash
npm run test:e2e:last-failed
```

Use after a failed full run to avoid burning another full browser matrix while
debugging.

### Refresh Documentation Screenshots

```bash
npm run docs:screenshots
```

Pass Playwright flags after `--` when needed:

```bash
npm run docs:screenshots -- --headed
```

### Specific Test Suite

```bash
npm run test:e2e -- tests/e2e/playwright/job-search-filtering.spec.ts
```

### Headed Mode (see browser)

```bash
npm run test:e2e:headed
```

### Debug Mode

```bash
npx --no-install playwright test --debug
```

### Specific Browser

```bash
npx --no-install playwright test --project=chromium
npx --no-install playwright test --project=webkit
```

## Configuration

Tests are configured in `playwright.config.ts`:

- Base URL: an available loopback port selected by `scripts/dev/run-playwright.mjs`
- Test directory: `tests/e2e/playwright`
- Projects: Chromium, WebKit
- Local default: Chromium functional tests, excluding `screenshots.spec.ts`
- Local workers: 4 by default, override with `PLAYWRIGHT_WORKERS`
- Reporter: line by default, HTML when `PLAYWRIGHT_HTML_REPORT=1`
- Reduced motion: enabled to avoid waiting on hover animations and transitions
- Retries: 2 when `CI=true`, 0 otherwise
- Mock data: Uses `npm run dev:mock`

## Test Patterns

### Platform Gaps

Active E2E suites should assert current mocked UI behavior instead of hiding
stale element probes behind runtime skips. If a browser or native capability gap
is real, document it in `docs/plans/tech-debt-tracker.md`, keep the affected
assertion narrow, and prefer a command-line project filter while debugging.

### Error Handling

All tests include error boundaries and handle missing elements:

```typescript
const hasElement = await element.isVisible().catch(() => false);
```

### Waiting Strategies

- Prefer `expect(locator).toBeVisible()` or `expect.poll()` for user-visible
  state.
- Use page-object methods that wait for `#main-content` or a feature-specific
  element.
- Keep fixed timeouts out of normal functional suites. Screenshot capture can
  use explicit waits for visual stability.

### Page Object Pattern

```typescript
const dashboard = new DashboardPage(page);
await dashboard.navigateTo();
await dashboard.searchForJobs("customer support");
const card = await dashboard.getJobCard(0);
await card.bookmark();
```

## Test Data

### Mock Data

Tests run against mock data defined in `src/dev-runtime/mocks/`:

- Mock job listings
- Mock applications
- Mock settings
- Mock resumes, skills, builder drafts, templates, previews, and match results

## Automation Profile

JobSentinel has no hosted continuous integration during private pre-alpha
development. Setting `CI=true` explicitly selects the conservative automation
profile: two retries and one worker. HTML reports remain opt-in through
`PLAYWRIGHT_HTML_REPORT=1`; screenshots, video, and traces are retained on
failure according to `playwright.config.ts`.

## Best Practices

1. **Test Isolation** - Each test is independent
2. **Page Objects** - Reusable components
3. **Explicit Platform Gaps** - Skip only intentional unavailable capabilities
4. **Error Handling** - All edge cases covered
5. **Accessibility** - Keyboard and screen reader support
6. **Performance** - Fast execution with parallel tests

## Debugging

### View Test Report

```bash
npx --no-install playwright show-report
```

### View Trace

```bash
npx --no-install playwright show-trace trace.zip
```

### Screenshots on Failure

Screenshots automatically saved to `test-results/` on failure.

## Contributing

When adding new tests:

1. Create page object if needed
2. Follow existing patterns (AAA: Arrange, Act, Assert)
3. Avoid runtime skips for current UI; use seeded mock state instead
4. Test happy paths AND error scenarios
5. Add to this README

## Coverage

Current test coverage:

- Job search and filtering
- Resume upload and matching
- Application tracking
- Settings management
- Keyboard navigation
- Application Assist settings and screening answers
- Resume Builder wizard
- Hiring Trends features
- Job interactions (bookmarking, notes, status)
- Theme toggle
- Responsive design
- Accessibility

## Known Issues

- Drag-and-drop uses the production keyboard sensor and exact target assertions.
- Native file-picker coverage belongs in a Tauri-level smoke test, not browser-only Playwright.

## Resources

- [Playwright Documentation](https://playwright.dev)
- [Page Object Model](https://playwright.dev/docs/pom)
- [Best Practices](https://playwright.dev/docs/best-practices)
