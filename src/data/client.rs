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
                if let Ok(station) = parse_reference_station(record) {
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
                if let Ok((station_code, status)) = parse_realtime_status(record) {
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

    /// Seed the reference-station cache with pre-built data, bypassing the network.
    ///
    /// This method exists for integration-test helpers. It is always compiled so
    /// that `tests/*.rs` files can call it without requiring `--all-features` or
    /// `cfg(test)` to be set on the library crate. The `_for_testing` suffix
    /// signals that it must not be called in production code.
    #[doc(hidden)]
    pub async fn seed_for_testing(&self, stations: Vec<StationReference>) {
        self.reference_cache
            .insert("all_reference_stations".to_string(), stations)
            .await;
    }

    /// Seed the real-time cache with pre-built data, bypassing the network.
    ///
    /// See [`seed_for_testing`](Self::seed_for_testing) for the rationale.
    #[doc(hidden)]
    pub async fn seed_realtime_for_testing(&self, status_map: HashMap<String, RealTimeStatus>) {
        self.realtime_cache
            .insert("all_realtime_status".to_string(), status_map)
            .await;
    }
}

/// Parse one reference station record from the Paris Open Data API.
///
/// Extracted as a free function (not a method) because it has no dependency
/// on `VelibDataClient` state. This keeps the parser cheaply testable against
/// fixture JSON values.
pub(crate) fn parse_reference_station(record: &Value) -> Result<StationReference> {
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

    let geo_point = record["coordonnees_geo"]
        .as_object()
        .ok_or_else(|| Error::Internal(anyhow::anyhow!("Missing geo coordinates")))?;

    let latitude = geo_point["lat"]
        .as_f64()
        .ok_or_else(|| Error::Internal(anyhow::anyhow!("Missing latitude")))?;

    let longitude = geo_point["lon"]
        .as_f64()
        .ok_or_else(|| Error::Internal(anyhow::anyhow!("Missing longitude")))?;

    Ok(StationReference {
        station_code,
        name,
        coordinates: crate::types::Coordinates::new(latitude, longitude),
        capacity,
        // The Paris Open Data stations dataset does not expose these fields.
        capabilities: ServiceCapabilities::default(),
    })
}

/// Parse one real-time status record from the Paris Open Data API.
///
/// Returns the station code alongside the parsed status so callers can index
/// the result by code. Missing bike/dock counts are coerced to 0 rather than
/// failing the row -- this matches how the upstream API sometimes omits fields
/// for stations temporarily out of service.
pub(crate) fn parse_realtime_status(record: &Value) -> Result<(String, RealTimeStatus)> {
    let station_code = record["stationcode"]
        .as_str()
        .ok_or_else(|| Error::Internal(anyhow::anyhow!("Missing station code")))?
        .to_string();

    let mechanical_bikes = record["mechanical"].as_u64().unwrap_or(0) as u16;
    let electric_bikes = record["ebike"].as_u64().unwrap_or(0) as u16;
    let available_docks = record["numdocksavailable"].as_u64().unwrap_or(0) as u16;

    let status = match record["is_installed"].as_str().unwrap_or("NON") {
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

    let last_update = record["duedate"]
        .as_str()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map_or_else(Utc::now, |dt| dt.with_timezone(&Utc));

    Ok((
        station_code,
        RealTimeStatus::new(
            BikeAvailability::new(mechanical_bikes, electric_bikes),
            available_docks,
            status,
            last_update,
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture_reference() -> Value {
        json!({
            "stationcode": "16107",
            "name": "Benjamin Godard - Victor Hugo",
            "capacity": 35,
            "coordonnees_geo": { "lat": 48.8656, "lon": 2.2779 }
        })
    }

    #[test]
    fn parse_reference_station_happy_path() {
        let station = parse_reference_station(&fixture_reference()).expect("parses");
        assert_eq!(station.station_code, "16107");
        assert_eq!(station.name, "Benjamin Godard - Victor Hugo");
        assert_eq!(station.capacity, 35);
        assert!((station.coordinates.latitude - 48.8656).abs() < 1e-9);
        assert!((station.coordinates.longitude - 2.2779).abs() < 1e-9);
    }

    #[test]
    fn parse_reference_station_rejects_missing_fields() {
        let cases = [
            json!({"name": "x", "capacity": 10, "coordonnees_geo": {"lat": 48.85, "lon": 2.35}}),
            json!({"stationcode": "1", "capacity": 10, "coordonnees_geo": {"lat": 48.85, "lon": 2.35}}),
            json!({"stationcode": "1", "name": "x", "coordonnees_geo": {"lat": 48.85, "lon": 2.35}}),
            json!({"stationcode": "1", "name": "x", "capacity": 10}),
        ];
        for bad in cases {
            assert!(parse_reference_station(&bad).is_err(), "bad={bad}");
        }
    }

    #[test]
    fn parse_realtime_status_maps_renting_returning_to_open() {
        let record = json!({
            "stationcode": "16107",
            "mechanical": 5,
            "ebike": 3,
            "numdocksavailable": 12,
            "is_installed": "OUI",
            "is_renting": "OUI",
            "is_returning": "OUI",
            "duedate": "2026-04-23T10:00:00+00:00"
        });
        let (code, status) = parse_realtime_status(&record).expect("parses");
        assert_eq!(code, "16107");
        assert_eq!(status.bikes.mechanical, 5);
        assert_eq!(status.bikes.electric, 3);
        assert_eq!(status.available_docks, 12);
        assert_eq!(status.status, StationStatus::Open);
    }

    #[test]
    fn parse_realtime_status_installed_but_not_renting_is_maintenance() {
        let record = json!({
            "stationcode": "1",
            "is_installed": "OUI",
            "is_renting": "NON",
            "is_returning": "OUI",
        });
        let (_, status) = parse_realtime_status(&record).expect("parses");
        assert_eq!(status.status, StationStatus::Maintenance);
    }

    #[test]
    fn parse_realtime_status_uninstalled_is_closed() {
        let record = json!({
            "stationcode": "1",
            "is_installed": "NON",
        });
        let (_, status) = parse_realtime_status(&record).expect("parses");
        assert_eq!(status.status, StationStatus::Closed);
    }

    #[test]
    fn parse_realtime_status_missing_counts_default_to_zero() {
        // Matches upstream behavior: stations temporarily missing mechanical/ebike/
        // numdocksavailable fields should not fail the whole batch.
        let record = json!({
            "stationcode": "1",
            "is_installed": "OUI",
            "is_renting": "OUI",
            "is_returning": "OUI",
        });
        let (_, status) = parse_realtime_status(&record).expect("parses");
        assert_eq!(status.bikes.mechanical, 0);
        assert_eq!(status.bikes.electric, 0);
        assert_eq!(status.available_docks, 0);
    }

    #[test]
    fn parse_realtime_status_rejects_missing_station_code() {
        let record = json!({"is_installed": "OUI"});
        assert!(parse_realtime_status(&record).is_err());
    }

    /// A numeric `stationcode` must be rejected.
    ///
    /// `parse_realtime_status` extracts the station code via `.as_str()`, which
    /// returns `None` for non-string JSON values. This test asserts that a
    /// numeric station code (e.g. `16107` instead of `"16107"`) is treated as a
    /// parse failure rather than being silently coerced to a string — a coercion
    /// regression would silently discard or mis-key real-time records.
    #[test]
    fn parse_realtime_status_rejects_numeric_station_code() {
        let record = json!({
            "stationcode": 16107,
            "is_installed": "OUI",
            "is_renting": "OUI",
            "is_returning": "OUI",
        });
        assert!(
            parse_realtime_status(&record).is_err(),
            "a numeric stationcode should be rejected; as_str() returns None for non-string JSON values"
        );
    }
}
