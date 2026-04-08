# MCP Interface Specification — Velib Server

## Overview

This document defines the MCP resources and tools exposed by the Velib server.

## Server Identity

- **Name**: `velib-mcp`
- **Version**: `1.0.0`
- **Capabilities**: `resources`, `tools`
- **Transport**: JSON-RPC 2.0 over HTTP (`POST /mcp`) and WebSocket (`/mcp/ws`)
- **Default port**: 8080

## Resources

### `velib://stations/reference`

Static catalog of all Velib stations.

```json
{
  "stations": [
    {
      "station_code": "32017",
      "name": "Rouget de L'isle - Watteau",
      "coordinates": { "latitude": 48.936268, "longitude": 2.358866 },
      "capacity": 22,
      "commune": "Issy-les-Moulineaux"
    }
  ],
  "metadata": {
    "total_stations": 1400,
    "last_updated": "<ISO-8601 timestamp>"
  }
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
      "status": "OPEN",
      "last_update": "<ISO-8601 timestamp>",
      "data_freshness": "Fresh"
    }
  ],
  "metadata": {
    "total_stations": 1400,
    "response_time": "<ISO-8601 timestamp>"
  }
}
```

### `velib://stations/complete`

Combined reference and real-time data for all stations.

### `velib://health`

Service health and cache statistics.

```json
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime_seconds": 86400,
  "data_sources": {
    "real_time": { "status": "healthy", "last_update": "<ISO-8601 timestamp>", "lag_seconds": 45 },
    "reference": { "status": "healthy", "last_update": "<ISO-8601 timestamp>" }
  },
  "cache_stats": { "hit_rate": 0.85, "entries": 1400 }
}
```

## Tools

### `find_nearby_stations`

Find stations within a radius of a coordinate point.

**Input**:
```json
{
  "type": "object",
  "required": ["latitude", "longitude"],
  "properties": {
    "latitude":       { "type": "number",  "minimum": 48.7, "maximum": 49.0 },
    "longitude":      { "type": "number",  "minimum": 2.0,  "maximum": 2.6 },
    "radius_meters":  { "type": "integer", "minimum": 100,  "maximum": 5000, "default": 500 },
    "limit":          { "type": "integer", "minimum": 1,    "maximum": 100,  "default": 10 },
    "availability_filter": {
      "type": "object",
      "properties": {
        "min_bikes": { "type": "integer", "minimum": 0 },
        "min_docks": { "type": "integer", "minimum": 0 },
        "bike_type": { "type": "string", "enum": ["mechanical", "electric", "any"], "default": "any" }
      }
    }
  }
}
```

**Output**:
```json
{
  "stations": [ "<StationWithDistance>" ],
  "search_metadata": {
    "query_point": { "latitude": 48.8566, "longitude": 2.3522 },
    "radius_meters": 1000,
    "total_found": 5,
    "search_time_ms": 42
  }
}
```

**Example call**:
```json
{
  "name": "find_nearby_stations",
  "arguments": {
    "latitude": 48.8566, "longitude": 2.3522,
    "radius_meters": 1000, "limit": 5,
    "availability_filter": { "min_bikes": 2, "bike_type": "electric" }
  }
}
```

---

### `get_station_by_code`

Get full details for a specific station.

**Input**:
```json
{
  "type": "object",
  "required": ["station_code"],
  "properties": {
    "station_code":      { "type": "string", "pattern": "^[0-9]+$" },
    "include_real_time": { "type": "boolean", "default": true }
  }
}
```

**Output**:
```json
{ "found": true, "station": "<VelibStation>" }
```

---

### `search_stations_by_name`

Text search across station names.

**Input**:
```json
{
  "type": "object",
  "required": ["query"],
  "properties": {
    "query": { "type": "string", "minLength": 2 },
    "limit": { "type": "integer", "minimum": 1, "maximum": 50, "default": 10 },
    "fuzzy": { "type": "boolean", "default": true }
  }
}
```

---

### `get_area_statistics`

Aggregated stats for a geographic bounding box.

**Input**:
```json
{
  "type": "object",
  "required": ["bounds"],
  "properties": {
    "bounds": {
      "type": "object",
      "required": ["north", "south", "east", "west"],
      "properties": {
        "north": { "type": "number" }, "south": { "type": "number" },
        "east":  { "type": "number" }, "west":  { "type": "number" }
      }
    },
    "include_real_time": { "type": "boolean", "default": true }
  }
}
```

**Output**:
```json
{
  "area_stats": {
    "total_stations": 42,
    "operational_stations": 40,
    "total_capacity": 840,
    "available_bikes": { "mechanical": 120, "electric": 80, "total": 200 },
    "available_docks": 640,
    "occupancy_rate": 0.24
  },
  "bounds": { "north": 48.9, "south": 48.8, "east": 2.4, "west": 2.3 }
}
```

---

### `plan_bike_journey`

Suggest pickup and dropoff stations for a journey.

**Input**:
```json
{
  "type": "object",
  "required": ["origin", "destination"],
  "properties": {
    "origin":      { "type": "object", "required": ["latitude","longitude"], "properties": { "latitude": {"type":"number"}, "longitude": {"type":"number"} } },
    "destination": { "type": "object", "required": ["latitude","longitude"], "properties": { "latitude": {"type":"number"}, "longitude": {"type":"number"} } },
    "preferences": {
      "type": "object",
      "properties": {
        "bike_type":         { "type": "string", "enum": ["mechanical","electric","any"], "default": "any" },
        "max_walk_distance": { "type": "integer", "default": 500, "description": "Max walking distance in meters" }
      }
    }
  }
}
```

**Output**:
```json
{
  "journey": {
    "pickup_stations":  [ "<StationWithDistance>" ],
    "dropoff_stations": [ "<StationWithDistance>" ],
    "recommendations": [
      {
        "pickup_station":  "<VelibStation>",
        "dropoff_station": "<VelibStation>",
        "straight_line_to_pickup_meters":   120,
        "straight_line_from_dropoff_meters": 85,
        "confidence_score": 0.87
      }
    ]
  }
}
```

## Error Codes

| Code    | Meaning |
|---------|---------|
| `-32700` | Parse error (invalid JSON) |
| `-32602` | Invalid params (bad coordinates, radius too large, etc.) |
| `-32600` | Invalid request (station not found) |
| `-32603` | Internal error |

## Rate Limits

- Resources: 60 req/min
- Tools: 100 req/min
- Burst: 10 req/s

Response headers: `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`

## Authentication

Currently unauthenticated (rate-limited by IP). API key support is planned.
