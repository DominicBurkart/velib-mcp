use serde_json::{json, Value};

/// Names of all MCP tools provided by this server.
/// Used to verify documentation completeness in tests.
pub const TOOL_NAMES: &[&str] = &[
    "find_nearby_stations",
    "get_station_by_code",
    "search_stations_by_name",
    "get_area_statistics",
    "plan_bike_journey",
    "get_api_documentation",
];

/// Names of all MCP resources provided by this server.
pub const RESOURCE_URIS: &[&str] = &[
    "velib://stations/reference",
    "velib://stations/realtime",
    "velib://stations/complete",
    "velib://health",
    "velib://documentation",
];

/// Generate comprehensive API documentation as structured JSON.
///
/// This is auto-generated from the tool/resource definitions
/// so it stays in sync with the actual implementation.
#[must_use]
pub fn generate_documentation() -> Value {
    json!({
        "server": {
            "name": "velib-mcp",
            "version": "1.0.0",
            "description": "MCP server providing Velib Paris bike-sharing data to AI assistants",
            "protocol": "JSON-RPC 2.0 over HTTP POST (/mcp) and WebSocket (/mcp/ws)",
            "data_sources": [
                {
                    "name": "Velib Station Reference",
                    "url": "https://opendata.paris.fr/explore/dataset/velib-emplacement-des-stations/",
                    "description": "Static station metadata (location, capacity, name)"
                },
                {
                    "name": "Velib Real-time Availability",
                    "url": "https://opendata.paris.fr/explore/dataset/velib-disponibilite-en-temps-reel/",
                    "description": "Live bike and dock availability per station"
                }
            ]
        },
        "caching": {
            "reference_data_ttl_seconds": 300,
            "reference_data_ttl_description": "Station reference data (names, locations, capacity) is cached for 5 minutes",
            "realtime_data_ttl_seconds": 120,
            "realtime_data_ttl_description": "Real-time availability data (bikes, docks, status) is cached for 2 minutes"
        },
        "service_area": {
            "description": "All coordinate-based queries must be within the Paris metropolitan service area",
            "center": { "latitude": 48.8565, "longitude": 2.3514, "label": "Paris City Hall (Hotel de Ville)" },
            "max_radius_km": 50,
            "coordinate_bounds": {
                "latitude": { "min": 48.7, "max": 49.0 },
                "longitude": { "min": 2.0, "max": 2.6 }
            }
        },
        "enums": generate_enum_definitions(),
        "tools": generate_tool_docs(),
        "resources": generate_resource_docs(),
        "data_types": generate_data_type_docs(),
        "units": generate_units_docs()
    })
}

fn generate_enum_definitions() -> Value {
    json!({
        "StationStatus": {
            "description": "Operational status of a Velib station",
            "values": {
                "OPEN": "Station is fully operational: installed, accepting rentals, and accepting returns",
                "CLOSED": "Station is not installed or not operational",
                "MAINTENANCE": "Station is installed but either not renting or not accepting returns"
            }
        },
        "BikeTypeFilter": {
            "description": "Filter for selecting bike types in queries",
            "values": {
                "mechanical": "Only mechanical (non-electric) bikes",
                "electric": "Only electric bikes (e-bikes)",
                "any": "Any available bike type (default)"
            }
        },
        "DataFreshness": {
            "description": "Indicates how recent the real-time data is, computed from the station's last_update timestamp",
            "values": {
                "Fresh": "Data is less than 10 minutes old",
                "Recent": "Data is 10-30 minutes old",
                "Stale": "Data is 30-120 minutes old",
                "VeryStale": "Data is more than 120 minutes old"
            }
        },
        "DataSource": {
            "description": "Origin of the data in a response",
            "values": {
                "paris_open_data": "Fetched live from Paris Open Data API",
                "cache": "Served from in-memory cache",
                "fallback": "Fallback/default data used when API is unavailable"
            }
        }
    })
}

