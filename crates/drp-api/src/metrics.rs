//! Prometheus metrics exposition and HTTP instrumentation.
//!
//! Metrics are process-local and scraped via `GET /metrics`.
//! The default recorder is installed once at process start.

use std::sync::OnceLock;
use std::time::Instant;

use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use metrics::{counter, describe_counter, describe_histogram, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

static HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// Install the Prometheus recorder (idempotent).
pub fn init_metrics() -> drp_common::Result<()> {
    if HANDLE.get().is_some() {
        return Ok(());
    }

    let handle = PrometheusBuilder::new()
        .install_recorder()
        .map_err(|e| drp_common::Error::internal(format!("metrics recorder: {e}")))?;

    describe_counter!(
        "http_requests_total",
        "Total number of HTTP requests handled"
    );
    describe_histogram!(
        "http_request_duration_seconds",
        "HTTP request latency in seconds"
    );
    describe_counter!("http_responses_total", "HTTP responses by status class");
    describe_counter!("drp_process_starts_total", "Process start count");

    counter!("drp_process_starts_total").increment(1);

    let _ = HANDLE.set(handle);
    Ok(())
}

/// Axum handler: Prometheus text exposition format.
pub async fn metrics_handler() -> impl IntoResponse {
    let body = HANDLE
        .get()
        .map(|h| h.render())
        .unwrap_or_else(|| "# metrics recorder not initialized\n".into());
    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        body,
    )
}

/// Middleware that records request count and latency.
pub async fn track_http_metrics(req: Request<Body>, next: Next) -> Response {
    let method = req.method().as_str().to_owned();
    let path = normalize_path(req.uri().path());
    let start = Instant::now();

    let response = next.run(req).await;

    let status = response.status().as_u16();
    let class = status_class(status);
    let elapsed = start.elapsed().as_secs_f64();

    counter!(
        "http_requests_total",
        "method" => method.clone(),
        "path" => path.clone()
    )
    .increment(1);

    counter!(
        "http_responses_total",
        "method" => method.clone(),
        "path" => path.clone(),
        "status_class" => class
    )
    .increment(1);

    histogram!(
        "http_request_duration_seconds",
        "method" => method,
        "path" => path
    )
    .record(elapsed);

    response
}

fn status_class(code: u16) -> &'static str {
    match code {
        100..=199 => "1xx",
        200..=299 => "2xx",
        300..=399 => "3xx",
        400..=499 => "4xx",
        _ => "5xx",
    }
}

/// Collapse dynamic path segments so cardinality stays bounded.
fn normalize_path(path: &str) -> String {
    // Keep health/metrics exact; collapse ULID-looking segments.
    let mut out = String::with_capacity(path.len());
    for (i, seg) in path.split('/').enumerate() {
        if i > 0 {
            out.push('/');
        }
        if seg.is_empty() {
            continue;
        }
        if looks_like_id(seg) {
            out.push_str(":id");
        } else {
            out.push_str(seg);
        }
    }
    if out.is_empty() {
        "/".into()
    } else {
        out
    }
}

fn looks_like_id(s: &str) -> bool {
    // ULID is 26 crockford base32 chars; also accept UUID-ish.
    (s.len() == 26 && s.chars().all(|c| c.is_ascii_alphanumeric()))
        || (s.len() == 36 && s.chars().filter(|c| *c == '-').count() == 4)
}
