//! Self-documentation for the Velib MCP server.
//!
//! Issue #9 — "MCP documentation endpoint". LLMs that consume this server need
//! more context than a bare JSON Schema can convey: data-freshness/cache
//! semantics, units on every numerical value, the meaning of every enum
//! variant, and Velib-specific gotchas (e.g. the 50 km service-area limit,
//! `(latitude, longitude)` order, station-code conventions). This module is
//! the single source of truth for that context.
//!
//! ## Design choices
//!
//! - **Hardcoded, not introspected.** The schema is built from `const`
//!   functions that mirror what `tools/list` advertises. Runtime introspection
//!   (e.g. reflecting over serde-derived types) was rejected because it cannot
//!   capture the per-field units, enum value definitions, freshness/cache
//!   semantics, or example payloads that LLMs actually need.
//! - **Drift is caught at test time.** [`tool_names`] enumerates every tool
//!   documented here; tests in this module (and in
//!   `tests/mcp_server_routing_tests.rs`) assert that this list matches the
//!   set of tools served by `tools/list` and dispatched by `tools/call`.
//!   Adding a tool without documenting it fails CI.
//! - **Two formats.** JSON is the default (machine-friendly, RFC-compatible
//!   with the rest of the MCP surface). Markdown is offered because in
//!   practice LLMs read prose narrative better than nested JSON; it is
//!   rendered from the same `ApiDocumentation` value, so the content cannot
//!   diverge.
//!
//! See [`api_documentation`] for the full structured value and
//! [`render_markdown`] for the LLM-friendly prose form.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Output format requested by the caller. Defaults to JSON.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DocumentationFormat {
    /// Machine-readable JSON. Default.
    #[default]
    Json,
    /// Human-readable Markdown, friendlier for LLM prompts.
    Markdown,
}

/// Input parameters for the `get_api_documentation` tool.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GetApiDocumentationInput {
    /// Output format. Defaults to JSON when omitted.
    #[serde(default)]
    pub format: DocumentationFormat,
}

/// Output of the `get_api_documentation` tool.
///
/// The payload always includes the rendered documentation (in the requested
/// format) and echoes the format that was used, so callers can pipe the
/// response into the right parser without re-deriving it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetApiDocumentationOutput {
    pub format: DocumentationFormat,
    /// JSON value when `format == Json`, otherwise a Markdown string wrapped
    /// in a JSON string. Kept as `serde_json::Value` so the `tools/call`
    /// wrapper can serialize it without an extra branch.
    pub documentation: Value,
}

/// The canonical list of MCP tool names this server exposes.
///
/// Must stay in sync with [`api_documentation`] and the `tools/list` /
/// `tools/call` dispatch in `src/mcp/server.rs`. Drift is caught by the unit
/// tests in this module and by `tools_list_includes_get_api_documentation` in
/// `tests/mcp_server_routing_tests.rs`.
pub const TOOL_NAMES: &[&str] = &[
    "find_nearby_stations",
    "get_station_by_code",
    "search_stations_by_name",
    "get_area_statistics",
    "plan_bike_journey",
    "get_api_documentation",
];

/// Cache TTL advertised for reference station data (minutes).
///
/// Must match `REFERENCE_CACHE_TTL_MINUTES` in `src/data/client.rs`. The
/// constant is intentionally re-declared here (rather than re-exported) so
/// the documentation file stays self-contained and so callers see exactly
/// what's documented, not the implementation detail.
pub const REFERENCE_CACHE_TTL_MINUTES: u32 = 5;

/// Cache TTL advertised for real-time station availability (minutes).
///
/// Must match `REALTIME_CACHE_TTL_MINUTES` in `src/data/client.rs`.
pub const REALTIME_CACHE_TTL_MINUTES: u32 = 2;

/// Service-area radius around Paris City Hall, in kilometres.
///
/// Coordinates outside this radius are rejected by
/// `ensure_in_service_area` in `src/mcp/handlers.rs`. The value is also
/// enforced in `Coordinates::is_within_paris_service_area`.
pub const SERVICE_AREA_RADIUS_KM: u32 = 50;

/// Maximum search radius (metres) accepted by `find_nearby_stations`.
pub const MAX_SEARCH_RADIUS_METERS: u32 = 5_000;

/// Maximum number of results any tool will return.
pub const MAX_RESULT_LIMIT: u16 = 100;

