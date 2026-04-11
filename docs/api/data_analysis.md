# Velib Data Analysis

## Overview

Two datasets from the Paris Open Data API:

1. **Real-time availability** — current bike/dock counts per station
2. **Station locations** — static reference data for all stations

## Dataset 1: Real-time Availability

**Endpoint**
```
https://opendata.paris.fr/api/explore/v2.1/catalog/datasets/velib-disponibilite-en-temps-reel/records
```

**Characteristics**
- Format: JSON (UTF-8), GBFS 1.0
- Update frequency: every minute
- No authentication required
- Coverage: ~1,400 stations across 55 communes

**Fields**

| Field | Type | Description | Example |
|-------|------|-------------|---------|
| `stationcode` | string | Unique station ID | `"32017"` |
| `name` | string | Station name | `"Rouget de L'isle - Watteau"` |
| `coordonnees_geo` | object | `{lat, lon}` | `{lat: 48.936, lon: 2.358}` |
| `capacity` | integer | Total dock count | `22` |
| `numbikesavailable` | integer | Total available bikes | `15` |
| `numdocksavailable` | integer | Available docks | `7` |
| `mechanical` | integer | Available mechanical bikes | `8` |
| `ebike` | integer | Available electric bikes | `4` |
| `is_installed` | string | `"OUI"` / `"NON"` | `"OUI"` |
| `is_renting` | string | `"OUI"` / `"NON"` | `"OUI"` |
| `is_returning` | string | `"OUI"` / `"NON"` | `"OUI"` |
| `duedate` | string | ISO 8601 last-update time | `"2025-06-14T19:31:22+00:00"` |
| `nom_arrondissement_communes` | string | Municipality/arrondissement | `"Issy-les-Moulineaux"` |
| `code_insee_commune` | string | INSEE administrative code | `"92040"` |

**Constraints**
- `numbikesavailable` = `mechanical` + `ebike`
- `numdocksavailable` = `capacity` - `numbikesavailable`
- Coordinate precision: 7–8 decimal places (~1 m)
- Observed capacity range: 12–60 docks

## Dataset 2: Station Locations

**Endpoint**
```
https://opendata.paris.fr/api/explore/v2.1/catalog/datasets/velib-emplacement-des-stations/records
```

**Characteristics**
- Static reference data; updated occasionally (station additions/removals)
- No authentication required

**Fields**

| Field | Type | Description | Example |
|-------|------|-------------|---------|
| `stationcode` | string | Unique station ID (matches real-time) | `"32017"` |
| `name` | string | Station name | `"Basilique"` |
| `capacity` | integer | Maximum capacity | `22` |
| `coordonnees_geo` | object | `{lat, lon}` | `{lat: 48.936, lon: 2.358}` |

## Dataset Relationship

- **Join key**: `stationcode` (1:1)
- Real-time data joined to reference data in [`src/data/client.rs`](../../src/data/client.rs)
- Cache TTLs: 2 min (real-time), 5 min (reference)
