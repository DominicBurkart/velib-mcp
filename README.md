# Velib MCP Server

[![Test Coverage](https://img.shields.io/badge/coverage-check%20actions-brightgreen)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml)
[![Tests](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml)
[![Security Audit](https://img.shields.io/badge/security-audit%20passing-brightgreen)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](https://github.com/DominicBurkart/velib-mcp#license)
[![Rust Version](https://img.shields.io/badge/rust-latest%20stable-orange)](https://www.rust-lang.org/)
[![Deploy Status](https://github.com/DominicBurkart/velib-mcp/actions/workflows/deploy.yml/badge.svg)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/deploy.yml)

A high-performance Model Context Protocol (MCP) server providing access to Paris Velib bike sharing data for AI assistants.

## Quick Start — Claude Code

```bash
cargo install --git https://github.com/dominicburkart/velib-mcp.git
PORT=8080 velib-mcp
```

Then configure Claude Code to connect to `http://localhost:8080` and use the tools:

```
@velib find nearby stations at latitude 48.8566 longitude 2.3522
@velib get station by code 16107
@velib search stations by name "châtelet"
```

## Overview

This project exposes two Parisian datasets through MCP:
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

### Cursor
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

### Other MCP-compatible clients
Run the server with `PORT=8080 velib-mcp` and point your client at `http://localhost:8080`.

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
```

### Podman

```bash
# Build container image
podman build -t velib-mcp .

# Run container
podman run -p 8080:8080 velib-mcp
```

## Deployment

The project deploys to Scaleway Container Serverless via GitHub Actions on pushes to main.
The server is configured via `IP` and `PORT` environment variables (defaults: `0.0.0.0:8080`).

## Architecture

- **Language**: Rust
- **Deployment**: Scaleway Container Serverless
- **CI/CD**: GitHub Actions
- **Development**: Test-Driven Development
- **Container**: Distroless Debian base image

## License

Licensed under either of:
- Apache License, Version 2.0
- MIT License

at your option.
