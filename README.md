# Velib MCP Server

[![Test Coverage](https://img.shields.io/badge/coverage-check%20actions-brightgreen)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml)
[![Tests](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml)
[![Security Audit](https://img.shields.io/badge/security-audit%20passing-brightgreen)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](https://github.com/DominicBurkart/velib-mcp#license)
[![Rust Version](https://img.shields.io/badge/rust-latest%20stable-orange)](https://www.rust-lang.org/)
[![Deploy Status](https://github.com/DominicBurkart/velib-mcp/actions/workflows/deploy.yml/badge.svg)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/deploy.yml)

A high-performance Model Context Protocol (MCP) server providing access to Paris Velib bike sharing data for AI assistants.

## Quick Start - Install with Claude Code

Install and configure the server:

```bash
cargo install --git https://github.com/dominicburkart/velib-mcp.git
PORT=3000 velib-mcp &
claude config add-server velib-mcp "http://localhost:3000/mcp"
```

The server is configured via environment variables (see [Configuration](#configuration)).

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

## Configuration

The server reads its listen address from two environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `IP`     | `0.0.0.0` | IP address to bind |
| `PORT`   | `8080`    | TCP port to listen on |

Example:
```bash
IP=127.0.0.1 PORT=9000 velib-mcp
```

## Integration with Other AI Tools

<details>
<summary>Click to expand integration guides</summary>

### ChatGPT
```bash
# Install and start the server
cargo install --git https://github.com/dominicburkart/velib-mcp.git
PORT=8080 velib-mcp
# Configure in ChatGPT Custom Instructions or use via API
```

### Cursor

Add to Cursor's `settings.json`. The server must be started separately before
launching Cursor.

```json
{
  "mcp.servers": {
    "velib": {
      "url": "http://localhost:8080/mcp"
    }
  }
}
```

### Le Chat / Mistral
```bash
# Install and start the server
cargo install --git https://github.com/dominicburkart/velib-mcp.git
PORT=8080 velib-mcp
# Then point your Mistral integration at http://localhost:8080/mcp
```

### Windsurf
```bash
# Install and start the server
cargo install --git https://github.com/dominicburkart/velib-mcp.git
PORT=8080 velib-mcp
# Configure in Windsurf MCP settings pointing to http://localhost:8080/mcp
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
```

### Container

```bash
# Build image (uses Podman; substitute docker if preferred)
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
