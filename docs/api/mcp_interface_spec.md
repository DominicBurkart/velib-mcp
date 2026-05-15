# Spécification des Interfaces MCP - Velib Server

## Vue d'ensemble

Ce document définit les interfaces MCP (Model Context Protocol) exposées par le serveur Velib pour l'accès aux données de vélos en libre-service parisien.

## Architecture MCP

### Serveur MCP
- **Nom** : `velib-mcp`
- **Version** : `1.0.0`
- **Description** : Serveur MCP pour les données Velib Paris
- **Capacités** : `resources`, `tools`

### Transport
- **Protocole** : JSON-RPC 2.0 over HTTP/WebSocket
- **Encoding** : UTF-8
- **Port par défaut** : 8080

## Resources MCP

### 1. Stations de Référence

#### Resource URI
```
velib://stations/reference
```

#### Description
Catalogue complet des stations Velib avec leurs métadonnées statiques.

#### Content Type
```
application/json
```

#### Exemple de Contenu
```json
{
  "stations": [
    {
      "station_code": "32017",
      "name": "Rouget de L'isle - Watteau",
      "coordinates": {
        "latitude": 48.936268,
        "longitude": 2.358866
      },
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

### 2. Disponibilité Temps Réel

#### Resource URI
```
velib://stations/realtime
```

#### Description
État actuel de toutes les stations avec disponibilité des vélos et emplacements.

#### Content Type
```
application/json
```

#### Exemple de Contenu
```json
{
  "stations": [
    {
      "station_code": "32017",
      "bikes": {
        "mechanical": 8,
        "electric": 4
      },
      "available_docks": 10,
      "service": {
        "renting_enabled": true,
        "returning_enabled": true,
        "installed": true
      },
      "status": "Operational",
      "last_updated": "2025-06-14T19:31:22Z"
    }
  ],
  "metadata": {
    "data_freshness": "Fresh",
    "response_time": "2025-06-14T19:31:25Z"
  }
}
```

### 3. Stations Consolidées

#### Resource URI
```
velib://stations/complete
```

#### Description
Vue complète combinant données de référence et temps réel.

#### Content Type
```
application/json
```

## Tools MCP

### 1. Recherche de Stations Proches

#### Tool Name
```
find_nearby_stations
```

#### Description
Trouve les stations Velib dans un rayon donné autour d'un point.

#### Input Schema
```json
{
  "type": "object",
  "properties": {
    "latitude": {
      "type": "number",
      "minimum": 48.7,
      "maximum": 49.0,
      "description": "Latitude du point central"
    },
    "longitude": {
      "type": "number", 
      "minimum": 2.0,
      "maximum": 2.6,
      "description": "Longitude du point central"
    },
    "radius_meters": {
      "type": "integer",
      "minimum": 100,
      "maximum": 5000,
      "default": 500,
      "description": "Rayon de recherche en mètres"
    },
    "limit": {
      "type": "integer",
      "minimum": 1,
      "maximum": 100,
      "default": 10,
      "description": "Nombre maximum de stations"
    },
    "availability_filter": {
      "type": "object",
      "properties": {
        "min_bikes": {
          "type": "integer",
          "minimum": 0,
          "description": "Minimum de vélos disponibles"
        },
        "min_docks": {
          "type": "integer", 
          "minimum": 0,
          "description": "Minimum d'emplacements libres"
        },
        "bike_type": {
          "type": "string",
          "enum": ["mechanical", "electric", "any"],
          "default": "any",
          "description": "Type de vélos requis"
        }
      }
    }
  },
  "required": ["latitude", "longitude"]
}
```

#### Output Schema
```json
{
  "type": "object",
  "properties": {
    "stations": {
      "type": "array",
      "items": {
        "$ref": "#/definitions/VelibStation"
      }
    },
    "search_metadata": {
      "type": "object",
      "properties": {
        "query_point": {
          "type": "object",
          "properties": {
            "latitude": {"type": "number"},
            "longitude": {"type": "number"}
          }
        },
        "radius_meters": {"type": "integer"},
        "total_found": {"type": "integer"},
        "search_time_ms": {"type": "integer"}
      }
    }
  }
}
```

#### Exemple d'Utilisation
```json
{
  "name": "find_nearby_stations",
  "arguments": {
    "latitude": 48.8566,
    "longitude": 2.3522,
    "radius_meters": 1000,
    "limit": 5,
    "availability_filter": {
      "min_bikes": 2,
      "bike_type": "electric"
    }
  }
}
```

### 2. Obtenir Station par Code

#### Tool Name
```
get_station_by_code
```

#### Description
Récupère les informations complètes d'une station spécifique.

#### Input Schema
```json
{
  "type": "object",
  "properties": {
    "station_code": {
      "type": "string",
      "pattern": "^[0-9]+$",
      "description": "Code unique de la station"
    },
    "include_real_time": {
      "type": "boolean",
      "default": true,
      "description": "Inclure les données temps réel"
    }
  },
  "required": ["station_code"]
}
```

#### Output Schema
```json
{
  "type": "object",
  "properties": {
    "station": {
      "$ref": "#/definitions/VelibStation"
    },
    "found": {
      "type": "boolean",
      "description": "Station trouvée ou non"
    }
  }
}
```

### 3. Recherche par Nom de Station

#### Tool Name
```
search_stations_by_name
```

#### Description
Recherche textuelle dans les noms de stations.

#### Input Schema
```json
{
  "type": "object",
  "properties": {
    "query": {
      "type": "string",
      "minLength": 2,
      "description": "Terme de recherche dans le nom"
    },
    "limit": {
      "type": "integer",
      "minimum": 1,
      "maximum": 100,
      "default": 10,
      "description": "Nombre maximum de résultats"
    },
    "fuzzy": {
      "type": "boolean",
      "default": true,
      "description": "Recherche approximative autorisée"
    }
  },
  "required": ["query"]
}
```

### 4. Statistiques de Zone

#### Tool Name
```
get_area_statistics
```

#### Description
Calcule des statistiques agrégées pour une zone géographique.

#### Input Schema
```json
{
  "type": "object",
  "properties": {
    "bounds": {
      "type": "object",
      "properties": {
        "north": {"type": "number"},
        "south": {"type": "number"},
        "east": {"type": "number"},
        "west": {"type": "number"}
      },
      "required": ["north", "south", "east", "west"]
    },
    "include_real_time": {
      "type": "boolean",
      "default": true
    }
  },
  "required": ["bounds"]
}
```

#### Output Schema
```json
{
  "type": "object",
  "properties": {
    "area_stats": {
      "type": "object",
      "properties": {
        "total_stations": {"type": "integer"},
        "operational_stations": {"type": "integer"},
        "total_capacity": {"type": "integer"},
        "available_bikes": {
          "type": "object",
          "properties": {
            "mechanical": {"type": "integer"},
            "electric": {"type": "integer"},
            "total": {"type": "integer"}
          }
        },
        "available_docks": {"type": "integer"},
        "occupancy_rate": {
          "type": "number",
          "minimum": 0,
          "maximum": 1,
          "description": "Taux d'occupation (0-1)"
        }
      }
    },
    "bounds": {"$ref": "#/definitions/GeographicBounds"}
  }
}
```

### 5. Itinéraire avec Vélos

#### Tool Name
```
plan_bike_journey
```

#### Description
Planifie un trajet en suggérant stations de départ et d'arrivée.

#### Input Schema
```json
{
  "type": "object",
  "properties": {
    "origin": {
      "type": "object",
      "properties": {
        "latitude": {"type": "number"},
        "longitude": {"type": "number"}
      },
      "required": ["latitude", "longitude"]
    },
    "destination": {
      "type": "object", 
      "properties": {
        "latitude": {"type": "number"},
        "longitude": {"type": "number"}
      },
      "required": ["latitude", "longitude"]
    },
    "preferences": {
      "type": "object",
      "properties": {
        "bike_type": {
          "type": "string",
          "enum": ["mechanical", "electric", "any"],
          "default": "any"
        },
        "max_walk_distance": {
          "type": "integer",
          "default": 500,
          "description": "Distance max à pied en mètres"
        }
      }
    }
  },
  "required": ["origin", "destination"]
}
```

#### Output Schema
```json
{
  "type": "object",
  "properties": {
    "journey": {
      "type": "object",
      "properties": {
        "pickup_stations": {
          "type": "array",
          "items": {"$ref": "#/definitions/StationWithDistance"}
        },
        "dropoff_stations": {
          "type": "array", 
          "items": {"$ref": "#/definitions/StationWithDistance"}
        },
        "recommendations": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "pickup_station": {"$ref": "#/definitions/VelibStation"},
              "dropoff_station": {"$ref": "#/definitions/VelibStation"},
              "straight_line_to_pickup_meters": {"type": "integer"},
              "straight_line_from_dropoff_meters": {"type": "integer"},
              "confidence_score": {
                "type": "number",
                "minimum": 0.1,
                "maximum": 1
              }
            }
          }
        }
      }
    }
  }
}
```

## Gestion des Erreurs

### Format JSON-RPC

Toute erreur est renvoyée dans le champ `error` d'une réponse JSON-RPC
2.0. Le champ `data.error_type` est une chaîne stable utilisable par les
clients pour discriminer les variantes sans parser le message.

```json
{
  "error": {
    "code": -32600,
    "message": "Station not found: 99999",
    "data": {
      "error_type": "station_not_found"
    }
  }
}
```

### Mapping codes / `error_type`

La source de vérité est [`src/error.rs`](../../src/error.rs)
(`Error::mcp_error_code` et `Error::error_type`).

| Code     | `error_type`              | Variante Rust                |
|----------|---------------------------|------------------------------|
| `-32700` | `json_error`              | `Error::Json`                |
| `-32600` | `station_not_found`       | `Error::StationNotFound`     |
| `-32602` | `invalid_coordinates`     | `Error::InvalidCoordinates`  |
| `-32602` | `outside_service_area`    | `Error::OutsideServiceArea`  |
| `-32602` | `search_radius_too_large` | `Error::SearchRadiusTooLarge`|
| `-32602` | `result_limit_exceeded`   | `Error::ResultLimitExceeded` |
| `-32602` | `validation_error`        | `Error::Validation`          |
| `-32603` | `mcp_protocol_error`      | `Error::McpProtocol`         |
| `-32603` | `cache_error`             | `Error::Cache`               |
| `-32603` | `internal_error`          | `Error::Internal`            |
| `-32001` | `http_error`              | `Error::Http`                |
| `-32001` | `rate_limited`            | `Error::RateLimited`         |

## Rate Limiting

Pas de rate limiting applicatif côté serveur à ce jour. Les retries vers
l'API Paris Open Data utilisent un backoff exponentiel
(`Error::RateLimited` + `Retry-After`); voir
[`src/data/retry.rs`](../../src/data/retry.rs).

## Authentification

Aucune authentification requise; le serveur expose les données Velib
publiques telles quelles.

## Métadonnées de Santé

### Resource URI
```
velib://health
```

#### Contenu
La réponse est produite par
[`get_health_resource`](../../src/mcp/server.rs). `cache_stats` reflète
la taille réelle des caches; `hit_rate` est volontairement omis (le cache
ne suit pas hits/misses, voir
[`src/data/cache.rs`](../../src/data/cache.rs)). `lag_seconds` est calculé
à partir du `last_update` le plus récent observé dans les données live.

```json
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime_seconds": 86400,
  "data_sources": {
    "real_time": {
      "status": "healthy",
      "last_update": "2026-04-23T19:31:22Z",
      "lag_seconds": 45
    },
    "reference": {
      "status": "healthy",
      "last_update": "2026-04-23T19:31:22Z"
    }
  },
  "cache_stats": {
    "entries": 2,
    "reference_cache_size": 1,
    "realtime_cache_size": 1
  }
}
```
