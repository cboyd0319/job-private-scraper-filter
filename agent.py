import os
import argparse
from dotenv import load_dotenv

from utils.logging import setup_logging, get_logger
from utils.config import config_manager
from utils.errors import ConfigurationException, ScrapingException
from utils.health import health_monitor
from utils.resilience import run_startup_checks, db_resilience, network_resilience, process_resilience

from database import init_db, get_job_by_hash, add_job, get_jobs_for_digest, mark_jobs_digest_sent, mark_job_alert_sent, cleanup_old_jobs
from sources import greenhouse, lever, workday, generic_js
from matchers.rules import score_job
from notify import slack, emailer

# Load environment variables
load_dotenv()

# Setup logging
logger = setup_logging(log_level=os.getenv('LOG_LEVEL', 'INFO'))
main_logger = get_logger('agent')


def load_user_prefs():
    """Loads and validates user preferences."""
    try:
        return config_manager.load_config()
    except ConfigurationException as e:
        main_logger.error(f"Configuration error: {e}")
        raise
    except Exception as e:
        main_logger.error(f"Failed to load configuration: {e}")
        raise


def poll_sources(prefs):
    """Scrapes all configured sources for new jobs."""
    main_logger.info("Starting polling cycle")
    all_new_jobs = []
    total_jobs_found = 0
    errors = 0

    # Map board types to scraper functions
    scrapers = {
        "greenhouse": greenhouse.scrape,
        "lever": lever.scrape,
        "workday": workday.scrape,
        "generic_js": generic_js.scrape
    }

    companies = config_manager.get_companies()
    scraping_config = config_manager.get_scraping_config()

    for company in companies[:scraping_config.max_companies_per_run]:
        scraper_func = scrapers.get(company.board_type)
        if not scraper_func:
            main_logger.warning(f"No scraper found for board type '{company.board_type}' for {company.id}")
            continue

        # Check if domain should be skipped due to network failures
        from urllib.parse import urlparse
        domain = urlparse(company.url).netloc
        if network_resilience.should_skip_domain(domain):
            failure_count = network_resilience.get_failure_count(domain)
            main_logger.warning(f"Skipping {company.id} due to {failure_count} consecutive failures")
            continue

        try:
            main_logger.info(f"Scraping {company.id} ({company.board_type})...")

            # Call scraper with enhanced parameters
            jobs = scraper_func(
                company.url,
                fetch_descriptions=company.fetch_descriptions and scraping_config.fetch_descriptions
            )

            # Record successful scraping
            network_resilience.record_success(domain)

            new_jobs_count = 0
            for job in jobs:
                total_jobs_found += 1
                existing_job = get_job_by_hash(job['hash'])
                if not existing_job:
                    all_new_jobs.append(job)
                    new_jobs_count += 1
                    main_logger.info(f"  New job: {job['title']} at {job['location']}")
                else:
                    # Update existing job's last_seen timestamp
                    add_job(job)  # This will update the existing job

            main_logger.info(f"Completed {company.id}: {len(jobs)} total, {new_jobs_count} new")

        except ScrapingException as e:
            errors += 1
            network_resilience.record_failure(domain)
            main_logger.error(f"Scraping failed for {company.id}: {e}")
        except Exception as e:
            errors += 1
            network_resilience.record_failure(domain)
            main_logger.error(f"Unexpected error scraping {company.id}: {e}")

    main_logger.info(f"Polling completed: {total_jobs_found} jobs found, {len(all_new_jobs)} new, {errors} errors")
    return all_new_jobs


