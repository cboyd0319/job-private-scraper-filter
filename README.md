# Private Job Scraper & Filter

A robust, private job monitoring service that runs on your own machine. It features a rule-based scoring system, support for JavaScript-heavy pages, a persistent database, and alerts via Slack and email.

## ⭐ Features

* **Multi-Platform Support**: Works on both Windows 11 and macOS.
* **Intelligent Scraping**: Handles both simple job boards and complex, JavaScript-rendered career pages.
* **Customizable Scoring**: Uses a powerful rule-based system to score and filter jobs based on your unique preferences.
* **Web Interface**: An easy-to-use web dashboard to configure settings, view stats, and monitor logs.
* **Resilient by Design**: Built-in self-healing capabilities, including database backups, integrity checks, and process locks to ensure stable, long-term operation.
* **Notification System**: Delivers immediate alerts for high-scoring jobs via Slack and sends a daily digest email.
* **(Optional) AI Enhancement**: Can be integrated with the ChatGPT API for even smarter, AI-assisted job scoring and summaries.

---

## 🚀 Quick Start: Windows 11

This guide is designed to be as simple as possible.

### Step 1: Open PowerShell as an Administrator

1.  Click the **Start** button, type `PowerShell`.
2.  Right-click on **Windows PowerShell** and select **"Run as administrator"**.
3.  If prompted, click **Yes** to allow the app to make changes.

### Step 2: Run the Setup Script

Copy the full command below. Right-click inside the blue PowerShell window to paste it, then press **Enter**. This will download the project, install all dependencies, and configure the scheduled tasks.

```powershell
Set-ExecutionPolicy Bypass -Scope Process -Force; [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072; iex ((New-Object System.Net.WebClient).DownloadString('https://raw.githubusercontent.com/cboyd0319/job-private-scraper-filter/main/setup_windows.ps1'))

The process may take several minutes. It's finished when you see a "🎉 Setup completed successfully!" message.

Step 3: Configure Your Settings
The script will create a job-scraper folder on your Desktop.

Edit .env for Notifications: Open the .env file in Notepad and fill in your details for Slack and/or Email alerts.

Edit user_prefs.json for Job Searches: Open user_prefs.json in Notepad to control what jobs the agent looks for. Change the companies, keywords, and other filters to match your preferences.

Step 4: Test Everything
On your Desktop, you will find two new shortcuts:

"Test Job Scraper": Double-click this to send a test notification and confirm your settings are correct.

"Run Job Scraper": Double-click this to perform a manual scan for jobs.

The agent is now scheduled to run automatically in the background.

------

🍎 Quick Start: macOS
Step 1: Open Your Terminal
Open the Terminal application on your Mac.

Step 2: Run the Setup Script
Copy and paste the following command into your Terminal and press Enter. This will download the project and run the setup script.

git clone [https://github.com/cboyd0319/job-private-scraper-filter.git](https://github.com/cboyd0319/job-private-scraper-filter.git) && cd job-private-scraper-filter && chmod +x setup.sh && ./setup.sh

The script will guide you through installing dependencies and will provide you with the cron commands to copy for full automation.

Step 3: Configure and Test
Edit .env and user_prefs.json with your notification secrets and job preferences.

Activate the virtual environment: source .venv/bin/activate

Run a test: python agent.py --mode test

🖥️ Usage
Web Interface (Recommended)
The agent includes a web dashboard for easy configuration and monitoring.

Activate your virtual environment:

Windows: .\.venv\Scripts\Activate.ps1

macOS: source .venv/bin/activate

Run the web UI: python web_ui.py

Open your browser to http://127.0.0.1:5000.

Command Line Interface

For advanced use, you can run the agent directly from the command line:

# Poll job boards and send alerts
python agent.py --mode poll

# Send the daily email digest
python agent.py --mode digest

# Test your notification channels
python agent.py --mode test

# Clean up old database entries
python agent.py --mode cleanup

# Check the system's operational health
python agent.py --mode health

🔒 Windows Operating Guide & Security Considerations
This section covers important configurations for running the agent smoothly and securely in a Windows environment.

Dedicated User Account for Security
For the best security, it's highly recommended that you run this agent under a separate, non-administrator local user account. This ensures the script has only the minimum permissions required to run.

Create a New Local User: Go to Settings > Accounts > Family & other users and add a user without a Microsoft account. A name like jobsbot is recommended.

Install as the New User: Log into Windows with the new account and follow the Quick Start guide. This correctly associates all files and tasks with this limited-access user.

Windows Task Scheduler
The setup script creates three automated tasks. Here are some key considerations:

Running While Logged Off: By default, tasks only run when you are logged on. To ensure the agent runs continuously on an always-on machine, you should change this setting.

How to Change: Open Task Scheduler, find the "JobScraper-Poll" task, and open its Properties. On the General tab, select "Run whether user is logged on or not." You will be prompted for your password to allow the task to run in the background.

Network Connection: In the task's Properties, under the Conditions tab, you can require a network connection for the task to start.

Resilience and Self-Healing
The agent has several built-in features to ensure reliability:

Process Locking: The script creates a scraper.lock file in the data directory to prevent multiple instances from running simultaneously. If the script crashes, this file is automatically removed on the next run.

Database Backups: The system automatically creates backups of the jobs.sqlite database in the data/backups folder to protect against data corruption.

Integrity Checks: On every startup, the script checks the database for corruption. If an issue is found, the interactive --mode health command will help you restore from a backup.

Other Windows-Specific Notes
Firewall: The first time the script runs, Windows Defender Firewall may ask for permission. You must Allow access for Python.

Anti-Virus: In rare cases, aggressive anti-virus software may flag the script. If this happens, you may need to add a security exception for the python.exe file located inside the project's .venv folder.

.env File Security: Ensure the project folder is in a secure location within your user profile to protect the API keys and passwords stored in the .env file.
