import asyncio
from .common import fetch_url, create_job_hash, fetch_job_description, extract_company_from_url
from utils.logging import get_logger

logger = get_logger("sources.greenhouse")

def scrape(board_url: str, fetch_descriptions: bool = True):
    """Scrapes jobs from a Greenhouse board with a JSON endpoint."""
    logger.info(f"Starting Greenhouse scrape for {board_url}")

    # Greenhouse boards often have a '?for=json' API endpoint
    api_url = f"{board_url}?for=json"

    try:
        data = fetch_url(api_url)

        # Handle both direct JSON response and wrapped response
        if isinstance(data, dict) and 'jobs' in data:
            jobs_data = data['jobs']
        else:
            jobs_data = data

        scraped_jobs = []
        company_name = extract_company_from_url(board_url)

        logger.info(f"Found {len(jobs_data)} jobs for {company_name}")

        for job in jobs_data:
            job_title = job.get('title', 'N/A')
            job_url = job.get('absolute_url', '#')
            job_location = job.get('location', {}).get('name', 'N/A')

            # Initial description from API (usually limited)
            job_description = job.get('content', '')

            # Fetch full description if requested and URL is available
            if fetch_descriptions and job_url and job_url != '#':
                try:
                    full_description = asyncio.run(
                        fetch_job_description(job_url, '.content')
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

        logger.info(f"Successfully scraped {len(scraped_jobs)} jobs from {company_name}")
        return scraped_jobs

    except Exception as e:
        logger.error(f"Failed to scrape Greenhouse board {board_url}: {e}")
        raise