def score_job(job: dict, prefs: dict, use_llm: bool = None) -> tuple[float, list[str], dict]:
    """
    Applies rule-based scoring to a job, with optional LLM enhancement.

    Args:
        job: Job data dictionary
        prefs: User preferences
        use_llm: Whether to use LLM scoring (None = auto-detect from env)

    Returns:
        Tuple of (score, reasons, metadata)
    """
    # Get rule-based score first
    rules_score, rules_reasons = score_job_rules_only(job, prefs)

    # Try LLM enhancement if enabled and job passed basic rules
    llm_result = None
    if rules_score > 0 and _should_use_llm(use_llm, prefs):
        try:
            from utils.llm import score_job_with_llm
            llm_result = score_job_with_llm(job, prefs)
        except ImportError:
            pass  # LLM not available, continue with rules only
        except Exception as e:
            from utils.logging import get_logger
            logger = get_logger("scoring")
            logger.debug(f"LLM scoring failed for {job.get('title', 'Unknown')}: {e}")

    # Create hybrid score
    if llm_result:
        from utils.llm import create_hybrid_score
        llm_weight = prefs.get('llm_weight', 0.5)
        rules_weight = 1.0 - llm_weight
        final_score, combined_reasons, metadata = create_hybrid_score(
            rules_score, rules_reasons, llm_result, rules_weight
        )
        return final_score, combined_reasons, metadata
    else:
        # Rules only
        metadata = {
            "rules_score": rules_score,
            "llm_score": None,
            "llm_used": False,
            "tokens_used": 0,
            "scoring_method": "rules_only"
        }
        return rules_score, rules_reasons, metadata


def score_job_rules_only(job: dict, prefs: dict) -> tuple[float, list[str]]:
    """Legacy rule-based scoring function (backward compatible)."""
    score = 0.0
    reasons = []

    title = job.get('title', '').lower()

    # --- BLOCKLIST FILTER (IMMEDIATE REJECTION) ---
    for blocked_word in prefs.get("title_blocklist", []):
        if blocked_word.lower() in title:
            return 0.0, [f"Rejected: Title contains blocked word '{blocked_word}'"]

    # --- ALLOWLIST FILTER (MUST MATCH ONE) ---
    has_allowed_title = False
    for allowed_word in prefs.get("title_allowlist", []):
        if allowed_word.lower() in title:
            score += 0.6  # Base score for matching title
            reasons.append(f"Title matched '{allowed_word}'")
            has_allowed_title = True
            break
    if not has_allowed_title:
        return 0.0, ["Rejected: Title did not match allowlist"]

    # --- LOCATION SCORING ---
    location = job.get('location', '').lower()
    for loc_pref in prefs.get("location_constraints", []):
        if loc_pref.lower() in location:
            score += 0.2
            reasons.append(f"Location matched '{loc_pref}'")
            break # Only add points for the first location match

    # --- KEYWORD BOOSTS ---
    description = job.get('description', '').lower()
    full_text = title + ' ' + description
    for boost_word in prefs.get("keywords_boost", []):
        if boost_word.lower() in full_text:
            score += 0.05
            reasons.append(f"Keyword boost: '{boost_word}'")

    # --- SALARY FILTER ---
    salary_floor = prefs.get('salary_floor_usd')
    if salary_floor:
        # Try to extract salary from description
        salary_found = _extract_salary(full_text)
        if salary_found and salary_found < salary_floor:
            return 0.0, [f"Rejected: Salary ${salary_found:,} below floor ${salary_floor:,}"]
        elif salary_found and salary_found >= salary_floor:
            score += 0.1
            reasons.append(f"Salary ${salary_found:,} meets requirements")

    return min(score, 1.0), reasons # Cap score at 1.0


def _should_use_llm(use_llm: bool, prefs: dict) -> bool:
    """Determine if LLM should be used for scoring."""
    if use_llm is not None:
        return use_llm

    # Check preferences
    if 'use_llm' in prefs:
        return prefs['use_llm']

    # Check environment
    import os
    return os.getenv('LLM_ENABLED', 'false').lower() == 'true'


def _extract_salary(text: str) -> int:
    """Try to extract salary information from job text."""
    import re

    # Common salary patterns
    patterns = [
        r'\$(\d{1,3}(?:,\d{3})*(?:\.\d{2})?)[kK]?',  # $150k, $150,000
        r'(\d{1,3}(?:,\d{3})*)[kK]',                 # 150k, 150,000
        r'salary.*?(\d{1,3}(?:,\d{3})*)',            # salary of 150000
        r'compensation.*?(\d{1,3}(?:,\d{3})*)',      # compensation 150000
    ]

    for pattern in patterns:
        matches = re.findall(pattern, text, re.IGNORECASE)
        for match in matches:
            try:
                # Clean and convert to integer
                salary_str = match.replace(',', '').replace('$', '')
                salary = int(salary_str)

                # Handle 'k' suffix
                if 'k' in match.lower():
                    salary *= 1000

                # Reasonable salary range check
                if 30000 <= salary <= 1000000:
                    return salary
            except (ValueError, TypeError):
                continue

    return None