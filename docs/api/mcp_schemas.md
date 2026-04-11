# MCP Data Schemas

The authoritative type definitions are in the source code:

| File | Contents |
|------|----------|
| [`src/types.rs`](../../src/types.rs) | Core domain types: `Coordinates`, `StationReference`, `RealTimeStatus`, `VelibStation`, `BikeAvailability`, `DataFreshness`, `StationStatus` |
| [`src/mcp/types.rs`](../../src/mcp/types.rs) | MCP tool inputs/outputs, JSON-RPC types, `GeographicBounds`, `AvailabilityFilter` |
| [`src/error.rs`](../../src/error.rs) | `Error` enum with MCP error codes |

## Key Types Summary

### `VelibStation`
Combines static reference data with optional real-time status.
- `reference: StationReference` — code, name, coordinates, capacity
- `real_time: Option<RealTimeStatus>` — bike counts, dock count, status, last update

### `StationStatus`
`Open` | `Closed` | `Maintenance`

### `DataFreshness`
`Fresh` (<10 min) | `Recent` (10–30 min) | `Stale` (30–120 min) | `VeryStale` (>120 min)

### Tool I/O types
Each MCP tool has a matching `*Input` / `*Output` struct in [`src/mcp/types.rs`](../../src/mcp/types.rs):
- `FindNearbyStationsInput` / `FindNearbyStationsOutput`
- `GetStationByCodeInput` / `GetStationByCodeOutput`
- `SearchStationsByNameInput` / `SearchStationsByNameOutput`
- `GetAreaStatisticsInput` / `GetAreaStatisticsOutput`
- `PlanBikeJourneyInput` / `PlanBikeJourneyOutput`
