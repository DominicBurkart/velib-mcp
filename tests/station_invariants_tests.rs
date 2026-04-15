//! Tests for untested invariants in VelibStation, StationReference, and BikeAvailability.

use chrono::Utc;
use velib_mcp::types::{
    BikeAvailability, BikeTypeFilter, Coordinates, DataFreshness, ServiceCapabilities,
    StationReference, StationStatus, RealTimeStatus, VelibStation,
};

fn paris_coords() -> Coordinates {
    Coordinates::new(48.8566, 2.3522)
}

fn make_reference(code: &str, name: &str, capacity: u16) -> StationReference {
    StationReference {
        station_code: code.to_string(),
        name: name.to_string(),
        coordinates: paris_coords(),
        capacity,
        capabilities: ServiceCapabilities::default(),
    }
}

// --- StationReference::validate() ---

#[test]
fn station_reference_validate_rejects_empty_code() {
    let r = make_reference("", "Good Name", 20);
    let err = r.validate().unwrap_err();
    assert!(err.contains("code"), "Expected error about code, got: {err}");
}

#[test]
fn station_reference_validate_rejects_empty_name() {
    let r = make_reference("ABC", "", 20);
    let err = r.validate().unwrap_err();
    assert!(err.contains("name"), "Expected error about name, got: {err}");
}

#[test]
fn station_reference_validate_rejects_zero_capacity() {
    let r = make_reference("ABC", "Good Name", 0);
    assert!(r.validate().is_err());
}

// --- VelibStation::is_operational() ---

#[test]
fn station_is_operational_true_when_no_real_time_data() {
    let station = VelibStation::new(make_reference("X1", "Test", 10));
    assert!(station.is_operational(), "Should be operational without real-time data");
}

#[test]
fn station_is_operational_false_when_closed() {
    let rt = RealTimeStatus {
        bikes: BikeAvailability::new(0, 0),
        available_docks: 10,
        status: StationStatus::Closed,
        last_update: Utc::now(),
        data_freshness: DataFreshness::Fresh,
    };
    let station = VelibStation::new(make_reference("X2", "Test", 10)).with_real_time(rt);
    assert!(!station.is_operational());
}

#[test]
fn station_is_operational_false_when_maintenance() {
    let rt = RealTimeStatus {
        bikes: BikeAvailability::new(2, 1),
        available_docks: 7,
        status: StationStatus::Maintenance,
        last_update: Utc::now(),
        data_freshness: DataFreshness::Fresh,
    };
    let station = VelibStation::new(make_reference("X3", "Test", 10)).with_real_time(rt);
    assert!(!station.is_operational());
}

// --- VelibStation::has_available_docks() ---

#[test]
fn station_has_available_docks_returns_false_without_real_time() {
    let station = VelibStation::new(make_reference("D1", "Test", 20));
    assert!(!station.has_available_docks(1));
}

#[test]
fn station_has_available_docks_exact_boundary() {
    let rt = RealTimeStatus {
        bikes: BikeAvailability::new(5, 5),
        available_docks: 3,
        status: StationStatus::Open,
        last_update: Utc::now(),
        data_freshness: DataFreshness::Fresh,
    };
    let station = VelibStation::new(make_reference("D2", "Test", 20)).with_real_time(rt);
    assert!(station.has_available_docks(3));  // exactly 3 docks available
    assert!(!station.has_available_docks(4)); // one more than available
}

// --- BikeAvailability::total() saturation ---

#[test]
fn bike_availability_total_saturates_at_u16_max() {
    let bikes = BikeAvailability::new(u16::MAX, 1);
    assert_eq!(bikes.total(), u16::MAX, "saturating_add should not overflow");
}

// --- has_available_bikes with no bikes ---

#[test]
fn station_has_available_bikes_false_when_only_wrong_type() {
    let rt = RealTimeStatus {
        bikes: BikeAvailability::new(5, 0), // only mechanical
        available_docks: 5,
        status: StationStatus::Open,
        last_update: Utc::now(),
        data_freshness: DataFreshness::Fresh,
    };
    let station = VelibStation::new(make_reference("B1", "Test", 20)).with_real_time(rt);
    assert!(!station.has_available_bikes(&BikeTypeFilter::ElectricOnly));
    assert!(station.has_available_bikes(&BikeTypeFilter::MechanicalOnly));
}