/// Build the structured API documentation for every MCP tool the server
/// exposes.
///
/// The returned value is the canonical JSON representation; the Markdown
/// rendering is derived from this same value via [`render_markdown`].
#[must_use]
pub fn api_documentation() -> Value {
    json!({
        "schema_version": "1.0.0",
        "server": {
            "name": "velib-mcp",
            "description": "MCP server exposing the Velib Paris bike-sharing open data: real-time station availability and station reference metadata.",
            "data_sources": [
                {
                    "name": "Velib real-time availability",
                    "url": "https://opendata.paris.fr/explore/dataset/velib-disponibilite-en-temps-reel/",
                    "cache_ttl_minutes": REALTIME_CACHE_TTL_MINUTES,
                    "notes": "Bike and dock counts, station open/closed status, last_update timestamp."
                },
                {
                    "name": "Velib station locations",
                    "url": "https://opendata.paris.fr/explore/dataset/velib-emplacement-des-stations/",
                    "cache_ttl_minutes": REFERENCE_CACHE_TTL_MINUTES,
                    "notes": "Station code, name, coordinates (WGS84), capacity, service capabilities."
                }
            ],
            "service_area": {
                "description": "All coordinate inputs MUST be within a 50 km radius of Paris City Hall (48.8565° N, 2.3514° E). Outside this radius the server returns OutsideServiceArea (-32004).",
                "radius_km": SERVICE_AREA_RADIUS_KM,
                "anchor": { "latitude": 48.8565, "longitude": 2.3514 }
            },
            "coordinate_convention": {
                "order": "(latitude, longitude)",
                "system": "WGS84 decimal degrees",
                "latitude_range": [48.7, 49.0],
                "longitude_range": [2.0, 2.6],
                "notes": "Paris metro bounding box. Coordinates outside this box are rejected with InvalidCoordinates (-32001)."
            },
            "station_code_convention": "String identifier assigned by Velib. Treat as opaque; do not parse. Use `get_station_by_code` or `search_stations_by_name` to discover codes."
        },
        "enums": enums_documentation(),
        "freshness": {
            "DataFreshness": {
                "description": "Age bucket of a real-time observation. Computed from `last_update` to `now()`.",
                "values": {
                    "Fresh":     "Less than 10 minutes old. Safe to act on.",
                    "Recent":    "10 to 30 minutes old. Usable but consider staleness.",
                    "Stale":     "30 to 120 minutes old. Likely out of date.",
                    "VeryStale": "More than 120 minutes old. Treat as unreliable."
                }
            },
            "cache_policy": {
                "reference_ttl_minutes": REFERENCE_CACHE_TTL_MINUTES,
                "realtime_ttl_minutes": REALTIME_CACHE_TTL_MINUTES,
                "notes": "Responses may be served from cache. The included `last_update` timestamp reflects the upstream observation time, not the cache hit time."
            }
        },
        "tools": tool_specs(),
        "error_codes": {
            "-32700": "Parse error (malformed JSON-RPC request)",
            "-32601": "Method not found (unknown JSON-RPC method or MCP tool)",
            "-32602": "Invalid params (deserialization failure or schema violation)",
            "-32603": "Internal error",
            "-32001": "InvalidCoordinates — latitude/longitude outside the Paris metro bounding box.",
            "-32002": "SearchRadiusTooLarge — radius_meters exceeds the 5 000 m cap.",
            "-32003": "ResultLimitExceeded — limit exceeds the 100-result cap.",
            "-32004": "OutsideServiceArea — coordinate more than 50 km from Paris City Hall.",
            "-32005": "Validation — generic input validation failure (e.g. search query too short/long)."
        }
    })
}

/// Documentation of every enum surfaced through the MCP API, including the
/// wire value of each variant and what it means.
fn enums_documentation() -> Value {
    json!({
        "BikeTypeFilter": {
            "description": "Filter on the type of bike a station provides.",
            "values": {
                "mechanical": "Pedal-only bikes. No motor.",
                "electric":   "Electrically assisted bikes (VAE).",
                "any":        "Either type. Default."
            }
        },
        "StationStatus": {
            "description": "Operational status of a station, sourced from the real-time feed.",
            "values": {
                "OPEN":        "Station is open and renting bikes.",
                "CLOSED":      "Station is closed; cannot rent or return.",
                "MAINTENANCE": "Station is under maintenance; treat as unavailable."
            }
        }
    })
}

