# CLAUDE.md — Velib MCP Server

## Context

You are a Rust developer working on an open-source MCP server.

- **Working directory**: `~/code/velib-mcp/velib-mcp` (main repo)
- **Worktree layout**: sibling directories (`~/code/velib-mcp/branch1/`, etc.)
- **Available tools**: git, cargo, podman, scw CLI, gh CLI
- **Audience**: AI assistants that need access to Paris Velib data

CLAUDE.md is symlinked into all worktrees so context stays consistent across branches.

### Project Structure

```
~/code/velib-mcp/
├── velib-mcp/          # main repo (this directory)
│   ├── src/            # Rust source
│   └── docs/           # documentation
├── branch1/            # git worktree
│   ├── CLAUDE.md -> ../velib-mcp/CLAUDE.md
│   └── ...
└── branch2/
    ├── CLAUDE.md -> ../velib-mcp/CLAUDE.md
    └── ...
```

## Project Goal

Build a performant cloud MCP server exposing two Paris open datasets to AI assistants:

- **Real-time availability**: <https://opendata.paris.fr/explore/dataset/velib-disponibilite-en-temps-reel/>
- **Station locations**: <https://opendata.paris.fr/explore/dataset/velib-emplacement-des-stations/>

## Status

All phases complete. The server is implemented, tested, and deployed.

- `src/main.rs` — entry point
- `src/mcp/` — MCP protocol implementation
- `src/data/` — data client and cache
- `src/types.rs` — core data types
- `docs/api/data_analysis.md` — dataset field analysis
- `docs/api/mcp_interface_spec.md` — MCP tool/resource specifications

## Development Commands

```bash
cargo test          # run all tests
cargo fmt           # format code
cargo clippy        # lint
cargo audit         # security audit
```

## Deployment

- **Target**: Scaleway Container Serverless
- **Trigger**: push to `main`
- **Registry**: Scaleway Container Registry
- **Build tool**: Podman

## Worktree Management

```bash
# create worktree
git worktree add ../branch-name branch-name
cd ../branch-name
ln -s ../velib-mcp/CLAUDE.md CLAUDE.md

# remove worktree
git worktree remove ../branch-name
git worktree prune
```

## Architecture

- **Language**: Rust
- **Transport**: JSON-RPC 2.0 over HTTP and WebSocket
- **Deployment**: Scaleway Container Serverless (distroless Debian image)
- **CI/CD**: GitHub Actions
- **Approach**: TDD — tests required for all features
- **Security**: 50 km service-area limit enforced on all coordinate inputs
