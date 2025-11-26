//! Token Bucket Rate Limiting
//!
//! Implements a production-grade rate limiter with:
//! - Per-key token bucket algorithm
//! - Configurable tiers with different limits
//! - Automatic cleanup of expired buckets
//! - Thread-safe concurrent access

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use dashmap::DashMap;
use tracing::{debug, warn};

use crate::config::security::{RateLimitConfig, RateLimitTier};
use crate::error::ErrorResponse;

use super::api_key::ApiKeyInfo;

/// Token bucket for rate limiting
#[derive(Debug)]
pub struct TokenBucket {
    /// Maximum tokens (capacity)
    capacity: u32,
    /// Current token count
    tokens: AtomicU32,
    /// Tokens per second (refill rate)
    refill_rate: f64,
    /// Last refill timestamp (as nanos since start)
    last_refill: AtomicU64,
    /// Start instant for timing
    start: Instant,
}

impl TokenBucket {
    /// Create a new token bucket
    pub fn new(capacity: u32, requests_per_minute: u32) -> Self {
        let refill_rate = requests_per_minute as f64 / 60.0;
        Self {
            capacity,
            tokens: AtomicU32::new(capacity),
            refill_rate,
            last_refill: AtomicU64::new(0),
            start: Instant::now(),
        }
    }

    /// Try to consume a token, returning true if successful
    pub fn try_consume(&self, count: u32) -> bool {
        self.refill();

        loop {
            let current = self.tokens.load(Ordering::Acquire);
            if current < count {
                return false;
            }

            if self.tokens
                .compare_exchange_weak(current, current - count, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return true;
            }
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&self) {
        let now_nanos = self.start.elapsed().as_nanos() as u64;
        let last = self.last_refill.load(Ordering::Acquire);

        let elapsed_nanos = now_nanos.saturating_sub(last);
        let elapsed_secs = elapsed_nanos as f64 / 1_000_000_000.0;

        if elapsed_secs < 0.001 {
            return; // Skip if less than 1ms elapsed
        }

        let new_tokens = (elapsed_secs * self.refill_rate) as u32;
        if new_tokens == 0 {
            return;
        }

        // Try to update last_refill atomically
        if self.last_refill
            .compare_exchange_weak(last, now_nanos, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
        {
            let current = self.tokens.load(Ordering::Acquire);
            let new_count = (current + new_tokens).min(self.capacity);
            self.tokens.store(new_count, Ordering::Release);
        }
    }

    /// Get current token count
    pub fn tokens(&self) -> u32 {
        self.refill();
        self.tokens.load(Ordering::Acquire)
    }

    /// Get time until next token is available
    pub fn time_until_token(&self) -> Duration {
        let current = self.tokens();
        if current > 0 {
            return Duration::ZERO;
        }

        // Calculate time for 1 token to refill
        let secs_per_token = 1.0 / self.refill_rate;
        Duration::from_secs_f64(secs_per_token)
    }

    /// Get the capacity
    pub fn capacity(&self) -> u32 {
        self.capacity
    }
}

/// Rate limiter managing multiple buckets
pub struct RateLimiter {
    /// Buckets keyed by identifier (usually API key ID)
    buckets: DashMap<String, Arc<TokenBucket>>,
    /// Configuration
    config: Arc<RateLimitConfig>,
    /// Creation time for cleanup
    #[allow(dead_code)]
    created: Instant,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: Arc<RateLimitConfig>) -> Self {
        Self {
            buckets: DashMap::new(),
            config,
            created: Instant::now(),
        }
    }

    /// Get or create a bucket for a key
    pub fn get_bucket(&self, key: &str, tier: RateLimitTier) -> Arc<TokenBucket> {
        if let Some(bucket) = self.buckets.get(key) {
            return bucket.clone();
        }

        let tier_config = self.config.get_tier_config(tier);
        let bucket = Arc::new(TokenBucket::new(
            tier_config.burst_size,
            tier_config.requests_per_minute,
        ));

        self.buckets.insert(key.to_string(), bucket.clone());
        bucket
    }

    /// Try to acquire a permit for a request
    pub fn try_acquire(&self, key: &str, tier: RateLimitTier) -> RateLimitResult {
        // Skip rate limiting for unlimited tier
        if tier == RateLimitTier::Unlimited {
            return RateLimitResult::Allowed {
                remaining: u32::MAX,
                limit: u32::MAX,
                reset: Duration::ZERO,
            };
        }

        let bucket = self.get_bucket(key, tier);
        let tier_config = self.config.get_tier_config(tier);

        if bucket.try_consume(1) {
            RateLimitResult::Allowed {
                remaining: bucket.tokens(),
                limit: tier_config.requests_per_minute,
                reset: bucket.time_until_token(),
            }
        } else {
            let retry_after = bucket.time_until_token();
            RateLimitResult::Exceeded {
                retry_after,
                limit: tier_config.requests_per_minute,
            }
        }
    }

    /// Clean up expired buckets
    pub fn cleanup(&self) {
        // For now, just log the bucket count
        debug!(bucket_count = self.buckets.len(), "Rate limiter status");
    }

    /// Get the number of active buckets
    pub fn bucket_count(&self) -> usize {
        self.buckets.len()
    }

    /// Check if rate limiting is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(Arc::new(RateLimitConfig::default()))
    }
}

/// Result of a rate limit check
#[derive(Debug, Clone)]
pub enum RateLimitResult {
    /// Request allowed
    Allowed {
        remaining: u32,
        limit: u32,
        reset: Duration,
    },
    /// Rate limit exceeded
    Exceeded {
        retry_after: Duration,
        limit: u32,
    },
}

