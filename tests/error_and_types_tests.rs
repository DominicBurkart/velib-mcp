//! Offline unit tests for error codes, domain types, geographic bounds,
//! and handler input-validation guards.
//!
//! None of these tests make network calls.

use chrono::Utc;
use velib_mcp::{
    error::Error,
    mcp::{
        handlers::McpToolHandler,
        types::{FindNearbyStationsInput, GeographicBounds, SearchStationsByNameInput},
    },
    types::{
        BikeAvailability, BikeTypeFilter, Coordinates, DataFreshness, RealTimeStatus,
        ServiceCapabilities, StationReference, StationStatus, VelibStation,
    },
};

// ---------------------------------------------------------------------------
// Error: MCP error codes and error_type strings
// ---------------------------------------------------------------------------

#[test]
fn mcp_error_codes_match_jsonrpc_spec() {
    // -32602 = Invalid params
    assert_eq!(
        Error::InvalidCoordinates {
            latitude: 0.0,
            longitude: 0.0
        }
        .mcp_error_code(),
        -32602
    );
    assert_eq!(
        Error::OutsideServiceArea { distance_km: 60.0 }.mcp_error_code(),
        -32602
    );
    assert_eq!(
        Error::SearchRadiusTooLarge {
            radius: 9000,
            max: 5000
        }
        .mcp_error_code(),
        -32602
    );
    assert_eq!(
        Error::ResultLimitExceeded { limit: 200, max: 100 }.mcp_error_code(),
        -32602
    );
    assert_eq!(
        Error::Validation("bad".into()).mcp_error_code(),
        -32602
    );

    // -32600 = Invalid request
    assert_eq!(
        Error::StationNotFound {
            station_code: "X".into()
        }
        .mcp_error_code(),
        -32600
    );

    // -32603 = Internal error
    assert_eq!(
        Error::McpProtocol("oops".into()).mcp_error_code(),
        -32603
    );
    assert_eq!(
        Error::Cache("oops".into()).mcp_error_code(),
        -32603
    );
    assert_eq!(
        Error::Internal(anyhow::anyhow!("oops")).mcp_error_code(),
        -32603
    );

    // -32001 = Server / rate-limited
    assert_eq!(
        Error::RateLimited {
            retry_after_seconds: None
        }
        .mcp_error_code(),
        -32001
    );
}

#[test]
fn error_type_strings_are_stable() {
    assert_eq!(
        Error::RateLimited {
            retry_after_seconds: Some(5)
        }
        .error_type(),
        "rate_limited"
    );
    assert_eq!(
        Error::StationNotFound {
            station_code: "X".into()
        }
        .error_type(),
        "station_not_found"
    );
    assert_eq!(
        Error::OutsideServiceArea { distance_km: 55.0 }.error_type(),
        "outside_service_area"
    );
    assert_eq!(
        Error::SearchRadiusTooLarge {
            radius: 6000,
            max: 5000
        }
        .error_type(),
        "search_radius_too_large"
    );
    assert_eq!(
        Error::ResultLimitExceeded { limit: 101, max: 100 }.error_type(),
        "result_limit_exceeded"
    );
    assert_eq!(Error::Cache("x".into()).error_type(), "cache_error");
    assert_eq!(
        Error::Validation("x".into()).error_type(),
        "validation_error"
    );
    assert_eq!(
        Error::McpProtocol("x".into()).error_type(),
        "mcp_protocol_error"
    );
    assert_eq!(
        Error::Internal(anyhow::anyhow!("x")).error_type(),
        "internal_error"
    );
}

#[test]
fn rate_limited_display_includes_seconds_when_present() {
    let with_retry = Error::RateLimited {
        retry_after_seconds: Some(30),
    };
    assert!(with_retry.to_string().contains("retry after 30s"));

    let without_retry = Error::RateLimited {
        retry_after_seconds: None,
    };
    let msg = without_retry.to_string();
    assert!(msg.contains("Rate limited"));
    assert!(!msg.contains("retry after"));
}

// ---------------------------------------------------------------------------
// BikeAvailability
// ---------------------------------------------------------------------------

