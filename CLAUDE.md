# Velib MCP — Claude Context

## Role
Rust developer on an open-source MCP server project.

- **Working directory**: `~/code/velib-mcp/velib-mcp` (main repo)
- **Worktree layout**: Adjacent branches at `~/code/velib-mcp/<branch>/`
- **Available tools**: git, cargo, podman, scw CLI, gh CLI
- **Target users**: AI assistants needing access to Paris Velib data

## Project Goal
A cloud MCP server that exposes two Paris open datasets to AI assistants:

- **Real-time availability**: https://opendata.paris.fr/explore/dataset/velib-disponibilite-en-temps-reel/
- **Station locations**: https://opendata.paris.fr/explore/dataset/velib-emplacement-des-stations/

## Project Status

### Completed Phases ✅
- **Phase 0**: Project setup, CI/CD, documentation structure
- **Phase 1**: Full Velib data analysis (15+ fields documented)
- **Phase 2A**: Environment setup and base server foundation
- **Phase 2B**: MCP protocol foundation and base types
- **Phase 3A**: Live API integration and data client
- **Phase 3B**: Full MCP handlers with live data integration
- **Phase 4**: Repository structure cleanup (committed worktrees removed)

### Key Source Files
- `src/main.rs` — entry point
- `src/mcp/` — MCP protocol implementation
- `src/data/` — data client and cache
- `src/types.rs` — core data structures
- `docs/api/data_analysis.md` — full data analysis
- `docs/api/mcp_interface_spec.md` — MCP tool specifications
- `docs/api/mcp_schemas.md` — data schemas

## Development Commands

```bash
cargo test                     # run all tests
cargo fmt                      # format code
cargo clippy                   # static analysis
cargo audit                    # security audit
```

## Deployment

- **Target**: Scaleway Container Serverless
- **Trigger**: Push to main branch
- **Registry**: Scaleway Container Registry
- **Build**: Podman containerisation
- **Config**: `IP` and `PORT` environment variables (defaults: `0.0.0.0:8080`)

## Worktree Management

```bash
# Create worktree
git worktree add ../branch-name branch-name
cd ../branch-name
ln -s ../velib-mcp/CLAUDE.md CLAUDE.md

# Remove worktree
git worktree remove ../branch-name
git worktree prune
```

**Note**: This file is shared via symlinks across all worktrees for consistent context.
