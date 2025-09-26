"""
ChatGPT API integration for intelligent job scoring and summaries.
Designed for easy integration with existing rule-based system.
"""

import os
import json
from typing import Dict, List, Tuple, Optional
from dataclasses import dataclass
from datetime import datetime, timedelta

from utils.logging import get_logger

logger = get_logger("llm")

# Global settings
LLM_ENABLED = False
openai_client = None


@dataclass
class LLMConfig:
    """Configuration for LLM integration."""
    enabled: bool = False
    api_key: Optional[str] = None
    model: str = "gpt-4o-mini"
    temperature: float = 0.0
    max_tokens: int = 500
    max_daily_tokens: int = 50000
    max_requests_per_minute: int = 20
    fallback_to_rules: bool = True


@dataclass
class LLMResult:
    """Result from LLM scoring."""
    score: float
    reasons: List[str]
    summary: str
    tokens_used: int
    model_used: str


class TokenTracker:
    """Track token usage and rate limits."""

    def __init__(self, max_daily_tokens: int = 50000, max_rpm: int = 20):
        self.max_daily_tokens = max_daily_tokens
        self.max_rpm = max_rpm
        self.daily_tokens = 0
        self.requests_this_minute = 0
        self.last_reset = datetime.now()
        self.minute_start = datetime.now()

    def can_make_request(self) -> bool:
        """Check if we can make another request within limits."""
        now = datetime.now()

        # Reset daily counter
        if now.date() > self.last_reset.date():
            self.daily_tokens = 0
            self.last_reset = now

        # Reset minute counter
        if (now - self.minute_start).total_seconds() >= 60:
            self.requests_this_minute = 0
            self.minute_start = now

        # Check limits
        if self.daily_tokens >= self.max_daily_tokens:
            logger.warning(f"Daily token limit reached: {self.daily_tokens}/{self.max_daily_tokens}")
            return False

        if self.requests_this_minute >= self.max_rpm:
            logger.debug(f"Rate limit reached: {self.requests_this_minute}/{self.max_rpm} requests per minute")
            return False

        return True

    def record_usage(self, tokens: int):
        """Record token usage."""
        self.daily_tokens += tokens
        self.requests_this_minute += 1
        logger.debug(f"Token usage: +{tokens} tokens (daily total: {self.daily_tokens})")


# Global token tracker
token_tracker = TokenTracker()


def initialize_llm() -> bool:
    """Initialize ChatGPT client if configured."""
    global LLM_ENABLED, openai_client

    try:
        # Check if LLM is enabled in environment
        llm_enabled = os.getenv('LLM_ENABLED', 'false').lower() == 'true'
        api_key = os.getenv('OPENAI_API_KEY')

        if not llm_enabled:
            logger.info("LLM integration disabled (LLM_ENABLED=false)")
            return False

        if not api_key:
            logger.warning("LLM enabled but OPENAI_API_KEY not found, disabling LLM features")
            return False

        # Try to import OpenAI
        try:
            import openai
            openai_client = openai.OpenAI(api_key=api_key)
            LLM_ENABLED = True

            # Update token tracker with environment settings
            max_daily = int(os.getenv('LLM_MAX_DAILY_TOKENS', '50000'))
            max_rpm = int(os.getenv('LLM_MAX_RPM', '20'))
            token_tracker.max_daily_tokens = max_daily
            token_tracker.max_rpm = max_rpm

            logger.info(f"LLM integration initialized successfully (model: {get_llm_config().model})")
            return True

        except ImportError:
            logger.error("OpenAI package not installed. Run: pip install openai>=1.40")
            return False

    except Exception as e:
        logger.error(f"Failed to initialize LLM: {e}")
        return False


def get_llm_config() -> LLMConfig:
    """Get LLM configuration from environment."""
    return LLMConfig(
        enabled=LLM_ENABLED,
        api_key=os.getenv('OPENAI_API_KEY'),
        model=os.getenv('OPENAI_MODEL', 'gpt-4o-mini'),
        temperature=float(os.getenv('LLM_TEMPERATURE', '0.0')),
        max_tokens=int(os.getenv('LLM_MAX_TOKENS', '500')),
        max_daily_tokens=int(os.getenv('LLM_MAX_DAILY_TOKENS', '50000')),
        max_requests_per_minute=int(os.getenv('LLM_MAX_RPM', '20')),
        fallback_to_rules=os.getenv('LLM_FALLBACK_TO_RULES', 'true').lower() == 'true'
    )