#[test]
fn bike_availability_total_saturates_on_overflow() {
    let bikes = BikeAvailability::new(u16::MAX, 1);
    // saturating_add must not panic and must stay at u16::MAX
    assert_eq!(bikes.total(), u16::MAX);
}

#[test]
fn bike_availability_all_zero_has_no_bikes() {
    let empty = BikeAvailability::new(0, 0);
    assert!(!empty.has_bikes());
    assert!(!empty.has_mechanical());
    assert!(!empty.has_electric());
}

#[test]
fn bike_availability_mixed_flags() {
    let mech_only = BikeAvailability::new(3, 0);
    assert!(mech_only.has_bikes());
    assert!(mech_only.has_mechanical());
    assert!(!mech_only.has_electric());

    let elec_only = BikeAvailability::new(0, 2);
    assert!(elec_only.has_bikes());
    assert!(!elec_only.has_mechanical());
    assert!(elec_only.has_electric());
}

// ---------------------------------------------------------------------------
// StationReference::validate
// ---------------------------------------------------------------------------

fn valid_reference() -> StationReference {
    StationReference {
        station_code: "001".to_string(),
        name: "Test Station".to_string(),
        coordinates: Coordinates::new(48.8566, 2.3522),
        capacity: 20,
        capabilities: ServiceCapabilities::default(),
    }
}

#[test]
fn station_reference_validate_rejects_empty_code() {
    let mut r = valid_reference();
    r.station_code = String::new();
    assert!(r.validate().is_err());
}

#[test]
fn station_reference_validate_rejects_empty_name() {
    let mut r = valid_reference();
    r.name = String::new();
    assert!(r.validate().is_err());
}

#[test]
fn station_reference_validate_rejects_zero_capacity() {
    let mut r = valid_reference();
    r.capacity = 0;
    assert!(r.validate().is_err());
}

#[test]
fn station_reference_validate_rejects_excess_capacity() {
    let mut r = valid_reference();
    r.capacity = 201; // > 200 limit
    assert!(r.validate().is_err());
}

#[test]
fn station_reference_validate_rejects_non_paris_coords() {
    let mut r = valid_reference();
    r.coordinates = Coordinates::new(51.5074, -0.1278); // London
    assert!(r.validate().is_err());
}

// ---------------------------------------------------------------------------
// VelibStation: has_available_docks, has_available_bikes without real_time
// ---------------------------------------------------------------------------

fn make_station(mechanical: u16, electric: u16, docks: u16) -> VelibStation {
    let reference = valid_reference();
    let bikes = BikeAvailability::new(mechanical, electric);
    let rt = RealTimeStatus::new(bikes, docks, StationStatus::Open, Utc::now());
    VelibStation::new(reference).with_real_time(rt)
}

#[test]
fn station_without_realtime_has_no_bikes_and_no_docks() {
    let station = VelibStation::new(valid_reference());
    assert!(!station.has_available_bikes(&BikeTypeFilter::AnyType));
    assert!(!station.has_available_docks(1));
    // But it is assumed operational
    assert!(station.is_operational());
}

#[test]
fn station_has_available_docks_threshold() {
    let station = make_station(0, 0, 5);
    assert!(station.has_available_docks(5));
    assert!(station.has_available_docks(1));
    assert!(!station.has_available_docks(6));
}

#[test]
fn station_closed_is_not_operational() {
    let reference = valid_reference();
    let rt = RealTimeStatus::new(
        BikeAvailability::new(3, 0),
        10,
        StationStatus::Closed,
        Utc::now(),
    );
    let station = VelibStation::new(reference).with_real_time(rt);
    assert!(!station.is_operational());
}

#[test]
fn station_validate_rejects_bikes_plus_docks_exceeding_capacity() {
    // capacity=10, bikes=8, docks=5 => total 13 > 10
    let mut station = make_station(8, 0, 5);
    station.reference.capacity = 10;
    assert!(station.validate().is_err());
}

// ---------------------------------------------------------------------------
// DataFreshness boundaries
// ---------------------------------------------------------------------------

