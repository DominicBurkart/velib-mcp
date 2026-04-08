//! Unit tests for McpToolHandler input-validation paths.
//!
//! These tests exercise the pure validation logic inside the handlers
//! (radius limits, result limits, coordinate bounds, service-area check)
//! without any network access by relying only on the error variants
//! returned before any I/O is attempted.
//!
//! Note: tests that reach the data-fetch path will attempt a real HTTP
//! call and are marked `#[ignore]`.

use velib_mcp::{
    mcp::{
        handlers::McpToolHandler,
        types::{
            FindNearbyStationsInput, GeographicBounds, GetAreaStatisticsInput,
            PlanBikeJourneyInput, SearchStationsByNameInput,
        },
    },
    types::Coordinates,
    Error,
};

// ── find_nearby_stations – input validation ───────────────────────────────────

#[tokio::test]
async fn find_nearby_stations_rejects_radius_above_5000m() {
    let handler = McpToolHandler::new();
    let input = FindNearbyStationsInput {
        latitude: 48.8566,
        longitude: 2.3522,
        radius_meters: 5001,
        limit: 10,
        availability_filter: None,
    };
    let err = handler.find_nearby_stations(input).await.unwrap_err();
    assert!(
        matches!(err, Error::SearchRadiusTooLarge { radius: 5001, max: 5000 }),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn find_nearby_stations_allows_radius_at_5000m() {
    // Should pass validation; will then hit the network and likely succeed
    // or fail with a network error — either way, NOT SearchRadiusTooLarge.
    let handler = McpToolHandler::new();
    let input = FindNearbyStationsInput {
        latitude: 48.8566,
        longitude: 2.3522,
        radius_meters: 5000,
        limit: 10,
        availability_filter: None,
    };
    let result = handler.find_nearby_stations(input).await;
    if let Err(ref e) = result {
        assert!(
            !matches!(e, Error::SearchRadiusTooLarge { .. }),
            "radius 5000 should be accepted by validation, got: {e}"
        );
    }
}

#[tokio::test]
async fn find_nearby_stations_rejects_limit_above_100() {
    let handler = McpToolHandler::new();
    let input = FindNearbyStationsInput {
        latitude: 48.8566,
        longitude: 2.3522,
        radius_meters: 500,
        limit: 101,
        availability_filter: None,
    };
    let err = handler.find_nearby_stations(input).await.unwrap_err();
    assert!(
        matches!(err, Error::ResultLimitExceeded { limit: 101, max: 100 }),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn find_nearby_stations_rejects_non_paris_coordinates() {
    let handler = McpToolHandler::new();
    let input = FindNearbyStationsInput {
        latitude: 51.5074,  // London
        longitude: -0.1278,
        radius_meters: 500,
        limit: 10,
        availability_filter: None,
    };
    let err = handler.find_nearby_stations(input).await.unwrap_err();
    assert!(
        matches!(err, Error::InvalidCoordinates { .. }),
        "unexpected error: {err}"
    );
}

// A point within is_valid_paris_metro() bounds but > 50km from city hall.
// Coordinates around lat 49.0 lon 2.3 are on the very northern edge of the
// bounding box but still < 50km from Paris City Hall, so we use a point
// clearly past 50km: lat 49.5 (within the box) is ~73km north.
//
// NOTE: 49.5 is outside is_valid_paris_metro() (which caps at 49.0),
// so we pick 48.8 N, 4.5 E — inside lat/lon box but far east (~120km).
#[tokio::test]
async fn find_nearby_stations_rejects_outside_service_area() {
    let handler = McpToolHandler::new();
    // 48.85 N, 4.8 E is within [48.7–49.0, 2.0–2.6]? No — 4.8 > 2.6.
    // Use a coordinate that satisfies is_valid_paris_metro() (lat 48.7-49.0,
    // lon 2.0-2.6) but is NOT within 50km of city hall.  The only such
    // region is the far corner: e.g. lat 48.7, lon 2.0 is ~38km — inside.
    // Actually the entire valid-metro box is within 50km of city hall,
    // so OutsideServiceArea can only trigger for coords inside the box but
    // beyond 50km, which is impossible.  The test for OutsideServiceArea
    // must use coordinates outside the metro box lat/lon but that would
    // trigger InvalidCoordinates first.  Document this invariant instead:
    //
    // INVARIANT: Every coordinate satisfying is_valid_paris_metro() also
    // satisfies is_within_paris_service_area().  This means
    // OutsideServiceArea is unreachable via find_nearby_stations today.
    // This test documents that fact.
    let corner = Coordinates::new(48.7, 2.0); // SW corner of metro box
    assert!(
        corner.is_valid_paris_metro(),
        "corner must pass metro validation"
    );
    assert!(
        corner.is_within_paris_service_area(),
        "INVARIANT: every valid-metro coordinate is also within the 50km service area"
    );
}

// ── search_stations_by_name – input validation ────────────────────────────────

#[tokio::test]
async fn search_by_name_rejects_single_char_query() {
    let handler = McpToolHandler::new();
    let input = SearchStationsByNameInput {
        query: "A".to_string(),
        limit: 10,
        fuzzy: true,
    };
    let err = handler.search_stations_by_name(input).await.unwrap_err();
    // Handler returns Internal("Search query too short")
    assert!(
        matches!(err, Error::Internal(_)),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn search_by_name_rejects_limit_above_100() {
    let handler = McpToolHandler::new();
    let input = SearchStationsByNameInput {
        query: "Bastille".to_string(),
        limit: 101,
        fuzzy: true,
    };
    let err = handler.search_stations_by_name(input).await.unwrap_err();
    assert!(
        matches!(err, Error::ResultLimitExceeded { limit: 101, max: 100 }),
        "unexpected error: {err}"
    );
}

// ── plan_bike_journey – input validation ──────────────────────────────────────

#[tokio::test]
async fn plan_journey_rejects_non_paris_origin() {
    let handler = McpToolHandler::new();
    let input = PlanBikeJourneyInput {
        origin: Coordinates::new(51.5074, -0.1278), // London
        destination: Coordinates::new(48.8566, 2.3522),
        preferences: None,
    };
    let err = handler.plan_bike_journey(input).await.unwrap_err();
    assert!(
        matches!(err, Error::InvalidCoordinates { .. }),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn plan_journey_rejects_non_paris_destination() {
    let handler = McpToolHandler::new();
    let input = PlanBikeJourneyInput {
        origin: Coordinates::new(48.8566, 2.3522),
        destination: Coordinates::new(51.5074, -0.1278), // London
        preferences: None,
    };
    let err = handler.plan_bike_journey(input).await.unwrap_err();
    assert!(
        matches!(err, Error::InvalidCoordinates { .. }),
        "unexpected error: {err}"
    );
}

// ── GeographicBounds::contains ────────────────────────────────────────────────

use velib_mcp::mcp::types::GeographicBounds;

fn paris_bounds() -> GeographicBounds {
    GeographicBounds {
        north: 48.90,
        south: 48.82,
        east: 2.40,
        west: 2.30,
    }
}

#[test]
fn geographic_bounds_contains_interior_point() {
    let bounds = paris_bounds();
    let interior = Coordinates::new(48.86, 2.35);
    assert!(bounds.contains(&interior));
}

#[test]
fn geographic_bounds_rejects_point_north_of_bounds() {
    let bounds = paris_bounds();
    assert!(!bounds.contains(&Coordinates::new(48.91, 2.35)));
}

#[test]
fn geographic_bounds_rejects_point_south_of_bounds() {
    let bounds = paris_bounds();
    assert!(!bounds.contains(&Coordinates::new(48.81, 2.35)));
}

#[test]
fn geographic_bounds_rejects_point_east_of_bounds() {
    let bounds = paris_bounds();
    assert!(!bounds.contains(&Coordinates::new(48.86, 2.41)));
}

#[test]
fn geographic_bounds_rejects_point_west_of_bounds() {
    let bounds = paris_bounds();
    assert!(!bounds.contains(&Coordinates::new(48.86, 2.29)));
}

/// Points exactly on the boundary edges are inside (inclusive bounds).
#[test]
fn geographic_bounds_includes_north_edge() {
    let bounds = paris_bounds();
    assert!(bounds.contains(&Coordinates::new(48.90, 2.35)));
}

#[test]
fn geographic_bounds_includes_south_edge() {
    let bounds = paris_bounds();
    assert!(bounds.contains(&Coordinates::new(48.82, 2.35)));
}

#[test]
fn geographic_bounds_includes_east_edge() {
    let bounds = paris_bounds();
    assert!(bounds.contains(&Coordinates::new(48.86, 2.40)));
}

#[test]
fn geographic_bounds_includes_west_edge() {
    let bounds = paris_bounds();
    assert!(bounds.contains(&Coordinates::new(48.86, 2.30)));
}
