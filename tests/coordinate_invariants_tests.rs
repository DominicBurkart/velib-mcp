//! Invariant tests for `Coordinates::distance_to` (Haversine) and
//! `DataFreshness::from_age`.
//!
//! These behaviors sit under every geospatial handler and every freshness
//! label the server emits, yet their contracts (symmetry, self-distance
//! zero, triangle inequality, monotonic freshness, boundary classification)
//! were not exercised by any existing test. A regression here would silently
//! corrupt `find_nearby_stations` ordering or mislabel cached data.

use velib_mcp::types::{
    BikeAvailability, Coordinates, DataFreshness, PARIS_CITY_HALL, PARIS_SERVICE_AREA_MAX_METERS,
};

// --- helpers -----------------------------------------------------------------

/// Compare two f64 distances with an absolute tolerance in meters.
fn approx_eq_m(a: f64, b: f64, tol_m: f64) -> bool {
    (a - b).abs() <= tol_m
}

/// A handful of well-known Paris-area points. Using real landmarks keeps the
/// assertions meaningful: if distances suddenly diverge wildly, the Haversine
/// implementation has broken.
fn paris_landmarks() -> Vec<(&'static str, Coordinates)> {
    vec![
        ("city_hall", Coordinates::new(48.8565, 2.3514)),
        ("louvre", Coordinates::new(48.8606, 2.3376)),
        ("notre_dame", Coordinates::new(48.8530, 2.3499)),
        ("eiffel", Coordinates::new(48.8584, 2.2945)),
        ("montmartre", Coordinates::new(48.8867, 2.3431)),
        ("gare_de_lyon", Coordinates::new(48.8443, 2.3743)),
    ]
}

// --- Coordinates::distance_to: fundamental metric axioms --------------------

#[test]
fn distance_to_self_is_zero() {
    for (_name, c) in paris_landmarks() {
        let d = c.distance_to(&c);
        assert!(
            d.abs() < 1e-6,
            "distance from a point to itself must be 0, got {d}"
        );
    }
}

#[test]
fn distance_is_symmetric() {
    let points = paris_landmarks();
    for (na, a) in &points {
        for (nb, b) in &points {
            let d_ab = a.distance_to(b);
            let d_ba = b.distance_to(a);
            assert!(
                approx_eq_m(d_ab, d_ba, 1e-6),
                "distance asymmetric between {na} and {nb}: {d_ab} vs {d_ba}"
            );
        }
    }
}

#[test]
fn distance_is_non_negative() {
    let points = paris_landmarks();
    for (_, a) in &points {
        for (_, b) in &points {
            let d = a.distance_to(b);
            assert!(d >= 0.0, "distance must be non-negative, got {d}");
            assert!(d.is_finite(), "distance must be finite, got {d}");
        }
    }
}

#[test]
fn distance_triangle_inequality_holds_for_landmark_triples() {
    // Haversine is a metric on the sphere, so d(a,c) <= d(a,b) + d(b,c)
    // must hold for every triple.
    let points = paris_landmarks();
    // 1e-3 m tolerance to absorb f64 rounding over ~10 km distances.
    let tol = 1e-3;
    for (_, a) in &points {
        for (_, b) in &points {
            for (_, c) in &points {
                let d_ac = a.distance_to(c);
                let d_ab = a.distance_to(b);
                let d_bc = b.distance_to(c);
                assert!(
                    d_ac <= d_ab + d_bc + tol,
                    "triangle inequality violated: d(a,c)={d_ac}, d(a,b)+d(b,c)={}",
                    d_ab + d_bc
                );
            }
        }
    }
}

#[test]
fn distance_city_hall_to_louvre_matches_known_value() {
    // City Hall -> Louvre is ~1.3 km. Guards against drift in Earth-radius
    // constants or angle-unit bugs (degree vs radian).
    let city_hall = Coordinates::new(48.8565, 2.3514);
    let louvre = Coordinates::new(48.8606, 2.3376);
    let d = city_hall.distance_to(&louvre);
    assert!(
        (900.0..=1700.0).contains(&d),
        "city_hall->louvre should be ~1.3km, got {d} m"
    );
}

#[test]
fn distance_scales_with_latitude_displacement() {
    // Moving 0.01 degrees latitude north of City Hall is ~1.11 km; moving
    // 0.001 degrees is ~111 m. Distances should scale roughly linearly.
    let origin = PARIS_CITY_HALL;
    let near = Coordinates::new(origin.latitude + 0.001, origin.longitude);
    let far = Coordinates::new(origin.latitude + 0.01, origin.longitude);
    let d_near = origin.distance_to(&near);
    let d_far = origin.distance_to(&far);
    // Ratio should be close to 10x (small-angle approximation on a sphere).
    let ratio = d_far / d_near;
    assert!(
        (9.0..=11.0).contains(&ratio),
        "expected ~10x scaling, got {ratio} (near={d_near}, far={d_far})"
    );
}

// --- is_within_paris_service_area: boundary consistency --------------------

