//! Server-level wiring tests not covered by `mcp_server_routing_tests.rs`.
//!
//! The MCP-specific dispatch cases (tools/list, resources/list, unknown
//! method, malformed JSON, request-id preservation, etc.) are exercised
//! more strictly in `tests/mcp_server_routing_tests.rs`. The only piece
//! that file does not cover is the top-level `Server::router()` -- in
//! particular the `/health` route, which lives on `Server` rather than
//! `McpServer`.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn health_endpoint_returns_healthy() {
    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = velib_mcp::Server::new(addr);
    let router = server.router();

    let request = Request::builder()
        .uri("/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["status"], "healthy");
    assert_eq!(body["service"], "velib-mcp");
}
