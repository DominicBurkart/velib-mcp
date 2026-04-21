//! Self-documentation for the Velib MCP API.
//!
//! This module is the **single source of truth** for the machine-readable
//! description of the server's public contract that LLM clients discover at
//! runtime: server info, service area, units, cache TTLs, enum definitions,
//! per-tool input/output schemas, and error codes.
//!
//! The payload is assembled from:
//! - Cache TTL constants imported from [`crate::data::client`] so the document
//!   cannot drift from the actual cache configuration.
//! - Per-tool `inputSchema` values taken from
//!   [`crate::mcp::server::tool_definitions`] so the schemas reported here are
//!   byte-for-byte the same ones returned by the `tools/list` JSON-RPC method.
//! - Static metadata (enum variants, units, service-area bounds, error codes)
//!   hard-coded here; any change to the corresponding Rust types must be
//!   reflected here and is exercised by the drift-guard tests in
//!   `tests/mcp_describe_tests.rs`.
//!
//! Both the `describe_api` tool (`tools/call`) and the
//! `velib://api/description` resource (`resources/read`) render from the exact
//! same JSON produced by [`api_description`].

use serde_json::{json, Value};

use crate::data::client::{REALTIME_CACHE_TTL_MINUTES, REFERENCE_CACHE_TTL_MINUTES};
use crate::mcp::server::tool_definitions;
use crate::types::{PARIS_CITY_HALL, PARIS_SERVICE_AREA_MAX_METERS};

const SERVER_NAME: &str = "velib-mcp";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const MCP_PROTOCOL: &str = "2024-11-05";

/// Build the full machine-readable API description.
///
/// Pure function: no I/O, no allocations that depend on runtime state beyond
/// formatting the returned JSON value. Safe to call from any context.
#[must_use]
pub fn api_description() -> Value {
    json!({
        "server": {
            "name": SERVER_NAME,
            "version": SERVER_VERSION,
            "protocol": MCP_PROTOCOL,
            "endpoints": {
                "http_jsonrpc": "POST /mcp",
                "websocket_jsonrpc": "GET /mcp/ws (upgrade)",
                "resource_http": "GET /resources/<uri>"
            }
        },
        "service_area": {
            "reference_point": {
                "lat": PARIS_CITY_HALL.latitude,
                "lon": PARIS_CITY_HALL.longitude,
                "label": "Paris City Hall (Hôtel de Ville)"
            },
            "max_radius_meters": PARIS_SERVICE_AREA_MAX_METERS,
            "bounding_box": {
                "lat_min": 48.7,
                "lat_max": 49.0,
                "lon_min": 2.0,
                "lon_max": 2.6
            }
        },
        "units": {
            "distance": "meters",
            "time": "seconds_unix_utc",
            "walking_distance": "meters",
            "coordinates": "wgs84_decimal_degrees",
            "occupancy_rate": "ratio_0_to_1"
        },
        "caching": {
            "reference_ttl_seconds": REFERENCE_CACHE_TTL_MINUTES * 60,
            "realtime_ttl_seconds": REALTIME_CACHE_TTL_MINUTES * 60,
            "freshness_thresholds_minutes": {
                "fresh": "<10",
                "recent": "10-30",
                "stale": "30-120",
                "very_stale": ">120"
            }
        },
        "enums": enums(),
        "tools": tools_with_schemas(),
        "resources": resources(),
        "error_codes": error_codes()
    })
}

