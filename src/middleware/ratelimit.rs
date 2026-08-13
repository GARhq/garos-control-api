//! Per-IP rate limiting via `governor`.

use crate::config::Settings;
use axum::body::Body;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::Response;
use axum::middleware::Next;
use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

type Limiter = RateLimiter<NotKeyed, InMemoryState, DefaultClock>;

/// Shared map of `IpAddr -> per-IP rate limiter`.
#[derive(Clone, Default)]
pub struct RateLimitRegistry {
    inner: Arc<Mutex<HashMap<IpAddr, Arc<Limiter>>>>,
    rpm: u32,
    burst: u32,
}

impl RateLimitRegistry {
    pub fn new(rpm: u32, burst: u32) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            rpm,
            burst,
        }
    }

    fn limiter(&self, ip: IpAddr) -> Arc<Limiter> {
        let mut g = self.inner.lock();
        g.entry(ip)
            .or_insert_with(|| {
                let r = NonZeroU32::new(self.rpm.max(1)).unwrap();
                let b = NonZeroU32::new(self.burst.max(1)).unwrap();
                Arc::new(RateLimiter::direct(Quota::per_minute(r).allow_burst(b)))
            })
            .clone()
    }
}

/// Middleware function (use via `axum::middleware::from_fn_with_state`).
pub async fn middleware(
    State(reg): State<RateLimitRegistry>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Result<Response<Body>, crate::error::AppError> {
    let lim = reg.limiter(addr.ip());
    match lim.check() {
        Ok(_) => Ok(next.run(req).await),
        Err(_) => {
            let wait = lim
                .check()
                .wait_time_from(governor::clock::Clock::now(&DefaultClock::default()));
            Err(crate::error::AppError::RateLimited {
                retry_after_secs: wait.map_or(1, |d| d.as_secs().max(1)),
            })
        }
    }
}

/// Build the layer (no-op shim, we install via `from_fn_with_state` in router).
pub fn layer_from_settings(settings: &Settings) -> RateLimitRegistry {
    RateLimitRegistry::new(settings.ratelimit.requests_per_minute, settings.ratelimit.burst)
}

/// Sweep stale entries every [`Duration`].
pub fn gc(registry: &RateLimitRegistry, max_entries: usize) {
    let mut g = registry.inner.lock();
    while g.len() > max_entries {
        if let Some(k) = g.keys().next().copied() {
            g.remove(&k);
        } else {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_limiter_for_ip() {
        let r = RateLimitRegistry::new(60, 5);
        let _ = r.limiter("127.0.0.1".parse().unwrap());
        let _ = r.limiter("127.0.0.1".parse().unwrap());
    }
}
