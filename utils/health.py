"""
Health monitoring and system status reporting for the job scraper.
"""

import os
import time
import platform
import psutil
from datetime import datetime, timedelta
from typing import Dict, List, Optional
from dataclasses import dataclass

from utils.logging import get_logger
from utils.config import config_manager
from database import get_database_stats
from notify import slack

logger = get_logger("health")


@dataclass
class HealthMetric:
    """A single health metric measurement."""
    name: str
    value: float
    unit: str
    status: str  # "ok", "warning", "critical"
    threshold_warning: Optional[float] = None
    threshold_critical: Optional[float] = None
    message: Optional[str] = None


class HealthMonitor:
    """Monitors system health and performance metrics."""

    def __init__(self):
        self.start_time = time.time()
        self.last_check = None

    def check_system_resources(self) -> List[HealthMetric]:
        """Check system resources (CPU, memory, disk)."""
        metrics = []

        # CPU usage
        cpu_percent = psutil.cpu_percent(interval=1)
        cpu_status = "ok"
        if cpu_percent > 80:
            cpu_status = "critical"
        elif cpu_percent > 60:
            cpu_status = "warning"

        metrics.append(HealthMetric(
            name="cpu_usage",
            value=cpu_percent,
            unit="%",
            status=cpu_status,
            threshold_warning=60,
            threshold_critical=80,
            message=f"CPU usage: {cpu_percent:.1f}%"
        ))

        # Memory usage
        memory = psutil.virtual_memory()
        mem_percent = memory.percent
        mem_status = "ok"
        if mem_percent > 90:
            mem_status = "critical"
        elif mem_percent > 75:
            mem_status = "warning"

        metrics.append(HealthMetric(
            name="memory_usage",
            value=mem_percent,
            unit="%",
            status=mem_status,
            threshold_warning=75,
            threshold_critical=90,
            message=f"Memory usage: {mem_percent:.1f}% ({memory.used / 1024**3:.1f}GB / {memory.total / 1024**3:.1f}GB)"
        ))

        # Disk usage (where the scraper is installed)
        disk = psutil.disk_usage('.')
        disk_percent = (disk.used / disk.total) * 100
        disk_status = "ok"
        if disk_percent > 95:
            disk_status = "critical"
        elif disk_percent > 85:
            disk_status = "warning"

        metrics.append(HealthMetric(
            name="disk_usage",
            value=disk_percent,
            unit="%",
            status=disk_status,
            threshold_warning=85,
            threshold_critical=95,
            message=f"Disk usage: {disk_percent:.1f}% ({disk.used / 1024**3:.1f}GB / {disk.total / 1024**3:.1f}GB)"
        ))

        return metrics

    def check_database_health(self) -> List[HealthMetric]:
        """Check database health and statistics."""
        metrics = []

        try:
            stats = get_database_stats()

            # Total jobs
            total_jobs = stats.get('total_jobs', 0)
            metrics.append(HealthMetric(
                name="total_jobs",
                value=total_jobs,
                unit="jobs",
                status="ok",
                message=f"Total jobs in database: {total_jobs}"
            ))

            # Recent jobs (last 24h)
            recent_jobs = stats.get('recent_jobs_24h', 0)
            recent_status = "ok"
            if recent_jobs == 0:
                recent_status = "warning"
                message = "No new jobs found in last 24 hours"
            else:
                message = f"New jobs in last 24h: {recent_jobs}"

            metrics.append(HealthMetric(
                name="recent_jobs_24h",
                value=recent_jobs,
                unit="jobs",
                status=recent_status,
                message=message
            ))

            # High score jobs
            high_score_jobs = stats.get('high_score_jobs', 0)
            metrics.append(HealthMetric(
                name="high_score_jobs",
                value=high_score_jobs,
                unit="jobs",
                status="ok",
                message=f"High-scoring jobs (≥0.8): {high_score_jobs}"
            ))

        except Exception as e:
            logger.error(f"Failed to get database stats: {e}")
            metrics.append(HealthMetric(
                name="database_status",
                value=0,
                unit="status",
                status="critical",
                message=f"Database error: {e}"
            ))

        return metrics

    def check_log_files(self) -> List[HealthMetric]:
        """Check log file sizes and recent activity."""
        metrics = []

        log_dir = "data/logs"
        if not os.path.exists(log_dir):
            metrics.append(HealthMetric(
                name="log_directory",
                value=0,
                unit="status",
                status="warning",
                message="Log directory does not exist"
            ))
            return metrics

        try:
            # Check log file sizes
            total_log_size = 0
            recent_log_activity = False
            now = datetime.now()

            for log_file in os.listdir(log_dir):
                if log_file.endswith('.log'):
                    log_path = os.path.join(log_dir, log_file)
                    if os.path.isfile(log_path):
                        file_size = os.path.getsize(log_path)
                        total_log_size += file_size

                        # Check if file was modified in last hour
                        mod_time = datetime.fromtimestamp(os.path.getmtime(log_path))
                        if now - mod_time < timedelta(hours=1):
                            recent_log_activity = True

            # Log size metric
            log_size_mb = total_log_size / (1024 * 1024)
            log_size_status = "ok"
            if log_size_mb > 100:
                log_size_status = "warning"
            elif log_size_mb > 500:
                log_size_status = "critical"

            metrics.append(HealthMetric(
                name="log_size",
                value=log_size_mb,
                unit="MB",
                status=log_size_status,
                threshold_warning=100,
                threshold_critical=500,
                message=f"Total log size: {log_size_mb:.1f}MB"
            ))

            # Recent activity metric
            activity_status = "ok" if recent_log_activity else "warning"
            metrics.append(HealthMetric(
                name="log_activity",
                value=1 if recent_log_activity else 0,
                unit="status",
                status=activity_status,
                message="Recent log activity detected" if recent_log_activity else "No recent log activity"
            ))

        except Exception as e:
            logger.error(f"Failed to check log files: {e}")
            metrics.append(HealthMetric(
                name="log_check",
                value=0,
                unit="status",
                status="critical",
                message=f"Log check error: {e}"
            ))

        return metrics

    def check_configuration(self) -> List[HealthMetric]:
        """Check configuration validity and notification setup."""
        metrics = []

        try:
            # Test configuration loading
            config_manager.load_config()
            metrics.append(HealthMetric(
                name="configuration",
                value=1,
                unit="status",
                status="ok",
                message="Configuration loaded successfully"
            ))

            # Check notification setup
            notification_config = config_manager.get_notification_config()

            slack_status = "ok" if notification_config.validate_slack() else "warning"
            metrics.append(HealthMetric(
                name="slack_config",
                value=1 if notification_config.validate_slack() else 0,
                unit="status",
                status=slack_status,
                message="Slack configured" if notification_config.validate_slack() else "Slack not configured"
            ))

            email_status = "ok" if notification_config.validate_email() else "warning"
            metrics.append(HealthMetric(
                name="email_config",
                value=1 if notification_config.validate_email() else 0,
                unit="status",
                status=email_status,
                message="Email configured" if notification_config.validate_email() else "Email not configured"
            ))

        except Exception as e:
            logger.error(f"Configuration check failed: {e}")
            metrics.append(HealthMetric(
                name="configuration",
                value=0,
                unit="status",
                status="critical",
                message=f"Configuration error: {e}"
            ))

        return metrics

    def generate_health_report(self) -> Dict:
        """Generate comprehensive health report."""
        logger.info("Generating health report...")

        all_metrics = []
        all_metrics.extend(self.check_system_resources())
        all_metrics.extend(self.check_database_health())
        all_metrics.extend(self.check_log_files())
        all_metrics.extend(self.check_configuration())

        # Calculate overall status
        critical_count = len([m for m in all_metrics if m.status == "critical"])
        warning_count = len([m for m in all_metrics if m.status == "warning"])

        if critical_count > 0:
            overall_status = "critical"
        elif warning_count > 0:
            overall_status = "warning"
        else:
            overall_status = "ok"

        uptime = time.time() - self.start_time
        uptime_hours = uptime / 3600

        report = {
            "timestamp": datetime.now().isoformat(),
            "overall_status": overall_status,
            "uptime_hours": uptime_hours,
            "system_info": {
                "platform": platform.platform(),
                "python_version": platform.python_version(),
                "hostname": platform.node()
            },
            "metrics": [
                {
                    "name": m.name,
                    "value": m.value,
                    "unit": m.unit,
                    "status": m.status,
                    "message": m.message,
                    "threshold_warning": m.threshold_warning,
                    "threshold_critical": m.threshold_critical
                }
                for m in all_metrics
            ],
            "summary": {
                "total_metrics": len(all_metrics),
                "ok_count": len([m for m in all_metrics if m.status == "ok"]),
                "warning_count": warning_count,
                "critical_count": critical_count
            }
        }

        self.last_check = datetime.now()
        logger.info(f"Health report generated: {overall_status} status with {warning_count} warnings, {critical_count} critical issues")

        return report

    def send_health_alert(self, report: Dict):
        """Send health alert if there are critical issues."""
        if report["overall_status"] == "critical":
            try:
                notification_config = config_manager.get_notification_config()

                if notification_config.validate_slack():
                    critical_metrics = [m for m in report["metrics"] if m["status"] == "critical"]

                    alert_message = {
                        "blocks": [
                            {
                                "type": "header",
                                "text": {
                                    "type": "plain_text",
                                    "text": "🚨 Job Scraper Health Alert"
                                }
                            },
                            {
                                "type": "section",
                                "text": {
                                    "type": "mrkdwn",
                                    "text": f"*Critical issues detected on {report['system_info']['hostname']}*\n\n"
                                           f"*Issues:*\n" +
                                           "\n".join([f"• {m['message']}" for m in critical_metrics])
                                }
                            },
                            {
                                "type": "section",
                                "text": {
                                    "type": "mrkdwn",
                                    "text": f"*System uptime:* {report['uptime_hours']:.1f} hours\n"
                                           f"*Timestamp:* {report['timestamp']}"
                                }
                            }
                        ]
                    }

                    slack.send_slack_alert([], custom_message=alert_message)
                    logger.info("Health alert sent to Slack")

            except Exception as e:
                logger.error(f"Failed to send health alert: {e}")


# Global health monitor instance
health_monitor = HealthMonitor()