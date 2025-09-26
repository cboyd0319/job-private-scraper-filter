"""Custom exceptions for the job scraper."""

class JobScraperException(Exception):
    """Base exception for job scraper errors."""
    pass


class ScrapingException(JobScraperException):
    """Exception raised during job scraping operations."""
    def __init__(self, company: str, url: str, message: str, original_error: Exception = None):
        self.company = company
        self.url = url
        self.original_error = original_error
        super().__init__(f"Failed to scrape {company} at {url}: {message}")


class NotificationException(JobScraperException):
    """Exception raised during notification sending."""
    def __init__(self, notification_type: str, message: str, original_error: Exception = None):
        self.notification_type = notification_type
        self.original_error = original_error
        super().__init__(f"Failed to send {notification_type} notification: {message}")


class ConfigurationException(JobScraperException):
    """Exception raised for configuration errors."""
    pass


class DatabaseException(JobScraperException):
    """Exception raised for database operations."""
    def __init__(self, operation: str, message: str, original_error: Exception = None):
        self.operation = operation
        self.original_error = original_error
        super().__init__(f"Database {operation} failed: {message}")


class RateLimitException(JobScraperException):
    """Exception raised when rate limits are exceeded."""
    def __init__(self, domain: str, retry_after: int = None):
        self.domain = domain
        self.retry_after = retry_after
        message = f"Rate limited by {domain}"
        if retry_after:
            message += f", retry after {retry_after} seconds"
        super().__init__(message)