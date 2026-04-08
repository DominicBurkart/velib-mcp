# MCP Data Schemas — Velib Server

> **Note**: This document captures the original design-phase type definitions.
> The authoritative, implemented types live in [`src/types.rs`](../../src/types.rs).
> Some details below (enum variants, field names) differ from the final implementation.

## Core Types

### Coordinates
```rust
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}
```

### StationStatus
```rust
pub enum StationStatus {
    #[serde(rename = "OPEN")]       Open,
    #[serde(rename = "CLOSED")]     Closed,
    #[serde(rename = "MAINTENANCE")] Maintenance,
}
```

### ServiceCapabilities
```rust
pub struct ServiceCapabilities {
    pub accepts_credit_card: bool,
    pub has_charging_station: bool,
    pub is_virtual_station: bool,
}
```

### BikeAvailability
```rust
pub struct BikeAvailability {
    pub mechanical: u16,
    pub electric: u16,
}

impl BikeAvailability {
    pub fn total(&self) -> u16 { self.mechanical.saturating_add(self.electric) }
}
```

### DataFreshness
```rust
pub enum DataFreshness {
    Fresh,     // < 10 minutes
    Recent,    // 10–30 minutes
    Stale,     // 30–120 minutes
    VeryStale, // > 120 minutes
}
```

### BikeTypeFilter
```rust
pub enum BikeTypeFilter {
    #[serde(rename = "mechanical")] MechanicalOnly,
    #[serde(rename = "electric")]   ElectricOnly,
    #[serde(rename = "any")]        AnyType,
}
```

## Station Types

### StationReference
```rust
pub struct StationReference {
    pub station_code: String,
    pub name: String,
    pub coordinates: Coordinates,
    pub capacity: u16,
    pub capabilities: ServiceCapabilities,
}
```

### RealTimeStatus
```rust
pub struct RealTimeStatus {
    pub bikes: BikeAvailability,
    pub available_docks: u16,
    pub status: StationStatus,
    pub last_update: DateTime<Utc>,
    pub data_freshness: DataFreshness,
}
```

### VelibStation (consolidated view)
```rust
pub struct VelibStation {
    pub reference: StationReference,
    pub real_time: Option<RealTimeStatus>,
}
```

## Request Types

### GeographicQuery
```rust
pub struct GeographicQuery {
    pub center: Coordinates,
    pub radius_meters: u32,
    pub limit: u16,  // default 50
}
```

### AvailabilityFilter
```rust
pub struct AvailabilityFilter {
    pub min_bikes: Option<u16>,
    pub min_docks: Option<u16>,
    pub bike_type: Option<BikeTypeFilter>,
    pub exclude_out_of_service: bool,  // default true
}
```

## Validation

`StationReference::validate()` checks:
- `station_code` and `name` are non-empty
- `capacity` is in range 1–200
- Coordinates are within the Paris metro bounding box (lat 48.7–49.0, lon 2.0–2.6)

`VelibStation::validate()` additionally checks:
- `bikes.total() + available_docks <= capacity`

Coordinate methods:
- `is_valid_paris_metro()` — bounding box check
- `is_within_paris_service_area()` — 50 km radius from Paris City Hall (48.8565°N, 2.3514°E)

## JSON Example

```json
{
  "station_code": "32017",
  "name": "Rouget de L'isle - Watteau",
  "coordinates": { "latitude": 48.936268, "longitude": 2.358866 },
  "capacity": 22,
  "capabilities": {
    "accepts_credit_card": false,
    "has_charging_station": false,
    "is_virtual_station": false
  },
  "real_time": {
    "bikes": { "mechanical": 8, "electric": 4 },
    "available_docks": 10,
    "status": "OPEN",
    "last_update": "<ISO-8601 timestamp>",
    "data_freshness": "Fresh"
  }
}
```
