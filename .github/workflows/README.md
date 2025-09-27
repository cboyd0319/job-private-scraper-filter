# GitHub Actions Workflows

This directory contains the CI/CD workflows for the Private Job Scraper & Filter project.

## Active Workflows

### 1. `ci.yml` - CI/CD Pipeline
- **Triggers**: Push to main/develop, PRs to main
- **Purpose**: Comprehensive testing, security scanning, and code quality checks
- **Features**:
  - Intelligent path-based filtering (only runs full tests when code changes)
  - Cross-platform testing (Ubuntu, Windows, macOS)
  - Python 3.11, 3.12, 3.13 support
  - Security scanning with Bandit and Safety
  - Code quality checks with Black, isort, flake8

### 2. `release.yml` - Automated Releases
- **Triggers**: Version tags (v*.*.*), manual workflow dispatch
- **Purpose**: Creates GitHub releases with automated changelog and assets
- **Features**:
  - Semantic versioning support
  - Automatic changelog generation from commits
  - Release asset building (tar.gz, zip)
  - Professional release notes with installation instructions

### 3. `dependency-submission.yml` - Dependency Management
- **Triggers**: Changes to requirements.txt, weekly schedule
- **Purpose**: Submits Python dependencies to GitHub's dependency graph
- **Features**:
  - Security vulnerability alerts
  - Dependency insights and management
  - Compatible with Dependabot

## Configuration Files

### `dependabot.yml`
- Automated dependency updates for Python packages and GitHub Actions
- Weekly schedule with review assignments
- Conventional commit message formatting

## Workflow Philosophy

1. **Efficiency**: Workflows only run when necessary (path-based filtering)
2. **Reliability**: Cross-platform testing ensures broad compatibility
3. **Security**: Multiple security scanning tools and dependency monitoring
4. **Automation**: Minimal manual intervention required for releases and updates

## Troubleshooting

### Common Issues

1. **"python-version input not set" warning**
   - Resolved by explicit `python-version` specification in all workflows
   - All workflows now use Python 3.12 as the default version

2. **Excessive workflow runs**
   - CI pipeline uses path filtering to avoid unnecessary runs
   - Documentation changes only trigger lightweight checks

3. **Permission errors**
   - Workflows have minimal required permissions for security
   - Dependency submission has `security-events: write` for vulnerability alerts

### Manual Workflow Triggers

```bash
# Create a new release
git tag v1.0.1
git push origin v1.0.1

# Or use GitHub UI: Actions > Release > Run workflow
```

## Security Considerations

- Workflows run with minimal permissions
- No secrets are exposed in logs
- Security scanning runs on all code changes
- Dependency vulnerabilities are automatically detected