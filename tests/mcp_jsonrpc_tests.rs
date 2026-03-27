//! Tests for MCP server JSON-RPC request processing without spawning a
//! separate process. Uses tower::ServiceExt::oneshot for in-process testing.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;
use velib_mcp::mcp::server::McpServer;

fn mcp_router() -> axum::Router {
    McpServer::new().router()
}

async fn post_mcp(body: Value) -> (StatusCode, Value) {
    let router = mcp_router();
    let request = Request::builder()
        .uri("/mcp")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body_bytes).unwrap();
    (status, json)
}

#[tokio::test]
async fn tools_list_returns_all_five_tools() {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });
    let (status, json) = post_mcp(body).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["jsonrpc"], "2.0");
    assert_eq!(json["id"], 1);

    let tools = json["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 5, "Expected 5 MCP tools");

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
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "resources/list",
        "params": {}
    });
    let (status, json) = post_mcp(body).await;

    assert_eq!(status, StatusCode::OK);
    let resources = json["result"]["resources"].as_array().unwrap();
    assert_eq!(resources.len(), 4, "Expected 4 MCP resources");
}

#[tokio::test]
async fn unknown_method_returns_error() {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "nonexistent/method",
        "params": {}
    });
    let (status, json) = post_mcp(body).await;

    assert_eq!(status, StatusCode::OK);
    assert!(json["error"].is_object(), "Should return a JSON-RPC error");
    assert_eq!(json["error"]["code"], -32603);
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Unknown method"),
        "Error message should mention unknown method"
    );
}

#[tokio::test]
async fn unknown_tool_returns_error() {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "nonexistent_tool",
            "arguments": {}
        }
    });
    let (status, json) = post_mcp(body).await;

    assert_eq!(status, StatusCode::OK);
    assert!(json["error"].is_object());
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Unknown tool"),
        "Error message should mention unknown tool"
    );
}

#[tokio::test]
async fn tools_call_without_name_returns_error() {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "arguments": {}
        }
    });
    let (status, json) = post_mcp(body).await;

    assert_eq!(status, StatusCode::OK);
    assert!(json["error"].is_object(), "Missing tool name should produce an error");
}

#[tokio::test]
async fn malformed_json_returns_parse_error() {
    let router = mcp_router();
    let request = Request::builder()
        .uri("/mcp")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(b"this is not json".to_vec()))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(json["error"].is_object());
    assert_eq!(json["error"]["code"], -32700, "Should be JSON-RPC parse error code");
}

#[tokio::test]
async fn response_id_echoes_request_id() {
    // String ID
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "my-request-id",
        "method": "tools/list",
        "params": {}
    });
    let (_, json) = post_mcp(body).await;
    assert_eq!(json["id"], "my-request-id");

    // Numeric ID
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 999,
        "method": "tools/list",
        "params": {}
    });
    let (_, json) = post_mcp(body).await;
    assert_eq!(json["id"], 999);
}

#[tokio::test]
async fn tools_have_required_schema_fields() {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });
    let (_, json) = post_mcp(body).await;

    let tools = json["result"]["tools"].as_array().unwrap();
    for tool in tools {
        assert!(
            tool["name"].is_string(),
            "Tool missing 'name': {:?}",
            tool
        );
        assert!(
            tool["description"].is_string(),
            "Tool missing 'description': {:?}",
            tool
        );
        assert!(
            tool["inputSchema"].is_object(),
            "Tool missing 'inputSchema': {:?}",
            tool
        );
        assert_eq!(
            tool["inputSchema"]["type"], "object",
            "inputSchema type should be 'object'"
        );
    }
}