/// Per-tool documentation: name, description, when to use, full input/output
/// schema with units and enum references, and an example response.
fn tool_specs() -> Value {
    json!([
        {
            "name": "find_nearby_stations",
            "description": "Find Velib stations within a radius of a coordinate, sorted by straight-line distance.",
            "when_to_use": "When the user has a location (lat/lon) and wants to know which stations are nearby — typically with constraints on bike type or operational status.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "latitude":  { "type": "number", "minimum": 48.7, "maximum": 49.0, "units": "degrees (WGS84)" },
                    "longitude": { "type": "number", "minimum": 2.0,  "maximum": 2.6,  "units": "degrees (WGS84)" },
                    "radius_meters": {
                        "type": "integer", "minimum": 100, "maximum": MAX_SEARCH_RADIUS_METERS,
                        "default": 500, "units": "metres"
                    },
                    "limit": {
                        "type": "integer", "minimum": 1, "maximum": MAX_RESULT_LIMIT,
                        "default": 10, "units": "stations"
                    },
                    "availability_filter": {
                        "type": "object",
                        "description": "Optional filter on bike availability.",
                        "properties": {
                            "min_bikes": { "type": "integer", "minimum": 0, "units": "bikes" },
                            "min_docks": { "type": "integer", "minimum": 0, "units": "docks" },
                            "bike_type": { "$ref": "#/enums/BikeTypeFilter" },
                            "exclude_out_of_service": { "type": "boolean", "default": true }
                        }
                    }
                },
                "required": ["latitude", "longitude"]
            },
            "output_schema": {
                "type": "object",
                "properties": {
                    "stations": {
                        "type": "array",
                        "description": "Matching stations, sorted by ascending distance.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "straight_line_distance_meters": { "type": "integer", "units": "metres" }
                            }
                        }
                    },
                    "search_metadata": {
                        "type": "object",
                        "properties": {
                            "query_point": { "type": "object", "description": "Echoed input coordinate." },
                            "radius_meters":   { "type": "integer", "units": "metres" },
                            "total_found":     { "type": "integer", "units": "stations" },
                            "search_time_ms":  { "type": "integer", "units": "milliseconds" }
                        }
                    }
                }
            },
            "freshness": "Reads from the real-time cache (TTL 2 minutes). The result reflects the most recent cached snapshot of station availability.",
            "example_response": {
                "stations": [
                    {
                        "reference": { "station_code": "16107", "name": "Benjamin Godard - Victor Hugo" },
                        "straight_line_distance_meters": 142
                    }
                ],
                "search_metadata": {
                    "query_point": { "latitude": 48.8566, "longitude": 2.3522 },
                    "radius_meters": 500,
                    "total_found": 1,
                    "search_time_ms": 12
                }
            }
        },
        {
            "name": "get_station_by_code",
            "description": "Fetch a single station's reference data and (optionally) its current real-time availability.",
            "when_to_use": "When you already know a station_code — e.g. from a previous search result — and want its full details.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "station_code": {
                        "type": "string",
                        "description": "Opaque Velib station identifier. Do not parse; treat as a key."
                    },
                    "include_real_time": { "type": "boolean", "default": true }
                },
                "required": ["station_code"]
            },
            "output_schema": {
                "type": "object",
                "properties": {
                    "station": {
                        "type": ["object", "null"],
                        "description": "Null when `found` is false."
                    },
                    "found": { "type": "boolean" }
                }
            },
            "freshness": "Reference data: cache TTL 5 minutes. Real-time data: cache TTL 2 minutes. `station.real_time.last_update` reflects the upstream observation time.",
            "example_response": {
                "found": true,
                "station": {
                    "reference": { "station_code": "16107", "name": "Benjamin Godard - Victor Hugo", "capacity": 35 },
                    "real_time": {
                        "bikes": { "mechanical": 4, "electric": 6 },
                        "available_docks": 25,
                        "status": "OPEN",
                        "data_freshness": "Fresh"
                    }
                }
            }
        },
        {
            "name": "search_stations_by_name",
            "description": "Search stations whose name matches a query, with prefix or substring matching.",
            "when_to_use": "When the user names a landmark, street, or place (e.g. 'République', 'Bastille') and you need to find candidate stations. Names are normalised (case-fold + Unicode NFC) before matching.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string", "minLength": 2, "maxLength": 100,
                        "description": "Length is measured in Unicode code points, not bytes. Accented characters count as one."
                    },
                    "limit": { "type": "integer", "minimum": 1, "maximum": MAX_RESULT_LIMIT, "default": 10, "units": "stations" },
                    "fuzzy": {
                        "type": "boolean", "default": true,
                        "description": "true = substring match; false = prefix match."
                    }
                },
                "required": ["query"]
            },
            "output_schema": {
                "type": "object",
                "properties": {
                    "stations": { "type": "array", "description": "Matching stations, sorted by name." },
                    "search_metadata": {
                        "type": "object",
                        "properties": {
                            "query":          { "type": "string" },
                            "total_found":    { "type": "integer", "units": "stations" },
                            "fuzzy_enabled":  { "type": "boolean" },
                            "search_time_ms": { "type": "integer", "units": "milliseconds" }
                        }
                    }
                }
            },
            "freshness": "Reads from the reference cache (TTL 5 minutes) for the station list. Real-time fields, when present on each station, reflect a snapshot no older than 2 minutes.",
            "example_response": {
                "stations": [
                    { "reference": { "station_code": "11104", "name": "République" } }
                ],
                "search_metadata": {
                    "query": "république", "total_found": 1, "fuzzy_enabled": true, "search_time_ms": 8
                }
            }
        },
        {
            "name": "get_area_statistics",
            "description": "Aggregate bike, dock, and capacity totals across every station inside a geographic bounding box.",
            "when_to_use": "When you need a single-number summary of supply/demand for a neighbourhood, arrondissement, or other rectangular area.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "bounds": {
                        "type": "object",
                        "description": "Axis-aligned bounding box in WGS84 decimal degrees.",
                        "properties": {
                            "north": { "type": "number", "units": "degrees latitude" },
                            "south": { "type": "number", "units": "degrees latitude" },
                            "east":  { "type": "number", "units": "degrees longitude" },
                            "west":  { "type": "number", "units": "degrees longitude" }
                        },
                        "required": ["north", "south", "east", "west"]
                    },
                    "include_real_time": { "type": "boolean", "default": true }
                },
                "required": ["bounds"]
            },
            "output_schema": {
                "type": "object",
                "properties": {
                    "area_stats": {
                        "type": "object",
                        "properties": {
                            "total_stations":       { "type": "integer", "units": "stations" },
                            "operational_stations": { "type": "integer", "units": "stations" },
                            "total_capacity":       { "type": "integer", "units": "docks" },
                            "available_bikes": {
                                "type": "object",
                                "properties": {
                                    "mechanical": { "type": "integer", "units": "bikes" },
                                    "electric":   { "type": "integer", "units": "bikes" },
                                    "total":      { "type": "integer", "units": "bikes" }
                                }
                            },
                            "available_docks": { "type": "integer", "units": "docks" },
                            "occupancy_rate": {
                                "type": "number",
                                "description": "available_bikes.total / total_capacity. 0.0 when capacity is zero.",
                                "units": "ratio in [0, 1]"
                            }
                        }
                    },
                    "bounds": { "type": "object", "description": "Echoed input bounds." }
                }
            },
            "freshness": "Reads from the real-time cache (TTL 2 minutes). Stations without a real-time observation count toward `total_stations` and `total_capacity` but contribute zero bikes/docks.",
            "example_response": {
                "bounds": { "north": 48.87, "south": 48.85, "east": 2.36, "west": 2.34 },
                "area_stats": {
                    "total_stations": 12, "operational_stations": 11, "total_capacity": 380,
                    "available_bikes": { "mechanical": 84, "electric": 31, "total": 115 },
                    "available_docks": 261, "occupancy_rate": 0.3026
                }
            }
        },
        {
            "name": "plan_bike_journey",
            "description": "Suggest pickup and dropoff stations for a Velib trip between two coordinates, with a confidence score.",
            "when_to_use": "When the user wants to plan a bike trip: provide origin and destination coordinates and (optionally) walking budget and bike-type preferences.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "origin":      { "type": "object", "description": "{ latitude, longitude } WGS84." },
                    "destination": { "type": "object", "description": "{ latitude, longitude } WGS84." },
                    "preferences": {
                        "type": "object",
                        "properties": {
                            "bike_type":         { "$ref": "#/enums/BikeTypeFilter", "default": "any" },
                            "max_walk_distance": { "type": "integer", "default": 500, "units": "metres" }
                        }
                    }
                },
                "required": ["origin", "destination"]
            },
            "output_schema": {
                "type": "object",
                "properties": {
                    "journey": {
                        "type": "object",
                        "properties": {
                            "pickup_stations":  { "type": "array", "description": "Up to 3 candidates near the origin, sorted by distance." },
                            "dropoff_stations": { "type": "array", "description": "Up to 3 candidates near the destination, sorted by distance." },
                            "recommendations": {
                                "type": "array",
                                "description": "Recommended pickup/dropoff pairs.",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "straight_line_to_pickup_meters":     { "type": "integer", "units": "metres" },
                                        "straight_line_from_dropoff_meters":  { "type": "integer", "units": "metres" },
                                        "confidence_score": {
                                            "type": "number",
                                            "description": "1.0 = stations at the exact origin/destination; 0.5 = walking budget fully used on both ends; floor 0.1.",
                                            "units": "ratio in [0.1, 1.0]"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "freshness": "Reads from the real-time cache (TTL 2 minutes) to pick stations with available bikes (pickup) and available docks (dropoff).",
            "example_response": {
                "journey": {
                    "pickup_stations":  [{ "reference": { "station_code": "9020" }, "straight_line_distance_meters": 90 }],
                    "dropoff_stations": [{ "reference": { "station_code": "11104" }, "straight_line_distance_meters": 120 }],
                    "recommendations": [{
                        "straight_line_to_pickup_meters": 90,
                        "straight_line_from_dropoff_meters": 120,
                        "confidence_score": 0.79
                    }]
                }
            }
        },
        {
            "name": "get_api_documentation",
            "description": "Return the self-documentation of this MCP server: every tool's purpose, full input/output schema, units, enum definitions, data-freshness/caching semantics, and an example response.",
            "when_to_use": "Call once at the start of a session to ground your understanding of this server's surface. Prefer `format: \"markdown\"` for prompting LLMs; use `format: \"json\"` (default) for programmatic consumption.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "format": {
                        "type": "string",
                        "enum": ["json", "markdown"],
                        "default": "json",
                        "description": "Output format. JSON returns the structured schema verbatim; markdown returns a human-friendly prose rendering of the same content."
                    }
                }
            },
            "output_schema": {
                "type": "object",
                "properties": {
                    "format":        { "type": "string", "enum": ["json", "markdown"] },
                    "documentation": { "description": "Object when format=json, string when format=markdown." }
                }
            },
            "freshness": "Static. Hardcoded in source; updated whenever a tool is added or changed (enforced by unit tests)."
        }
    ])
}