/// Render the API description as Markdown. Uses [`api_description`] as its
/// only data source so the two formats can never drift.
#[must_use]
pub fn api_description_markdown() -> String {
    let desc = api_description();
    let mut out = String::new();

    let name = desc["server"]["name"].as_str().unwrap_or(SERVER_NAME);
    let version = desc["server"]["version"].as_str().unwrap_or(SERVER_VERSION);
    out.push_str(&format!("# {name} API ({version})\n\n"));

    out.push_str("## Server\n\n");
    out.push_str(&format!("- **Name**: `{name}`\n"));
    out.push_str(&format!("- **Version**: `{version}`\n"));
    if let Some(protocol) = desc["server"]["protocol"].as_str() {
        out.push_str(&format!("- **MCP protocol**: `{protocol}`\n"));
    }
    out.push('\n');

    out.push_str("## Service area\n\n");
    let rp = &desc["service_area"]["reference_point"];
    if let (Some(lat), Some(lon), Some(label)) =
        (rp["lat"].as_f64(), rp["lon"].as_f64(), rp["label"].as_str())
    {
        out.push_str(&format!("Reference point: {label} ({lat:.4}, {lon:.4}).\n"));
    }
    if let Some(max_m) = desc["service_area"]["max_radius_meters"].as_f64() {
        out.push_str(&format!("Maximum radius: {max_m} meters.\n"));
    }
    out.push('\n');

    out.push_str("## Units\n\n");
    if let Some(units) = desc["units"].as_object() {
        for (k, v) in units {
            if let Some(vs) = v.as_str() {
                out.push_str(&format!("- `{k}`: {vs}\n"));
            }
        }
    }
    out.push('\n');

    out.push_str("## Caching\n\n");
    if let Some(ref_ttl) = desc["caching"]["reference_ttl_seconds"].as_i64() {
        out.push_str(&format!("- Reference data TTL: {ref_ttl}s\n"));
    }
    if let Some(rt_ttl) = desc["caching"]["realtime_ttl_seconds"].as_i64() {
        out.push_str(&format!("- Real-time data TTL: {rt_ttl}s\n"));
    }
    out.push('\n');

    out.push_str("## Enums\n\n");
    if let Some(enums) = desc["enums"].as_array() {
        for e in enums {
            if let Some(name) = e["name"].as_str() {
                out.push_str(&format!("### {name}\n\n"));
                if let Some(values) = e["values"].as_array() {
                    for v in values {
                        let val = v["value"].as_str().unwrap_or("");
                        let descr = v["description"].as_str().unwrap_or("");
                        out.push_str(&format!("- `{val}`: {descr}\n"));
                    }
                }
                out.push('\n');
            }
        }
    }

    out.push_str("## Tools\n\n");
    if let Some(tools) = desc["tools"].as_array() {
        for t in tools {
            let tname = t["name"].as_str().unwrap_or("");
            let summary = t["summary"].as_str().unwrap_or("");
            out.push_str(&format!("### `{tname}`\n\n"));
            if !summary.is_empty() {
                out.push_str(&format!("{summary}\n\n"));
            }
            if let Some(when) = t["when_to_use"].as_str() {
                out.push_str(&format!("**When to use**: {when}\n\n"));
            }
            out.push_str("**Input schema**:\n\n");
            out.push_str("```json\n");
            out.push_str(&serde_json::to_string_pretty(&t["input_schema"]).unwrap_or_default());
            out.push_str("\n```\n\n");
        }
    }

    out.push_str("## Resources\n\n");
    if let Some(resources) = desc["resources"].as_array() {
        for r in resources {
            let uri = r["uri"].as_str().unwrap_or("");
            let rname = r["name"].as_str().unwrap_or("");
            let rdesc = r["description"].as_str().unwrap_or("");
            out.push_str(&format!("- `{uri}` — **{rname}**: {rdesc}\n"));
        }
    }
    out.push('\n');

    out.push_str("## Error codes\n\n");
    if let Some(codes) = desc["error_codes"].as_array() {
        for c in codes {
            let code = c["code"].as_i64().unwrap_or(0);
            let cname = c["name"].as_str().unwrap_or("");
            let meaning = c["meaning"].as_str().unwrap_or("");
            out.push_str(&format!("- `{code}` ({cname}): {meaning}\n"));
        }
    }
    out.push('\n');

    out
}

