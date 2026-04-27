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
                "minimum": 0,
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
Les erreurs sont retournées dans l'enveloppe JSON-RPC 2.0 standard avec un
`error_type` machine-readable dans `data` :

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

### Codes JSON-RPC Utilisés
La source autoritative est [`src/error.rs::Error::mcp_error_code`](../../src/error.rs).
Les variantes mappent sur les codes JSON-RPC 2.0 standard :

| Code | Variantes `Error` | `error_type` |
|------|-------------------|--------------|
| `-32001` | `Http`, `RateLimited` | `http_error`, `rate_limited` |
| `-32600` | `StationNotFound` | `station_not_found` |
| `-32602` | `InvalidCoordinates`, `OutsideServiceArea`, `SearchRadiusTooLarge`, `ResultLimitExceeded`, `Validation` | `invalid_coordinates`, `outside_service_area`, `search_radius_too_large`, `result_limit_exceeded`, `validation_error` |
| `-32603` | `McpProtocol`, `Cache`, `Internal` | `mcp_protocol_error`, `cache_error`, `internal_error` |
| `-32700` | `Json` (parse error) | `json_error` |

## Rate Limiting

Le serveur ne fait pas de rate-limiting au niveau applicatif (et ne renvoie
aucun header `X-RateLimit-*`). Le seul rate-limiting visible côté client est
celui de l'API Paris Open Data en amont, propagé via les erreurs `RateLimited`
(code `-32001`) avec un champ `retry_after_seconds` calculé depuis l'en-tête
`Retry-After` de la réponse 429.

## Authentification

### Mode Public
- Aucune authentification requise
- Rate limiting appliqué par IP

### Mode API Key (Futur)
```http
Authorization: Bearer <api_key>
```

## Métadonnées de Santé

### Resource URI
```
velib://health
```

#### Contenu
```json
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime_seconds": 86400,
  "data_sources": {
    "real_time": {
      "status": "healthy",
      "last_update": "2025-06-14T19:31:22Z",
      "lag_seconds": 45
    },
    "reference": {
      "status": "healthy", 
      "last_update": "2025-06-14T06:00:00Z"
    }
  },
  "cache_stats": {
    "entries": 2,
    "reference_cache_size": 1,
    "realtime_cache_size": 1
  }
}
```

Note : `cache_stats` ne contient pas de `hit_rate` — le cache ne suit pas
les hits/misses, et fabriquer une métrique synthétique serait trompeur.
Voir [`get_health_resource`](../../src/mcp/server.rs).