//! Routing-level tests for `McpServer`.
//!
//! Exercises the HTTP dispatch logic in `src/mcp/server.rs` in-process via
//! `tower::ServiceExt::oneshot`. Every path here is reachable without any live
//! network I/O: either the method is served from static JSON
//! (`tools/list`, `resources/list`), the request is rejected before the
//! handler is invoked (unknown method, malformed JSON, missing params), or
//! the handler's validation layer rejects the input before any HTTP call.
//!
//! Resource endpoints that require the Velib Open Data API live in
//! `tests/mcp_resource_tests.rs` and are gated behind `#[ignore]`.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use velib_mcp::mcp::server::McpServer;

async fn post_mcp(body: Value) -> (StatusCode, Value) {
    let router = McpServer::new().router();
    let request = Request::builder()
        .uri("/mcp")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

async fn post_mcp_raw(raw: &str) -> (StatusCode, Value) {
    let router = McpServer::new().router();
    let request = Request::builder()
        .uri("/mcp")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(raw.to_owned()))
        .unwrap();
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

async fn get_resource(uri: &str) -> (StatusCode, Value) {
    let router = McpServer::new().router();
    let request = Request::builder()
        .uri(format!("/resources/{uri}"))
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

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
    let tools = body["result"]["tools"].as_array().expect("tools array");
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"find_nearby_stations"));
    assert!(names.contains(&"get_station_by_code"));
    assert!(names.contains(&"search_stations_by_name"));
    assert!(names.contains(&"get_area_statistics"));
    assert!(names.contains(&"plan_bike_journey"));
    assert_eq!(tools.len(), 5);
}

#[tokio::test]
async fn search_stations_by_name_schema_limit_matches_handler_enforcement() {
    // The advertised schema max must match the handler's `MAX_RESULT_LIMIT`
    // (100). A drift here silently shrinks the tool's advertised capability
    // and can be caught early by this regression test.
    let (_, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    }))
    .await;

    let tools = body["result"]["tools"].as_array().unwrap();
    let search = tools
        .iter()
        .find(|t| t["name"] == "search_stations_by_name")
        .expect("schema includes search_stations_by_name");
    assert_eq!(search["inputSchema"]["properties"]["limit"]["maximum"], 100);
}

#[tokio::test]
async fn tools_list_entries_have_required_schema_fields() {
    let (_, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": "abc",
        "method": "tools/list",
        "params": {}
    }))
    .await;

    assert_eq!(body["id"], "abc");
    for tool in body["result"]["tools"].as_array().unwrap() {
        assert!(tool["name"].is_string());
        assert!(tool["description"].is_string());
        assert_eq!(tool["inputSchema"]["type"], "object");
        assert!(tool["inputSchema"]["required"].is_array());
    }
}

#[tokio::test]
async fn resources_list_returns_four_resources() {
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "resources/list",
        "params": {}
    }))
    .await;

    assert_eq!(status, StatusCode::OK);
    let resources = body["result"]["resources"]
        .as_array()
        .expect("resources array");
    let uris: Vec<&str> = resources
        .iter()
        .map(|r| r["uri"].as_str().unwrap())
        .collect();
    assert!(uris.contains(&"velib://stations/reference"));
    assert!(uris.contains(&"velib://stations/realtime"));
    assert!(uris.contains(&"velib://stations/complete"));
    assert!(uris.contains(&"velib://health"));
    assert_eq!(resources.len(), 4);
    for resource in resources {
        assert_eq!(resource["mimeType"], "application/json");
        assert!(resource["name"].is_string());
        assert!(resource["description"].is_string());
    }
}

#[tokio::test]
async fn unknown_method_returns_jsonrpc_error() {
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 42,
        "method": "totally/unknown",
        "params": {}
    }))
    .await;

    assert_eq!(status, StatusCode::OK); // JSON-RPC errors travel in 200 responses
    assert_eq!(body["id"], 42);
    assert!(body["result"].is_null());
    let error = &body["error"];
    assert!(error.is_object());
    assert!(error["message"]
        .as_str()
        .unwrap()
        .contains("Unknown method"));
}

#[tokio::test]
async fn tools_call_unknown_tool_returns_jsonrpc_error() {
    // Unknown tools bubble up through the `result` variable (no `?`), so the
    // error is delivered inside the JSON-RPC envelope with HTTP 200.
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "not_a_real_tool",
            "arguments": {}
        }
    }))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["error"].is_object());
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Unknown tool"));
}

#[tokio::test]
async fn tools_call_missing_tool_name_returns_500() {
    // The `?` after `ok_or_else(... "Missing tool name")` early-returns from
    // `process_jsonrpc_request`, so the outer handler maps the error to a
    // 500 with a plain `{"error": ...}` body (no JSON-RPC envelope).
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "arguments": {}
        }
    }))
    .await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(body["error"]
        .as_str()
        .unwrap()
        .to_lowercase()
        .contains("missing tool name"));
}

