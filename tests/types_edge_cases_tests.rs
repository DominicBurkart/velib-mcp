use chrono::Utc;
use velib_mcp::{
    BikeAvailability, BikeTypeFilter, Coordinates, DataFreshness, RealTimeStatus,
    ServiceCapabilities, StationReference, StationStatus, VelibStation,
};

// --- BikeAvailability edge cases ---

#[test]
fn bike_availability_saturating_add_on_overflow() {
    let bikes = BikeAvailability::new(u16::MAX, 1);
    assert_eq!(bikes.total(), u16::MAX); // should not wrap
}

#[test]
fn bike_availability_default_is_zero() {
    let bikes = BikeAvailability::default();
    assert_eq!(bikes.mechanical, 0);
    assert_eq!(bikes.electric, 0);
    assert!(!bikes.has_bikes());
}

#[test]
fn bike_availability_only_mechanical() {
    let bikes = BikeAvailability::new(3, 0);
    assert!(bikes.has_mechanical());
    assert!(!bikes.has_electric());
    assert!(bikes.has_bikes());
}

#[test]
fn bike_availability_only_electric() {
    let bikes = BikeAvailability::new(0, 5);
    assert!(!bikes.has_mechanical());
    assert!(bikes.has_electric());
    assert!(bikes.has_bikes());
}

// --- VelibStation::is_operational ---

#[test]
fn station_without_realtime_is_operational() {
    let station = VelibStation::new(make_ref("1", "Test", 20));
    assert!(station.is_operational());
}

#[test]
fn closed_station_is_not_operational() {
    let station = VelibStation::new(make_ref("2", "Closed", 20)).with_real_time(RealTimeStatus {
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
    let station =
        VelibStation::new(make_ref("3", "Maintenance", 20)).with_real_time(RealTimeStatus {
            bikes: BikeAvailability::new(5, 3),
            available_docks: 12,
            status: StationStatus::Maintenance,
            last_update: Utc::now(),
            data_freshness: DataFreshness::Fresh,
        });
    assert!(!station.is_operational());
}

// --- has_available_docks ---

#[test]
fn has_available_docks_true_when_enough() {
    let station = VelibStation::new(make_ref("4", "Docks", 20)).with_real_time(RealTimeStatus {
        bikes: BikeAvailability::new(5, 3),
        available_docks: 5,
        status: StationStatus::Open,
        last_update: Utc::now(),
        data_freshness: DataFreshness::Fresh,
    });
    assert!(station.has_available_docks(5));
    assert!(station.has_available_docks(1));
    assert!(!station.has_available_docks(6));
}

#[test]
fn has_available_docks_false_without_realtime() {
    let station = VelibStation::new(make_ref("5", "NoDocks", 20));
    assert!(!station.has_available_docks(1));
}

// --- has_available_bikes with filters ---

#[test]
fn has_available_bikes_false_without_realtime() {
    let station = VelibStation::new(make_ref("6", "NoBikes", 20));
    assert!(!station.has_available_bikes(&BikeTypeFilter::AnyType));
}

#[test]
fn mechanical_only_filter_needs_mechanical() {
    let station = VelibStation::new(make_ref("7", "Electric", 20)).with_real_time(RealTimeStatus {
        bikes: BikeAvailability::new(0, 5),
        available_docks: 15,
        status: StationStatus::Open,
        last_update: Utc::now(),
        data_freshness: DataFreshness::Fresh,
    });
    assert!(!station.has_available_bikes(&BikeTypeFilter::MechanicalOnly));
    assert!(station.has_available_bikes(&BikeTypeFilter::ElectricOnly));
    assert!(station.has_available_bikes(&BikeTypeFilter::AnyType));
}

// --- StationReference::validate ---

#[test]
fn validate_rejects_empty_station_code() {
    let r = StationReference {
        station_code: "".to_string(),
        name: "OK".to_string(),
        coordinates: Coordinates::new(48.85, 2.35),
        capacity: 20,
        capabilities: ServiceCapabilities::default(),
    };
    let err = r.validate().unwrap_err();
    assert!(err.contains("code"), "Got: {err}");
}

#[test]
fn validate_rejects_empty_name() {
    let r = StationReference {
        station_code: "123".to_string(),
        name: "".to_string(),
        coordinates: Coordinates::new(48.85, 2.35),
        capacity: 20,
        capabilities: ServiceCapabilities::default(),
    };
    let err = r.validate().unwrap_err();
    assert!(err.contains("name"), "Got: {err}");
}

#[test]
fn validate_rejects_zero_capacity() {
    let r = StationReference {
        station_code: "123".to_string(),
        name: "Test".to_string(),
        coordinates: Coordinates::new(48.85, 2.35),
        capacity: 0,
        capabilities: ServiceCapabilities::default(),
    };
    let err = r.validate().unwrap_err();
    assert!(err.contains("capacity"), "Got: {err}");
}

#[test]
fn validate_rejects_coords_outside_paris() {
    let r = StationReference {
        station_code: "123".to_string(),
        name: "Test".to_string(),
        coordinates: Coordinates::new(40.0, -74.0), // NYC
        capacity: 20,
        capabilities: ServiceCapabilities::default(),
    };
    assert!(r.validate().is_err());
}

// --- Serde round-trips for enums ---

#[test]
fn station_status_serde_round_trip() {
    for status in [
        StationStatus::Open,
        StationStatus::Closed,
        StationStatus::Maintenance,
    ] {
        let json = serde_json::to_string(&status).unwrap();
        let back: StationStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, status);
    }
}

#[test]
fn data_freshness_round_trip() {
    for freshness in [
        DataFreshness::Fresh,
        DataFreshness::Recent,
        DataFreshness::Stale,
        DataFreshness::VeryStale,
    ] {
        let json = serde_json::to_string(&freshness).unwrap();
        let back: DataFreshness = serde_json::from_str(&json).unwrap();
        assert_eq!(back, freshness);
    }
}

#[test]
fn bike_type_filter_round_trip() {
    for filter in [
        BikeTypeFilter::MechanicalOnly,
        BikeTypeFilter::ElectricOnly,
        BikeTypeFilter::AnyType,
    ] {
        let json = serde_json::to_string(&filter).unwrap();
        let back: BikeTypeFilter = serde_json::from_str(&json).unwrap();
        assert_eq!(back, filter);
    }
}

// --- DataFreshness boundary values ---

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
    assert_eq!(DataFreshness::from_age(-5.0), DataFreshness::Fresh);
}

