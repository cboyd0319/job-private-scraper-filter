# Private Job Scraper & Filter

A robust, private job monitoring service that runs on your own machine. It features a rule-based scoring system, support for JavaScript-heavy pages, a persistent database, and alerts via Slack and email.

---

## 🚀 Windows 11 Quick Start (Easy Mode)

This guide is designed to be as simple as possible. Just follow these steps carefully.

### Step 1: Open PowerShell as an Administrator

1.  Click the **Start** button on your taskbar.
2.  Type `PowerShell`.
3.  In the search results, right-click on **Windows PowerShell** and select **"Run as administrator"**.
    4.  A blue window will appear. If it asks "Do you want to allow this app to make changes to your device?", click **Yes**.

### Step 2: Run the Setup Script

The setup script will automatically install all required software (like Python) and configure the agent.

1.  **Copy the command below:** Click the copy button to the right.
    ```powershell
    Set-ExecutionPolicy Bypass -Scope Process -Force; [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072; iex ((New-Object System.Net.WebClient).DownloadString('[https://raw.githubusercontent.com/cboyd0319/job-private-scraper-filter/refs/heads/main/setup_windows.ps1](https://raw.githubusercontent.com/cboyd0319/job-private-scraper-filter/refs/heads/main/setup_windows.ps1)'))
    ```
    *(Note: You would replace the URL above with the actual raw URL to your `setup_windows.ps1` file on GitHub or another host.)*

2.  **Paste and Run in PowerShell:** Right-click inside the blue PowerShell window. The command should paste automatically. Press **Enter**.

3.  **Wait for it to finish.** The script will download the project, install dependencies, and set up scheduled tasks. This might take several minutes. You will see a lot of text scrolling by. It's finished when you see a "🎉 Setup completed successfully!" message.

### Step 3: Configure Your Settings

The setup script created two important configuration files on your Desktop in a folder called `job-scraper`.

1.  **Edit `.env` for Notifications:**
    * Open the `job-scraper` folder on your Desktop.
    * Right-click the `.env` file and choose "Open with" -> "Notepad".
    * Fill in your details for Slack and/or Email alerts. Your changes will be saved automatically.

2.  **Edit `user_prefs.json` for Job Searches:**
    * Right-click the `user_prefs.json` file and open it with Notepad.
    * This file controls *what* jobs the agent looks for. You can change the `companies`, `title_allowlist`, `location_constraints`, etc.

### Step 4: Test Everything

On your Desktop, you will find two new shortcuts:

1.  **Double-click "Test Job Scraper"**. This will run a quick test and send a notification to Slack and/or your email. This confirms your settings in the `.env` file are correct.
2.  **Double-click "Run Job Scraper"**. This will manually start one full scan for jobs. You can use this if you don't want to wait for the automated schedule.

That's it! The agent is now running automatically in the background and will check for jobs every 15 minutes. You will get a daily digest email and immediate Slack alerts for high-scoring jobs.

---

## 🔧 Command Line Interface

For advanced users, you can run the agent from the command line.

```bash
# Poll job boards and send alerts
python agent.py --mode poll

# Send daily email digest
python agent.py --mode digest

# Test notification channels
python agent.py --mode test

# Clean up old database entries
python agent.py --mode cleanup

# Check system health
python agent.py --mode health

