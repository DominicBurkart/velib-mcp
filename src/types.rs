use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}

impl Coordinates {
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
        }
    }

    /// Calculate distance to another coordinate in meters using Haversine formula
    pub fn distance_to(&self, other: &Coordinates) -> f64 {
        let earth_radius = 6371000.0; // Earth radius in meters

        let lat1_rad = self.latitude.to_radians();
        let lat2_rad = other.latitude.to_radians();
        let delta_lat = (other.latitude - self.latitude).to_radians();
        let delta_lon = (other.longitude - self.longitude).to_radians();

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);

        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

        earth_radius * c
    }

    /// Check if coordinates are within reasonable bounds for Paris metro area
    pub fn is_valid_paris_metro(&self) -> bool {
        // Paris metro area bounds (approximate)
        self.latitude >= 48.7
            && self.latitude <= 49.0
            && self.longitude >= 2.0
            && self.longitude <= 2.6
    }

    /// Check if coordinates are within 50km of Paris City Hall (Hôtel de Ville)
    /// Latitude: 48.8565° N, Longitude: 2.3514° E
    pub fn is_within_paris_service_area(&self) -> bool {
        const PARIS_CITY_HALL_LAT: f64 = 48.8565;
        const PARIS_CITY_HALL_LON: f64 = 2.3514;
        const MAX_DISTANCE_METERS: f64 = 50_000.0; // 50km

        let city_hall = Coordinates::new(PARIS_CITY_HALL_LAT, PARIS_CITY_HALL_LON);
        let distance = self.distance_to(&city_hall);

        distance <= MAX_DISTANCE_METERS
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StationStatus {
    #[serde(rename = "OPEN")]
    Open,
    #[serde(rename = "CLOSED")]
    Closed,
    #[serde(rename = "MAINTENANCE")]
    Maintenance,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceCapabilities {
    pub accepts_credit_card: bool,
    pub has_charging_station: bool,
    pub is_virtual_station: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BikeAvailability {
    pub mechanical: u16,
    pub electric: u16,
}

impl BikeAvailability {
    pub fn new(mechanical: u16, electric: u16) -> Self {
        Self {
            mechanical,
            electric,
        }
    }

    pub fn total(&self) -> u16 {
        self.mechanical.saturating_add(self.electric)
    }

    pub fn has_bikes(&self) -> bool {
        self.total() > 0
    }

    pub fn has_mechanical(&self) -> bool {
        self.mechanical > 0
    }

    pub fn has_electric(&self) -> bool {
        self.electric > 0
    }
}

impl Default for BikeAvailability {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DataFreshness {
    Fresh,     // < 5 minutes old
    Recent,    // 5-15 minutes old
    Stale,     // 15-60 minutes old
    VeryStale, // > 60 minutes old
}

impl DataFreshness {
    pub fn from_age(age_minutes: f64) -> Self {
        match age_minutes {
            age if age < 5.0 => DataFreshness::Fresh,
            age if age < 15.0 => DataFreshness::Recent,
            age if age < 60.0 => DataFreshness::Stale,
            _ => DataFreshness::VeryStale,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum BikeTypeFilter {
    #[serde(rename = "mechanical")]
    MechanicalOnly,
    #[serde(rename = "electric")]
    ElectricOnly,
    #[serde(rename = "any")]
    #[default]
    AnyType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationReference {
    pub station_code: String,
    pub name: String,
    pub coordinates: Coordinates,
    pub capacity: u16,
    pub capabilities: ServiceCapabilities,
}

impl StationReference {
    pub fn validate(&self) -> Result<(), String> {
        if self.station_code.is_empty() {
            return Err("Station code cannot be empty".to_string());
        }

        if self.name.is_empty() {
            return Err("Station name cannot be empty".to_string());
        }

        if self.capacity == 0 {
            return Err("Station capacity must be greater than 0".to_string());
        }

        // Check for reasonable capacity limits (prevent overflow)
        if self.capacity > 200 {
            return Err("Station capacity seems unreasonably high".to_string());
        }

        if !self.coordinates.is_valid_paris_metro() {
            return Err("Coordinates are outside valid Paris metro area".to_string());
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealTimeStatus {
    pub bikes: BikeAvailability,
    pub available_docks: u16,
    pub status: StationStatus,
    pub last_update: DateTime<Utc>,
    pub data_freshness: DataFreshness,
}

impl RealTimeStatus {
    pub fn new(
        bikes: BikeAvailability,
        available_docks: u16,
        status: StationStatus,
        last_update: DateTime<Utc>,
    ) -> Self {
        let age_minutes = (Utc::now() - last_update).num_minutes() as f64;
        let data_freshness = DataFreshness::from_age(age_minutes);

        Self {
            bikes,
            available_docks,
            status,
            last_update,
            data_freshness,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VelibStation {
    pub reference: StationReference,
    pub real_time: Option<RealTimeStatus>,
}

impl VelibStation {
    pub fn new(reference: StationReference) -> Self {
        Self {
            reference,
            real_time: None,
        }
    }

    pub fn with_real_time(mut self, real_time: RealTimeStatus) -> Self {
        self.real_time = Some(real_time);
        self
    }

    pub fn is_operational(&self) -> bool {
        match &self.real_time {
            Some(rt) => matches!(rt.status, StationStatus::Open),
            None => true, // Assume operational if no real-time data
        }
    }

    pub fn has_available_bikes(&self, bike_type: &BikeTypeFilter) -> bool {
        match &self.real_time {
            Some(rt) => match bike_type {
                BikeTypeFilter::MechanicalOnly => rt.bikes.has_mechanical(),
                BikeTypeFilter::ElectricOnly => rt.bikes.has_electric(),
                BikeTypeFilter::AnyType => rt.bikes.has_bikes(),
            },
            None => false,
        }
    }

    pub fn has_available_docks(&self, min_docks: u16) -> bool {
        match &self.real_time {
            Some(rt) => rt.available_docks >= min_docks,
            None => false,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        self.reference.validate()?;

        if let Some(rt) = &self.real_time {
            let total_bikes = rt.bikes.total() as u32;
            let total_docks = rt.available_docks as u32;
            let capacity = self.reference.capacity as u32;

            if total_bikes + total_docks > capacity {
                return Err(format!(
                    "Total bikes ({}) + docks ({}) exceeds capacity ({})",
                    total_bikes, total_docks, capacity
                ));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataSource {
    #[serde(rename = "paris_open_data")]
    ParisOpenData,
    #[serde(rename = "cache")]
    Cache,
    #[serde(rename = "fallback")]
    Fallback,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinates_distance() {
        let coord1 = Coordinates::new(48.8566, 2.3522); // Paris center
        let coord2 = Coordinates::new(48.8606, 2.3376); // Louvre

        let distance = coord1.distance_to(&coord2);

        // Should be roughly 1.3km between these points
        assert!(distance > 1000.0 && distance < 1500.0);
    }

    #[test]
    fn test_coordinates_paris_validation() {
        let valid_paris = Coordinates::new(48.8566, 2.3522);
        let invalid_coords = Coordinates::new(40.7128, -74.0060); // NYC

        assert!(valid_paris.is_valid_paris_metro());
        assert!(!invalid_coords.is_valid_paris_metro());
    }

    #[test]
    fn test_paris_service_area_validation() {
        // Paris City Hall
        let city_hall = Coordinates::new(48.8565, 2.3514);
        assert!(city_hall.is_within_paris_service_area());

        // Paris center (should be within 50km)
        let paris_center = Coordinates::new(48.8566, 2.3522);
        assert!(paris_center.is_within_paris_service_area());

        // London (should be outside 50km)
        let london = Coordinates::new(51.5074, -0.1278);
        assert!(!london.is_within_paris_service_area());

        // Test edge case: exactly at the boundary
        // Calculate a point approximately 50km away
        let far_point = Coordinates::new(49.2, 2.3514); // ~38km north
        assert!(far_point.is_within_paris_service_area());

        // Point definitely outside 50km
        let very_far_point = Coordinates::new(50.0, 2.3514); // ~130km north
        assert!(!very_far_point.is_within_paris_service_area());
    }

    #[test]
    fn test_bike_availability() {
        let bikes = BikeAvailability::new(5, 3);

        assert_eq!(bikes.total(), 8);
        assert!(bikes.has_bikes());
        assert!(bikes.has_mechanical());
        assert!(bikes.has_electric());

        let no_bikes = BikeAvailability::new(0, 0);
        assert!(!no_bikes.has_bikes());
    }

    #[test]
    fn test_data_freshness() {
        assert_eq!(DataFreshness::from_age(2.0), DataFreshness::Fresh);
        assert_eq!(DataFreshness::from_age(10.0), DataFreshness::Recent);
        assert_eq!(DataFreshness::from_age(30.0), DataFreshness::Stale);
        assert_eq!(DataFreshness::from_age(90.0), DataFreshness::VeryStale);
    }

    #[test]
    fn test_bike_type_filter() {
        let bikes = BikeAvailability::new(2, 3);
        let station = VelibStation {
            reference: StationReference {
                station_code: "123".to_string(),
                name: "Test Station".to_string(),
                coordinates: Coordinates::new(48.8566, 2.3522),
                capacity: 20,
                capabilities: ServiceCapabilities::default(),
            },
            real_time: Some(RealTimeStatus {
                bikes,
                available_docks: 15,
                status: StationStatus::Open,
                last_update: Utc::now(),
                data_freshness: DataFreshness::Fresh,
            }),
        };

        assert!(station.has_available_bikes(&BikeTypeFilter::MechanicalOnly));
        assert!(station.has_available_bikes(&BikeTypeFilter::ElectricOnly));
        assert!(station.has_available_bikes(&BikeTypeFilter::AnyType));
    }

    #[test]
    fn test_station_validation() {
        let valid_station = VelibStation {
            reference: StationReference {
                station_code: "123".to_string(),
                name: "Test Station".to_string(),
                coordinates: Coordinates::new(48.8566, 2.3522),
                capacity: 20,
                capabilities: ServiceCapabilities::default(),
            },
            real_time: Some(RealTimeStatus {
                bikes: BikeAvailability::new(5, 3),
                available_docks: 12,
                status: StationStatus::Open,
                last_update: Utc::now(),
                data_freshness: DataFreshness::Fresh,
            }),
        };

        assert!(valid_station.validate().is_ok());
    }

    #[test]
    fn test_station_validation_errors() {
        // Test capacity overflow
        let invalid_station = VelibStation {
            reference: StationReference {
                station_code: "123".to_string(),
                name: "Test Station".to_string(),
                coordinates: Coordinates::new(48.8566, 2.3522),
                capacity: 10,
                capabilities: ServiceCapabilities::default(),
            },
            real_time: Some(RealTimeStatus {
                bikes: BikeAvailability::new(8, 5), // 13 bikes
                available_docks: 5,                 // total 18 > capacity 10
                status: StationStatus::Open,
                last_update: Utc::now(),
                data_freshness: DataFreshness::Fresh,
            }),
        };

        assert!(invalid_station.validate().is_err());
    }

    #[test]
    fn test_capacity_overflow_validation() {
        let reference = StationReference {
            station_code: "overflow_test".to_string(),
            name: "Overflow Test Station".to_string(),
            coordinates: Coordinates::new(48.8566, 2.3522),
            capacity: 300, // This should fail validation
            capabilities: ServiceCapabilities::default(),
        };

        assert!(reference.validate().is_err());
    }
}
