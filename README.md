
# WAIT
## This project is VERY new, and I am making updates quite often. It is NOT ready for use. I will remove this message when it is.


# 🔍 Private Job Scraper & Filter

<div align="center">

**A robust, enterprise-grade job monitoring service that runs entirely on your own machine**

[![Python 3.11+](https://img.shields.io/badge/python-3.11+-blue.svg)](https://www.python.org/downloads/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Cross Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)](https://github.com/cboyd0319/job-private-scraper-filter)
[![CI/CD](https://github.com/cboyd0319/job-private-scraper-filter/workflows/CI%2FCD%20Pipeline/badge.svg)](https://github.com/cboyd0319/job-private-scraper-filter/actions)
[![Security](https://img.shields.io/badge/security-verified-green.svg)](SECURITY.md)
[![Code of Conduct](https://img.shields.io/badge/code%20of%20conduct-contributor%20covenant-purple.svg)](CODE_OF_CONDUCT.md)
[![GitHub release](https://img.shields.io/github/v/release/cboyd0319/job-private-scraper-filter.svg)](https://github.com/cboyd0319/job-private-scraper-filter/releases)
[![GitHub stars](https://img.shields.io/github/stars/cboyd0319/job-private-scraper-filter.svg?style=social)](https://github.com/cboyd0319/job-private-scraper-filter/stargazers)

</div>

## 📋 Table of Contents

- [✨ Features](#-features)
- [🏗️ Supported Job Boards](#️-supported-job-boards)
- [🚀 Quick Start](#-quick-start)
- [⚙️ Configuration](#️-configuration)
- [🧠 AI Enhancement](#-ai-enhancement-optional)
- [🎮 Usage](#-usage)
- [📊 System Health & Monitoring](#-system-health--monitoring)
- [🛡️ Enterprise Features](#️-enterprise-features)
- [📁 Project Structure](#-project-structure)
- [🔒 Security & Privacy](#-security--privacy)
- [🤝 Contributing](#-contributing)
- [📄 License](#-license)
- [🆘 Support](#-support)

## ✨ Features

- 🤖 **Automated Job Discovery** - Monitors multiple job boards every 15 minutes
- 🎯 **Smart Filtering** - Rule-based scoring with optional AI enhancement via ChatGPT
- 📱 **Real-time Alerts** - Immediate Slack notifications for high-scoring matches
- 📧 **Daily Digests** - Comprehensive email summaries of relevant opportunities
- 🔒 **Privacy First** - Runs 100% locally, your data never leaves your machine
- 🛡️ **Enterprise Resilience** - Auto-recovery, health monitoring, and database backups
- 🌐 **JavaScript Support** - Handles modern SPA job boards (React, Vue, etc.)
- ⚙️ **Zero Maintenance** - Self-healing system with comprehensive error handling

## 🏗️ Supported Job Boards

| Platform | Type | Examples |
|----------|------|----------|
| **Greenhouse** | `greenhouse` | Discord, Cloudflare, Stripe |
| **Lever** | `lever` | Netflix, Uber, Airbnb |
| **Workday** | `workday` | Most Fortune 500 companies |
| **Generic JS** | `generic_js` | Modern SPA career pages |

## 🚀 Quick Start

### Windows (Recommended)

**One-command setup** that installs everything automatically:

```powershell
Set-ExecutionPolicy Bypass -Scope Process -Force; [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072; irm "https://raw.githubusercontent.com/cboyd0319/job-private-scraper-filter/main/setup_windows.ps1" | iex
```

See [**📘 Detailed Windows Setup Guide**](docs/WINDOWS_SETUP_GUIDE.md) for step-by-step instructions.

### macOS/Linux

```bash
git clone https://github.com/cboyd0319/job-private-scraper-filter.git
cd job-private-scraper-filter
chmod +x setup.sh && ./setup.sh
```

## ⚙️ Configuration

Create your configuration files from the examples:

```bash
cp .env.example .env
cp user_prefs.example.json user_prefs.json
```

### 📧 Notifications Setup

Edit `.env` file:

```bash
# Slack webhook for instant alerts
SLACK_WEBHOOK_URL=https://hooks.slack.com/services/YOUR/SLACK/WEBHOOK

# Email settings for daily digest
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USER=your_email@gmail.com
SMTP_PASS=your_app_password
DIGEST_TO=your_email@gmail.com
```

### 🎯 Job Filters

Edit `user_prefs.json`:

```json
{
  "companies": [
    {
      "id": "stripe",
      "board_type": "lever",
      "url": "https://jobs.lever.co/stripe"
    }
  ],
  "title_allowlist": ["Security Engineer", "DevSecOps"],
  "keywords_boost": ["Zero Trust", "Kubernetes", "AWS"],
  "location_constraints": ["Remote", "US"],
  "salary_floor_usd": 150000,
  "immediate_alert_threshold": 0.9
}
```

## 🧠 AI Enhancement (Optional)

Enable ChatGPT for smarter job matching:

```bash
# In .env file
LLM_ENABLED=true
OPENAI_API_KEY=sk-your-api-key-here
```

**Cost**: ~$5/month for typical usage with built-in cost controls.

## 🎮 Usage

```bash
# Run job search and send alerts
python agent.py --mode poll

# Send daily digest email
python agent.py --mode digest

# Test notification setup
python agent.py --mode test

# System health check
python agent.py --mode health

# Clean up old data
python agent.py --mode cleanup
```

## 📊 System Health & Monitoring

Built-in health monitoring tracks:
- CPU, memory, and disk usage
- Database integrity and job statistics
- Network connectivity and API status
- Log file growth and error rates

```bash
python agent.py --mode health
```

## 🛡️ Enterprise Features

- **Database Resilience**: Automatic backups and corruption recovery
- **Network Resilience**: Exponential backoff and domain-specific failure tracking
- **Process Management**: Prevents multiple instances with file locking
- **Health Monitoring**: 11-metric system status with automated alerts
- **Token Management**: ChatGPT API cost controls and rate limiting
- **Cross-Platform**: Works on Windows, macOS, and Linux

## 📁 Project Structure

```
job-private-scraper-filter/
├── agent.py                 # Main application entry point
├── database.py             # SQLite database management
├── requirements.txt        # Python dependencies
├── setup_windows.ps1       # Windows automated setup
├── setup.sh               # macOS/Linux setup script
├── docs/                  # Documentation
│   ├── WINDOWS_SETUP_GUIDE.md
│   └── ChatGPT-Integration.md
├── sources/               # Job board scrapers
│   ├── greenhouse.py
│   ├── lever.py
│   ├── workday.py
│   └── generic_js.py
├── utils/                 # Core utilities
│   ├── config.py         # Configuration management
│   ├── resilience.py     # Error handling & recovery
│   ├── health.py         # System monitoring
│   ├── llm.py           # ChatGPT integration
│   └── scraping.py      # Web scraping utilities
├── notify/               # Notification systems
│   ├── slack.py
│   └── emailer.py
└── matchers/            # Job scoring algorithms
    └── rules.py
```

## 🔒 Security & Privacy

- **No External Dependencies**: Runs entirely on your machine
- **Environment Variables**: All secrets stored in local `.env` file
- **Rate Limiting**: Respects website terms of service
- **No Telemetry**: Zero data collection or tracking
- **Open Source**: Full transparency, inspect every line of code

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🆘 Support

- **Documentation**: See `docs/` directory for detailed guides
- **Health Check**: Run `python agent.py --mode health` for diagnostics
- **Logs**: Check `data/logs/` for detailed error information
- **Issues**: Report bugs via GitHub Issues

---

<div align="center">

**Made with ❤️ for job seekers everywhere**

*Star ⭐ this repo if it helps you land your dream job!*

</div>