#[test]
fn service_area_matches_haversine_threshold() {
    // For any point, is_within_paris_service_area must agree with
    // `distance_to(PARIS_CITY_HALL) <= PARIS_SERVICE_AREA_MAX_METERS`.
    let samples = [
        Coordinates::new(48.8565, 2.3514),  // city hall itself
        Coordinates::new(48.8566, 2.3522),  // Paris center
        Coordinates::new(49.2, 2.3514),     // ~38 km north
        Coordinates::new(50.0, 2.3514),     // ~130 km north
        Coordinates::new(51.5074, -0.1278), // London
    ];
    for c in samples {
        let d = c.distance_to(&PARIS_CITY_HALL);
        let within_by_fn = c.is_within_paris_service_area();
        let within_by_math = d <= PARIS_SERVICE_AREA_MAX_METERS;
        assert_eq!(
            within_by_fn, within_by_math,
            "service-area classification disagrees at {c:?}: d={d}, fn={within_by_fn}, math={within_by_math}"
        );
    }
}

#[test]
fn distance_to_paris_city_hall_km_is_meters_over_1000() {
    // Unit conversion invariant: km == m / 1000.
    let samples = [
        Coordinates::new(48.8566, 2.3522),
        Coordinates::new(49.0, 2.5),
        Coordinates::new(50.0, 2.3514),
    ];
    for c in samples {
        let m = c.distance_to(&PARIS_CITY_HALL);
        let km = c.distance_to_paris_city_hall_km();
        assert!(
            approx_eq_m(km * 1000.0, m, 1e-3),
            "km/m mismatch at {c:?}: km={km}, m={m}"
        );
    }
}

// --- DataFreshness::from_age: boundary & monotonicity ----------------------

#[test]
fn data_freshness_boundary_values_classify_correctly() {
    // The thresholds used by the implementation are < 10, < 30, < 120.
    // Just below each boundary must be the sharper class; exactly at the
    // boundary must tip into the next class.
    assert_eq!(DataFreshness::from_age(0.0), DataFreshness::Fresh);
    assert_eq!(DataFreshness::from_age(9.999), DataFreshness::Fresh);
    assert_eq!(DataFreshness::from_age(10.0), DataFreshness::Recent);
    assert_eq!(DataFreshness::from_age(29.999), DataFreshness::Recent);
    assert_eq!(DataFreshness::from_age(30.0), DataFreshness::Stale);
    assert_eq!(DataFreshness::from_age(119.999), DataFreshness::Stale);
    assert_eq!(DataFreshness::from_age(120.0), DataFreshness::VeryStale);
    assert_eq!(DataFreshness::from_age(10_000.0), DataFreshness::VeryStale);
}

#[test]
fn data_freshness_is_monotonic_in_age() {
    // As age increases, freshness may only stay the same or get worse.
    // Encode the ordering: Fresh < Recent < Stale < VeryStale.
    fn rank(f: DataFreshness) -> u8 {
        match f {
            DataFreshness::Fresh => 0,
            DataFreshness::Recent => 1,
            DataFreshness::Stale => 2,
            DataFreshness::VeryStale => 3,
        }
    }

    let ages = [
        -5.0, 0.0, 1.0, 5.0, 9.9, 10.0, 15.0, 29.9, 30.0, 60.0, 119.9, 120.0, 300.0, 1_000.0,
    ];
    for window in ages.windows(2) {
        let a = window[0];
        let b = window[1];
        let ra = rank(DataFreshness::from_age(a));
        let rb = rank(DataFreshness::from_age(b));
        assert!(
            ra <= rb,
            "freshness not monotonic: age {a} -> rank {ra}, age {b} -> rank {rb}"
        );
    }
}

#[test]
fn data_freshness_negative_age_treated_as_fresh() {
    // Clock skew can produce a slightly-future `last_update`, yielding
    // a negative age. That data is logically fresher than anything, so
    // it must classify as Fresh (not fall through to VeryStale).
    assert_eq!(DataFreshness::from_age(-1.0), DataFreshness::Fresh);
    assert_eq!(DataFreshness::from_age(-3600.0), DataFreshness::Fresh);
}

#[test]
fn data_freshness_nan_and_infinity_do_not_panic() {
    // These inputs are unlikely, but the function takes an f64 — it must
    // at least produce a well-formed variant rather than panic.
    let _ = DataFreshness::from_age(f64::NAN);
    let _ = DataFreshness::from_age(f64::INFINITY);
    // +infinity is definitely "very stale".
    assert_eq!(
        DataFreshness::from_age(f64::INFINITY),
        DataFreshness::VeryStale
    );
}

// --- BikeAvailability: saturating total invariant ---------------------------

#[test]
fn bike_availability_total_saturates_on_overflow() {
    // `total()` uses saturating_add, so u16::MAX + any u16 must cap at u16::MAX
    // rather than wrapping to a small number. Wrapping here would silently
    // break any "has bikes" filter in the handlers.
    let max = BikeAvailability::new(u16::MAX, u16::MAX);
    assert_eq!(max.total(), u16::MAX);
    assert!(max.has_bikes());

    let almost = BikeAvailability::new(u16::MAX - 3, 10);
    assert_eq!(almost.total(), u16::MAX);
}

#[test]
fn bike_availability_total_equals_sum_when_no_overflow() {
    for (m, e) in [(0u16, 0u16), (1, 1), (5, 3), (100, 200), (1000, 2000)] {
        let b = BikeAvailability::new(m, e);
        assert_eq!(u32::from(b.total()), u32::from(m) + u32::from(e));
        assert_eq!(b.has_bikes(), (m + e) > 0);
        assert_eq!(b.has_mechanical(), m > 0);
        assert_eq!(b.has_electric(), e > 0);
    }
}
