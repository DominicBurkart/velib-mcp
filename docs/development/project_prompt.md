# Project Prompt Reference

This file preserves the original project brief for historical context. For current development guidance, see [`CLAUDE.md`](../../CLAUDE.md) in the repo root.

## Original Brief

Build a high-performance cloud MCP server in Rust that makes the following Paris Open Data datasets accessible to AI assistants:

- **Real-time availability**: <https://opendata.paris.fr/explore/dataset/velib-disponibilite-en-temps-reel/information/>
- **Station locations**: <https://opendata.paris.fr/explore/dataset/velib-emplacement-des-stations/information/>

**Goal**: Enable AI assistants to answer transport planning and journey flow questions using live Vélib data.

**Constraints**:
- Deploy to Scaleway Container Serverless
- Use TDD throughout
- Support parallel development via git worktrees
- Tools: `git`, `cargo`, `podman`, `scw` CLI, `gh` CLI
