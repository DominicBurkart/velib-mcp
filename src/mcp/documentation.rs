//! Self-documentation payload for the Velib MCP server.
//!
//! Produces a single structured JSON document describing every tool and
//! resource exposed by the server: input/output schemas, units for
//! numerical values, all enum variants with definitions, cache TTLs and
//! data-freshness behavior, and the JSON-RPC error code map.
//!
//! The payload is intentionally hand-curated rather than derived from
//! reflection so that descriptions and units stay tight and useful for
//! LLMs (issue #9). Drift against runtime values (cache TTLs, advertised
//! tools, advertised resources, error codes) is caught by
//! `tests/mcp_documentation_tests.rs`.
//!
//! Exposed two ways:
//! - JSON-RPC method `docs/describe` (no params).
//! - MCP resource URI `velib://docs/api`.
//!
//! Both return byte-identical JSON.

use serde_json::{json, Value};

use crate::data::{REALTIME_CACHE_TTL_MINUTES, REFERENCE_CACHE_TTL_MINUTES};
use crate::types::{PARIS_CITY_HALL, PARIS_SERVICE_AREA_MAX_METERS};

/// Stable URI under which the documentation resource is served.
pub const DOCS_RESOURCE_URI: &str = "velib://docs/api";

/// Server semantic version reported in the documentation payload.
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build the self-documentation payload.
///
/// Returns a `serde_json::Value` so the same structure can be returned as
/// the JSON-RPC `result` and as the body of the resource endpoint without
/// any reformatting.
#[must_use]
pub fn build_api_description() -> Value {
    json!({
        "schema_version": "1.0.0",
        "server": {
            "name": "velib-mcp",
            "version": SERVER_VERSION,
            "description": "Model Context Protocol server exposing Paris Velib bike-sharing data \
                (real-time availability and station reference metadata) for AI assistants.",
            "transport": {
                "protocol": "JSON-RPC 2.0",
                "encoding": "UTF-8",
                "channels": ["HTTP POST /mcp", "WebSocket /mcp/ws"]
            },
            "service_area": {
                "reference_point": {
                    "name": "Paris City Hall (Hôtel de Ville)",
                    "latitude": PARIS_CITY_HALL.latitude,
                    "longitude": PARIS_CITY_HALL.longitude
                },
                "max_distance_meters": PARIS_SERVICE_AREA_MAX_METERS,
                "notes": "All input coordinates are validated against this 50 km radius before any I/O."
            }
        },
        "data_freshness": {
            "description": "Upstream Paris Open Data is fetched on demand and cached in-process. \
                Documentation values reflect the actual TTLs used at runtime.",
            "caches": [
                {
                    "name": "reference",
                    "ttl_minutes": REFERENCE_CACHE_TTL_MINUTES,
                    "scope": "Static station metadata (location, capacity, capabilities).",
                    "source": "https://opendata.paris.fr/.../velib-emplacement-des-stations"
                },
                {
                    "name": "realtime",
                    "ttl_minutes": REALTIME_CACHE_TTL_MINUTES,
                    "scope": "Live bike/dock availability and station status.",
                    "source": "https://opendata.paris.fr/.../velib-disponibilite-en-temps-reel"
                }
            ],
            "freshness_classifier": {
                "field": "data_freshness",
                "unit": "minutes since last_update",
                "ranges": [
                    {"value": "Fresh",     "max_minutes_exclusive": 10},
                    {"value": "Recent",    "max_minutes_exclusive": 30},
                    {"value": "Stale",     "max_minutes_exclusive": 120},
                    {"value": "VeryStale", "max_minutes_exclusive": null}
                ]
            }
        },
        "units": {
            "latitude":  {"type": "decimal degrees", "range": [-90.0, 90.0]},
            "longitude": {"type": "decimal degrees", "range": [-180.0, 180.0]},
            "distance":  {"type": "meters", "notes": "Straight-line (Haversine) unless otherwise noted."},
            "duration":  {"type": "seconds"},
            "ttl":       {"type": "minutes"},
            "capacity":  {"type": "count of docks"},
            "bikes":     {"type": "count of bikes"},
            "occupancy_rate": {"type": "ratio in [0.0, 1.0]"},
            "confidence_score": {"type": "ratio in [0.0, 1.0]"}
        },
        "enums": enums_documentation(),
        "tools": tools_documentation(),
        "resources": resources_documentation(),
        "error_codes": error_codes_documentation(),
        "methods": [
            {
                "name": "tools/list",
                "description": "List the available MCP tools and their input schemas."
            },
            {
                "name": "tools/call",
                "description": "Invoke a tool by name with structured arguments."
            },
            {
                "name": "resources/list",
                "description": "List the available MCP resources."
            },
            {
                "name": "docs/describe",
                "description": "Return this self-documentation payload (no params).",
                "also_available_as": format!("resource {DOCS_RESOURCE_URI}")
            }
        ]
    })
}