def process_jobs(jobs, prefs):
    """Scores and alerts for new jobs."""
    immediate_alerts = []
    digest_jobs = []
    processed_count = 0

    filter_config = config_manager.get_filter_config()
    notification_config = config_manager.get_notification_config()

    main_logger.info(f"Processing {len(jobs)} jobs...")

    for job in jobs:
        try:
            # Get enhanced scoring with metadata
            result = score_job(job, prefs)

            # Handle both old and new scoring formats
            if len(result) == 3:
                score, reasons, metadata = result
                job['score_metadata'] = metadata
            else:
                # Backward compatibility
                score, reasons = result
                job['score_metadata'] = {"scoring_method": "legacy"}

            job['score'] = score
            job['score_reasons'] = reasons

            if score > 0:
                # Add to database
                db_job = add_job(job)
                processed_count += 1

                if score >= filter_config.immediate_alert_threshold:
                    immediate_alerts.append(job)
                    # Mark as alert sent
                    mark_job_alert_sent(db_job.id)
                else:
                    digest_jobs.append(job)

                # Enhanced logging with metadata
                method = job['score_metadata'].get('scoring_method', 'unknown')
                tokens = job['score_metadata'].get('tokens_used', 0)
                log_msg = f"Processed job: {job['title']} (score: {score:.2f}, method: {method}"
                if tokens > 0:
                    log_msg += f", tokens: {tokens}"
                log_msg += ")"
                main_logger.debug(log_msg)
            else:
                main_logger.debug(f"Filtered out job: {job['title']} (score: {score:.2f})")

        except Exception as e:
            main_logger.error(f"Error processing job {job.get('title', 'Unknown')}: {e}")

    # Send immediate Slack alerts
    if immediate_alerts and notification_config.validate_slack():
        try:
            main_logger.info(f"Sending {len(immediate_alerts)} immediate alerts to Slack")
            slack.send_slack_alert(immediate_alerts)
        except Exception as e:
            main_logger.error(f"Failed to send Slack alerts: {e}")
    elif immediate_alerts:
        main_logger.warning(f"Have {len(immediate_alerts)} high-score jobs but Slack not configured")

    main_logger.info(f"Job processing completed: {processed_count} jobs added to database")


def send_digest():
    """Sends the daily email digest."""
    main_logger.info("Starting digest generation...")

    try:
        notification_config = config_manager.get_notification_config()

        if not notification_config.validate_email():
            main_logger.warning("Email not configured, skipping digest")
            return

        # Get jobs for digest, using the new preference
        filter_config = config_manager.get_filter_config()
        min_score = getattr(filter_config, 'digest_min_score', 0.0) # Safely get the new attribute
        digest_jobs = get_jobs_for_digest(min_score=min_score, hours_back=24)

        if not digest_jobs:
            main_logger.info("No jobs to include in digest")
            return

        # Convert to dict format expected by emailer
        jobs_data = []
        for job in digest_jobs:
            jobs_data.append({
                'title': job.title,
                'url': job.url,
                'company': job.company,
                'location': job.location,
                'score': job.score,
                'score_reasons': eval(job.score_reasons) if job.score_reasons else []
            })

        # Send digest email
        emailer.send_digest_email(jobs_data)

        # Mark jobs as digest sent
        job_ids = [job.id for job in digest_jobs]
        mark_jobs_digest_sent(job_ids)

        main_logger.info(f"Digest sent successfully with {len(digest_jobs)} jobs")

    except Exception as e:
        main_logger.error(f"Failed to send digest: {e}")
        raise


def test_notifications():
    """Sends a test message to all configured notification channels."""
    main_logger.info("Testing notification channels...")

    notification_config = config_manager.get_notification_config()

    test_job = [{
        'title': 'Test Security Engineer Position',
        'url': 'https://example.com/job/test-123',
        'company': 'TestCorp',
        'location': 'Remote (US)',
        'score': 0.95,
        'score_reasons': [
            'Title matched "Security Engineer"',
            'Location matched "Remote"',
            'Keyword boost: "Security"'
        ]
    }]

    # Test Slack
    if notification_config.validate_slack():
        try:
            slack.send_slack_alert(test_job)
            main_logger.info("✅ Slack test message sent successfully")
        except Exception as e:
            main_logger.error(f"❌ Slack test failed: {e}")
    else:
        main_logger.warning("❌ Slack not configured or invalid webhook URL")

    # Test Email
    if notification_config.validate_email():
        try:
            emailer.send_digest_email(test_job)
            main_logger.info("✅ Email test message sent successfully")
        except Exception as e:
            main_logger.error(f"❌ Email test failed: {e}")
    else:
        main_logger.warning("❌ Email not configured or missing required settings")

    main_logger.info("Notification testing completed")


def cleanup():
    """Perform database cleanup and maintenance."""
    main_logger.info("Starting cleanup tasks...")

    try:
        # Clean up old jobs (configurable, default 90 days)
        cleanup_days = int(os.getenv('CLEANUP_DAYS', '90'))
        deleted_count = cleanup_old_jobs(cleanup_days)
        main_logger.info(f"Cleanup completed: removed {deleted_count} old jobs")
    except Exception as e:
        main_logger.error(f"Cleanup failed: {e}")