fn enums() -> Value {
    json!([
        {
            "name": "StationStatus",
            "description": "Operational state of a Velib station",
            "values": [
                {"value": "OPEN", "description": "Station accepts rentals and returns."},
                {"value": "CLOSED", "description": "Station is out of service."},
                {"value": "MAINTENANCE", "description": "Station is physically installed but not currently renting or returning bikes."}
            ]
        },
        {
            "name": "DataFreshness",
            "description": "Age bucket of the real-time snapshot for a station.",
            "values": [
                {"value": "Fresh", "description": "Less than 10 minutes old."},
                {"value": "Recent", "description": "Between 10 and 30 minutes old."},
                {"value": "Stale", "description": "Between 30 and 120 minutes old."},
                {"value": "VeryStale", "description": "More than 120 minutes old."}
            ]
        },
        {
            "name": "BikeTypeFilter",
            "description": "Filter bikes by propulsion type.",
            "values": [
                {"value": "mechanical", "description": "Only mechanical (non-electric) bikes."},
                {"value": "electric", "description": "Only electric-assist bikes."},
                {"value": "any", "description": "Any bike type (default)."}
            ]
        }
    ])
}

fn tools_with_schemas() -> Value {
    let defs = tool_definitions();
    let mut out = Vec::with_capacity(defs.len());
    for def in &defs {
        let name = def["name"].as_str().unwrap_or("").to_string();
        let description = def["description"].as_str().unwrap_or("").to_string();
        let input_schema = def["inputSchema"].clone();
        let (when_to_use, output_schema, example_request, example_response, error_codes) =
            tool_metadata(&name);
        out.push(json!({
            "name": name,
            "summary": description,
            "when_to_use": when_to_use,
            "input_schema": input_schema,
            "output_schema": output_schema,
            "example_request": example_request,
            "example_response": example_response,
            "error_codes": error_codes
        }));
    }
    Value::Array(out)
}

fn tool_metadata(name: &str) -> (Value, Value, Value, Value, Value) {
    match name {
        "find_nearby_stations" => (
            json!("Given a user location, list nearby stations with distances and real-time availability."),
            json!({
                "type": "object",
                "properties": {
                    "stations": {"type": "array", "items": {"type": "object"}},
                    "search_metadata": {"type": "object"}
                },
                "required": ["stations", "search_metadata"]
            }),
            json!({
                "latitude": 48.8566,
                "longitude": 2.3522,
                "radius_meters": 500,
                "limit": 5
            }),
            json!({
                "stations": [],
                "search_metadata": {
                    "query_point": {"latitude": 48.8566, "longitude": 2.3522},
                    "radius_meters": 500,
                    "total_found": 0,
                    "search_time_ms": 12
                }
            }),
            json!([-32602, -32001]),
        ),
        "get_station_by_code" => (
            json!("Look up a single station by its canonical station code."),
            json!({
                "type": "object",
                "properties": {
                    "station": {"type": ["object", "null"]},
                    "found": {"type": "boolean"}
                },
                "required": ["station", "found"]
            }),
            json!({"station_code": "16107", "include_real_time": true}),
            json!({"station": null, "found": false}),
            json!([-32600, -32602, -32001]),
        ),
        "search_stations_by_name" => (
            json!("Search stations by (possibly fuzzy) name when the user refers to a station by label rather than coordinates."),
            json!({
                "type": "object",
                "properties": {
                    "stations": {"type": "array"},
                    "search_metadata": {"type": "object"}
                },
                "required": ["stations", "search_metadata"]
            }),
            json!({"query": "châtelet", "limit": 5, "fuzzy": true}),
            json!({
                "stations": [],
                "search_metadata": {
                    "query": "châtelet",
                    "total_found": 0,
                    "fuzzy_enabled": true,
                    "search_time_ms": 8
                }
            }),
            json!([-32602, -32001]),
        ),
        "get_area_statistics" => (
            json!("Aggregate bike and dock availability over a rectangular geographic region."),
            json!({
                "type": "object",
                "properties": {
                    "area_stats": {"type": "object"},
                    "bounds": {"type": "object"}
                },
                "required": ["area_stats", "bounds"]
            }),
            json!({
                "bounds": {"north": 48.87, "south": 48.85, "east": 2.36, "west": 2.34},
                "include_real_time": true
            }),
            json!({
                "area_stats": {
                    "total_stations": 0,
                    "operational_stations": 0,
                    "total_capacity": 0,
                    "available_bikes": {"mechanical": 0, "electric": 0, "total": 0},
                    "available_docks": 0,
                    "occupancy_rate": 0.0
                },
                "bounds": {"north": 48.87, "south": 48.85, "east": 2.36, "west": 2.34}
            }),
            json!([-32602, -32001]),
        ),
        "plan_bike_journey" => (
            json!("Propose pickup and drop-off station pairs for a bike trip between two points."),
            json!({
                "type": "object",
                "properties": {
                    "journey": {"type": "object"}
                },
                "required": ["journey"]
            }),
            json!({
                "origin": {"latitude": 48.8566, "longitude": 2.3522},
                "destination": {"latitude": 48.8606, "longitude": 2.3376}
            }),
            json!({
                "journey": {
                    "pickup_stations": [],
                    "dropoff_stations": [],
                    "recommendations": []
                }
            }),
            json!([-32602, -32001]),
        ),
        "describe_api" => (
            json!("Call once at session start so the agent knows every tool, every enum, and every error code without reading source."),
            json!({
                "type": "object",
                "properties": {
                    "content": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": {"type": "string", "enum": ["text"]},
                                "text": {"type": "string"}
                            }
                        }
                    }
                }
            }),
            json!({"format": "json"}),
            json!({"content": [{"type": "text", "text": "{...api description...}"}]}),
            json!([-32602]),
        ),
        _ => (
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Array(vec![]),
        ),
    }
}

