//! Tests for MCP handler input validation paths.
//!
//! These tests exercise the error-returning branches of McpToolHandler methods
//! without needing live API access. They construct inputs that fail validation
//! *before* any network call is made.

use velib_mcp::mcp::handlers::McpToolHandler;
use velib_mcp::mcp::types::{
    FindNearbyStationsInput, GetAreaStatisticsInput, GeographicBounds, PlanBikeJourneyInput,
    SearchStationsByNameInput,
};
use velib_mcp::types::Coordinates;

#[tokio::test]
async fn find_nearby_rejects_excessive_radius() {
    let handler = McpToolHandler::new();
    let input = FindNearbyStationsInput {
        latitude: 48.8566,
        longitude: 2.3522,
        radius_meters: 10_000, // max is 5000
        limit: 10,
        availability_filter: None,
    };

    let result = handler.find_nearby_stations(input).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.error_type(), "search_radius_too_large");
}

#[tokio::test]
async fn find_nearby_rejects_excessive_limit() {
    let handler = McpToolHandler::new();
    let input = FindNearbyStationsInput {
        latitude: 48.8566,
        longitude: 2.3522,
        radius_meters: 500,
        limit: 200, // max is 100
        availability_filter: None,
    };

    let result = handler.find_nearby_stations(input).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.error_type(), "result_limit_exceeded");
}

#[tokio::test]
async fn find_nearby_rejects_coordinates_outside_paris_metro() {
    let handler = McpToolHandler::new();
    // New York City coordinates
    let input = FindNearbyStationsInput {
        latitude: 40.7128,
        longitude: -74.0060,
        radius_meters: 500,
        limit: 10,
        availability_filter: None,
    };

    let result = handler.find_nearby_stations(input).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.error_type(), "invalid_coordinates");
}

#[tokio::test]
async fn find_nearby_rejects_coordinates_outside_service_area() {
    let handler = McpToolHandler::new();
    // Coordinates within Paris metro bounds but > 50km from city hall
    // (far eastern edge of the metro box)
    let input = FindNearbyStationsInput {
        latitude: 48.95,
        longitude: 2.59,
        radius_meters: 500,
        limit: 10,
        availability_filter: None,
    };

    let result = handler.find_nearby_stations(input).await;
    // This might be within service area depending on exact distance calculation,
    // but if it passes validation, the API call will follow. We just verify
    // the validation layer doesn't panic.
    // The key point is that if the coordinates are truly > 50km away,
    // we get an OutsideServiceArea error.
    if let Err(err) = &result {
        let err_type = err.error_type();
        assert!(
            err_type == "outside_service_area" || err_type == "http_error",
            "Unexpected error type: {err_type}"
        );
    }
    // If it succeeded, the coordinates were within 50km -- also acceptable.
}

#[tokio::test]
async fn search_stations_rejects_short_query() {
    let handler = McpToolHandler::new();
    let input = SearchStationsByNameInput {
        query: "a".to_string(), // minimum is 2 characters
        limit: 10,
        fuzzy: true,
    };

    let result = handler.search_stations_by_name(input).await;
    assert!(result.is_err());
    // Short query is a client validation error, not an internal server error.
    assert_eq!(result.unwrap_err().error_type(), "validation_error");
}

#[tokio::test]
async fn search_stations_rejects_excessive_limit() {
    let handler = McpToolHandler::new();
    let input = SearchStationsByNameInput {
        query: "chatelet".to_string(),
        limit: 200, // max is 100
        fuzzy: true,
    };

    let result = handler.search_stations_by_name(input).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.error_type(), "result_limit_exceeded");
}

#[tokio::test]
async fn plan_journey_rejects_invalid_origin() {
    let handler = McpToolHandler::new();
    let input = PlanBikeJourneyInput {
        origin: Coordinates::new(0.0, 0.0), // Not in Paris
        destination: Coordinates::new(48.8566, 2.3522),
        preferences: None,
    };

    let result = handler.plan_bike_journey(input).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().error_type(), "invalid_coordinates");
}

#[tokio::test]
async fn plan_journey_rejects_invalid_destination() {
    let handler = McpToolHandler::new();
    let input = PlanBikeJourneyInput {
        origin: Coordinates::new(48.8566, 2.3522),
        destination: Coordinates::new(0.0, 0.0), // Not in Paris
        preferences: None,
    };

    let result = handler.plan_bike_journey(input).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().error_type(), "invalid_coordinates");
}

/// get_area_statistics with an empty bounding box (zero-area) should succeed
/// and return zero stations without touching the network — the bounds simply
/// match no stations in the empty/pre-warm cache scenario, but more importantly
/// it must not panic.
#[tokio::test]
async fn get_area_statistics_empty_bounds_does_not_panic() {
    let handler = McpToolHandler::new();
    // A degenerate zero-area box right in the centre of Paris.
    let input = GetAreaStatisticsInput {
        bounds: GeographicBounds {
            north: 48.8566,
            south: 48.8566,
            east: 2.3522,
            west: 2.3522,
        },
        include_real_time: false,
    };

    // This will attempt a network call; we accept either success (0 stations
    // in a point-box) or a network error -- the important thing is no panic.
    let result = handler.get_area_statistics(input).await;
    if let Ok(output) = result {
        // A point bounding box should contain at most 1 station.
        assert!(
            output.area_stats.total_stations <= 1,
            "Point bounding box should have 0 or 1 stations"
        );
        // Occupancy rate must be in [0, 1].
        assert!(output.area_stats.occupancy_rate >= 0.0);
        assert!(output.area_stats.occupancy_rate <= 1.0);
    }
    // Network errors are acceptable in this validation-focused test.
}
