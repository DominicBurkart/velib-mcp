# CLAUDE.md — Velib MCP Server

## Project Overview

Rust MCP server exposing Paris Velib bike-sharing data to AI assistants.

- **Data sources**: real-time station availability + static station locations (Paris Open Data API)
- **Deployment**: Scaleway Container Serverless via GitHub Actions (`main` branch push)
- **Tools**: git, cargo, podman, `scw` CLI, `gh` CLI

## Key Files

| Path | Purpose |
|------|---------|
| `src/main.rs` | Entry point |
| `src/mcp/` | MCP protocol implementation |
| `src/data/` | Data client and cache |
| `src/types.rs` | Core data structures |
| `docs/api/data_analysis.md` | Velib API field reference |
| `docs/api/mcp_interface_spec.md` | MCP tool/resource specs |

## Development Commands

```bash
cargo test          # Run all tests
cargo fmt           # Format code
cargo clippy        # Lint
cargo audit         # Security audit
```

## Worktree Setup

```bash
# Create worktree for a feature branch
git worktree add ../branch-name branch-name
ln -s ../velib-mcp/CLAUDE.md ../branch-name/CLAUDE.md

# Remove worktree
git worktree remove ../branch-name
git worktree prune
```

## Architecture

- `src/data/client.rs` — fetches and caches reference + real-time data from Paris Open Data API
- `src/mcp/handlers.rs` — implements the five MCP tools
- `src/mcp/server.rs` — JSON-RPC 2.0 HTTP/WebSocket server (axum)
- `src/server/` — address parsing and top-level axum router
- Cache TTLs: 2 min (real-time), 5 min (reference)
- Service area validation: 50 km radius from Paris City Hall

## Development Process

- TDD: write failing tests before implementation
- Each feature in its own branch/PR
- CI must pass before merge (fmt, clippy, audit, tests)
