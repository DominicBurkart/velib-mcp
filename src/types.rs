use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

    pub fn is_valid_paris_metro(&self) -> bool {
        self.latitude >= 48.7
            && self.latitude <= 49.0
            && self.longitude >= 2.0
            && self.longitude <= 2.6
    }

    pub fn distance_to(&self, other: &Coordinates) -> f64 {
        const EARTH_RADIUS_KM: f64 = 6371.0;

        let lat1_rad = self.latitude.to_radians();
        let lat2_rad = other.latitude.to_radians();
        let delta_lat = (other.latitude - self.latitude).to_radians();
        let delta_lon = (other.longitude - self.longitude).to_radians();

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

        EARTH_RADIUS_KM * c * 1000.0 // Convert to meters
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum StationStatus {
    #[default]
    Operational,
    Installed,
    Maintenance,
    OutOfService,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceCapabilities {
    pub renting_enabled: bool,
    pub returning_enabled: bool,
    pub installed: bool,
}

impl Default for ServiceCapabilities {
    fn default() -> Self {
        Self {
            renting_enabled: true,
            returning_enabled: true,
            installed: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
        self.mechanical + self.electric
    }

    pub fn is_empty(&self) -> bool {
        self.total() == 0
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataFreshness {
    Fresh,
    Acceptable,
    Stale,
    Unavailable,
}

impl DataFreshness {
    pub fn from_age_seconds(age_seconds: u64) -> Self {
        match age_seconds {
            0..=120 => DataFreshness::Fresh,        // < 2 minutes
            121..=300 => DataFreshness::Acceptable, // < 5 minutes
            301..=1800 => DataFreshness::Stale,     // < 30 minutes
            _ => DataFreshness::Unavailable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataSource {
    Live,
    Cached { age_seconds: u64 },
    Fallback,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum BikeTypeFilter {
    MechanicalRequired,
    ElectricRequired,
    BothRequired,
    #[default]
    AnyType,
}

impl BikeTypeFilter {
    pub fn matches(&self, bikes: &BikeAvailability) -> bool {
        match self {
            BikeTypeFilter::MechanicalRequired => bikes.has_mechanical(),
            BikeTypeFilter::ElectricRequired => bikes.has_electric(),
            BikeTypeFilter::BothRequired => bikes.has_mechanical() && bikes.has_electric(),
            BikeTypeFilter::AnyType => !bikes.is_empty(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationReference {
    pub station_code: String,
    pub name: String,
    pub coordinates: Coordinates,
    pub capacity: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commune: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insee_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealTimeStatus {
    pub station_code: String,
    pub bikes: BikeAvailability,
    pub available_docks: u16,
    pub service: ServiceCapabilities,
    pub status: StationStatus,
    pub last_updated: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VelibStation {
    #[serde(flatten)]
    pub reference: StationReference,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub real_time: Option<RealTimeStatus>,
    pub data_freshness: DataFreshness,
}

impl VelibStation {
    pub fn new(reference: StationReference) -> Self {
        Self {
            reference,
            real_time: None,
            data_freshness: DataFreshness::Unavailable,
        }
    }

    pub fn with_real_time(mut self, real_time: RealTimeStatus) -> Self {
        let age = Utc::now().signed_duration_since(real_time.last_updated);
        self.data_freshness = DataFreshness::from_age_seconds(age.num_seconds().max(0) as u64);
        self.real_time = Some(real_time);
        self
    }

    pub fn validate(&self) -> crate::Result<()> {
        use crate::Error;

        if !self.reference.coordinates.is_valid_paris_metro() {
            return Err(Error::InvalidCoordinates {
                latitude: self.reference.coordinates.latitude,
                longitude: self.reference.coordinates.longitude,
            });
        }

        if let Some(rt) = &self.real_time {
            let total_bikes = rt.bikes.total();
            let total_used = total_bikes + rt.available_docks;

            if total_used > self.reference.capacity {
                return Err(Error::CapacityOverflow {
                    station_code: self.reference.station_code.clone(),
                });
            }

            let expected_docks = self.reference.capacity - total_bikes;
            if rt.available_docks != expected_docks {
                return Err(Error::DockCountMismatch {
                    station_code: self.reference.station_code.clone(),
                });
            }
        }

        Ok(())
    }

    pub fn is_operational(&self) -> bool {
        self.real_time
            .as_ref()
            .map(|rt| matches!(rt.status, StationStatus::Operational))
            .unwrap_or(false)
    }

    pub fn has_available_bikes(&self, filter: &BikeTypeFilter) -> bool {
        self.real_time
            .as_ref()
            .map(|rt| filter.matches(&rt.bikes))
            .unwrap_or(false)
    }

    pub fn has_available_docks(&self, min_docks: u16) -> bool {
        self.real_time
            .as_ref()
            .map(|rt| rt.available_docks >= min_docks)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_coordinates_distance() {
        let paris_center = Coordinates::new(48.8566, 2.3522); // Notre-Dame
        let tour_eiffel = Coordinates::new(48.8584, 2.2945); // Tour Eiffel

        let distance = paris_center.distance_to(&tour_eiffel);
        // Should be around 4.5km
        assert!(distance > 4000.0 && distance < 6000.0);
    }

    #[test]
    fn test_coordinates_paris_validation() {
        let valid_paris = Coordinates::new(48.8566, 2.3522);
        assert!(valid_paris.is_valid_paris_metro());

        let invalid_coords = Coordinates::new(40.7128, -74.0060); // New York
        assert!(!invalid_coords.is_valid_paris_metro());
    }

    #[test]
    fn test_bike_availability() {
        let bikes = BikeAvailability::new(5, 3);
        assert_eq!(bikes.total(), 8);
        assert!(bikes.has_mechanical());
        assert!(bikes.has_electric());
        assert!(!bikes.is_empty());

        let empty_bikes = BikeAvailability::new(0, 0);
        assert!(empty_bikes.is_empty());
        assert!(!empty_bikes.has_mechanical());
        assert!(!empty_bikes.has_electric());
    }

    #[test]
    fn test_bike_type_filter() {
        let bikes = BikeAvailability::new(2, 1);

        assert!(BikeTypeFilter::MechanicalRequired.matches(&bikes));
        assert!(BikeTypeFilter::ElectricRequired.matches(&bikes));
        assert!(BikeTypeFilter::BothRequired.matches(&bikes));
        assert!(BikeTypeFilter::AnyType.matches(&bikes));

        let only_mechanical = BikeAvailability::new(3, 0);
        assert!(BikeTypeFilter::MechanicalRequired.matches(&only_mechanical));
        assert!(!BikeTypeFilter::ElectricRequired.matches(&only_mechanical));
        assert!(!BikeTypeFilter::BothRequired.matches(&only_mechanical));
        assert!(BikeTypeFilter::AnyType.matches(&only_mechanical));
    }

    #[test]
    fn test_data_freshness() {
        assert_eq!(DataFreshness::from_age_seconds(60), DataFreshness::Fresh);
        assert_eq!(
            DataFreshness::from_age_seconds(200),
            DataFreshness::Acceptable
        );
        assert_eq!(DataFreshness::from_age_seconds(600), DataFreshness::Stale);
        assert_eq!(
            DataFreshness::from_age_seconds(2000),
            DataFreshness::Unavailable
        );
    }

    #[test]
    fn test_station_validation() {
        let reference = StationReference {
            station_code: "12345".to_string(),
            name: "Test Station".to_string(),
            coordinates: Coordinates::new(48.8566, 2.3522),
            capacity: 20,
            commune: None,
            insee_code: None,
        };

        let mut station = VelibStation::new(reference);
        assert!(station.validate().is_ok());

        // Add valid real-time data
        let real_time = RealTimeStatus {
            station_code: "12345".to_string(),
            bikes: BikeAvailability::new(8, 2),
            available_docks: 10, // 8+2+10=20 = capacity
            service: ServiceCapabilities::default(),
            status: StationStatus::Operational,
            last_updated: Utc::now(),
            valid_until: Utc::now() + Duration::minutes(5),
        };

        station = station.with_real_time(real_time);
        assert!(station.validate().is_ok());
        assert!(station.is_operational());
    }

    #[test]
    fn test_station_validation_errors() {
        // Test invalid coordinates
        let invalid_reference = StationReference {
            station_code: "12345".to_string(),
            name: "Test Station".to_string(),
            coordinates: Coordinates::new(40.0, -74.0), // New York
            capacity: 20,
            commune: None,
            insee_code: None,
        };

        let station = VelibStation::new(invalid_reference);
        assert!(station.validate().is_err());
    }

    #[test]
    fn test_capacity_overflow_validation() {
        let reference = StationReference {
            station_code: "12345".to_string(),
            name: "Test Station".to_string(),
            coordinates: Coordinates::new(48.8566, 2.3522),
            capacity: 20,
            commune: None,
            insee_code: None,
        };

        let real_time = RealTimeStatus {
            station_code: "12345".to_string(),
            bikes: BikeAvailability::new(15, 10), // 25 bikes
            available_docks: 5,                   // 25+5=30 > 20 capacity
            service: ServiceCapabilities::default(),
            status: StationStatus::Operational,
            last_updated: Utc::now(),
            valid_until: Utc::now() + Duration::minutes(5),
        };

        let station = VelibStation::new(reference).with_real_time(real_time);
        let result = station.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::Error::CapacityOverflow { .. }
        ));
    }
}
