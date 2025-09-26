"""
Advanced web scraping utilities with rate limiting, retries, and respectful practices.
"""

import time
import hashlib
import asyncio
from dataclasses import dataclass, field
from typing import Dict, Optional
from urllib.parse import urlparse
from datetime import datetime, timedelta

import requests
from playwright.async_api import async_playwright, Browser
from tenacity import retry, stop_after_attempt, wait_exponential, retry_if_exception_type

from utils.logging import get_logger
from utils.errors import ScrapingException, RateLimitException

logger = get_logger("scraping")


@dataclass
class RateLimitConfig:
    """Configuration for rate limiting per domain."""
    requests_per_minute: int = 30
    min_delay_seconds: float = 2.0
    max_delay_seconds: float = 10.0
    respect_robots_txt: bool = True


@dataclass
class DomainStats:
    """Track stats for a domain."""
    last_request_time: datetime = field(default_factory=datetime.now)
    request_count: int = 0
    failed_requests: int = 0
    rate_limited: bool = False


class RateLimiter:
    """Intelligent rate limiter that adapts to website responses."""

    def __init__(self):
        self.domain_configs: Dict[str, RateLimitConfig] = {}
        self.domain_stats: Dict[str, DomainStats] = {}
        self.request_history: Dict[str, list] = {}  # domain -> list of request timestamps

    def get_domain(self, url: str) -> str:
        """Extract domain from URL."""
        return urlparse(url).netloc.lower()

    def configure_domain(self, domain: str, config: RateLimitConfig):
        """Set custom rate limiting for a specific domain."""
        self.domain_configs[domain] = config
        logger.info(f"Configured rate limiting for {domain}: {config.requests_per_minute} req/min")

    def get_config(self, domain: str) -> RateLimitConfig:
        """Get rate limit config for domain."""
        return self.domain_configs.get(domain, RateLimitConfig())

    def should_wait(self, domain: str) -> float:
        """
        Calculate how long to wait before making a request to domain.
        Returns 0 if no wait is needed, otherwise seconds to wait.
        """
        config = self.get_config(domain)
        stats = self.domain_stats.get(domain, DomainStats())

        # Clean old requests from history (older than 1 minute)
        now = datetime.now()
        cutoff = now - timedelta(minutes=1)

        if domain not in self.request_history:
            self.request_history[domain] = []

        self.request_history[domain] = [
            ts for ts in self.request_history[domain]
            if ts > cutoff
        ]

        # Check if we're within rate limits
        recent_requests = len(self.request_history[domain])
        if recent_requests >= config.requests_per_minute:
            # Need to wait until oldest request ages out
            oldest_request = min(self.request_history[domain])
            wait_until = oldest_request + timedelta(minutes=1)
            wait_seconds = (wait_until - now).total_seconds()
            return max(0, wait_seconds)

        # Check minimum delay since last request
        if stats.last_request_time:
            time_since_last = (now - stats.last_request_time).total_seconds()
            min_wait = config.min_delay_seconds

            # Adaptive delay based on recent failures
            if stats.failed_requests > 0:
                failure_multiplier = min(2.0, 1 + (stats.failed_requests * 0.2))
                min_wait *= failure_multiplier

            if time_since_last < min_wait:
                return min_wait - time_since_last

        return 0

    def record_request(self, domain: str, success: bool = True):
        """Record a request for rate limiting tracking."""
        now = datetime.now()

        if domain not in self.domain_stats:
            self.domain_stats[domain] = DomainStats()

        stats = self.domain_stats[domain]
        stats.last_request_time = now
        stats.request_count += 1

        if not success:
            stats.failed_requests += 1
        else:
            # Reset failure count on success
            stats.failed_requests = max(0, stats.failed_requests - 1)

        # Add to request history
        if domain not in self.request_history:
            self.request_history[domain] = []
        self.request_history[domain].append(now)

    async def wait_if_needed(self, url: str):
        """Wait if needed before making request to URL."""
        domain = self.get_domain(url)
        wait_time = self.should_wait(domain)

        if wait_time > 0:
            logger.info(f"Rate limiting: waiting {wait_time:.2f}s for {domain}")
            await asyncio.sleep(wait_time)


# Global rate limiter instance
rate_limiter = RateLimiter()


