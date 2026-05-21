//! Integration tests for the MCP self-documentation endpoint (issue #9).
//!
//! Exercises both ways the documentation payload is exposed:
//! - JSON-RPC method `docs/describe`.
//! - MCP resource URI `velib://docs/api`.
//!
//! Includes drift guards so that:
//! - Every tool advertised by `tools/list` is documented.
//! - Every resource advertised by `resources/list` is documented.
//! - Cache TTLs in the documentation match the runtime constants.
//! - Both delivery paths return identical payloads.
//!
//! All paths are reachable without any live network I/O — the docs payload
//! is hand-curated and pure, and `tools/list` / `resources/list` are
//! served from static JSON.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use velib_mcp::data::{REALTIME_CACHE_TTL_MINUTES, REFERENCE_CACHE_TTL_MINUTES};
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

async fn fetch_docs_via_jsonrpc() -> Value {
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "docs/describe",
        "params": {}
    }))
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], 1);
    assert!(
        body["error"].is_null(),
        "docs/describe should not return an error: {:?}",
        body["error"]
    );
    body["result"].clone()
}

#[tokio::test]
async fn docs_describe_returns_required_top_level_keys() {
    let doc = fetch_docs_via_jsonrpc().await;
    for key in [
        "schema_version",
        "server",
        "data_freshness",
        "units",
        "enums",
        "types",
        "tools",
        "resources",
        "error_codes",
        "methods",
    ] {
        assert!(doc.get(key).is_some(), "missing top-level key: {key}");
    }

    // Server identity is meaningful.
    assert_eq!(doc["server"]["name"], "velib-mcp");
    assert!(doc["server"]["version"].as_str().unwrap().contains('.'));
}

#[tokio::test]
async fn docs_describe_omits_params_per_jsonrpc_2() {
    // JSON-RPC 2.0 permits omitting `params`; the server must accept it.
    let (status, body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "docs/describe"
    }))
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["result"]["server"]["name"].is_string());
}

