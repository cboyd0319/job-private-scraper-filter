use super::*;

#[tokio::test]
async fn test_rate_limiter_allows_initial_requests() {
    let limiter = RateLimiter::new();

    // Should allow first request immediately
    assert!(limiter.is_allowed("test", 10).await);
    assert!(limiter.is_allowed("test", 10).await);
}

#[tokio::test]
async fn test_shared_rate_limiter_reuses_buckets_across_handles() {
    let source = format!(
        "shared-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos()
    );
    let first = RateLimiter::shared();
    let second = RateLimiter::shared();

    first.reset(&source).await;
    first.wait(&source, 1).await;

    assert!(
        !second.is_allowed(&source, 1).await,
        "shared limiter should preserve exhausted bucket state"
    );

    first.reset(&source).await;
}

#[test]
fn usajobs_policy_uses_a_single_request_burst() {
    let mut bucket = TokenBucket::new_with_burst(
        u32::from(jobsentinel_domain::v3_source_manifest::USAJOBS_REQUEST_LIMIT_PER_HOUR),
        1,
    );

    assert_eq!(bucket.capacity, 1);
    assert!(bucket.try_take_token().is_ok());
    assert_eq!(
        bucket.try_take_token().unwrap_err(),
        Duration::from_secs(60)
    );
}

#[tokio::test]
async fn paced_wait_tightens_an_existing_burst_bucket() {
    let limiter = RateLimiter::new();
    limiter.wait("usajobs", 60).await;
    limiter.wait_paced("usajobs", 60).await;

    let buckets = limiter.buckets.lock().await;
    let bucket = buckets.get("usajobs").unwrap();
    assert_eq!(bucket.capacity, 1);
    assert_eq!(bucket.tokens, 0);
}

#[tokio::test]
async fn test_rate_limiter_respects_limit() {
    let limiter = RateLimiter::new();

    // Exhaust all tokens (10 requests)
    for _ in 0..10 {
        limiter.wait("test", 10).await;
    }

    // Next request should be blocked
    assert!(!limiter.is_allowed("test", 10).await);
}

#[tokio::test]
async fn test_token_bucket_refill() {
    let mut bucket = TokenBucket::new(3600); // 3600 requests/hour = 1/second

    // Exhaust tokens
    bucket.tokens = 0;
    bucket.last_refill = Instant::now();

    // Wait 2 seconds
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Should have ~2 tokens after refill
    bucket.refill();
    assert!(bucket.tokens >= 1);
}

#[tokio::test]
async fn test_rate_limiter_reset() {
    let limiter = RateLimiter::new();

    // Exhaust tokens
    for _ in 0..10 {
        limiter.wait("test", 10).await;
    }

    assert!(!limiter.is_allowed("test", 10).await);

    // Reset should restore capacity
    limiter.reset("test").await;
    assert!(limiter.is_allowed("test", 10).await);
}

#[tokio::test]
async fn test_different_scrapers_independent_limits() {
    let limiter = RateLimiter::new();

    // Exhaust linkedin
    for _ in 0..10 {
        limiter.wait("linkedin", 10).await;
    }

    // Indeed should still be available
    assert!(limiter.is_allowed("indeed", 10).await);
}

#[tokio::test]
async fn test_waiting_bucket_does_not_block_other_scrapers() {
    let limiter = RateLimiter::new();

    // Exhaust the slow bucket.
    limiter.wait("slow", 1).await;

    let waiting_limiter = limiter.clone();
    let waiting_task = tokio::spawn(async move {
        waiting_limiter.wait("slow", 1).await;
    });

    tokio::time::sleep(Duration::from_millis(10)).await;

    let fast_result =
        tokio::time::timeout(Duration::from_millis(100), limiter.is_allowed("fast", 1)).await;

    waiting_task.abort();

    assert!(
        fast_result.is_ok(),
        "An exhausted scraper bucket must not block unrelated scraper buckets"
    );
    assert!(fast_result.unwrap());
}

#[tokio::test]
async fn test_zero_rate_is_clamped_to_safe_minimum() {
    let limiter = RateLimiter::new();

    limiter.wait("zero-rate", 0).await;

    let tokens_left = {
        let buckets = limiter.buckets.lock().await;
        buckets.get("zero-rate").unwrap().tokens
    };

    assert_eq!(tokens_left, 0);

    let second_wait =
        tokio::time::timeout(Duration::from_millis(100), limiter.wait("zero-rate", 0)).await;

    assert!(
        second_wait.is_err(),
        "Second request should wait for refill instead of panicking or bypassing the limiter"
    );
}

#[tokio::test]
async fn test_default_trait() {
    let limiter = RateLimiter::default();

    // Should work identically to RateLimiter::new()
    assert!(limiter.is_allowed("test", 10).await);
}

