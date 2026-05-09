use crate::data::cache::InMemoryCache;
use crate::data::retry::{RetryConfig, RetryPolicy, RetryableHttpClient};
use crate::types::{
    BikeAvailability, RealTimeStatus, ServiceCapabilities, StationReference, StationStatus,
    VelibStation,
};
use crate::{Error, Result};
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, info};

// Paris Open Data API endpoints
const VELIB_STATIONS_URL: &str = "https://opendata.paris.fr/api/explore/v2.1/catalog/datasets/velib-emplacement-des-stations/records";
const VELIB_REALTIME_URL: &str = "https://opendata.paris.fr/api/explore/v2.1/catalog/datasets/velib-disponibilite-en-temps-reel/records";

// Cache TTLs
const REFERENCE_CACHE_TTL_MINUTES: i64 = 5; // 5 minutes for reference data
const REALTIME_CACHE_TTL_MINUTES: i64 = 2; // 2 minutes for real-time data

#[derive(Debug)]
pub struct VelibDataClient {
    client: RetryableHttpClient,
    reference_cache: InMemoryCache<String, Vec<StationReference>>,
    realtime_cache: InMemoryCache<String, HashMap<String, RealTimeStatus>>,
}

impl Default for VelibDataClient {
    fn default() -> Self {
        Self::new()
    }
}

