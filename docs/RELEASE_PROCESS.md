# 🚀 Release Process Guide

This document outlines the professional release and versioning system for the Private Job Scraper & Filter.

## 📋 Versioning Strategy

This project follows **[Semantic Versioning (SemVer)](https://semver.org/)** with the format `MAJOR.MINOR.PATCH`:

- **MAJOR** (X.0.0) - Breaking changes that require user action
- **MINOR** (0.X.0) - New features that are backward compatible
- **PATCH** (0.0.X) - Bug fixes and security updates

### Current Version: `1.0.0`

## 🎯 Release Types

### 🔴 Major Release (Breaking Changes)
- New architecture or significant rewrites
- Changes that break existing configurations
- Removal of deprecated features
- **Example**: `1.0.0` → `2.0.0`

### 🟡 Minor Release (New Features)
- New job board integrations
- Additional notification channels
- New configuration options (backward compatible)
- **Example**: `1.0.0` → `1.1.0`

### 🟢 Patch Release (Bug Fixes)
- Security updates
- Bug fixes
- Performance improvements
- **Example**: `1.0.0` → `1.0.1`

## 🛠️ How to Create a Release

### Method 1: Automatic Release (Recommended)

1. **Ensure all changes are committed and pushed to `main`**

2. **Create and push a version tag**:
   ```bash
   git tag v1.0.1
   git push origin v1.0.1
   ```

3. **GitHub Actions will automatically**:
   - Update the VERSION file
   - Generate changelog from commits
   - Create GitHub release with notes
   - Build and upload distribution assets
   - Update release badges

### Method 2: Manual Release (GitHub UI)

1. **Go to GitHub Actions** → **Release workflow**
2. **Click "Run workflow"**
3. **Enter the version** (e.g., `1.0.1`)
4. **Click "Run workflow"**

## 📝 Release Notes Format

The automated system generates release notes with:

### ✅ Automatic Sections:
- **What's Changed** - Generated from commit messages
- **Installation Instructions** - Platform-specific setup guides
- **Security Information** - Links to security documentation
- **Full Changelog** - Comparison links between versions

### 📋 Commit Message Format

For best release notes, use conventional commits:

```bash
feat: add new job board integration for Workday
fix: resolve timeout issue with Greenhouse scraping
docs: update installation guide for Windows 11
chore: bump dependencies to latest versions
security: update Playwright to fix CVE-2024-xxxx
```

## 🔧 Release Checklist

### Pre-Release
- [ ] All tests pass in CI/CD
- [ ] Security scan shows no new vulnerabilities
- [ ] Documentation is updated
- [ ] Breaking changes are documented
- [ ] VERSION file will be auto-updated

### Post-Release
- [ ] GitHub release is created automatically
- [ ] Release assets (tar.gz, zip) are uploaded
- [ ] Release notes are properly formatted
- [ ] Installation instructions are accurate
- [ ] Community is notified (if major release)

## 📦 Release Assets

Each release automatically includes:

1. **Source Code Archives**:
   - `job-private-scraper-filter-vX.Y.Z.tar.gz`
   - `job-private-scraper-filter-vX.Y.Z.zip`

2. **Complete Project Bundle**:
   - All source files and documentation
   - Setup scripts for all platforms
   - Example configuration files
   - License and legal documents

## 🎯 Version Management in Code

### Checking Current Version:
```bash
python agent.py --version
```

### Programmatic Access:
```python
from pathlib import Path

version_file = Path("VERSION")
current_version = version_file.read_text().strip()
print(f"Current version: {current_version}")
```

## 🔄 Hotfix Process

For critical security or bug fixes:

1. **Create hotfix branch**:
   ```bash
   git checkout -b hotfix/1.0.1
   ```

2. **Make the fix and test thoroughly**

3. **Create release**:
   ```bash
   git tag v1.0.1
   git push origin v1.0.1
   ```

4. **Merge back to main**:
   ```bash
   git checkout main
   git merge hotfix/1.0.1
   ```

## 📊 Release Schedule

### Regular Releases
- **Patch releases**: As needed for bugs/security
- **Minor releases**: Monthly (if new features ready)
- **Major releases**: Quarterly or when breaking changes necessary

### Security Releases
- **Critical vulnerabilities**: Within 24-48 hours
- **High severity**: Within 1 week
- **Medium/Low severity**: Next regular release

## 🎉 Release Promotion

### Automatic
- GitHub release with full changelog
- Updated badges in README
- Release assets for download

### Manual (for major releases)
- Blog post or announcement
- Social media promotion
- Community discussion post
- Update project website

## 🔍 Monitoring Releases

### Release Health Metrics
- Download statistics
- Issue reports post-release
- User feedback and adoption
- Security vulnerability reports

### Rollback Plan
If a release causes issues:
1. Create hotfix with revert
2. Release new patch version
3. Communicate issue to users
4. Update documentation as needed

## 📚 Additional Resources

- [Semantic Versioning Specification](https://semver.org/)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [GitHub Releases Documentation](https://docs.github.com/en/repositories/releasing-projects-on-github)
- [Security Release Best Practices](../SECURITY.md)

---

**Remember**: Every release should provide value to users while maintaining the stability and security of the platform. When in doubt, prefer smaller, more frequent releases over large, infrequent ones.