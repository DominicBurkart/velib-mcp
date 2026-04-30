//! Invariant tests for `Coordinates` validation and distance helpers.
//!
//! Disposition before this file: `src/types.rs::tests` and
//! `tests/types_edge_cases_tests.rs` cover happy-path Paris coordinates and
//! a few clearly-far points (NYC, London, Reims, Lyon). They do not cover
//! the exact bounding-box edges, NaN/infinite inputs, distance-metric
//! properties, or the structural relationship between the two validation
//! stages that `ensure_in_service_area` relies on.
//!
//! These tests use only deterministic fixed-point inputs and pure
//! computations -- no network, no time, no sleeps.

use velib_mcp::mcp::types::JourneyPreferences;
use velib_mcp::types::{
    BikeTypeFilter, Coordinates, PARIS_CITY_HALL, PARIS_SERVICE_AREA_MAX_METERS,
};

// -----------------------------------------------------------------------------
// `is_valid_paris_metro` bounding-box edges.
//
// The function uses the inclusive box 47.0 <= lat <= 50.5, 0.0 <= lon <= 5.0.
// These tests pin that contract so a future refactor cannot silently shrink or
// shift the box without an explicit test update.
// -----------------------------------------------------------------------------

#[test]
fn bounding_box_corners_are_inclusive() {
    // All four corners must be considered "inside" because the comparison
    // operators in `is_valid_paris_metro` are <=/>=.
    let nw = Coordinates::new(50.5, 0.0);
    let ne = Coordinates::new(50.5, 5.0);
    let sw = Coordinates::new(47.0, 0.0);
    let se = Coordinates::new(47.0, 5.0);
    assert!(nw.is_valid_paris_metro(), "NW corner must be inside bbox");
    assert!(ne.is_valid_paris_metro(), "NE corner must be inside bbox");
    assert!(sw.is_valid_paris_metro(), "SW corner must be inside bbox");
    assert!(se.is_valid_paris_metro(), "SE corner must be inside bbox");
}

#[test]
fn bounding_box_just_outside_each_edge_is_rejected() {
    // Move one ULP-ish past each edge in the rejecting direction.
    // Using 1e-6 (~11cm in latitude) keeps inputs realistic.
    let too_north = Coordinates::new(50.5 + 1e-6, 2.5);
    let too_south = Coordinates::new(47.0 - 1e-6, 2.5);
    let too_east = Coordinates::new(48.5, 5.0 + 1e-6);
    let too_west = Coordinates::new(48.5, 0.0 - 1e-6);
    assert!(!too_north.is_valid_paris_metro());
    assert!(!too_south.is_valid_paris_metro());
    assert!(!too_east.is_valid_paris_metro());
    assert!(!too_west.is_valid_paris_metro());
}

#[test]
fn nan_coordinates_are_rejected() {
    // Any comparison against NaN yields false in IEEE 754, which means the
    // bounding-box check correctly rejects NaN. Lock that in: a regression
    // that swapped to `!(lat < 47.0)` style logic would silently accept NaN.
    let nan_lat = Coordinates::new(f64::NAN, 2.35);
    let nan_lon = Coordinates::new(48.85, f64::NAN);
    let both_nan = Coordinates::new(f64::NAN, f64::NAN);
    assert!(!nan_lat.is_valid_paris_metro());
    assert!(!nan_lon.is_valid_paris_metro());
    assert!(!both_nan.is_valid_paris_metro());
}

#[test]
fn infinite_coordinates_are_rejected() {
    let inf_lat = Coordinates::new(f64::INFINITY, 2.35);
    let neg_inf_lat = Coordinates::new(f64::NEG_INFINITY, 2.35);
    let inf_lon = Coordinates::new(48.85, f64::INFINITY);
    let neg_inf_lon = Coordinates::new(48.85, f64::NEG_INFINITY);
    assert!(!inf_lat.is_valid_paris_metro());
    assert!(!neg_inf_lat.is_valid_paris_metro());
    assert!(!inf_lon.is_valid_paris_metro());
    assert!(!neg_inf_lon.is_valid_paris_metro());
}

// -----------------------------------------------------------------------------
// Service-area radius vs. bounding box.
//
// `ensure_in_service_area` first checks the broad bounding box, then the
// 50km radius. For that staged check to make sense the 50km ball around
// Paris City Hall must lie *entirely* inside the broad bbox. Otherwise
// there could be coordinates that pass `is_within_paris_service_area` but
// fail `is_valid_paris_metro`, which would mean the order of checks is
// observable to callers.
// -----------------------------------------------------------------------------

