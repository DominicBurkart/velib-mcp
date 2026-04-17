# CLAUDE.md — Velib MCP Server

## Role
You are an expert Rust developer working on an open-source MCP server project.

- **Working directory**: `~/code/velib-mcp/velib-mcp` (main repo)
- **Worktree layout**: Adjacent branches at `~/code/velib-mcp/<branch>/`
- **Available tools**: git, cargo, podman, scw CLI, gh CLI
- **Target users**: AI assistants needing access to Paris Velib data

This file is symlinked into all worktrees so context is shared.

## Project Goal

A high-performance cloud MCP server exposing two Paris open datasets to AI assistants:

- **Real-time availability**: <https://opendata.paris.fr/explore/dataset/velib-disponibilite-en-temps-reel/>
- **Station locations**: <https://opendata.paris.fr/explore/dataset/velib-emplacement-des-stations/>

## Repository Layout

```
src/
  main.rs          # entry point
  mcp/             # MCP protocol implementation
  data/            # data client and cache
  types.rs         # core data structures
  error.rs         # error types
docs/
  api/             # data analysis and MCP interface specs
  context/         # project status tracking
  development/     # development notes
tests/             # integration tests
```

## Development Commands

```bash
cargo test                                          # run all tests
cargo fmt                                           # format code
cargo clippy --all-targets --all-features -- -D warnings  # lint
cargo audit                                         # security audit
```

## Deployment

Deployed to Scaleway Container Serverless on push to `main` via GitHub Actions.

## Worktree Management

```bash
# Create a new worktree
git worktree add ../branch-name branch-name
cd ../branch-name
ln -s ../velib-mcp/CLAUDE.md CLAUDE.md

# Remove a worktree
git worktree remove ../branch-name
git worktree prune
```
