//! Unit tests for handler business logic and pure filtering/sorting functions.
//!
//! These tests exercise the filtering, sorting, distance calculation, and
//! coordinate validation logic used by McpToolHandler methods, all without
//! touching the real Velib API. Test data is constructed inline using the
//! exact types from src/types.rs and src/mcp/types.rs.

use chrono::Utc;
use velib_mcp::{
    mcp::types::{GeographicBounds, StationWithDistance},
    types::{
        BikeAvailability, BikeTypeFilter, Coordinates, DataFreshness, RealTimeStatus,
        ServiceCapabilities, StationReference, StationStatus, VelibStation,
    },
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a minimal valid Paris-area station with real-time data.
fn make_station(
    code: &str,
    name: &str,
    lat: f64,
    lon: f64,
    mechanical: u16,
    electric: u16,
    docks: u16,
    status: StationStatus,
) -> VelibStation {
    let reference = StationReference {
        station_code: code.to_string(),
        name: name.to_string(),
        coordinates: Coordinates::new(lat, lon),
        capacity: mechanical + electric + docks,
        capabilities: ServiceCapabilities::default(),
    };
    let real_time = RealTimeStatus {
        bikes: BikeAvailability::new(mechanical, electric),
        available_docks: docks,
        status,
        last_update: Utc::now(),
        data_freshness: DataFreshness::Fresh,
    };
    VelibStation::new(reference).with_real_time(real_time)
}

/// Build a station with no real-time data (reference-only).
fn make_reference_station(code: &str, name: &str, lat: f64, lon: f64) -> VelibStation {
    let reference = StationReference {
        station_code: code.to_string(),
        name: name.to_string(),
        coordinates: Coordinates::new(lat, lon),
        capacity: 20,
        capabilities: ServiceCapabilities::default(),
    };
    VelibStation::new(reference)
}

// ---------------------------------------------------------------------------
// Coordinate validation tests
// ---------------------------------------------------------------------------

#[test]
fn coordinates_paris_center_is_valid() {
    // Approximate geographic centre of Paris (Place de la République)
    let paris_center = Coordinates::new(48.8674, 2.3633);
    assert!(
        paris_center.is_valid_paris_metro(),
        "Paris centre should be valid for the metro area"
    );
    assert!(
        paris_center.is_within_paris_service_area(),
        "Paris centre should be within the 50 km service area"
    );
}

#[test]
fn coordinates_outside_paris_fails_validation() {
    // London, UK — clearly outside any Paris bounding box
    let london = Coordinates::new(51.5074, -0.1278);
    assert!(
        !london.is_valid_paris_metro(),
        "London coordinates should fail Paris metro validation"
    );
    assert!(
        !london.is_within_paris_service_area(),
        "London should be outside the 50 km service area"
    );
}

#[test]
fn coordinates_boundary_just_inside_paris_metro_box() {
    // The is_valid_paris_metro check uses an inclusive bounding box:
    //   lat in [48.7, 49.0], lon in [2.0, 2.6]
    // Test each corner of the box.
    let corners = [
        Coordinates::new(48.7, 2.0),
        Coordinates::new(48.7, 2.6),
        Coordinates::new(49.0, 2.0),
        Coordinates::new(49.0, 2.6),
    ];
    for c in &corners {
        assert!(
            c.is_valid_paris_metro(),
            "Corner {:?} should be valid (inclusive boundary)",
            c
        );
    }
}

#[test]
fn coordinates_boundary_just_outside_paris_metro_box() {
    let outside = [
        Coordinates::new(48.6999, 2.3),  // lat too low
        Coordinates::new(49.0001, 2.3),  // lat too high
        Coordinates::new(48.85, 1.9999), // lon too low
        Coordinates::new(48.85, 2.6001), // lon too high
    ];
    for c in &outside {
        assert!(
            !c.is_valid_paris_metro(),
            "Point {:?} should be outside Paris metro box",
            c
        );
    }
}

#[test]
fn coordinates_new_york_outside_service_area() {
    let nyc = Coordinates::new(40.7128, -74.0060);
    assert!(
        !nyc.is_valid_paris_metro(),
        "New York should fail metro validation"
    );
    assert!(
        !nyc.is_within_paris_service_area(),
        "New York should be outside the service area"
    );
}

// ---------------------------------------------------------------------------
// Distance calculation tests
// ---------------------------------------------------------------------------

#[test]
fn distance_calculation_known_pair() {
    // Eiffel Tower → Notre-Dame: roughly 4.1 km straight-line.
    // Accepted tolerance: ±200 m.
    let eiffel_tower = Coordinates::new(48.8584, 2.2945);
    let notre_dame = Coordinates::new(48.8530, 2.3499);
    let distance = eiffel_tower.distance_to(&notre_dame);
    assert!(
        (3900.0..=4300.0).contains(&distance),
        "Eiffel Tower to Notre-Dame should be ~4.1 km, got {} m",
        distance
    );
}

#[test]
fn distance_calculation_same_point_is_zero() {
    let point = Coordinates::new(48.8566, 2.3522);
    let distance = point.distance_to(&point);
    assert!(
        distance < 0.001,
        "Distance from a point to itself should be ~0, got {}",
        distance
    );
}

#[test]
fn distance_calculation_is_symmetric() {
    let a = Coordinates::new(48.8566, 2.3522);
    let b = Coordinates::new(48.8606, 2.3376);
    let ab = a.distance_to(&b);
    let ba = b.distance_to(&a);
    // Allow floating-point rounding; difference should be sub-millimetre.
    assert!(
        (ab - ba).abs() < 0.001,
        "Distance should be symmetric: {} vs {}",
        ab,
        ba
    );
}

#[test]
fn distance_calculation_short_known_pair() {
    // Hôtel de Ville → Centre Pompidou: roughly 550 m.
    // Tolerance ±100 m.
    let hotel_de_ville = Coordinates::new(48.8565, 2.3514);
    let pompidou = Coordinates::new(48.8606, 2.3522);
    let distance = hotel_de_ville.distance_to(&pompidou);
    assert!(
        (400.0..=700.0).contains(&distance),
        "Hôtel de Ville to Pompidou should be ~550 m, got {} m",
        distance
    );
}

// ---------------------------------------------------------------------------
// Name-search filtering logic (mirrors handler logic without HTTP)
// ---------------------------------------------------------------------------

/// Replicate the name-filter predicate from search_stations_by_name.
fn filter_by_name(stations: &[VelibStation], query: &str, fuzzy: bool) -> Vec<VelibStation> {
    use unicode_normalization::UnicodeNormalization;
    let query_normalized = query.to_lowercase().nfc().collect::<String>();
    let mut matching: Vec<VelibStation> = stations
        .iter()
        .filter(|s| {
            let name_normalized = s.reference.name.to_lowercase().nfc().collect::<String>();
            if fuzzy {
                name_normalized.contains(&query_normalized)
            } else {
                name_normalized.starts_with(&query_normalized)
            }
        })
        .cloned()
        .collect();
    matching.sort_by(|a, b| a.reference.name.cmp(&b.reference.name));
    matching
}

#[test]
fn search_by_name_exact_prefix_match() {
    let stations = vec![
        make_reference_station("001", "Bastille - Beaumarchais", 48.8533, 2.3692),
        make_reference_station("002", "Nation - Place de la Nation", 48.8484, 2.3961),
        make_reference_station("003", "Bastille - Opéra", 48.8529, 2.3680),
    ];

    // Non-fuzzy prefix match: only "Bastille" stations
    let results = filter_by_name(&stations, "Bastille", false);
    assert_eq!(results.len(), 2, "Should return exactly 2 Bastille stations");
    for s in &results {
        assert!(
            s.reference.name.starts_with("Bastille"),
            "Each result should start with 'Bastille'"
        );
    }
}

#[test]
fn search_by_name_case_insensitive() {
    let stations = vec![
        make_reference_station("001", "Montmartre - Abbesses", 48.8842, 2.3387),
        make_reference_station("002", "République", 48.8674, 2.3633),
    ];

    // Uppercase query should still match the lowercase-normalised name.
    let results = filter_by_name(&stations, "MONTMARTRE", false);
    assert_eq!(results.len(), 1, "Case-insensitive prefix should match");
    assert_eq!(results[0].reference.station_code, "001");
}

#[test]
fn search_by_name_fuzzy_matches_substring() {
    let stations = vec![
        make_reference_station("001", "Gare du Nord - Terminus", 48.8809, 2.3553),
        make_reference_station("002", "Gare de Lyon", 48.8448, 2.3735),
        make_reference_station("003", "Châtelet", 48.8583, 2.3470),
    ];

    // Fuzzy match on substring "Nord" should only match station 001.
    let results = filter_by_name(&stations, "Nord", true);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].reference.station_code, "001");
}