#[test]
fn service_area_implies_paris_metro_bbox_on_cardinal_extremes() {
    // Sample the four cardinal extremes near the edge of the 50km circle.
    // These offsets are well under 0.5 degrees which keeps them inside the
    // broad bbox even after any plausible service-area expansion.
    //
    // Each point is chosen to be within ~49 km of City Hall (so it must be
    // service-area-valid) and we then assert the bbox check also passes.
    //
    // Latitude offset for ~49 km north/south: 49 / 111.32 ≈ 0.4401 deg.
    // Longitude offset for ~49 km east/west at lat 48.8565:
    //   49 / (111.32 * cos(48.8565°)) ≈ 0.6695 deg.
    let north = Coordinates::new(PARIS_CITY_HALL.latitude + 0.44, PARIS_CITY_HALL.longitude);
    let south = Coordinates::new(PARIS_CITY_HALL.latitude - 0.44, PARIS_CITY_HALL.longitude);
    let east = Coordinates::new(PARIS_CITY_HALL.latitude, PARIS_CITY_HALL.longitude + 0.66);
    let west = Coordinates::new(PARIS_CITY_HALL.latitude, PARIS_CITY_HALL.longitude - 0.66);

    for (label, coord) in [
        ("north", north),
        ("south", south),
        ("east", east),
        ("west", west),
    ] {
        assert!(
            coord.is_within_paris_service_area(),
            "{label} sample (dist={:.1}km) must be inside 50km service area",
            coord.distance_to_paris_city_hall_km()
        );
        assert!(
            coord.is_valid_paris_metro(),
            "{label} sample must also be inside the bbox -- otherwise the staged \
             validation in ensure_in_service_area would be order-dependent"
        );
    }
}

#[test]
fn service_area_max_meters_constant_matches_assertion() {
    // City Hall must lie on the service-area boundary at distance 0, which is
    // far less than the 50km maximum. This test pins both the constant value
    // and the unit (meters, not kilometers).
    assert_eq!(PARIS_SERVICE_AREA_MAX_METERS, 50_000.0);
    assert!(PARIS_CITY_HALL.distance_to(&PARIS_CITY_HALL) < PARIS_SERVICE_AREA_MAX_METERS);
}

// -----------------------------------------------------------------------------
// `distance_to` metric properties.
//
// We use a small fixed sample of Paris-area landmarks and check the standard
// metric axioms. These are not exhaustive proofs; they are guard-rails that
// would catch a regression that, say, swapped lat/lon or dropped the
// `cos(lat)` term in the Haversine formula.
// -----------------------------------------------------------------------------

fn paris_sample() -> Vec<(&'static str, Coordinates)> {
    vec![
        ("city_hall", Coordinates::new(48.8565, 2.3514)),
        ("louvre", Coordinates::new(48.8606, 2.3376)),
        ("eiffel_tower", Coordinates::new(48.8584, 2.2945)),
        ("gare_du_nord", Coordinates::new(48.8809, 2.3553)),
        ("montmartre", Coordinates::new(48.8867, 2.3431)),
    ]
}

#[test]
fn distance_identity_is_zero() {
    for (name, c) in paris_sample() {
        let d = c.distance_to(&c);
        // Floating point sloppiness: assert sub-millimeter rather than == 0.0.
        assert!(d < 1e-3, "{name} self-distance should be ~0, got {d}");
    }
}

#[test]
fn distance_is_symmetric() {
    let sample = paris_sample();
    for (na, a) in &sample {
        for (nb, b) in &sample {
            let d_ab = a.distance_to(b);
            let d_ba = b.distance_to(a);
            // Allow 1mm of float tolerance.
            assert!(
                (d_ab - d_ba).abs() < 1e-3,
                "asymmetry {na}->{nb}: {d_ab} vs {d_ba}"
            );
        }
    }
}

#[test]
fn distance_is_non_negative() {
    let sample = paris_sample();
    for (na, a) in &sample {
        for (nb, b) in &sample {
            let d = a.distance_to(b);
            assert!(d >= 0.0, "negative distance {na}->{nb}: {d}");
        }
    }
}

