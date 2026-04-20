//! Tests for MCP types: GeographicBounds, JsonRpc serde, and error conversion.

use velib_mcp::mcp::types::{
    AvailabilityFilter, FindNearbyStationsOutput, GeographicBounds, GetStationByCodeOutput,
    JsonRpcError, JsonRpcRequest, JsonRpcResponse, SearchMetadata,
};
use velib_mcp::types::Coordinates;

// --- GeographicBounds ---

#[test]
fn geographic_bounds_contains_point_inside() {
    let bounds = GeographicBounds {
        north: 48.90,
        south: 48.80,
        east: 2.40,
        west: 2.30,
    };
    let inside = Coordinates::new(48.85, 2.35);
    assert!(bounds.contains(&inside));
}

#[test]
fn geographic_bounds_rejects_point_outside() {
    let bounds = GeographicBounds {
        north: 48.90,
        south: 48.80,
        east: 2.40,
        west: 2.30,
    };
    // Too far north
    assert!(!bounds.contains(&Coordinates::new(49.0, 2.35)));
    // Too far south
    assert!(!bounds.contains(&Coordinates::new(48.70, 2.35)));
    // Too far east
    assert!(!bounds.contains(&Coordinates::new(48.85, 2.50)));
    // Too far west
    assert!(!bounds.contains(&Coordinates::new(48.85, 2.20)));
}

#[test]
fn geographic_bounds_contains_point_on_boundary() {
    let bounds = GeographicBounds {
        north: 48.90,
        south: 48.80,
        east: 2.40,
        west: 2.30,
    };
    // Points exactly on each boundary should be included (<=, >=)
    assert!(bounds.contains(&Coordinates::new(48.90, 2.35))); // north edge
    assert!(bounds.contains(&Coordinates::new(48.80, 2.35))); // south edge
    assert!(bounds.contains(&Coordinates::new(48.85, 2.40))); // east edge
    assert!(bounds.contains(&Coordinates::new(48.85, 2.30))); // west edge
                                                              // Corner
    assert!(bounds.contains(&Coordinates::new(48.90, 2.40)));
}

/// An inverted (south > north) bounding box should contain no points, not panic.
#[test]
fn geographic_bounds_inverted_north_south_contains_nothing() {
    let inverted = GeographicBounds {
        north: 48.80, // south > north — logically empty
        south: 48.90,
        east: 2.40,
        west: 2.30,
    };
    // No point can satisfy both lat >= south (48.90) AND lat <= north (48.80)
    assert!(!inverted.contains(&Coordinates::new(48.85, 2.35)));
    assert!(!inverted.contains(&Coordinates::new(48.90, 2.35)));
    assert!(!inverted.contains(&Coordinates::new(48.80, 2.35)));
}

/// An inverted (east < west) bounding box should contain no points, not panic.
#[test]
fn geographic_bounds_inverted_east_west_contains_nothing() {
    let inverted = GeographicBounds {
        north: 48.90,
        south: 48.80,
        east: 2.30,  // east < west — logically empty
        west: 2.40,
    };
    assert!(!inverted.contains(&Coordinates::new(48.85, 2.35)));
    assert!(!inverted.contains(&Coordinates::new(48.85, 2.30)));
    assert!(!inverted.contains(&Coordinates::new(48.85, 2.40)));
}

// --- AvailabilityFilter defaults ---

#[test]
fn availability_filter_default_excludes_out_of_service() {
    let filter = AvailabilityFilter::default();
    assert!(
        filter.exclude_out_of_service,
        "exclude_out_of_service should default to true"
    );
    assert!(filter.bike_type.is_none(), "bike_type should default to None");
    assert!(filter.min_bikes.is_none(), "min_bikes should default to None");
    assert!(filter.min_docks.is_none(), "min_docks should default to None");
}

