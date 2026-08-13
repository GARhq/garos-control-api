//! Tracing + (optional) OpenTelemetry init.

use crate::config::{LogFormat, Settings};
use opentelemetry::trace::TracerProvider as _;
use opentelemetry::KeyValue;
use opentelemetry_sdk::trace::TracerProvider;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

/// Guard returned by [`init`] — flushing telemetry on drop.
pub struct TelemetryGuard {
    provider: Option<TracerProvider>,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Some(p) = self.provider.take() {
            // Best-effort shutdown.
            for r in p.shutdown() {
                if let Err(e) = r {
                    eprintln!("OTel provider shutdown error: {e}");
                }
            }
        }
    }
}

/// Initialise global tracing subscribers.
///
/// - `Pretty`/`Compact` formats write to stdout.
/// - `Json` writes newline-delimited JSON.
/// - If `features.enable_otlp = true` and an OTLP endpoint is set, an
///   additional OTLP layer is installed.
pub fn init(settings: &Settings) -> anyhow::Result<TelemetryGuard> {
    let env_filter = EnvFilter::try_new(&settings.logging.level)
        .or_else(|_| EnvFilter::try_from_default_env())
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = match settings.logging.format {
        LogFormat::Json => tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_thread_ids(false)
            .with_level(true)
            .with_current_span(true)
            .with_span_list(false)
            .json()
            .boxed(),
        LogFormat::Pretty => tracing_subscriber::fmt::layer()
            .with_target(true)
            .pretty()
            .boxed(),
        LogFormat::Compact => tracing_subscriber::fmt::layer()
            .with_target(false)
            .compact()
            .boxed(),
    };

    let registry = tracing_subscriber::registry().with(env_filter).with(fmt_layer);

    let provider = if settings.features.enable_otlp {
        if let Some(endpoint) = settings.telemetry.otlp_endpoint.as_deref() {
            let exporter = opentelemetry_otlp::SpanExporter::builder()
                .with_tonic()
                .with_endpoint(endpoint)
                .build()?;

            let resource = opentelemetry_sdk::Resource::new(vec![
                KeyValue::new("service.name", settings.telemetry.service_name.clone()),
                KeyValue::new("deployment.environment", settings.telemetry.environment.clone()),
            ]);

            let provider = TracerProvider::builder()
                .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
                .with_resource(resource)
                .build();

            let tracer = provider.tracer(settings.telemetry.service_name.clone());
            let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
            registry.with(otel_layer).try_init().map_err(|e| {
                anyhow::anyhow!("failed to install OpenTelemetry layer: {e}")
            })?;
            Some(provider)
        } else {
            registry.try_init().map_err(|e| anyhow::anyhow!("init: {e}"))?;
            None
        }
    } else {
        registry.try_init().map_err(|e| anyhow::anyhow!("init: {e}"))?;
        None
    };

    Ok(TelemetryGuard { provider })
}

/// Build a span around a request. Used by [`crate::middleware::logging`].
pub fn make_request_span(
    trace_id: uuid::Uuid,
    method: &axum::http::Method,
    uri: &axum::http::Uri,
) -> tracing::Span {
    tracing::info_span!(
        "http_request",
        trace_id = %trace_id,
        http.method = %method,
        http.target = %uri,
        http.status_code = tracing::field::Empty,
        otel.kind = "server",
        otel.status_code = tracing::field::Empty,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LoggingSettings;

    #[test]
    fn init_with_pretty() {
        let mut s = Settings::load_from(Some("never")).ok();
        // settings may fail to load in tests; build a minimal one
        let mut settings = s.unwrap_or_else(|| {
            // fallback: build from disk
            panic!("settings required for telemetry test")
        });
        settings.logging = LoggingSettings {
            format: LogFormat::Pretty,
            level: "info".into(),
        };
        let _g = init(&settings).expect("init should succeed");
    }
}
