//! Tests for McpToolHandler input validation logic.
//!
//! These tests exercise the validation paths in handlers.rs without
//! requiring network access -- they fail fast before any API call.

use velib_mcp::mcp::handlers::McpToolHandler;
use velib_mcp::mcp::types::{
    FindNearbyStationsInput, PlanBikeJourneyInput, SearchStationsByNameInput,
};
use velib_mcp::Coordinates;

fn handler() -> McpToolHandler {
    McpToolHandler::new()
}

// --- find_nearby_stations validation ---

#[tokio::test]
async fn find_nearby_rejects_radius_too_large() {
    let input = FindNearbyStationsInput {
        latitude: 48.8566,
        longitude: 2.3522,
        radius_meters: 10_000, // max is 5000
        limit: 10,
        availability_filter: None,
    };
    let err = handler().find_nearby_stations(input).await.unwrap_err();
    assert!(
        err.to_string().contains("radius too large"),
        "Expected radius error, got: {err}"
    );
}

#[tokio::test]
async fn find_nearby_rejects_limit_too_high() {
    let input = FindNearbyStationsInput {
        latitude: 48.8566,
        longitude: 2.3522,
        radius_meters: 500,
        limit: 200, // max is 100
        availability_filter: None,
    };
    let err = handler().find_nearby_stations(input).await.unwrap_err();
    assert!(
        err.to_string().contains("limit") || err.to_string().contains("Result limit"),
        "Expected limit error, got: {err}"
    );
}

#[tokio::test]
async fn find_nearby_rejects_coords_outside_paris() {
    let input = FindNearbyStationsInput {
        latitude: 40.7128, // NYC
        longitude: -74.006,
        radius_meters: 500,
        limit: 10,
        availability_filter: None,
    };
    let err = handler().find_nearby_stations(input).await.unwrap_err();
    assert!(
        err.to_string().contains("coordinates")
            || err.to_string().contains("Invalid")
            || err.to_string().contains("service area"),
        "Expected coordinates error, got: {err}"
    );
}

// --- search_stations_by_name validation ---

#[tokio::test]
async fn search_by_name_rejects_short_query() {
    let input = SearchStationsByNameInput {
        query: "a".to_string(), // min 2 chars
        limit: 10,
        fuzzy: true,
    };
    let err = handler().search_stations_by_name(input).await.unwrap_err();
    assert!(
        err.to_string().contains("too short"),
        "Expected too-short error, got: {err}"
    );
}

#[tokio::test]
async fn search_by_name_rejects_limit_too_high() {
    let input = SearchStationsByNameInput {
        query: "chatelet".to_string(),
        limit: 200, // max 100
        fuzzy: true,
    };
    let err = handler().search_stations_by_name(input).await.unwrap_err();
    assert!(
        err.to_string().contains("limit") || err.to_string().contains("Result limit"),
        "Expected limit error, got: {err}"
    );
}

// --- plan_bike_journey validation ---

#[tokio::test]
async fn plan_journey_rejects_origin_outside_paris() {
    let input = PlanBikeJourneyInput {
        origin: Coordinates::new(51.5074, -0.1278), // London
        destination: Coordinates::new(48.8566, 2.3522),
        preferences: None,
    };
    let err = handler().plan_bike_journey(input).await.unwrap_err();
    assert!(
        err.to_string().contains("coordinates")
            || err.to_string().contains("Invalid")
            || err.to_string().contains("service area"),
        "Expected coordinates error for origin, got: {err}"
    );
}

#[tokio::test]
async fn plan_journey_rejects_destination_outside_paris() {
    let input = PlanBikeJourneyInput {
        origin: Coordinates::new(48.8566, 2.3522),
        destination: Coordinates::new(51.5074, -0.1278), // London
        preferences: None,
    };
    let err = handler().plan_bike_journey(input).await.unwrap_err();
    assert!(
        err.to_string().contains("coordinates")
            || err.to_string().contains("Invalid")
            || err.to_string().contains("service area"),
        "Expected coordinates error for destination, got: {err}"
    );
}
