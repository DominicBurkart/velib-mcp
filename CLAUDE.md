# CLAUDE.md — Velib MCP Server

Rust expert working on an open-source MCP server that exposes Paris Vélib bike-sharing data to AI assistants.

## Project Goal

Expose two Paris Open Data datasets through the Model Context Protocol (MCP):

- **Real-time availability**: <https://opendata.paris.fr/explore/dataset/velib-disponibilite-en-temps-reel/>
- **Station locations**: <https://opendata.paris.fr/explore/dataset/velib-emplacement-des-stations/>

## Key Files

| Path | Purpose |
|------|---------|
| `src/main.rs` | Entry point |
| `src/lib.rs` | Public API surface |
| `src/types.rs` | Core data types |
| `src/error.rs` | Error types and MCP error codes |
| `src/data/` | HTTP client and cache |
| `src/mcp/` | MCP protocol implementation |
| `src/server/` | Axum HTTP server |
| `docs/api/data_analysis.md` | Vélib dataset field reference |
| `docs/api/mcp_interface_spec.md` | MCP tool/resource specifications |
| `docs/api/mcp_schemas.md` | Data schema reference (see `src/types.rs` for authoritative definitions) |

## Development Commands

```bash
cargo test                                        # run all tests
cargo fmt                                         # format code
cargo clippy --all-targets --all-features -- -D warnings  # lint
cargo audit                                       # security audit
```

## Architecture

- **Language**: Rust (latest stable)
- **Server**: Axum over HTTP/WebSocket
- **Deployment**: Scaleway Container Serverless via GitHub Actions on `main`
- **Container**: Podman, Debian distroless base
- **Approach**: TDD — write failing tests first

## Worktrees

Branch worktrees live adjacent to the main repo:

```
~/code/velib-mcp/
├── velib-mcp/     # main repo
├── branch1/       # feature worktree
└── branch2/       # another worktree
```

```bash
# Create
git worktree add ../branch-name branch-name

# Remove
git worktree remove ../branch-name && git worktree prune
```

## Deployment

Triggered automatically on push to `main`. Uses `deploy_to_scaleway.sh` and the env vars in `.env.deploy.example`.
