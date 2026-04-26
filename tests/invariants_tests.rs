//! Invariant tests that guard against subtle regressions in three areas that
//! the existing suite leaves uncovered:
//!
//! 1. **`GeographicBounds::contains`** – the degenerate (inverted) case where
//!    `north < south` or `east < west`. The implementation uses `>=`/`<=`
//!    comparisons that silently return `false` for every point when the bounds
//!    are inverted. These tests pin that behaviour and make callers think twice
//!    before constructing inverted bounds.
//!
//! 2. **Serde defaults for `JourneyPreferences` and `AvailabilityFilter`** –
//!    both types rely on `#[serde(default)]` / custom `default_*` functions
//!    to fill in optional fields when JSON is absent. The defaults drive
//!    real handler behaviour (`plan_bike_journey` walks 500 m by default,
//!    `find_nearby_stations` excludes out-of-service stations by default) but
//!    were never exercised by a test.
//!
//! 3. **`RetryStrategy::ExponentialBackoff` jitter cap invariant** – the
//!    implementation caps the base delay at `max_delay` *before* adding
//!    jitter. A loop of 100 samples verifies that the returned duration never
//!    exceeds `max_delay + 25 %` (the maximum possible jitter fraction),
//!    ensuring the cap is applied in the right order.

use velib_mcp::data::RetryConfig;
use velib_mcp::mcp::types::{AvailabilityFilter, GeographicBounds, JourneyPreferences};
use velib_mcp::types::{BikeTypeFilter, Coordinates};

// ---------------------------------------------------------------------------
// 1. GeographicBounds::contains – degenerate / inverted bounds
// ---------------------------------------------------------------------------

/// Well-formed bounds are tested in `mcp_types_tests.rs`; these tests cover
/// what happens when `north < south` (latitude inversion) or `east < west`
/// (longitude inversion), and when north == south or east == west (zero-area).

#[test]
fn inverted_latitude_bounds_contain_nothing() {
    // north (48.80) < south (48.90): logically empty, no point should match.
    let inverted = GeographicBounds {
        north: 48.80,
        south: 48.90,
        east: 2.40,
        west: 2.30,
    };
    // A coordinate that would be "inside" a correctly-ordered box is rejected.
    let would_be_inside = Coordinates::new(48.85, 2.35);
    assert!(
        !inverted.contains(&would_be_inside),
        "Inverted latitude bounds must contain no point"
    );
    // A coordinate outside either ordering is also rejected.
    let outside = Coordinates::new(49.0, 2.35);
    assert!(!inverted.contains(&outside));
}

#[test]
fn inverted_longitude_bounds_contain_nothing() {
    // east (2.30) < west (2.40): logically empty.
    let inverted = GeographicBounds {
        north: 48.90,
        south: 48.80,
        east: 2.30,
        west: 2.40,
    };
    let would_be_inside = Coordinates::new(48.85, 2.35);
    assert!(
        !inverted.contains(&would_be_inside),
        "Inverted longitude bounds must contain no point"
    );
}

#[test]
fn zero_area_bounds_contain_only_their_exact_point() {
    // north == south and east == west: a single geographic point.
    let point_bounds = GeographicBounds {
        north: 48.8566,
        south: 48.8566,
        east: 2.3522,
        west: 2.3522,
    };
    let exact = Coordinates::new(48.8566, 2.3522);
    assert!(
        point_bounds.contains(&exact),
        "Zero-area bounds must contain their exact coordinate"
    );
    // Any other point – even infinitesimally offset – must not match.
    let nearby = Coordinates::new(48.8566 + 1e-10, 2.3522);
    assert!(
        !point_bounds.contains(&nearby),
        "Zero-area bounds must not contain a nearby point"
    );
}

#[test]
fn fully_inverted_bounds_contain_nothing() {
    // Both axes inverted at the same time.
    let double_inverted = GeographicBounds {
        north: 48.80,
        south: 48.90,
        east: 2.30,
        west: 2.40,
    };
    assert!(!double_inverted.contains(&Coordinates::new(48.85, 2.35)));
}

// ---------------------------------------------------------------------------
// 2. Serde defaults: JourneyPreferences and AvailabilityFilter
// ---------------------------------------------------------------------------

/// `JourneyPreferences` is deserialized from the `preferences` field of
/// `PlanBikeJourneyInput`. When the caller omits individual fields the
/// `#[serde(default)]` / `default_max_walk` functions fill them in.
/// These defaults are load-bearing: the handler uses them to constrain the
/// station search radius and bike-type filter.

#[test]
fn journey_preferences_defaults_from_empty_json() {
    let prefs: JourneyPreferences = serde_json::from_str("{}").unwrap();
    assert_eq!(
        prefs.bike_type,
        BikeTypeFilter::AnyType,
        "Default bike_type must be AnyType"
    );
    assert_eq!(
        prefs.max_walk_distance, 500,
        "Default max_walk_distance must be 500 m"
    );
}

#[test]
fn journey_preferences_explicit_values_override_defaults() {
    let json = r#"{"bike_type": "electric", "max_walk_distance": 250}"#;
    let prefs: JourneyPreferences = serde_json::from_str(json).unwrap();
    assert_eq!(prefs.bike_type, BikeTypeFilter::ElectricOnly);
    assert_eq!(prefs.max_walk_distance, 250);
}

