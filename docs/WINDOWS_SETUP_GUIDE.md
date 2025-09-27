# 🪟 **Windows 11 Setup Guide - Job Private Scraper & Filter**

This guide is written for your friend who needs **crystal-clear, step-by-step instructions** to get the job scraper running on Windows 11 with **zero technical background required**.

---

## 🎯 **What This Does**

This software will:
- **Automatically monitor job boards** every 15 minutes
- **Find jobs that match your criteria** (title, location, salary, keywords)
- **Send immediate Slack alerts** for high-scoring matches
- **Email daily digest** of all relevant jobs
- **Run completely private** on your own computer
- **Work 24/7** in the background

**No ChatGPT required** - it works perfectly without any external AI services!

---

## ⚡ **Super Quick Start (15 Minutes)**

### **Step 1: Prepare Your Computer**

1. **Update Windows** (Important!)
   - Press `Windows key + I`
   - Click "Windows Update"
   - Click "Check for updates"
   - Install any available updates and restart if needed

2. **Open PowerShell as Administrator**
   - Press `Windows key + X`
   - Click "Windows PowerShell (Admin)" or "Terminal (Admin)"
   - When it asks "Do you want to allow this app to make changes?", click **YES**
   - You should see a blue window with white text

### **Step 2: Run the Magic Setup Command**

**Copy this entire command** (click the copy button if available):

```powershell
Set-ExecutionPolicy Bypass -Scope Process -Force; [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072; irm "https://raw.githubusercontent.com/cboyd0319/job-private-scraper-filter/main/setup_windows.ps1" | iex
```

**Paste it in PowerShell**:
- Right-click in the blue PowerShell window
- Press Enter

**What happens next:**
- Downloads and installs Python 3.11
- Downloads the job scraper
- Installs all required packages
- Sets up automatic scheduling
- Creates configuration files
- **Takes 5-10 minutes** - lots of text will scroll by

**When it's done**, you'll see: `🎉 Setup completed successfully!`

### **Step 3: Configure Your Job Search**

The setup creates a folder on your Desktop called `job-scraper`. Open it and you'll find two important files:

#### **A. Edit `.env` file (for notifications)**
- Right-click `.env` → "Open with" → "Notepad"
- **For Slack alerts** (immediate notifications):
  1. Go to your Slack workspace
  2. Add the "Incoming Webhooks" app
  3. Create a webhook URL (starts with `https://hooks.slack.com/`)
  4. Paste it after `SLACK_WEBHOOK_URL=`

- **For email digest** (daily summary):
  1. Use Gmail and create an "App Password" (Google for instructions)
  2. Fill in your email settings in the `.env` file

#### **B. Edit `user_prefs.json` (what jobs to find)**
- Right-click `user_prefs.json` → "Open with" → "Notepad"
- **Change these sections:**
  - `companies`: Add job board URLs you want to monitor
  - `title_allowlist`: Job titles you want (e.g., "Software Engineer")
  - `location_constraints`: Where you want to work (e.g., "Remote", "New York")
  - `salary_floor_usd`: Minimum salary you'll accept

### **Step 4: Test Everything**

On your Desktop, you'll find these shortcuts:

1. **Double-click "Test Job Scraper"**
   - This sends a test notification to verify your setup
   - You should receive a Slack message and/or email

2. **Double-click "Run Job Scraper"**
   - This manually runs one complete job search
   - Check the output to see if it finds jobs

**If both work**: Congratulations! Your job scraper is now running automatically!

---

## 🔧 **Detailed Configuration**

### **Job Board Setup**

In `user_prefs.json`, the `companies` section tells the scraper where to look:

```json
{
  "companies": [
    {
      "id": "google",
      "board_type": "greenhouse",
      "url": "https://boards.greenhouse.io/google"
    },
    {
      "id": "stripe",
      "board_type": "lever",
      "url": "https://jobs.lever.co/stripe"
    },
    {
      "id": "cloudflare",
      "board_type": "greenhouse",
      "url": "https://boards.greenhouse.io/cloudflare"
    }
  ]
}
```

**Supported job board types:**
- `greenhouse` - boards.greenhouse.io
- `lever` - jobs.lever.co
- `workday` - Most corporate career pages
- `generic_js` - JavaScript-heavy career pages

### **Smart Filtering**

The scraper uses intelligent filtering:

```json
{
  "title_allowlist": ["Software Engineer", "Senior Developer", "Full Stack"],
  "title_blocklist": ["Manager", "Director", "Intern"],
  "keywords_boost": ["Python", "React", "Remote"],
  "location_constraints": ["Remote", "San Francisco", "New York"],
  "salary_floor_usd": 120000,
  "immediate_alert_threshold": 0.9
}
```

### **Scoring System**

Jobs are scored 0.0 to 1.0:
- **Title match**: +0.6 (required)
- **Location match**: +0.2
- **Keyword boosts**: +0.05 each
- **Salary meets minimum**: +0.1

Score ≥ 0.9 = Immediate Slack alert
Score ≥ 0.7 = Included in daily digest

---

## 📱 **Notifications Setup**

### **Slack Setup (Immediate Alerts)**

1. **In Slack**: Go to your workspace
2. **Click your workspace name** → Settings & administration → Manage apps
3. **Search for "Incoming Webhooks"** → Add to Slack
4. **Choose a channel** where you want job alerts
5. **Copy the Webhook URL** (starts with `https://hooks.slack.com/`)
6. **In `.env` file**: Paste it after `SLACK_WEBHOOK_URL=`

### **Email Setup (Daily Digest)**

