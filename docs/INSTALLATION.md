# Installation Guide

This guide provides detailed installation instructions for all supported platforms.

## 📋 Prerequisites

- **Python 3.11 or higher** (3.12 recommended for best compatibility)
- **Git** (for cloning the repository)
- **Internet connection** (for downloading dependencies and browser drivers)

## 🪟 Windows Installation

### Option 1: Automated Setup (Recommended)

1. **Open PowerShell as Administrator**:
   - Press `Windows + X`
   - Select "Windows PowerShell (Admin)" or "Terminal (Admin)"
   - Click "Yes" when prompted

2. **Run the setup command**:
   ```powershell
   Set-ExecutionPolicy Bypass -Scope Process -Force; [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072; irm "https://raw.githubusercontent.com/cboyd0319/job-private-scraper-filter/main/setup_windows.ps1" | iex
   ```

3. **Wait for completion**: The script will install Python, dependencies, and set up automated scheduling.

### Option 2: Manual Installation

1. **Install Python 3.11+**:
   - Download from [python.org](https://www.python.org/downloads/)
   - ✅ Check "Add Python to PATH" during installation

2. **Clone the repository**:
   ```powershell
   git clone https://github.com/cboyd0319/job-private-scraper-filter.git
   cd job-private-scraper-filter
   ```

3. **Set up virtual environment**:
   ```powershell
   python -m venv .venv
   .\.venv\Scripts\Activate.ps1
   ```

4. **Install dependencies**:
   ```powershell
   pip install -r requirements.txt
   python -m playwright install chromium
   ```

5. **Configure settings**:
   ```powershell
   copy .env.example .env
   copy user_prefs.example.json user_prefs.json
   # Edit these files with your settings
   ```

## 🍎 macOS Installation

### Option 1: Automated Setup

```bash
git clone https://github.com/cboyd0319/job-private-scraper-filter.git
cd job-private-scraper-filter
chmod +x setup.sh && ./setup.sh
```

### Option 2: Manual Installation

1. **Install Python 3.11+**:
   ```bash
   # Using Homebrew (recommended)
   brew install python@3.12

   # Or download from python.org
   ```

2. **Clone and set up**:
   ```bash
   git clone https://github.com/cboyd0319/job-private-scraper-filter.git
   cd job-private-scraper-filter

   python3 -m venv .venv
   source .venv/bin/activate

   pip install -r requirements.txt
   python -m playwright install chromium
   ```

3. **Configure**:
   ```bash
   cp .env.example .env
   cp user_prefs.example.json user_prefs.json
   # Edit these files with your settings
   ```

## 🐧 Linux Installation

### Ubuntu/Debian

1. **Install system dependencies**:
   ```bash
   sudo apt update
   sudo apt install python3.11 python3.11-venv python3-pip git
   ```

2. **Clone and set up**:
   ```bash
   git clone https://github.com/cboyd0319/job-private-scraper-filter.git
   cd job-private-scraper-filter

   python3 -m venv .venv
   source .venv/bin/activate

   pip install -r requirements.txt
   python -m playwright install chromium
   ```

### CentOS/RHEL/Fedora

1. **Install system dependencies**:
   ```bash
   # Fedora
   sudo dnf install python3.11 python3-pip git

   # CentOS/RHEL
   sudo yum install python3.11 python3-pip git
   ```

2. **Follow the same setup steps as Ubuntu**

## 🔧 Post-Installation Setup

### 1. Configuration

Edit the configuration files:

```bash
# Environment variables (secrets)
nano .env  # or your preferred editor

# Job search preferences
nano user_prefs.json
```

### 2. Test Installation

```bash
# Test basic functionality
python agent.py --mode health

# Test notifications (will show errors with dummy credentials - this is expected)
python agent.py --mode test
```

### 3. Set Up Automation

#### Windows (Task Scheduler)
The automated setup script creates scheduled tasks automatically. To manage them:
1. Press `Windows + R`, type `taskschd.msc`
2. Look for tasks starting with "JobScraper"

#### macOS/Linux (Cron)
Add to crontab:
```bash
crontab -e

# Add these lines (adjust paths as needed):
# Run every 15 minutes
*/15 * * * * cd /path/to/job-private-scraper-filter && .venv/bin/python agent.py --mode poll

# Send daily digest at 9 AM
0 9 * * * cd /path/to/job-private-scraper-filter && .venv/bin/python agent.py --mode digest

# Weekly cleanup on Sundays at 2 AM
0 2 * * 0 cd /path/to/job-private-scraper-filter && .venv/bin/python agent.py --mode cleanup
```

## 🚨 Troubleshooting

### Common Issues

**Python not found**:
- Ensure Python is in your PATH
- Try `python3` instead of `python`
- Reinstall Python with "Add to PATH" option

**Permission errors**:
- Use virtual environments to avoid system-wide installations
- On Linux/macOS, ensure you have write permissions to the project directory

**Playwright installation fails**:
- Run `python -m playwright install-deps` for system dependencies
- Check internet connection and firewall settings

**Import errors**:
- Activate your virtual environment: `.venv/bin/activate` or `.venv\Scripts\Activate.ps1`
- Reinstall dependencies: `pip install -r requirements.txt`

### Getting Help

1. **Check the logs**: `data/logs/scraper_YYYYMMDD.log`
2. **Run health check**: `python agent.py --mode health`
3. **Create an issue**: [GitHub Issues](https://github.com/cboyd0319/job-private-scraper-filter/issues)

## 🔄 Updating

To update to the latest version:

```bash
git pull origin main
pip install -r requirements.txt --upgrade
python -m playwright install chromium
```

## 🗑️ Uninstallation

### Windows
1. Remove scheduled tasks from Task Scheduler
2. Delete the project folder
3. Optionally uninstall Python if not needed for other projects

### macOS/Linux
1. Remove cron jobs: `crontab -e`
2. Delete the project folder: `rm -rf job-private-scraper-filter`

---

For detailed platform-specific guides, see:
- [Windows Setup Guide](WINDOWS_SETUP_GUIDE.md)
- [ChatGPT Integration](ChatGPT-Integration.md)
