# CLAUDE.md — Velib MCP Server

## Project Overview

A Rust MCP server that exposes Paris Vélib bike-sharing data to AI assistants.
Two upstream datasets are consumed:

- **Real-time availability**: `https://opendata.paris.fr/explore/dataset/velib-disponibilite-en-temps-reel/`
- **Station locations**: `https://opendata.paris.fr/explore/dataset/velib-emplacement-des-stations/`

## Source Layout

```
src/
├── main.rs            # Entry point
├── lib.rs             # Public re-exports
├── types.rs           # Shared domain types (Coordinates, VelibStation, …)
├── error.rs           # Error enum with MCP error-code mapping
├── data/
│   ├── client.rs      # VelibDataClient — fetches and caches API data
│   ├── cache.rs       # Generic in-memory TTL cache
│   └── retry.rs       # RetryPolicy / RetryableHttpClient
├── mcp/
│   ├── server.rs      # McpServer — JSON-RPC routing (HTTP + WebSocket)
│   ├── handlers.rs    # McpToolHandler — tool implementations
│   └── types.rs       # MCP request/response types
└── server/
    ├── mod.rs         # Server struct, /health route
    └── config.rs      # parse_server_address (PORT / IP env vars)
```

## Development Commands

```bash
cargo test                                          # run all tests
cargo fmt                                           # format
cargo clippy --all-targets --all-features -- -D warnings  # lint
cargo audit                                         # dependency security audit
cargo deny check licenses bans sources              # license / supply-chain check
```

## Key Constraints

- **Service area**: all coordinate inputs are validated against a 50 km radius
  centred on Paris City Hall (`PARIS_CITY_HALL` in `src/types.rs`).
- **Search limits**: maximum search radius 5 000 m; maximum result limit 100.
- **Cache TTLs**: reference data 5 min, real-time data 2 min
  (`REFERENCE_CACHE_TTL_MINUTES` / `REALTIME_CACHE_TTL_MINUTES` in
  `src/data/client.rs`).

## Deployment

Deployed to Scaleway Container Serverless on push to `main` via GitHub Actions.
Runtime configuration uses `PORT` (default `8080`) and `IP` (default `0.0.0.0`)
environment variables.
