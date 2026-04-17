//! Focused unit tests for core domain-type invariants.
//!
//! All tests run offline — no network, no spawned process.  They target
//! the exact boundary values and edge cases that the existing test suite
//! leaves uncovered:
//!
//! * `DataFreshness::from_age` – exact transition points (10.0, 30.0, 120.0)
//! * `BikeAvailability::total` – saturating-add at `u16::MAX`
//! * `Coordinates::distance_to` – symmetry, self-distance, known reference pair
//! * `StationReference::validate` – every error branch
//! * `VelibStation` methods – no-real-time fallback paths
//! * `GeographicBounds::contains` – inverted / degenerate bounds
//! * `AvailabilityFilter` – serde default for `exclude_out_of_service`

use chrono::Utc;
use velib_mcp::mcp::types::{AvailabilityFilter, GeographicBounds};
use velib_mcp::types::{
    BikeAvailability, BikeTypeFilter, Coordinates, DataFreshness, RealTimeStatus,
    ServiceCapabilities, StationReference, StationStatus, VelibStation,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn paris_ref(code: &str) -> StationReference {
    StationReference {
        station_code: code.to_string(),
        name: "Test Station".to_string(),
        coordinates: Coordinates::new(48.8566, 2.3522),
        capacity: 20,
        capabilities: ServiceCapabilities::default(),
    }
}

fn open_real_time(mechanical: u16, electric: u16, docks: u16) -> RealTimeStatus {
    RealTimeStatus {
        bikes: BikeAvailability::new(mechanical, electric),
        available_docks: docks,
        status: StationStatus::Open,
        last_update: Utc::now(),
        data_freshness: DataFreshness::Fresh,
    }
}

// ---------------------------------------------------------------------------
// DataFreshness – boundary values at exactly 10, 30, 120 minutes
// ---------------------------------------------------------------------------

/// Values strictly below each threshold should map to the *fresher* bucket.
/// Values at or above the threshold should map to the *staler* bucket.
/// The implementation uses `age < 10.0`, `age < 30.0`, `age < 120.0`.
#[test]
fn data_freshness_boundary_at_ten_minutes() {
    // 9.999... → Fresh
    assert_eq!(DataFreshness::from_age(9.999), DataFreshness::Fresh);
    // Exactly 10.0 is NOT < 10.0, so → Recent
    assert_eq!(DataFreshness::from_age(10.0), DataFreshness::Recent);
    // 10.001 → Recent
    assert_eq!(DataFreshness::from_age(10.001), DataFreshness::Recent);
}

#[test]
fn data_freshness_boundary_at_thirty_minutes() {
    assert_eq!(DataFreshness::from_age(29.999), DataFreshness::Recent);
    assert_eq!(DataFreshness::from_age(30.0), DataFreshness::Stale);
    assert_eq!(DataFreshness::from_age(30.001), DataFreshness::Stale);
}

#[test]
fn data_freshness_boundary_at_one_twenty_minutes() {
    assert_eq!(DataFreshness::from_age(119.999), DataFreshness::Stale);
    assert_eq!(DataFreshness::from_age(120.0), DataFreshness::VeryStale);
    assert_eq!(DataFreshness::from_age(120.001), DataFreshness::VeryStale);
}

#[test]
fn data_freshness_zero_age_is_fresh() {
    assert_eq!(DataFreshness::from_age(0.0), DataFreshness::Fresh);
}

#[test]
fn data_freshness_negative_age_is_fresh() {
    // Clocks can skew; negative age must not panic and should be Fresh.
    assert_eq!(DataFreshness::from_age(-1.0), DataFreshness::Fresh);
}

// ---------------------------------------------------------------------------
// BikeAvailability – saturation and helper methods
// ---------------------------------------------------------------------------

#[test]
fn bike_availability_total_saturates_at_u16_max() {
    let bikes = BikeAvailability::new(u16::MAX, 1);
    // saturating_add must not panic or wrap; result == u16::MAX
    assert_eq!(bikes.total(), u16::MAX);
}

#[test]
fn bike_availability_total_both_max() {
    let bikes = BikeAvailability::new(u16::MAX, u16::MAX);
    assert_eq!(bikes.total(), u16::MAX);
}

#[test]
fn bike_availability_has_bikes_false_when_zero() {
    assert!(!BikeAvailability::new(0, 0).has_bikes());
}

#[test]
fn bike_availability_has_mechanical_only() {
    let bikes = BikeAvailability::new(1, 0);
    assert!(bikes.has_mechanical());
    assert!(!bikes.has_electric());
    assert!(bikes.has_bikes());
}

#[test]
fn bike_availability_has_electric_only() {
    let bikes = BikeAvailability::new(0, 1);
    assert!(!bikes.has_mechanical());
    assert!(bikes.has_electric());
    assert!(bikes.has_bikes());
}

// ---------------------------------------------------------------------------
// Coordinates – distance_to invariants
// ---------------------------------------------------------------------------

#[test]
fn coordinates_distance_to_self_is_zero() {
    let c = Coordinates::new(48.8566, 2.3522);
    let d = c.distance_to(&c);
    // Floating-point haversine of identical points must be negligibly small.
    assert!(d < 1e-6, "distance to self should be ~0, got {d}");
}

#[test]
fn coordinates_distance_is_symmetric() {
    let a = Coordinates::new(48.8566, 2.3522);
    let b = Coordinates::new(48.8606, 2.3376);
    let d_ab = a.distance_to(&b);
    let d_ba = b.distance_to(&a);
    // The two values should agree to within floating-point rounding.
    assert!(
        (d_ab - d_ba).abs() < 1e-6,
        "distance not symmetric: {d_ab} vs {d_ba}"
    );
}

#[test]
fn coordinates_distance_known_pair_louvre_to_notre_dame() {
    // Louvre: 48.8606° N, 2.3376° E
    // Notre-Dame: 48.8530° N, 2.3499° E
    // Real geodetic distance ≈ 1.05 km – allow ±10 % for Haversine.
    let louvre = Coordinates::new(48.8606, 2.3376);
    let notre_dame = Coordinates::new(48.8530, 2.3499);
    let d = louvre.distance_to(&notre_dame);
    assert!(
        d > 900.0 && d < 1200.0,
        "Louvre→Notre-Dame distance out of range: {d:.1} m"
    );
}

#[test]
fn coordinates_distance_increases_with_separation() {
    let origin = Coordinates::new(48.8565, 2.3514);
    let near = Coordinates::new(48.8575, 2.3514); // ~110 m north
    let far = Coordinates::new(48.8665, 2.3514); // ~1.1 km north
    assert!(origin.distance_to(&near) < origin.distance_to(&far));
}

// ---------------------------------------------------------------------------
// StationReference::validate – every error branch
// ---------------------------------------------------------------------------

#[test]
fn station_reference_validate_empty_code_is_err() {
    let mut r = paris_ref("valid");
    r.station_code = String::new();
    let err = r.validate().unwrap_err();
    assert!(err.contains("code"), "error should mention 'code': {err}");
}

#[test]
fn station_reference_validate_empty_name_is_err() {
    let mut r = paris_ref("valid");
    r.name = String::new();
    let err = r.validate().unwrap_err();
    assert!(err.contains("name"), "error should mention 'name': {err}");
}

#[test]
fn station_reference_validate_zero_capacity_is_err() {
    let mut r = paris_ref("valid");
    r.capacity = 0;
    let err = r.validate().unwrap_err();
    assert!(
        err.contains("capacity"),
        "error should mention 'capacity': {err}"
    );
}

#[test]
fn station_reference_validate_capacity_201_is_err() {
    let mut r = paris_ref("valid");
    r.capacity = 201;
    assert!(r.validate().is_err());
}

#[test]
fn station_reference_validate_capacity_200_is_ok() {
    let mut r = paris_ref("valid");
    r.capacity = 200; // boundary: exactly 200 is allowed
    assert!(r.validate().is_ok());
}

#[test]
fn station_reference_validate_out_of_paris_bounds_is_err() {
    let mut r = paris_ref("valid");
    r.coordinates = Coordinates::new(40.7128, -74.0060); // New York
    let err = r.validate().unwrap_err();
    assert!(
        err.contains("Paris") || err.contains("coordinate") || err.contains("Coordinates"),
        "error should mention location: {err}"
    );
}

#[test]
fn station_reference_validate_happy_path() {
    assert!(paris_ref("12345").validate().is_ok());
}

// ---------------------------------------------------------------------------
// VelibStation – no-real-time fallback paths
// ---------------------------------------------------------------------------

#[test]
fn velib_station_is_operational_no_real_time_defaults_true() {
    // Without real-time data the station is assumed operational.
    let station = VelibStation::new(paris_ref("X"));
    assert!(station.is_operational());
}

#[test]
fn velib_station_is_operational_closed_status_is_false() {
    let mut rt = open_real_time(2, 0, 18);
    rt.status = StationStatus::Closed;
    let station = VelibStation::new(paris_ref("X")).with_real_time(rt);
    assert!(!station.is_operational());
}

#[test]
fn velib_station_is_operational_maintenance_status_is_false() {
    let mut rt = open_real_time(2, 0, 18);
    rt.status = StationStatus::Maintenance;
    let station = VelibStation::new(paris_ref("X")).with_real_time(rt);
    assert!(!station.is_operational());
}

#[test]
fn velib_station_has_available_bikes_no_real_time_returns_false() {
    let station = VelibStation::new(paris_ref("X"));
    assert!(!station.has_available_bikes(&BikeTypeFilter::AnyType));
    assert!(!station.has_available_bikes(&BikeTypeFilter::MechanicalOnly));
    assert!(!station.has_available_bikes(&BikeTypeFilter::ElectricOnly));
}

#[test]
fn velib_station_has_available_docks_no_real_time_returns_false() {
    let station = VelibStation::new(paris_ref("X"));
    assert!(!station.has_available_docks(1));
}

#[test]
fn velib_station_has_available_docks_zero_threshold_always_true_when_open() {
    // min_docks=0 should always be satisfied when real-time data is present.
    let station = VelibStation::new(paris_ref("X")).with_real_time(open_real_time(0, 0, 0));
    assert!(station.has_available_docks(0));
}

#[test]
fn velib_station_has_available_docks_exact_threshold() {
    let station = VelibStation::new(paris_ref("X")).with_real_time(open_real_time(0, 0, 5));
    assert!(station.has_available_docks(5)); // exactly meets threshold
    assert!(!station.has_available_docks(6)); // one above threshold
}

// ---------------------------------------------------------------------------
// VelibStation::validate – capacity overflow
// ---------------------------------------------------------------------------

#[test]
fn velib_station_validate_bikes_plus_docks_exceeds_capacity_is_err() {
    let mut r = paris_ref("X");
    r.capacity = 10;
    let rt = open_real_time(6, 5, 5); // 11 bikes + 5 docks = 16 > capacity 10
    let station = VelibStation::new(r).with_real_time(rt);
    assert!(station.validate().is_err());
}

#[test]
fn velib_station_validate_bikes_plus_docks_equals_capacity_is_ok() {
    let mut r = paris_ref("X");
    r.capacity = 20;
    // 5 mechanical + 5 electric = 10 bikes; 10 docks; total = 20 = capacity
    let rt = open_real_time(5, 5, 10);
    let station = VelibStation::new(r).with_real_time(rt);
    assert!(station.validate().is_ok());
}

#[test]
fn velib_station_validate_no_real_time_uses_reference_only() {
    // Without real-time data, validate() delegates entirely to reference.validate().
    let station = VelibStation::new(paris_ref("X"));
    assert!(station.validate().is_ok());
}

// ---------------------------------------------------------------------------
// GeographicBounds – edge cases
// ---------------------------------------------------------------------------

#[test]
fn geographic_bounds_degenerate_point_contains_only_itself() {
    // A bounds where north==south and east==west is a single point.
    let bounds = GeographicBounds {
        north: 48.85,
        south: 48.85,
        east: 2.35,
        west: 2.35,
    };
    let exact = Coordinates::new(48.85, 2.35);
    let off = Coordinates::new(48.85, 2.351);
    assert!(bounds.contains(&exact));
    assert!(!bounds.contains(&off));
}

#[test]
fn geographic_bounds_inverted_latitude_contains_nothing() {
    // south > north: no point can satisfy lat >= south AND lat <= north simultaneously.
    let bounds = GeographicBounds {
        north: 48.80,
        south: 48.90, // inverted
        east: 2.40,
        west: 2.30,
    };
    let c = Coordinates::new(48.85, 2.35);
    assert!(!bounds.contains(&c));
}

#[test]
fn geographic_bounds_inverted_longitude_contains_nothing() {
    let bounds = GeographicBounds {
        north: 48.90,
        south: 48.80,
        east: 2.30,
        west: 2.40, // inverted
    };
    let c = Coordinates::new(48.85, 2.35);
    assert!(!bounds.contains(&c));
}

// ---------------------------------------------------------------------------
// AvailabilityFilter – serde default for exclude_out_of_service
// ---------------------------------------------------------------------------

#[test]
fn availability_filter_default_excludes_out_of_service() {
    let filter = AvailabilityFilter::default();
    assert!(
        filter.exclude_out_of_service,
        "default should exclude out-of-service stations"
    );
}

#[test]
fn availability_filter_deserializes_exclude_out_of_service_default_true() {
    // When the field is absent from JSON the serde default (`default_true`) fires.
    let json = r#"{"min_bikes": 1}"#;
    let filter: AvailabilityFilter = serde_json::from_str(json).unwrap();
    assert!(
        filter.exclude_out_of_service,
        "omitted field should default to true"
    );
    assert_eq!(filter.min_bikes, Some(1));
}

#[test]
fn availability_filter_deserializes_exclude_out_of_service_explicit_false() {
    let json = r#"{"exclude_out_of_service": false}"#;
    let filter: AvailabilityFilter = serde_json::from_str(json).unwrap();
    assert!(!filter.exclude_out_of_service);
}

#[test]
fn availability_filter_optional_fields_absent_are_none() {
    let filter: AvailabilityFilter = serde_json::from_str("{}").unwrap();
    assert!(filter.min_bikes.is_none());
    assert!(filter.min_docks.is_none());
    assert!(filter.bike_type.is_none());
}
