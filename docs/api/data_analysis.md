# Velib Data Analysis

## Overview

Two datasets are available via the Paris Open Data API:

1. **Real-time availability** — current station state (bikes/docks available)
2. **Station locations** — static station reference data

## Dataset 1: Real-Time Availability

### Endpoint

```
https://opendata.paris.fr/api/records/1.0/search/?dataset=velib-disponibilite-en-temps-reel
```

### Characteristics

- Format: JSON (UTF-8), GBFS 1.0
- Update frequency: every minute
- Authentication: none
- Coverage: ~1,400 stations across 55 communes

### Fields

#### Core

| Field | Type | Description | Example |
|-------|------|-------------|---------|
| `name` | string | Station name | `"Rouget de L'isle - Watteau"` |
| `stationcode` | string | Unique station ID | `"32017"` |
| `coordonnees_geo` | float[2] | `[lat, lon]` | `[48.936, 2.358]` |
| `capacity` | integer | Total dock capacity | `22` |
| `numbikesavailable` | integer | Total bikes available | `15` |
| `numdocksavailable` | integer | Free docks | `7` |

#### Bike type breakdown

| Field | Type | Description |
|-------|------|-------------|
| `ebike` | integer | Electric bikes available |
| `mechanical` | integer | Mechanical bikes available |

#### Station state flags

| Field | Type | Values | Description |
|-------|------|--------|-------------|
| `is_renting` | string | `"OUI"`/`"NON"` | Rentals allowed |
| `is_installed` | string | `"OUI"`/`"NON"` | Station is operational |
| `is_returning` | string | `"OUI"`/`"NON"` | Returns allowed |

#### Timestamps

| Field | Type | Format | Example |
|-------|------|--------|---------|
| `duedate` | string | ISO 8601 | `"2025-06-14T19:31:22+00:00"` |
| `record_timestamp` | string | ISO 8601 | `"2025-06-14T19:31:22+00:00"` |

#### Administrative

| Field | Type | Description |
|-------|------|-------------|
| `nom_arrondissement_communes` | string | Commune / arrondissement |
| `code_insee_commune` | string | INSEE administrative code |

### Invariants

- `numbikesavailable` = `ebike` + `mechanical`
- `numdocksavailable` = `capacity` − `numbikesavailable`
- `0 ≤ numbikesavailable ≤ capacity`
- Coordinate precision: 7–8 decimal places
- Observed capacity range: 12–60 docks

## Dataset 2: Station Locations

### Endpoint

```
https://opendata.paris.fr/api/records/1.0/search/?dataset=velib-emplacement-des-stations
```

### Characteristics

- Format: JSON (UTF-8)
- Nature: static reference data
- Update frequency: occasional (station additions/removals)
- Authentication: none

### Fields

| Field | Type | Description | Example |
|-------|------|-------------|---------|
| `stationcode` | string | Unique ID (same key as real-time) | `"32017"` |
| `name` | string | Location name | `"Basilique"` |
| `capacity` | integer | Maximum capacity | `22` |
| `coordonnees_geo` | float[2] | `[lat, lon]` | `[48.936, 2.358]` |

## Dataset Relationship

- **Join key**: `stationcode` (1:1)
- Every real-time record must have a corresponding location entry.

| Aspect | Real-time | Locations |
|--------|-----------|-----------|
| Update frequency | Every minute | Occasional |
| Data type | Dynamic state | Static metadata |
| Primary use | Live planning | Geographic reference |

## Update Strategy

- **Reference data**: sync daily
- **Real-time data**: poll every 2–3 minutes (respects rate limits)
- **Cache TTL**: 2 minutes for real-time data
