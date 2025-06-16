# Analyse des Données Velib - Phase 1

## Vue d'ensemble

Ce document analyse les deux jeux de données Velib disponibles via l'API Open Data de Paris :

1. **Disponibilité temps réel** : État actuel des stations (vélos/emplacements disponibles)
2. **Emplacements des stations** : Données de référence statiques des stations

## Dataset 1 : Disponibilité Temps Réel

### Endpoint API
```
https://opendata.paris.fr/api/records/1.0/search/?dataset=velib-disponibilite-en-temps-reel
```

### Caractéristiques Techniques
- **Format** : JSON (UTF-8)
- **Standard** : GBFS 1.0
- **Fréquence de mise à jour** : Chaque minute
- **Accès** : Sans clé d'authentification
- **Couverture** : ~1,400 stations sur 55 communes

### Structure des Données

#### Champs Principaux
| Champ | Type | Description | Exemple |
|-------|------|-------------|---------|
| `name` | string | Nom de la station | "Rouget de L'isle - Watteau" |
| `stationcode` | string | Identifiant unique de station | "32017" |
| `coordonnees_geo` | array[float] | Coordonnées [lat, lon] | [48.936, 2.358] |
| `capacity` | integer | Capacité totale de la station | 22 |
| `numbikesavailable` | integer | Vélos disponibles (total) | 15 |
| `numdocksavailable` | integer | Emplacements libres | 7 |

#### Détail par Type de Vélo
| Champ | Type | Description |
|-------|------|-------------|
| `ebike` | integer | Vélos électriques disponibles |
| `mechanical` | integer | Vélos mécaniques disponibles |

#### États de la Station
| Champ | Type | Valeurs | Description |
|-------|------|---------|-------------|
| `is_renting` | string | "OUI"/"NON" | Location autorisée |
| `is_installed` | string | "OUI"/"NON" | Station opérationnelle |
| `is_returning` | string | "OUI"/"NON" | Retour autorisé |

#### Données Temporelles
| Champ | Type | Format | Exemple |
|-------|------|--------|---------|
| `duedate` | string | ISO 8601 | "2025-06-14T19:31:22+00:00" |
| `record_timestamp` | string | ISO 8601 | "2025-06-14T19:31:22+00:00" |

#### Informations Administratives
| Champ | Type | Description |
|-------|------|-------------|
| `nom_arrondissement_communes` | string | Commune/arrondissement |
| `code_insee_commune` | string | Code INSEE administratif |

### Contraintes et Validations
- **Coordonnées** : Précision décimale de 7-8 places (niveau mètre)
- **Capacité** : Observée entre 12-60 emplacements
- **Disponibilité** : 0 ≤ `numbikesavailable` ≤ `capacity`
- **Cohérence** : `numbikesavailable` = `ebike` + `mechanical`
- **Emplacements** : `numdocksavailable` = `capacity` - `numbikesavailable`

## Dataset 2 : Emplacements des Stations

### Endpoint API
```
https://opendata.paris.fr/api/records/1.0/search/?dataset=velib-emplacement-des-stations
```

### Caractéristiques Techniques
- **Format** : JSON (UTF-8)
- **Nature** : Données de référence statiques
- **Mise à jour** : Occasionnelle (ajout/suppression de stations)
- **Accès** : Sans clé d'authentification

### Structure des Données

#### Champs de Référence
| Champ | Type | Description | Exemple |
|-------|------|-------------|---------|
| `stationcode` | string | Identifiant unique (même que temps réel) | "32017" |
| `name` | string | Nom de localisation | "Basilique" |
| `capacity` | integer | Capacité maximale | 22 |
| `coordonnees_geo` | array[float] | Position [lat, lon] | [48.936, 2.358] |

### Relation entre les Datasets

#### Clé de Liaison
- **Champ commun** : `stationcode`
- **Cardinalité** : 1:1 (une station = un code unique)
- **Intégrité** : Toutes les stations temps réel doivent avoir une référence

#### Différences Fonctionnelles
| Aspect | Temps Réel | Emplacements |
|--------|------------|-------------|
| **Fréquence** | Minute | Occasionnelle |
| **Données** | État dynamique | Métadonnées statiques |
| **Utilisation** | Planification immédiate | Référence géographique |

## Opportunités d'Intégration MCP

### Cas d'Usage Identifiés

1. **Planification de trajets**
   - Recherche de stations proches avec vélos disponibles
   - Calcul d'itinéraires avec disponibilité temps réel

2. **Analyse spatiale**
   - Densité de stations par zone
   - Rayons de couverture géographique

3. **Monitoring d'état**
   - Stations hors service ou pleines
   - Tendances de disponibilité

4. **Optimisation logistique**
   - Répartition des vélos par type
   - Prédiction de demande par zone

### Défis Techniques

1. **Synchronisation des données**
   - Cohérence entre datasets
   - Gestion des stations temporairement indisponibles

2. **Performance**
   - Cache intelligent pour données quasi-statiques
   - Optimisation des requêtes géospatiales

3. **Fiabilité**
   - Gestion des pannes d'API
   - Validation de la cohérence des données

## Recommandations pour l'Architecture MCP

### Modèle de Données Unifié
```rust
struct VelibStation {
    // Référence statique
    station_code: String,
    name: String,
    capacity: u16,
    coordinates: (f64, f64), // (lat, lon)
    
    // État temps réel
    available_bikes: u16,
    available_ebikes: u16,
    available_mechanical: u16,
    available_docks: u16,
    
    // États opérationnels
    is_renting: bool,
    is_installed: bool,
    is_returning: bool,
    
    // Métadonnées
    commune: Option<String>,
    insee_code: Option<String>,
    last_updated: DateTime<Utc>,
}
```

### Stratégie de Mise à Jour
1. **Référence** : Synchronisation quotidienne des emplacements
2. **Temps réel** : Polling toutes les 2-3 minutes (respect du rate limiting)
3. **Cache** : TTL de 2 minutes pour les données temps réel