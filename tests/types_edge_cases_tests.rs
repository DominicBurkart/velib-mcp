//! Edge-case and boundary tests for types.rs that are not covered by existing
//! inline unit tests.

use chrono::Utc;
use velib_mcp::{
    BikeAvailability, BikeTypeFilter, Coordinates, DataFreshness, RealTimeStatus,
    ServiceCapabilities, StationReference, StationStatus, VelibStation,
};

// ---------------------------------------------------------------------------
// Coordinates
// ---------------------------------------------------------------------------

#[test]
fn distance_to_self_is_zero() {
    let c = Coordinates::new(48.8566, 2.3522);
    assert!(
        c.distance_to(&c) < 0.01,
        "Distance from a point to itself should be ~0"
    );
}

#[test]
fn distance_is_symmetric() {
    let a = Coordinates::new(48.8566, 2.3522);
    let b = Coordinates::new(48.8606, 2.3376);
    let d1 = a.distance_to(&b);
    let d2 = b.distance_to(&a);
    assert!(
        (d1 - d2).abs() < 0.01,
        "Haversine distance should be symmetric: {d1} vs {d2}"
    );
}

#[test]
fn distance_across_antimeridian() {
    // This isn't relevant for Paris, but validates the formula doesn't panic
    // on extreme longitudes.
    let a = Coordinates::new(0.0, 179.9);
    let b = Coordinates::new(0.0, -179.9);
    let d = a.distance_to(&b);
    // ~22 km at the equator for 0.2 degrees
    assert!(d > 0.0, "Distance should be positive");
}

#[test]
fn paris_metro_boundary_checks() {
    // Just inside each boundary
    assert!(Coordinates::new(48.7, 2.0).is_valid_paris_metro());
    assert!(Coordinates::new(49.0, 2.6).is_valid_paris_metro());

    // Just outside each boundary
    assert!(!Coordinates::new(48.699, 2.3).is_valid_paris_metro()); // south
    assert!(!Coordinates::new(49.001, 2.3).is_valid_paris_metro()); // north
    assert!(!Coordinates::new(48.85, 1.999).is_valid_paris_metro()); // west
    assert!(!Coordinates::new(48.85, 2.601).is_valid_paris_metro()); // east
}

// ---------------------------------------------------------------------------
// BikeAvailability
// ---------------------------------------------------------------------------

#[test]
fn total_saturates_on_overflow() {
    let bikes = BikeAvailability::new(u16::MAX, 1);
    // saturating_add should cap at u16::MAX, not wrap around
    assert_eq!(bikes.total(), u16::MAX);
}

#[test]
fn total_of_max_values_saturates() {
    let bikes = BikeAvailability::new(u16::MAX, u16::MAX);
    assert_eq!(bikes.total(), u16::MAX);
}

#[test]
fn default_bike_availability_has_no_bikes() {
    let bikes = BikeAvailability::default();
    assert_eq!(bikes.mechanical, 0);
    assert_eq!(bikes.electric, 0);
    assert!(!bikes.has_bikes());
    assert!(!bikes.has_mechanical());
    assert!(!bikes.has_electric());
}

#[test]
fn has_mechanical_only() {
    let bikes = BikeAvailability::new(1, 0);
    assert!(bikes.has_mechanical());
    assert!(!bikes.has_electric());
    assert!(bikes.has_bikes());
}

#[test]
fn has_electric_only() {
    let bikes = BikeAvailability::new(0, 1);
    assert!(!bikes.has_mechanical());
    assert!(bikes.has_electric());
    assert!(bikes.has_bikes());
}

// ---------------------------------------------------------------------------
// DataFreshness boundary values
// ---------------------------------------------------------------------------

#[test]
fn data_freshness_boundary_at_10_minutes() {
    assert_eq!(DataFreshness::from_age(9.99), DataFreshness::Fresh);
    assert_eq!(DataFreshness::from_age(10.0), DataFreshness::Recent);
}

#[test]
fn data_freshness_boundary_at_30_minutes() {
    assert_eq!(DataFreshness::from_age(29.99), DataFreshness::Recent);
    assert_eq!(DataFreshness::from_age(30.0), DataFreshness::Stale);
}

#[test]
fn data_freshness_boundary_at_120_minutes() {
    assert_eq!(DataFreshness::from_age(119.99), DataFreshness::Stale);
    assert_eq!(DataFreshness::from_age(120.0), DataFreshness::VeryStale);
}

#[test]
fn data_freshness_negative_age_is_fresh() {
    // Negative age (clock skew) should be treated as Fresh
    assert_eq!(DataFreshness::from_age(-5.0), DataFreshness::Fresh);
}

#[test]
fn data_freshness_zero_is_fresh() {
    assert_eq!(DataFreshness::from_age(0.0), DataFreshness::Fresh);
}

// ---------------------------------------------------------------------------
// StationReference::validate individual error paths
// ---------------------------------------------------------------------------

