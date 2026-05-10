# Velib MCP Server

[![Test Coverage](https://img.shields.io/badge/coverage-check%20actions-brightgreen)](https://github.com/dominicburkart/velib-mcp/actions/workflows/ci.yml)
[![Tests](https://github.com/dominicburkart/velib-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/dominicburkart/velib-mcp/actions/workflows/ci.yml)
[![Security Audit](https://img.shields.io/badge/security-audit%20passing-brightgreen)](https://github.com/dominicburkart/velib-mcp/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](https://github.com/dominicburkart/velib-mcp#license)
[![Rust Version](https://img.shields.io/badge/rust-latest%20stable-orange)](https://www.rust-lang.org/)
[![Deploy Status](https://github.com/dominicburkart/velib-mcp/actions/workflows/deploy.yml/badge.svg)](https://github.com/dominicburkart/velib-mcp/actions/workflows/deploy.yml)

A high-performance Model Context Protocol (MCP) server providing access to Paris Velib bike sharing data for AI assistants.

## Quick Start - Install with Claude Code

```bash
# PORT (default 8080) and IP (default 0.0.0.0) configure the bind address.
cargo install --git https://github.com/dominicburkart/velib-mcp.git
claude config add-server velib-mcp "PORT=3000 velib-mcp"
```

Then in Claude Code:
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

All integrations share the same install step:

```bash
cargo install --git https://github.com/dominicburkart/velib-mcp.git
```

<details>
<summary>Per-client configuration</summary>

- **ChatGPT / Le Chat / Mistral**: run `velib-mcp` (binds `0.0.0.0:8080`;
  override with `PORT`/`IP`) and call it via the client's custom-tool / API
  hooks.
- **Windsurf**: register the binary in Windsurf's MCP settings.
- **Cursor**: add to `settings.json`:

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

`cargo-husky` (a dev-dependency) installs a Git pre-commit hook on the first
build. The hook (`.cargo-husky/hooks/pre-commit`) runs `cargo clippy --fix`,
`cargo fmt --all`, `cargo sort`, and `cargo deny check licenses bans sources`
(mirroring the `license-compliance` CI job). Any non-zero exit aborts the
commit.

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
