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
const REFERENCE_CACHE_TTL_MINUTES: i64 = 5;
const REALTIME_CACHE_TTL_MINUTES: i64 = 2;

// Number of records requested per API page
const API_PAGE_SIZE: usize = 100;

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

    /// Fetch all pages from a paginated API endpoint, calling `parse_record` on each item.
    ///
    /// Records that fail to parse are silently skipped (logged upstream callers can add
    /// tracing if needed).  Returns when a page comes back with fewer items than the page
    /// size, signalling the last page.
    async fn fetch_paginated<T, F>(&self, url: &str, mut parse_record: F) -> Result<Vec<T>>
    where
        F: FnMut(&Value) -> Result<T>,
    {
        let mut results = Vec::new();
        let mut offset = 0;

        loop {
            let query_params = &[
                ("limit", &API_PAGE_SIZE.to_string()),
                ("offset", &offset.to_string()),
            ];

            let response = self.client.get_with_query(url, query_params).await?;
            let json: Value = response.json().await?;
            let records = json["results"]
                .as_array()
                .ok_or_else(|| Error::Internal(anyhow::anyhow!("Invalid API response format")))?;

            if records.is_empty() {
                break;
            }

            for record in records {
                if let Ok(item) = parse_record(record) {
                    results.push(item);
                }
            }

            offset += API_PAGE_SIZE;
            if records.len() < API_PAGE_SIZE {
                break;
            }
        }

        Ok(results)
    }

    /// Fetch all station reference data
    pub async fn fetch_reference_stations(&mut self) -> Result<Vec<StationReference>> {
        const CACHE_KEY: &str = "all_reference_stations";

        if let Some(cached) = self.reference_cache.get(&CACHE_KEY.to_string()).await {
            debug!("Using cached reference stations: {} stations", cached.len());
            return Ok(cached);
        }

        info!("Fetching reference stations from Paris Open Data API");

        let stations = self
            .fetch_paginated(VELIB_STATIONS_URL, |r| self.parse_reference_station(r))
            .await?;

        info!("Fetched {} reference stations", stations.len());
        self.reference_cache
            .insert(CACHE_KEY.to_string(), stations.clone())
            .await;

        Ok(stations)
    }

    /// Fetch real-time station status data
    pub async fn fetch_realtime_status(&mut self) -> Result<HashMap<String, RealTimeStatus>> {
        const CACHE_KEY: &str = "all_realtime_status";

        if let Some(cached) = self.realtime_cache.get(&CACHE_KEY.to_string()).await {
            debug!("Using cached real-time status: {} stations", cached.len());
            return Ok(cached);
        }

        info!("Fetching real-time status from Paris Open Data API");

        let pairs = self
            .fetch_paginated(VELIB_REALTIME_URL, |r| self.parse_realtime_status(r))
            .await?;

        let all_status: HashMap<String, RealTimeStatus> = pairs.into_iter().collect();

        info!("Fetched real-time status for {} stations", all_status.len());
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
            capabilities: ServiceCapabilities::default(),
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

        let bikes = BikeAvailability::new(mechanical_bikes, electric_bikes);
        Ok((
            station_code,
            RealTimeStatus::new(bikes, available_docks, status, last_update),
        ))
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
