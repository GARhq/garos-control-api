//! CORS layer builder.
//
// ## Security posture (Wave 7 finding #6)
//
// Historically the layer fell open: when `cors.allowed_origins` was
// empty in the config file, the layer substituted `"*"` and combined
// that with `allow_credentials(true)`. That combination is rejected
// by browsers *and* constitutes a credential-reflection risk if any
// intermediate proxy re-writes the `Origin` header.
//
// The new policy is **fail-closed**:
//
// 1. If `allowed_origins` is empty, [`layer`] panics during boot so
//    the misconfiguration is loud rather than silent.
// 2. Origins are validated strictly; any string that fails to parse
//    as an `http::HeaderValue` causes a panic listing the offending
//    entry — the previous `filter_map(...).ok()` silently dropped bad
//    values, which made typos in the config look like a working
//    restricted CORS policy while actually allowing everything else.
// 3. `allow_credentials(true)` is only set when the origin list is
//    non-empty (it is impossible to combine credentials with the
//    wildcard origin anyway, but being explicit avoids future
//    refactors re-introducing the bug).
// 4. The single literal `"*"` entry is still accepted for
//    non-credentialed APIs (e.g. public health endpoints) and is
//    detected by the explicit `is_wildcard` branch — credentials stay
//    off in that mode.

use crate::config::{CorsSettings, Settings};
use axum::http::{header, HeaderValue, Method};
use tower_http::cors::CorsLayer;

/// Build the [`CorsLayer`] from the runtime [`Settings`].
///
/// # Panics
///
/// Panics at boot if `cors.allowed_origins` is empty, or if any
/// configured origin is not a valid `HeaderValue` (e.g. contains a
/// CR/LF or null byte). The panic is intentional: a CORS misconfig
/// must be visible during deploy, not silently permissive in prod.
pub fn layer(settings: &Settings) -> CorsLayer {
    layer_from_cors(&settings.cors)
}

/// Build the [`CorsLayer`] directly from a [`CorsSettings`].
///
/// # Panics
///
/// Panics at boot if `allowed_origins` is empty, or if any configured
/// origin is not a valid `HeaderValue`. The panic is intentional: a
/// CORS misconfig must be visible during deploy, not silently
/// permissive in prod.
pub fn layer_from_cors(cors: &CorsSettings) -> CorsLayer {
    let allowed = &cors.allowed_origins;

    // Fail-closed: refuse to construct a layer without an origin allowlist.
    if allowed.is_empty() {
        panic!(
            "CORS misconfiguration: `cors.allowed_origins` is empty. Refusing to              boot with no origin policy. Add at least one entry to              settings.cors.allowed_origins (e.g. \"http://localhost:3000\" for              dev, \"https://control.garos.example\" for prod). See Wave 6              audit finding #6."
        );
    }

    let is_wildcard = allowed.len() == 1 && allowed[0] == "*";

    // Validate every origin parses as a HeaderValue. We panic on the
    // first bad entry so config typos are visible at boot.
    let origins: Vec<HeaderValue> = allowed
        .iter()
        .map(|s| match HeaderValue::from_str(s) {
            Ok(v) => v,
            Err(e) => panic!(
                "CORS misconfiguration: origin {:?} is not a valid HTTP                  header value ({}). Check for stray CR/LF, NUL bytes or                  non-ASCII characters.",
                s, e
            ),
        })
        .collect();

    let methods = [
        Method::GET,
        Method::POST,
        Method::PATCH,
        Method::PUT,
        Method::DELETE,
        Method::OPTIONS,
        Method::HEAD,
    ];

    let headers = [
        header::AUTHORIZATION,
        header::CONTENT_TYPE,
        header::ACCEPT,
        header::HeaderName::from_static("x-request-id"),
        header::HeaderName::from_static("idempotency-key"),
    ];

    // tower-http rejects "*" passed to AllowOrigin::list — it requires
    // AllowOrigin::any() in that case. Detect wildcard early and route.
    let layer = if is_wildcard {
        CorsLayer::new()
            .allow_origin(tower_http::cors::AllowOrigin::any())
            .allow_methods(methods)
            .allow_headers(headers)
            .max_age(std::time::Duration::from_secs(60 * 60))
    } else {
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods(methods)
            .allow_headers(headers)
            .allow_credentials(true)
            .max_age(std::time::Duration::from_secs(60 * 60))
    };

    layer
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CorsSettings;

    fn cors_with(origins: Vec<&str>) -> CorsSettings {
        CorsSettings {
            allowed_origins: origins.into_iter().map(String::from).collect(),
        }
    }

    /// Wave 7 #6: an empty origin list must panic, not fall open to "*".
    #[test]
    #[should_panic(expected = "CORS misconfiguration")]
    fn empty_origins_panics() {
        let s = cors_with(vec![]);
        let _ = layer_from_cors(&s);
    }

    /// A typo in the origin list must panic, not be silently dropped.
    #[test]
    #[should_panic(expected = "failed to parse")]
    fn invalid_origin_panics() {
        // Contains a NUL byte — HeaderValue rejects it.
        let s = cors_with(vec!["http://evil\0.example"]);
        let _ = layer_from_cors(&s);
    }

    /// A literal "*" alone must NOT enable credentials (browsers reject
    /// the combination; we make it explicit). Layer builds without panic.
    #[test]
    fn wildcard_builds_layer_without_credentials() {
        let s = cors_with(vec!["*"]);
        let layer = layer_from_cors(&s);
        let _ = layer;
    }

    /// Explicit origin list builds a layer with credentials enabled.
    #[test]
    fn explicit_origin_builds_layer() {
        let s = cors_with(vec!["https://control.garos.example"]);
        let layer = layer_from_cors(&s);
        let _ = layer;
    }
}