#[tokio::test]
async fn docs_describe_documents_every_advertised_tool() {
    // Drift guard: anything in tools/list must appear in the documentation.
    let (_, list_body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    }))
    .await;
    let listed: Vec<String> = list_body["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();

    let doc = fetch_docs_via_jsonrpc().await;
    let documented: Vec<String> = doc["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();

    for name in &listed {
        assert!(
            documented.contains(name),
            "tool {name} advertised by tools/list but not documented"
        );
    }
    assert_eq!(listed.len(), documented.len());

    // Each documented tool must carry meaningful schema info.
    for tool in doc["tools"].as_array().unwrap() {
        assert!(tool["description"].as_str().unwrap().len() > 10);
        assert_eq!(tool["input_schema"]["type"], "object");
        assert!(tool["input_schema"]["properties"].is_object());
        assert!(tool["output_schema"].is_object());
    }
}

#[tokio::test]
async fn docs_describe_documents_every_advertised_resource() {
    let (_, list_body) = post_mcp(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "resources/list",
        "params": {}
    }))
    .await;
    let listed: Vec<String> = list_body["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["uri"].as_str().unwrap().to_string())
        .collect();

    let doc = fetch_docs_via_jsonrpc().await;
    let documented: Vec<String> = doc["resources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["uri"].as_str().unwrap().to_string())
        .collect();

    for uri in &listed {
        assert!(
            documented.contains(uri),
            "resource {uri} advertised by resources/list but not documented"
        );
    }
    assert!(documented.contains(&"velib://docs/api".to_string()));
}

#[tokio::test]
async fn docs_cache_ttls_match_runtime_constants() {
    let doc = fetch_docs_via_jsonrpc().await;
    let caches = doc["data_freshness"]["caches"].as_array().unwrap();
    let reference = caches
        .iter()
        .find(|c| c["name"] == "reference")
        .expect("reference cache documented");
    let realtime = caches
        .iter()
        .find(|c| c["name"] == "realtime")
        .expect("realtime cache documented");
    assert_eq!(
        reference["ttl_minutes"].as_i64().unwrap(),
        REFERENCE_CACHE_TTL_MINUTES
    );
    assert_eq!(
        realtime["ttl_minutes"].as_i64().unwrap(),
        REALTIME_CACHE_TTL_MINUTES
    );
}

#[tokio::test]
async fn docs_lists_all_required_enums_with_definitions() {
    let doc = fetch_docs_via_jsonrpc().await;
    let enums = &doc["enums"];

    let cases = [
        ("StationStatus", &["OPEN", "CLOSED", "MAINTENANCE"][..]),
        (
            "DataFreshness",
            &["Fresh", "Recent", "Stale", "VeryStale"][..],
        ),
        ("BikeTypeFilter", &["mechanical", "electric", "any"][..]),
        ("DataSource", &["paris_open_data", "cache", "fallback"][..]),
    ];

    for (enum_name, expected_values) in cases {
        let entry = enums
            .get(enum_name)
            .unwrap_or_else(|| panic!("enum {enum_name} not documented"));
        assert!(
            entry["description"].as_str().unwrap().len() > 5,
            "enum {enum_name} missing description"
        );
        let values = entry["values"].as_array().unwrap();
        let documented_values: Vec<&str> = values
            .iter()
            .map(|v| v["value"].as_str().unwrap())
            .collect();
        for v in expected_values {
            assert!(
                documented_values.contains(v),
                "enum {enum_name} missing value {v}"
            );
        }
        for v in values {
            assert!(
                v["definition"].as_str().unwrap().len() > 5,
                "enum {enum_name} value {} missing definition",
                v["value"]
            );
        }
    }
}

#[tokio::test]
async fn docs_documents_jsonrpc_error_codes() {
    let doc = fetch_docs_via_jsonrpc().await;
    let codes: Vec<i64> = doc["error_codes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["code"].as_i64().unwrap())
        .collect();
    for expected in [-32700, -32600, -32601, -32602, -32603, -32001] {
        assert!(
            codes.contains(&expected),
            "error code {expected} not documented"
        );
    }
}

#[tokio::test]
async fn docs_describe_every_ref_resolves() {
    // Drift guard: every `$ref` in the payload must resolve to a real node.
    // This catches dangling pointers like `#/types/Foo` when `types/Foo`
    // hasn't been defined (regression coverage for issue spotted on PR #142).
    let doc = fetch_docs_via_jsonrpc().await;
    let mut unresolved: Vec<String> = Vec::new();
    collect_unresolved_refs(&doc, &doc, &mut unresolved);
    assert!(
        unresolved.is_empty(),
        "unresolved $ref pointers in docs payload: {unresolved:?}"
    );
}

fn collect_unresolved_refs(node: &Value, root: &Value, out: &mut Vec<String>) {
    match node {
        Value::Object(map) => {
            if let Some(Value::String(pointer)) = map.get("$ref") {
                if resolve_pointer(root, pointer).is_none() {
                    out.push(pointer.clone());
                }
            }
            for v in map.values() {
                collect_unresolved_refs(v, root, out);
            }
        }
        Value::Array(items) => {
            for v in items {
                collect_unresolved_refs(v, root, out);
            }
        }
        _ => {}
    }
}

fn resolve_pointer<'a>(root: &'a Value, pointer: &str) -> Option<&'a Value> {
    let stripped = pointer.strip_prefix('#').unwrap_or(pointer);
    let stripped = stripped.strip_prefix('/').unwrap_or(stripped);
    if stripped.is_empty() {
        return Some(root);
    }
    let mut cur = root;
    for raw in stripped.split('/') {
        let token = raw.replace("~1", "/").replace("~0", "~");
        cur = match cur {
            Value::Object(m) => m.get(&token)?,
            Value::Array(a) => a.get(token.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(cur)
}

#[tokio::test]
async fn docs_resource_uri_returns_same_payload_as_jsonrpc_method() {
    let via_method = fetch_docs_via_jsonrpc().await;
    let (status, via_resource) = get_resource("velib://docs/api").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        via_method, via_resource,
        "docs/describe and velib://docs/api must return identical payloads"
    );
}

#[tokio::test]
async fn docs_describe_documents_units_for_numeric_values() {
    let doc = fetch_docs_via_jsonrpc().await;
    let units = doc["units"].as_object().unwrap();
    for required in [
        "latitude",
        "longitude",
        "distance",
        "duration",
        "ttl",
        "capacity",
        "bikes",
    ] {
        assert!(
            units.contains_key(required),
            "missing unit documentation for {required}"
        );
        assert!(units[required]["type"].is_string());
    }
}

#[tokio::test]
async fn docs_describe_documents_service_area() {
    let doc = fetch_docs_via_jsonrpc().await;
    let area = &doc["server"]["service_area"];
    assert_eq!(area["max_distance_meters"].as_f64().unwrap(), 50_000.0);
    assert!(area["reference_point"]["latitude"].is_number());
    assert!(area["reference_point"]["longitude"].is_number());
}
