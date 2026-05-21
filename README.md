# Velib MCP Server

[![Test Coverage](https://img.shields.io/badge/coverage-check%20actions-brightgreen)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml)
[![Tests](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml)
[![Security Audit](https://img.shields.io/badge/security-audit%20passing-brightgreen)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](https://github.com/DominicBurkart/velib-mcp#license)
[![Rust Version](https://img.shields.io/badge/rust-latest%20stable-orange)](https://www.rust-lang.org/)
[![Deploy Status](https://github.com/DominicBurkart/velib-mcp/actions/workflows/deploy.yml/badge.svg)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/deploy.yml)

A high-performance Model Context Protocol (MCP) server providing access to Paris Velib bike sharing data for AI assistants.

## Quick Start - Install with Claude Code

Install and use the Velib MCP server with Claude Code in one command:

```bash
# Install and configure the server (port is read from the PORT env var; default 8080)
cargo install --git https://github.com/dominicburkart/velib-mcp.git
claude config add-server velib-mcp "PORT=3000 velib-mcp"
```

Then use in Claude Code:
```
@velib find nearby stations at latitude 48.8566 longitude 2.3522
@velib get station by code 16107
@velib search stations by name "châtelet"
```

## Overview

This project exposes two key Parisian datasets through MCP:
- **Real-time availability**: Current bike and dock availability at stations
- **Station locations**: Geographic information and details about all Velib stations

## Data Sources

- [Velib Real-time Availability](https://opendata.paris.fr/explore/dataset/velib-disponibilite-en-temps-reel/)
- [Velib Station Locations](https://opendata.paris.fr/explore/dataset/velib-emplacement-des-stations/)

## Available Tools

- `find_nearby_stations`: Find Velib stations within a radius of coordinates
- `get_station_by_code`: Get detailed information about a specific station
- `search_stations_by_name`: Search stations by name with optional fuzzy matching
- `get_area_statistics`: Get aggregated statistics for a geographic area
- `plan_bike_journey`: Plan a bike journey with pickup and dropoff suggestions

## Integration with Other AI Tools

<details>
<summary>Click to expand integration guides</summary>

### ChatGPT
```bash
# Install server
cargo install --git https://github.com/dominicburkart/velib-mcp.git
# Run server (defaults to 0.0.0.0:8080; set PORT/IP to override)
velib-mcp
# Configure in ChatGPT Custom Instructions or use via API
```

### Cursor
```bash
# Install server
cargo install --git https://github.com/dominicburkart/velib-mcp.git
```
Add to Cursor's `settings.json`:
```json
{
  "mcp.servers": {
    "velib": {
      "command": "velib-mcp",
      "env": { "PORT": "8080" }
    }
  }
}
```

### Le Chat / Mistral
```bash
# Install server
cargo install --git https://github.com/dominicburkart/velib-mcp.git
# Run server and use via API calls (PORT/IP env vars configure the bind address)
PORT=8080 velib-mcp
```

### Windsurf
```bash
# Install server
cargo install --git https://github.com/dominicburkart/velib-mcp.git
# Configure in Windsurf MCP settings
```

</details>

## Development

### Prerequisites

- Rust (latest stable)
- OpenSSL development libraries
- pkg-config

### Setup

```bash
git clone https://github.com/dominicburkart/velib-mcp.git
cd velib-mcp
cargo build
```

### Testing

```bash
cargo test
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo audit
cargo deny check licenses bans sources
```

### Pre-commit hooks

`cargo-husky` is installed as a dev-dependency and automatically installs a Git
pre-commit hook the first time `cargo test` (or any cargo command that builds
dev-dependencies) runs. The hook lives at `.cargo-husky/hooks/pre-commit` and
enforces:

- `cargo clippy --fix --all-features --all-targets`
- `cargo fmt --all`
- `cargo sort`
- `cargo deny check licenses bans sources` (mirrors the `license-compliance` CI
  job so license, bans, and source-provenance regressions are caught locally)

Any non-zero exit aborts the commit.

### Coverage ratchet (pre-push)

The repo ships with a tarpaulin-based untested-lines ratchet. It compares the
number of uncovered lines at `HEAD` against the merge-base with `origin/main`
and refuses pushes that *increase* the uncovered count. Tracking uncovered
lines (not percentage) avoids false alarms when tested code is deleted.

Install tarpaulin once:

```bash
cargo install cargo-tarpaulin --locked
```

Wire the hook locally (symlink, so the script stays in-repo):

```bash
ln -s ../../scripts/coverage-ratchet.sh .git/hooks/pre-push
chmod +x .git/hooks/pre-push
```

Or invoke ad-hoc:

```bash
# Full run (takes ~75s on this codebase)
scripts/coverage-ratchet.sh

# Fixture-only sanity check (runs in <1s, no cargo needed)
scripts/coverage-ratchet.sh --self-test

# Debug: show the top-N uncovered files for an existing tarpaulin JSON
scripts/coverage-ratchet.sh --summary coverage/head/tarpaulin-report.json
```

On failure the script prints a markdown summary (mirrored to
`$GITHUB_STEP_SUMMARY` in CI) listing the top files by uncovered lines, the
affected line ranges, and a suggested test paradigm (unit / integration /
property). See `docs/development/coverage-ratchet.md` for the heuristic and
debugging tips.

### Podman

```bash
# Build container image
podman build -t velib-mcp .

# Run container
podman run -p 8080:8080 velib-mcp
```

## Deployment

The project is configured for deployment to Scaleway Container Serverless via GitHub Actions on pushes to the main branch.

## Architecture

- **Language**: Rust
- **Deployment**: Scaleway Container Serverless  
- **CI/CD**: GitHub Actions
- **Development**: Test-Driven Development approach
- **Container**: Distroless Debian base image

## License

Licensed under either of:
- Apache License, Version 2.0
- MIT License

at your option.
