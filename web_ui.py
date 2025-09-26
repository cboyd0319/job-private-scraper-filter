import os
import json
from flask import Flask, render_template, request, redirect, url_for, flash
from utils.config import config_manager
from database import get_database_stats, engine
from sqlmodel import Session, select, Job

# --- Flask App Initialization ---
app = Flask(__name__)
app.secret_key = os.urandom(24)

# --- Helper Functions ---
def read_logs(lines=100):
    """Reads the last N lines from the log file."""
    try:
        log_file = max([p for p in (config_manager.config_path.parent / "data/logs").glob("*.log")], key=os.path.getctime)
        with open(log_file, 'r', encoding='utf-8') as f:
            log_lines = f.readlines()
            return "".join(log_lines[-lines:])
    except Exception:
        return "No log file found."

# --- App Routes ---
@app.route("/")
def index():
    """Main dashboard page."""
    try:
        prefs = config_manager.load_config()
        db_stats = get_database_stats()
        with Session(engine) as session:
            recent_jobs = session.exec(select(Job).order_by(Job.created_at.desc()).limit(10)).all()
        return render_template("index.html", prefs=prefs, stats=db_stats, jobs=recent_jobs)
    except Exception as e:
        flash(f"Error loading dashboard: {e}", "danger")
        return render_template("index.html", prefs={}, stats={}, jobs=[])

@app.route("/save", methods=["POST"])
def save_config():
    """Saves the user_prefs.json configuration."""
    try:
        # Get the form data as a string
        config_str = request.form.get("config")
        if not config_str:
            flash("Configuration data was empty.", "danger")
            return redirect(url_for("index"))

        # Validate JSON
        try:
            config_data = json.loads(config_str)
        except json.JSONDecodeError:
            flash("Invalid JSON format. Please check your syntax.", "danger")
            return redirect(url_for("index"))
        
        # Write to file
        with open(config_manager.config_path, "w", encoding='utf-8') as f:
            json.dump(config_data, f, indent=2)

        flash("Configuration saved successfully!", "success")
    except Exception as e:
        flash(f"Error saving configuration: {e}", "danger")
    
    return redirect(url_for("index"))

@app.route("/logs")
def logs():
    """Displays the log viewer page."""
    log_content = read_logs(lines=500)
    return render_template("logs.html", log_content=log_content)

if __name__ == "__main__":
    print("🚀 Starting Job Scraper Web UI...")
    print("View and edit your configuration at http://127.0.0.1:5000")
    app.run(debug=True, port=5000)