/// Render the structured documentation as Markdown. The Markdown form is
/// derived from the same JSON value returned by [`api_documentation`], so the
/// two outputs cannot drift apart in content.
#[must_use]
pub fn render_markdown() -> String {
    let doc = api_documentation();
    let mut out = String::new();

    out.push_str("# Velib MCP — API Documentation\n\n");
    if let Some(server) = doc.get("server") {
        if let Some(d) = server.get("description").and_then(Value::as_str) {
            out.push_str(d);
            out.push_str("\n\n");
        }
        out.push_str("## Service area & coordinate conventions\n\n");
        if let Some(s) = server
            .get("service_area")
            .and_then(|v| v.get("description"))
            .and_then(Value::as_str)
        {
            out.push_str("- ");
            out.push_str(s);
            out.push('\n');
        }
        if let Some(c) = server.get("coordinate_convention") {
            if let Some(order) = c.get("order").and_then(Value::as_str) {
                out.push_str(&format!(
                    "- Coordinate order: **{order}**, WGS84 decimal degrees.\n"
                ));
            }
        }
        if let Some(s) = server
            .get("station_code_convention")
            .and_then(Value::as_str)
        {
            out.push_str(&format!("- Station code: {s}\n"));
        }
        out.push('\n');

        out.push_str("## Data sources & freshness\n\n");
        if let Some(arr) = server.get("data_sources").and_then(Value::as_array) {
            for src in arr {
                let name = src.get("name").and_then(Value::as_str).unwrap_or("");
                let ttl = src
                    .get("cache_ttl_minutes")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                let notes = src.get("notes").and_then(Value::as_str).unwrap_or("");
                out.push_str(&format!("- **{name}** (cache TTL: {ttl} min) — {notes}\n"));
            }
        }
        out.push('\n');
    }

    if let Some(enums) = doc.get("enums").and_then(Value::as_object) {
        out.push_str("## Enums\n\n");
        for (name, def) in enums {
            out.push_str(&format!("### `{name}`\n\n"));
            if let Some(d) = def.get("description").and_then(Value::as_str) {
                out.push_str(d);
                out.push_str("\n\n");
            }
            if let Some(values) = def.get("values").and_then(Value::as_object) {
                for (variant, meaning) in values {
                    let m = meaning.as_str().unwrap_or("");
                    out.push_str(&format!("- `{variant}` — {m}\n"));
                }
                out.push('\n');
            }
        }
    }

    if let Some(freshness) = doc.get("freshness") {
        out.push_str("## Data freshness\n\n");
        if let Some(df) = freshness.get("DataFreshness") {
            if let Some(values) = df.get("values").and_then(Value::as_object) {
                for (variant, meaning) in values {
                    let m = meaning.as_str().unwrap_or("");
                    out.push_str(&format!("- `{variant}` — {m}\n"));
                }
                out.push('\n');
            }
        }
        if let Some(cp) = freshness.get("cache_policy") {
            if let (Some(r), Some(rt)) = (
                cp.get("reference_ttl_minutes").and_then(Value::as_u64),
                cp.get("realtime_ttl_minutes").and_then(Value::as_u64),
            ) {
                out.push_str(&format!(
                    "Cache policy: reference data {r} min, real-time data {rt} min.\n\n"
                ));
            }
        }
    }

    if let Some(tools) = doc.get("tools").and_then(Value::as_array) {
        out.push_str("## Tools\n\n");
        for tool in tools {
            let name = tool.get("name").and_then(Value::as_str).unwrap_or("");
            let desc = tool
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("");
            let when = tool
                .get("when_to_use")
                .and_then(Value::as_str)
                .unwrap_or("");
            let fresh = tool.get("freshness").and_then(Value::as_str).unwrap_or("");
            out.push_str(&format!("### `{name}`\n\n"));
            out.push_str(&format!("{desc}\n\n"));
            if !when.is_empty() {
                out.push_str(&format!("**When to use:** {when}\n\n"));
            }
            if !fresh.is_empty() {
                out.push_str(&format!("**Freshness:** {fresh}\n\n"));
            }
            out.push_str("**Input schema:**\n\n```json\n");
            out.push_str(
                &serde_json::to_string_pretty(tool.get("input_schema").unwrap_or(&Value::Null))
                    .unwrap_or_default(),
            );
            out.push_str("\n```\n\n");
            out.push_str("**Output schema:**\n\n```json\n");
            out.push_str(
                &serde_json::to_string_pretty(tool.get("output_schema").unwrap_or(&Value::Null))
                    .unwrap_or_default(),
            );
            out.push_str("\n```\n\n");
            if let Some(ex) = tool.get("example_response") {
                out.push_str("**Example response:**\n\n```json\n");
                out.push_str(&serde_json::to_string_pretty(ex).unwrap_or_default());
                out.push_str("\n```\n\n");
            }
        }
    }

    if let Some(errs) = doc.get("error_codes").and_then(Value::as_object) {
        out.push_str("## Error codes\n\n");
        for (code, desc) in errs {
            out.push_str(&format!("- `{code}` — {}\n", desc.as_str().unwrap_or("")));
        }
        out.push('\n');
    }

    out
}

