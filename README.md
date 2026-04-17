# Velib MCP Server

[![Tests](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml)
[![Deploy Status](https://github.com/DominicBurkart/velib-mcp/actions/workflows/deploy.yml/badge.svg)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/deploy.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](https://github.com/DominicBurkart/velib-mcp#license)

A high-performance Model Context Protocol (MCP) server providing access to Paris Velib bike sharing data for AI assistants.

## Quick Start

```bash
cargo install --git https://github.com/dominicburkart/velib-mcp.git
```

Then add to your MCP client config (example for Claude Code):

```json
{
  "mcp.servers": {
    "velib": {
      "command": "velib-mcp"
    }
  }
}
```

Example prompts:
```
find nearby stations at latitude 48.8566 longitude 2.3522
get station by code 16107
search stations by name "châtelet"
```

## Overview

Exposes two Parisian open datasets through MCP:

- **Real-time availability**: Current bike and dock availability at stations — [source](https://opendata.paris.fr/explore/dataset/velib-disponibilite-en-temps-reel/)
- **Station locations**: Geographic metadata for all Velib stations — [source](https://opendata.paris.fr/explore/dataset/velib-emplacement-des-stations/)

## Available Tools

| Tool | Description |
|------|-------------|
| `find_nearby_stations` | Find stations within a radius of coordinates |
| `get_station_by_code` | Get details for a specific station |
| `search_stations_by_name` | Search stations by name (supports fuzzy matching) |
| `get_area_statistics` | Aggregated statistics for a geographic bounding box |
| `plan_bike_journey` | Suggest pickup and dropoff stations for a journey |

See [`docs/api/mcp_interface_spec.md`](docs/api/mcp_interface_spec.md) for full schema details.

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

### Commands

```bash
cargo test                                          # run tests
cargo fmt --check                                   # check formatting
cargo clippy --all-targets --all-features -- -D warnings  # lint
cargo audit                                         # security audit
```

### Container

```bash
podman build -t velib-mcp .
podman run -p 8080:8080 velib-mcp
```

## Deployment

Automatically deployed to Scaleway Container Serverless on push to `main` via GitHub Actions. See [`deploy_to_scaleway.sh`](deploy_to_scaleway.sh) and [`.github/workflows/deploy.yml`](.github/workflows/deploy.yml).

## Architecture

- **Language**: Rust
- **Transport**: Scaleway Container Serverless
- **CI/CD**: GitHub Actions
- **Container**: Distroless Debian base image

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