fn enums_documentation() -> Value {
    json!({
        "StationStatus": {
            "description": "Operational state of a Velib station.",
            "values": [
                {"value": "OPEN",        "definition": "Station is operational and renting/returning bikes."},
                {"value": "CLOSED",      "definition": "Station is administratively closed."},
                {"value": "MAINTENANCE", "definition": "Station is temporarily out of service for maintenance."}
            ]
        },
        "DataFreshness": {
            "description": "Categorical age of the most recent real-time update for a station. \
                Boundaries match `DataFreshness::from_age` in `src/types.rs`.",
            "values": [
                {"value": "Fresh",     "definition": "Updated < 10 minutes ago."},
                {"value": "Recent",    "definition": "Updated 10–30 minutes ago."},
                {"value": "Stale",     "definition": "Updated 30–120 minutes ago."},
                {"value": "VeryStale", "definition": "Updated > 120 minutes ago."}
            ]
        },
        "BikeTypeFilter": {
            "description": "Filter applied when checking bike availability.",
            "values": [
                {"value": "mechanical", "definition": "Only mechanical (non-electric) bikes."},
                {"value": "electric",   "definition": "Only electric-assist bikes."},
                {"value": "any",        "definition": "Any bike type (default)."}
            ]
        },
        "DataSource": {
            "description": "Provenance tag attached to a payload.",
            "values": [
                {"value": "paris_open_data", "definition": "Fetched live from the Paris Open Data API."},
                {"value": "cache",           "definition": "Served from the in-process cache."},
                {"value": "fallback",        "definition": "Reserved for future degraded-mode fallbacks."}
            ]
        }
    })
}

fn tools_documentation() -> Value {
    json!([
        {
            "name": "find_nearby_stations",
            "description": "Find Velib stations within a radius of coordinates. \
                Coordinates outside the 50 km Paris service area are rejected.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "latitude":  {"type": "number", "unit": "decimal degrees", "minimum": 48.7, "maximum": 49.0},
                    "longitude": {"type": "number", "unit": "decimal degrees", "minimum": 2.0,  "maximum": 2.6},
                    "radius_meters": {"type": "integer", "unit": "meters", "minimum": 100, "maximum": 5000, "default": 500},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 100, "default": 10},
                    "availability_filter": {
                        "type": "object",
                        "description": "Optional filter on bikes/docks/bike-type.",
                        "properties": {
                            "min_bikes": {"type": "integer", "minimum": 0},
                            "min_docks": {"type": "integer", "minimum": 0},
                            "bike_type": {"$ref": "#/enums/BikeTypeFilter"},
                            "exclude_out_of_service": {"type": "boolean", "default": true}
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
                        "description": "Stations sorted by ascending distance from the query point.",
                        "items": {"$ref": "#/types/StationWithDistance"}
                    },
                    "search_metadata": {"$ref": "#/types/SearchMetadata"}
                }
            }
        },
        {
            "name": "get_station_by_code",
            "description": "Get detailed information about a specific station by its station code.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "station_code": {"type": "string"},
                    "include_real_time": {"type": "boolean", "default": true}
                },
                "required": ["station_code"]
            },
            "output_schema": {
                "type": "object",
                "properties": {
                    "station": {"$ref": "#/types/VelibStation", "nullable": true},
                    "found":   {"type": "boolean"}
                }
            }
        },
        {
            "name": "search_stations_by_name",
            "description": "Search stations by name with optional fuzzy matching.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "minLength": 2},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 100, "default": 10},
                    "fuzzy": {"type": "boolean", "default": true}
                },
                "required": ["query"]
            },
            "output_schema": {
                "type": "object",
                "properties": {
                    "stations": {"type": "array", "items": {"$ref": "#/types/VelibStation"}},
                    "search_metadata": {"$ref": "#/types/TextSearchMetadata"}
                }
            }
        },
        {
            "name": "get_area_statistics",
            "description": "Get aggregated statistics for a geographic bounding box.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "bounds": {
                        "type": "object",
                        "properties": {
                            "north": {"type": "number", "unit": "decimal degrees"},
                            "south": {"type": "number", "unit": "decimal degrees"},
                            "east":  {"type": "number", "unit": "decimal degrees"},
                            "west":  {"type": "number", "unit": "decimal degrees"}
                        },
                        "required": ["north", "south", "east", "west"]
                    },
                    "include_real_time": {"type": "boolean", "default": true}
                },
                "required": ["bounds"]
            },
            "output_schema": {
                "type": "object",
                "properties": {
                    "area_stats": {"$ref": "#/types/AreaStatistics"},
                    "bounds":     {"$ref": "#/types/GeographicBounds"}
                }
            }
        },
        {
            "name": "plan_bike_journey",
            "description": "Plan a bike journey: suggest pickup and drop-off stations near origin/destination.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "origin": {
                        "type": "object",
                        "properties": {
                            "latitude":  {"type": "number", "unit": "decimal degrees"},
                            "longitude": {"type": "number", "unit": "decimal degrees"}
                        },
                        "required": ["latitude", "longitude"]
                    },
                    "destination": {
                        "type": "object",
                        "properties": {
                            "latitude":  {"type": "number", "unit": "decimal degrees"},
                            "longitude": {"type": "number", "unit": "decimal degrees"}
                        },
                        "required": ["latitude", "longitude"]
                    },
                    "preferences": {
                        "type": "object",
                        "properties": {
                            "bike_type": {"$ref": "#/enums/BikeTypeFilter"},
                            "max_walk_distance": {"type": "integer", "unit": "meters", "default": 500}
                        }
                    }
                },
                "required": ["origin", "destination"]
            },
            "output_schema": {
                "type": "object",
                "properties": {
                    "journey": {"$ref": "#/types/BikeJourney"}
                }
            }
        }
    ])
}

