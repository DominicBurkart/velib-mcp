//! Tests for core domain-type invariants.
//!
//! Covers gaps in `src/types.rs`: DataFreshness exact boundaries,
//! BikeAvailability saturation, StationReference validation sad paths,
//! and VelibStation operational / capacity invariants.
//!
//! All tests are offline.

use chrono::Utc;
use velib_mcp::{
    BikeAvailability, BikeTypeFilter, Coordinates, DataFreshness, RealTimeStatus, ServiceCapabilities,
    StationReference, StationStatus, VelibStation,
};

// ── DataFreshness exact boundaries ───────────────────────────────────────────

/// The spec says Fresh < 10 min.  Exactly 10.0 should be Recent.
#[test]
fn data_freshness_boundary_at_ten_minutes() {
    assert_eq!(DataFreshness::from_age(9.999), DataFreshness::Fresh);
    assert_eq!(DataFreshness::from_age(10.0), DataFreshness::Recent);
}

/// Exactly 30.0 minutes should be Stale, not Recent.
#[test]
fn data_freshness_boundary_at_thirty_minutes() {
    assert_eq!(DataFreshness::from_age(29.999), DataFreshness::Recent);
    assert_eq!(DataFreshness::from_age(30.0), DataFreshness::Stale);
}

/// Exactly 120.0 minutes should be VeryStale, not Stale.
#[test]
fn data_freshness_boundary_at_120_minutes() {
    assert_eq!(DataFreshness::from_age(119.999), DataFreshness::Stale);
    assert_eq!(DataFreshness::from_age(120.0), DataFreshness::VeryStale);
}

// ── BikeAvailability saturation invariant ────────────────────────────────────

/// `total()` must never overflow — saturating_add prevents wrapping.
#[test]
fn bike_availability_total_saturates_at_u16_max() {
    let bikes = BikeAvailability::new(u16::MAX, 1);
    assert_eq!(bikes.total(), u16::MAX);
}

/// has_bikes is false only when both counts are zero.
#[test]
fn bike_availability_has_bikes_iff_any_nonzero() {
    assert!(!BikeAvailability::new(0, 0).has_bikes());
    assert!(BikeAvailability::new(1, 0).has_bikes());
    assert!(BikeAvailability::new(0, 1).has_bikes());
}

// ── StationReference validation sad paths ────────────────────────────────────

fn valid_reference() -> StationReference {
    StationReference {
        station_code: "TEST01".to_string(),
        name: "Test Station".to_string(),
        coordinates: Coordinates::new(48.8566, 2.3522),
        capacity: 20,
        capabilities: ServiceCapabilities::default(),
    }
}

#[test]
fn station_reference_rejects_empty_code() {
    let mut r = valid_reference();
    r.station_code = String::new();
    assert!(r.validate().is_err());
}

#[test]
fn station_reference_rejects_empty_name() {
    let mut r = valid_reference();
    r.name = String::new();
    assert!(r.validate().is_err());
}

#[test]
fn station_reference_rejects_zero_capacity() {
    let mut r = valid_reference();
    r.capacity = 0;
    assert!(r.validate().is_err());
}

#[test]
fn station_reference_rejects_capacity_above_200() {
    let mut r = valid_reference();
    r.capacity = 201;
    assert!(r.validate().is_err());
    // 200 is still valid
    r.capacity = 200;
    assert!(r.validate().is_ok());
}

#[test]
fn station_reference_rejects_non_paris_coordinates() {
    let mut r = valid_reference();
    r.coordinates = Coordinates::new(51.5074, -0.1278); // London
    assert!(r.validate().is_err());
}

// ── VelibStation::is_operational ─────────────────────────────────────────────

fn station_with_status(status: StationStatus) -> VelibStation {
    let reference = valid_reference();
    let rt = RealTimeStatus {
        bikes: BikeAvailability::new(3, 2),
        available_docks: 15,
        status,
        last_update: Utc::now(),
        data_freshness: DataFreshness::Fresh,
    };
    VelibStation::new(reference).with_real_time(rt)
}

#[test]
fn station_is_operational_when_open() {
    assert!(station_with_status(StationStatus::Open).is_operational());
}

#[test]
fn station_is_not_operational_when_closed() {
    assert!(!station_with_status(StationStatus::Closed).is_operational());
}

#[test]
fn station_is_not_operational_when_in_maintenance() {
    assert!(!station_with_status(StationStatus::Maintenance).is_operational());
}

/// A station with no real-time data is assumed operational (optimistic default).
#[test]
fn station_without_realtime_is_assumed_operational() {
    let s = VelibStation::new(valid_reference());
    assert!(s.is_operational());
}

// ── VelibStation::validate capacity boundary ──────────────────────────────────

/// bikes + docks exactly equal to capacity should be valid.
#[test]
fn station_validate_accepts_exact_capacity_fit() {
    let reference = valid_reference(); // capacity = 20
    let rt = RealTimeStatus {
        bikes: BikeAvailability::new(10, 5), // 15 bikes
        available_docks: 5,                  // 15 + 5 = 20 == capacity
        status: StationStatus::Open,
        last_update: Utc::now(),
        data_freshness: DataFreshness::Fresh,
    };
    let station = VelibStation::new(reference).with_real_time(rt);
    assert!(station.validate().is_ok());
}

/// bikes + docks one over capacity should be rejected.
#[test]
fn station_validate_rejects_one_over_capacity() {
    let reference = valid_reference(); // capacity = 20
    let rt = RealTimeStatus {
        bikes: BikeAvailability::new(10, 6), // 16 bikes
        available_docks: 5,                  // 16 + 5 = 21 > capacity 20
        status: StationStatus::Open,
        last_update: Utc::now(),
        data_freshness: DataFreshness::Fresh,
    };
    let station = VelibStation::new(reference).with_real_time(rt);
    assert!(station.validate().is_err());
}

// ── has_available_bikes without real-time data ────────────────────────────────

/// When no real-time data is present, has_available_bikes returns false for
/// every filter variant — callers must treat absence as unavailability.
#[test]
fn station_without_realtime_has_no_available_bikes() {
    let s = VelibStation::new(valid_reference());
    assert!(!s.has_available_bikes(&BikeTypeFilter::AnyType));
    assert!(!s.has_available_bikes(&BikeTypeFilter::MechanicalOnly));
    assert!(!s.has_available_bikes(&BikeTypeFilter::ElectricOnly));
}

// ── Coordinates helpers ───────────────────────────────────────────────────────

/// distance_to is symmetric: d(a,b) == d(b,a).
#[test]
fn coordinates_distance_is_symmetric() {
    let a = Coordinates::new(48.8566, 2.3522);
    let b = Coordinates::new(48.8606, 2.3376);
    let diff = (a.distance_to(&b) - b.distance_to(&a)).abs();
    assert!(diff < 1e-6, "distance_to should be symmetric, diff = {diff}");
}

/// distance_to self is zero.
#[test]
fn coordinates_distance_to_self_is_zero() {
    let a = Coordinates::new(48.8566, 2.3522);
    assert!(a.distance_to(&a) < 1e-6);
}
