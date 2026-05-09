//! Tests for MCP handler input validation paths.
//!
//! These tests exercise the error-returning branches of McpToolHandler methods
//! without needing live API access. They construct inputs that fail validation
//! *before* any network call is made.

use velib_mcp::mcp::handlers::McpToolHandler;
use velib_mcp::mcp::types::{
    FindNearbyStationsInput, PlanBikeJourneyInput, SearchStationsByNameInput,
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
async fn is_within_paris_service_area_rejects_distant_coordinates() {
    // The handler's service-area check is gated behind `is_valid_paris_metro`
    // (a bounding box fully contained within the 50km radius), so the
    // `OutsideServiceArea` branch is not reachable via the public handler API
    // with currently valid box coordinates. Instead, verify the guard
    // function `is_within_paris_service_area` directly with coordinates
    // provably outside the 50km service radius from Paris City Hall
    // (48.8565, 2.3514).
    //
    // Reims is ~130km ENE of Paris -- unambiguously outside 50km.
    let reims = Coordinates::new(49.2583, 4.0317);
    assert!(
        !reims.is_within_paris_service_area(),
        "Reims must be outside the 50km Paris service area"
    );

    // Lyon is ~390km SSE -- unambiguously outside.
    let lyon = Coordinates::new(45.7640, 4.8357);
    assert!(
        !lyon.is_within_paris_service_area(),
        "Lyon must be outside the 50km Paris service area"
    );

    // Paris City Hall itself must be inside.
    let city_hall = Coordinates::new(48.8565, 2.3514);
    assert!(
        city_hall.is_within_paris_service_area(),
        "Paris City Hall must be inside the 50km Paris service area"
    );
}

#[tokio::test]
async fn find_nearby_rejects_coordinates_outside_service_area() {
    let handler = McpToolHandler::new();
    // ~100 km north of Paris City Hall (48.8565, 2.3514) -- unambiguously
    // outside the 50 km service-area radius. Validation must reject this
    // before any network call is made.
    let input = FindNearbyStationsInput {
        latitude: 49.75,
        longitude: 2.3522,
        radius_meters: 500,
        limit: 10,
        availability_filter: None,
    };

    let result = handler.find_nearby_stations(input).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.error_type(), "outside_service_area");
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
