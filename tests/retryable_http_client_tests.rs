//! HTTP-level integration tests for `RetryableHttpClient`.
//!
//! `src/data/retry.rs` exposes the retry policy logic and a small `reqwest`-
//! based HTTP client wrapper. The retry _policy_ is well covered by unit tests
//! against synthetic `Error` values, but the HTTP wrapper itself
//! (`RetryableHttpClient::get`, `get_with_query`, and the private
//! `check_response`) has no coverage of the actual response-mapping invariants:
//!
//! - 2xx ----> `Ok(Response)` with the body intact.
//! - 429 with `Retry-After` header --> `Error::RateLimited { retry_after_seconds: Some(N) }`.
//! - 429 without `Retry-After`     --> `Error::RateLimited { retry_after_seconds: None }`.
//! - 5xx                          --> `Error::Http(_)` (a retryable status).
//! - 4xx (non-429)                --> `Error::Http(_)` (a non-retryable status, fails fast).
//! - Query parameters from `get_with_query` reach the server unmodified.
//! - A transient 5xx that resolves on the next call eventually succeeds, with
//!   the upstream observing exactly two requests (initial + 1 retry).
//!
//! These invariants are reachable end-to-end by pointing the client at a tiny
//! in-process axum mock server bound to an ephemeral port, so no network
//! access (and no flaky external dependency) is required.

mod common;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use serde::Deserialize;
use velib_mcp::data::retry::{
    create_rate_limited_error, extract_retry_after_from_response, RetryableHttpClient,
};
use velib_mcp::data::{RetryConfig, RetryPolicy};
use velib_mcp::Error;

/// Build a `RetryableHttpClient` whose retry sleeps are zero-length so tests
/// stay fast even when retries are exercised.
fn fast_retry_client(max_attempts: u32) -> RetryableHttpClient {
    RetryableHttpClient::with_retry_policy(RetryPolicy::with_config(RetryConfig {
        max_attempts,
        base_delay_seconds: 0,
        max_delay_seconds: 0,
        use_jitter: false,
    }))
}

/// Spawn a fresh axum router on an ephemeral port and return its base URL
/// (`http://127.0.0.1:<port>`). The caller drives requests against it via
/// `RetryableHttpClient`.
async fn spawn_mock(router: Router) -> String {
    let port = common::find_available_port();
    let addr: std::net::SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    // Give the listener a moment to start accepting connections.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    format!("http://127.0.0.1:{port}")
}

// --- 200 OK ---

#[tokio::test]
async fn get_returns_ok_response_with_body_on_200() {
    let router = Router::new().route("/ok", get(|| async { "hello world" }));
    let base = spawn_mock(router).await;

    let client = fast_retry_client(0);
    let response = client.get(&format!("{base}/ok")).await.expect("ok");
    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert_eq!(body, "hello world");
}

// --- 429 Rate limited ---

#[tokio::test]
async fn get_maps_429_with_retry_after_to_rate_limited_with_seconds() {
    let router = Router::new().route(
        "/limited",
        get(|| async {
            (
                StatusCode::TOO_MANY_REQUESTS,
                [("retry-after", "42")],
                "slow down",
            )
        }),
    );
    let base = spawn_mock(router).await;

    // max_attempts=0 ensures we don't waste time retrying the rate-limit;
    // we only care that the very first response is mapped correctly.
    let client = fast_retry_client(0);
    let err = client.get(&format!("{base}/limited")).await.unwrap_err();

    match err {
        Error::RateLimited {
            retry_after_seconds: Some(n),
        } => assert_eq!(n, 42),
        other => panic!("expected RateLimited(Some(42)), got {other:?}"),
    }
}

#[tokio::test]
async fn get_maps_429_without_retry_after_to_rate_limited_with_none() {
    let router = Router::new().route(
        "/limited",
        get(|| async { (StatusCode::TOO_MANY_REQUESTS, "slow down") }),
    );
    let base = spawn_mock(router).await;

    let client = fast_retry_client(0);
    let err = client.get(&format!("{base}/limited")).await.unwrap_err();

    match err {
        Error::RateLimited {
            retry_after_seconds: None,
        } => {}
        other => panic!("expected RateLimited(None), got {other:?}"),
    }
}

// --- 5xx (retryable) ---

#[tokio::test]
async fn get_maps_5xx_to_http_error() {
    let router = Router::new().route(
        "/boom",
        get(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "boom") }),
    );
    let base = spawn_mock(router).await;

    // Exhaust retries quickly with max_attempts=0 so we get the final error
    // synchronously without sleeping.
    let client = fast_retry_client(0);
    let err = client.get(&format!("{base}/boom")).await.unwrap_err();

    match err {
        Error::Http(http_err) => {
            assert_eq!(http_err.status().map(|s| s.as_u16()), Some(500));
        }
        other => panic!("expected Http(500), got {other:?}"),
    }
}

// --- 4xx (non-429, non-retryable) ---