fn generate_tool_docs() -> Value {
    json!([
        {
            "name": "find_nearby_stations",
            "description": "Find Velib stations within a radius of coordinates. Returns stations sorted by distance, filtered by operational status and optional bike type.",
            "parameters": {
                "latitude": {
                    "type": "number",
                    "required": true,
                    "description": "Latitude of the search center point",
                    "unit": "degrees",
                    "constraints": { "min": 48.7, "max": 49.0 }
                },
                "longitude": {
                    "type": "number",
                    "required": true,
                    "description": "Longitude of the search center point",
                    "unit": "degrees",
                    "constraints": { "min": 2.0, "max": 2.6 }
                },
                "radius_meters": {
                    "type": "integer",
                    "required": false,
                    "default": 500,
                    "description": "Search radius around the center point",
                    "unit": "meters",
                    "constraints": { "min": 1, "max": 5000 }
                },
                "limit": {
                    "type": "integer",
                    "required": false,
                    "default": 10,
                    "description": "Maximum number of stations to return",
                    "constraints": { "min": 1, "max": 100 }
                },
                "availability_filter": {
                    "type": "object",
                    "required": false,
                    "description": "Optional filter for bike/dock availability",
                    "properties": {
                        "min_bikes": { "type": "integer", "description": "Minimum number of available bikes" },
                        "min_docks": { "type": "integer", "description": "Minimum number of available docks" },
                        "bike_type": { "type": "BikeTypeFilter", "description": "Type of bike to filter for" },
                        "exclude_out_of_service": { "type": "boolean", "default": true, "description": "Exclude stations not in service" }
                    }
                }
            },
            "returns": {
                "search_metadata": {
                    "query_point": "Coordinates echoed back",
                    "radius_meters": "Search radius used (meters)",
                    "total_found": "Number of matching stations",
                    "search_time_ms": "Server-side processing time (milliseconds)"
                },
                "stations": "Array of VelibStation objects with straight_line_distance_meters (meters) added"
            }
        },
        {
            "name": "get_station_by_code",
            "description": "Get detailed information about a specific station by its unique station code.",
            "parameters": {
                "station_code": {
                    "type": "string",
                    "required": true,
                    "description": "Unique identifier for the Velib station (e.g. '16107')"
                },
                "include_real_time": {
                    "type": "boolean",
                    "required": false,
                    "default": true,
                    "description": "Whether to include real-time availability data"
                }
            },
            "returns": {
                "found": "Boolean indicating if the station was found",
                "station": "VelibStation object or null if not found"
            }
        },
        {
            "name": "search_stations_by_name",
            "description": "Search stations by name with optional fuzzy matching. Uses Unicode-normalized case-insensitive comparison.",
            "parameters": {
                "query": {
                    "type": "string",
                    "required": true,
                    "description": "Search query string (minimum 2 characters)",
                    "constraints": { "minLength": 2 }
                },
                "limit": {
                    "type": "integer",
                    "required": false,
                    "default": 10,
                    "description": "Maximum number of stations to return",
                    "constraints": { "min": 1, "max": 100 }
                },
                "fuzzy": {
                    "type": "boolean",
                    "required": false,
                    "default": true,
                    "description": "If true, matches any substring; if false, matches only from the start of the name"
                }
            },
            "returns": {
                "search_metadata": {
                    "query": "Search string echoed back",
                    "total_found": "Number of matching stations",
                    "fuzzy_enabled": "Whether fuzzy matching was used",
                    "search_time_ms": "Server-side processing time (milliseconds)"
                },
                "stations": "Array of matching VelibStation objects sorted by name"
            }
        },
        {
            "name": "get_area_statistics",
            "description": "Get aggregated statistics for a geographic bounding box, including station counts, bike/dock totals, and occupancy rate.",
            "parameters": {
                "bounds": {
                    "type": "object",
                    "required": true,
                    "description": "Geographic bounding box defining the area",
                    "properties": {
                        "north": { "type": "number", "unit": "degrees latitude", "description": "Northern boundary" },
                        "south": { "type": "number", "unit": "degrees latitude", "description": "Southern boundary" },
                        "east": { "type": "number", "unit": "degrees longitude", "description": "Eastern boundary" },
                        "west": { "type": "number", "unit": "degrees longitude", "description": "Western boundary" }
                    }
                },
                "include_real_time": {
                    "type": "boolean",
                    "required": false,
                    "default": true,
                    "description": "Whether to include real-time availability data in statistics"
                }
            },
            "returns": {
                "bounds": "Geographic bounds echoed back",
                "area_stats": {
                    "total_stations": "Total stations in the area",
                    "operational_stations": "Stations with OPEN status",
                    "total_capacity": "Sum of all station capacities (dock count)",
                    "available_bikes": {
                        "mechanical": "Total mechanical bikes available",
                        "electric": "Total electric bikes available",
                        "total": "Total bikes available (mechanical + electric)"
                    },
                    "available_docks": "Total empty docks available",
                    "occupancy_rate": "Ratio of total bikes to total capacity (0.0 to 1.0)"
                }
            }
        },
        {
            "name": "plan_bike_journey",
            "description": "Plan a bike journey with pickup and dropoff station suggestions. Finds the closest operational stations near origin and destination.",
            "parameters": {
                "origin": {
                    "type": "object",
                    "required": true,
                    "description": "Starting point coordinates",
                    "properties": {
                        "latitude": { "type": "number", "unit": "degrees" },
                        "longitude": { "type": "number", "unit": "degrees" }
                    }
                },
                "destination": {
                    "type": "object",
                    "required": true,
                    "description": "Ending point coordinates",
                    "properties": {
                        "latitude": { "type": "number", "unit": "degrees" },
                        "longitude": { "type": "number", "unit": "degrees" }
                    }
                },
                "preferences": {
                    "type": "object",
                    "required": false,
                    "description": "Journey preferences",
                    "properties": {
                        "bike_type": {
                            "type": "BikeTypeFilter",
                            "default": "any",
                            "description": "Preferred bike type"
                        },
                        "max_walk_distance": {
                            "type": "integer",
                            "default": 500,
                            "unit": "meters",
                            "description": "Maximum walking distance to/from a station"
                        }
                    }
                }
            },
            "returns": {
                "journey": {
                    "pickup_stations": "Up to 3 nearest stations with available bikes near origin, with straight_line_distance_meters (meters)",
                    "dropoff_stations": "Up to 3 nearest stations with available docks near destination, with straight_line_distance_meters (meters)",
                    "recommendations": "Ranked journey options pairing best pickup/dropoff, with confidence_score (0.0 to 1.0) and walking distances (meters)"
                }
            }
        },
        {
            "name": "get_api_documentation",
            "description": "Returns this comprehensive API documentation as structured JSON. No parameters required.",
            "parameters": {},
            "returns": {
                "description": "Full API schema including all tools, resources, enums, data types, units, and caching behavior"
            }
        }
    ])
}

