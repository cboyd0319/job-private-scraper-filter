import os
import smtplib
from email.mime.text import MIMEText
from email.mime.multipart import MIMEMultipart

def format_jobs_for_email(jobs: list[dict]) -> str:
    """Formats a list of jobs into an HTML email body with AI insights."""

    # Email header with stats
    total_jobs = len(jobs)
    llm_jobs = len([j for j in jobs if j.get('score_metadata', {}).get('llm_used', False)])

    html = f"""
    <html>
    <head>
        <style>
            body {{ font-family: Arial, sans-serif; line-height: 1.6; color: #333; }}
            .header {{ background: #f8f9fa; padding: 20px; border-radius: 5px; margin-bottom: 20px; }}
            .job {{ margin-bottom: 25px; border-left: 4px solid #007bff; padding-left: 15px; }}
            .job-title {{ margin: 0 0 10px 0; color: #007bff; }}
            .job-meta {{ color: #666; margin: 5px 0; }}
            .score-high {{ color: #28a745; font-weight: bold; }}
            .score-medium {{ color: #ffc107; font-weight: bold; }}
            .ai-insights {{ background: #e8f4fd; padding: 10px; border-radius: 4px; margin: 10px 0; }}
            .reasons {{ background: #f8f9fa; padding: 8px; border-radius: 4px; font-size: 0.9em; }}
        </style>
    </head>
    <body>
        <div class="header">
            <h2>🎯 Daily Job Digest</h2>
            <p>Found {total_jobs} matching job{'s' if total_jobs != 1 else ''}"""

    if llm_jobs > 0:
        html += f" • {llm_jobs} enhanced with AI analysis"

    html += "</p></div>"

    # Sort jobs by score (highest first)
    sorted_jobs = sorted(jobs, key=lambda x: x.get('score', 0), reverse=True)

    for job in sorted_jobs:
        score_percent = int(job.get('score', 0) * 100)
        metadata = job.get('score_metadata', {})
        llm_used = metadata.get('llm_used', False)

        # Determine score class for styling
        score_class = "score-high" if score_percent >= 80 else "score-medium"

        html += f"""
        <div class="job">
            <h3 class="job-title"><a href="{job['url']}">{job['title']}</a></h3>
            <div class="job-meta">
                <strong>Company:</strong> {job['company'].title()} |
                <strong>Location:</strong> {job['location']}
            </div>
        """

        # Enhanced scoring display
        if llm_used:
            rules_score = metadata.get('rules_score', 0)
            llm_score = metadata.get('llm_score', 0)
            html += f"""
            <div class="job-meta">
                <span class="{score_class}">Overall Score: {score_percent}%</span>
                (Rules: {int(rules_score*100)}% • AI: {int(llm_score*100)}%)
            </div>
            """

            # Add AI summary
            llm_summary = metadata.get('llm_summary', '')
            if llm_summary:
                html += f"""
                <div class="ai-insights">
                    <strong>🤖 AI Summary:</strong> {llm_summary}
                </div>
                """
        else:
            html += f"""
            <div class="job-meta">
                <span class="{score_class}">Match Score: {score_percent}%</span>
            </div>
            """

        # Organize and display reasons
        reasons = job.get('score_reasons', [])
        if reasons:
            rules_reasons = [r.replace('Rules: ', '') for r in reasons if r.startswith('Rules:')]
            ai_reasons = [r.replace('AI: ', '') for r in reasons if r.startswith('AI:')]
            other_reasons = [r for r in reasons if not r.startswith(('Rules:', 'AI:', 'Summary:'))]

            html += '<div class="reasons">'

            if rules_reasons or other_reasons:
                all_rules = rules_reasons + other_reasons
                html += f"<strong>✅ Matched:</strong> {', '.join(all_rules)}<br>"

            if ai_reasons:
                html += f"<strong>🧠 AI Insights:</strong> {', '.join(ai_reasons)}"

            html += '</div>'

        # Add job description snippet if available
        description = job.get('description', '')
        if description and len(description) > 100:
            snippet = description[:400] + "..." if len(description) > 400 else description
            html += f"""
            <div style="margin-top: 10px; font-size: 0.9em; color: #666;">
                <strong>Description:</strong> {snippet}
            </div>
            """

        html += "</div>"

    # Footer with timestamp and usage stats
    from datetime import datetime
    html += f"""
        <div style="margin-top: 30px; padding-top: 20px; border-top: 1px solid #eee; color: #666; font-size: 0.8em;">
            Generated on {datetime.now().strftime('%Y-%m-%d at %H:%M:%S')}
        </div>
    </body>
    </html>
    """

    return html

def send_digest_email(jobs: list[dict]):
    """Sends a digest email with the given jobs."""
    smtp_host = os.getenv("SMTP_HOST")
    smtp_port = int(os.getenv("SMTP_PORT", 587))
    smtp_user = os.getenv("SMTP_USER")
    smtp_pass = os.getenv("SMTP_PASS")
    digest_to = os.getenv("DIGEST_TO")

    if not all([smtp_host, smtp_port, smtp_user, smtp_pass, digest_to]):
        print("Warning: SMTP environment variables not fully set. Cannot send email.")
        return

    message = MIMEMultipart("alternative")
    message["Subject"] = "Your Daily Job Digest"
    message["From"] = smtp_user
    message["To"] = digest_to
    
    html_body = format_jobs_for_email(jobs)
    message.attach(MIMEText(html_body, "html"))

    try:
        with smtplib.SMTP(smtp_host, smtp_port) as server:
            server.starttls()
            server.login(smtp_user, smtp_pass)
            server.sendmail(smtp_user, digest_to, message.as_string())
        print("Digest email sent successfully.")
    except Exception as e:
        print(f"Error sending email: {e}")