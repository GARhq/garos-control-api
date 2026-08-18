//! Idempotency-Key middleware + storage.
//!
//! Stores responses keyed by `(user_id, key)` for a configurable TTL and
//! replays them on duplicate POSTs.

use crate::error::AppError;
use axum::body::{to_bytes, Body};
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant as StdInstant};
use axum::http::StatusCode;

const MAX_BODY: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    pub status: u16,
    pub body: Vec<u8>,
    #[serde(with = "instant_serde")]
    pub stored_at: std::time::Instant,
}

mod instant_serde {
    use once_cell::sync::Lazy;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    pub(super) fn serialize<S: Serializer>(t: &Instant, s: S) -> Result<S::Ok, S::Error> {
        // `saturating_duration_since` returns `Duration` directly, avoiding
        // an `Instant::duration_since` Result that gets shadowed by chrono::Duration.
        let dur = t.saturating_duration_since(*REGRESS);
        dur.as_secs().serialize(s)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Instant, D::Error> {
        let secs = u64::deserialize(d)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();
        let offset = now.saturating_sub(secs);
        Ok(*REGRESS + Duration::from_secs(offset))
    }

    static REGRESS: Lazy<Instant> = Lazy::new(Instant::now);
}

#[derive(Debug, Default)]
pub struct IdempotencyStore {
    inner: DashMap<String, CachedResponse>,
    ttl: Duration,
}

impl IdempotencyStore {
    pub fn new(ttl: Duration) -> Self {
        Self {
            inner: DashMap::new(),
            ttl,
        }
    }

    pub fn put(&self, key: String, status: u16, body: Vec<u8>) {
        self.inner.insert(
            key,
            CachedResponse {
                status,
                body,
                stored_at: StdInstant::now(),
            },
        );
    }

    pub fn get(&self, key: &str) -> Option<CachedResponse> {
        let entry = self.inner.get(key)?;
        if entry.stored_at.elapsed() > self.ttl {
            drop(entry);
            self.inner.remove(key);
            return None;
        }
        Some(entry.clone())
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn gc(&self) {
        let now = StdInstant::now();
        self.inner
            .retain(|_, v| now.duration_since(v.stored_at) <= self.ttl);
    }
}

pub async fn middleware(req: Request, next: Next) -> Response {
    // Only act on POST / PUT / PATCH with a header.
    let method = req.method().clone();
    if !matches!(method.as_str(), "POST" | "PUT" | "PATCH") {
        return next.run(req).await;
    }
    let key = req
        .headers()
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let key = match key {
        Some(k) if !k.is_empty() => k,
        _ => return next.run(req).await,
    };
    let user_id = req
        .extensions()
        .get::<crate::auth::extractor::CurrentUser>()
        .map(|u| u.id.to_string())
        .unwrap_or_else(|| "anon".to_string());
    let store_key = format!("{user_id}:{key}");

    if let Some(state) = req.extensions().get::<axum::extract::State<crate::state::AppState>>() {
        if let Some(hit) = state.idempotency.get(&store_key) {
            let mut resp = (StatusCode::from_u16(hit.status).unwrap(), hit.body.clone())
                .into_response();
            resp.headers_mut()
                .insert("idempotency-replayed", "true".parse().unwrap());
            return resp;
        }
    }

    // Buffer request body so we can replay it after the inner handler reads it.
    let (parts, body) = req.into_parts();
    let bytes = match to_bytes(body, MAX_BODY).await {
        Ok(b) => b,
        Err(e) => {
            return AppError::BadRequest(format!("body too large: {e}")).into_response();
        }
    };
    let req = Request::from_parts(parts, Body::from(bytes));

    let resp = next.run(req).await;
    let (parts, body) = resp.into_parts();
    let body_bytes = to_bytes(body, MAX_BODY).await.unwrap_or_default();
    if let Some(state) = parts.extensions.get::<axum::extract::State<crate::state::AppState>>() {
        if parts.status.is_success() {
            state
                .idempotency
                .put(store_key, parts.status.as_u16(), body_bytes.to_vec());
        }
    }
    Response::from_parts(parts, Body::from(body_bytes))
}
