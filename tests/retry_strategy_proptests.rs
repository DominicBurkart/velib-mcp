//! Property-based tests for [`velib_mcp::data::retry::RetryStrategy`].
//!
//! The retry/backoff layer sits on the critical path of every outbound
//! HTTP call to the Velib open-data API (rate limiting, transient 5xx,
//! network blips), so subtle regressions in the delay calculation
//! directly translate to either:
//!
//! * thundering-herd retries that hammer the upstream API, or
//! * arbitrarily long stalls that block MCP tool responses.
//!
//! The pre-existing unit tests in `src/data/retry.rs` only exercise a
//! handful of hand-picked `(attempt, base_delay, max_delay)` triples.
//! The properties below quantify universally over the input domain and
//! lock down the invariants any future refactor of the backoff formula
//! must preserve.
//!
//! # Invariants validated
//!
//! 1. **Cap is exact (no jitter).** `delay == min(base * 2^attempt, max)`
//!    for every sensible `(base, max, attempt)`.
//! 2. **Cap is respected (with jitter).** `delay <= capped + 25%`,
//!    matching the documented thundering-herd mitigation envelope.
//! 3. **Lower bound (with jitter).** `delay >= capped`; jitter never
//!    shortens the back-off below the deterministic floor.
//! 4. **Monotonicity (no jitter).** The delay sequence is non-decreasing
//!    in `attempt`, so later retries always wait at least as long as
//!    earlier ones.
//! 5. **Determinism (no jitter).** Repeated invocations with identical
//!    inputs produce identical delays; no hidden state or randomness.
//! 6. **Attempt-0 anchor.** At `attempt = 0`, the delay equals
//!    `min(base, max)` — a hand-verified anchor case pinning the
//!    formula's documented contract.
//! 7. **Collapse to constant.** When `base_delay == max_delay`, the
//!    exponential backoff degenerates into a fixed delay equal to that
//!    value, regardless of the attempt number.

use std::time::Duration;

use proptest::prelude::*;
use velib_mcp::data::retry::RetryStrategy;

/// Maximum `attempt` value used in the proptests.
///
/// We deliberately stay below the 64-bit overflow boundary for
/// `2^attempt`; the current implementation uses non-saturating
/// `2_u64.pow(attempt)`, so testing `attempt >= 64` would panic on
/// overflow rather than reveal a meaningful property. Production usage
/// caps `max_attempts` in [`velib_mcp::data::RetryConfig`] at single
/// digits, so this range is far larger than any realistic call site.
const MAX_TESTED_ATTEMPT: u32 = 30;

/// Saturating reference implementation of the formula `base * 2^attempt`.
/// Used by the property tests to derive the expected (pre-cap) delay
/// without risking overflow in the test harness itself.
fn expected_uncapped_delay(base: u64, attempt: u32) -> u64 {
    base.saturating_mul(2_u64.checked_pow(attempt).unwrap_or(u64::MAX))
}

fn exp_backoff(base: u64, max: u64, use_jitter: bool) -> RetryStrategy {
    RetryStrategy::ExponentialBackoff {
        base_delay: base,
        max_delay: max,
        use_jitter,
    }
}

proptest! {
    /// Without jitter, the delay equals `min(base * 2^attempt, max)`
    /// exactly. This combines the strongest forms of the cap and
    /// lower-bound invariants.
    #[test]
    fn no_jitter_delay_equals_capped_formula(
        base in 1_u64..1_000,
        max in 1_u64..1_000,
        attempt in 0_u32..MAX_TESTED_ATTEMPT,
    ) {
        let strategy = exp_backoff(base, max, false);
        let expected = expected_uncapped_delay(base, attempt).min(max);
        prop_assert_eq!(
            strategy.calculate_delay(attempt),
            Duration::from_secs(expected)
        );
    }

    /// With jitter, the delay stays inside the documented
    /// `[capped, capped + 25%]` envelope on every draw.
    #[test]
    fn jittered_delay_stays_in_envelope(
        base in 1_u64..100,
        max in 1_u64..1_000,
        attempt in 0_u32..MAX_TESTED_ATTEMPT,
    ) {
        let strategy = exp_backoff(base, max, true);
        let capped = expected_uncapped_delay(base, attempt).min(max);
        // Implementation rounds the jitter to whole seconds via `.round()`,
        // so the worst-case ceiling is `capped + round(capped * 0.25)`.
        let jitter_ceiling = (capped as f64 * 0.25).round() as u64;
        let upper = capped.saturating_add(jitter_ceiling);

        // Sample multiple times because jitter is random; each draw must
        // satisfy the envelope independently.
        for _ in 0..16 {
            let delay = strategy.calculate_delay(attempt);
            prop_assert!(
                delay >= Duration::from_secs(capped),
                "jittered delay {:?} fell below floor {}s (base={} max={} attempt={})",
                delay, capped, base, max, attempt
            );
            prop_assert!(
                delay <= Duration::from_secs(upper),
                "jittered delay {:?} exceeded ceiling {}s (base={} max={} attempt={})",
                delay, upper, base, max, attempt
            );
        }
    }

    /// Without jitter, the delay sequence is monotonically non-decreasing
    /// in the attempt number. Once it hits the cap it stays there.
    #[test]
    fn no_jitter_delay_is_monotonic(
        base in 1_u64..100,
        max in 1_u64..1_000,
        attempt in 0_u32..(MAX_TESTED_ATTEMPT - 1),
    ) {
        let strategy = exp_backoff(base, max, false);
        let a = strategy.calculate_delay(attempt);
        let b = strategy.calculate_delay(attempt + 1);
        prop_assert!(
            b >= a,
            "delay decreased between attempts {} ({:?}) and {} ({:?}) (base={}, max={})",
            attempt, a, attempt + 1, b, base, max
        );
    }

    /// Without jitter, repeated invocations on the same strategy return
    /// identical delays. Guards against accidental shared state or
    /// unintended randomness creeping into the hot path.
    #[test]
    fn no_jitter_delay_is_deterministic(
        base in 1_u64..1_000,
        max in 1_u64..1_000,
        attempt in 0_u32..MAX_TESTED_ATTEMPT,
    ) {
        let strategy = exp_backoff(base, max, false);
        prop_assert_eq!(
            strategy.calculate_delay(attempt),
            strategy.calculate_delay(attempt)
        );
    }

    /// When `base_delay == max_delay`, the exponential backoff collapses
    /// to a fixed delay equal to that value (jitter disabled).
    #[test]
    fn no_jitter_collapses_when_base_equals_max(
        same in 1_u64..1_000,
        attempt in 0_u32..MAX_TESTED_ATTEMPT,
    ) {
        let strategy = exp_backoff(same, same, false);
        prop_assert_eq!(
            strategy.calculate_delay(attempt),
            Duration::from_secs(same)
        );
    }
}

/// Anchor: at `attempt = 0`, the no-jitter delay equals
/// `min(base, max)`. Hand-verified case kept alongside the quantified
/// properties so future readers see the formula's contract at a glance.
#[test]
fn attempt_zero_equals_min_of_base_and_max() {
    for &(base, max) in &[(1_u64, 10), (5, 5), (7, 3), (100, 1_000)] {
        let strategy = exp_backoff(base, max, false);
        assert_eq!(
            strategy.calculate_delay(0),
            Duration::from_secs(base.min(max)),
            "attempt-0 delay should equal min(base, max) for base={base} max={max}"
        );
    }
}