#[tokio::test]
async fn tools_call_non_object_params_returns_500() {
    // Same early-return path as the missing-tool-name case: `params.as_object()`
    // returns None and `?` propagates "Invalid params" out as a 500.
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": "not an object"
    }))
    .await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(body["error"]
        .as_str()
        .unwrap()
        .to_lowercase()
        .contains("invalid params"));
}

#[tokio::test]
async fn tools_call_find_nearby_stations_dispatches_and_validates() {
    // Radius over 5000m triggers validation failure before any network I/O.
    // The handler returns Err which is wrapped into a JSON-RPC error response
    // (HTTP 200) by `process_jsonrpc_request`.
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "find_nearby_stations",
            "arguments": {
                "latitude": 48.8566,
                "longitude": 2.3522,
                "radius_meters": 99999,
                "limit": 10
            }
        }
    }))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["error"].is_object(), "expected JSON-RPC error object");
    let msg = body["error"]["message"].as_str().unwrap();
    assert!(msg.contains("radius") || msg.contains("99999"), "{msg}");
}

#[tokio::test]
async fn tools_call_search_stations_by_name_dispatches_and_validates() {
    // Query under the 2-character minimum fails validation pre-network,
    // exercising the `search_stations_by_name` dispatch arm.
    // The error is wrapped into a JSON-RPC error response (HTTP 200).
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "search_stations_by_name",
            "arguments": {
                "query": "a",
                "limit": 10,
                "fuzzy": true
            }
        }
    }))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["error"].is_object(), "expected JSON-RPC error object");
}

#[tokio::test]
async fn tools_call_plan_bike_journey_dispatches_and_validates() {
    // (0, 0) origin fails Paris-bounds validation pre-network, exercising the
    // `plan_bike_journey` dispatch arm.
    // The error is wrapped into a JSON-RPC error response (HTTP 200).
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "plan_bike_journey",
            "arguments": {
                "origin": {"latitude": 0.0, "longitude": 0.0},
                "destination": {"latitude": 48.8566, "longitude": 2.3522}
            }
        }
    }))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["error"].is_object(), "expected JSON-RPC error object");
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .to_lowercase()
        .contains("invalid coordinates"));
}

#[tokio::test]
async fn tools_call_with_malformed_arguments_returns_jsonrpc_error() {
    // Arguments missing required `latitude` field make
    // `serde_json::from_value` fail inside `tool_text_content`; the error is
    // wrapped into a JSON-RPC error response (HTTP 200).
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "find_nearby_stations",
            "arguments": {
                "longitude": 2.3522
            }
        }
    }))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["error"].is_object(), "expected JSON-RPC error object");
}

#[tokio::test]
async fn malformed_json_body_returns_parse_error() {
    let (status, body) = post_mcp_raw("{ not valid json").await;

    // The JsonRejection branch still returns 200 with a JSON-RPC error body.
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["error"]["code"], -32700);
    assert!(body["id"].is_null());
}

#[tokio::test]
async fn unknown_resource_uri_returns_404() {
    let (status, body) = get_resource("velib://unknown/resource").await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "Resource not found");
}

#[tokio::test]
async fn tools_call_without_arguments_uses_default_empty_object() {
    // When `arguments` is absent, the server falls back to `json!({})` as the
    // default. For `search_stations_by_name`, that deserializes with a missing
    // required `query` field, so serde fails and the error is wrapped into a
    // JSON-RPC error response (HTTP 200).
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "search_stations_by_name"
        }
    }))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["error"].is_object(), "expected JSON-RPC error object");
}

#[tokio::test]
async fn request_with_string_id_preserves_id_in_response() {
    let (_, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": "client-generated-id",
        "method": "tools/list",
        "params": {}
    }))
    .await;

    assert_eq!(body["id"], "client-generated-id");
}

#[tokio::test]
async fn tools_list_accepts_request_with_omitted_params() {
    // JSON-RPC 2.0 allows `params` to be omitted; the server must not reject
    // parameterless calls like `tools/list`. Regression guard for the
    // `#[serde(default)]` on `JsonRpcRequest::params`.
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 99,
        "method": "tools/list"
    }))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], 99);
    assert!(body["result"]["tools"].is_array());
}

#[tokio::test]
async fn request_with_null_id_preserves_null_id_in_response() {
    let (_, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": null,
        "method": "resources/list",
        "params": {}
    }))
    .await;

    assert!(body["id"].is_null());
    assert!(body["result"]["resources"].is_array());
}