// --- Coordinates::distance_to ---

#[test]
fn distance_to_self_is_zero() {
    let c = Coordinates::new(48.8566, 2.3522);
    assert!((c.distance_to(&c) - 0.0).abs() < 0.01);
}

#[test]
fn distance_is_symmetric() {
    let a = Coordinates::new(48.8566, 2.3522);
    let b = Coordinates::new(48.8606, 2.3376);
    let d1 = a.distance_to(&b);
    let d2 = b.distance_to(&a);
    assert!((d1 - d2).abs() < 0.01);
}

// --- VelibStation::validate capacity check ---

#[test]
fn validate_station_bikes_plus_docks_within_capacity() {
    let station = VelibStation::new(make_ref("ok", "OK", 20)).with_real_time(RealTimeStatus {
        bikes: BikeAvailability::new(10, 5),
        available_docks: 5,
        status: StationStatus::Open,
        last_update: Utc::now(),
        data_freshness: DataFreshness::Fresh,
    });
    assert!(station.validate().is_ok());
}

#[test]
fn validate_station_bikes_plus_docks_exceeds_capacity() {
    let station = VelibStation::new(make_ref("bad", "Bad", 10)).with_real_time(RealTimeStatus {
        bikes: BikeAvailability::new(8, 5),
        available_docks: 5,
        status: StationStatus::Open,
        last_update: Utc::now(),
        data_freshness: DataFreshness::Fresh,
    });
    assert!(station.validate().is_err());
}

// --- Helper ---

fn make_ref(code: &str, name: &str, capacity: u16) -> StationReference {
    StationReference {
        station_code: code.to_string(),
        name: name.to_string(),
        coordinates: Coordinates::new(48.8566, 2.3522),
        capacity,
        capabilities: ServiceCapabilities::default(),
    }
}
