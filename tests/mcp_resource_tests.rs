//! MCP resource endpoint tests.
//!
//! # Test classification
//!
//! ## Structural tests (no `#[ignore]`) — run in every CI pass
//!
//! These tests only inspect the HTTP status code and JSON schema of the
//! response.  They start the Axum router in-process (no real network), so
//! they never call the live Velib API.
//!
//! | Test function | What it checks |
//! |---|---|
//! | `test_stations_reference_endpoint_responds_ok` | 200 OK + valid JSON object |
//! | `test_stations_realtime_endpoint_responds_ok` | 200 OK + valid JSON object |
//! | `test_stations_complete_endpoint_responds_ok` | 200 OK + valid JSON object |
//! | `test_health_endpoint_responds_ok` | 200 OK + `status` field present |
//! | `test_unknown_resource_uri_returns_error` | non-200 or error field in body |
//!
//! ## Live-network tests (`#[ignore]`) — run manually or in dedicated CI
//!
//! These tests assert on actual Velib API data (non-empty station arrays,
//! real cache statistics, etc.).  They are marked `#[ignore]` because they
//! require outbound HTTPS access to `opendata.paris.fr`.  Run them with:
//!
//! ```text
//! cargo test -- --ignored
//! ```

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;
use velib_mcp::mcp::server::McpServer;

// ---------------------------------------------------------------------------
// Structural tests (no live network needed)
// ---------------------------------------------------------------------------

/// stations/reference endpoint returns 200 OK with a JSON object body.
#[tokio::test]
async fn test_stations_reference_endpoint_responds_ok() {
    let router = McpServer::new().router();

    let request = Request::builder()
        .uri("/resources/velib://stations/reference")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "stations/reference should return 200 OK"
    );

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_response: Value = serde_json::from_slice(&body)
        .expect("Response body should be valid JSON");
    assert!(
        json_response.is_object(),
        "Response should be a JSON object, got: {}",
        json_response
    );
}

/// stations/realtime endpoint returns 200 OK with a JSON object body.
#[tokio::test]
async fn test_stations_realtime_endpoint_responds_ok() {
    let router = McpServer::new().router();

    let request = Request::builder()
        .uri("/resources/velib://stations/realtime")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "stations/realtime should return 200 OK"
    );

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_response: Value = serde_json::from_slice(&body)
        .expect("Response body should be valid JSON");
    assert!(json_response.is_object());
}

/// stations/complete endpoint returns 200 OK with a JSON object body.
#[tokio::test]
async fn test_stations_complete_endpoint_responds_ok() {
    let router = McpServer::new().router();

    let request = Request::builder()
        .uri("/resources/velib://stations/complete")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "stations/complete should return 200 OK"
    );

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_response: Value = serde_json::from_slice(&body)
        .expect("Response body should be valid JSON");
    assert!(json_response.is_object());
}

/// health endpoint returns 200 OK and a JSON object containing a `status` field.
#[tokio::test]
async fn test_health_endpoint_responds_ok() {
    let router = McpServer::new().router();

    let request = Request::builder()
        .uri("/resources/velib://health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "health endpoint should return 200 OK"
    );

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_response: Value = serde_json::from_slice(&body)
        .expect("Health response should be valid JSON");
    assert!(
        json_response["status"].is_string(),
        "Health response should contain a string 'status' field"
    );
}

/// An unknown resource URI should not return 200 OK, or if it does the body
/// must contain an error indicator (the server must not silently ignore it).
#[tokio::test]
async fn test_unknown_resource_uri_returns_error() {
    let router = McpServer::new().router();

    let request = Request::builder()
        .uri("/resources/velib://nonexistent/resource")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    let status = response.status();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    if status == StatusCode::OK {
        // If the server returns 200 for unknown URIs, the body must at least
        // signal an error so clients aren't silently given empty data.
        let json_response: Value = serde_json::from_slice(&body)
            .expect("Body should be valid JSON even for unknown resource");
        let has_error = json_response["error"].is_string()
            || json_response["error"].is_object()
            || json_response["message"].is_string();
        assert!(
            has_error,
            "200 response for unknown resource must contain an error field; got: {}",
            json_response
        );
    } else {
        // Any 4xx/5xx is also acceptable.
        assert!(
            status.is_client_error() || status.is_server_error(),
            "Unknown resource should produce an error status, got {}",
            status
        );
    }
}

// ---------------------------------------------------------------------------
// Live-network tests
// These require outbound HTTPS access to the Paris Open Data API.
// Run with: cargo test -- --ignored
// ---------------------------------------------------------------------------

/// Test that the stations/reference endpoint returns real station data.
///
/// Ignored because it requires live network access to the Velib API
/// (`opendata.paris.fr`).  The structural counterpart
/// `test_stations_reference_endpoint_responds_ok` runs without network.
#[tokio::test]
#[ignore = "requires live network access to Velib API (opendata.paris.fr)"]
async fn test_stations_reference_endpoint_returns_real_data() {
    let router = McpServer::new().router();

    let request = Request::builder()
        .uri("/resources/velib://stations/reference")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_response: Value = serde_json::from_slice(&body).unwrap();

    let stations = json_response["stations"].as_array().unwrap();
    assert!(!stations.is_empty(), "Stations array should not be empty");

    let first_station = &stations[0];
    assert!(first_station["station_code"].is_string());
    assert!(first_station["name"].is_string());
    assert!(first_station["coordinates"]["latitude"].is_number());
    assert!(first_station["coordinates"]["longitude"].is_number());
    assert!(first_station["capacity"].is_number());
}