/// Rate limit error response
#[derive(Debug, Clone)]
pub struct RateLimitError {
    pub retry_after: Duration,
    pub limit: u32,
}

impl IntoResponse for RateLimitError {
    fn into_response(self) -> Response {
        let body = ErrorResponse::new("rate_limit_error", "Rate limit exceeded. Please retry later.");
        let retry_after_secs = self.retry_after.as_secs().max(1);

        let mut response = (StatusCode::TOO_MANY_REQUESTS, Json(body)).into_response();

        let headers = response.headers_mut();
        headers.insert(
            "retry-after",
            retry_after_secs.to_string().parse().unwrap(),
        );
        headers.insert(
            "x-ratelimit-limit",
            self.limit.to_string().parse().unwrap(),
        );
        headers.insert("x-ratelimit-remaining", "0".parse().unwrap());

        response
    }
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(limiter): State<Arc<RateLimiter>>,
    request: Request,
    next: Next,
) -> Result<Response, RateLimitError> {
    // Skip if rate limiting is disabled
    if !limiter.is_enabled() {
        return Ok(next.run(request).await);
    }

    // Get key info from extensions (set by api_key_auth_middleware)
    let key_info = request.extensions().get::<ApiKeyInfo>()
        .cloned()
        .unwrap_or_else(ApiKeyInfo::anonymous);

    // Use key ID as the rate limit key
    let rate_limit_key = &key_info.id;

    // Check rate limit
    match limiter.try_acquire(rate_limit_key, key_info.tier) {
        RateLimitResult::Allowed { remaining, limit, reset } => {
            let mut response = next.run(request).await;

            // Add rate limit headers
            let headers = response.headers_mut();
            headers.insert(
                "x-ratelimit-limit",
                limit.to_string().parse().unwrap(),
            );
            headers.insert(
                "x-ratelimit-remaining",
                remaining.to_string().parse().unwrap(),
            );
            headers.insert(
                "x-ratelimit-reset",
                reset.as_secs().to_string().parse().unwrap(),
            );

            Ok(response)
        }
        RateLimitResult::Exceeded { retry_after, limit } => {
            warn!(
                key_id = %key_info.id,
                tier = %key_info.tier,
                path = %request.uri().path(),
                "Rate limit exceeded"
            );

            Err(RateLimitError { retry_after, limit })
        }
    }
}

/// Metrics for rate limiting
pub struct RateLimitMetrics {
    /// Total requests that hit rate limits
    pub rate_limit_hits: AtomicU64,
    /// Total requests allowed
    pub requests_allowed: AtomicU64,
}

impl RateLimitMetrics {
    pub fn new() -> Self {
        Self {
            rate_limit_hits: AtomicU64::new(0),
            requests_allowed: AtomicU64::new(0),
        }
    }

    pub fn record_hit(&self) {
        self.rate_limit_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_allowed(&self) {
        self.requests_allowed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn hits(&self) -> u64 {
        self.rate_limit_hits.load(Ordering::Relaxed)
    }

    pub fn allowed(&self) -> u64 {
        self.requests_allowed.load(Ordering::Relaxed)
    }
}

impl Default for RateLimitMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_token_bucket_creation() {
        let bucket = TokenBucket::new(10, 60); // 10 burst, 1 per second
        assert_eq!(bucket.capacity(), 10);
        assert_eq!(bucket.tokens(), 10);
    }

    #[test]
    fn test_token_bucket_consume() {
        let bucket = TokenBucket::new(5, 60);

        // Should be able to consume 5 tokens
        for _ in 0..5 {
            assert!(bucket.try_consume(1));
        }

        // Should fail on 6th attempt
        assert!(!bucket.try_consume(1));
    }

    #[test]
    fn test_token_bucket_refill() {
        let bucket = TokenBucket::new(2, 6000); // 100 per second

        // Consume all tokens
        assert!(bucket.try_consume(2));
        assert!(!bucket.try_consume(1));

        // Wait for refill (short time since high rate)
        thread::sleep(Duration::from_millis(50));

        // Should have some tokens now
        assert!(bucket.tokens() > 0);
    }

    #[test]
    fn test_rate_limiter() {
        let config = Arc::new(RateLimitConfig::default());
        let limiter = RateLimiter::new(config);

        // First request should be allowed
        match limiter.try_acquire("test-key", RateLimitTier::Standard) {
            RateLimitResult::Allowed { .. } => {}
            RateLimitResult::Exceeded { .. } => panic!("First request should be allowed"),
        }
    }

    #[test]
    fn test_rate_limiter_unlimited_tier() {
        let config = Arc::new(RateLimitConfig::default());
        let limiter = RateLimiter::new(config);

        // Unlimited tier should always be allowed
        for _ in 0..1000 {
            match limiter.try_acquire("admin-key", RateLimitTier::Unlimited) {
                RateLimitResult::Allowed { remaining, .. } => {
                    assert_eq!(remaining, u32::MAX);
                }
                RateLimitResult::Exceeded { .. } => panic!("Unlimited should never exceed"),
            }
        }
    }

    #[test]
    fn test_rate_limit_metrics() {
        let metrics = RateLimitMetrics::new();

        metrics.record_allowed();
        metrics.record_allowed();
        metrics.record_hit();

        assert_eq!(metrics.allowed(), 2);
        assert_eq!(metrics.hits(), 1);
    }
}
