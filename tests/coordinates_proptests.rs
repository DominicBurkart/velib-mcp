//! Property-based tests for `Coordinates` geometry.
//!
//! `Coordinates::distance_to` implements the Haversine formula. Existing tests
//! in `src/types.rs` and `tests/types_edge_cases_tests.rs` cover only a handful
//! of fixed point pairs (Paris/Louvre, Paris/NYC, a doctest, and a single
//! reverse-direction sanity check). The metric invariants of distance --
//! identity, symmetry, non-negativity, boundedness, triangle inequality -- are
//! never asserted, so silent regressions in the formula (units, sign of the
//! latitude/longitude swap, missing `.cos()`, wrong earth radius) could pass.
//!
//! These property tests fix that gap by sampling latitude/longitude broadly
//! and asserting the metric laws plus the consistency between
//! `is_within_paris_service_area` and `distance_to_paris_city_hall_km`.

use proptest::prelude::*;
use velib_mcp::types::{Coordinates, PARIS_CITY_HALL, PARIS_SERVICE_AREA_MAX_METERS};

/// Earth circumference (meters), using the same radius (6,371,000 m) the
/// implementation uses. Distance between any two points must be <= half this.
const EARTH_HALF_CIRCUMFERENCE_M: f64 = std::f64::consts::PI * 6_371_000.0;

/// Float tolerance for Haversine round-trips. The formula uses `sin`, `cos`,
/// `atan2`, and `sqrt`, each carrying a few ULPs of error; for distances on
/// the order of Earth's circumference (~2e7 m) we comfortably allow 1 mm.
const TOLERANCE_M: f64 = 1e-3;

/// Generator for any valid WGS84 coordinate (latitude in [-90, 90], longitude
/// in [-180, 180]). Bounded so the Haversine inputs are well-defined; we
/// exclude the poles (±90) by epsilon to avoid degenerate longitude behavior
/// where every meridian collapses to the same point.
fn arb_coords() -> impl Strategy<Value = Coordinates> {
    (-89.999_999_f64..=89.999_999_f64, -180.0_f64..=180.0_f64)
        .prop_map(|(lat, lon)| Coordinates::new(lat, lon))
}

proptest! {
    /// d(p, p) == 0 for every coordinate. Catches accidental sign or unit
    /// changes that would yield non-zero self-distance.
    #[test]
    fn distance_to_self_is_zero(p in arb_coords()) {
        let d = p.distance_to(&p);
        prop_assert!(
            d.abs() < TOLERANCE_M,
            "distance_to_self should be 0, got {d} for ({}, {})",
            p.latitude,
            p.longitude
        );
    }

    /// d(a, b) == d(b, a). Haversine is symmetric in its arguments by
    /// construction; if a refactor ever swaps `lat1`/`lat2` asymmetrically,
    /// this catches it.
    #[test]
    fn distance_is_symmetric(a in arb_coords(), b in arb_coords()) {
        let d_ab = a.distance_to(&b);
        let d_ba = b.distance_to(&a);
        prop_assert!(
            (d_ab - d_ba).abs() < TOLERANCE_M,
            "asymmetric distance: d(a,b)={d_ab}, d(b,a)={d_ba}"
        );
    }

    /// d(a, b) >= 0 and finite. Negative or NaN distances would silently
    /// poison every downstream filter (`find_stations_within_radius`,
    /// `ensure_in_service_area`, journey scoring) so this is the most
    /// load-bearing invariant.
    #[test]
    fn distance_is_non_negative_and_finite(a in arb_coords(), b in arb_coords()) {
        let d = a.distance_to(&b);
        prop_assert!(d.is_finite(), "non-finite distance: {d}");
        prop_assert!(d >= 0.0, "negative distance: {d}");
    }

    /// d(a, b) <= π * R. The maximum geodesic distance on a sphere of radius
    /// R is half its circumference (antipodes). A small slack covers float
    /// rounding; a regression that drops the `0.5` factor in the formula or
    /// uses 2*R*atan2 would push results well past this bound.
    #[test]
    fn distance_is_bounded_by_half_circumference(a in arb_coords(), b in arb_coords()) {
        let d = a.distance_to(&b);
        prop_assert!(
            d <= EARTH_HALF_CIRCUMFERENCE_M + 1.0,
            "distance {d} exceeds half-circumference {EARTH_HALF_CIRCUMFERENCE_M}"
        );
    }

    /// Triangle inequality: d(a, c) <= d(a, b) + d(b, c). On a sphere the
    /// geodesic distance satisfies this; we allow a tiny tolerance to absorb
    /// float error accumulated across three Haversine calls.
    #[test]
    fn distance_satisfies_triangle_inequality(
        a in arb_coords(),
        b in arb_coords(),
        c in arb_coords(),
    ) {
        let d_ac = a.distance_to(&c);
        let d_ab = a.distance_to(&b);
        let d_bc = b.distance_to(&c);
        prop_assert!(
            d_ac <= d_ab + d_bc + TOLERANCE_M,
            "triangle inequality violated: d(a,c)={d_ac} > d(a,b)+d(b,c)={}",
            d_ab + d_bc
        );
    }

    /// `is_within_paris_service_area` must agree with
    /// `distance_to_paris_city_hall_km`: the boolean is just the threshold
    /// applied to the kilometer reading. This catches drift between the two
    /// helpers (e.g. one updated to a new center, the other not, or
    /// km/m unit confusion).
    #[test]
    fn service_area_check_matches_distance_threshold(p in arb_coords()) {
        let inside = p.is_within_paris_service_area();
        let km = p.distance_to_paris_city_hall_km();
        let expected = km * 1000.0 <= PARIS_SERVICE_AREA_MAX_METERS;
        prop_assert_eq!(
            inside,
            expected,
            "service-area check disagrees with distance: inside={}, km={}, max_m={}",
            inside,
            km,
            PARIS_SERVICE_AREA_MAX_METERS
        );
    }

    /// `distance_to_paris_city_hall_km` must equal `distance_to(&PARIS_CITY_HALL) / 1000`.
    /// Trivial relationship, but the helper exists specifically to centralize
    /// the unit conversion used by `OutsideServiceArea` errors -- a regression
    /// that swaps km/m here corrupts every error message and the radius check.
    #[test]
    fn km_helper_matches_meter_distance_divided_by_1000(p in arb_coords()) {
        let km = p.distance_to_paris_city_hall_km();
        let m = p.distance_to(&PARIS_CITY_HALL);
        prop_assert!(
            (km - m / 1000.0).abs() < 1e-6,
            "km helper drifted: km={km}, m/1000={}",
            m / 1000.0
        );
    }
}
