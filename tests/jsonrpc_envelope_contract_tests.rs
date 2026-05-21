//! End-to-end JSON-RPC envelope contract tests for `tools/call`.
//!
//! These tests pin down the **HTTP-level** shape of error responses produced
//! by `tools/call`. The unit tests in `src/error.rs` and `tests/error_tests.rs`
//! verify the `Error` -> `mcp_error_code()` / `error_type()` mapping in
//! isolation, and `tests/mcp_types_tests.rs` covers the
//! `From<Error> for JsonRpcError` conversion. What was missing: a wire-level
//! contract that an `Error` raised inside a tool handler reaches the HTTP
//! client with the documented `code` (per JSON-RPC 2.0 + MCP) and
//! `data.error_type` string.
//!
//! All cases here trigger handler validation that fires *before* any network
//! call to the Velib Open Data API, so the suite is hermetic.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use velib_mcp::mcp::server::McpServer;

/// Issue a single `POST /mcp` and return `(status, parsed body)`.
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

/// Build a `tools/call` request body for the given tool with the given id and
/// arguments.
fn tools_call(id: Value, tool: &str, arguments: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": { "name": tool, "arguments": arguments }
    })
}

/// Assert the body is a well-formed JSON-RPC error envelope and return
/// `(code, error_type)` for further per-case checks.
fn assert_jsonrpc_error_envelope(body: &Value, expected_id: &Value) -> (i64, String) {
    assert_eq!(
        body["jsonrpc"], "2.0",
        "every JSON-RPC response must echo jsonrpc=2.0; got {body}"
    );
    assert_eq!(
        &body["id"], expected_id,
        "id must round-trip verbatim; got {body}"
    );
    assert!(
        body["result"].is_null(),
        "result must be absent on error envelope; got {body}"
    );
    let error = &body["error"];
    assert!(error.is_object(), "error must be a JSON object; got {body}");
    let code = error["code"]
        .as_i64()
        .unwrap_or_else(|| panic!("error.code must be an integer; got {body}"));
    let message = error["message"].as_str().unwrap_or("");
    assert!(!message.is_empty(), "error.message must be non-empty");
    let error_type = error["data"]["error_type"]
        .as_str()
        .unwrap_or_else(|| panic!("error.data.error_type must be a string; got {body}"))
        .to_string();
    (code, error_type)
}

// --- find_nearby_stations: each validation branch reaches HTTP with right code ---

#[tokio::test]
async fn find_nearby_radius_too_large_surfaces_invalid_params_envelope() {
    // Handler emits Error::SearchRadiusTooLarge -> -32602 / search_radius_too_large.
    let id = json!(1);
    let (status, body) = post_mcp(tools_call(
        id.clone(),
        "find_nearby_stations",
        json!({
            "latitude": 48.8566,
            "longitude": 2.3522,
            "radius_meters": 99_999,
            "limit": 10
        }),
    ))
    .await;
    assert_eq!(status, StatusCode::OK);
    let (code, error_type) = assert_jsonrpc_error_envelope(&body, &id);
    assert_eq!(code, -32602);
    assert_eq!(error_type, "search_radius_too_large");
}

#[tokio::test]
async fn find_nearby_limit_exceeded_surfaces_invalid_params_envelope() {
    // Handler emits Error::ResultLimitExceeded -> -32602 / result_limit_exceeded.
    let id = json!(2);
    let (status, body) = post_mcp(tools_call(
        id.clone(),
        "find_nearby_stations",
        json!({
            "latitude": 48.8566,
            "longitude": 2.3522,
            "radius_meters": 500,
            "limit": 200
        }),
    ))
    .await;
    assert_eq!(status, StatusCode::OK);
    let (code, error_type) = assert_jsonrpc_error_envelope(&body, &id);
    assert_eq!(code, -32602);
    assert_eq!(error_type, "result_limit_exceeded");
}

#[tokio::test]
async fn find_nearby_invalid_coordinates_surfaces_invalid_params_envelope() {
    // (0, 0) is outside the broad Paris bounding box -> InvalidCoordinates.
    let id = json!("nyc");
    let (status, body) = post_mcp(tools_call(
        id.clone(),
        "find_nearby_stations",
        json!({
            "latitude": 0.0,
            "longitude": 0.0,
            "radius_meters": 500,
            "limit": 10
        }),
    ))
    .await;
    assert_eq!(status, StatusCode::OK);
    let (code, error_type) = assert_jsonrpc_error_envelope(&body, &id);
    assert_eq!(code, -32602);
    assert_eq!(error_type, "invalid_coordinates");
}

#[tokio::test]
async fn find_nearby_outside_service_area_surfaces_invalid_params_envelope() {
    // ~100km north of Paris City Hall: inside the bounding box but >50km from
    // city hall, hitting the OutsideServiceArea branch.
    let id = json!(null);
    let (status, body) = post_mcp(tools_call(
        id.clone(),
        "find_nearby_stations",
        json!({
            "latitude": 49.75,
            "longitude": 2.3522,
            "radius_meters": 500,
            "limit": 10
        }),
    ))
    .await;
    assert_eq!(status, StatusCode::OK);
    let (code, error_type) = assert_jsonrpc_error_envelope(&body, &id);
    assert_eq!(code, -32602);
    assert_eq!(error_type, "outside_service_area");
}

// --- search_stations_by_name validation routes ---

