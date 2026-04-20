//! Randomized invariant tests for `Coordinates` great-circle distance and
//! service-area checks.
//!
//! These exercise properties that should hold for every input, using
//! `fastrand` (already a project dependency) in lieu of `proptest`. Each
//! property runs over `CASES` random samples, which keeps the test fast
//! while providing meaningful coverage of the input space.

use velib_mcp::types::Coordinates;

/// Number of random cases per invariant. Small enough to stay well under a
/// second, large enough to catch regressions in the Haversine formula.
const CASES: usize = 256;

/// Haversine-on-sphere upper bound: half the Earth's circumference.
/// Earth radius used by `distance_to` is 6_371_000 m, so antipodal distance
/// is `π * R ≈ 2.0015e7 m`. We add a small numerical slack.
const ANTIPODAL_MAX_M: f64 = std::f64::consts::PI * 6_371_000.0 + 1.0;

fn random_coord(rng: &mut fastrand::Rng) -> Coordinates {
    // Full valid lat/lon domain.
    let lat = rng.f64() * 180.0 - 90.0;
    let lon = rng.f64() * 360.0 - 180.0;
    Coordinates::new(lat, lon)
}

#[test]
fn distance_to_self_is_zero() {
    let mut rng = fastrand::Rng::with_seed(0xC001_D00D);
    for _ in 0..CASES {
        let p = random_coord(&mut rng);
        let d = p.distance_to(&p);
        // Haversine at identity is analytically zero; float noise can produce
        // a tiny positive value. Allow sub-millimeter tolerance.
        assert!(
            (0.0..1e-3).contains(&d),
            "self-distance should be ~0, got {d} at {p:?}"
        );
    }
}

#[test]
fn distance_is_symmetric() {
    let mut rng = fastrand::Rng::with_seed(0xBEEF_F00D);
    for _ in 0..CASES {
        let a = random_coord(&mut rng);
        let b = random_coord(&mut rng);
        let ab = a.distance_to(&b);
        let ba = b.distance_to(&a);
        // Pure float ops are order-sensitive; allow a tiny relative error.
        let diff = (ab - ba).abs();
        let tolerance = (ab.abs().max(ba.abs()) * 1e-9).max(1e-6);
        assert!(
            diff <= tolerance,
            "asymmetric distance: d(a,b)={ab}, d(b,a)={ba}, diff={diff}, a={a:?}, b={b:?}"
        );
    }
}

#[test]
fn distance_is_non_negative_and_finite() {
    let mut rng = fastrand::Rng::with_seed(0xDEAD_BEEF);
    for _ in 0..CASES {
        let a = random_coord(&mut rng);
        let b = random_coord(&mut rng);
        let d = a.distance_to(&b);
        assert!(d.is_finite(), "distance must be finite, got {d}");
        assert!(d >= 0.0, "distance must be non-negative, got {d}");
        assert!(
            d <= ANTIPODAL_MAX_M,
            "distance exceeds antipodal bound: {d} > {ANTIPODAL_MAX_M}"
        );
    }
}

#[test]
fn distance_is_translation_symmetric_along_equator() {
    // On the equator, a displacement of Δlon degrees covers the same
    // distance regardless of absolute longitude. This catches errors where
    // latitude cosines are mishandled.
    let mut rng = fastrand::Rng::with_seed(0xFACE_CAFE);
    for _ in 0..CASES {
        let lon = rng.f64() * 360.0 - 180.0;
        let delta = rng.f64() * 10.0; // up to 10°
                                      // Guard against wrapping past ±180° for this property.
        if lon + delta > 180.0 {
            continue;
        }
        let a = Coordinates::new(0.0, lon);
        let b = Coordinates::new(0.0, lon + delta);
        let a2 = Coordinates::new(0.0, 0.0);
        let b2 = Coordinates::new(0.0, delta);
        let d1 = a.distance_to(&b);
        let d2 = a2.distance_to(&b2);
        let diff = (d1 - d2).abs();
        // Distances can be several million meters; allow mm-scale slack.
        assert!(
            diff < 1e-3,
            "equatorial Δlon={delta} should be translation-invariant, got d1={d1}, d2={d2}"
        );
    }
}

#[test]
fn service_area_is_consistent_with_distance_to_city_hall() {
    // Invariant: `is_within_paris_service_area` ⇔ `distance_to(city_hall) ≤ 50 000 m`.
    // We cover both sides of the boundary by sampling inside Europe where
    // points can straddle the 50 km radius.
    let city_hall = Coordinates::new(48.8565, 2.3514);
    let mut rng = fastrand::Rng::with_seed(0x1234_5678);
    for _ in 0..CASES {
        // Sample broadly around Paris (roughly Western Europe) so we cover
        // inside, near-boundary, and outside cases.
        let lat = 45.0 + rng.f64() * 8.0; // 45..53
        let lon = -2.0 + rng.f64() * 10.0; // -2..8
        let p = Coordinates::new(lat, lon);

        let d = p.distance_to(&city_hall);
        let flag = p.is_within_paris_service_area();
        let expected = d <= 50_000.0;
        assert_eq!(
            flag, expected,
            "service-area flag {flag} disagrees with distance {d} m at {p:?}"
        );
    }
}

#[test]
fn service_area_boundary_is_inclusive() {
    // A point at Paris City Hall itself must be in the service area (d = 0).
    let city_hall = Coordinates::new(48.8565, 2.3514);
    assert!(city_hall.is_within_paris_service_area());
    assert!(city_hall.distance_to(&city_hall) <= 50_000.0);
}

#[test]
fn distance_known_value_paris_to_london() {
    // Regression anchor: Paris (City Hall) ↔ London (Charing Cross) ≈ 344 km.
    // Allow a generous band to tolerate model choice (sphere vs ellipsoid)
    // without making this a brittle snapshot.
    let paris = Coordinates::new(48.8565, 2.3514);
    let london = Coordinates::new(51.5074, -0.1278);
    let d_km = paris.distance_to(&london) / 1000.0;
    assert!(
        (335.0..355.0).contains(&d_km),
        "Paris-London distance {d_km} km outside expected band"
    );
}
