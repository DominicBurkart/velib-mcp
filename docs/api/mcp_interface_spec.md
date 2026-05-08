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

Toutes les resources renvoient `application/json`. Voir
[`src/mcp/server.rs`](../../src/mcp/server.rs) pour le routage.

### 1. Stations de Référence

URI : `velib://stations/reference` — catalogue complet des stations Velib
avec leurs métadonnées statiques.

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

URI : `velib://stations/realtime` — état actuel de toutes les stations
avec disponibilité des vélos et emplacements.

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

URI : `velib://stations/complete` — vue complète combinant données de
référence et temps réel.

## Tools MCP

Les handlers sont implémentés dans
[`src/mcp/handlers.rs`](../../src/mcp/handlers.rs).

### 1. Recherche de Stations Proches

Tool : [`find_nearby_stations`](../../src/mcp/handlers.rs#L153) — trouve
les stations Velib dans un rayon donné autour d'un point.

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

Tool : [`get_station_by_code`](../../src/mcp/handlers.rs#L212) —
récupère les informations complètes d'une station spécifique.

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

Tool : [`search_stations_by_name`](../../src/mcp/handlers.rs#L227) —
recherche textuelle dans les noms de stations.

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
      "maximum": 50,
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

Tool : [`get_area_statistics`](../../src/mcp/handlers.rs#L286) —
calcule des statistiques agrégées pour une zone géographique.

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

Tool : [`plan_bike_journey`](../../src/mcp/handlers.rs#L303) — planifie
un trajet en suggérant stations de départ et d'arrivée.

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
              "walk_to_pickup": {"type": "integer"},
              "walk_from_dropoff": {"type": "integer"},
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

Les codes JSON-RPC suivent le mapping défini dans
[`Error::mcp_error_code`](../../src/error.rs#L49). Exemple :

```json
{
  "error": {
    "code": -32600,
    "message": "Station not found: 99999",
    "data": {
      "station_code": "99999",
      "error_type": "station_not_found"
    }
  }
}
```

### Mapping des codes
- `-32700` : erreur de parsing JSON
- `-32600` : requête invalide (`StationNotFound`)
- `-32602` : paramètres invalides (`InvalidCoordinates`,
  `OutsideServiceArea`, `SearchRadiusTooLarge`, `ResultLimitExceeded`,
  `Validation`)
- `-32603` : erreur interne (`McpProtocol`, `Cache`, `Internal`)
- `-32001` : erreur serveur (`Http`, `RateLimited`)

## Authentification et Rate Limiting

Non implémentés à ce jour : aucune authentification n'est requise et
aucun en-tête `X-RateLimit-*` n'est émis. Le client API amont
(`opendata.paris.fr`) peut renvoyer un HTTP 429 ; ce cas remonte sous
forme de `Error::RateLimited`.

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
    "hit_rate": 0.85,
    "entries": 1400
  }
}
```