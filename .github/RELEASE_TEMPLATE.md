## 🎉 Job Scraper v{VERSION}

<!-- Brief description of this release -->

### ✨ What's New

<!-- Highlight major new features -->
-

### 🔧 Improvements

<!-- List enhancements and optimizations -->
-

### 🐛 Bug Fixes

<!-- List fixed issues -->
-

### 🛡️ Security Updates

<!-- List security-related changes -->
-

### ⚠️ Breaking Changes

<!-- List any breaking changes (for major versions) -->
-

## 📦 Installation

### Windows (Recommended)
```powershell
Set-ExecutionPolicy Bypass -Scope Process -Force; [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072; irm "https://raw.githubusercontent.com/cboyd0319/job-private-scraper-filter/main/setup_windows.ps1" | iex
```

### macOS/Linux
```bash
git clone https://github.com/cboyd0319/job-private-scraper-filter.git
cd job-private-scraper-filter
git checkout v{VERSION}
chmod +x setup.sh && ./setup.sh
```

### Manual Installation
Download the source code archive below and follow the [installation guide](docs/INSTALLATION.md).

## 🔄 Upgrade Instructions

### From Previous Versions
```bash
cd job-private-scraper-filter
git fetch origin
git checkout v{VERSION}
pip install -r requirements.txt --upgrade
python -m playwright install chromium
```

### Configuration Changes
<!-- List any configuration file changes needed -->
-

## 🧪 Testing

This release has been tested with:
- ✅ Python 3.11, 3.12, 3.13
- ✅ Windows 10/11, macOS, Ubuntu Linux
- ✅ All supported job board platforms
- ✅ Security vulnerability scans
- ✅ Cross-platform compatibility

## 📊 Metrics

<!-- Include relevant metrics if available -->
- Job boards supported: X
- Total commits: X
- Contributors: X
- Lines of code: X

## 🙏 Contributors

Thanks to all contributors who made this release possible!

<!-- Auto-generated contributor list -->

## 📚 Documentation

- 📖 [Installation Guide](docs/INSTALLATION.md)
- 🛡️ [Security Policy](SECURITY.md)
- 🤝 [Contributing Guidelines](CONTRIBUTING.md)
- 🚀 [Future Enhancements](docs/FUTURE_ENHANCEMENTS.md)
- 🔄 [Release Process](docs/RELEASE_PROCESS.md)

## 🆘 Support

- 🐛 [Report Issues](https://github.com/cboyd0319/job-private-scraper-filter/issues)
- 💬 [Discussions](https://github.com/cboyd0319/job-private-scraper-filter/discussions)
- 📧 [Security Reports](SECURITY.md#reporting-a-vulnerability)

---

**Full Changelog**: https://github.com/cboyd0319/job-private-scraper-filter/compare/v{PREVIOUS_VERSION}...v{VERSION}