fn generate_resource_docs() -> Value {
    json!([
        {
            "uri": "velib://stations/reference",
            "name": "Velib Station Reference Data",
            "description": "Complete catalog of all Velib stations with static metadata (name, location, capacity, capabilities)",
            "mimeType": "application/json",
            "cache_ttl_seconds": 300,
            "response_schema": {
                "stations": "Array of StationReference objects",
                "metadata": {
                    "total_stations": "integer",
                    "last_updated": "ISO 8601 timestamp",
                    "data_source": "string"
                }
            }
        },
        {
            "uri": "velib://stations/realtime",
            "name": "Velib Real-time Availability",
            "description": "Current bike and dock availability for all stations",
            "mimeType": "application/json",
            "cache_ttl_seconds": 120,
            "response_schema": {
                "stations": "Array of objects with station_code, bikes {mechanical, electric}, available_docks, status, last_update, data_freshness",
                "metadata": {
                    "total_stations": "integer",
                    "data_freshness": "DataFreshness enum",
                    "response_time": "ISO 8601 timestamp",
                    "data_source": "string"
                }
            }
        },
        {
            "uri": "velib://stations/complete",
            "name": "Velib Complete Station Data",
            "description": "Combined reference and real-time data for all stations",
            "mimeType": "application/json",
            "cache_ttl_seconds": 120,
            "response_schema": {
                "stations": "Array of VelibStation objects (reference + optional real_time)",
                "metadata": {
                    "total_stations": "integer",
                    "data_freshness": "DataFreshness enum",
                    "response_time": "ISO 8601 timestamp",
                    "data_source": "string"
                }
            }
        },
        {
            "uri": "velib://health",
            "name": "Service Health Status",
            "description": "System health, uptime, data source statuses, and cache statistics",
            "mimeType": "application/json",
            "response_schema": {
                "status": "string (healthy/degraded)",
                "version": "string",
                "uptime_seconds": "integer",
                "data_sources": {
                    "real_time": { "status": "string", "last_update": "ISO 8601 timestamp", "lag_seconds": "integer" },
                    "reference": { "status": "string", "last_update": "ISO 8601 timestamp" }
                },
                "cache_stats": {
                    "hit_rate": "float (0.0-1.0)",
                    "entries": "integer",
                    "reference_cache_size": "integer",
                    "realtime_cache_size": "integer"
                }
            }
        },
        {
            "uri": "velib://documentation",
            "name": "API Documentation",
            "description": "This self-documentation endpoint. Returns the full API schema.",
            "mimeType": "application/json"
        }
    ])
}

