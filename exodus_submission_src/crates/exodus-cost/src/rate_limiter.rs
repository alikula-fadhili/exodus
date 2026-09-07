//! Dynamic provider rate limiting and adaptive jitter backoff middleware.

use reqwest::header::HeaderMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// Shared provider rate limiter protecting subagent workers from HTTP 429 cascades.
#[derive(Clone)]
pub struct ProviderRateLimiter {
    pub tpm_semaphore: Arc<Semaphore>,
    pub rpm_semaphore: Arc<Semaphore>,
    pub remaining_tokens: Arc<AtomicU64>,
    pub remaining_requests: Arc<AtomicU64>,
}

impl Default for ProviderRateLimiter {
    fn default() -> Self {
        Self::new(100, 1_000_000)
    }
}

impl ProviderRateLimiter {
    pub fn new(max_concurrent_requests: usize, max_tpm: usize) -> Self {
        Self {
            tpm_semaphore: Arc::new(Semaphore::new(max_tpm)),
            rpm_semaphore: Arc::new(Semaphore::new(max_concurrent_requests)),
            remaining_tokens: Arc::new(AtomicU64::new(max_tpm as u64)),
            remaining_requests: Arc::new(AtomicU64::new(max_concurrent_requests as u64)),
        }
    }

    /// Reads provider rate limit headers and updates live bucket levels.
    pub fn update_from_headers(&self, headers: &HeaderMap) {
        if let Some(rem_tok) = headers.get("x-ratelimit-remaining-tokens") {
            if let Ok(val) = rem_tok.to_str().unwrap_or("0").parse::<u64>() {
                self.remaining_tokens.store(val, Ordering::Relaxed);
            }
        }
        if let Some(rem_req) = headers.get("x-ratelimit-remaining-requests") {
            if let Ok(val) = rem_req.to_str().unwrap_or("0").parse::<u64>() {
                self.remaining_requests.store(val, Ordering::Relaxed);
            }
        }
    }

    /// Computes exponential backoff with full randomized jitter.
    pub fn compute_backoff_jitter(&self, attempt: u32, retry_after_hdr: Option<u64>) -> Duration {
        if let Some(secs) = retry_after_hdr {
            return Duration::from_secs(secs);
        }
        let base_ms = 500u64.saturating_mul(2u64.pow(attempt.min(6)));
        let capped_ms = base_ms.min(30_000);
        let jitter = rand::random::<u64>() % (capped_ms / 2 + 1);
        Duration::from_millis(capped_ms / 2 + jitter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_updating_and_backoff_jitter() {
        let limiter = ProviderRateLimiter::default();
        let mut headers = HeaderMap::new();
        headers.insert("x-ratelimit-remaining-tokens", "45000".parse().unwrap());
        headers.insert("x-ratelimit-remaining-requests", "88".parse().unwrap());

        limiter.update_from_headers(&headers);
        assert_eq!(limiter.remaining_tokens.load(Ordering::Relaxed), 45000);
        assert_eq!(limiter.remaining_requests.load(Ordering::Relaxed), 88);

        let duration_fixed = limiter.compute_backoff_jitter(1, Some(5));
        assert_eq!(duration_fixed, Duration::from_secs(5));

        let duration_jitter = limiter.compute_backoff_jitter(2, None);
        assert!(duration_jitter.as_millis() >= 1000);
        assert!(duration_jitter.as_millis() <= 30000);
    }
}