def health_check():
    """Perform an interactive system health check."""
    main_logger.info("Starting interactive health check...")
    report = health_monitor.generate_health_report()
    
    # --- ANSI Colors for printing ---
    C_OK, C_WARN, C_CRIT, C_END = '\033[92m', '\033[93m', '\033[91m', '\033[0m'

    def print_metric(m):
        status_colors = {"ok": C_OK, "warning": C_WARN, "critical": C_CRIT}
        status_color = status_colors.get(m['status'], '')
        print(f"  - {m['name']:<20} | Status: {status_color}{m['status'].upper():<10}{C_END} | {m['message']}")

    print("\n--- 🏥 Job Scraper Health Report ---")
    print(f"Overall Status: {report['overall_status'].upper()}")
    print("-" * 35)
    
    for metric in report['metrics']:
        print_metric(metric)
    
    print("-" * 35)
    
    # --- Interactive Actions for Critical Issues ---
    critical_metrics = [m for m in report['metrics'] if m['status'] == 'critical']
    if critical_metrics:
        print(f"\n{C_CRIT}CRITICAL ISSUES DETECTED:{C_END}")
        
        # Check for database corruption issue
        db_integrity_issue = any(m['name'] == 'database_status' and 'Integrity check failed' in m['message'] for m in critical_metrics)
        
        if db_integrity_issue:
            from utils.resilience import db_resilience
            latest_backup = db_resilience._get_latest_backup()
            if latest_backup:
                print(f"Database integrity check failed. A recent backup is available:")
                print(f"  -> {latest_backup.name}")
                
                try:
                    response = input("Attempt to restore from this backup? (y/n): ").lower()
                    if response == 'y':
                        print("Restoring database...")
                        if db_resilience.restore_from_backup(latest_backup):
                            print(f"{C_OK}Database restored successfully.{C_END}")
                        else:
                            print(f"{C_CRIT}Database restore failed. Check logs for details.{C_END}")
                    else:
                        print("Skipping database restore.")
                except KeyboardInterrupt:
                    print("\nOperation cancelled.")
    else:
        print(f"\n{C_OK}System is healthy. No critical issues found.{C_END}")

    return report


def main():
    """Main entry point for the job scraper agent."""
    parser = argparse.ArgumentParser(
        description="Private Job Scraper Agent",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""Examples:
  %(prog)s --mode poll        # Scrape job boards and send alerts
  %(prog)s --mode digest      # Send daily digest email
  %(prog)s --mode test        # Test notification channels
  %(prog)s --mode cleanup     # Clean up old database entries
        """
    )
    parser.add_argument(
        "--mode",
        choices=["poll", "digest", "test", "cleanup", "health"],
        required=True,
        help="The mode to run the agent in"
    )
    parser.add_argument(
        "--verbose", "-v",
        action="store_true",
        help="Enable verbose logging"
    )

    args = parser.parse_args()

    # Set log level based on verbose flag
    if args.verbose:
        import logging
        logging.getLogger("job_scraper").setLevel(logging.DEBUG)
        main_logger.info("Verbose logging enabled")

    try:
        main_logger.info(f"Starting job scraper in {args.mode} mode")

        # Acquire process lock to prevent multiple instances
        if not process_resilience.acquire_lock():
            main_logger.error("Another instance is already running")
            exit(1)

        # Run startup checks and recovery
        startup_results = run_startup_checks()
        if startup_results["issues_found"]:
            main_logger.warning("Startup issues detected but continuing...")

        # Initialize database
        init_db()

        # Load and validate configuration
        prefs = load_user_prefs()

        # Execute requested mode
        if args.mode == "poll":
            new_jobs = poll_sources(prefs)
            process_jobs(new_jobs, prefs)
        elif args.mode == "digest":
            send_digest()
        elif args.mode == "test":
            test_notifications()
        elif args.mode == "cleanup":
            cleanup()
        elif args.mode == "health":
            health_check()

        main_logger.info(f"Job scraper completed successfully ({args.mode} mode)")

    except ConfigurationException as e:
        main_logger.error(f"Configuration error: {e}")
        exit(1)
    except KeyboardInterrupt:
        main_logger.info("Received interrupt signal, shutting down gracefully...")
        exit(0)
    except Exception as e:
        main_logger.error(f"Unexpected error: {e}")
        # Create emergency backup on critical failure
        try:
            db_resilience.create_backup("emergency")
            main_logger.info("Emergency database backup created")
        except Exception:
            pass
        exit(1)
    finally:
        # Always release the process lock
        process_resilience.release_lock()


if __name__ == "__main__":
    main()