fn generate_data_type_docs() -> Value {
    json!({
        "Coordinates": {
            "description": "Geographic coordinates (WGS84)",
            "fields": {
                "latitude": { "type": "number", "unit": "degrees (WGS84)" },
                "longitude": { "type": "number", "unit": "degrees (WGS84)" }
            }
        },
        "StationReference": {
            "description": "Static metadata for a Velib station",
            "fields": {
                "station_code": { "type": "string", "description": "Unique station identifier" },
                "name": { "type": "string", "description": "Human-readable station name" },
                "coordinates": { "type": "Coordinates", "description": "Station location" },
                "capacity": { "type": "integer", "unit": "docks", "description": "Total number of docking points" },
                "capabilities": {
                    "type": "ServiceCapabilities",
                    "description": "Station service features",
                    "fields": {
                        "accepts_credit_card": { "type": "boolean" },
                        "has_charging_station": { "type": "boolean" },
                        "is_virtual_station": { "type": "boolean" }
                    }
                }
            }
        },
        "RealTimeStatus": {
            "description": "Live availability data for a station",
            "fields": {
                "bikes": {
                    "type": "BikeAvailability",
                    "fields": {
                        "mechanical": { "type": "integer", "unit": "bikes", "description": "Available mechanical bikes" },
                        "electric": { "type": "integer", "unit": "bikes", "description": "Available electric bikes" }
                    }
                },
                "available_docks": { "type": "integer", "unit": "docks", "description": "Empty docking points" },
                "status": { "type": "StationStatus", "description": "Operational status" },
                "last_update": { "type": "string", "format": "ISO 8601 datetime", "description": "When the station last reported data" },
                "data_freshness": { "type": "DataFreshness", "description": "Computed freshness category based on last_update age" }
            }
        },
        "VelibStation": {
            "description": "Complete station object combining reference and optional real-time data",
            "fields": {
                "reference": { "type": "StationReference", "description": "Static station metadata" },
                "real_time": { "type": "RealTimeStatus | null", "description": "Live availability (null if not requested or unavailable)" }
            }
        },
        "StationWithDistance": {
            "description": "A VelibStation with distance information (flattened — station fields appear at top level)",
            "fields": {
                "...VelibStation": "All VelibStation fields are included via flatten",
                "straight_line_distance_meters": { "type": "integer", "unit": "meters", "description": "Straight-line (Haversine) distance from query point" }
            }
        }
    })
}

