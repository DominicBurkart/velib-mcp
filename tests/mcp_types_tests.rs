use velib_mcp::mcp::types::{
    FindNearbyStationsInput, GeographicBounds, GetStationByCodeInput,
    JsonRpcError, JsonRpcRequest, SearchStationsByNameInput,
};
use velib_mcp::Coordinates;

#[test]
fn geographic_bounds_contains_point_inside() {
    let bounds = GeographicBounds {
        north: 49.0,
        south: 48.7,
        east: 2.6,
        west: 2.0,
    };
    let inside = Coordinates::new(48.85, 2.35);
    assert!(bounds.contains(&inside));
}

#[test]
fn geographic_bounds_rejects_point_outside() {
    let bounds = GeographicBounds {
        north: 49.0,
        south: 48.7,
        east: 2.6,
        west: 2.0,
    };
    let outside = Coordinates::new(50.0, 3.0);
    assert!(!bounds.contains(&outside));
}

#[test]
fn geographic_bounds_edge_inclusive() {
    let bounds = GeographicBounds {
        north: 49.0,
        south: 48.7,
        east: 2.6,
        west: 2.0,
    };
    // Exactly on the boundary should be included
    let on_edge = Coordinates::new(48.7, 2.0);
    assert!(bounds.contains(&on_edge));

    let on_opposite_edge = Coordinates::new(49.0, 2.6);
    assert!(bounds.contains(&on_opposite_edge));
}

#[test]
fn find_nearby_stations_input_deserializes_defaults() {
    let json = serde_json::json!({
        "latitude": 48.8566,
        "longitude": 2.3522
    });
    let input: FindNearbyStationsInput = serde_json::from_value(json).unwrap();
    assert_eq!(input.radius_meters, 500); // default
    assert_eq!(input.limit, 10); // default
    assert!(input.availability_filter.is_none());
}

#[test]
fn search_stations_input_deserializes_defaults() {
    let json = serde_json::json!({
        "query": "chatelet"
    });
    let input: SearchStationsByNameInput = serde_json::from_value(json).unwrap();
    assert_eq!(input.limit, 10);
    assert!(input.fuzzy); // default true
}

#[test]
fn get_station_by_code_input_deserializes_defaults() {
    let json = serde_json::json!({
        "station_code": "16107"
    });
    let input: GetStationByCodeInput = serde_json::from_value(json).unwrap();
    assert!(input.include_real_time); // default true
}

#[test]
fn jsonrpc_request_round_trip() {
    let original = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 42,
        "method": "tools/list",
        "params": {}
    });
    let req: JsonRpcRequest = serde_json::from_value(original.clone()).unwrap();
    assert_eq!(req.jsonrpc, "2.0");
    assert_eq!(req.method, "tools/list");
    assert_eq!(req.id, 42);
}

#[test]
fn jsonrpc_error_from_all_param_error_variants() {
    let err = velib_mcp::Error::InvalidCoordinates {
        latitude: 0.0,
        longitude: 0.0,
    };
    let rpc_err = JsonRpcError::from(err);
    assert_eq!(rpc_err.code, -32602);
    assert!(rpc_err.message.contains("Invalid coordinates"));
    let data = rpc_err.data.unwrap();
    assert_eq!(data["error_type"], "invalid_coordinates");
}
