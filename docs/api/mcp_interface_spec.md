# MCP Interface Specification

## Server Info

- **Name**: `velib-mcp`
- **Version**: `1.0.0`
- **Capabilities**: `resources`, `tools`
- **Transport**: JSON-RPC 2.0 over HTTP/WebSocket, UTF-8, default port 8080

## Resources

### `velib://stations/reference`

Full catalogue of Velib stations with static metadata.

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
    "last_updated": "2025-06-14T06:00:00Z"
  }
}
```

### `velib://stations/realtime`

Current availability for all stations.

```json
{
  "stations": [
    {
      "station_code": "32017",
      "bikes": { "mechanical": 8, "electric": 4 },
      "available_docks": 10,
      "status": "OPEN",
      "last_updated": "2025-06-14T19:31:22Z"
    }
  ],
  "metadata": {
    "data_freshness": "Fresh",
    "response_time": "2025-06-14T19:31:25Z"
  }
}
```

### `velib://stations/complete`

Merged view combining reference and real-time data.

### `velib://health`

```json
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime_seconds": 86400,
  "data_sources": {
    "real_time": { "status": "healthy", "last_update": "2025-06-14T19:31:22Z", "lag_seconds": 45 },
    "reference": { "status": "healthy", "last_update": "2025-06-14T06:00:00Z" }
  },
  "cache_stats": { "hit_rate": 0.85, "entries": 1400 }
}
```

## Tools

### `find_nearby_stations`

Find stations within a radius of a coordinate.

**Input**

```json
{
  "type": "object",
  "required": ["latitude", "longitude"],
  "properties": {
    "latitude":      { "type": "number",  "minimum": 48.7, "maximum": 49.0 },
    "longitude":     { "type": "number",  "minimum": 2.0,  "maximum": 2.6 },
    "radius_meters": { "type": "integer", "minimum": 100,  "maximum": 5000, "default": 500 },
    "limit":         { "type": "integer", "minimum": 1,    "maximum": 100,  "default": 10 },
    "availability_filter": {
      "type": "object",
      "properties": {
        "min_bikes": { "type": "integer", "minimum": 0 },
        "min_docks": { "type": "integer", "minimum": 0 },
        "bike_type": { "type": "string",  "enum": ["mechanical", "electric", "any"], "default": "any" }
      }
    }
  }
}
```

**Output**

```json
{
  "type": "object",
  "properties": {
    "stations": { "type": "array", "items": { "$ref": "#/definitions/VelibStation" } },
    "search_metadata": {
      "type": "object",
      "properties": {
        "query_point":    { "type": "object", "properties": { "latitude": {"type": "number"}, "longitude": {"type": "number"} } },
        "radius_meters":  { "type": "integer" },
        "total_found":    { "type": "integer" },
        "search_time_ms": { "type": "integer" }
      }
    }
  }
}
```

**Example call**

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

### `get_station_by_code`

Fetch complete information for a specific station.

**Input**

```json
{
  "type": "object",
  "required": ["station_code"],
  "properties": {
    "station_code":     { "type": "string",  "pattern": "^[0-9]+$" },
    "include_real_time": { "type": "boolean", "default": true }
  }
}
```

**Output**

```json
{
  "type": "object",
  "properties": {
    "station": { "$ref": "#/definitions/VelibStation" },
    "found":   { "type": "boolean" }
  }
}
```

### `search_stations_by_name`

Text search over station names.

**Input**

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

### `get_area_statistics`

Aggregated statistics for a geographic bounding box.

**Input**

```json
{
  "type": "object",
  "required": ["bounds"],
  "properties": {
    "bounds": {
      "type": "object",
      "required": ["north", "south", "east", "west"],
      "properties": {
        "north": { "type": "number" },
        "south": { "type": "number" },
        "east":  { "type": "number" },
        "west":  { "type": "number" }
      }
    },
    "include_real_time": { "type": "boolean", "default": true }
  }
}
```

**Output**

```json
{
  "type": "object",
  "properties": {
    "area_stats": {
      "type": "object",
      "properties": {
        "total_stations":      { "type": "integer" },
        "operational_stations": { "type": "integer" },
        "total_capacity":      { "type": "integer" },
        "available_bikes": {
          "type": "object",
          "properties": {
            "mechanical": { "type": "integer" },
            "electric":   { "type": "integer" },
            "total":      { "type": "integer" }
          }
        },
        "available_docks": { "type": "integer" },
        "occupancy_rate":  { "type": "number", "minimum": 0, "maximum": 1 }
      }
    }
  }
}
```

### `plan_bike_journey`

Suggest pickup and dropoff stations for a journey.

**Input**

```json
{
  "type": "object",
  "required": ["origin", "destination"],
  "properties": {
    "origin":      { "type": "object", "required": ["latitude", "longitude"], "properties": { "latitude": {"type": "number"}, "longitude": {"type": "number"} } },
    "destination": { "type": "object", "required": ["latitude", "longitude"], "properties": { "latitude": {"type": "number"}, "longitude": {"type": "number"} } },
    "preferences": {
      "type": "object",
      "properties": {
        "bike_type":          { "type": "string",  "enum": ["mechanical", "electric", "any"], "default": "any" },
        "max_walk_distance": { "type": "integer", "default": 500, "description": "Max walking distance in meters" }
      }
    }
  }
}
```

**Output**

```json
{
  "type": "object",
  "properties": {
    "journey": {
      "type": "object",
      "properties": {
        "pickup_stations":  { "type": "array", "items": { "$ref": "#/definitions/StationWithDistance" } },
        "dropoff_stations": { "type": "array", "items": { "$ref": "#/definitions/StationWithDistance" } },
        "recommendations": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "pickup_station":   { "$ref": "#/definitions/VelibStation" },
              "dropoff_station":  { "$ref": "#/definitions/VelibStation" },
              "walk_to_pickup":   { "type": "integer" },
              "walk_from_dropoff": { "type": "integer" },
              "confidence_score": { "type": "number", "minimum": 0, "maximum": 1 }
            }
          }
        }
      }
    }
  }
}
```

## Error Codes

| Code | Constant | Meaning |
|------|----------|---------|
| `-32001` | `STATION_NOT_FOUND` | Station code does not exist |
| `-32002` | `INVALID_COORDINATES` | Coordinates outside service area |
| `-32003` | `REALTIME_UNAVAILABLE` | Real-time data temporarily unavailable |
| `-32004` | `RADIUS_TOO_LARGE` | Search radius exceeds maximum |
| `-32005` | `LIMIT_EXCEEDED` | Result limit exceeds maximum |

**Error response shape**

```json
{ "error": { "code": -32001, "message": "Station not found", "data": { "station_code": "99999", "error_type": "STATION_NOT_FOUND" } } }
```

## Rate Limits

| Scope | Limit |
|-------|-------|
| Resources | 60 req/min |
| Tools | 100 req/min |
| Burst | 10 req/s |

Response headers: `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`.

## Authentication

Currently none required. Rate limiting is applied per IP. API key support is planned.
