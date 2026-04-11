# Velib MCP Server

[![Tests](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml)
[![Deploy Status](https://github.com/DominicBurkart/velib-mcp/actions/workflows/deploy.yml/badge.svg)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/deploy.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](https://github.com/DominicBurkart/velib-mcp#license)

A Model Context Protocol (MCP) server providing real-time Paris Velib bike sharing data for AI assistants.

## Quick Start - Install with Claude Code

```bash
cargo install --git https://github.com/dominicburkart/velib-mcp.git
claude config add-server velib-mcp "cargo run --release -- --port 3000"
```

Then use in Claude Code:
```
@velib find nearby stations at latitude 48.8566 longitude 2.3522
@velib get station by code 16107
@velib search stations by name "châtelet"
```

## Overview

Exposes two Parisian datasets via MCP:
- **Real-time availability**: Current bike and dock availability at stations
- **Station locations**: Geographic information and details about all Velib stations

## Data Sources

- [Velib Real-time Availability](https://opendata.paris.fr/explore/dataset/velib-disponibilite-en-temps-reel/)
- [Velib Station Locations](https://opendata.paris.fr/explore/dataset/velib-emplacement-des-stations/)

## Available Tools

- `find_nearby_stations`: Find stations within a radius of coordinates
- `get_station_by_code`: Get detailed information about a specific station
- `search_stations_by_name`: Search stations by name with optional fuzzy matching
- `get_area_statistics`: Get aggregated statistics for a geographic area
- `plan_bike_journey`: Plan a bike journey with pickup and dropoff suggestions

## Integration with Other AI Tools

First, install the server binary:
```bash
cargo install --git https://github.com/dominicburkart/velib-mcp.git
```

<details>
<summary>Click to expand integration guides</summary>

### ChatGPT
Run `velib-mcp` and configure via ChatGPT Custom Instructions or the API.

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
Run `velib-mcp --port 8080` and use via API calls.

### Windsurf
Run `velib-mcp` and configure in Windsurf MCP settings.

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
podman build -t velib-mcp .
podman run -p 8080:8080 velib-mcp
```

## Deployment

Configured for deployment to Scaleway Container Serverless via GitHub Actions on pushes to `main`.

## Architecture

- **Language**: Rust
- **Deployment**: Scaleway Container Serverless
- **CI/CD**: GitHub Actions
- **Container**: Distroless Debian base image

## License

Licensed under either of:
- Apache License, Version 2.0
- MIT License

at your option.
