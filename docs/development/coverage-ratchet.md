# Coverage Ratchet (untested-lines)

This doc is agent-facing: it explains how `scripts/coverage-ratchet.sh` and
`.github/workflows/coverage-ratchet.yml` work, how to read the output, and
how to debug false positives.

## Why untested lines, not %

A percentage moves when the *denominator* changes. If you delete a block of
well-tested code, covered lines drop but the percentage can stay flat or
even fall. Meanwhile uncovered *line count* has a clean semantic: it is the
number of source lines tarpaulin believes are reachable and never executed
by the test suite.

The ratchet fails only when `HEAD uncovered > base uncovered` (strict `>`).

## What runs where

- **Local pre-push hook** (invoked, not installed): `scripts/coverage-ratchet.sh`
  See README for how to symlink it.
- **CI**: `.github/workflows/coverage-ratchet.yml` runs on every PR targeting
  `main`. Pushes to `main` only run the self-test (fixtures only), so they
  are never blocked by coverage deltas.
- **Composite action**: `.github/actions/coverage-ratchet/` wraps the same
  script for reuse.

## The heuristic (how suggestions are chosen)

For each file in the top-N uncovered list the summary suggests a test
paradigm. The rules are intentionally simple and live in
`scripts/coverage-ratchet-summary.py::_heuristic`:

| Pattern in path                          | Suggestion                       |
| ---------------------------------------- | -------------------------------- |
| `/data/`, `client`, `http`, `reqwest`, `fetch` | `integration (I/O path)`   |
| `/server/`, `/mcp/`, `handler`, `route`, `router` | `integration (routing/protocol)` |
| `types.rs`, `serde`, `parse`, `convert`, `decode`, `encode`, `codec` | `property (data transforms)` |
| otherwise                                 | `unit (pure function)`          |

If the heuristic mis-fires for your module, add a pattern to `_heuristic`
and extend the self-test in `scripts/coverage-ratchet.sh`.

## Reading the output

```
## Coverage Ratchet (untested lines)
| metric | value |
| --- | --- |
| base (<sha>) uncovered lines | 618 |
| HEAD uncovered lines | 625 |
| delta | 7 |

### Top files by uncovered lines at HEAD
  205  src/mcp/server.rs  lines=34-35,41,43-45,...  suggest=integration (routing/protocol)
  173  src/mcp/handlers.rs lines=42,49,67-68,...    suggest=integration (routing/protocol)
   ...
```

Agent loop:

1. Look at the file with the biggest uncovered count.
2. Open the listed line ranges.
3. Pick the suggested paradigm and write a targeted test.
4. Re-run `scripts/coverage-ratchet.sh` locally (~75s) until delta <= 0.

## Debugging false positives

- **Tarpaulin reports different numbers on base vs HEAD due to build
  caching.** The script uses `--skip-clean`, which keeps prior artifacts
  but can miss newly-introduced modules if rebuilds fail mid-way. Try
  `cargo clean && scripts/coverage-ratchet.sh`.
- **Merge-base is stale.** The ratchet compares against the *merge-base*,
  not `origin/main` directly. If you rebased, the merge-base can point at
  an ancient commit. Refresh with `git fetch origin main`.
- **Doctests or macro-expanded lines reported as uncovered.** Tarpaulin
  sometimes misattributes these. Inspect the raw report at
  `coverage/head/tarpaulin-report.json` and confirm the line is genuinely
  an executable statement.
- **Self-test fails but `cargo test` passes.** The self-test uses
  fixtures and does not invoke cargo. A failure there means the parsing
  or decision logic regressed — look at
  `scripts/coverage-ratchet-summary.py` first.
- **CI fails, local passes.** Usually the agent forgot to push a test
  file, or CI has a different base SHA because `fetch-depth: 0` gives
  the full history. Check the job summary at the bottom of the CI page.

## Overrides

```
COVERAGE_BASE_REF=origin/develop scripts/coverage-ratchet.sh
COVERAGE_TOP_N=10 scripts/coverage-ratchet.sh --summary coverage/head/tarpaulin-report.json
```

Do **not** set a fixed baseline for the whole repo (`FAIL_ON_UNCOVERED=0`
style). This is a delta check only.