/// Execute the `get_api_documentation` tool.
#[must_use]
pub fn run(input: GetApiDocumentationInput) -> GetApiDocumentationOutput {
    match input.format {
        DocumentationFormat::Json => GetApiDocumentationOutput {
            format: DocumentationFormat::Json,
            documentation: api_documentation(),
        },
        DocumentationFormat::Markdown => GetApiDocumentationOutput {
            format: DocumentationFormat::Markdown,
            documentation: Value::String(render_markdown()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every documented tool must have all the fields LLMs need to use it
    /// correctly. A missing field on any tool is a documentation bug.
    #[test]
    fn every_tool_has_required_documentation_fields() {
        let doc = api_documentation();
        let tools = doc["tools"].as_array().expect("tools array");
        assert_eq!(
            tools.len(),
            TOOL_NAMES.len(),
            "tool count drift: docs={}, TOOL_NAMES={}",
            tools.len(),
            TOOL_NAMES.len()
        );
        for tool in tools {
            let name = tool["name"].as_str().expect("name is string");
            assert!(
                tool["description"].is_string(),
                "tool {name} missing description"
            );
            assert!(
                tool["when_to_use"].is_string(),
                "tool {name} missing when_to_use"
            );
            assert!(
                tool["input_schema"].is_object(),
                "tool {name} missing input_schema"
            );
            assert!(
                tool["output_schema"].is_object(),
                "tool {name} missing output_schema"
            );
            assert!(
                tool["freshness"].is_string(),
                "tool {name} missing freshness note"
            );
            assert_eq!(
                tool["input_schema"]["type"], "object",
                "tool {name} input_schema is not an object schema"
            );
        }
    }

    /// `TOOL_NAMES` must list every tool that appears in the structured
    /// documentation, and vice versa. This guards the parity contract that
    /// `tests/mcp_server_routing_tests.rs` then enforces against the live
    /// `tools/list` dispatch.
    #[test]
    fn tool_names_match_documented_tools() {
        let doc = api_documentation();
        let mut documented: Vec<&str> = doc["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        documented.sort_unstable();
        let mut declared: Vec<&str> = TOOL_NAMES.to_vec();
        declared.sort_unstable();
        assert_eq!(
            declared, documented,
            "TOOL_NAMES drifted from the documented tool list"
        );
    }

    #[test]
    fn enums_contain_bike_type_and_station_status() {
        let doc = api_documentation();
        let enums = doc["enums"].as_object().unwrap();
        assert!(enums.contains_key("BikeTypeFilter"));
        assert!(enums.contains_key("StationStatus"));

        // Each enum variant must be documented with a non-empty string.
        for def in enums.values() {
            let values = def["values"].as_object().unwrap();
            assert!(!values.is_empty(), "enum has no values documented");
            for (variant, meaning) in values {
                let s = meaning.as_str().unwrap_or("");
                assert!(
                    !s.is_empty(),
                    "enum variant {variant} has empty documentation"
                );
            }
        }
    }

    #[test]
    fn freshness_section_mentions_cache_ttls() {
        let doc = api_documentation();
        let cp = &doc["freshness"]["cache_policy"];
        assert_eq!(cp["reference_ttl_minutes"], REFERENCE_CACHE_TTL_MINUTES);
        assert_eq!(cp["realtime_ttl_minutes"], REALTIME_CACHE_TTL_MINUTES);
    }

    #[test]
    fn service_area_radius_is_50km() {
        let doc = api_documentation();
        assert_eq!(doc["server"]["service_area"]["radius_km"], 50);
    }

    #[test]
    fn run_with_default_input_returns_json() {
        let output = run(GetApiDocumentationInput::default());
        assert_eq!(output.format, DocumentationFormat::Json);
        assert!(
            output.documentation.is_object(),
            "default JSON output should be an object"
        );
        assert!(output.documentation["tools"].is_array());
    }

    #[test]
    fn run_with_markdown_returns_string_payload() {
        let output = run(GetApiDocumentationInput {
            format: DocumentationFormat::Markdown,
        });
        assert_eq!(output.format, DocumentationFormat::Markdown);
        let md = output.documentation.as_str().expect("markdown is a string");
        assert!(md.starts_with("# Velib MCP"), "missing top-level heading");
        // Every tool name must appear in the rendered markdown.
        for name in TOOL_NAMES {
            assert!(md.contains(name), "markdown does not mention tool `{name}`");
        }
        // Critical Velib-specific gotchas must be surfaced for LLMs.
        assert!(
            md.contains("50 km"),
            "service area limit missing from markdown"
        );
        assert!(
            md.contains("WGS84"),
            "coordinate system missing from markdown"
        );
    }

    #[test]
    fn format_defaults_to_json_when_deserialized_from_empty_object() {
        let input: GetApiDocumentationInput = serde_json::from_value(json!({})).unwrap();
        assert_eq!(input.format, DocumentationFormat::Json);
    }

    #[test]
    fn format_deserializes_lowercase_variants() {
        let json_input: GetApiDocumentationInput =
            serde_json::from_value(json!({"format": "json"})).unwrap();
        assert_eq!(json_input.format, DocumentationFormat::Json);
        let md_input: GetApiDocumentationInput =
            serde_json::from_value(json!({"format": "markdown"})).unwrap();
        assert_eq!(md_input.format, DocumentationFormat::Markdown);
    }
}