**For Gmail:**
1. **Enable 2-Factor Authentication** on your Google account
2. **Generate App Password**:
   - Go to Google Account settings
   - Security → 2-Step Verification → App passwords
   - Generate password for "Mail"
3. **In `.env` file**:
   ```
   SMTP_HOST=smtp.gmail.com
   SMTP_PORT=587
   SMTP_USER=your_email@gmail.com
   SMTP_PASS=your_app_password_here
   DIGEST_TO=your_email@gmail.com
   ```

**For Outlook/Hotmail:**
```
SMTP_HOST=smtp-mail.outlook.com
SMTP_PORT=587
SMTP_USER=your_email@outlook.com
SMTP_PASS=your_password
DIGEST_TO=your_email@outlook.com
```

---

## 🕒 **Automatic Scheduling**

The setup creates these automatic tasks:

- **Every 15 minutes**: Check for new jobs → Send immediate alerts
- **Daily at 9 AM**: Send email digest of all matched jobs
- **Weekly on Sunday**: Clean up old database entries
- **Every 6 hours**: Health check and system monitoring

**To view/modify schedule:**
1. Press `Windows key + R`
2. Type `taskschd.msc` and press Enter
3. Look for tasks starting with "JobScraper"

---

## 🔍 **Monitoring & Troubleshooting**

### **Check if it's working:**

1. **Desktop shortcuts**: Use "Test Job Scraper" anytime
2. **Log files**: `Desktop/job-scraper/data/logs/` - check for errors
3. **Database**: `Desktop/job-scraper/data/jobs.sqlite` - grows as jobs are found

### **Common Issues:**

**"No jobs found"**
- Check your `title_allowlist` - might be too specific
- Verify company URLs are correct
- Run "Test Job Scraper" to check for errors

**"Notifications not working"**
- Run "Test Job Scraper" - check error messages
- Verify `.env` file has correct credentials
- Check Slack webhook URL format

**"Script not running automatically"**
- Open Task Scheduler (`taskschd.msc`)
- Check if JobScraper tasks are there and enabled
- Right-click a task → "Run" to test manually

**"Getting too many alerts"**
- Increase `immediate_alert_threshold` (try 0.95)
- Add more words to `title_blocklist`
- Reduce number of companies

### **Getting Help:**

1. **Check logs first**: `Desktop/job-scraper/data/logs/scraper_YYYYMMDD.log`
2. **Run health check**: Open PowerShell in the job-scraper folder, run:
   ```powershell
   .\.venv\Scripts\python.exe agent.py --mode health
   ```
3. **Manual test run**:
   ```powershell
   .\.venv\Scripts\python.exe agent.py --mode test --verbose
   ```

---

## 🚀 **Optional: ChatGPT Integration (Advanced)**

If you want AI-enhanced job matching:

1. **Get OpenAI API key**: Go to platform.openai.com
2. **Edit `.env` file**, uncomment and fill:
   ```
   LLM_ENABLED=true
   OPENAI_API_KEY=sk-your-key-here
   ```
3. **Install AI packages**: In PowerShell (job-scraper folder):
   ```powershell
   .\.venv\Scripts\pip.exe install openai tiktoken
   ```

**Cost**: ~$5/month for typical usage

**Benefits**: Better job matching, AI summaries, "why this fits you" analysis

---

## 🔒 **Security & Privacy**

- **Runs 100% on your computer** - no external dependencies
- **Your data never leaves your machine** (except for configured notifications)
- **No telemetry or tracking**
- **API keys stored securely** in local `.env` file
- **Rate-limited scraping** respects website terms

---

## 📊 **Advanced Features**

### **Multiple Job Searches**
You can create different configurations for different job types:
1. Copy `user_prefs.json` to `user_prefs_fulltime.json`
2. Create different search criteria in each file
3. Run manually: `python agent.py --mode poll --config user_prefs_fulltime.json`

### **Salary Extraction**
The scraper automatically detects salary information in job descriptions and filters based on your `salary_floor_usd`.

### **Duplicate Detection**
Jobs are automatically deduplicated - you'll never get the same job alert twice.

### **Database Management**
- **Automatic cleanup**: Old jobs removed after 90 days
- **Backup system**: Automatic daily backups in `data/backups/`
- **Health monitoring**: System checks itself for issues

---

## 🛠 **Manual Installation (If Script Fails)**

If the automatic script doesn't work:

### **Step 1: Install Python 3.11**
1. Go to python.org
2. Download Python 3.11.x for Windows
3. **Important**: Check "Add Python to PATH" during installation

### **Step 2: Download Project**
1. Download the project ZIP file
2. Extract to `C:\job-scraper\`

### **Step 3: Setup**
Open PowerShell in `C:\job-scraper\` and run:
```powershell
python -m venv .venv
.\.venv\Scripts\Activate.ps1
pip install -r requirements.txt
python -m playwright install chromium
```

### **Step 4: Configure**
1. Copy `.env.example` to `.env` and edit
2. Copy `user_prefs.example.json` to `user_prefs.json` and edit

### **Step 5: Test**
```powershell
python agent.py --mode test
```

### **Step 6: Schedule**
Create Windows scheduled tasks manually or use the provided PowerShell commands in `setup_windows.ps1`.

---

## 📞 **Support**

This system is designed to be **completely self-sufficient**. If something goes wrong:

1. **Check the logs** first (they're very detailed)
2. **Try the desktop shortcuts** to test individual components
3. **Use the health check** to diagnose issues
4. **The system auto-recovers** from most problems

**Remember**: This runs completely on your computer, so it works as long as your computer is on and connected to the internet!

---

**🎉 Enjoy your automated job hunting! The system will find opportunities while you sleep!** 🎯