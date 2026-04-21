//! End-to-end and drift-guard tests for the `describe_api` self-documentation
//! endpoint.
//!
//! These tests cover the five reviewer-required guarantees from issue #9:
//!
//! 1. The describe payload is exposed both as an MCP tool (`describe_api`)
//!    and as an MCP resource (`velib://api/description`).
//! 2. Cache TTLs come from the imported constants in `data::client` and are
//!    never duplicated inside the describe module (asserted indirectly via
//!    the constant-import test in `src/mcp/describe.rs` plus the value check
//!    below).
//! 3. Enum-variant drift guard: every value listed in
//!    `api_description().enums[*].values[*]` round-trips through the
//!    corresponding Rust enum's `Deserialize` impl.
//! 4. Schema-parity guard: for every tool returned by `tools/list`, the
//!    describe payload's `tools[*].input_schema` equals the `inputSchema`
//!    served by `tools/list` (byte-for-byte `serde_json::Value` equality).
//! 5. Markdown output: non-empty, starts with `# `, and names every tool.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use velib_mcp::data::client::{REALTIME_CACHE_TTL_MINUTES, REFERENCE_CACHE_TTL_MINUTES};
use velib_mcp::mcp::describe::{api_description, api_description_markdown};
use velib_mcp::mcp::server::McpServer;
use velib_mcp::types::{BikeTypeFilter, DataFreshness, StationStatus};

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

#[test]
fn description_has_expected_top_level_shape() {
    let desc = api_description();
    for key in [
        "server",
        "service_area",
        "units",
        "caching",
        "enums",
        "tools",
        "resources",
        "error_codes",
    ] {
        assert!(
            desc.get(key).is_some(),
            "describe_api payload missing top-level key `{key}`"
        );
    }

    // Cache TTLs must match the imported constants converted from minutes to
    // seconds (reviewer revision 2: no duplicate values).
    assert_eq!(
        desc["caching"]["reference_ttl_seconds"].as_i64(),
        Some(REFERENCE_CACHE_TTL_MINUTES * 60),
        "reference TTL must be sourced from data::client::REFERENCE_CACHE_TTL_MINUTES"
    );
    assert_eq!(
        desc["caching"]["realtime_ttl_seconds"].as_i64(),
        Some(REALTIME_CACHE_TTL_MINUTES * 60),
        "realtime TTL must be sourced from data::client::REALTIME_CACHE_TTL_MINUTES"
    );
}

#[tokio::test]
async fn describe_api_tool_is_registered_and_callable() {
    // Tool must appear in tools/list with the documented input schema.
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    }))
    .await;
    assert_eq!(status, StatusCode::OK);
    let tools = body["result"]["tools"].as_array().unwrap();
    let describe = tools
        .iter()
        .find(|t| t["name"] == "describe_api")
        .expect("describe_api tool missing from tools/list");
    assert_eq!(describe["inputSchema"]["type"], "object");
    assert_eq!(
        describe["inputSchema"]["properties"]["format"]["enum"],
        json!(["json", "markdown"])
    );
    assert_eq!(
        describe["inputSchema"]["additionalProperties"],
        json!(false)
    );

    // Calling the tool returns the documented content envelope with JSON text.
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {"name": "describe_api", "arguments": {"format": "json"}}
    }))
    .await;
    assert_eq!(status, StatusCode::OK);
    let text = body["result"]["content"][0]["text"]
        .as_str()
        .expect("describe_api tool response missing text content");
    let parsed: Value = serde_json::from_str(text).expect("describe_api returned invalid JSON");
    assert!(parsed["tools"].is_array());
}

#[tokio::test]
async fn describe_api_resource_is_registered_and_readable() {
    // Reviewer revision 1: the description must be exposed as a resource too.
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "resources/list",
        "params": {}
    }))
    .await;
    assert_eq!(status, StatusCode::OK);
    let resources = body["result"]["resources"].as_array().unwrap();
    let api_resource = resources
        .iter()
        .find(|r| r["uri"] == "velib://api/description")
        .expect("velib://api/description not listed in resources/list");
    assert_eq!(api_resource["mimeType"], "application/json");
    assert!(api_resource["name"].is_string());
    assert!(api_resource["description"].is_string());

    // resources/read returns the JSON description as text content.
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "resources/read",
        "params": {"uri": "velib://api/description"}
    }))
    .await;
    assert_eq!(status, StatusCode::OK);
    let contents = body["result"]["contents"].as_array().unwrap();
    assert_eq!(contents[0]["uri"], "velib://api/description");
    assert_eq!(contents[0]["mimeType"], "application/json");
    let text = contents[0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed["server"]["name"].is_string());
}