#[test]
fn data_freshness_boundaries_are_exclusive_at_thresholds() {
    // Boundary: exactly 10.0 minutes → Recent (not Fresh)
    assert_eq!(DataFreshness::from_age(10.0), DataFreshness::Recent);
    // Boundary: exactly 30.0 minutes → Stale (not Recent)
    assert_eq!(DataFreshness::from_age(30.0), DataFreshness::Stale);
    // Boundary: exactly 120.0 minutes → VeryStale (not Stale)
    assert_eq!(DataFreshness::from_age(120.0), DataFreshness::VeryStale);
    // Below each threshold
    assert_eq!(DataFreshness::from_age(9.99), DataFreshness::Fresh);
    assert_eq!(DataFreshness::from_age(29.99), DataFreshness::Recent);
    assert_eq!(DataFreshness::from_age(119.99), DataFreshness::Stale);
}

// ---------------------------------------------------------------------------
// GeographicBounds::contains
// ---------------------------------------------------------------------------

fn paris_bounds() -> GeographicBounds {
    GeographicBounds {
        north: 49.0,
        south: 48.7,
        east: 2.6,
        west: 2.0,
    }
}

#[test]
fn bounds_contains_interior_point() {
    let b = paris_bounds();
    assert!(b.contains(&Coordinates::new(48.8566, 2.3522))); // Paris centre
}

#[test]
fn bounds_contains_exact_corner() {
    let b = paris_bounds();
    // NW corner: lat==north, lon==west — should be included (>=south, <=north etc.)
    assert!(b.contains(&Coordinates::new(49.0, 2.0)));
    // SE corner
    assert!(b.contains(&Coordinates::new(48.7, 2.6)));
}

#[test]
fn bounds_excludes_point_outside() {
    let b = paris_bounds();
    assert!(!b.contains(&Coordinates::new(51.5, -0.1))); // London
    assert!(!b.contains(&Coordinates::new(48.6, 2.3))); // South of bounds
    assert!(!b.contains(&Coordinates::new(49.1, 2.3))); // North of bounds
    assert!(!b.contains(&Coordinates::new(48.85, 1.9))); // West of bounds
    assert!(!b.contains(&Coordinates::new(48.85, 2.7))); // East of bounds
}

// ---------------------------------------------------------------------------
// Handler input-validation guards (no network calls)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn find_nearby_rejects_radius_too_large() {
    let handler = McpToolHandler::new();
    let input = FindNearbyStationsInput {
        latitude: 48.8566,
        longitude: 2.3522,
        radius_meters: 9_999, // > 5000 limit
        limit: 10,
        availability_filter: None,
    };
    let err = handler.find_nearby_stations(input).await.unwrap_err();
    assert!(matches!(err, Error::SearchRadiusTooLarge { .. }));
}

#[tokio::test]
async fn find_nearby_rejects_limit_too_large() {
    let handler = McpToolHandler::new();
    let input = FindNearbyStationsInput {
        latitude: 48.8566,
        longitude: 2.3522,
        radius_meters: 500,
        limit: 200, // > 100 limit
        availability_filter: None,
    };
    let err = handler.find_nearby_stations(input).await.unwrap_err();
    assert!(matches!(err, Error::ResultLimitExceeded { .. }));
}

#[tokio::test]
async fn find_nearby_rejects_non_paris_coordinates() {
    let handler = McpToolHandler::new();
    let input = FindNearbyStationsInput {
        latitude: 40.7128,  // New York
        longitude: -74.0060,
        radius_meters: 500,
        limit: 10,
        availability_filter: None,
    };
    let err = handler.find_nearby_stations(input).await.unwrap_err();
    assert!(matches!(err, Error::InvalidCoordinates { .. }));
}

#[tokio::test]
async fn search_by_name_rejects_single_char_query() {
    let handler = McpToolHandler::new();
    let input = SearchStationsByNameInput {
        query: "A".to_string(), // < 2 characters
        limit: 10,
        fuzzy: true,
    };
    let result = handler.search_stations_by_name(input).await;
    assert!(result.is_err(), "Single-char query should be rejected");
}