#[test]
fn distance_triangle_inequality_on_sample() {
    // For every triple from the sample, d(a,c) <= d(a,b) + d(b,c) up to
    // a small tolerance to absorb spherical-geometry float jitter.
    let sample = paris_sample();
    for (_, a) in &sample {
        for (_, b) in &sample {
            for (_, c) in &sample {
                let d_ac = a.distance_to(c);
                let d_ab = a.distance_to(b);
                let d_bc = b.distance_to(c);
                // Allow 1mm slack.
                assert!(
                    d_ac <= d_ab + d_bc + 1e-3,
                    "triangle inequality violated: d(a,c)={d_ac} > d(a,b)+d(b,c)={}",
                    d_ab + d_bc
                );
            }
        }
    }
}

// -----------------------------------------------------------------------------
// `distance_to_paris_city_hall_km` consistency.
//
// This helper exists so handlers can construct `OutsideServiceArea` errors
// without recomputing the distance manually. It must agree with
// `distance_to(&PARIS_CITY_HALL) / 1000.0`.
// -----------------------------------------------------------------------------

#[test]
fn distance_to_city_hall_km_equals_meters_distance_divided_by_1000() {
    for (name, c) in paris_sample() {
        let km = c.distance_to_paris_city_hall_km();
        let expected_km = c.distance_to(&PARIS_CITY_HALL) / 1000.0;
        assert!(
            (km - expected_km).abs() < 1e-9,
            "{name}: km helper {km} disagrees with meters/1000 {expected_km}"
        );
    }
}

#[test]
fn distance_to_city_hall_km_at_origin_is_zero() {
    let km = PARIS_CITY_HALL.distance_to_paris_city_hall_km();
    assert!(km < 1e-6, "City Hall to itself must be ~0 km, got {km}");
}

// -----------------------------------------------------------------------------
// Coordinates serde round-trip.
//
// `Coordinates` appears as input in `plan_bike_journey` (origin/destination)
// and in many output structs. Lock in the JSON shape: { latitude, longitude }
// as f64 fields. A change to (e.g.) tuple-form serialization would silently
// break every published tool input schema.
// -----------------------------------------------------------------------------

#[test]
fn coordinates_serialize_to_named_fields() {
    let c = Coordinates::new(48.8566, 2.3522);
    let v = serde_json::to_value(c).unwrap();
    assert_eq!(v["latitude"], 48.8566);
    assert_eq!(v["longitude"], 2.3522);
    // No extra keys.
    assert_eq!(v.as_object().unwrap().len(), 2);
}

#[test]
fn coordinates_round_trip_preserves_values() {
    let original = Coordinates::new(48.8566, 2.3522);
    let json = serde_json::to_string(&original).unwrap();
    let back: Coordinates = serde_json::from_str(&json).unwrap();
    assert_eq!(original, back);
}

#[test]
fn coordinates_deserialize_rejects_missing_fields() {
    let missing_lon = serde_json::from_str::<Coordinates>(r#"{"latitude": 48.8566}"#);
    let missing_lat = serde_json::from_str::<Coordinates>(r#"{"longitude": 2.3522}"#);
    assert!(missing_lon.is_err());
    assert!(missing_lat.is_err());
}

// -----------------------------------------------------------------------------
// `JourneyPreferences::default()` -- the fallback used by `plan_bike_journey`
// when callers omit `preferences`. The numeric default leaks into the
// confidence-score ratio (`pickup_walk / max_walk`) and the search radius for
// pickup/dropoff stations, so a silent change here would change recommended
// journeys.
// -----------------------------------------------------------------------------

#[test]
fn journey_preferences_default_is_any_bike_500m_walk() {
    let prefs = JourneyPreferences::default();
    assert_eq!(prefs.bike_type, BikeTypeFilter::AnyType);
    assert_eq!(prefs.max_walk_distance, 500);
}

#[test]
fn journey_preferences_default_walk_distance_is_within_max_search_radius() {
    // The handler caps `find_nearby_stations` radius at MAX_SEARCH_RADIUS = 5000.
    // The default walk distance is reused as the radius for pickup/dropoff
    // searches in `plan_bike_journey`, so it must stay within that cap.
    let prefs = JourneyPreferences::default();
    assert!(
        prefs.max_walk_distance <= 5000,
        "default walk {} would exceed the MAX_SEARCH_RADIUS cap",
        prefs.max_walk_distance
    );
}