#[test]
fn search_by_name_returns_sorted_alphabetically() {
    let stations = vec![
        make_reference_station("001", "Zola - Square", 48.84, 2.30),
        make_reference_station("002", "Alésia - Didot", 48.829, 2.326),
        make_reference_station("003", "Montparnasse", 48.843, 2.319),
    ];

    // Fuzzy with empty-ish query — use "a" to match all three
    let results = filter_by_name(&stations, "a", true);
    // All three contain 'a': Zola, Alésia, Montparnasse
    assert!(!results.is_empty());
    let names: Vec<&str> = results.iter().map(|s| s.reference.name.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "Results should be sorted alphabetically");
}

#[test]
fn search_by_name_no_match_returns_empty() {
    let stations = vec![
        make_reference_station("001", "Bastille", 48.8533, 2.3692),
        make_reference_station("002", "Nation", 48.8484, 2.3961),
    ];
    let results = filter_by_name(&stations, "Versailles", true);
    assert!(results.is_empty(), "No station should match 'Versailles'");
}

// ---------------------------------------------------------------------------
// Distance-based filtering / sorting logic (mirrors find_nearby_stations)
// ---------------------------------------------------------------------------

/// Replicate the nearby-station filter+sort from the handler.
fn filter_nearby(
    stations: Vec<VelibStation>,
    query_point: Coordinates,
    radius_meters: u32,
    bike_filter: Option<&BikeTypeFilter>,
) -> Vec<StationWithDistance> {
    let mut nearby: Vec<StationWithDistance> = stations
        .into_iter()
        .filter_map(|station| {
            let distance = query_point.distance_to(&station.reference.coordinates) as u32;
            if distance <= radius_meters {
                let has_bikes = match bike_filter {
                    Some(bt) => station.has_available_bikes(bt),
                    None => true,
                };
                if has_bikes && station.is_operational() {
                    Some(StationWithDistance {
                        station,
                        straight_line_distance_meters: distance,
                    })
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();
    nearby.sort_by_key(|s| s.straight_line_distance_meters);
    nearby
}

#[test]
fn find_nearby_stations_filters_by_radius() {
    // Place station A at ~100 m and station B at ~3 km from query point.
    // Only A should appear with a 500 m radius.
    let query = Coordinates::new(48.8566, 2.3522); // Paris centre
    let close = make_station(
        "CLOSE",
        "Close Station",
        48.8570,
        2.3528,
        3,
        2,
        10,
        StationStatus::Open,
    );
    let far = make_station(
        "FAR",
        "Far Station",
        48.8800,
        2.3700,
        3,
        2,
        10,
        StationStatus::Open,
    );

    let results = filter_nearby(vec![close, far], query, 500, None);
    assert_eq!(
        results.len(),
        1,
        "Only the close station should be within 500 m"
    );
    assert_eq!(results[0].station.reference.station_code, "CLOSE");
}

#[test]
fn find_nearby_stations_returns_sorted_by_distance() {
    let query = Coordinates::new(48.8566, 2.3522);
    // Three stations at increasing distances.
    // ~50 m north
    let s1 = make_station("S1", "Near", 48.8571, 2.3522, 2, 1, 5, StationStatus::Open);
    // ~400 m north
    let s2 = make_station("S2", "Mid", 48.8602, 2.3522, 2, 1, 5, StationStatus::Open);
    // ~800 m north
    let s3 = make_station("S3", "Far", 48.8638, 2.3522, 2, 1, 5, StationStatus::Open);

    // Supply in reverse order to confirm sorting works.
    let results = filter_nearby(vec![s3, s1, s2], query, 1000, None);
    assert_eq!(results.len(), 3);
    assert_eq!(
        results[0].station.reference.station_code,
        "S1",
        "Nearest first"
    );
    assert_eq!(results[1].station.reference.station_code, "S2");
    assert_eq!(
        results[2].station.reference.station_code,
        "S3",
        "Furthest last"
    );
    // Verify distances are non-decreasing.
    let dists: Vec<u32> = results
        .iter()
        .map(|r| r.straight_line_distance_meters)
        .collect();
    assert!(
        dists.windows(2).all(|w| w[0] <= w[1]),
        "Distances should be non-decreasing"
    );
}

#[test]
fn find_nearby_stations_excludes_closed_stations() {
    let query = Coordinates::new(48.8566, 2.3522);
    let open = make_station(
        "OPEN",
        "Open Station",
        48.8570,
        2.3528,
        3,
        0,
        10,
        StationStatus::Open,
    );
    let closed = make_station(
        "CLOSED",
        "Closed Station",
        48.8572,
        2.3530,
        0,
        0,
        20,
        StationStatus::Closed,
    );

    let results = filter_nearby(vec![open, closed], query, 500, None);
    // Closed station is not operational, so it must be excluded.
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].station.reference.station_code, "OPEN");
}

#[test]
fn find_nearby_stations_bike_type_filter_mechanical_only() {
    let query = Coordinates::new(48.8566, 2.3522);
    // One station with mechanical bikes, one electric-only.
    let mech = make_station(
        "MECH",
        "Mechanical Station",
        48.8570,
        2.3528,
        5,
        0,
        10,
        StationStatus::Open,
    );
    let elec = make_station(
        "ELEC",
        "Electric Station",
        48.8572,
        2.3530,
        0,
        5,
        10,
        StationStatus::Open,
    );

    let results = filter_nearby(
        vec![mech, elec],
        query,
        500,
        Some(&BikeTypeFilter::MechanicalOnly),
    );
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].station.reference.station_code, "MECH");
}

