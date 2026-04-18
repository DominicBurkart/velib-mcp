# MCP Interface Specification — Velib Server

## Server

- **Name**: `velib-mcp`
- **Version**: `1.0.0`
- **Protocol**: JSON-RPC 2.0 over HTTP
- **Default port**: 8080 (set via `PORT` environment variable)
- **Capabilities**: `resources`, `tools`

## Resources

### `velib://stations/reference`
Complete catalogue of stations with static metadata.

```json
{
  "stations": [
    {
      "station_code": "32017",
      "name": "Rouget de L'isle - Watteau",
      "coordinates": { "latitude": 48.936268, "longitude": 2.358866 },
      "capacity": 22
    }
  ],
  "metadata": { "total_stations": 1400 }
}
```

### `velib://stations/realtime`
Current bike and dock availability for all stations.

```json
{
  "stations": [
    {
      "station_code": "32017",
      "bikes": { "mechanical": 8, "electric": 4 },
      "available_docks": 10,
      "status": "OPEN"
    }
  ]
}
```

### `velib://stations/complete`
Consolidated view combining reference and real-time data.

### `velib://health`

```json
{
  "status": "healthy",
  "version": "1.0.0",
  "data_sources": {
    "real_time": { "status": "healthy", "lag_seconds": 45 },
    "reference": { "status": "healthy" }
  }
}
```

## Tools

### `find_nearby_stations`
Finds stations within a radius of a coordinate.

**Input**

| Parameter | Type | Required | Default | Constraints |
|-----------|------|----------|---------|-------------|
| `latitude` | number | yes | — | 48.7–49.0 |
| `longitude` | number | yes | — | 2.0–2.6 |
| `radius_meters` | integer | no | 500 | 100–5000 |
| `limit` | integer | no | 10 | 1–100 |
| `availability_filter.min_bikes` | integer | no | — | ≥ 0 |
| `availability_filter.min_docks` | integer | no | — | ≥ 0 |
| `availability_filter.bike_type` | string | no | `"any"` | `"mechanical"`, `"electric"`, `"any"` |

**Output**: Array of stations with `straight_line_distance_meters`, plus `search_metadata` (query point, radius, count, search time).

**Example**
```json
{
  "name": "find_nearby_stations",
  "arguments": {
    "latitude": 48.8566,
    "longitude": 2.3522,
    "radius_meters": 1000,
    "limit": 5,
    "availability_filter": { "min_bikes": 2, "bike_type": "electric" }
  }
}
```

---

### `get_station_by_code`
Retrieves full information for a specific station.

**Input**

| Parameter | Type | Required | Default |
|-----------|------|----------|---------|
| `station_code` | string (numeric) | yes | — |
| `include_real_time` | boolean | no | `true` |

**Output**: `{ "station": VelibStation | null, "found": boolean }`

---

### `search_stations_by_name`
Text search over station names.

**Input**

| Parameter | Type | Required | Default |
|-----------|------|----------|---------|
| `query` | string (≥ 2 chars) | yes | — |
| `limit` | integer | no | 10 (max 100) |
| `fuzzy` | boolean | no | `true` |

With `fuzzy: true` the query is matched as a substring (case-insensitive, Unicode-normalised). With `fuzzy: false` the name must start with the query.

**Output**: Array of matching `VelibStation` objects plus `search_metadata`.

---

### `get_area_statistics`
Aggregated statistics for a bounding box.

**Input**

| Parameter | Type | Required |
|-----------|------|----------|
| `bounds.north` | number | yes |
| `bounds.south` | number | yes |
| `bounds.east` | number | yes |
| `bounds.west` | number | yes |
| `include_real_time` | boolean | no (default `true`) |

**Output**
```json
{
  "area_stats": {
    "total_stations": 42,
    "operational_stations": 40,
    "total_capacity": 840,
    "available_bikes": { "mechanical": 120, "electric": 80, "total": 200 },
    "available_docks": 600,
    "occupancy_rate": 0.24
  },
  "bounds": { "north": 48.87, "south": 48.85, "east": 2.36, "west": 2.33 }
}
```

---

### `plan_bike_journey`
Suggests pickup and dropoff stations for a journey.

**Input**

| Parameter | Type | Required | Default |
|-----------|------|----------|---------|
| `origin.latitude` | number | yes | — |
| `origin.longitude` | number | yes | — |
| `destination.latitude` | number | yes | — |
| `destination.longitude` | number | yes | — |
| `preferences.bike_type` | string | no | `"any"` |
| `preferences.max_walk_distance` | integer (metres) | no | 500 |

Both origin and destination must be within the Paris metro bounds and within 50 km of Paris City Hall.

**Output**: `pickup_stations`, `dropoff_stations` (up to 3 each), and `recommendations` with `confidence_score` (0–1).

## Error Codes

| Code | Meaning |
|------|---------|
| `-32602` | Invalid parameters (bad coordinates, radius too large, limit exceeded) |
| `-32600` | Station not found |
| `-32700` | JSON parse error |
| `-32603` | Internal / cache / protocol error |
| `-32001` | HTTP or rate-limit error from upstream API |

## Rate Limiting

- Resources: 60 requests/minute
- Tools: 100 requests/minute
- Burst: 10 requests/second

Response headers: `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`.

## Authentication

No authentication required. Rate limiting is applied per IP.
