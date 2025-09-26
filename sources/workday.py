import asyncio
from .common import fetch_url, create_job_hash, fetch_job_description, extract_company_from_url
from utils.logging import get_logger

logger = get_logger("sources.workday")


def scrape(board_url: str, fetch_descriptions: bool = True):
    """Scrapes jobs from a Workday board."""
    logger.info(f"Starting Workday scrape for {board_url}")

    company_name = extract_company_from_url(board_url)

    # Workday often uses GraphQL or specific API endpoints
    # This is a generic approach that may need customization per company

    try:
        # First, try to fetch the main page to look for job data
        page_data = fetch_url(board_url)

        scraped_jobs = []

        # Workday pages often contain JSON data embedded in script tags
        # or use specific API endpoints that vary by company
        content = page_data.get('content', '')

        # Look for common Workday job data patterns
        if 'workday' in content.lower() and 'jobs' in content.lower():
            # Try to extract job links for further processing
            from bs4 import BeautifulSoup
            soup = BeautifulSoup(content, 'html.parser')

            # Common selectors for Workday job boards
            job_links = soup.find_all('a', href=True)
            job_data = []

            for link in job_links:
                href = link.get('href', '')
                text = link.get_text(strip=True)

                # Filter for actual job postings
                if ('job' in href.lower() or 'position' in href.lower()) and text and len(text) > 5:
                    if not href.startswith('http'):
                        if href.startswith('/'):
                            from urllib.parse import urljoin
                            href = urljoin(board_url, href)
                        else:
                            continue

                    job_data.append({
                        'title': text,
                        'url': href,
                        'location': 'Various'  # Will be updated if we can extract more detail
                    })

            logger.info(f"Found {len(job_data)} potential jobs for {company_name}")

            # Process each job
            for job in job_data[:20]:  # Limit to 20 jobs to avoid overwhelming
                job_title = job['title']
                job_url = job['url']
                job_location = job['location']

                job_description = ""

                # Fetch full description if requested
                if fetch_descriptions and job_url:
                    try:
                        full_description = asyncio.run(
                            fetch_job_description(job_url, '[data-automation-id="jobPostingDescription"]')
                        )
                        if full_description:
                            job_description = full_description
                    except Exception as e:
                        logger.warning(f"Failed to fetch description for {job_title}: {e}")

                job_hash = create_job_hash(company_name, job_title, job_description[:250])

                scraped_jobs.append({
                    'hash': job_hash,
                    'title': job_title,
                    'url': job_url,
                    'company': company_name,
                    'location': job_location,
                    'description': job_description
                })

        else:
            logger.warning(f"No recognizable job data found for Workday board: {board_url}")

        logger.info(f"Successfully scraped {len(scraped_jobs)} jobs from {company_name}")
        return scraped_jobs

    except Exception as e:
        logger.error(f"Failed to scrape Workday board {board_url}: {e}")
        raise