// ---------------------------------------------------------------------------
// Area statistics aggregation logic (mirrors get_area_statistics)
// ---------------------------------------------------------------------------

/// Replicate the area-stats aggregation from the handler, applied to a slice.
fn compute_area_stats(
    all_stations: &[VelibStation],
    bounds: &GeographicBounds,
) -> (u32, u32, u32, u32, u32) {
    // Returns (total, operational, mechanical, electric, docks)
    let area: Vec<&VelibStation> = all_stations
        .iter()
        .filter(|s| bounds.contains(&s.reference.coordinates))
        .collect();

    let total = area.len() as u32;
    let operational = area.iter().filter(|s| s.is_operational()).count() as u32;
    let mut mech = 0u32;
    let mut elec = 0u32;
    let mut docks = 0u32;
    for s in &area {
        if let Some(rt) = &s.real_time {
            mech += u32::from(rt.bikes.mechanical);
            elec += u32::from(rt.bikes.electric);
            docks += u32::from(rt.available_docks);
        }
    }
    (total, operational, mech, elec, docks)
}

#[test]
fn area_statistics_empty_area_returns_zero_counts() {
    // Bounds in the middle of the Seine — no station coordinates match.
    let bounds = GeographicBounds {
        north: 48.851,
        south: 48.850,
        east: 2.352,
        west: 2.351,
    };
    let stations = vec![
        make_station(
            "001",
            "Bastille",
            48.8533,
            2.3692,
            3,
            2,
            10,
            StationStatus::Open,
        ),
        make_station(
            "002",
            "Nation",
            48.8484,
            2.3961,
            1,
            0,
            15,
            StationStatus::Open,
        ),
    ];

    let (total, operational, mech, elec, docks) = compute_area_stats(&stations, &bounds);
    assert_eq!(total, 0, "No stations should be in the empty bounds");
    assert_eq!(operational, 0);
    assert_eq!(mech, 0);
    assert_eq!(elec, 0);
    assert_eq!(docks, 0);
}

