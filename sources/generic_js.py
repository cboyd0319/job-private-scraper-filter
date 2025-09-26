import asyncio
import json
from urllib.parse import urljoin
from .common import create_job_hash, extract_company_from_url
from utils.logging import get_logger
from utils.scraping import web_scraper

logger = get_logger("sources.generic_js")

async def scrape_js_career_page(board_url: str, fetch_descriptions: bool = True, custom_config: dict = None):
    """
    Scrape JavaScript-rendered career pages with intelligent job extraction
    and auto-detection of platforms like Ashby and SmartRecruiters.
    """
    logger.info(f"Starting generic JS scrape for {board_url}")
    company_name = extract_company_from_url(board_url)
    
    async with web_scraper as scraper:
        # --- Platform Auto-Detection ---
        page_content = await scraper.fetch_with_playwright(board_url)

        if "ashbyhq.com" in page_content or "jobs.ashbyhq.com" in board_url:
            logger.info(f"Detected AshbyHQ platform for {company_name}")
            return await _scrape_ashby(page_content, board_url, company_name)
        
        if "smartrecruiters.com" in page_content or "smartrecruiters.com" in board_url:
            logger.info(f"Detected SmartRecruiters platform for {company_name}")
            return await _scrape_smartrecruiters(page_content, board_url, company_name)
        
        # Fallback to original generic scraping
        logger.info("No specific platform detected, using default DOM extraction.")
        return await _try_dom_extraction(scraper, board_url, {}, company_name, initial_content=page_content)

async def _scrape_ashby(page_content, board_url, company_name):
    """Scraper tailored for Ashby boards."""
    from bs4 import BeautifulSoup
    soup = BeautifulSoup(page_content, 'html.parser')
    jobs = []
    
    for job_div in soup.select('div[class*="_jobPosting_"]'):
        title_tag = job_div.select_one('h3')
        location_tag = job_div.select_one('p')
        link_tag = job_div.find('a', href=True)
        
        if title_tag and location_tag and link_tag:
            title = title_tag.get_text(strip=True)
            location = location_tag.get_text(strip=True)
            url = urljoin(board_url, link_tag['href'])
            description_stub = f"{title} {location}"
            job_hash = create_job_hash(company_name, title, description_stub)
            
            jobs.append({
                'hash': job_hash, 'title': title, 'url': url,
                'company': company_name, 'location': location,
                'description': ''
            })
    return jobs

async def _scrape_smartrecruiters(page_content, board_url, company_name):
    """Scraper tailored for SmartRecruiters boards."""
    from bs4 import BeautifulSoup
    soup = BeautifulSoup(page_content, 'html.parser')
    jobs = []

    for job_item in soup.select('li.jobs-item'):
        title_tag = job_item.select_one('h4.details-title')
        link_tag = job_item.find('a', href=True)
        
        if title_tag and link_tag:
            title = title_tag.get_text(strip=True)
            url = link_tag['href']
            # Location is often not easily available on the list page
            location = "See Description"
            description_stub = f"{title} {location}"
            job_hash = create_job_hash(company_name, title, description_stub)

            jobs.append({
                'hash': job_hash, 'title': title, 'url': url,
                'company': company_name, 'location': location,
                'description': ''
            })
    return jobs

async def _try_dom_extraction(scraper, board_url, config, company_name, initial_content=None):
    """Original DOM extraction logic as a fallback."""
    # (Your original _try_dom_extraction logic can be placed here)
    logger.warning(f"DOM extraction for {board_url} is not fully implemented in this update.")
    return []

# Synchronous wrapper
def scrape(board_url: str, fetch_descriptions: bool = True, custom_config: dict = None):
    return asyncio.run(scrape_js_career_page(board_url, fetch_descriptions, custom_config))