fn resources() -> Value {
    json!([
        {
            "uri": "velib://stations/reference",
            "name": "Velib Station Reference Data",
            "description": "Complete catalog of Velib stations with static metadata.",
            "mime_type": "application/json",
            "freshness": "reference"
        },
        {
            "uri": "velib://stations/realtime",
            "name": "Velib Real-time Availability",
            "description": "Current bike and dock availability for all stations.",
            "mime_type": "application/json",
            "freshness": "realtime"
        },
        {
            "uri": "velib://stations/complete",
            "name": "Velib Complete Station Data",
            "description": "Combined reference and real-time data for all stations.",
            "mime_type": "application/json",
            "freshness": "realtime"
        },
        {
            "uri": "velib://health",
            "name": "Service Health Status",
            "description": "System health and data source status information.",
            "mime_type": "application/json",
            "freshness": "realtime"
        },
        {
            "uri": "velib://api/description",
            "name": "Velib MCP API Self-Description",
            "description": "Machine-readable description of this MCP API.",
            "mime_type": "application/json",
            "freshness": "static"
        }
    ])
}

fn error_codes() -> Value {
    json!([
        {"code": -32700, "name": "parse_error", "meaning": "Invalid JSON was received by the server."},
        {"code": -32600, "name": "invalid_request", "meaning": "The JSON sent is not a valid request; also used for station_not_found."},
        {"code": -32601, "name": "method_not_found", "meaning": "The requested JSON-RPC method does not exist."},
        {"code": -32602, "name": "invalid_params", "meaning": "Invalid method parameters: bad coordinates, outside service area, search radius too large, result limit exceeded, or validation failure."},
        {"code": -32603, "name": "internal_error", "meaning": "Server-side error: MCP protocol issue, cache failure, or unclassified internal error."},
        {"code": -32001, "name": "server_error", "meaning": "Upstream error: HTTP failure talking to Paris Open Data, including rate limiting (HTTP 429)."}
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn description_has_expected_top_level_keys() {
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
            assert!(desc.get(key).is_some(), "missing top-level key: {key}");
        }
    }

    #[test]
    fn caching_ttls_reflect_imported_constants() {
        let desc = api_description();
        assert_eq!(
            desc["caching"]["reference_ttl_seconds"].as_i64(),
            Some(REFERENCE_CACHE_TTL_MINUTES * 60)
        );
        assert_eq!(
            desc["caching"]["realtime_ttl_seconds"].as_i64(),
            Some(REALTIME_CACHE_TTL_MINUTES * 60)
        );
    }

    #[test]
    fn markdown_starts_with_h1_and_mentions_tools() {
        let md = api_description_markdown();
        assert!(md.starts_with("# "), "markdown must start with `# `");
        for tool in [
            "find_nearby_stations",
            "get_station_by_code",
            "search_stations_by_name",
            "get_area_statistics",
            "plan_bike_journey",
            "describe_api",
        ] {
            assert!(md.contains(tool), "markdown missing tool {tool}");
        }
    }
}
