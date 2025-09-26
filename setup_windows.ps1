# PowerShell script for Windows 11 setup
# Private Job Scraper - Windows Deployment

param(
    [switch]$SkipPython,
    [switch]$SkipChocolatey,
    [string]$InstallPath = "$env:USERPROFILE\job-scraper"
)

# Colors for output
$Red = "`e[31m"
$Green = "`e[32m"
$Yellow = "`e[33m"
$Blue = "`e[34m"
$Reset = "`e[0m"

function Write-Success {
    param($Message)
    Write-Host "${Green}✅ $Message${Reset}"
}

function Write-Error {
    param($Message)
    Write-Host "${Red}❌ ERROR: $Message${Reset}" -ForegroundColor Red
}

function Write-Warning {
    param($Message)
    Write-Host "${Yellow}⚠️  WARNING: $Message${Reset}" -ForegroundColor Yellow
}

function Write-Info {
    param($Message)
    Write-Host "${Blue}ℹ️  $Message${Reset}" -ForegroundColor Blue
}

function Test-AdminRights {
    $currentUser = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($currentUser)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Install-Chocolatey {
    if (Get-Command choco -ErrorAction SilentlyContinue) {
        Write-Success "Chocolatey is already installed"
        return
    }

    Write-Info "Installing Chocolatey package manager..."
    Set-ExecutionPolicy Bypass -Scope Process -Force
    [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072
    Invoke-Expression ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))

    if ($LASTEXITCODE -eq 0) {
        Write-Success "Chocolatey installed successfully"
    } else {
        Write-Error "Failed to install Chocolatey"
        exit 1
    }
}

function Install-Python {
    if (Get-Command python -ErrorAction SilentlyContinue) {
        $pythonVersion = python --version 2>&1
        if ($pythonVersion -match "Python 3\.[89]|Python 3\.1[0-9]") {
            Write-Success "Python is already installed: $pythonVersion"
            return
        }
    }

    Write-Info "Installing Python 3.11..."
    choco install python311 -y

    if ($LASTEXITCODE -eq 0) {
        Write-Success "Python 3.11 installed successfully"
        # Refresh environment variables
        $env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
    } else {
        Write-Error "Failed to install Python"
        exit 1
    }
}

function Install-Git {
    if (Get-Command git -ErrorAction SilentlyContinue) {
        Write-Success "Git is already installed"
        return
    }

    Write-Info "Installing Git..."
    choco install git -y

    if ($LASTEXITCODE -eq 0) {
        Write-Success "Git installed successfully"
        # Refresh environment variables
        $env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
    } else {
        Write-Error "Failed to install Git"
        exit 1
    }
}

function Setup-JobScraper {
    param($InstallPath)

    Write-Info "Setting up Job Scraper at $InstallPath..."

    # Create install directory
    if (!(Test-Path $InstallPath)) {
        New-Item -ItemType Directory -Path $InstallPath -Force | Out-Null
        Write-Success "Created installation directory: $InstallPath"
    }

    Set-Location $InstallPath

    # Create Python virtual environment
    Write-Info "Creating Python virtual environment..."
    python -m venv .venv

    if ($LASTEXITCODE -eq 0) {
        Write-Success "Virtual environment created"
    } else {
        Write-Error "Failed to create virtual environment"
        exit 1
    }

    # Activate virtual environment
    & ".\.venv\Scripts\Activate.ps1"

    # Upgrade pip
    python -m pip install --upgrade pip

    # Install requirements if requirements.txt exists
    if (Test-Path "requirements.txt") {
        Write-Info "Installing Python packages..."
        pip install -r requirements.txt
    } else {
        Write-Info "Installing core Python packages..."
        $packages = @(
            "playwright==1.45.1",
            "requests==2.32.3",
            "pydantic==2.8.2",
            "sqlmodel==0.0.19",
            "python-dotenv==1.0.1",
            "tenacity==8.5.0",
            "beautifulsoup4==4.12.3",
            "lxml==5.3.0",
            "aiofiles==24.1.0"
        )

        foreach ($package in $packages) {
            pip install $package
        }
    }

    if ($LASTEXITCODE -eq 0) {
        Write-Success "Python packages installed"
    } else {
        Write-Error "Failed to install Python packages"
        exit 1
    }

    # Install Playwright browsers
    Write-Info "Installing Playwright browsers..."
    python -m playwright install chromium

    if ($LASTEXITCODE -eq 0) {
        Write-Success "Playwright browsers installed"
    } else {
        Write-Warning "Playwright browser installation failed, but continuing..."
    }
}

