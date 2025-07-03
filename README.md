# Velib MCP Server

A high-performance Model Context Protocol (MCP) server providing access to Paris Velib bike sharing data for AI assistants.

## Quick Start - Install with Claude Code

Install and use the Velib MCP server with Claude Code in one command:

```bash
# Install and configure the server
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
# Run server on port 8080
velib-mcp
# Configure in ChatGPT Custom Instructions or use via API
```

### Cursor
```bash
# Install server
cargo install --git https://github.com/dominicburkart/velib-mcp.git
# Add to Cursor's settings.json
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
```bash
# Install server
cargo install --git https://github.com/dominicburkart/velib-mcp.git
# Run server and use via API calls
velib-mcp --port 8080
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
```

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