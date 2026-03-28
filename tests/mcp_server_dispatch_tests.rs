//! Tests for McpServer JSON-RPC dispatch without live network access.
//!
//! These exercise the HTTP routing layer, JSON-RPC parsing, and method dispatch
//! using axum's oneshot testing -- no server process needed.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use velib_mcp::mcp::server::McpServer;

async fn post_mcp(router: axum::Router, body: Value) -> (StatusCode, Value) {
    let request = Request::builder()
        .uri("/mcp")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    (status, json)
}

#[tokio::test]
async fn tools_list_returns_all_five_tools() {
    let router = McpServer::new().router();
    let (status, body) = post_mcp(
        router,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let tools = body["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 5);

    let names: Vec<&str> = tools
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"find_nearby_stations"));
    assert!(names.contains(&"get_station_by_code"));
    assert!(names.contains(&"search_stations_by_name"));
    assert!(names.contains(&"get_area_statistics"));
    assert!(names.contains(&"plan_bike_journey"));
}

#[tokio::test]
async fn resources_list_returns_four_resources() {
    let router = McpServer::new().router();
    let (status, body) = post_mcp(
        router,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/list",
            "params": {}
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let resources = body["result"]["resources"].as_array().unwrap();
    assert_eq!(resources.len(), 4);
}

#[tokio::test]
async fn unknown_method_returns_error() {
    let router = McpServer::new().router();
    let (status, body) = post_mcp(
        router,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "nonexistent/method",
            "params": {}
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["error"].is_object(), "Expected error in response");
    assert!(body["result"].is_null());
}

#[tokio::test]
async fn unknown_tool_returns_error() {
    let router = McpServer::new().router();
    let (status, body) = post_mcp(
        router,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "nonexistent_tool",
                "arguments": {}
            }
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["error"].is_object(), "Expected error for unknown tool");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Unknown tool"),
        "Error message should mention unknown tool"
    );
}

#[tokio::test]
async fn malformed_json_returns_parse_error() {
    let router = McpServer::new().router();
    let request = Request::builder()
        .uri("/mcp")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(b"not valid json".to_vec()))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["error"]["code"], -32700);
}

#[tokio::test]
async fn tools_call_missing_name_returns_error() {
    let router = McpServer::new().router();
    let (status, body) = post_mcp(
        router,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "arguments": {}
            }
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["error"].is_object());
}

#[tokio::test]
async fn response_preserves_request_id() {
    let router = McpServer::new().router();
    let (_, body) = post_mcp(
        router,
        json!({
            "jsonrpc": "2.0",
            "id": "my-string-id",
            "method": "tools/list",
            "params": {}
        }),
    )
    .await;

    assert_eq!(body["id"], "my-string-id");
}

#[tokio::test]
async fn health_endpoint_returns_healthy() {
    let router = McpServer::new().router();
    let request = Request::builder()
        .uri("/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    // The /health route is on Server, not McpServer. We need the full server router.
    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = velib_mcp::Server::new(addr);
    let router = server.router();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["status"], "healthy");
    assert_eq!(body["service"], "velib-mcp");
}