function Setup-ConfigFiles {
    param($InstallPath)

    Write-Info "Setting up configuration files..."

    # Create .env file if it doesn't exist
    if (!(Test-Path ".env")) {
        if (Test-Path ".env.example") {
            Copy-Item ".env.example" ".env"
            Write-Success "Created .env file from example"
        } else {
            @"
# Timezone for logging and scheduling
TZ=America/New_York

# Slack Incoming Webhook URL for immediate alerts
SLACK_WEBHOOK_URL=https://hooks.slack.com/services/XXX/YYY/ZZZ

# SMTP settings for daily digest emails
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USER=your_email@example.com
SMTP_PASS=your_app_password
DIGEST_TO=recipient_email@example.com

# Logging level (DEBUG, INFO, WARNING, ERROR)
LOG_LEVEL=INFO

# Database cleanup (days to keep old jobs)
CLEANUP_DAYS=90
"@ | Out-File -FilePath ".env" -Encoding UTF8
            Write-Success "Created default .env file"
        }
    }

    # Create user_prefs.json if it doesn't exist
    if (!(Test-Path "user_prefs.json")) {
        if (Test-Path "user_prefs.example.json") {
            Copy-Item "user_prefs.example.json" "user_prefs.json"
            Write-Success "Created user_prefs.json from example"
        } else {
            @"
{
  "companies": [
    {"id":"cloudflare","board_type":"greenhouse", "url":"https://boards.greenhouse.io/cloudflare"},
    {"id":"discord","board_type":"greenhouse", "url":"https://boards.greenhouse.io/discord"}
  ],
  "title_allowlist": ["Security Engineer", "Product Security", "Application Security", "AppSec"],
  "title_blocklist": ["Director", "Manager", "VP", "Intern", "Contract"],
  "keywords_boost": ["Okta", "Zero Trust", "Kubernetes", "AWS", "IAM"],
  "keywords_exclude": ["recruiter", "sales", "marketing"],
  "location_constraints": ["Remote", "US"],
  "salary_floor_usd": 150000,
  "immediate_alert_threshold": 0.9,
  "max_matches_per_run": 10,
  "fetch_descriptions": true,
  "max_companies_per_run": 10
}
"@ | Out-File -FilePath "user_prefs.json" -Encoding UTF8
            Write-Success "Created default user_prefs.json"
        }
    }

    # Create data directory
    if (!(Test-Path "data")) {
        New-Item -ItemType Directory -Path "data" -Force | Out-Null
        New-Item -ItemType Directory -Path "data\logs" -Force | Out-Null
        Write-Success "Created data directories"
    }
}

