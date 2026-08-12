//! Rate Limiting for Job Board Scrapers
//!
//! Implements token bucket rate limiting to prevent IP bans and respect
//! rate limits of job boards.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

static SHARED_RATE_LIMITER: LazyLock<RateLimiter> = LazyLock::new(RateLimiter::new);

/// Rate limiter using token bucket algorithm
#[derive(Debug, Clone)]
pub struct RateLimiter {
    buckets: Arc<Mutex<HashMap<String, TokenBucket>>>,
}

/// Token bucket for rate limiting
#[derive(Debug)]
struct TokenBucket {
    /// Refill budget per hour
    requests_per_hour: u32,
    /// Maximum tokens (requests) allowed
    capacity: u32,
    /// Current token count
    tokens: u32,
    /// Last refill time
    last_refill: Instant,
    /// Refill rate (tokens per second)
    refill_rate: f64,
}

impl RateLimiter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Return the process-wide limiter used by production scraper instances.
    #[must_use]
    pub fn shared() -> Self {
        SHARED_RATE_LIMITER.clone()
    }

    /// Wait until request is allowed for given scraper
    ///
    /// # Arguments
    /// * `scraper_name` - Name of the scraper (e.g., "linkedin", "indeed")
    /// * `max_requests_per_hour` - Maximum requests allowed per hour
    ///
    /// The scraper owner supplies a stable source identifier and an hourly
    /// request limit for each call.
    #[tracing::instrument(skip(self))]
    pub async fn wait(&self, scraper_name: &str, max_requests_per_hour: u32) {
        self.wait_with_burst(scraper_name, max_requests_per_hour, max_requests_per_hour)
            .await;
    }

    /// Pace requests without an initial multi-request burst.
    #[tracing::instrument(skip(self))]
    pub async fn wait_paced(&self, scraper_name: &str, max_requests_per_hour: u32) {
        self.wait_with_burst(scraper_name, max_requests_per_hour, 1)
            .await;
    }

    async fn wait_with_burst(
        &self,
        scraper_name: &str,
        max_requests_per_hour: u32,
        max_burst: u32,
    ) {
        tracing::debug!("Acquiring rate limit token for scraper");

        loop {
            let wait_duration = {
                let mut buckets = self.buckets.lock().await;

                let bucket = buckets.entry(scraper_name.to_string()).or_insert_with(|| {
                    TokenBucket::new_with_burst(max_requests_per_hour, max_burst)
                });
                bucket.reconfigure(max_requests_per_hour, max_burst);

                match bucket.try_take_token() {
                    Ok(()) => return,
                    Err(wait_duration) => wait_duration,
                }
            };

            tokio::time::sleep(wait_duration).await;
        }
    }

    /// Check if request is allowed without waiting
    #[must_use]
    #[cfg(test)]
    pub async fn is_allowed(&self, scraper_name: &str, max_requests_per_hour: u32) -> bool {
        let mut buckets = self.buckets.lock().await;

        let bucket = buckets
            .entry(scraper_name.to_string())
            .or_insert_with(|| TokenBucket::new(max_requests_per_hour));

        bucket.refill();
        bucket.tokens > 0
    }

    /// Reset rate limiter for a scraper (useful for testing)
    #[cfg(test)]
    pub async fn reset(&self, scraper_name: &str) {
        let mut buckets = self.buckets.lock().await;
        buckets.remove(scraper_name);
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenBucket {
    #[must_use]
    #[cfg(test)]
    fn new(max_requests_per_hour: u32) -> Self {
        Self::new_with_burst(max_requests_per_hour, max_requests_per_hour)
    }

    #[must_use]
    fn new_with_burst(max_requests_per_hour: u32, max_burst: u32) -> Self {
        let rate = max_requests_per_hour.max(1);
        let capacity = max_burst.clamp(1, rate);
        Self {
            requests_per_hour: rate,
            capacity,
            tokens: capacity,
            last_refill: Instant::now(),
            refill_rate: rate as f64 / 3600.0, // tokens per second
        }
    }

    fn reconfigure(&mut self, max_requests_per_hour: u32, max_burst: u32) {
        let rate = max_requests_per_hour.max(1);
        let capacity = max_burst.clamp(1, rate);
        if self.capacity == capacity && self.requests_per_hour == rate {
            return;
        }
        self.refill();
        self.requests_per_hour = rate;
        self.capacity = capacity;
        self.tokens = self.tokens.min(capacity);
        self.refill_rate = rate as f64 / 3600.0;
        self.last_refill = Instant::now();
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();

        // Calculate tokens to add
        let tokens_to_add = (elapsed * self.refill_rate) as u32;

        if tokens_to_add > 0 {
            self.tokens = (self.tokens + tokens_to_add).min(self.capacity);
            self.last_refill = now;
        }
    }

    /// Try to consume a token, or return how long the caller should wait.
    fn try_take_token(&mut self) -> Result<(), Duration> {
        self.refill();

        if self.tokens > 0 {
            self.tokens -= 1;
            tracing::debug!("Token consumed, {} tokens remaining", self.tokens);
            return Ok(());
        }

        let wait_secs = 1.0 / self.refill_rate;
        let wait_duration = Duration::from_secs_f64(wait_secs);

        tracing::warn!(
            "Rate limit exhausted, waiting {:?} for token refill (refill_rate: {}/sec)",
            wait_duration,
            self.refill_rate
        );

        Err(wait_duration)
    }
}

/// Scraper-specific rate limits (requests per hour)
pub mod limits {
    /// LinkedIn: 100 requests/hour (conservative to avoid detection)
    #[cfg(test)]
    pub const LINKEDIN: u32 = 100;

    /// Indeed: 500 requests/hour (more generous, has public API)
    pub const INDEED: u32 = 500;

    /// JobsWithGPT: 10,000 requests/hour (MCP server)
    pub const JOBSWITHGPT: u32 = 10_000;
}

#[cfg(test)]
mod tests;