#[tokio::test]
async fn test_token_bucket_no_refill_when_zero_elapsed() {
    let mut bucket = TokenBucket::new(3600);

    // Record initial state
    let initial_tokens = bucket.tokens;
    let initial_capacity = bucket.capacity;

    // Refill immediately (zero time elapsed)
    bucket.refill();

    // Tokens should not change (no time passed)
    assert_eq!(bucket.tokens, initial_tokens);
    assert_eq!(bucket.capacity, initial_capacity);
}

#[tokio::test]
async fn test_token_bucket_caps_at_capacity() {
    let mut bucket = TokenBucket::new(100);

    // Start with fewer tokens
    bucket.tokens = 50;
    bucket.last_refill = Instant::now() - Duration::from_secs(3600); // 1 hour ago

    // Refill should cap at capacity, not exceed it
    bucket.refill();
    assert_eq!(bucket.tokens, 100);
    assert!(bucket.tokens <= bucket.capacity);
}

#[tokio::test]
async fn test_wait_blocks_when_no_tokens() {
    let limiter = RateLimiter::new();

    // Use only 2 tokens capacity for fast test
    // 2 requests per hour = 1 token every 1800 seconds
    limiter.wait("slow", 2).await; // Use first token
    limiter.wait("slow", 2).await; // Use second token

    // Verify we're out of tokens
    assert!(!limiter.is_allowed("slow", 2).await);

    // Next wait should block until refill
    // Since refill_rate = 2/3600 = 0.000556 tokens/sec,
    // wait time = 1 / 0.000556 = 1800 seconds
    // This is too long for a test, so we use timeout to verify blocking behavior

    let start = Instant::now();

    // Use a short timeout to verify it blocks (doesn't complete immediately)
    let wait_result =
        tokio::time::timeout(Duration::from_millis(100), limiter.wait("slow", 2)).await;

    let elapsed = start.elapsed();

    // Verify wait timed out (proving it was blocking)
    assert!(
        wait_result.is_err(),
        "Wait should have timed out, proving it blocks when no tokens available"
    );

    // Should have waited approximately the timeout duration
    assert!(
        elapsed >= Duration::from_millis(100) && elapsed < Duration::from_millis(200),
        "Expected timeout after ~100ms, but elapsed: {:?}",
        elapsed
    );
}

#[tokio::test]
async fn test_token_bucket_refill_rate_calculation() {
    let bucket = TokenBucket::new(3600);

    // 3600 requests/hour should be 1 request/second
    assert_eq!(bucket.refill_rate, 1.0);

    let bucket_slow = TokenBucket::new(100);
    // 100 requests/hour should be ~0.0278 requests/second
    assert!((bucket_slow.refill_rate - (100.0 / 3600.0)).abs() < 0.001);
}

#[tokio::test]
async fn test_is_allowed_does_not_consume_token() {
    let limiter = RateLimiter::new();

    // Check if allowed (should not consume)
    assert!(limiter.is_allowed("test", 10).await);

    // Should still have full capacity for actual requests
    for _ in 0..10 {
        limiter.wait("test", 10).await;
    }

    // Now should be exhausted
    assert!(!limiter.is_allowed("test", 10).await);
}

#[tokio::test]
async fn test_limits_constants_are_reasonable() {
    // Verify constants are defined and reasonable
    assert!(limits::LINKEDIN > 0);
    assert!(limits::INDEED > 0);
    assert!(limits::JOBSWITHGPT > 0);

    // Verify conservative LinkedIn limit
    assert!(limits::LINKEDIN < limits::INDEED);

    // Verify MCP server has highest limit
}

#[tokio::test]
async fn test_multiple_wait_cycles() {
    let limiter = RateLimiter::new();

    // Use tokens
    for _ in 0..5 {
        limiter.wait("cycle", 10).await;
    }

    // Verify some consumed
    let tokens_left = {
        let buckets = limiter.buckets.lock().await;
        buckets.get("cycle").unwrap().tokens
    };
    assert_eq!(tokens_left, 5);

    // Use remaining
    for _ in 0..5 {
        limiter.wait("cycle", 10).await;
    }

    // Should be exhausted
    assert!(!limiter.is_allowed("cycle", 10).await);
}

#[tokio::test]
async fn test_zero_tokens_edge_case() {
    let limiter = RateLimiter::new();

    // Exhaust all tokens
    for _ in 0..10 {
        limiter.wait("zero", 10).await;
    }

    // Manually verify bucket is at zero
    let is_zero = {
        let buckets = limiter.buckets.lock().await;
        buckets.get("zero").unwrap().tokens == 0
    };
    assert!(is_zero);

    // is_allowed should return false
    assert!(!limiter.is_allowed("zero", 10).await);
}
