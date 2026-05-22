# CLAUDE.md - Velib MCP Server

Guidance for Claude Code agents working in this repository. User-facing
overview lives in [`README.md`](README.md); current project status lives in
[`docs/context/etat_actuel.md`](docs/context/etat_actuel.md).

## Project

Rust MCP server exposing two Paris Open Data Velib datasets to AI assistants:

- Real-time availability: <https://opendata.paris.fr/explore/dataset/velib-disponibilite-en-temps-reel/>
- Station locations: <https://opendata.paris.fr/explore/dataset/velib-emplacement-des-stations/>

Five MCP tools (`find_nearby_stations`, `get_station_by_code`,
`search_stations_by_name`, `get_area_statistics`, `plan_bike_journey`) and
four resources (`velib://stations/{reference,realtime,complete}`,
`velib://health`). Service area is bounded to 50 km of Paris City Hall.

## Layout

- `src/main.rs` - binary entry point
- `src/lib.rs` - public re-exports
- `src/mcp/` - MCP protocol, handlers, types
- `src/data/` - upstream client, cache, retry
- `src/server/` - HTTP/WebSocket server and config
- `src/types.rs` - core domain types and constants (e.g. `MAX_DISTANCE_FROM_CITY_HALL_M`)
- `src/error.rs` - error enum and `Result` alias
- `tests/` - integration tests
- `docs/api/` - protocol specs and data analysis
- `docs/context/etat_actuel.md` - current status (single source of truth)

## Commands

```bash
cargo test                                                  # unit + integration tests
cargo fmt --all                                             # format
cargo clippy --all-targets --all-features -- -D warnings    # lint
cargo deny check licenses bans sources                      # license/bans/sources
cargo audit                                                 # security advisories
```

A `cargo-husky` pre-commit hook (`.cargo-husky/hooks/pre-commit`) runs
`clippy --fix`, `fmt`, `cargo sort`, and `cargo deny`. Any non-zero exit
aborts the commit. Do not use `--no-verify`.

## Deployment

Push to `main` triggers `.github/workflows/deploy.yml`, which builds a
Podman image and deploys to Scaleway Container Serverless (registry
`rg.fr-par.scw.cloud/<namespace>`, region `fr-par`). The runtime base is
distroless Debian. The server reads `PORT` (default `8080`) and `IP`
(default `0.0.0.0`) from the environment.

## Worktree convention

When working on a parallel branch, link this file rather than copying it:

```bash
git worktree add ../<branch-name> <branch-name>
cd ../<branch-name>
ln -s ../velib-mcp/CLAUDE.md CLAUDE.md
git worktree remove ../<branch-name>   # when done
```

## Conventions for agents

- Code pointers in prose: `path:line` (e.g. `src/mcp/handlers.rs:124`).
- Dates: ISO 8601 (`YYYY-MM-DD` or full RFC 3339).
- Repository URLs: lowercase `https://github.com/dominicburkart/velib-mcp`.
- Keep new docs in `docs/`; avoid duplicating facts already in `README.md`,
  `etat_actuel.md`, or the `docs/api/` specs - link instead.
- Update `docs/context/etat_actuel.md` (and its `Dernière mise à jour`
  date) when project status changes.