class WebScraper:
    """Advanced web scraper with Playwright support and intelligent retry logic."""

    def __init__(self):
        self.browser: Optional[Browser] = None
        self.session = requests.Session()

        # Set realistic headers
        self.session.headers.update({
            'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
            'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8',
            'Accept-Language': 'en-US,en;q=0.5',
            'Accept-Encoding': 'gzip, deflate, br',
            'DNT': '1',
            'Connection': 'keep-alive',
            'Upgrade-Insecure-Requests': '1',
        })

    async def __aenter__(self):
        """Async context manager entry."""
        playwright = await async_playwright().start()
        self.browser = await playwright.chromium.launch(
            headless=True,
            args=[
                '--no-sandbox',
                '--disable-dev-shm-usage',
                '--disable-gpu',
                '--disable-features=VizDisplayCompositor'
            ]
        )
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit."""
        if self.browser:
            await self.browser.close()

    @retry(
        stop=stop_after_attempt(3),
        wait=wait_exponential(multiplier=1, min=1, max=10),
        retry=retry_if_exception_type((requests.RequestException, RateLimitException))
    )
    def fetch_url(self, url: str, timeout: int = 30) -> requests.Response:
        """
        Fetch URL with requests, respecting rate limits.
        """
        domain = rate_limiter.get_domain(url)

        # Check rate limiting
        wait_time = rate_limiter.should_wait(domain)
        if wait_time > 0:
            logger.info(f"Rate limiting: waiting {wait_time:.2f}s for {domain}")
            time.sleep(wait_time)

        try:
            logger.debug(f"Fetching URL: {url}")
            response = self.session.get(url, timeout=timeout)

            # Check for rate limiting indicators
            if response.status_code == 429:
                retry_after = response.headers.get('Retry-After')
                retry_seconds = int(retry_after) if retry_after else 60
                rate_limiter.record_request(domain, success=False)
                raise RateLimitException(domain, retry_seconds)

            response.raise_for_status()
            rate_limiter.record_request(domain, success=True)

            logger.debug(f"Successfully fetched {url} ({response.status_code})")
            return response

        except requests.RequestException as e:
            rate_limiter.record_request(domain, success=False)
            logger.warning(f"Failed to fetch {url}: {e}")
            raise

    async def fetch_with_playwright(self, url: str, wait_for_selector: str = None, timeout: int = 30000) -> str:
        """
        Fetch URL content using Playwright for JavaScript-heavy sites.
        """
        if not self.browser:
            raise ScrapingException("", url, "Browser not initialized. Use async context manager.")

        domain = rate_limiter.get_domain(url)
        await rate_limiter.wait_if_needed(url)

        try:
            page = await self.browser.new_page()

            # Set realistic viewport and user agent
            await page.set_viewport_size({"width": 1920, "height": 1080})

            logger.debug(f"Loading page with Playwright: {url}")
            response = await page.goto(url, timeout=timeout, wait_until='domcontentloaded')

            if response.status >= 400:
                rate_limiter.record_request(domain, success=False)
                raise ScrapingException(domain, url, f"HTTP {response.status}")

            # Wait for specific selector if provided
            if wait_for_selector:
                try:
                    await page.wait_for_selector(wait_for_selector, timeout=10000)
                except Exception as e:
                    logger.warning(f"Selector '{wait_for_selector}' not found on {url}: {e}")

            # Get page content
            content = await page.content()
            await page.close()

            rate_limiter.record_request(domain, success=True)
            logger.debug(f"Successfully loaded {url} with Playwright")
            return content

        except Exception as e:
            rate_limiter.record_request(domain, success=False)
            logger.error(f"Failed to load {url} with Playwright: {e}")
            raise ScrapingException(domain, url, str(e), e)

    def get_content_hash(self, content: str) -> str:
        """Generate hash of content for change detection."""
        return hashlib.sha256(content.encode('utf-8')).hexdigest()[:16]


# Global scraper instance
web_scraper = WebScraper()


def configure_domain_rate_limit(domain: str, requests_per_minute: int = 30, min_delay: float = 2.0):
    """Configure rate limiting for a specific domain."""
    config = RateLimitConfig(
        requests_per_minute=requests_per_minute,
        min_delay_seconds=min_delay
    )
    rate_limiter.configure_domain(domain, config)


# Pre-configure common job board domains with respectful limits
def setup_default_rate_limits():
    """Set up default rate limits for known job boards."""

    # Conservative limits for major job boards
    job_board_configs = {
        'boards.greenhouse.io': RateLimitConfig(requests_per_minute=20, min_delay_seconds=3.0),
        'jobs.lever.co': RateLimitConfig(requests_per_minute=15, min_delay_seconds=4.0),
        'careers.workday.com': RateLimitConfig(requests_per_minute=10, min_delay_seconds=6.0),
        'jobs.ashbyhq.com': RateLimitConfig(requests_per_minute=20, min_delay_seconds=3.0),
        'jobs.smartrecruiters.com': RateLimitConfig(requests_per_minute=15, min_delay_seconds=4.0),

        # More aggressive limits for sites that are known to be strict
        'linkedin.com': RateLimitConfig(requests_per_minute=5, min_delay_seconds=12.0),
        'indeed.com': RateLimitConfig(requests_per_minute=8, min_delay_seconds=8.0),
        'angel.co': RateLimitConfig(requests_per_minute=10, min_delay_seconds=6.0),
    }

    for domain, config in job_board_configs.items():
        rate_limiter.configure_domain(domain, config)

    logger.info(f"Configured rate limits for {len(job_board_configs)} job board domains")


# Initialize default rate limits
setup_default_rate_limits()