def create_scoring_prompt(job: Dict, preferences: Dict) -> str:
    """Create a prompt for job scoring."""

    # Extract preferences
    title_allowlist = preferences.get('title_allowlist', [])
    title_blocklist = preferences.get('title_blocklist', [])
    keywords_boost = preferences.get('keywords_boost', [])
    location_constraints = preferences.get('location_constraints', [])
    salary_floor = preferences.get('salary_floor_usd')

    # Truncate description to save tokens
    description = job.get('description', '')[:1200]

    prompt = f"""You are a job matching expert. Score this job posting for relevance to the candidate's preferences.

CANDIDATE PREFERENCES:
- Desired Titles: {', '.join(title_allowlist)}
- Avoid Titles: {', '.join(title_blocklist)}
- Location Requirements: {', '.join(location_constraints)}
- Minimum Salary: ${salary_floor:,} USD (if specified)
- Preferred Keywords: {', '.join(keywords_boost)}

JOB POSTING:
Title: {job.get('title', 'N/A')}
Company: {job.get('company', 'N/A')}
Location: {job.get('location', 'N/A')}
Description: {description}

INSTRUCTIONS:
1. Score from 0.0 to 1.0 (1.0 = perfect match)
2. Be strict about location and title requirements
3. Penalize management/director roles unless specifically desired
4. Boost for relevant keywords and technologies
5. Consider salary if mentioned

Return JSON format:
{{
    "score": 0.85,
    "reasons": ["Title matches Product Security", "Remote location fits", "Mentions Kubernetes"],
    "summary": "Strong security role with relevant tech stack",
    "red_flags": ["Potential management duties", "Salary not specified"]
}}"""

    return prompt


def score_job_with_llm(job: Dict, preferences: Dict) -> Optional[LLMResult]:
    """Score a job using ChatGPT API."""
    if not LLM_ENABLED or not openai_client:
        return None

    if not token_tracker.can_make_request():
        return None

    config = get_llm_config()

    try:
        prompt = create_scoring_prompt(job, preferences)

        logger.debug(f"Sending job '{job.get('title')}' to LLM for scoring")

        response = openai_client.chat.completions.create(
            model=config.model,
            messages=[
                {
                    "role": "system",
                    "content": "You are a precise job matching expert. Always return valid JSON."
                },
                {"role": "user", "content": prompt}
            ],
            temperature=config.temperature,
            max_tokens=config.max_tokens,
            response_format={"type": "json_object"}
        )

        # Parse response
        result_text = response.choices[0].message.content
        result_data = json.loads(result_text)

        # Record token usage
        tokens_used = response.usage.total_tokens
        token_tracker.record_usage(tokens_used)

        # Create result object
        llm_result = LLMResult(
            score=float(result_data.get('score', 0.0)),
            reasons=result_data.get('reasons', []),
            summary=result_data.get('summary', ''),
            tokens_used=tokens_used,
            model_used=config.model
        )

        logger.debug(f"LLM scored '{job.get('title')}': {llm_result.score:.2f} ({tokens_used} tokens)")
        return llm_result

    except json.JSONDecodeError as e:
        logger.error(f"Failed to parse LLM response as JSON: {e}")
        return None
    except Exception as e:
        logger.error(f"LLM scoring failed: {e}")
        return None


def create_hybrid_score(rules_score: float, rules_reasons: List[str],
                       llm_result: Optional[LLMResult],
                       rules_weight: float = 0.5) -> Tuple[float, List[str], Dict]:
    """
    Create hybrid score combining rules and LLM results.

    Args:
        rules_score: Score from rule-based system (0.0-1.0)
        rules_reasons: Reasons from rule-based system
        llm_result: Result from LLM scoring (optional)
        rules_weight: Weight for rules vs LLM (0.5 = equal weight)

    Returns:
        Tuple of (final_score, combined_reasons, metadata)
    """
    metadata = {
        "rules_score": rules_score,
        "llm_score": None,
        "llm_used": False,
        "tokens_used": 0,
        "scoring_method": "rules_only"
    }

    # If no LLM result, use rules only
    if not llm_result:
        return rules_score, rules_reasons, metadata

    # Combine scores
    llm_weight = 1.0 - rules_weight
    final_score = (rules_score * rules_weight) + (llm_result.score * llm_weight)

    # Combine reasons
    combined_reasons = []

    # Add rules reasons with prefix
    for reason in rules_reasons:
        combined_reasons.append(f"Rules: {reason}")

    # Add LLM reasons with prefix
    for reason in llm_result.reasons:
        combined_reasons.append(f"AI: {reason}")

    # Add LLM summary if available
    if llm_result.summary:
        combined_reasons.append(f"Summary: {llm_result.summary}")

    # Update metadata
    metadata.update({
        "llm_score": llm_result.score,
        "llm_used": True,
        "tokens_used": llm_result.tokens_used,
        "scoring_method": f"hybrid_{int(rules_weight*100)}r_{int(llm_weight*100)}ai",
        "llm_summary": llm_result.summary
    })

    return final_score, combined_reasons, metadata


def get_token_usage_stats() -> Dict:
    """Get current token usage statistics."""
    return {
        "daily_tokens_used": token_tracker.daily_tokens,
        "daily_limit": token_tracker.max_daily_tokens,
        "requests_this_minute": token_tracker.requests_this_minute,
        "rpm_limit": token_tracker.max_rpm,
        "llm_enabled": LLM_ENABLED,
        "can_make_request": token_tracker.can_make_request()
    }


def reset_daily_usage():
    """Reset daily token usage (for testing)."""
    token_tracker.daily_tokens = 0
    token_tracker.last_reset = datetime.now()


# Initialize on import if environment is configured
initialize_llm()