//! Tests for McpServer JSON-RPC routing via the axum router.
//!
//! These tests send requests directly to the router (no real server process,
//! no network calls to the Velib API) and verify that the JSON-RPC dispatch
//! layer behaves correctly for well-formed and ill-formed inputs.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use velib_mcp::mcp::server::McpServer;

/// Helper: send a POST /mcp with a JSON body and return the parsed response.
async fn post_mcp(body: Value) -> (StatusCode, Value) {
    let router = McpServer::new().router();
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

// ---------------------------------------------------------------------------
// tools/list
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tools_list_returns_all_five_tools() {
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    }))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], 1);
    assert!(body["error"].is_null(), "tools/list should not return an error");

    let tools = body["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 5, "Expected exactly 5 tools");

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
async fn tools_list_each_tool_has_name_description_and_input_schema() {
    let (_, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    }))
    .await;

    let tools = body["result"]["tools"].as_array().unwrap();
    for tool in tools {
        let name = tool["name"].as_str().unwrap_or("");
        assert!(!name.is_empty(), "Tool must have a name");
        assert!(
            tool["description"].is_string(),
            "Tool {name} must have a description"
        );
        assert!(
            tool["inputSchema"].is_object(),
            "Tool {name} must have an inputSchema"
        );
    }
}

// ---------------------------------------------------------------------------
// resources/list
// ---------------------------------------------------------------------------

#[tokio::test]
async fn resources_list_returns_expected_uris() {
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "resources/list",
        "params": {}
    }))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["error"].is_null());

    let resources = body["result"]["resources"].as_array().unwrap();
    let uris: Vec<&str> = resources
        .iter()
        .map(|r| r["uri"].as_str().unwrap())
        .collect();

    assert!(uris.contains(&"velib://stations/reference"));
    assert!(uris.contains(&"velib://stations/realtime"));
    assert!(uris.contains(&"velib://stations/complete"));
    assert!(uris.contains(&"velib://health"));
}

// ---------------------------------------------------------------------------
// Unknown method
// ---------------------------------------------------------------------------

#[tokio::test]
async fn unknown_method_returns_mcp_protocol_error() {
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "nonexistent/method",
        "params": {}
    }))
    .await;

    assert_eq!(status, StatusCode::OK); // JSON-RPC errors are HTTP 200
    assert!(body["result"].is_null(), "Should have no result");
    assert!(body["error"].is_object(), "Should have an error object");
    // Unknown method is a protocol-level error; code should be non-zero
    let code = body["error"]["code"].as_i64().unwrap();
    assert_ne!(code, 0);
}

// ---------------------------------------------------------------------------
// tools/call — unknown tool
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tools_call_unknown_tool_returns_error() {
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "does_not_exist",
            "arguments": {}
        }
    }))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["result"].is_null());
    assert!(body["error"].is_object());
    let message = body["error"]["message"].as_str().unwrap_or("");
    assert!(
        message.contains("does_not_exist"),
        "Error message should name the unknown tool"
    );
}

// ---------------------------------------------------------------------------
// tools/call — missing params object
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tools_call_missing_params_returns_error() {
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 6,
        "method": "tools/call",
        "params": "not_an_object"
    }))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["error"].is_object(), "Should return an error for non-object params");
}

// ---------------------------------------------------------------------------
// Malformed JSON body
// ---------------------------------------------------------------------------

#[tokio::test]
async fn malformed_json_returns_parse_error() {
    let router = McpServer::new().router();
    let request = Request::builder()
        .uri("/mcp")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(b"{not valid json".as_ref()))
        .unwrap();
    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK); // parse errors are still 200
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(body["error"].is_object());
    let code = body["error"]["code"].as_i64().unwrap();
    assert_eq!(code, -32700, "Parse error should use code -32700");
}

// ---------------------------------------------------------------------------
// tools/call — find_nearby_stations validation (no network required)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tools_call_find_nearby_excessive_radius_returns_error() {
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "tools/call",
        "params": {
            "name": "find_nearby_stations",
            "arguments": {
                "latitude": 48.8566,
                "longitude": 2.3522,
                "radius_meters": 99999
            }
        }
    }))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["result"].is_null());
    assert!(body["error"].is_object());
    let data = &body["error"]["data"];
    assert_eq!(data["error_type"], "search_radius_too_large");
}

#[tokio::test]
async fn tools_call_find_nearby_nyc_coordinates_returns_invalid_coords_error() {
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 8,
        "method": "tools/call",
        "params": {
            "name": "find_nearby_stations",
            "arguments": {
                "latitude": 40.7128,
                "longitude": -74.0060
            }
        }
    }))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["error"].is_object());
    let data = &body["error"]["data"];
    assert_eq!(data["error_type"], "invalid_coordinates");
}

// ---------------------------------------------------------------------------
// tools/call — search_stations_by_name short query (no network required)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tools_call_search_short_query_returns_validation_error() {
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 9,
        "method": "tools/call",
        "params": {
            "name": "search_stations_by_name",
            "arguments": {
                "query": "x"
            }
        }
    }))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["error"].is_object());
    let data = &body["error"]["data"];
    // Must be validation_error (-32602), not internal_error (-32603)
    assert_eq!(data["error_type"], "validation_error");
    let code = body["error"]["code"].as_i64().unwrap();
    assert_eq!(code, -32602);
}

// ---------------------------------------------------------------------------
// id echo: response id matches request id
// ---------------------------------------------------------------------------

#[tokio::test]
async fn response_id_matches_request_id_for_string_id() {
    let (_, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": "my-request-id",
        "method": "tools/list",
        "params": {}
    }))
    .await;

    assert_eq!(body["id"], "my-request-id");
}

#[tokio::test]
async fn response_id_matches_request_id_for_numeric_id() {
    let (_, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 42,
        "method": "tools/list",
        "params": {}
    }))
    .await;

    assert_eq!(body["id"], 42);
}