#[test]
fn area_statistics_counts_only_stations_inside_bounds() {
    let bounds = GeographicBounds {
        north: 48.870,
        south: 48.840,
        east: 2.380,
        west: 2.340,
    };
    // Two stations inside, one outside.
    let inside1 = make_station(
        "IN1",
        "Inside One",
        48.855,
        2.360,
        4,
        2,
        8,
        StationStatus::Open,
    );
    let inside2 = make_station(
        "IN2",
        "Inside Two",
        48.860,
        2.350,
        1,
        3,
        12,
        StationStatus::Open,
    );
    let outside = make_station(
        "OUT",
        "Outside",
        48.900,
        2.400,
        5,
        5,
        5,
        StationStatus::Open,
    );

    let (total, operational, mech, elec, docks) =
        compute_area_stats(&[inside1, inside2, outside], &bounds);
    assert_eq!(total, 2);
    assert_eq!(operational, 2);
    assert_eq!(mech, 5, "4+1 mechanical bikes");
    assert_eq!(elec, 5, "2+3 electric bikes");
    assert_eq!(docks, 20, "8+12 available docks");
}

#[test]
fn area_statistics_operational_count_excludes_closed() {
    let bounds = GeographicBounds {
        north: 48.870,
        south: 48.840,
        east: 2.380,
        west: 2.340,
    };
    let open = make_station("OPEN", "Open", 48.855, 2.360, 3, 1, 10, StationStatus::Open);
    let closed = make_station(
        "CLOSED",
        "Closed",
        48.860,
        2.355,
        0,
        0,
        14,
        StationStatus::Closed,
    );
    let maintenance = make_station(
        "MAINT",
        "Maintenance",
        48.862,
        2.365,
        0,
        0,
        14,
        StationStatus::Maintenance,
    );

    let (total, operational, ..) = compute_area_stats(&[open, closed, maintenance], &bounds);
    assert_eq!(total, 3, "All three stations are within bounds");
    assert_eq!(operational, 1, "Only Open is operational");
}