fn valid_reference() -> StationReference {
    StationReference {
        station_code: "12345".to_string(),
        name: "Test Station".to_string(),
        coordinates: Coordinates::new(48.8566, 2.3522),
        capacity: 20,
        capabilities: ServiceCapabilities::default(),
    }
}

#[test]
fn validate_rejects_empty_station_code() {
    let mut r = valid_reference();
    r.station_code = String::new();
    let err = r.validate().unwrap_err();
    assert!(
        err.contains("Station code"),
        "Expected station code error, got: {err}"
    );
}

#[test]
fn validate_rejects_empty_name() {
    let mut r = valid_reference();
    r.name = String::new();
    let err = r.validate().unwrap_err();
    assert!(
        err.contains("Station name"),
        "Expected station name error, got: {err}"
    );
}

#[test]
fn validate_rejects_zero_capacity() {
    let mut r = valid_reference();
    r.capacity = 0;
    let err = r.validate().unwrap_err();
    assert!(
        err.contains("capacity"),
        "Expected capacity error, got: {err}"
    );
}

#[test]
fn validate_rejects_capacity_over_200() {
    let mut r = valid_reference();
    r.capacity = 201;
    let err = r.validate().unwrap_err();
    assert!(
        err.contains("unreasonably high"),
        "Expected high capacity error, got: {err}"
    );
}

#[test]
fn validate_accepts_capacity_at_200() {
    let mut r = valid_reference();
    r.capacity = 200;
    assert!(r.validate().is_ok());
}

#[test]
fn validate_rejects_coordinates_outside_paris() {
    let mut r = valid_reference();
    r.coordinates = Coordinates::new(40.0, -74.0); // NYC
    let err = r.validate().unwrap_err();
    assert!(
        err.contains("outside valid Paris"),
        "Expected Paris bounds error, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// VelibStation behavior without real-time data
// ---------------------------------------------------------------------------

#[test]
fn station_without_realtime_is_operational() {
    let station = VelibStation::new(valid_reference());
    assert!(station.is_operational());
}

#[test]
fn station_without_realtime_has_no_available_bikes() {
    let station = VelibStation::new(valid_reference());
    assert!(!station.has_available_bikes(&BikeTypeFilter::AnyType));
    assert!(!station.has_available_bikes(&BikeTypeFilter::MechanicalOnly));
    assert!(!station.has_available_bikes(&BikeTypeFilter::ElectricOnly));
}

#[test]
fn station_without_realtime_has_no_available_docks() {
    let station = VelibStation::new(valid_reference());
    assert!(!station.has_available_docks(1));
    assert!(!station.has_available_docks(0));
}

#[test]
fn station_without_realtime_validates_ok() {
    let station = VelibStation::new(valid_reference());
    assert!(station.validate().is_ok());
}

#[test]
fn closed_station_is_not_operational() {
    let station = VelibStation::new(valid_reference()).with_real_time(RealTimeStatus {
        bikes: BikeAvailability::new(5, 3),
        available_docks: 12,
        status: StationStatus::Closed,
        last_update: Utc::now(),
        data_freshness: DataFreshness::Fresh,
    });
    assert!(!station.is_operational());
}

#[test]
fn maintenance_station_is_not_operational() {
    let station = VelibStation::new(valid_reference()).with_real_time(RealTimeStatus {
        bikes: BikeAvailability::new(5, 3),
        available_docks: 12,
        status: StationStatus::Maintenance,
        last_update: Utc::now(),
        data_freshness: DataFreshness::Fresh,
    });
    assert!(!station.is_operational());
}

#[test]
fn validate_catches_bikes_plus_docks_exceeding_capacity() {
    let station = VelibStation {
        reference: StationReference {
            station_code: "V1".into(),
            name: "Small Station".into(),
            coordinates: Coordinates::new(48.8566, 2.3522),
            capacity: 5,
            capabilities: ServiceCapabilities::default(),
        },
        real_time: Some(RealTimeStatus {
            bikes: BikeAvailability::new(3, 2),
            available_docks: 1, // 3 + 2 + 1 = 6 > capacity 5
            status: StationStatus::Open,
            last_update: Utc::now(),
            data_freshness: DataFreshness::Fresh,
        }),
    };
    let err = station.validate().unwrap_err();
    assert!(
        err.contains("exceeds capacity"),
        "Expected capacity exceeded error, got: {err}"
    );
}

#[test]
fn validate_accepts_bikes_plus_docks_equal_to_capacity() {
    let station = VelibStation {
        reference: StationReference {
            station_code: "V2".into(),
            name: "Full Station".into(),
            coordinates: Coordinates::new(48.8566, 2.3522),
            capacity: 10,
            capabilities: ServiceCapabilities::default(),
        },
        real_time: Some(RealTimeStatus {
            bikes: BikeAvailability::new(4, 3),
            available_docks: 3, // 4 + 3 + 3 = 10 == capacity
            status: StationStatus::Open,
            last_update: Utc::now(),
            data_freshness: DataFreshness::Fresh,
        }),
    };
    assert!(station.validate().is_ok());
}
