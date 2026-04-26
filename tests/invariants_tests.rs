//! Invariant-style tests covering math and serde behavior that the existing
//! suite touches only with hand-picked single examples.
//!
//! Targets:
//!   * `Coordinates::distance_to` (symmetry, identity, triangle inequality)
//!   * `Coordinates::is_within_paris_service_area` (boundary precision against
//!     `distance_to_paris_city_hall_km`)
//!   * `GeographicBounds::contains` (degenerate point, full corner coverage)
//!   * `DataFreshness::from_age` boundary monotonicity
//!   * `RealTimeStatus::new` derives `data_freshness` from `last_update`
//!   * `JourneyPreferences::default` and `AvailabilityFilter` serde defaults
//!   * `JsonRpcRequest` `params` and `jsonrpc` `#[serde(default)]` fallbacks
//!
//! No live network access; everything runs offline against the public surface
//! re-exported from `velib_mcp::lib`.

use chrono::{Duration, Utc};
use serde_json::json;

use velib_mcp::mcp::types::{
    AvailabilityFilter, FindNearbyStationsInput, GeographicBounds, JourneyPreferences,
    JsonRpcRequest, PlanBikeJourneyInput,
};
use velib_mcp::types::{
    BikeAvailability, BikeTypeFilter, Coordinates, DataFreshness, RealTimeStatus, StationStatus,
    PARIS_CITY_HALL, PARIS_SERVICE_AREA_MAX_METERS,
};

// ---------------------------------------------------------------------------
// Coordinates::distance_to invariants
// ---------------------------------------------------------------------------

/// A handful of well-spread sample points around (and outside) Paris.
/// Hand-picked rather than randomized to keep the test deterministic without
/// a proptest dependency, while still spanning enough of the input space to
/// catch sign / haversine-formula regressions.
fn sample_points() -> Vec<Coordinates> {
    vec![
        Coordinates::new(48.8565, 2.3514),    // Paris City Hall
        Coordinates::new(48.8606, 2.3376),    // Louvre
        Coordinates::new(48.8738, 2.2950),    // Arc de Triomphe
        Coordinates::new(48.8584, 2.2945),    // Eiffel Tower
        Coordinates::new(48.7000, 2.0000),    // SW corner of Paris metro box
        Coordinates::new(49.0000, 2.6000),    // NE corner of Paris metro box
        Coordinates::new(45.7640, 4.8357),    // Lyon (far)
        Coordinates::new(51.5074, -0.1278),   // London (far)
        Coordinates::new(0.0, 0.0),           // Null Island
        Coordinates::new(-33.8688, 151.2093), // Sydney (antipodean-ish)
    ]
}

#[test]
fn distance_to_is_symmetric_for_all_sample_pairs() {
    let points = sample_points();
    for a in &points {
        for b in &points {
            let d_ab = a.distance_to(b);
            let d_ba = b.distance_to(a);
            // Allow only tiny numerical drift from reordered float ops.
            assert!(
                (d_ab - d_ba).abs() < 1e-6,
                "asymmetric distance: {a:?} -> {b:?} = {d_ab}, reverse = {d_ba}"
            );
        }
    }
}

#[test]
fn distance_from_point_to_itself_is_zero() {
    for p in sample_points() {
        let d = p.distance_to(&p);
        assert!(d < 1e-3, "{p:?} -> self distance must be ~0, got {d}");
    }
}

#[test]
fn distance_is_non_negative() {
    let points = sample_points();
    for a in &points {
        for b in &points {
            assert!(
                a.distance_to(b) >= 0.0,
                "negative distance between {a:?} and {b:?}"
            );
        }
    }
}

