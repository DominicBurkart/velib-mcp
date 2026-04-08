# Velib MCP Server

[![Tests](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml)
[![Deploy Status](https://github.com/DominicBurkart/velib-mcp/actions/workflows/deploy.yml/badge.svg)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/deploy.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](https://github.com/DominicBurkart/velib-mcp#license)

A Rust MCP server exposing Paris Velib bike sharing data to AI assistants.

## Quick Start

```bash
cargo install --git https://github.com/dominicburkart/velib-mcp.git
claude mcp add velib velib-mcp
```

Example queries:
```
@velib find nearby stations at latitude 48.8566 longitude 2.3522
@velib get station by code 16107
@velib search stations by name "châtelet"
```

## Data Sources

- [Real-time availability](https://opendata.paris.fr/explore/dataset/velib-disponibilite-en-temps-reel/)
- [Station locations](https://opendata.paris.fr/explore/dataset/velib-emplacement-des-stations/)

## Available Tools

| Tool | Description |
|------|-------------|
| `find_nearby_stations` | Find stations within a radius of coordinates |
| `get_station_by_code` | Get details for a specific station |
| `search_stations_by_name` | Search stations by name (supports fuzzy matching) |
| `get_area_statistics` | Aggregated stats for a geographic bounding box |
| `plan_bike_journey` | Suggest pickup/dropoff stations for a journey |

## Integration with Cursor

Add to `settings.json`:
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

## Development

**Prerequisites**: Rust (latest stable), OpenSSL dev libraries, pkg-config

```bash
git clone https://github.com/dominicburkart/velib-mcp.git
cd velib-mcp
cargo build
```

**Checks** (all required before merging):
```bash
cargo test
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo audit
```

**Container**:
```bash
podman build -t velib-mcp .
podman run -p 8080:8080 velib-mcp
```

See [CLAUDE.md](CLAUDE.md) for architecture details and deployment instructions.

## License

Licensed under either of Apache License 2.0 or MIT License, at your option.