fn resources_documentation() -> Value {
    json!([
        {
            "uri": "velib://stations/reference",
            "name": "Velib Station Reference Data",
            "description": "Complete catalog of Velib stations with static metadata (location, capacity, capabilities).",
            "mime_type": "application/json",
            "freshness": "reference cache"
        },
        {
            "uri": "velib://stations/realtime",
            "name": "Velib Real-time Availability",
            "description": "Current bike and dock availability for all stations.",
            "mime_type": "application/json",
            "freshness": "realtime cache"
        },
        {
            "uri": "velib://stations/complete",
            "name": "Velib Complete Station Data",
            "description": "Combined reference and real-time data for all stations.",
            "mime_type": "application/json",
            "freshness": "realtime cache"
        },
        {
            "uri": "velib://health",
            "name": "Service Health Status",
            "description": "Uptime, cache sizes, and data lag derived from the most recent real-time update.",
            "mime_type": "application/json",
            "freshness": "live"
        },
        {
            "uri": DOCS_RESOURCE_URI,
            "name": "Velib MCP API Description",
            "description": "Self-documentation payload (this document).",
            "mime_type": "application/json",
            "freshness": "static"
        }
    ])
}

fn error_codes_documentation() -> Value {
    json!([
        {"code": -32700, "name": "Parse error",      "definition": "Malformed JSON or unparseable upstream payload."},
        {"code": -32600, "name": "Invalid request",  "definition": "Request is well-formed JSON but not a valid request (e.g. station not found)."},
        {"code": -32602, "name": "Invalid params",   "definition": "Coordinates, radius, limit, or other validated parameter is out of range."},
        {"code": -32603, "name": "Internal error",   "definition": "Server-side failure: cache, MCP protocol, or unclassified internal error."},
        {"code": -32001, "name": "Server error",     "definition": "Upstream HTTP failure or rate-limited by the Paris Open Data API."}
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn description_has_required_top_level_keys() {
        let doc = build_api_description();
        for key in [
            "schema_version",
            "server",
            "data_freshness",
            "units",
            "enums",
            "tools",
            "resources",
            "error_codes",
            "methods",
        ] {
            assert!(doc.get(key).is_some(), "missing top-level key: {key}");
        }
    }

    #[test]
    fn cache_ttls_match_runtime_constants() {
        let doc = build_api_description();
        let caches = doc["data_freshness"]["caches"].as_array().unwrap();
        let reference = caches.iter().find(|c| c["name"] == "reference").unwrap();
        let realtime = caches.iter().find(|c| c["name"] == "realtime").unwrap();
        assert_eq!(
            reference["ttl_minutes"].as_i64().unwrap(),
            REFERENCE_CACHE_TTL_MINUTES
        );
        assert_eq!(
            realtime["ttl_minutes"].as_i64().unwrap(),
            REALTIME_CACHE_TTL_MINUTES
        );
    }

    #[test]
    fn all_five_tools_documented() {
        let doc = build_api_description();
        let tools = doc["tools"].as_array().unwrap();
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        for name in [
            "find_nearby_stations",
            "get_station_by_code",
            "search_stations_by_name",
            "get_area_statistics",
            "plan_bike_journey",
        ] {
            assert!(names.contains(&name), "tool missing: {name}");
        }
        assert_eq!(tools.len(), 5);
    }

    #[test]
    fn docs_resource_uri_listed() {
        let doc = build_api_description();
        let resources = doc["resources"].as_array().unwrap();
        assert!(resources.iter().any(|r| r["uri"] == DOCS_RESOURCE_URI));
    }
}