function Setup-TaskScheduler {
    param($InstallPath)

    Write-Info "Setting up Windows Task Scheduler..."

    $pythonPath = Join-Path $InstallPath ".venv\Scripts\python.exe"
    $agentPath = Join-Path $InstallPath "agent.py"
    $logPath = Join-Path $InstallPath "data\logs\scheduler.log"

    # Create polling task (every 15 minutes)
    $pollAction = New-ScheduledTaskAction -Execute $pythonPath -Argument "$agentPath --mode poll" -WorkingDirectory $InstallPath
    $pollTrigger = New-ScheduledTaskTrigger -RepetitionInterval (New-TimeSpan -Minutes 15) -Once -At (Get-Date).AddMinutes(1)
    $pollSettings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable
    $pollPrincipal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType Interactive

    try {
        Register-ScheduledTask -TaskName "JobScraper-Poll" -Action $pollAction -Trigger $pollTrigger -Settings $pollSettings -Principal $pollPrincipal -Force | Out-Null
        Write-Success "Created polling task (every 15 minutes)"
    } catch {
        Write-Warning "Failed to create polling task: $($_.Exception.Message)"
    }

    # Create digest task (daily at 9 AM)
    $digestAction = New-ScheduledTaskAction -Execute $pythonPath -Argument "$agentPath --mode digest" -WorkingDirectory $InstallPath
    $digestTrigger = New-ScheduledTaskTrigger -Daily -At "9:00AM"

    try {
        Register-ScheduledTask -TaskName "JobScraper-Digest" -Action $digestAction -Trigger $digestTrigger -Settings $pollSettings -Principal $pollPrincipal -Force | Out-Null
        Write-Success "Created digest task (daily at 9 AM)"
    } catch {
        Write-Warning "Failed to create digest task: $($_.Exception.Message)"
    }

    # Create cleanup task (weekly)
    $cleanupAction = New-ScheduledTaskAction -Execute $pythonPath -Argument "$agentPath --mode cleanup" -WorkingDirectory $InstallPath
    $cleanupTrigger = New-ScheduledTaskTrigger -Weekly -DaysOfWeek Sunday -At "2:00AM"

    try {
        Register-ScheduledTask -TaskName "JobScraper-Cleanup" -Action $cleanupAction -Trigger $cleanupTrigger -Settings $pollSettings -Principal $pollPrincipal -Force | Out-Null
        Write-Success "Created cleanup task (weekly on Sunday)"
    } catch {
        Write-Warning "Failed to create cleanup task: $($_.Exception.Message)"
    }
}

function Create-StartupShortcuts {
    param($InstallPath)

    Write-Info "Creating startup shortcuts..."

    $shell = New-Object -ComObject WScript.Shell
    $desktop = [System.Environment]::GetFolderPath('Desktop')

    # Test shortcut
    $testShortcut = $shell.CreateShortcut("$desktop\Test Job Scraper.lnk")
    $testShortcut.TargetPath = "powershell.exe"
    $testShortcut.Arguments = "-Command `"cd '$InstallPath'; .\.venv\Scripts\python.exe agent.py --mode test; pause`""
    $testShortcut.WorkingDirectory = $InstallPath
    $testShortcut.Description = "Test Job Scraper notifications"
    $testShortcut.Save()

    # Manual run shortcut
    $runShortcut = $shell.CreateShortcut("$desktop\Run Job Scraper.lnk")
    $runShortcut.TargetPath = "powershell.exe"
    $runShortcut.Arguments = "-Command `"cd '$InstallPath'; .\.venv\Scripts\python.exe agent.py --mode poll; pause`""
    $runShortcut.WorkingDirectory = $InstallPath
    $runShortcut.Description = "Manually run Job Scraper"
    $runShortcut.Save()

    Write-Success "Created desktop shortcuts"
}

# Main execution
Write-Info "🚀 Starting Private Job Scraper setup for Windows 11..."

# Check if running as administrator
if (!(Test-AdminRights)) {
    Write-Error "This script requires administrator privileges. Please run PowerShell as Administrator."
    exit 1
}

try {
    # Install Chocolatey
    if (!$SkipChocolatey) {
        Install-Chocolatey
    }

    # Install Python
    if (!$SkipPython) {
        Install-Python
    }

    # Install Git
    Install-Git

    # Setup Job Scraper
    Setup-JobScraper -InstallPath $InstallPath

    # Setup configuration files
    Setup-ConfigFiles -InstallPath $InstallPath

    # Setup Task Scheduler
    Setup-TaskScheduler -InstallPath $InstallPath

    # Create shortcuts
    Create-StartupShortcuts -InstallPath $InstallPath

    Write-Success "🎉 Setup completed successfully!"
    Write-Info ""
    Write-Info "Next steps:"
    Write-Info "1. Edit $InstallPath\.env with your notification settings"
    Write-Info "2. Edit $InstallPath\user_prefs.json with your job preferences"
    Write-Info "3. Use desktop shortcuts to test the setup"
    Write-Info "4. Check Task Scheduler for automated runs"
    Write-Info ""
    Write-Info "Installation location: $InstallPath"

} catch {
    Write-Error "Setup failed: $($_.Exception.Message)"
    exit 1
}