# Velib MCP Server

[![Test Coverage](https://img.shields.io/badge/coverage-check%20actions-brightgreen)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml)
[![Tests](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml)
[![Security Audit](https://img.shields.io/badge/security-audit%20passing-brightgreen)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](https://github.com/DominicBurkart/velib-mcp#license)
[![Rust Version](https://img.shields.io/badge/rust-latest%20stable-orange)](https://www.rust-lang.org/)
[![Deploy Status](https://github.com/DominicBurkart/velib-mcp/actions/workflows/deploy.yml/badge.svg)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/deploy.yml)

A high-performance Model Context Protocol (MCP) server providing access to Paris Velib bike-sharing data for AI assistants.

## Overview

This project exposes two Parisian datasets through MCP:

- **Real-time availability**: current bike and dock availability at stations
- **Station locations**: geographic information and metadata for all Velib stations

### Data Sources

- [Velib Real-time Availability](https://opendata.paris.fr/explore/dataset/velib-disponibilite-en-temps-reel/)
- [Velib Station Locations](https://opendata.paris.fr/explore/dataset/velib-emplacement-des-stations/)

### Available Tools

- `find_nearby_stations` — find Velib stations within a radius of coordinates
- `get_station_by_code` — get detailed information about a specific station
- `search_stations_by_name` — search stations by name, with optional fuzzy matching
- `get_area_statistics` — aggregated statistics for a geographic area
- `plan_bike_journey` — plan a bike journey with pickup and dropoff suggestions

## Quick Start — Claude Code

Install and configure the server in one step:

```bash
cargo install --git https://github.com/DominicBurkart/velib-mcp.git
claude config add-server velib-mcp "cargo run --release -- --port 3000"
```

Then use it in Claude Code:

```
@velib find nearby stations at latitude 48.8566 longitude 2.3522
@velib get station by code 16107
@velib search stations by name "châtelet"
```

## Integration with Other AI Tools

Install the binary once, then configure the client you are using:

```bash
cargo install --git https://github.com/DominicBurkart/velib-mcp.git
```

<details>
<summary>Integration guides</summary>

### ChatGPT

Run the server and wire it to ChatGPT Custom Instructions or the API:

```bash
velib-mcp --port 8080
```

### Cursor

Add to Cursor's `settings.json`:

```json
{
  "mcp.servers": {
    "velib": {
      "command": "velib-mcp",
      "args": ["--port", "8080"]
    }
  }
}
```

### Le Chat / Mistral

Run the server and call it via API:

```bash
velib-mcp --port 8080
```

### Windsurf

Configure the binary in Windsurf's MCP settings.

</details>

## Development

### Prerequisites

- Rust (latest stable)
- OpenSSL development libraries
- `pkg-config`

### Setup

```bash
git clone https://github.com/DominicBurkart/velib-mcp.git
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

`cargo-husky` is a dev-dependency and installs a Git pre-commit hook the first time `cargo test` (or any cargo command that builds dev-dependencies) runs. The hook at `.cargo-husky/hooks/pre-commit` enforces:

- `cargo clippy --fix --all-features --all-targets`
- `cargo fmt --all`
- `cargo sort`
- `cargo deny check licenses bans sources` (mirrors the `license-compliance` CI job so license, bans, and source-provenance regressions are caught locally)

Any non-zero exit aborts the commit.

### Podman

```bash
podman build -t velib-mcp .
podman run -p 8080:8080 velib-mcp
```

## Deployment

The project deploys to Scaleway Container Serverless via GitHub Actions on pushes to `main`.

## Architecture

- **Language**: Rust
- **Deployment**: Scaleway Container Serverless
- **CI/CD**: GitHub Actions
- **Development**: test-driven
- **Container**: distroless Debian base image

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT License

at your option.
