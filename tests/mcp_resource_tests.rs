use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;
use velib_mcp::mcp::server::McpServer;

/// Test that the stations/reference endpoint returns real station data
#[tokio::test]
async fn test_stations_reference_endpoint_returns_real_data() {
    // This test will initially fail because handle_resource returns hardcoded empty data
    let router = McpServer::new().router();
    
    let request = Request::builder()
        .uri("/resources/velib://stations/reference")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    
    let response = router.oneshot(request).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json_response: Value = serde_json::from_slice(&body).unwrap();
    
    // Currently fails: should have real station data, not empty array
    let stations = json_response["stations"].as_array().unwrap();
    assert!(!stations.is_empty(), "Stations array should not be empty");
    
    // Validate station structure
    let first_station = &stations[0];
    assert!(first_station["station_code"].is_string());
    assert!(first_station["name"].is_string());
    assert!(first_station["coordinates"]["latitude"].is_number());
    assert!(first_station["coordinates"]["longitude"].is_number());
    assert!(first_station["capacity"].is_number());
}

/// Test that the stations/realtime endpoint returns real availability data
#[tokio::test]
async fn test_stations_realtime_endpoint_returns_real_data() {
    let router = McpServer::new().router();
    
    let request = Request::builder()
        .uri("/resources/velib://stations/realtime")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    
    let response = router.oneshot(request).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json_response: Value = serde_json::from_slice(&body).unwrap();
    
    // Currently fails: should have real real-time data
    let stations = json_response["stations"].as_array().unwrap();
    assert!(!stations.is_empty(), "Real-time stations array should not be empty");
    
    // Validate real-time data structure
    let first_station = &stations[0];
    assert!(first_station["station_code"].is_string());
    assert!(first_station["bikes"]["mechanical"].is_number());
    assert!(first_station["bikes"]["electric"].is_number());
    assert!(first_station["available_docks"].is_number());
    assert!(first_station["status"].is_string());
    assert!(first_station["last_update"].is_string());
}

/// Test that the stations/complete endpoint returns combined data
#[tokio::test]
async fn test_stations_complete_endpoint_returns_combined_data() {
    let router = McpServer::new().router();
    
    let request = Request::builder()
        .uri("/resources/velib://stations/complete")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    
    let response = router.oneshot(request).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json_response: Value = serde_json::from_slice(&body).unwrap();
    
    // Currently fails: should have combined reference + real-time data
    let stations = json_response["stations"].as_array().unwrap();
    assert!(!stations.is_empty(), "Complete stations array should not be empty");
    
    // Validate combined data structure (both reference and real-time)
    let first_station = &stations[0];
    // Reference data (nested under "reference")
    assert!(first_station["reference"]["station_code"].is_string());
    assert!(first_station["reference"]["name"].is_string());
    assert!(first_station["reference"]["coordinates"]["latitude"].is_number());
    assert!(first_station["reference"]["capacity"].is_number());
    // Real-time data (when available)
    if first_station["real_time"].is_object() {
        let real_time = &first_station["real_time"];
        assert!(real_time["bikes"]["mechanical"].is_number());
        assert!(real_time["bikes"]["electric"].is_number());
        assert!(real_time["available_docks"].is_number());
    }
}

/// Test that the health endpoint returns real system metrics
#[tokio::test]
async fn test_health_endpoint_returns_real_metrics() {
    let router = McpServer::new().router();
    
    let request = Request::builder()
        .uri("/resources/velib://health")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    
    let response = router.oneshot(request).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json_response: Value = serde_json::from_slice(&body).unwrap();
    
    // Currently fails: should have real cache statistics, not hardcoded values
    assert_eq!(json_response["status"], "healthy");
    
    // Validate cache stats are real (not hardcoded)
    let cache_stats = &json_response["cache_stats"];
    assert!(cache_stats["hit_rate"].is_number());
    assert!(cache_stats["entries"].is_number());
    
    // Validate data source statuses are real
    let data_sources = &json_response["data_sources"];
    assert!(data_sources["real_time"]["status"].is_string());
    assert!(data_sources["real_time"]["last_update"].is_string());
    assert!(data_sources["reference"]["status"].is_string());
    
    // The hit_rate should not be exactly 0.85 (hardcoded value) when testing with real data
    let hit_rate = cache_stats["hit_rate"].as_f64().unwrap();
    assert_ne!(hit_rate, 0.85, "Hit rate should not be hardcoded 0.85");
}

/// Test error handling when data source is unavailable
#[tokio::test]
async fn test_resource_endpoints_handle_api_failures() {
    // This test verifies graceful degradation when the Paris Open Data API is unavailable
    let router = McpServer::new().router();
    
    let request = Request::builder()
        .uri("/resources/velib://stations/reference")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    
    let response = router.oneshot(request).await.unwrap();
    
    // Should still return 200 OK with appropriate error information
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json_response: Value = serde_json::from_slice(&body).unwrap();
    
    // Should include error information or fallback data
    assert!(json_response["metadata"].is_object());
    
    // If API fails, should indicate data source status
    if json_response["stations"].as_array().unwrap().is_empty() {
        let metadata = &json_response["metadata"];
        assert!(metadata["data_source_status"].is_string());
    }
}

/// Test that metadata includes accurate timestamps and data freshness
#[tokio::test] 
async fn test_resource_metadata_accuracy() {
    let router = McpServer::new().router();
    
    let request = Request::builder()
        .uri("/resources/velib://stations/reference")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    
    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json_response: Value = serde_json::from_slice(&body).unwrap();
    
    let metadata = &json_response["metadata"];
    
    // Validate metadata structure
    assert!(metadata["total_stations"].is_number());
    assert!(metadata["last_updated"].is_string());
    
    // Total stations should match actual array length (not hardcoded 0)
    let stations_count = json_response["stations"].as_array().unwrap().len();
    let metadata_count = metadata["total_stations"].as_u64().unwrap() as usize;
    assert_eq!(stations_count, metadata_count, "Metadata station count should match actual stations");
}

/// Performance test: Resource endpoints should respond within reasonable time
#[tokio::test]
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
    
    // Resource endpoints should respond within 10 seconds (real API calls can be slow)
    assert!(duration.as_secs() < 10, "Resource endpoint should respond within 10 seconds");
    
    // Should be reasonably fast even with real data
    assert!(duration.as_millis() < 8000, "Resource endpoint should respond within 8 seconds");
}