#[test]
fn distance_satisfies_triangle_inequality() {
    let points = sample_points();
    for a in &points {
        for b in &points {
            for c in &points {
                let d_ab = a.distance_to(b);
                let d_bc = b.distance_to(c);
                let d_ac = a.distance_to(c);
                // Allow a generous epsilon: haversine on a sphere only
                // approximates great-circle distance, and float roundoff
                // can produce tiny excess on degenerate triples.
                assert!(
                    d_ac <= d_ab + d_bc + 1e-3,
                    "triangle inequality violated: a={a:?} b={b:?} c={c:?} \
                     d(a,b)={d_ab} d(b,c)={d_bc} d(a,c)={d_ac}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Coordinates::is_within_paris_service_area boundary
// ---------------------------------------------------------------------------

#[test]
fn service_area_membership_matches_50km_threshold() {
    // For each sample point, compare the boolean predicate against the raw
    // distance. They must agree exactly (`<= 50km`).
    for p in sample_points() {
        let km = p.distance_to_paris_city_hall_km();
        let inside = p.is_within_paris_service_area();
        let expected = km <= PARIS_SERVICE_AREA_MAX_METERS / 1000.0;
        assert_eq!(
            inside, expected,
            "service-area mismatch at {p:?}: predicate={inside}, distance_km={km}"
        );
    }
}

#[test]
fn paris_city_hall_constant_is_self_consistent() {
    // The exposed constant must agree with `Coordinates::new` of the same lat/lon.
    let manual = Coordinates::new(48.8565, 2.3514);
    assert_eq!(PARIS_CITY_HALL.latitude, manual.latitude);
    assert_eq!(PARIS_CITY_HALL.longitude, manual.longitude);
    assert!(PARIS_CITY_HALL.is_within_paris_service_area());
    assert!(PARIS_CITY_HALL.distance_to_paris_city_hall_km() < 1e-6);
}

// ---------------------------------------------------------------------------
// GeographicBounds invariants
// ---------------------------------------------------------------------------

#[test]
fn degenerate_bounds_contains_only_its_single_point() {
    // North=south, east=west: a "point" bounding box.
    let bounds = GeographicBounds {
        north: 48.8565,
        south: 48.8565,
        east: 2.3514,
        west: 2.3514,
    };
    assert!(bounds.contains(&Coordinates::new(48.8565, 2.3514)));
    assert!(!bounds.contains(&Coordinates::new(48.8566, 2.3514)));
    assert!(!bounds.contains(&Coordinates::new(48.8565, 2.3515)));
}

#[test]
fn bounds_contains_all_four_corners() {
    let bounds = GeographicBounds {
        north: 48.90,
        south: 48.80,
        east: 2.40,
        west: 2.30,
    };
    let corners = [
        (bounds.north, bounds.west),
        (bounds.north, bounds.east),
        (bounds.south, bounds.west),
        (bounds.south, bounds.east),
    ];
    for (lat, lon) in corners {
        assert!(
            bounds.contains(&Coordinates::new(lat, lon)),
            "corner ({lat}, {lon}) must be inside an inclusive box"
        );
    }
}

#[test]
fn inverted_bounds_contains_nothing() {
    // If the caller passes north < south or east < west the predicate
    // degenerates to "no point can satisfy both halves of the AND". Document
    // that behavior so any future change is intentional.
    let bounds = GeographicBounds {
        north: 48.80,
        south: 48.90,
        east: 2.30,
        west: 2.40,
    };
    assert!(!bounds.contains(&Coordinates::new(48.85, 2.35)));
    assert!(!bounds.contains(&Coordinates::new(48.80, 2.30)));
}

// ---------------------------------------------------------------------------
// DataFreshness::from_age boundary monotonicity
// ---------------------------------------------------------------------------

#[test]
fn data_freshness_is_monotonic_in_age() {
    // The bucket should never get *fresher* as age increases. Encode
    // the bucket order explicitly to avoid coupling to derived `Ord`.
    fn rank(f: DataFreshness) -> u8 {
        match f {
            DataFreshness::Fresh => 0,
            DataFreshness::Recent => 1,
            DataFreshness::Stale => 2,
            DataFreshness::VeryStale => 3,
        }
    }
    let ages = [
        -100.0, 0.0, 5.0, 9.99, 10.0, 20.0, 29.99, 30.0, 60.0, 119.99, 120.0, 500.0, 100_000.0,
    ];
    let mut last = rank(DataFreshness::from_age(ages[0]));
    for age in &ages[1..] {
        let r = rank(DataFreshness::from_age(*age));
        assert!(
            r >= last,
            "freshness regressed at age={age}: rank={r}, prev={last}"
        );
        last = r;
    }
}

// ---------------------------------------------------------------------------
// RealTimeStatus::new derives data_freshness from last_update
// ---------------------------------------------------------------------------

#[test]
fn realtime_status_new_marks_recent_update_fresh() {
    let status = RealTimeStatus::new(
        BikeAvailability::new(2, 3),
        10,
        StationStatus::Open,
        Utc::now(),
    );
    assert_eq!(status.data_freshness, DataFreshness::Fresh);
}

#[test]
fn realtime_status_new_marks_old_update_very_stale() {
    let three_hours_ago = Utc::now() - Duration::hours(3);
    let status = RealTimeStatus::new(
        BikeAvailability::new(0, 0),
        0,
        StationStatus::Closed,
        three_hours_ago,
    );
    assert_eq!(status.data_freshness, DataFreshness::VeryStale);
}

#[test]
fn realtime_status_new_marks_45_minute_update_stale() {
    let earlier = Utc::now() - Duration::minutes(45);
    let status = RealTimeStatus::new(BikeAvailability::new(1, 1), 2, StationStatus::Open, earlier);
    assert_eq!(status.data_freshness, DataFreshness::Stale);
}

#[test]
fn realtime_status_new_marks_15_minute_update_recent() {
    let earlier = Utc::now() - Duration::minutes(15);
    let status = RealTimeStatus::new(BikeAvailability::new(1, 1), 2, StationStatus::Open, earlier);
    assert_eq!(status.data_freshness, DataFreshness::Recent);
}

// ---------------------------------------------------------------------------
// JourneyPreferences::default
// ---------------------------------------------------------------------------

#[test]
fn journey_preferences_default_uses_any_bike_type_and_500m_walk() {
    let prefs = JourneyPreferences::default();
    assert_eq!(prefs.bike_type, BikeTypeFilter::AnyType);
    assert_eq!(prefs.max_walk_distance, 500);
}

#[test]
fn plan_bike_journey_input_defaults_preferences_to_none() {
    // Regression: deserializing without a `preferences` field must yield None,
    // because the handler treats absent prefs as "use Default".
    let raw = json!({
        "origin": {"latitude": 48.8566, "longitude": 2.3522},
        "destination": {"latitude": 48.8606, "longitude": 2.3376}
    });
    let input: PlanBikeJourneyInput = serde_json::from_value(raw).unwrap();
    assert!(input.preferences.is_none());
}

// ---------------------------------------------------------------------------
// AvailabilityFilter serde defaults
// ---------------------------------------------------------------------------

#[test]
fn availability_filter_default_excludes_out_of_service() {
    // The `default = "default_true"` annotation on `exclude_out_of_service` is
    // a small but easy-to-break behavior. Lock it in.
    let filter: AvailabilityFilter = serde_json::from_value(json!({})).unwrap();
    assert!(filter.exclude_out_of_service);
    assert!(filter.min_bikes.is_none());
    assert!(filter.min_docks.is_none());
    assert!(filter.bike_type.is_none());
}

#[test]
fn availability_filter_caller_can_override_exclude_out_of_service() {
    let filter: AvailabilityFilter =
        serde_json::from_value(json!({"exclude_out_of_service": false})).unwrap();
    assert!(!filter.exclude_out_of_service);
}

// ---------------------------------------------------------------------------
// FindNearbyStationsInput / JsonRpcRequest serde defaults
// ---------------------------------------------------------------------------

#[test]
fn find_nearby_stations_input_uses_serde_defaults() {
    let input: FindNearbyStationsInput =
        serde_json::from_value(json!({"latitude": 48.8566, "longitude": 2.3522})).unwrap();
    assert_eq!(input.radius_meters, 500);
    assert_eq!(input.limit, 10);
    assert!(input.availability_filter.is_none());
}

#[test]
fn jsonrpc_request_jsonrpc_field_defaults_to_2_0() {
    // Both `jsonrpc` and `params` carry `#[serde(default)]`.
    let req: JsonRpcRequest =
        serde_json::from_value(json!({"id": 1, "method": "tools/list"})).unwrap();
    assert_eq!(req.jsonrpc, "2.0");
    assert!(req.params.is_null());
    assert_eq!(req.method, "tools/list");
}

#[test]
fn jsonrpc_request_round_trip_preserves_explicit_fields() {
    let original = json!({
        "jsonrpc": "2.0",
        "id": "abc",
        "method": "tools/call",
        "params": {"name": "x", "arguments": {}}
    });
    let req: JsonRpcRequest = serde_json::from_value(original.clone()).unwrap();
    assert_eq!(req.jsonrpc, "2.0");
    assert_eq!(req.method, "tools/call");
    assert_eq!(req.params, original["params"]);
}