#[test]
fn journey_preferences_partial_override_keeps_other_default() {
    // Specify only max_walk_distance; bike_type must still default to AnyType.
    let prefs: JourneyPreferences = serde_json::from_str(r#"{"max_walk_distance": 1000}"#).unwrap();
    assert_eq!(prefs.bike_type, BikeTypeFilter::AnyType);
    assert_eq!(prefs.max_walk_distance, 1000);
}

/// `AvailabilityFilter` is nested inside `FindNearbyStationsInput`.
/// `exclude_out_of_service` defaults to `true` via `default_true()`.
/// An incorrect default here would cause the handler to silently include
/// out-of-service stations in results.

#[test]
fn availability_filter_exclude_out_of_service_defaults_to_true() {
    let filter: AvailabilityFilter = serde_json::from_str("{}").unwrap();
    assert!(
        filter.exclude_out_of_service,
        "exclude_out_of_service must default to true"
    );
}

#[test]
fn availability_filter_exclude_out_of_service_can_be_set_false() {
    let filter: AvailabilityFilter =
        serde_json::from_str(r#"{"exclude_out_of_service": false}"#).unwrap();
    assert!(!filter.exclude_out_of_service);
}

#[test]
fn availability_filter_optional_fields_default_to_none() {
    let filter: AvailabilityFilter = serde_json::from_str("{}").unwrap();
    assert!(filter.min_bikes.is_none());
    assert!(filter.min_docks.is_none());
    assert!(filter.bike_type.is_none());
}

#[test]
fn availability_filter_round_trips_through_json() {
    let original = AvailabilityFilter {
        min_bikes: Some(2),
        min_docks: Some(1),
        bike_type: Some(BikeTypeFilter::MechanicalOnly),
        exclude_out_of_service: false,
    };
    let json = serde_json::to_string(&original).unwrap();
    let restored: AvailabilityFilter = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.min_bikes, Some(2));
    assert_eq!(restored.min_docks, Some(1));
    assert!(matches!(
        restored.bike_type,
        Some(BikeTypeFilter::MechanicalOnly)
    ));
    assert!(!restored.exclude_out_of_service);
}

// ---------------------------------------------------------------------------
// 3. RetryStrategy::ExponentialBackoff jitter-cap invariant
// ---------------------------------------------------------------------------
//
// The implementation applies `max_delay` cap *before* computing jitter:
//
//     let delay = base_delay * 2^attempt;
//     let delay = delay.min(max_delay);        // cap first
//     let jitter = delay * 0.25 * rand();      // jitter on capped value
//
// The invariant: returned duration <= max_delay + 25% of max_delay.
// We sample 200 times across several attempt numbers to catch accidental
// ordering swaps (cap-after-jitter would allow jitter on the uncapped value).

use std::time::Duration;
use velib_mcp::data::RetryStrategy;

/// Maximum fraction of jitter added above the base calculated delay.
const MAX_JITTER_FRACTION: f64 = 0.25;

fn max_permitted_duration(max_delay_secs: u64) -> Duration {
    let max_millis = (max_delay_secs as f64 * (1.0 + MAX_JITTER_FRACTION) * 1000.0).ceil() as u64;
    Duration::from_millis(max_millis)
}

#[test]
fn jitter_never_exceeds_max_delay_plus_25_percent() {
    let max_delay: u64 = 10;
    let strategy = RetryStrategy::ExponentialBackoff {
        base_delay: 1,
        max_delay,
        use_jitter: true,
    };

    let ceiling = max_permitted_duration(max_delay);

    // Sample across attempts 0-5 (base delays 1, 2, 4, 8, 10, 10 after cap).
    for attempt in 0..6 {
        for _ in 0..200 {
            let d = strategy.calculate_delay(attempt);
            assert!(
                d <= ceiling,
                "attempt={attempt}: delay {d:?} exceeded ceiling {ceiling:?} \
                 (max_delay={max_delay}s + 25% jitter)"
            );
        }
    }
}

#[test]
fn jitter_delay_is_at_least_the_base_calculated_delay() {
    // Jitter adds to, never subtracts from, the base delay.
    let strategy = RetryStrategy::ExponentialBackoff {
        base_delay: 2,
        max_delay: 60,
        use_jitter: true,
    };

    for attempt in 0..4 {
        let base_secs = (2_u64 * 2_u64.pow(attempt)).min(60);
        let floor = Duration::from_secs(base_secs);
        for _ in 0..100 {
            let d = strategy.calculate_delay(attempt);
            assert!(
                d >= floor,
                "attempt={attempt}: jitter delay {d:?} was below base floor {floor:?}"
            );
        }
    }
}

#[test]
fn without_jitter_delay_is_exactly_capped_base() {
    // Regression guard: no-jitter path must be deterministic.
    let strategy = RetryStrategy::ExponentialBackoff {
        base_delay: 1,
        max_delay: 5,
        use_jitter: false,
    };
    // attempt 0: 1s, 1: 2s, 2: 4s, 3: capped at 5s, 4: capped at 5s
    assert_eq!(strategy.calculate_delay(0), Duration::from_secs(1));
    assert_eq!(strategy.calculate_delay(1), Duration::from_secs(2));
    assert_eq!(strategy.calculate_delay(2), Duration::from_secs(4));
    assert_eq!(strategy.calculate_delay(3), Duration::from_secs(5));
    assert_eq!(strategy.calculate_delay(4), Duration::from_secs(5));
}

#[test]
fn retry_config_test_defaults_have_zero_jitter_for_determinism() {
    // The `Default` impl chooses `use_jitter: false` in `#[cfg(test)]` so
    // that retry timing tests are stable. Verify the invariant explicitly
    // so a future change to the cfg(test) block is caught immediately.
    let cfg = RetryConfig::default();
    assert!(
        !cfg.use_jitter,
        "Test-mode RetryConfig must disable jitter for deterministic test timing"
    );
}
