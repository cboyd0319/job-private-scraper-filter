import logging
import sys
from datetime import datetime
from pathlib import Path


class ColoredFormatter(logging.Formatter):
    """Custom formatter with colors for console output."""

    COLORS = {
        'DEBUG': '\033[36m',    # Cyan
        'INFO': '\033[32m',     # Green
        'WARNING': '\033[33m',  # Yellow
        'ERROR': '\033[31m',    # Red
        'CRITICAL': '\033[35m', # Magenta
        'RESET': '\033[0m'      # Reset
    }

    def format(self, record):
        if hasattr(record, 'no_color') and record.no_color:
            return super().format(record)

        color = self.COLORS.get(record.levelname, self.COLORS['RESET'])
        reset = self.COLORS['RESET']

        # Add color to the level name only
        record.levelname = f"{color}{record.levelname}{reset}"
        return super().format(record)


def setup_logging(log_level: str = "INFO", log_to_file: bool = True) -> logging.Logger:
    """
    Sets up comprehensive logging for the job scraper.

    Args:
        log_level: Logging level (DEBUG, INFO, WARNING, ERROR, CRITICAL)
        log_to_file: Whether to log to file in addition to console

    Returns:
        Configured logger instance
    """
    # Create logs directory if it doesn't exist
    log_dir = Path("data/logs")
    log_dir.mkdir(parents=True, exist_ok=True)

    # Get root logger
    logger = logging.getLogger("job_scraper")
    logger.setLevel(getattr(logging, log_level.upper()))

    # Clear existing handlers to avoid duplicates
    logger.handlers.clear()

    # Console handler with colors
    console_handler = logging.StreamHandler(sys.stdout)
    console_formatter = ColoredFormatter(
        fmt='%(asctime)s | %(levelname)s | %(name)s:%(lineno)d | %(message)s',
        datefmt='%H:%M:%S'
    )
    console_handler.setFormatter(console_formatter)
    logger.addHandler(console_handler)

    if log_to_file:
        # File handler for all logs
        log_file = log_dir / f"scraper_{datetime.now().strftime('%Y%m%d')}.log"
        file_handler = logging.FileHandler(log_file, encoding='utf-8')
        file_formatter = logging.Formatter(
            fmt='%(asctime)s | %(levelname)s | %(name)s:%(lineno)d | %(message)s',
            datefmt='%Y-%m-%d %H:%M:%S'
        )
        file_handler.setFormatter(file_formatter)
        logger.addHandler(file_handler)

        # Separate error log file
        error_file = log_dir / f"errors_{datetime.now().strftime('%Y%m%d')}.log"
        error_handler = logging.FileHandler(error_file, encoding='utf-8')
        error_handler.setLevel(logging.ERROR)
        error_handler.setFormatter(file_formatter)
        logger.addHandler(error_handler)

    return logger


def get_logger(name: str) -> logging.Logger:
    """Get a child logger with the given name."""
    return logging.getLogger(f"job_scraper.{name}")


def log_exception(logger: logging.Logger, message: str = "An error occurred"):
    """Log an exception with full traceback."""
    logger.exception(message)


def log_performance(logger: logging.Logger, operation: str, duration: float, extra_info: dict = None):
    """Log performance metrics for operations."""
    info = {"operation": operation, "duration_ms": round(duration * 1000, 2)}
    if extra_info:
        info.update(extra_info)

    logger.info(f"Performance: {operation} completed in {duration:.3f}s", extra=info)


def log_scrape_result(logger: logging.Logger, company: str, jobs_found: int, new_jobs: int, errors: int = 0):
    """Log scraping results in a standardized format."""
    logger.info(
        f"Scrape completed for {company}: {jobs_found} total jobs, {new_jobs} new, {errors} errors",
        extra={
            "company": company,
            "jobs_found": jobs_found,
            "new_jobs": new_jobs,
            "errors": errors
        }
    )


def log_notification_sent(logger: logging.Logger, notification_type: str, recipient: str, job_count: int):
    """Log notification events."""
    logger.info(
        f"Notification sent: {notification_type} to {recipient} with {job_count} jobs",
        extra={
            "notification_type": notification_type,
            "recipient": recipient,
            "job_count": job_count
        }
    )