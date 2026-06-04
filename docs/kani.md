# Kani Formal Verification

Kani is a bit-precise model checker for Rust. It complements unit tests and proptest
by exhaustively proving safety properties over bounded input domains — surfacing
overflow, panic, and assertion violations as concrete counterexamples.

## When to add a Kani harness

Reach for a Kani harness when reviewing code that has:

- Arithmetic that can overflow (`+`, `-`, `*`, `pow`, `as` casts widening or narrowing)
- `unwrap()` / `expect()` on a value whose `Some`/`Ok` invariant is non-trivial
- Float → integer casts (NaN / infinity are UB-adjacent)
- String length checks (`len()` is byte count, not char count)
- Cross-field validation invariants (e.g. `bikes + docks <= capacity`)

Unit tests prove specific examples. Kani proves the property holds for *all* inputs in
the domain you bound.

## Running locally

```
cargo install --locked kani-verifier
cargo kani setup
cargo kani                       # all harnesses
cargo kani --harness <name>      # single harness
```

## CI integration

`.github/workflows/kani.yml` should run `model-checking/kani-github-action@v1.1` on
every PR. The action installs Kani, runs all `#[kani::proof]` harnesses, and fails
the build on any counterexample. (Workflow file pending manual addition — see the
foundation PR body for the YAML.)

## Harness placement convention

Inline in the source file, behind `#[cfg(kani)]`:

```rust
#[cfg(kani)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn proof_foo_does_not_panic() {
        let x: u32 = kani::any();
        kani::assume(x <= 1000);
        let _ = foo(x);
    }
}
```

Name harnesses `proof_<property>` for clarity in CI logs.

## Per-component plan (issue #62)

| Issue | File | Harnesses |
|---|---|---|
| #63 | `src/data/retry.rs` | `2_u64.pow(attempt)` overflow threshold; `last_error.unwrap()` loop invariant; total backoff bound |
| #64 | `src/types.rs` | Haversine non-negative & finite & ≤ Earth circumference; metro ⊂ service-area; `distance_to` symmetric; invalid coords rejected |
| #65 | `src/mcp/handlers.rs` | `as u32` cast input finite & non-negative & ≤ `u32::MAX` |
| #66 | `src/mcp/handlers.rs` | `max_walk_distance == 0` reaches division (counterexample); confidence_score in [0,1]; no intermediate NaN/inf |
| #67 | `src/data/client.rs` | `u64 as u16` silent truncation counterexample; `try_from` is lossless for ≤ 65535 |
| #68 | `src/types.rs` | `total_bikes + available_docks ≤ capacity`; `saturating_add` documented; `StationReference::validate` covers all invariants |
| #69 | `src/data/cache.rs` | expired iff `now > expires_at`; `cleanup_expired` exact; insert→get roundtrip; TTL arithmetic bound |
| #70 | `src/mcp/handlers.rs` | `len() < 2` byte/char mismatch counterexample; radius ≤ `MAX_SEARCH_RADIUS`; limit ≤ `MAX_RESULT_LIMIT` |

Each row lands as its own draft PR per #62's instructions.
