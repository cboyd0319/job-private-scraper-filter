#!/bin/bash

echo "🚀 Starting the Jobs Agent setup for macOS..."

# --- Helper Functions ---
print_success() {
  echo "✅ $1"
}

print_error() {
  echo "❌ ERROR: $1" >&2
  exit 1
}

# --- Step 1: Check for Homebrew ---
if ! command -v brew &> /dev/null; then
  echo "Homebrew not found. Please install it first by following the instructions at https://brew.sh/"
  exit 1
fi
print_success "Homebrew is installed."

# --- Step 2: Install Dependencies ---
echo "Installing Python 3.11 and Git..."
brew install python@3.11 git || print_error "Failed to install dependencies with Homebrew."
print_success "Python and Git are installed."

# --- Step 3: Set up Python Virtual Environment ---
echo "Creating Python virtual environment in .venv/..."
python3 -m venv .venv || print_error "Failed to create virtual environment."
source .venv/bin/activate || print_error "Failed to activate virtual environment."
print_success "Virtual environment created and activated."

# --- Step 4: Install Python Packages ---
echo "Installing Python packages from requirements.txt..."
pip install -r requirements.txt || print_error "Failed to install Python packages."
print_success "Python packages installed."

# --- Step 5: Install Playwright Browsers ---
echo "Installing Playwright Chromium browser..."
python -m playwright install chromium || print_error "Failed to install Playwright browser."
print_success "Playwright browser installed."

# --- Step 6: Initialize Configuration Files ---
if [ ! -f ".env" ]; then
  cp .env.example .env
  print_success "Created .env file. Please edit it with your secrets."
else
  print_success ".env file already exists."
fi

if [ ! -f "user_prefs.json" ]; then
  cp user_prefs.example.json user_prefs.json
  print_success "Created user_prefs.json file. Edit it to customize your job search."
else
  print_success "user_prefs.json already exists."
fi

# --- Step 7: Create Data Directories ---
mkdir -p data/logs
touch data/jobs.sqlite
print_success "Created data directory and database file."

# --- Step 8: Cron Job Instructions ---
AGENT_PATH=$(pwd)
PYTHON_PATH="$AGENT_PATH/.venv/bin/python"

echo -e "\n🎉 Setup complete! To automate the agent, add the following lines to your crontab:"
echo "First, run 'crontab -e' to open the editor."
echo "Then, copy and paste these lines, then save and exit:"
echo ""
echo "# --- Jobs Agent ---"
echo "*/15 * * * * cd $AGENT_PATH && $PYTHON_PATH agent.py --mode poll >> $AGENT_PATH/data/logs/cron.log 2>&1"
echo "0 9 * * * cd $AGENT_PATH && $PYTHON_PATH agent.py --mode digest >> $AGENT_PATH/data/logs/cron.log 2>&1"
echo "# --- End Jobs Agent ---"
echo ""
echo "✨ Remember to edit the '.env' and 'user_prefs.json' files with your preferences!"