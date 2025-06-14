# Schémas de Données MCP - Velib Server

## Vue d'ensemble

Ce document définit les schémas de données formels pour l'exposition des données Velib via le protocole MCP (Model Context Protocol).

## Types de Base

### Coordonnées Géographiques
```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Coordinates {
    /// Latitude en degrés décimaux
    pub latitude: f64,
    /// Longitude en degrés décimaux  
    pub longitude: f64,
}
```

### État de Station
```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum StationStatus {
    /// Station opérationnelle
    Operational,
    /// Station installée mais non opérationnelle
    Installed,
    /// Station en maintenance
    Maintenance,
    /// Station hors service
    OutOfService,
}
```

### Capacités de Service
```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ServiceCapabilities {
    /// Location de vélos autorisée
    pub renting_enabled: bool,
    /// Retour de vélos autorisé
    pub returning_enabled: bool,
    /// Station installée et accessible
    pub installed: bool,
}
```

## Schémas Principaux

### Station de Référence
```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StationReference {
    /// Identifiant unique de la station
    pub station_code: String,
    
    /// Nom descriptif de la station
    pub name: String,
    
    /// Coordonnées géographiques précises
    pub coordinates: Coordinates,
    
    /// Capacité totale de la station
    pub capacity: u16,
    
    /// Commune ou arrondissement
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commune: Option<String>,
    
    /// Code INSEE de la commune
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insee_code: Option<String>,
}
```

### Disponibilité Temps Réel
```rust
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BikeAvailability {
    /// Vélos mécaniques disponibles
    pub mechanical: u16,
    
    /// Vélos électriques disponibles
    pub electric: u16,
}

impl BikeAvailability {
    /// Total de vélos disponibles
    pub fn total(&self) -> u16 {
        self.mechanical + self.electric
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RealTimeStatus {
    /// Référence à la station
    pub station_code: String,
    
    /// Vélos disponibles par type
    pub bikes: BikeAvailability,
    
    /// Emplacements libres pour retour
    pub available_docks: u16,
    
    /// Capacités de service actuelles
    pub service: ServiceCapabilities,
    
    /// État général de la station
    pub status: StationStatus,
    
    /// Horodatage de dernière mise à jour
    pub last_updated: DateTime<Utc>,
    
    /// Horodatage de validité des données
    pub valid_until: DateTime<Utc>,
}
```

### Station Complète (Vue Consolidée)
```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VelibStation {
    /// Données de référence de la station
    #[serde(flatten)]
    pub reference: StationReference,
    
    /// État temps réel actuel
    #[serde(skip_serializing_if = "Option::is_none")]
    pub real_time: Option<RealTimeStatus>,
    
    /// Indicateur de fraîcheur des données
    pub data_freshness: DataFreshness,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DataFreshness {
    /// Données très récentes (< 2 minutes)
    Fresh,
    /// Données acceptables (< 5 minutes)
    Acceptable,
    /// Données anciennes (> 5 minutes)
    Stale,
    /// Données indisponibles
    Unavailable,
}
```

## Schémas de Requête MCP

### Paramètres de Recherche Géographique
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GeographicQuery {
    /// Point central de recherche
    pub center: Coordinates,
    
    /// Rayon de recherche en mètres
    pub radius_meters: u32,
    
    /// Nombre maximum de résultats
    #[serde(default = "default_limit")]
    pub limit: u16,
}

fn default_limit() -> u16 { 50 }
```

### Filtres de Disponibilité
```rust
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AvailabilityFilter {
    /// Minimum de vélos disponibles requis
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_bikes: Option<u16>,
    
    /// Minimum d'emplacements libres requis
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_docks: Option<u16>,
    
    /// Type de vélos requis
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bike_type: Option<BikeTypeFilter>,
    
    /// Exclure les stations hors service
    #[serde(default = "default_true")]
    pub exclude_out_of_service: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BikeTypeFilter {
    /// Au moins un vélo mécanique
    MechanicalRequired,
    /// Au moins un vélo électrique
    ElectricRequired,
    /// Vélos mécaniques ET électriques
    BothRequired,
    /// Vélos mécaniques OU électriques
    AnyType,
}

fn default_true() -> bool { true }
```

### Requête Complète
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StationQuery {
    /// Contraintes géographiques
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geographic: Option<GeographicQuery>,
    
    /// Filtres de disponibilité
    #[serde(skip_serializing_if = "Option::is_none")]
    pub availability: Option<AvailabilityFilter>,
    
    /// Codes de stations spécifiques
    #[serde(skip_serializing_if = "Option::is_none")]
    pub station_codes: Option<Vec<String>>,
    
    /// Inclure les données temps réel
    #[serde(default = "default_true")]
    pub include_real_time: bool,
}
```

## Schémas de Réponse MCP

### Réponse de Liste de Stations
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StationListResponse {
    /// Stations correspondant aux critères
    pub stations: Vec<VelibStation>,
    
    /// Nombre total de stations trouvées
    pub total_count: usize,
    
    /// Pagination si applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<PaginationInfo>,
    
    /// Métadonnées de la requête
    pub metadata: ResponseMetadata,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaginationInfo {
    pub offset: usize,
    pub limit: usize,
    pub has_more: bool,
}
```

### Métadonnées de Réponse
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResponseMetadata {
    /// Horodatage de la réponse
    pub response_time: DateTime<Utc>,
    
    /// Durée de traitement en millisecondes
    pub processing_time_ms: u64,
    
    /// Source des données temps réel
    pub real_time_source: DataSource,
    
    /// Source des données de référence
    pub reference_source: DataSource,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DataSource {
    /// Données fraîches de l'API
    Live,
    /// Données du cache local
    Cached { age_seconds: u64 },
    /// Données de sauvegarde
    Fallback,
    /// Données indisponibles
    Unavailable,
}
```

## Validation et Contraintes

### Contraintes de Cohérence
```rust
impl VelibStation {
    /// Valide la cohérence des données de la station
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Vérifier la cohérence des compteurs
        if let Some(rt) = &self.real_time {
            let total_bikes = rt.bikes.total();
            let expected_docks = self.reference.capacity
                .checked_sub(total_bikes)
                .ok_or(ValidationError::CapacityOverflow)?;
                
            if rt.available_docks != expected_docks {
                return Err(ValidationError::DockCountMismatch);
            }
        }
        
        // Validation des coordonnées (Paris métropole)
        let coords = &self.reference.coordinates;
        if coords.latitude < 48.7 || coords.latitude > 49.0 ||
           coords.longitude < 2.0 || coords.longitude > 2.6 {
            return Err(ValidationError::InvalidCoordinates);
        }
        
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("La capacité de la station est dépassée")]
    CapacityOverflow,
    
    #[error("Incohérence entre vélos et emplacements disponibles")]
    DockCountMismatch,
    
    #[error("Coordonnées hors de la zone de service")]
    InvalidCoordinates,
}
```

## Sérialisation JSON

### Exemple de Station Complète
```json
{
  "station_code": "32017",
  "name": "Rouget de L'isle - Watteau",
  "coordinates": {
    "latitude": 48.936268,
    "longitude": 2.358866
  },
  "capacity": 22,
  "commune": "Issy-les-Moulineaux",
  "insee_code": "92040",
  "real_time": {
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
    "last_updated": "2025-06-14T19:31:22Z",
    "valid_until": "2025-06-14T19:33:22Z"
  },
  "data_freshness": "Fresh"
}
```