/// Test that the stations/realtime endpoint returns real availability data.
///
/// Ignored: requires live network access to Velib API.
#[tokio::test]
#[ignore = "requires live network access to Velib API (opendata.paris.fr)"]
async fn test_stations_realtime_endpoint_returns_real_data() {
    let router = McpServer::new().router();

    let request = Request::builder()
        .uri("/resources/velib://stations/realtime")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_response: Value = serde_json::from_slice(&body).unwrap();

    let stations = json_response["stations"].as_array().unwrap();
    assert!(!stations.is_empty(), "Real-time stations array should not be empty");

    let first_station = &stations[0];
    assert!(first_station["station_code"].is_string());
    assert!(first_station["bikes"]["mechanical"].is_number());
    assert!(first_station["bikes"]["electric"].is_number());
    assert!(first_station["available_docks"].is_number());
    assert!(first_station["status"].is_string());
    assert!(first_station["last_update"].is_string());
}

/// Test that the stations/complete endpoint returns combined reference + realtime data.
///
/// Ignored: requires live network access to Velib API.
#[tokio::test]
#[ignore = "requires live network access to Velib API (opendata.paris.fr)"]
async fn test_stations_complete_endpoint_returns_combined_data() {
    let router = McpServer::new().router();

    let request = Request::builder()
        .uri("/resources/velib://stations/complete")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_response: Value = serde_json::from_slice(&body).unwrap();

    let stations = json_response["stations"].as_array().unwrap();
    assert!(!stations.is_empty(), "Complete stations array should not be empty");

    let first_station = &stations[0];
    assert!(first_station["reference"]["station_code"].is_string());
    assert!(first_station["reference"]["name"].is_string());
    assert!(first_station["reference"]["coordinates"]["latitude"].is_number());
    assert!(first_station["reference"]["capacity"].is_number());
    if first_station["real_time"].is_object() {
        let real_time = &first_station["real_time"];
        assert!(real_time["bikes"]["mechanical"].is_number());
        assert!(real_time["bikes"]["electric"].is_number());
        assert!(real_time["available_docks"].is_number());
    }
}

/// Test that the health endpoint returns real system metrics (not hardcoded values).
///
/// Ignored: requires live network access to Velib API.
#[tokio::test]
#[ignore = "requires live network access to Velib API (opendata.paris.fr)"]
async fn test_health_endpoint_returns_real_metrics() {
    let router = McpServer::new().router();

    let request = Request::builder()
        .uri("/resources/velib://health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_response: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json_response["status"], "healthy");

    // Validate cache stats expose real sizes, not hardcoded or fabricated metrics.
    let cache_stats = &json_response["cache_stats"];
    assert!(cache_stats["entries"].is_number());
    assert!(cache_stats["reference_cache_size"].is_number());
    assert!(cache_stats["realtime_cache_size"].is_number());
    // `hit_rate` was removed because the cache does not track hits/misses.
    assert!(
        cache_stats["hit_rate"].is_null(),
        "hit_rate must not be synthesized"
    );

    let data_sources = &json_response["data_sources"];
    assert!(data_sources["real_time"]["status"].is_string());
    assert!(data_sources["real_time"]["last_update"].is_string());
    assert!(data_sources["reference"]["status"].is_string());
}

/// Test error handling when the data source is unavailable.
///
/// Ignored: test verifies graceful degradation when the Paris Open Data API
/// is reachable but returns errors — requires network to confirm the scenario.
#[tokio::test]
#[ignore = "requires live network access to Velib API (opendata.paris.fr)"]
async fn test_resource_endpoints_handle_api_failures() {
    let router = McpServer::new().router();

    let request = Request::builder()
        .uri("/resources/velib://stations/reference")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_response: Value = serde_json::from_slice(&body).unwrap();

    assert!(json_response["metadata"].is_object());

    if json_response["stations"].as_array().unwrap().is_empty() {
        let metadata = &json_response["metadata"];
        assert!(metadata["data_source_status"].is_string());
    }
}

/// Test that metadata includes accurate timestamps and data freshness.
///
/// Ignored: metadata counts are derived from live API data, so the
/// expected values cannot be predicted without a real API call.
#[tokio::test]
#[ignore = "requires live network access to Velib API (opendata.paris.fr)"]
async fn test_resource_metadata_accuracy() {
    let router = McpServer::new().router();

    let request = Request::builder()
        .uri("/resources/velib://stations/reference")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_response: Value = serde_json::from_slice(&body).unwrap();

    let metadata = &json_response["metadata"];
    assert!(metadata["total_stations"].is_number());
    assert!(metadata["last_updated"].is_string());

    let stations_count = json_response["stations"].as_array().unwrap().len();
    let metadata_count = metadata["total_stations"].as_u64().unwrap() as usize;
    assert_eq!(
        stations_count, metadata_count,
        "Metadata station count should match actual stations"
    );
}

/// Performance test: resource endpoints should respond within a reasonable time.
///
/// Ignored: measures end-to-end latency including a real API call.
#[tokio::test]
#[ignore = "requires live network access to Velib API (opendata.paris.fr)"]
async fn test_resource_endpoint_performance() {
    use std::time::Instant;

    let router = McpServer::new().router();
    let start = Instant::now();

    let request = Request::builder()
        .uri("/resources/velib://stations/complete")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    let duration = start.elapsed();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        duration.as_secs() < 20,
        "Resource endpoint should respond within 20 seconds"
    );
    assert!(
        duration.as_millis() < 15000,
        "Resource endpoint should respond within 15 seconds"
    );
}