fn generate_units_docs() -> Value {
    json!({
        "distance": {
            "straight_line_distance_meters": "meters (Haversine formula, straight-line)",
            "radius_meters": "meters",
            "max_walk_distance": "meters",
            "straight_line_to_pickup_meters": "meters",
            "straight_line_from_dropoff_meters": "meters"
        },
        "time": {
            "search_time_ms": "milliseconds (server-side processing)",
            "uptime_seconds": "seconds (server uptime since start)",
            "lag_seconds": "seconds (delay between most recent station update and current time)",
            "cache_ttl": "seconds (time-to-live for cached data)"
        },
        "counts": {
            "capacity": "docks (total docking points at a station)",
            "mechanical": "bikes (available mechanical bikes)",
            "electric": "bikes (available electric bikes)",
            "available_docks": "docks (empty docking points)"
        },
        "ratios": {
            "occupancy_rate": "0.0 to 1.0 (total bikes / total capacity)",
            "confidence_score": "0.0 to 1.0 (journey recommendation quality)",
            "hit_rate": "0.0 to 1.0 (cache hit ratio)"
        },
        "coordinates": {
            "latitude": "degrees (WGS84, positive = North)",
            "longitude": "degrees (WGS84, positive = East)"
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documentation_is_valid_json() {
        let doc = generate_documentation();
        // Roundtrip through string to ensure it serializes cleanly
        let serialized = serde_json::to_string_pretty(&doc).unwrap();
        let _parsed: Value = serde_json::from_str(&serialized).unwrap();
    }

    #[test]
    fn documentation_contains_all_top_level_sections() {
        let doc = generate_documentation();
        assert!(doc["server"].is_object(), "Missing 'server' section");
        assert!(doc["caching"].is_object(), "Missing 'caching' section");
        assert!(
            doc["service_area"].is_object(),
            "Missing 'service_area' section"
        );
        assert!(doc["enums"].is_object(), "Missing 'enums' section");
        assert!(doc["tools"].is_array(), "Missing 'tools' section");
        assert!(doc["resources"].is_array(), "Missing 'resources' section");
        assert!(
            doc["data_types"].is_object(),
            "Missing 'data_types' section"
        );
        assert!(doc["units"].is_object(), "Missing 'units' section");
    }

    #[test]
    fn all_tools_are_documented() {
        let doc = generate_documentation();
        let tools = doc["tools"].as_array().unwrap();
        let documented_names: Vec<&str> =
            tools.iter().map(|t| t["name"].as_str().unwrap()).collect();

        for tool_name in TOOL_NAMES {
            assert!(
                documented_names.contains(tool_name),
                "Tool '{}' is not documented",
                tool_name
            );
        }

        // Also verify no extra undocumented tools
        assert_eq!(
            tools.len(),
            TOOL_NAMES.len(),
            "Documentation has {} tools but TOOL_NAMES has {}",
            tools.len(),
            TOOL_NAMES.len()
        );
    }

    #[test]
    fn all_resources_are_documented() {
        let doc = generate_documentation();
        let resources = doc["resources"].as_array().unwrap();
        let documented_uris: Vec<&str> = resources
            .iter()
            .map(|r| r["uri"].as_str().unwrap())
            .collect();

        for uri in RESOURCE_URIS {
            assert!(
                documented_uris.contains(uri),
                "Resource '{}' is not documented",
                uri
            );
        }

        assert_eq!(
            resources.len(),
            RESOURCE_URIS.len(),
            "Documentation has {} resources but RESOURCE_URIS has {}",
            resources.len(),
            RESOURCE_URIS.len()
        );
    }

    #[test]
    fn all_enums_are_documented_with_values() {
        let doc = generate_documentation();
        let enums = &doc["enums"];

        // Verify all expected enums are present
        let expected_enums = [
            "StationStatus",
            "BikeTypeFilter",
            "DataFreshness",
            "DataSource",
        ];
        for enum_name in &expected_enums {
            assert!(
                enums[enum_name].is_object(),
                "Enum '{}' is not documented",
                enum_name
            );
            assert!(
                enums[enum_name]["values"].is_object(),
                "Enum '{}' has no values",
                enum_name
            );
        }

        // Verify StationStatus has all 3 values
        let station_status_values = enums["StationStatus"]["values"].as_object().unwrap();
        assert!(station_status_values.contains_key("OPEN"));
        assert!(station_status_values.contains_key("CLOSED"));
        assert!(station_status_values.contains_key("MAINTENANCE"));

        // Verify BikeTypeFilter has all 3 values
        let bike_type_values = enums["BikeTypeFilter"]["values"].as_object().unwrap();
        assert!(bike_type_values.contains_key("mechanical"));
        assert!(bike_type_values.contains_key("electric"));
        assert!(bike_type_values.contains_key("any"));

        // Verify DataFreshness has all 4 values
        let freshness_values = enums["DataFreshness"]["values"].as_object().unwrap();
        assert!(freshness_values.contains_key("Fresh"));
        assert!(freshness_values.contains_key("Recent"));
        assert!(freshness_values.contains_key("Stale"));
        assert!(freshness_values.contains_key("VeryStale"));

        // Verify DataSource has all 3 values
        let source_values = enums["DataSource"]["values"].as_object().unwrap();
        assert!(source_values.contains_key("paris_open_data"));
        assert!(source_values.contains_key("cache"));
        assert!(source_values.contains_key("fallback"));
    }

    #[test]
    fn tool_docs_have_required_fields() {
        let doc = generate_documentation();
        let tools = doc["tools"].as_array().unwrap();

        for tool in tools {
            let name = tool["name"].as_str().unwrap();
            assert!(
                tool["description"].is_string(),
                "Tool '{}' missing description",
                name
            );
            assert!(
                tool["parameters"].is_object(),
                "Tool '{}' missing parameters",
                name
            );
            assert!(
                tool["returns"].is_object(),
                "Tool '{}' missing returns",
                name
            );
        }
    }

    #[test]
    fn caching_info_matches_source_constants() {
        let doc = generate_documentation();
        // Reference TTL should be 300 seconds (5 minutes, matching REFERENCE_CACHE_TTL_MINUTES)
        assert_eq!(doc["caching"]["reference_data_ttl_seconds"], 300);
        // Real-time TTL should be 120 seconds (2 minutes, matching REALTIME_CACHE_TTL_MINUTES)
        assert_eq!(doc["caching"]["realtime_data_ttl_seconds"], 120);
    }

    #[test]
    fn units_section_documents_all_numerical_fields() {
        let doc = generate_documentation();
        let units = &doc["units"];

        assert!(units["distance"].is_object());
        assert!(units["time"].is_object());
        assert!(units["counts"].is_object());
        assert!(units["ratios"].is_object());
        assert!(units["coordinates"].is_object());

        // Spot-check specific fields
        assert!(units["distance"]["radius_meters"].is_string());
        assert!(units["time"]["search_time_ms"].is_string());
        assert!(units["counts"]["capacity"].is_string());
        assert!(units["ratios"]["occupancy_rate"].is_string());
    }
}
