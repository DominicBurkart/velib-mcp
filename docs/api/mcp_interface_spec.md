# MCP Interface Specification

## Server

- **Name**: `velib-mcp`
- **Transport**: JSON-RPC 2.0 over HTTP (`POST /mcp`) and WebSocket (`/mcp/ws`)
- **Default port**: 8080 (configurable via `PORT` env var)
- **Authentication**: none required

## MCP Resources

| URI | Description |
|-----|-------------|
| `velib://stations/reference` | Static station catalog (name, coordinates, capacity) |
| `velib://stations/realtime` | Current bike/dock availability for all stations |
| `velib://stations/complete` | Reference + real-time combined |
| `velib://health` | Service health and cache stats |

Resources are also accessible via `GET /resources/<uri>`.

## MCP Tools

### `find_nearby_stations`

Find stations within a radius of a point.

**Input**

| Field | Type | Default | Constraints |
|-------|------|---------|-------------|
| `latitude` | number | required | 48.7–49.0 |
| `longitude` | number | required | 2.0–2.6 |
| `radius_meters` | integer | 500 | 100–5000 |
| `limit` | integer | 10 | 1–100 |
| `availability_filter` | object | — | see below |

`availability_filter` fields: `min_bikes` (integer), `min_docks` (integer), `bike_type` (`"mechanical"` \| `"electric"` \| `"any"`), `exclude_out_of_service` (boolean, default true).

**Output**: `{ stations: StationWithDistance[], search_metadata: { query_point, radius_meters, total_found, search_time_ms } }`

---

### `get_station_by_code`

Get full details for one station.

**Input**: `station_code` (string, required), `include_real_time` (boolean, default true)

**Output**: `{ station: VelibStation | null, found: boolean }`

---

### `search_stations_by_name`

Text search across station names.

**Input**: `query` (string, min length 2, required), `limit` (integer, default 10, max 100), `fuzzy` (boolean, default true)

**Output**: `{ stations: VelibStation[], search_metadata: { query, total_found, fuzzy_enabled, search_time_ms } }`

---

### `get_area_statistics`

Aggregated stats for a bounding box.

**Input**: `bounds` (object with `north`, `south`, `east`, `west`; required), `include_real_time` (boolean, default true)

**Output**:
```json
{
  "area_stats": {
    "total_stations": 42,
    "operational_stations": 40,
    "total_capacity": 840,
    "available_bikes": { "mechanical": 200, "electric": 80, "total": 280 },
    "available_docks": 560,
    "occupancy_rate": 0.33
  },
  "bounds": { "north": 48.86, "south": 48.85, "east": 2.36, "west": 2.34 }
}
```

---

### `plan_bike_journey`

Suggest pickup and dropoff stations for a journey.

**Input**: `origin` (`{latitude, longitude}`, required), `destination` (`{latitude, longitude}`, required), `preferences` (optional: `bike_type`, `max_walk_distance` in meters, default 500)

**Output**: `{ journey: { pickup_stations, dropoff_stations, recommendations } }`

Each recommendation includes `pickup_station`, `dropoff_station`, walking distances, and a `confidence_score` (0–1).

---

## Error Codes

Errors follow JSON-RPC 2.0 (`{"code": N, "message": "...", "data": {"error_type": "..."}}`).

| Code | Meaning |
|------|---------|
| -32700 | Parse error |
| -32603 | Internal error |
| -32602 | Invalid parameters (coordinates, radius, limit) |
| -32600 | Station not found |
| -32001 | HTTP / rate-limit error from upstream API |

See [`src/error.rs`](../../src/error.rs) for the full mapping.