#[tokio::test]
async fn describe_api_markdown_format_is_well_formed() {
    // Reviewer revision 5: markdown output is non-empty, starts with `# `,
    // and contains every tool name at least once.
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {"name": "describe_api", "arguments": {"format": "markdown"}}
    }))
    .await;
    assert_eq!(status, StatusCode::OK);
    let text = body["result"]["content"][0]["text"].as_str().unwrap();

    assert!(!text.is_empty(), "markdown output must be non-empty");
    assert!(text.starts_with("# "), "markdown must start with `# `");
    for tool in [
        "find_nearby_stations",
        "get_station_by_code",
        "search_stations_by_name",
        "get_area_statistics",
        "plan_bike_journey",
        "describe_api",
    ] {
        assert!(
            text.contains(tool),
            "markdown output missing tool name `{tool}`"
        );
    }

    // Also test the helper directly so the failure surface is clear when the
    // renderer regresses independently of the JSON-RPC plumbing.
    let direct = api_description_markdown();
    assert!(direct.starts_with("# "));
    assert!(direct.contains("describe_api"));
}

#[tokio::test]
async fn schema_parity_between_tools_list_and_describe_payload() {
    // Reviewer revision 4: every tool's inputSchema in tools/list must equal
    // the tools[*].input_schema served by describe_api.
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    }))
    .await;
    assert_eq!(status, StatusCode::OK);

    let mut tools_list_schemas: std::collections::HashMap<String, Value> =
        std::collections::HashMap::new();
    for tool in body["result"]["tools"].as_array().unwrap() {
        let name = tool["name"].as_str().unwrap().to_string();
        tools_list_schemas.insert(name, tool["inputSchema"].clone());
    }

    let desc = api_description();
    let describe_tools = desc["tools"].as_array().unwrap();
    assert_eq!(
        describe_tools.len(),
        tools_list_schemas.len(),
        "describe_api tool count must match tools/list"
    );

    for tool in describe_tools {
        let name = tool["name"].as_str().unwrap();
        let expected = tools_list_schemas
            .get(name)
            .unwrap_or_else(|| panic!("tool `{name}` in describe payload not in tools/list"));
        assert_eq!(
            &tool["input_schema"], expected,
            "input_schema drift for tool `{name}`"
        );
    }
}

#[test]
fn enum_variant_drift_guard_matches_rust_types() {
    // Reviewer revision 3: every value listed in the describe payload's enums
    // must deserialize into the corresponding Rust enum. Missing enum types
    // skip gracefully.
    let desc = api_description();
    let enums = desc["enums"].as_array().expect("enums array");

    for enum_block in enums {
        let name = enum_block["name"].as_str().expect("enum name");
        let values = enum_block["values"].as_array().expect("enum values");

        match name {
            "StationStatus" => {
                for v in values {
                    let raw = v["value"].as_str().unwrap();
                    let json_value = Value::String(raw.to_string());
                    let parsed: std::result::Result<StationStatus, _> =
                        serde_json::from_value(json_value);
                    assert!(
                        parsed.is_ok(),
                        "StationStatus documented value `{raw}` does not round-trip: {parsed:?}"
                    );
                }
            }
            "DataFreshness" => {
                for v in values {
                    let raw = v["value"].as_str().unwrap();
                    let json_value = Value::String(raw.to_string());
                    let parsed: std::result::Result<DataFreshness, _> =
                        serde_json::from_value(json_value);
                    assert!(
                        parsed.is_ok(),
                        "DataFreshness documented value `{raw}` does not round-trip: {parsed:?}"
                    );
                }
            }
            "BikeTypeFilter" => {
                for v in values {
                    let raw = v["value"].as_str().unwrap();
                    let json_value = Value::String(raw.to_string());
                    let parsed: std::result::Result<BikeTypeFilter, _> =
                        serde_json::from_value(json_value);
                    assert!(
                        parsed.is_ok(),
                        "BikeTypeFilter documented value `{raw}` does not round-trip: {parsed:?}"
                    );
                }
            }
            // Skip gracefully for any enum we don't know about (plan: "if the
            // variants don't exist, skip gracefully").
            _ => {}
        }
    }
}

#[tokio::test]
async fn describe_api_rejects_unknown_format() {
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {"name": "describe_api", "arguments": {"format": "yaml"}}
    }))
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body["error"].is_object(),
        "unsupported format must produce a JSON-RPC error"
    );
}