// ---------------------------------------------------------------------------
// GeographicBounds::contains tests
// ---------------------------------------------------------------------------

#[test]
fn geographic_bounds_contains_interior_point() {
    let bounds = GeographicBounds {
        north: 49.0,
        south: 48.7,
        east: 2.6,
        west: 2.0,
    };
    let interior = Coordinates::new(48.8566, 2.3522);
    assert!(bounds.contains(&interior));
}

#[test]
fn geographic_bounds_excludes_exterior_point() {
    let bounds = GeographicBounds {
        north: 49.0,
        south: 48.7,
        east: 2.6,
        west: 2.0,
    };
    let exterior = Coordinates::new(51.5074, -0.1278); // London
    assert!(!bounds.contains(&exterior));
}

#[test]
fn geographic_bounds_includes_boundary_points() {
    let bounds = GeographicBounds {
        north: 49.0,
        south: 48.7,
        east: 2.6,
        west: 2.0,
    };
    // All four corners should be included (inclusive comparison in contains()).
    let corners = [
        Coordinates::new(48.7, 2.0),
        Coordinates::new(48.7, 2.6),
        Coordinates::new(49.0, 2.0),
        Coordinates::new(49.0, 2.6),
    ];
    for c in &corners {
        assert!(
            bounds.contains(c),
            "Boundary corner {:?} should be contained",
            c
        );
    }
}

// ---------------------------------------------------------------------------
// VelibStation method tests
// ---------------------------------------------------------------------------

#[test]
fn station_is_operational_when_open() {
    let s = make_station("X", "Test", 48.856, 2.352, 1, 0, 5, StationStatus::Open);
    assert!(s.is_operational());
}

#[test]
fn station_is_not_operational_when_closed() {
    let s = make_station("X", "Test", 48.856, 2.352, 0, 0, 10, StationStatus::Closed);
    assert!(!s.is_operational());
}

#[test]
fn station_without_realtime_is_assumed_operational() {
    let s = make_reference_station("X", "Test", 48.856, 2.352);
    assert!(
        s.is_operational(),
        "Station without real-time data should be assumed operational"
    );
}

#[test]
fn station_has_available_docks_checks_threshold() {
    let s = make_station("X", "Test", 48.856, 2.352, 2, 2, 5, StationStatus::Open);
    assert!(s.has_available_docks(1));
    assert!(s.has_available_docks(5));
    assert!(!s.has_available_docks(6), "Only 5 docks, should not have 6");
}
