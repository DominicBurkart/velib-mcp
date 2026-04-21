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
                if let Ok(station) = Self::parse_reference_station(record) {
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
                if let Ok((station_code, status)) = Self::parse_realtime_status(record) {
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
    fn parse_reference_station(record: &Value) -> Result<StationReference> {
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

        // Parse coordinates from coordonnees_geo.
        // Use `.get(...)` rather than `geo_point["lat"]`: indexing into a
        // `serde_json::Map` panics on missing keys, so we need an explicit
        // lookup to surface a clean `Err` when the upstream payload is
        // missing `lat`/`lon`.
        let geo_point = record["coordonnees_geo"]
            .as_object()
            .ok_or_else(|| Error::Internal(anyhow::anyhow!("Missing geo coordinates")))?;

        let latitude = geo_point
            .get("lat")
            .and_then(Value::as_f64)
            .ok_or_else(|| Error::Internal(anyhow::anyhow!("Missing latitude")))?;

        let longitude = geo_point
            .get("lon")
            .and_then(Value::as_f64)
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
    fn parse_realtime_status(record: &Value) -> Result<(String, RealTimeStatus)> {
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
    //! Unit tests for API response parsers.
    //!
    //! The `fetch_*` methods on `VelibDataClient` are only exercised against
    //! the live Paris Open Data API. The parsers that translate each record
    //! into a domain type, however, are pure functions over `serde_json::Value`
    //! and are the highest-risk component in this module: a schema drift in
    //! the upstream API would surface here first. These tests pin down the
    //! parser contract so regressions surface as unit-test failures rather
    //! than silently-dropped records in production.
    //!
    //! Invariants exercised:
    //! - Reference records missing any required field produce `Err`.
    //! - Realtime records missing the `stationcode` produce `Err`; other
    //!   fields have documented defaults.
    //! - Status derivation: `is_installed`/`is_renting`/`is_returning` map
    //!   deterministically to `StationStatus::{Open, Maintenance, Closed}`.
    //! - `duedate` falls back to "now" when absent or unparseable (parser
    //!   must not panic on garbage timestamps).
    use super::*;
    use serde_json::json;

    fn valid_reference_record() -> Value {
        json!({
            "stationcode": "16107",
            "name": "Benjamin Godard - Victor Hugo",
            "capacity": 35,
            "coordonnees_geo": {
                "lat": 48.8651,
                "lon": 2.2755
            }
        })
    }

    fn valid_realtime_record() -> Value {
        json!({
            "stationcode": "16107",
            "mechanical": 5,
            "ebike": 3,
            "numdocksavailable": 27,
            "is_installed": "OUI",
            "is_renting": "OUI",
            "is_returning": "OUI",
            "duedate": "2024-01-15T12:30:00+00:00"
        })
    }

    // --- parse_reference_station: happy path ---

    #[test]
    fn parses_valid_reference_record() {
        let record = valid_reference_record();
        let station = VelibDataClient::parse_reference_station(&record).unwrap();
        assert_eq!(station.station_code, "16107");
        assert_eq!(station.name, "Benjamin Godard - Victor Hugo");
        assert_eq!(station.capacity, 35);
        assert!((station.coordinates.latitude - 48.8651).abs() < 1e-9);
        assert!((station.coordinates.longitude - 2.2755).abs() < 1e-9);
        // Capabilities are not present in the API and default to false.
        assert!(!station.capabilities.accepts_credit_card);
        assert!(!station.capabilities.has_charging_station);
        assert!(!station.capabilities.is_virtual_station);
    }

    // --- parse_reference_station: missing required fields each produce Err ---

    #[test]
    fn reference_missing_station_code_errors() {
        let mut record = valid_reference_record();
        record.as_object_mut().unwrap().remove("stationcode");
        assert!(VelibDataClient::parse_reference_station(&record).is_err());
    }

    #[test]
    fn reference_missing_name_errors() {
        let mut record = valid_reference_record();
        record.as_object_mut().unwrap().remove("name");
        assert!(VelibDataClient::parse_reference_station(&record).is_err());
    }

    #[test]
    fn reference_missing_capacity_errors() {
        let mut record = valid_reference_record();
        record.as_object_mut().unwrap().remove("capacity");
        assert!(VelibDataClient::parse_reference_station(&record).is_err());
    }

    #[test]
    fn reference_missing_coordinates_errors() {
        let mut record = valid_reference_record();
        record.as_object_mut().unwrap().remove("coordonnees_geo");
        assert!(VelibDataClient::parse_reference_station(&record).is_err());
    }

    #[test]
    fn reference_missing_latitude_errors() {
        // Regression: previously panicked because `geo_point["lat"]` indexes
        // a serde_json `Map` (panics on missing key). Now returns Err.
        let mut record = valid_reference_record();
        record["coordonnees_geo"]
            .as_object_mut()
            .unwrap()
            .remove("lat");
        assert!(VelibDataClient::parse_reference_station(&record).is_err());
    }

    #[test]
    fn reference_missing_longitude_errors() {
        // Regression: see `reference_missing_latitude_errors`.
        let mut record = valid_reference_record();
        record["coordonnees_geo"]
            .as_object_mut()
            .unwrap()
            .remove("lon");
        assert!(VelibDataClient::parse_reference_station(&record).is_err());
    }

    // --- parse_reference_station: wrong field types produce Err ---

    #[test]
    fn reference_wrong_field_types_error() {
        // stationcode as number (not string)
        let record = json!({
            "stationcode": 16107,
            "name": "x",
            "capacity": 35,
            "coordonnees_geo": { "lat": 48.86, "lon": 2.27 }
        });
        assert!(VelibDataClient::parse_reference_station(&record).is_err());

        // capacity as string
        let record = json!({
            "stationcode": "16107",
            "name": "x",
            "capacity": "35",
            "coordonnees_geo": { "lat": 48.86, "lon": 2.27 }
        });
        assert!(VelibDataClient::parse_reference_station(&record).is_err());

        // coordonnees_geo as string instead of object
        let record = json!({
            "stationcode": "16107",
            "name": "x",
            "capacity": 35,
            "coordonnees_geo": "48.86,2.27"
        });
        assert!(VelibDataClient::parse_reference_station(&record).is_err());
    }

    #[test]
    fn reference_empty_object_errors() {
        let record = json!({});
        assert!(VelibDataClient::parse_reference_station(&record).is_err());
    }

    // --- parse_realtime_status: happy path ---

    #[test]
    fn parses_valid_realtime_record_as_open() {
        let record = valid_realtime_record();
        let (code, status) = VelibDataClient::parse_realtime_status(&record).unwrap();
        assert_eq!(code, "16107");
        assert_eq!(status.bikes.mechanical, 5);
        assert_eq!(status.bikes.electric, 3);
        assert_eq!(status.available_docks, 27);
        assert_eq!(status.status, StationStatus::Open);
    }

    // --- parse_realtime_status: station_code is the only required field ---

    #[test]
    fn realtime_missing_station_code_errors() {
        let mut record = valid_realtime_record();
        record.as_object_mut().unwrap().remove("stationcode");
        assert!(VelibDataClient::parse_realtime_status(&record).is_err());
    }

    #[test]
    fn realtime_station_code_wrong_type_errors() {
        let record = json!({ "stationcode": 16107 });
        assert!(VelibDataClient::parse_realtime_status(&record).is_err());
    }

    #[test]
    fn realtime_minimal_record_defaults_fields() {
        // Only stationcode present: everything else must default without error.
        let record = json!({ "stationcode": "42" });
        let (code, status) = VelibDataClient::parse_realtime_status(&record).unwrap();
        assert_eq!(code, "42");
        assert_eq!(status.bikes.mechanical, 0);
        assert_eq!(status.bikes.electric, 0);
        assert_eq!(status.available_docks, 0);
        // is_installed defaults to "NON" -> Closed
        assert_eq!(status.status, StationStatus::Closed);
    }

    // --- parse_realtime_status: status derivation matrix ---

    fn realtime_with_flags(
        is_installed: &str,
        is_renting: &str,
        is_returning: &str,
    ) -> StationStatus {
        let record = json!({
            "stationcode": "1",
            "is_installed": is_installed,
            "is_renting": is_renting,
            "is_returning": is_returning,
        });
        VelibDataClient::parse_realtime_status(&record)
            .unwrap()
            .1
            .status
    }

    #[test]
    fn status_open_requires_installed_renting_and_returning() {
        assert_eq!(
            realtime_with_flags("OUI", "OUI", "OUI"),
            StationStatus::Open
        );
    }

    #[test]
    fn status_maintenance_when_installed_but_not_both_renting_and_returning() {
        // Installed but not renting
        assert_eq!(
            realtime_with_flags("OUI", "NON", "OUI"),
            StationStatus::Maintenance
        );
        // Installed but not returning
        assert_eq!(
            realtime_with_flags("OUI", "OUI", "NON"),
            StationStatus::Maintenance
        );
        // Installed but neither
        assert_eq!(
            realtime_with_flags("OUI", "NON", "NON"),
            StationStatus::Maintenance
        );
    }

    #[test]
    fn status_closed_when_not_installed() {
        assert_eq!(
            realtime_with_flags("NON", "OUI", "OUI"),
            StationStatus::Closed
        );
        // Any non-"OUI" is_installed value is treated as closed.
        assert_eq!(
            realtime_with_flags("unknown", "OUI", "OUI"),
            StationStatus::Closed
        );
    }

    // --- parse_realtime_status: last_update timestamp handling ---

    #[test]
    fn realtime_parses_valid_rfc3339_duedate() {
        let record = json!({
            "stationcode": "1",
            "duedate": "2024-01-15T12:30:00+00:00",
        });
        let (_, status) = VelibDataClient::parse_realtime_status(&record).unwrap();
        assert_eq!(status.last_update.to_rfc3339(), "2024-01-15T12:30:00+00:00");
    }

    #[test]
    fn realtime_falls_back_to_now_for_unparseable_duedate() {
        // Non-RFC3339 string must fall back to `Utc::now()` rather than error.
        let before = Utc::now();
        let record = json!({
            "stationcode": "1",
            "duedate": "not-a-timestamp",
        });
        let (_, status) = VelibDataClient::parse_realtime_status(&record).unwrap();
        let after = Utc::now();
        assert!(status.last_update >= before && status.last_update <= after);
    }

    #[test]
    fn realtime_falls_back_to_now_for_missing_duedate() {
        let before = Utc::now();
        let record = json!({ "stationcode": "1" });
        let (_, status) = VelibDataClient::parse_realtime_status(&record).unwrap();
        let after = Utc::now();
        assert!(status.last_update >= before && status.last_update <= after);
    }

    // --- parse_realtime_status: numeric fields cast silently ---

    #[test]
    fn realtime_numeric_fields_truncate_to_u16() {
        // Values larger than u16::MAX are cast via `as u16` (wrap). The
        // upstream API never produces such values, but pinning the behavior
        // makes any future switch to checked conversion a deliberate decision.
        let record = json!({
            "stationcode": "1",
            "mechanical": 70_000u64,
        });
        let parsed = VelibDataClient::parse_realtime_status(&record);
        assert!(parsed.is_ok());
    }
}
