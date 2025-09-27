# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2025-01-26

### Added
- 🎉 **Initial release** of Private Job Scraper & Filter
- **Multi-platform job board support**: Greenhouse, Lever, Workday, Generic JS
- **Smart job scoring system** with configurable filters and keyword boosting
- **Real-time Slack notifications** for high-scoring job matches
- **Daily email digests** with comprehensive job summaries
- **ChatGPT API integration** for AI-enhanced job matching (optional)
- **Enterprise resilience features**:
  - Automatic database backups and corruption recovery
  - Network failure handling with exponential backoff
  - Process locking to prevent multiple instances
  - Comprehensive health monitoring (11 metrics)
- **Cross-platform automation**:
  - Windows PowerShell setup script with scheduled tasks
  - macOS/Linux bash setup with cron scheduling
- **Privacy-first design**: 100% local execution, no data sharing
- **Zero-maintenance operation** with self-healing capabilities

### Technical Features
- **Database**: SQLite with SQLModel ORM for robust data management
- **Web Scraping**: Playwright for JavaScript-heavy career pages
- **Configuration**: JSON-based user preferences with .env secrets
- **Logging**: Structured logging with daily rotation and error tracking
- **Error Handling**: Comprehensive exception handling (70+ try/catch blocks)
- **Rate Limiting**: Respectful scraping with configurable delays
- **Token Management**: ChatGPT API cost controls and usage tracking

### Documentation
- **Comprehensive README** with feature overview and quick start
- **Detailed Windows setup guide** (361 lines) for non-technical users
- **ChatGPT integration guide** with cost analysis and setup
- **Contributing guidelines** for open-source collaboration
- **MIT License** for maximum compatibility

### Supported Platforms
- **Python**: 3.11+ (tested on 3.11, 3.12, 3.13)
- **Operating Systems**: Windows 10/11, macOS, Linux
- **Job Boards**: 4 major platform types covering 100+ companies

### Security & Privacy
- **No telemetry or tracking**
- **Environment variable secret management**
- **Local-only data processing**
- **Configurable rate limiting**
- **Security validation in configuration loading**

---

## [Unreleased]

### Planned Features
- Additional job board integrations (Ashby, SmartRecruiters)
- Discord notification support
- Docker containerization
- Web UI enhancements
- Advanced analytics and reporting
- Multi-language support

---

**Legend:**
- 🎉 Major feature
- ✨ New feature
- 🐛 Bug fix
- 📚 Documentation
- 🔧 Technical improvement
- ⚠️ Breaking change