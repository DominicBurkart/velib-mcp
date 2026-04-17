# MCP Data Schemas

This document defines the canonical data types used by the Velib MCP server. The authoritative Rust source is [`src/types.rs`](../../src/types.rs).

## Core Types

### `Coordinates`

```rust
pub struct Coordinates {
    pub latitude: f64,   // decimal degrees
    pub longitude: f64,  // decimal degrees
}
```

Valid Paris metro area: latitude 48.7–49.0, longitude 2.0–2.6.
Service area check: within 50 km of Paris City Hall (48.8565° N, 2.3514° E).

### `StationStatus`

```rust
pub enum StationStatus {
    #[serde(rename = "OPEN")]        Open,
    #[serde(rename = "CLOSED")]      Closed,
    #[serde(rename = "MAINTENANCE")] Maintenance,
}
```

### `ServiceCapabilities`

```rust
pub struct ServiceCapabilities {
    pub accepts_credit_card: bool,
    pub has_charging_station: bool,
    pub is_virtual_station: bool,
}
```

### `BikeAvailability`

```rust
pub struct BikeAvailability {
    pub mechanical: u16,
    pub electric: u16,
}
```

`total()` returns `mechanical + electric` (saturating).

### `DataFreshness`

```rust
pub enum DataFreshness {
    Fresh,     // < 10 minutes
    Recent,    // 10–30 minutes
    Stale,     // 30–120 minutes
    VeryStale, // > 120 minutes
}
```

### `BikeTypeFilter`

```rust
pub enum BikeTypeFilter {
    #[serde(rename = "mechanical")] MechanicalOnly,
    #[serde(rename = "electric")]   ElectricOnly,
    #[serde(rename = "any")]        AnyType,  // default
}
```

## Station Types

### `StationReference`

```rust
pub struct StationReference {
    pub station_code: String,
    pub name: String,
    pub coordinates: Coordinates,
    pub capacity: u16,          // validated: 1–200
    pub capabilities: ServiceCapabilities,
}
```

### `RealTimeStatus`

```rust
pub struct RealTimeStatus {
    pub bikes: BikeAvailability,
    pub available_docks: u16,
    pub status: StationStatus,
    pub last_update: DateTime<Utc>,
    pub data_freshness: DataFreshness,  // derived from last_update age
}
```

### `VelibStation`

```rust
pub struct VelibStation {
    pub reference: StationReference,
    pub real_time: Option<RealTimeStatus>,
}
```

Validation invariant: `bikes.total() + available_docks ≤ capacity`.

## Query / Response Types

### `DataSource`

```rust
pub enum DataSource {
    #[serde(rename = "paris_open_data")] ParisOpenData,
    #[serde(rename = "cache")]           Cache,
    #[serde(rename = "fallback")]        Fallback,
}
```

## Example JSON

```json
{
  "reference": {
    "station_code": "32017",
    "name": "Rouget de L'isle - Watteau",
    "coordinates": { "latitude": 48.936268, "longitude": 2.358866 },
    "capacity": 22,
    "capabilities": {
      "accepts_credit_card": true,
      "has_charging_station": false,
      "is_virtual_station": false
    }
  },
  "real_time": {
    "bikes": { "mechanical": 8, "electric": 4 },
    "available_docks": 10,
    "status": "OPEN",
    "last_update": "2025-06-14T19:31:22Z",
    "data_freshness": "Fresh"
  }
}
```
