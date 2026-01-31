use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use std::{num::NonZeroU32, sync::Arc};

pub type SharedRateLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

/// Create a new rate limiter with the specified requests per second and burst size
pub fn create_rate_limiter(requests_per_second: u32, burst_size: u32) -> SharedRateLimiter {
    let quota = Quota::per_second(NonZeroU32::new(requests_per_second).unwrap())
        .allow_burst(NonZeroU32::new(burst_size).unwrap());

    Arc::new(RateLimiter::direct(quota))
}

/// Rate limiting middleware using token bucket algorithm
pub async fn rate_limit_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // For the middleware to work, you'd pass the rate limiter via request extensions
    // or state. This is a simplified version.

    // In production, you would:
    // 1. Extract client identifier (IP, API key, etc.)
    // 2. Check the rate limiter for that client
    // 3. Return 429 if rate limited

    Ok(next.run(request).await)
}

/// Rate limiting middleware with state
pub async fn rate_limit_with_state(
    rate_limiter: SharedRateLimiter,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip rate limiting for health checks
    if request.uri().path() == "/health" {
        return Ok(next.run(request).await);
    }

    match rate_limiter.check() {
        Ok(_) => Ok(next.run(request).await),
        Err(_) => {
            tracing::warn!(
                "Rate limit exceeded for request to {}",
                request.uri().path()
            );
            Err(StatusCode::TOO_MANY_REQUESTS)
        }
    }
}

/// Per-IP rate limiter using DashMap for concurrent access
pub mod per_ip {
    use dashmap::DashMap;
    use governor::{
        clock::DefaultClock,
        state::{InMemoryState, NotKeyed},
        Quota, RateLimiter,
    };
    use std::{net::IpAddr, num::NonZeroU32, sync::Arc};

    pub struct IpRateLimiter {
        limiters: DashMap<IpAddr, Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>>,
        quota: Quota,
    }

    impl IpRateLimiter {
        pub fn new(requests_per_second: u32, burst_size: u32) -> Self {
            let quota = Quota::per_second(NonZeroU32::new(requests_per_second).unwrap())
                .allow_burst(NonZeroU32::new(burst_size).unwrap());

            Self {
                limiters: DashMap::new(),
                quota,
            }
        }

        pub fn check(&self, ip: IpAddr) -> bool {
            let limiter = self.limiters.entry(ip).or_insert_with(|| {
                Arc::new(RateLimiter::direct(self.quota))
            });

            limiter.check().is_ok()
        }

        /// Clean up old entries (call periodically)
        pub fn cleanup(&self) {
            // Remove entries that haven't been used recently
            // In production, you'd track last access time
            if self.limiters.len() > 10000 {
                self.limiters.retain(|_, _| true); // Placeholder
            }
        }
    }
}