#[test]
fn availability_filter_round_trips_through_json() {
    let original = AvailabilityFilter {
        min_bikes: Some(3),
        min_docks: Some(2),
        bike_type: None,
        exclude_out_of_service: true,
    };
    let json = serde_json::to_value(&original).unwrap();
    let round_tripped: AvailabilityFilter = serde_json::from_value(json).unwrap();
    assert_eq!(round_tripped.min_bikes, Some(3));
    assert_eq!(round_tripped.min_docks, Some(2));
    assert!(round_tripped.exclude_out_of_service);
}

// --- JsonRpc serde round-trips ---

#[test]
fn jsonrpc_request_deserializes_with_defaults() {
    let json = r#"{"id": 1, "method": "tools/list", "params": {}}"#;
    let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.jsonrpc, "2.0"); // default
    assert_eq!(req.method, "tools/list");
    assert_eq!(req.id, serde_json::json!(1));
}

#[test]
fn jsonrpc_request_accepts_string_id() {
    let json = r#"{"jsonrpc": "2.0", "id": "abc", "method": "tools/list", "params": {}}"#;
    let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.id, serde_json::json!("abc"));
}

#[test]
fn jsonrpc_request_accepts_null_id() {
    let json = r#"{"jsonrpc": "2.0", "id": null, "method": "tools/list", "params": {}}"#;
    let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
    assert!(req.id.is_null());
}

#[test]
fn jsonrpc_response_round_trips_with_result() {
    let response = JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: serde_json::json!(42),
        result: Some(serde_json::json!({"tools": []})),
        error: None,
    };

    let serialized = serde_json::to_string(&response).unwrap();
    let deserialized: JsonRpcResponse = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized.jsonrpc, "2.0");
    assert_eq!(deserialized.id, serde_json::json!(42));
    assert!(deserialized.result.is_some());
    assert!(deserialized.error.is_none());
}

#[test]
fn jsonrpc_response_round_trips_with_error() {
    let response = JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: serde_json::json!(1),
        result: None,
        error: Some(JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        }),
    };

    let serialized = serde_json::to_string(&response).unwrap();
    let deserialized: JsonRpcResponse = serde_json::from_str(&serialized).unwrap();

    assert!(deserialized.result.is_none());
    assert!(deserialized.error.is_some());
    assert_eq!(deserialized.error.unwrap().code, -32602);
}

// --- Error conversion ---

#[test]
fn jsonrpc_error_from_all_error_variants_has_correct_structure() {
    let errors: Vec<velib_mcp::Error> = vec![
        velib_mcp::Error::InvalidCoordinates {
            latitude: 0.0,
            longitude: 0.0,
        },
        velib_mcp::Error::StationNotFound {
            station_code: "X".to_string(),
        },
        velib_mcp::Error::McpProtocol("unknown method".to_string()),
    ];

    for error in errors {
        let rpc_err = JsonRpcError::from(error);
        // All converted errors should have data with error_type
        assert!(rpc_err.data.is_some(), "Error data should be present");
        let data = rpc_err.data.unwrap();
        assert!(
            data["error_type"].is_string(),
            "error_type should be a string"
        );
        // Message should not be empty
        assert!(!rpc_err.message.is_empty());
    }
}

// --- Output type serde ---

#[test]
fn find_nearby_stations_output_serializes() {
    let output = FindNearbyStationsOutput {
        stations: vec![],
        search_metadata: SearchMetadata {
            query_point: Coordinates::new(48.8566, 2.3522),
            radius_meters: 500,
            total_found: 0,
            search_time_ms: 42,
        },
    };

    let json = serde_json::to_value(&output).unwrap();
    assert_eq!(json["search_metadata"]["total_found"], 0);
    assert_eq!(json["search_metadata"]["search_time_ms"], 42);
    assert!(json["stations"].as_array().unwrap().is_empty());
}

#[test]
fn get_station_by_code_output_not_found_serializes() {
    let output = GetStationByCodeOutput {
        station: None,
        found: false,
    };
    let json = serde_json::to_value(&output).unwrap();
    assert_eq!(json["found"], false);
    assert!(json["station"].is_null());
}