impl VelibDataClient {
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: RetryableHttpClient::new(),
            reference_cache: InMemoryCache::new(Duration::minutes(REFERENCE_CACHE_TTL_MINUTES)),
            realtime_cache: InMemoryCache::new(Duration::minutes(REALTIME_CACHE_TTL_MINUTES)),
        }
    }

    /// Create a new client with custom retry configuration
    ///
    /// # Example
    /// ```
    /// use velib_mcp::data::{VelibDataClient, RetryConfig};
    ///
    /// let retry_config = RetryConfig {
    ///     max_attempts: 5,
    ///     base_delay_seconds: 2,
    ///     max_delay_seconds: 120,
    ///     use_jitter: true,
    /// };
    ///
    /// let client = VelibDataClient::with_retry_config(retry_config);
    /// ```
    #[must_use]
    pub fn with_retry_config(retry_config: RetryConfig) -> Self {
        let retry_policy = RetryPolicy::with_config(retry_config);
        Self {
            client: RetryableHttpClient::with_retry_policy(retry_policy),
            reference_cache: InMemoryCache::new(Duration::minutes(REFERENCE_CACHE_TTL_MINUTES)),
            realtime_cache: InMemoryCache::new(Duration::minutes(REALTIME_CACHE_TTL_MINUTES)),
        }
    }

    /// Fetch all station reference data
    pub async fn fetch_reference_stations(&mut self) -> Result<Vec<StationReference>> {
        const CACHE_KEY: &str = "all_reference_stations";

        // Check cache first
        if let Some(cached) = self.reference_cache.get(&CACHE_KEY.to_string()).await {
            debug!("Using cached reference stations: {} stations", cached.len());
            return Ok(cached);
        }

        info!("Fetching reference stations from Paris Open Data API");

        let mut all_stations = Vec::new();
        let mut offset = 0;
        let limit = 100; // API limit

        loop {
            let query_params = &[
                ("limit", &limit.to_string()),
                ("offset", &offset.to_string()),
            ];

            let response = self
                .client
                .get_with_query(VELIB_STATIONS_URL, query_params)
                .await?;

            let json: Value = response.json().await?;
            let records = json["results"]
                .as_array()
                .ok_or_else(|| Error::Internal(anyhow::anyhow!("Invalid API response format")))?;

            if records.is_empty() {
                break; // No more records
            }

            for record in records {
                if let Ok(station) = self.parse_reference_station(record) {
                    all_stations.push(station);
                }
            }

            offset += limit;
            if records.len() < limit {
                break; // Last page
            }
        }

        info!("Fetched {} reference stations", all_stations.len());

        // Cache the results
        self.reference_cache
            .insert(CACHE_KEY.to_string(), all_stations.clone())
            .await;

        Ok(all_stations)
    }

    /// Fetch real-time station status data
    pub async fn fetch_realtime_status(&mut self) -> Result<HashMap<String, RealTimeStatus>> {
        const CACHE_KEY: &str = "all_realtime_status";

        // Check cache first
        if let Some(cached) = self.realtime_cache.get(&CACHE_KEY.to_string()).await {
            debug!("Using cached real-time status: {} stations", cached.len());
            return Ok(cached);
        }

        info!("Fetching real-time status from Paris Open Data API");

        let mut all_status = HashMap::new();
        let mut offset = 0;
        let limit = 100; // API limit

        loop {
            let query_params = &[
                ("limit", &limit.to_string()),
                ("offset", &offset.to_string()),
            ];

            let response = self
                .client
                .get_with_query(VELIB_REALTIME_URL, query_params)
                .await?;

            let json: Value = response.json().await?;
            let records = json["results"]
                .as_array()
                .ok_or_else(|| Error::Internal(anyhow::anyhow!("Invalid API response format")))?;

            if records.is_empty() {
                break; // No more records
            }

            for record in records {
                if let Ok((station_code, status)) = self.parse_realtime_status(record) {
                    all_status.insert(station_code, status);
                }
            }

            offset += limit;
            if records.len() < limit {
                break; // Last page
            }
        }

        info!("Fetched real-time status for {} stations", all_status.len());

        // Cache the results
        self.realtime_cache
            .insert(CACHE_KEY.to_string(), all_status.clone())
            .await;

        Ok(all_status)
    }

    /// Get all stations with optional real-time data
    pub async fn get_all_stations(&mut self, include_realtime: bool) -> Result<Vec<VelibStation>> {
        let reference_stations = self.fetch_reference_stations().await?;

        if !include_realtime {
            return Ok(reference_stations
                .into_iter()
                .map(VelibStation::new)
                .collect());
        }

        let realtime_status = self.fetch_realtime_status().await?;

        let stations = reference_stations
            .into_iter()
            .map(|ref_station| {
                let mut station = VelibStation::new(ref_station);
                if let Some(rt_status) = realtime_status.get(&station.reference.station_code) {
                    station = station.with_real_time(rt_status.clone());
                }
                station
            })
            .collect();

        Ok(stations)
    }

    /// Get a specific station by code
    pub async fn get_station_by_code(
        &mut self,
        station_code: &str,
        include_realtime: bool,
    ) -> Result<Option<VelibStation>> {
        let all_stations = self.get_all_stations(include_realtime).await?;
        Ok(all_stations
            .into_iter()
            .find(|station| station.reference.station_code == station_code))
    }

    /// Parse reference station data from API response
    fn parse_reference_station(&self, record: &Value) -> Result<StationReference> {
        let station_code = record["stationcode"]
            .as_str()
            .ok_or_else(|| Error::Internal(anyhow::anyhow!("Missing station code")))?
            .to_string();

        let name = record["name"]
            .as_str()
            .ok_or_else(|| Error::Internal(anyhow::anyhow!("Missing station name")))?
            .to_string();

        let capacity = record["capacity"]
            .as_u64()
            .ok_or_else(|| Error::Internal(anyhow::anyhow!("Missing capacity")))?
            as u16;

        // Parse coordinates from coordonnees_geo
        let geo_point = record["coordonnees_geo"]
            .as_object()
            .ok_or_else(|| Error::Internal(anyhow::anyhow!("Missing geo coordinates")))?;

        let latitude = geo_point["lat"]
            .as_f64()
            .ok_or_else(|| Error::Internal(anyhow::anyhow!("Missing latitude")))?;

        let longitude = geo_point["lon"]
            .as_f64()
            .ok_or_else(|| Error::Internal(anyhow::anyhow!("Missing longitude")))?;

        let coordinates = crate::types::Coordinates::new(latitude, longitude);

        // Parse service capabilities
        let capabilities = ServiceCapabilities {
            accepts_credit_card: false,  // Not available in current API
            has_charging_station: false, // Not available in current API
            is_virtual_station: false,   // Not available in current API
        };

        Ok(StationReference {
            station_code,
            name,
            coordinates,
            capacity,
            capabilities,
        })
    }

    /// Parse real-time status data from API response
    fn parse_realtime_status(&self, record: &Value) -> Result<(String, RealTimeStatus)> {
        let station_code = record["stationcode"]
            .as_str()
            .ok_or_else(|| Error::Internal(anyhow::anyhow!("Missing station code")))?
            .to_string();

        let mechanical_bikes = record["mechanical"].as_u64().unwrap_or(0) as u16;

        let electric_bikes = record["ebike"].as_u64().unwrap_or(0) as u16;

        let available_docks = record["numdocksavailable"].as_u64().unwrap_or(0) as u16;

        // Parse status
        let status_str = record["is_installed"].as_str().unwrap_or("NON");

        let status = match status_str {
            "OUI" => {
                let is_renting = record["is_renting"].as_str().unwrap_or("NON") == "OUI";
                let is_returning = record["is_returning"].as_str().unwrap_or("NON") == "OUI";

                if is_renting && is_returning {
                    StationStatus::Open
                } else {
                    StationStatus::Maintenance
                }
            }
            _ => StationStatus::Closed,
        };

        // Parse last update time
        let default_time = Utc::now().to_rfc3339();
        let last_update_str = record["duedate"].as_str().unwrap_or(&default_time);

        let last_update = DateTime::parse_from_rfc3339(last_update_str)
            .map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc));

        let bikes = BikeAvailability::new(mechanical_bikes, electric_bikes);

        let real_time_status = RealTimeStatus::new(bikes, available_docks, status, last_update);

        Ok((station_code, real_time_status))
    }

    /// Clean up expired cache entries
    pub async fn cleanup_cache(&self) {
        self.reference_cache.cleanup_expired().await;
        self.realtime_cache.cleanup_expired().await;
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> (usize, usize) {
        let reference_size = self.reference_cache.size().await;
        let realtime_size = self.realtime_cache.size().await;
        (reference_size, realtime_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Note: make_client() constructs a real VelibDataClient (which initialises
    // a live RetryableHttpClient internally). The network is never called in
    // these unit tests because only the pure parsing helpers are exercised.
    fn make_client() -> VelibDataClient {
        VelibDataClient::new()
    }

    // ---------------------------------------------------------------
    // parse_realtime_status tests
    // ---------------------------------------------------------------

    #[test]
    fn parse_realtime_status_open_station() {
        let client = make_client();
        let record = json!({
            "stationcode": "16107",
            "mechanical": 3,
            "ebike": 2,
            "numdocksavailable": 10,
            "is_installed": "OUI",
            "is_renting": "OUI",
            "is_returning": "OUI",
            "duedate": "2024-01-15T10:00:00+00:00"
        });

        let result = client.parse_realtime_status(&record);
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let (code, status) = result.unwrap();
        assert_eq!(code, "16107");
        assert_eq!(status.status, StationStatus::Open);
        assert_eq!(status.bikes.mechanical, 3);
        assert_eq!(status.bikes.electric, 2);
        assert_eq!(status.available_docks, 10);
    }

    #[test]
    fn parse_realtime_status_maintenance() {
        // is_installed OUI but is_renting NON → Maintenance
        let client = make_client();
        let record = json!({
            "stationcode": "16108",
            "mechanical": 0,
            "ebike": 0,
            "numdocksavailable": 20,
            "is_installed": "OUI",
            "is_renting": "NON",
            "is_returning": "OUI",
            "duedate": "2024-01-15T10:00:00+00:00"
        });

        let result = client.parse_realtime_status(&record);
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let (_, status) = result.unwrap();
        assert_eq!(status.status, StationStatus::Maintenance);
    }

    #[test]
    fn parse_realtime_status_renting_but_not_returning_is_maintenance() {
        // is_installed OUI, is_renting OUI but is_returning NON → Maintenance
        // (symmetry case: ensures the && condition catches both sub-cases)
        let client = make_client();
        let record = json!({
            "stationcode": "16110",
            "mechanical": 1,
            "ebike": 0,
            "numdocksavailable": 5,
            "is_installed": "OUI",
            "is_renting": "OUI",
            "is_returning": "NON",
            "duedate": "2024-01-15T10:00:00+00:00"
        });

        let result = client.parse_realtime_status(&record);
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let (_, status) = result.unwrap();
        assert_eq!(status.status, StationStatus::Maintenance);
    }

    #[test]
    fn parse_realtime_status_closed() {
        // is_installed NON → Closed
        let client = make_client();
        let record = json!({
            "stationcode": "16109",
            "mechanical": 0,
            "ebike": 0,
            "numdocksavailable": 0,
            "is_installed": "NON",
            "is_renting": "NON",
            "is_returning": "NON",
            "duedate": "2024-01-15T10:00:00+00:00"
        });

        let result = client.parse_realtime_status(&record);
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let (_, status) = result.unwrap();
        assert_eq!(status.status, StationStatus::Closed);
    }

    #[test]
    fn parse_realtime_status_missing_stationcode_returns_error() {
        let client = make_client();
        let record = json!({
            "mechanical": 2,
            "ebike": 1,
            "numdocksavailable": 8,
            "is_installed": "OUI",
            "is_renting": "OUI",
            "is_returning": "OUI",
            "duedate": "2024-01-15T10:00:00+00:00"
        });
        assert!(
            client.parse_realtime_status(&record).is_err(),
            "Expected Err for missing stationcode"
        );
    }

    // ---------------------------------------------------------------
    // parse_reference_station tests
    // ---------------------------------------------------------------

    #[test]
    fn parse_reference_station_happy_path() {
        let client = make_client();
        let record = json!({
            "stationcode": "10042",
            "name": "Benjamin Franklin - Ranelagh",
            "capacity": 35,
            "coordonnees_geo": {
                "lat": 48.8566,
                "lon": 2.3522
            }
        });

        let result = client.parse_reference_station(&record);
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let station = result.unwrap();
        assert_eq!(station.station_code, "10042");
        assert_eq!(station.name, "Benjamin Franklin - Ranelagh");
        assert_eq!(station.capacity, 35);
        assert!(
            (station.coordinates.latitude - 48.8566).abs() < 1e-6,
            "latitude mismatch"
        );
        assert!(
            (station.coordinates.longitude - 2.3522).abs() < 1e-6,
            "longitude mismatch"
        );
    }

    #[test]
    fn parse_reference_station_missing_field_returns_error() {
        let client = make_client();
        // Missing "capacity" field
        let record = json!({
            "stationcode": "10042",
            "name": "Some Station",
            "coordonnees_geo": {
                "lat": 48.8566,
                "lon": 2.3522
            }
        });

        let result = client.parse_reference_station(&record);
        assert!(result.is_err(), "Expected Err for missing capacity");
    }

    #[test]
    fn parse_reference_station_missing_coordinates_returns_error() {
        let client = make_client();
        // coordonnees_geo omitted — exercises the geo ok_or_else error path
        let record = json!({
            "stationcode": "10042",
            "name": "Some Station",
            "capacity": 35
        });

        assert!(
            client.parse_reference_station(&record).is_err(),
            "Expected Err for missing coordonnees_geo"
        );
    }
}
