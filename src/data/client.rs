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
use tracing::{debug, info, warn};

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
            let records = json["results"].as_array().ok_or_else(|| Error::Internal {
                message: "Invalid API response format".to_string(),
            })?;

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
            let records = json["results"].as_array().ok_or_else(|| Error::Internal {
                message: "Invalid API response format".to_string(),
            })?;

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

    /// Get all stations with optional real-time data and fallback handling
    pub async fn get_all_stations(&mut self, include_realtime: bool) -> Result<Vec<VelibStation>> {
        let reference_stations = self.get_reference_stations_with_fallback().await?;

        if !include_realtime {
            return Ok(reference_stations
                .into_iter()
                .map(VelibStation::new)
                .collect());
        }

        // Try to get real-time data, but fallback gracefully if it fails
        let realtime_status = match self.get_realtime_status_with_fallback().await {
            Ok(status) => status,
            Err(e) => {
                debug!(
                    "Failed to fetch real-time data, returning reference data only: {}",
                    e
                );
                return Ok(reference_stations
                    .into_iter()
                    .map(VelibStation::new)
                    .collect());
            }
        };

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

    /// Get reference stations with cache fallback
    async fn get_reference_stations_with_fallback(&mut self) -> Result<Vec<StationReference>> {
        // Try fresh data first, then fallback to cache
        match self.fetch_reference_stations().await {
            Ok(stations) => Ok(stations),
            Err(e) => {
                warn!("Failed to fetch fresh reference data, trying cache: {}", e);
                const CACHE_KEY: &str = "all_reference_stations";

                if let Some(cached) = self.reference_cache.get(&CACHE_KEY.to_string()).await {
                    info!(
                        "Using stale cached reference stations: {} stations",
                        cached.len()
                    );
                    Ok(cached)
                } else {
                    Err(Error::cache_error(
                        "No cached reference data available",
                        "fallback",
                    ))
                }
            }
        }
    }

    /// Get real-time status with cache fallback  
    async fn get_realtime_status_with_fallback(
        &mut self,
    ) -> Result<HashMap<String, RealTimeStatus>> {
        // Try fresh data first, then fallback to cache
        match self.fetch_realtime_status().await {
            Ok(status) => Ok(status),
            Err(e) => {
                warn!("Failed to fetch fresh real-time data, trying cache: {}", e);
                const CACHE_KEY: &str = "all_realtime_status";

                if let Some(cached) = self.realtime_cache.get(&CACHE_KEY.to_string()).await {
                    info!(
                        "Using stale cached real-time status: {} stations",
                        cached.len()
                    );
                    Ok(cached)
                } else {
                    Err(Error::cache_error(
                        "No cached real-time data available",
                        "fallback",
                    ))
                }
            }
        }
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
            .ok_or_else(|| Error::validation_error("Missing station code", "stationcode"))?
            .to_string();

        let name = record["name"]
            .as_str()
            .ok_or_else(|| Error::validation_error("Missing station name", "name"))?
            .to_string();

        let capacity = record["capacity"]
            .as_u64()
            .ok_or_else(|| Error::validation_error("Missing capacity", "capacity"))?
            as u16;

        // Parse coordinates from coordonnees_geo
        let geo_point = record["coordonnees_geo"]
            .as_object()
            .ok_or_else(|| Error::validation_error("Missing geo coordinates", "coordonnees_geo"))?;

        let latitude = geo_point["lat"]
            .as_f64()
            .ok_or_else(|| Error::validation_error("Missing latitude", "coordonnees_geo.lat"))?;

        let longitude = geo_point["lon"]
            .as_f64()
            .ok_or_else(|| Error::validation_error("Missing longitude", "coordonnees_geo.lon"))?;

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
            .ok_or_else(|| Error::validation_error("Missing station code", "stationcode"))?
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