#[tokio::test]
async fn get_maps_4xx_to_http_error_and_fails_fast() {
    // Count calls: a 404 (non-retryable) must not be retried, even when
    // max_attempts > 0.
    let count = Arc::new(AtomicU32::new(0));
    let count_state = Arc::clone(&count);
    let router = Router::new()
        .route(
            "/missing",
            get(|State(c): State<Arc<AtomicU32>>| async move {
                c.fetch_add(1, Ordering::SeqCst);
                (StatusCode::NOT_FOUND, "not here")
            }),
        )
        .with_state(count_state);
    let base = spawn_mock(router).await;

    let client = fast_retry_client(3); // would retry 3 times if 404 were retryable
    let err = client.get(&format!("{base}/missing")).await.unwrap_err();

    match err {
        Error::Http(http_err) => {
            assert_eq!(http_err.status().map(|s| s.as_u16()), Some(404));
        }
        other => panic!("expected Http(404), got {other:?}"),
    }
    // Exactly one upstream call: 404 is not retryable.
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

// --- Retry on transient 5xx ---

#[tokio::test]
async fn retry_recovers_from_transient_5xx() {
    // First call returns 503, second call returns 200. The client must succeed
    // overall and the upstream must observe exactly 2 calls.
    let count = Arc::new(AtomicU32::new(0));
    let count_state = Arc::clone(&count);
    let router = Router::new()
        .route(
            "/flaky",
            get(|State(c): State<Arc<AtomicU32>>| async move {
                let n = c.fetch_add(1, Ordering::SeqCst);
                if n == 0 {
                    (StatusCode::SERVICE_UNAVAILABLE, "try again").into_response()
                } else {
                    "recovered".into_response()
                }
            }),
        )
        .with_state(count_state);
    let base = spawn_mock(router).await;

    let client = fast_retry_client(3);
    let response = client.get(&format!("{base}/flaky")).await.expect("ok");
    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert_eq!(body, "recovered");
    assert_eq!(
        count.load(Ordering::SeqCst),
        2,
        "expected exactly initial + 1 retry"
    );
}

// --- Query parameters reach the server ---

#[derive(Deserialize)]
struct EchoQuery {
    limit: u32,
    offset: u32,
}

#[tokio::test]
async fn get_with_query_passes_parameters_to_server() {
    let router = Router::new().route(
        "/echo",
        get(|Query(q): Query<EchoQuery>| async move {
            format!("limit={};offset={}", q.limit, q.offset)
        }),
    );
    let base = spawn_mock(router).await;

    let client = fast_retry_client(0);
    let params = &[("limit", "100"), ("offset", "200")];
    let response = client
        .get_with_query(&format!("{base}/echo"), params)
        .await
        .expect("ok");
    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert_eq!(body, "limit=100;offset=200");
}

// --- Retry exhaustion on persistent 5xx ---

#[tokio::test]
async fn retry_exhaustion_on_persistent_5xx_returns_last_error() {
    // Always-failing upstream: max_attempts=2 => exactly 3 total calls
    // (initial + 2 retries), and the final error should be Http(503).
    let count = Arc::new(AtomicU32::new(0));
    let count_state = Arc::clone(&count);
    let router = Router::new()
        .route(
            "/down",
            get(|State(c): State<Arc<AtomicU32>>| async move {
                c.fetch_add(1, Ordering::SeqCst);
                (StatusCode::SERVICE_UNAVAILABLE, "down")
            }),
        )
        .with_state(count_state);
    let base = spawn_mock(router).await;

    let client = fast_retry_client(2);
    let err = client.get(&format!("{base}/down")).await.unwrap_err();
    match err {
        Error::Http(http_err) => {
            assert_eq!(http_err.status().map(|s| s.as_u16()), Some(503));
        }
        other => panic!("expected Http(503), got {other:?}"),
    }
    assert_eq!(count.load(Ordering::SeqCst), 3);
}

// --- Helper functions on real responses ---

/// `extract_retry_after_from_response` is exercised indirectly via the 429
/// tests above, but verifying it directly against a real `reqwest::Response`
/// guards the helper from drifting if the header lookup logic is refactored.
#[tokio::test]
async fn extract_retry_after_helper_reads_real_header() {
    async fn handler(headers: HeaderMap) -> Response {
        // Echo whatever Retry-After value the test asks for, so the same
        // upstream serves both branches.
        let value = headers
            .get("x-want-retry-after")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        if value.is_empty() {
            (StatusCode::TOO_MANY_REQUESTS, "no header").into_response()
        } else {
            let mut h = HeaderMap::new();
            h.insert("retry-after", value.parse().unwrap());
            (StatusCode::TOO_MANY_REQUESTS, h, "with header").into_response()
        }
    }
    let router = Router::new().route("/r", get(handler));
    let base = spawn_mock(router).await;

    let client = reqwest::Client::new();
    let with_header = client
        .get(format!("{base}/r"))
        .header("x-want-retry-after", "13")
        .send()
        .await
        .unwrap();
    assert_eq!(extract_retry_after_from_response(&with_header), Some(13));

    let without_header = client.get(format!("{base}/r")).send().await.unwrap();
    assert_eq!(extract_retry_after_from_response(&without_header), None);
}

/// `create_rate_limited_error` must read `Retry-After` from the response and
/// surface it on the resulting `Error::RateLimited` variant.
#[tokio::test]
async fn create_rate_limited_error_carries_retry_after() {
    let router = Router::new().route(
        "/r",
        get(|| async {
            (
                StatusCode::TOO_MANY_REQUESTS,
                [("retry-after", "7")],
                "later",
            )
        }),
    );
    let base = spawn_mock(router).await;

    let response = reqwest::Client::new()
        .get(format!("{base}/r"))
        .send()
        .await
        .unwrap();
    let err = create_rate_limited_error(&response);
    match err {
        Error::RateLimited {
            retry_after_seconds: Some(7),
        } => {}
        other => panic!("expected RateLimited(Some(7)), got {other:?}"),
    }
}
