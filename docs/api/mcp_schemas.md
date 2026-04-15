# MCP Data Schemas — Velib Server

> **Note**: These schemas are reference documentation. The authoritative Rust definitions live in [`src/types.rs`](../../src/types.rs). When the two diverge, the source code is correct.

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
    Open,        // renting and returning enabled
    Closed,      // not operational
    Maintenance, // temporarily unavailable
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
    // .total() returns mechanical + electric (saturating)
}
```

### DataFreshness
```rust
pub enum DataFreshness {
    Fresh,     // < 10 minutes old
    Recent,    // 10–30 minutes old
    Stale,     // 30–120 minutes old
    VeryStale, // > 120 minutes old
}
```

## Main Structs

### StationReference
```rust
pub struct StationReference {
    pub station_code: String,
    pub name: String,
    pub coordinates: Coordinates,
    pub capacity: u16,          // validated: 1–200
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
    pub data_freshness: DataFreshness,  // computed from last_update age
}
```

### VelibStation (consolidated view)
```rust
pub struct VelibStation {
    pub reference: StationReference,
    pub real_time: Option<RealTimeStatus>,
}
```

Validation rule: `bikes.total() + available_docks <= capacity`.

## Query Types

### GeographicQuery
```rust
pub struct GeographicQuery {
    pub center: Coordinates,
    pub radius_meters: u32,
    pub limit: u16,  // default 50
}
```

Service area constraint: coordinates must be within 50 km of Paris City Hall (48.8565°N, 2.3514°E).

### AvailabilityFilter
```rust
pub struct AvailabilityFilter {
    pub min_bikes: Option<u16>,
    pub min_docks: Option<u16>,
    pub bike_type: Option<BikeTypeFilter>,
    pub exclude_out_of_service: bool,  // default true
}

pub enum BikeTypeFilter {
    MechanicalOnly,
    ElectricOnly,
    AnyType,  // default
}
```

## Response Types

### StationListResponse
```rust
pub struct StationListResponse {
    pub stations: Vec<VelibStation>,
    pub total_count: usize,
    pub pagination: Option<PaginationInfo>,
    pub metadata: ResponseMetadata,
}
```

### DataSource
```rust
pub enum DataSource {
    ParisOpenData,              // fresh from API
    Cache,                      // served from local cache
    Fallback,                   // backup data
}
```

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
