import hashlib
from utils.scraping import web_scraper
from utils.logging import get_logger
from utils.errors import ScrapingException

logger = get_logger("sources.common")

def create_job_hash(company: str, title: str, description: str) -> str:
    """Creates a stable SHA-256 hash for a job based on its content."""
    # Normalize by lowercasing and removing whitespace
    norm_company = "".join(company.lower().split())
    norm_title = "".join(title.lower().split())
    # Use only the first 250 chars of the description for stability
    norm_desc = "".join(description.lower().split())[:250]
    
    hash_input = f"{norm_company}{norm_title}{norm_desc}".encode('utf-8')
    return hashlib.sha256(hash_input).hexdigest()

def fetch_url(url: str) -> dict:
    """Fetches a URL with retries and rate limiting. Returns response data."""
    try:
        response = web_scraper.fetch_url(url)

        # Try to parse as JSON, fall back to text
        try:
            return response.json()
        except ValueError:
            return {"content": response.text, "status_code": response.status_code}

    except Exception as e:
        logger.error(f"Failed to fetch {url}: {e}")
        raise ScrapingException("", url, str(e), e)


async def fetch_job_description(job_url: str, selector: str = None) -> str:
    """Fetch full job description using Playwright for JS-heavy sites."""
    try:
        async with web_scraper as scraper:
            content = await scraper.fetch_with_playwright(
                job_url,
                wait_for_selector=selector
            )

            # Extract text content from HTML
            from bs4 import BeautifulSoup
            soup = BeautifulSoup(content, 'html.parser')

            # Remove script and style elements
            for script in soup(["script", "style"]):
                script.decompose()

            # Get text content
            text = soup.get_text()

            # Clean up whitespace
            lines = (line.strip() for line in text.splitlines())
            chunks = (phrase.strip() for line in lines for phrase in line.split("  "))
            description = '\n'.join(chunk for chunk in chunks if chunk)

            logger.debug(f"Fetched job description from {job_url} ({len(description)} chars)")
            return description[:5000]  # Limit to 5000 characters

    except Exception as e:
        logger.warning(f"Failed to fetch job description from {job_url}: {e}")
        return ""


def extract_company_from_url(url: str) -> str:
    """Extract company name from job board URL."""
    from urllib.parse import urlparse

    parsed = urlparse(url)

    # Greenhouse boards
    if 'greenhouse.io' in parsed.netloc:
        return parsed.path.split('/')[-1] or 'unknown'

    # Lever boards
    if 'lever.co' in parsed.netloc:
        return parsed.netloc.split('.')[0]

    # Workday boards
    if 'workday.com' in parsed.netloc:
        return parsed.path.split('/')[1] if len(parsed.path.split('/')) > 1 else 'unknown'

    # Default fallback
    return parsed.netloc.replace('www.', '').split('.')[0]