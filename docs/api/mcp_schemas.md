# MCP Data Schemas — Velib Server

This document describes the data schemas used by the Velib MCP server. The canonical definitions live in `src/types.rs`.

## Core Types

### `Coordinates`
Geographic position in decimal degrees.

| Field | Type | Description |
|-------|------|-------------|
| `latitude` | `f64` | Decimal degrees |
| `longitude` | `f64` | Decimal degrees |

### `StationStatus`

| Variant | Serialised as | Meaning |
|---------|---------------|---------|
| `Open` | `"OPEN"` | Renting and returning enabled |
| `Closed` | `"CLOSED"` | Station closed |
| `Maintenance` | `"MAINTENANCE"` | Temporarily out of service |

### `BikeAvailability`

| Field | Type | Description |
|-------|------|-------------|
| `mechanical` | `u16` | Mechanical bikes available |
| `electric` | `u16` | Electric bikes available |

Helper methods: `total()`, `has_bikes()`, `has_mechanical()`, `has_electric()`.

### `DataFreshness`

| Variant | Age threshold |
|---------|---------------|
| `Fresh` | < 10 minutes |
| `Recent` | 10–30 minutes |
| `Stale` | 30–120 minutes |
| `VeryStale` | > 120 minutes |

### `ServiceCapabilities`

| Field | Type | Description |
|-------|------|-------------|
| `accepts_credit_card` | `bool` | Station accepts card payment |
| `has_charging_station` | `bool` | Electric bike charging available |
| `is_virtual_station` | `bool` | Virtual (non-physical) station |

### `BikeTypeFilter`

| Variant | Serialised as | Meaning |
|---------|---------------|---------|
| `MechanicalOnly` | `"mechanical"` | Requires mechanical bikes |
| `ElectricOnly` | `"electric"` | Requires electric bikes |
| `AnyType` (default) | `"any"` | Either type accepted |

## Station Types

### `StationReference`
Static station metadata.

| Field | Type | Description |
|-------|------|-------------|
| `station_code` | `String` | Unique station identifier |
| `name` | `String` | Descriptive station name |
| `coordinates` | `Coordinates` | Geographic position |
| `capacity` | `u16` | Total dock capacity (1–200) |
| `capabilities` | `ServiceCapabilities` | Station service features |

Validation: non-empty code and name, capacity 1–200, coordinates within Paris metro bounds.

### `RealTimeStatus`
Current station state.

| Field | Type | Description |
|-------|------|-------------|
| `bikes` | `BikeAvailability` | Available bikes by type |
| `available_docks` | `u16` | Free docks for returns |
| `status` | `StationStatus` | Operational status |
| `last_update` | `DateTime<Utc>` | Timestamp of last data update |
| `data_freshness` | `DataFreshness` | Age classification |

### `VelibStation`
Consolidated view (reference + real-time).

| Field | Type | Description |
|-------|------|-------------|
| `reference` | `StationReference` | Static metadata |
| `real_time` | `Option<RealTimeStatus>` | Live state, if available |

Validation: `bikes.total() + available_docks <= capacity`.

## Example JSON

```json
{
  "reference": {
    "station_code": "32017",
    "name": "Rouget de L'isle - Watteau",
    "coordinates": { "latitude": 48.936268, "longitude": 2.358866 },
    "capacity": 22,
    "capabilities": {
      "accepts_credit_card": false,
      "has_charging_station": false,
      "is_virtual_station": false
    }
  },
  "real_time": {
    "bikes": { "mechanical": 8, "electric": 4 },
    "available_docks": 10,
    "status": "OPEN",
    "last_update": "2024-03-15T19:31:22Z",
    "data_freshness": "Fresh"
  }
}
```

## Data Sources

| Source | Serialised as |
|--------|---------------|
| `ParisOpenData` | `"paris_open_data"` |
| `Cache` | `"cache"` |
| `Fallback` | `"fallback"` |
