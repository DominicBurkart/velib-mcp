# Velib MCP Server

[![Tests](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml)
[![Deploy Status](https://github.com/DominicBurkart/velib-mcp/actions/workflows/deploy.yml/badge.svg)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/deploy.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](https://github.com/DominicBurkart/velib-mcp#license)

A Rust MCP server providing real-time Paris Vélib bike-sharing data to AI assistants.

## Data Sources

- [Real-time availability](https://opendata.paris.fr/explore/dataset/velib-disponibilite-en-temps-reel/)
- [Station locations](https://opendata.paris.fr/explore/dataset/velib-emplacement-des-stations/)

## Available Tools

| Tool | Description |
|------|-------------|
| `find_nearby_stations` | Stations within a radius of given coordinates |
| `get_station_by_code` | Full details for a specific station |
| `search_stations_by_name` | Name search with optional fuzzy matching |
| `get_area_statistics` | Aggregated stats for a bounding box |
| `plan_bike_journey` | Pickup/dropoff station suggestions for a journey |

See [`docs/api/mcp_interface_spec.md`](docs/api/mcp_interface_spec.md) for full input/output schemas.

## Quick Start

```bash
cargo install --git https://github.com/dominicburkart/velib-mcp.git
```

### Claude Code

```bash
claude config add-server velib-mcp "velib-mcp"
```

Example prompts:
```
@velib find nearby stations at latitude 48.8566 longitude 2.3522
@velib get station by code 16107
@velib search stations by name "châtelet"
```

### Cursor

Add to `settings.json`:
```json
{
  "mcp.servers": {
    "velib": {
      "command": "velib-mcp"
    }
  }
}
```

### Other MCP-compatible clients

Run `velib-mcp` (listens on `$IP:$PORT`, defaulting to `0.0.0.0:8080`) and point your client at it over HTTP/WebSocket (JSON-RPC 2.0).

## Development

**Prerequisites**: Rust (stable), OpenSSL dev libraries, pkg-config

```bash
git clone https://github.com/dominicburkart/velib-mcp.git
cd velib-mcp
cargo build
cargo test
```

### Podman

```bash
podman build -t velib-mcp .
podman run -p 8080:8080 velib-mcp
```

## Deployment

Push to `main` triggers automatic deployment to Scaleway Container Serverless via GitHub Actions. See `.env.deploy.example` for required environment variables.

## Architecture

- **Language**: Rust
- **Deployment**: Scaleway Container Serverless
- **CI/CD**: GitHub Actions
- **Container**: Distroless Debian
- **Approach**: Test-Driven Development

## License

Licensed under either of Apache License 2.0 or MIT License, at your option.
