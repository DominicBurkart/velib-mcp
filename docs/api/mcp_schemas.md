# Schémas de Données MCP - Velib Server

## Source de vérité

Les types Rust sont la source de vérité; ce document décrit la forme
canonique sérialisée. Les fichiers à consulter en cas de divergence:

- Types de domaine: [`src/types.rs`](../../src/types.rs)
  (`Coordinates`, `StationStatus`, `ServiceCapabilities`,
  `BikeAvailability`, `DataFreshness`, `BikeTypeFilter`,
  `StationReference`, `RealTimeStatus`, `VelibStation`).
- Types d'entrée/sortie MCP:
  [`src/mcp/types.rs`](../../src/mcp/types.rs)
  (`AvailabilityFilter`, `GeographicBounds`, `StationWithDistance`,
  `JourneyRecommendation`, `BikeJourney`, `AreaStatistics`, et les
  variantes `*Input` / `*Output` de chaque tool).
- Erreurs: [`src/error.rs`](../../src/error.rs) (`Error`,
  `mcp_error_code`, `error_type`).

## Conventions

- Sérialisation: JSON UTF-8 via `serde_json`.
- Enums avec `#[serde(rename = "...")]` explicitement listés
  ci-dessous; les autres utilisent la variante PascalCase.
- Tous les champs `Option<T>` portent
  `#[serde(skip_serializing_if = "Option::is_none")]` — le JSON ne
  contient pas le champ quand il est absent.

### Renommages serde notables

| Type            | Variante Rust       | JSON          |
|-----------------|---------------------|---------------|
| `StationStatus` | `Open`              | `"OPEN"`      |
| `StationStatus` | `Closed`            | `"CLOSED"`    |
| `StationStatus` | `Maintenance`       | `"MAINTENANCE"` |
| `BikeTypeFilter`| `MechanicalOnly`    | `"mechanical"`|
| `BikeTypeFilter`| `ElectricOnly`      | `"electric"`  |
| `BikeTypeFilter`| `AnyType`           | `"any"`       |

`DataFreshness` (Fresh/Recent/Stale/VeryStale) sérialise avec les noms
PascalCase par défaut.

## Exemple de Station Complète

`VelibStation` n'utilise pas `#[serde(flatten)]`: les données de
référence vivent sous `reference`, les données temps réel sous
`real_time` quand elles sont disponibles.

```json
{
  "reference": {
    "station_code": "32017",
    "name": "Rouget de L'isle - Watteau",
    "coordinates": {
      "latitude": 48.936268,
      "longitude": 2.358866
    },
    "capacity": 22,
    "capabilities": {
      "accepts_credit_card": false,
      "has_charging_station": false,
      "is_virtual_station": false
    }
  },
  "real_time": {
    "bikes": {
      "mechanical": 8,
      "electric": 4
    },
    "available_docks": 10,
    "status": "OPEN",
    "last_update": "2026-04-23T19:31:22Z",
    "data_freshness": "Fresh"
  }
}
```

Les champs `nom_arrondissement_communes` / `code_insee_commune` du
dataset Paris Open Data ne sont pas exposés par l'API MCP actuelle.

## Validation

`StationReference::validate` et `VelibStation::validate` renvoient une
`Result<(), String>` et appliquent:

- `station_code` non vide;
- `name` non vide;
- `capacity` strictement positive et `<= 200`;
- coordonnées dans le bounding box Paris métropole
  (`is_valid_paris_metro`, ~250 km autour de Paris);
- bikes + docks `<= capacity` (overflow physique de la station).

La validation des entrées des tools (rayon max, limite max, zone de
service 50 km autour de Paris) vit dans
[`src/mcp/handlers.rs`](../../src/mcp/handlers.rs) — voir
`ensure_in_service_area`, `MAX_SEARCH_RADIUS`, `MAX_RESULT_LIMIT`.
