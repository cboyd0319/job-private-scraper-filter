import asyncio
from .common import fetch_url, create_job_hash, fetch_job_description, extract_company_from_url
from utils.logging import get_logger

logger = get_logger("sources.lever")


def scrape(board_url: str, fetch_descriptions: bool = True):
    """Scrapes jobs from a Lever board using their API endpoint."""
    logger.info(f"Starting Lever scrape for {board_url}")

    # Lever boards typically use format: https://jobs.lever.co/company
    # API endpoint: https://api.lever.co/v0/postings/company
    company_name = extract_company_from_url(board_url)
    api_url = f"https://api.lever.co/v0/postings/{company_name}"

    try:
        data = fetch_url(api_url)

        # Lever API returns list of jobs directly
        if isinstance(data, list):
            jobs_data = data
        else:
            jobs_data = data.get('data', [])

        scraped_jobs = []
        logger.info(f"Found {len(jobs_data)} jobs for {company_name}")

        for job in jobs_data:
            job_title = job.get('text', 'N/A')
            job_url = job.get('hostedUrl', '#')

            # Location can be in different formats
            location_obj = job.get('categories', {}).get('location')
            if location_obj:
                job_location = location_obj if isinstance(location_obj, str) else location_obj.get('text', 'N/A')
            else:
                job_location = 'N/A'

            # Initial description from API
            job_description = job.get('description', '')

            # Fetch full description if requested
            if fetch_descriptions and job_url and job_url != '#':
                try:
                    full_description = asyncio.run(
                        fetch_job_description(job_url, '.posting-content')
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
        logger.error(f"Failed to scrape Lever board {board_url}: {e}")
        raise