#[tokio::test]
async fn search_short_query_surfaces_internal_error_envelope() {
    // Handler emits Error::Internal("Search query too short") -> -32603 /
    // internal_error. (The handler's choice of `Internal` here is itself a
    // smell that this test pins down -- if we ever upgrade it to a typed
    // Validation error, both this assertion and the user-facing error code
    // need to change in lockstep.)
    let id = json!(3);
    let (status, body) = post_mcp(tools_call(
        id.clone(),
        "search_stations_by_name",
        json!({ "query": "a", "limit": 10, "fuzzy": true }),
    ))
    .await;
    assert_eq!(status, StatusCode::OK);
    let (code, error_type) = assert_jsonrpc_error_envelope(&body, &id);
    assert_eq!(code, -32603);
    assert_eq!(error_type, "internal_error");
}

#[tokio::test]
async fn search_limit_exceeded_surfaces_invalid_params_envelope() {
    let id = json!(4);
    let (status, body) = post_mcp(tools_call(
        id.clone(),
        "search_stations_by_name",
        json!({ "query": "chatelet", "limit": 200, "fuzzy": false }),
    ))
    .await;
    assert_eq!(status, StatusCode::OK);
    let (code, error_type) = assert_jsonrpc_error_envelope(&body, &id);
    assert_eq!(code, -32602);
    assert_eq!(error_type, "result_limit_exceeded");
}

// --- plan_bike_journey validation routes ---

#[tokio::test]
async fn plan_journey_invalid_origin_surfaces_invalid_params_envelope() {
    let id = json!(5);
    let (status, body) = post_mcp(tools_call(
        id.clone(),
        "plan_bike_journey",
        json!({
            "origin": { "latitude": 0.0, "longitude": 0.0 },
            "destination": { "latitude": 48.8566, "longitude": 2.3522 }
        }),
    ))
    .await;
    assert_eq!(status, StatusCode::OK);
    let (code, error_type) = assert_jsonrpc_error_envelope(&body, &id);
    assert_eq!(code, -32602);
    assert_eq!(error_type, "invalid_coordinates");
}

#[tokio::test]
async fn plan_journey_invalid_destination_surfaces_invalid_params_envelope() {
    let id = json!(6);
    let (status, body) = post_mcp(tools_call(
        id.clone(),
        "plan_bike_journey",
        json!({
            "origin": { "latitude": 48.8566, "longitude": 2.3522 },
            "destination": { "latitude": 0.0, "longitude": 0.0 }
        }),
    ))
    .await;
    assert_eq!(status, StatusCode::OK);
    let (code, error_type) = assert_jsonrpc_error_envelope(&body, &id);
    assert_eq!(code, -32602);
    assert_eq!(error_type, "invalid_coordinates");
}

#[tokio::test]
async fn plan_journey_destination_outside_service_area_surfaces_envelope() {
    // Origin valid; destination ~100km north of Paris City Hall.
    let id = json!(7);
    let (status, body) = post_mcp(tools_call(
        id.clone(),
        "plan_bike_journey",
        json!({
            "origin": { "latitude": 48.8566, "longitude": 2.3522 },
            "destination": { "latitude": 49.75, "longitude": 2.3522 }
        }),
    ))
    .await;
    assert_eq!(status, StatusCode::OK);
    let (code, error_type) = assert_jsonrpc_error_envelope(&body, &id);
    assert_eq!(code, -32602);
    assert_eq!(error_type, "outside_service_area");
}

// --- Malformed serde input surfaces as a Json error ---

#[tokio::test]
async fn malformed_arguments_for_find_nearby_surface_json_error_envelope() {
    // Missing required `latitude` field -> serde_json::Error -> Error::Json
    // -> -32700 / json_error.
    let id = json!(8);
    let (status, body) = post_mcp(tools_call(
        id.clone(),
        "find_nearby_stations",
        json!({ "longitude": 2.3522 }),
    ))
    .await;
    assert_eq!(status, StatusCode::OK);
    let (code, error_type) = assert_jsonrpc_error_envelope(&body, &id);
    assert_eq!(code, -32700);
    assert_eq!(error_type, "json_error");
}

// --- Unknown tool dispatch ---

#[tokio::test]
async fn unknown_tool_call_surfaces_mcp_protocol_error_envelope() {
    let id = json!(9);
    let (status, body) = post_mcp(tools_call(id.clone(), "totally_unknown", json!({}))).await;
    assert_eq!(status, StatusCode::OK);
    let (code, error_type) = assert_jsonrpc_error_envelope(&body, &id);
    // McpProtocol -> -32603 / mcp_protocol_error.
    assert_eq!(code, -32603);
    assert_eq!(error_type, "mcp_protocol_error");
}

// --- Id round-trip across types: number, string, null ---

#[tokio::test]
async fn error_envelope_preserves_string_id() {
    let id = json!("client-abc-123");
    let (_, body) = post_mcp(tools_call(
        id.clone(),
        "find_nearby_stations",
        json!({ "latitude": 0.0, "longitude": 0.0 }),
    ))
    .await;
    assert_jsonrpc_error_envelope(&body, &id);
}

#[tokio::test]
async fn error_envelope_preserves_null_id() {
    let id = json!(null);
    let (_, body) = post_mcp(tools_call(
        id.clone(),
        "search_stations_by_name",
        json!({ "query": "x", "limit": 200 }),
    ))
    .await;
    assert_jsonrpc_error_envelope(&body, &id);
}
