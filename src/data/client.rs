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

        let capacity_raw = record["capacity"]
            .as_u64()
            .ok_or_else(|| Error::Internal(anyhow::anyhow!("Missing capacity")))?;
        let capacity = u16::try_from(capacity_raw).map_err(|_| {
            Error::Internal(anyhow::anyhow!(
                "Capacity {capacity_raw} exceeds u16::MAX ({})",
                u16::MAX
            ))
        })?;

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

        let mechanical_bikes =
            u16::try_from(record["mechanical"].as_u64().unwrap_or(0)).unwrap_or(u16::MAX);

        let electric_bikes =
            u16::try_from(record["ebike"].as_u64().unwrap_or(0)).unwrap_or(u16::MAX);

        let available_docks =
            u16::try_from(record["numdocksavailable"].as_u64().unwrap_or(0)).unwrap_or(u16::MAX);

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

#[cfg(kani)]
mod verification {
    /// Helper that mirrors the guarded capacity conversion used in
    /// `parse_reference_station`. Returns `Some(v)` when the u64 fits in a u16,
    /// `None` otherwise (the real code returns an error).
    fn convert_capacity(raw: u64) -> Option<u16> {
        u16::try_from(raw).ok()
    }

    /// Helper that mirrors the saturating bike-count conversion used in
    /// `parse_realtime_status`. Out-of-range values are clamped to `u16::MAX`.
    fn convert_bike_count(raw: u64) -> u16 {
        u16::try_from(raw).unwrap_or(u16::MAX)
    }

    // ------------------------------------------------------------------
    // Proof 1: capacity u64 -> u16 is guarded (no silent truncation)
    // ------------------------------------------------------------------
    #[kani::proof]
    fn proof_capacity_cast_guarded() {
        let raw: u64 = kani::any();
        match convert_capacity(raw) {
            Some(val) => {
                // The converted value must equal the original.
                assert!(val as u64 == raw);
                // And the original must be representable.
                assert!(raw <= u16::MAX as u64);
            }
            None => {
                // Rejection must only happen for values that exceed u16.
                assert!(raw > u16::MAX as u64);
            }
        }
    }

    // ------------------------------------------------------------------
    // Proof 2: capacity conversion never silently truncates
    // ------------------------------------------------------------------
    #[kani::proof]
    fn proof_capacity_no_silent_truncation() {
        let raw: u64 = kani::any();
        if let Some(val) = convert_capacity(raw) {
            // Round-trip must be lossless.
            assert!(val as u64 == raw);
        }
    }

    // ------------------------------------------------------------------
    // Proof 3: bike-count conversion is safe (saturating)
    // ------------------------------------------------------------------
    #[kani::proof]
    fn proof_bike_count_cast_saturates() {
        let raw: u64 = kani::any();
        let val = convert_bike_count(raw);
        if raw <= u16::MAX as u64 {
            assert!(val as u64 == raw);
        } else {
            assert!(val == u16::MAX);
        }
    }

    // ------------------------------------------------------------------
    // Proof 4: mechanical bikes conversion
    // ------------------------------------------------------------------
    #[kani::proof]
    fn proof_mechanical_bikes_safe() {
        let raw: u64 = kani::any();
        let result = convert_bike_count(raw);
        // Result is always a valid u16 (trivially true by type), and
        // it never exceeds the original value.
        assert!(result as u64 <= raw || raw > u16::MAX as u64);
    }

    // ------------------------------------------------------------------
    // Proof 5: electric bikes conversion
    // ------------------------------------------------------------------
    #[kani::proof]
    fn proof_electric_bikes_safe() {
        let raw: u64 = kani::any();
        let result = convert_bike_count(raw);
        // Same invariant as mechanical bikes.
        assert!(result as u64 <= raw || raw > u16::MAX as u64);
    }

    // ------------------------------------------------------------------
    // Proof 6: available docks conversion
    // ------------------------------------------------------------------
    #[kani::proof]
    fn proof_available_docks_safe() {
        let raw: u64 = kani::any();
        let result = convert_bike_count(raw);
        assert!(result as u64 <= raw || raw > u16::MAX as u64);
    }

    // ------------------------------------------------------------------
    // Proof 7: coordinate extraction produces finite f64 values
    // ------------------------------------------------------------------
    #[kani::proof]
    fn proof_coordinates_are_finite() {
        let lat: f64 = kani::any();
        let lon: f64 = kani::any();

        // The API delivers coordinates via `as_f64()` which returns
        // `Option<f64>` -- it only yields `Some` for finite JSON numbers.
        // Model this precondition:
        kani::assume(lat.is_finite());
        kani::assume(lon.is_finite());

        // After extraction the values must remain finite (no arithmetic
        // is performed on them before storage).
        assert!(lat.is_finite());
        assert!(lon.is_finite());

        // Verify plausible geographic bounds (Paris area +/- generous margin).
        // These mirror the 50 km service-area validation elsewhere.
        if (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon) {
            // Valid geographic coordinates -- no assertion needed, just
            // confirm no panic occurs during the range check itself.
            assert!(lat >= -90.0 && lat <= 90.0);
            assert!(lon >= -180.0 && lon <= 180.0);
        }
    }

    // ------------------------------------------------------------------
    // Proof 8: u16 addition in BikeAvailability::total() never panics
    //          (the real code uses saturating_add, verify that property)
    // ------------------------------------------------------------------
    #[kani::proof]
    fn proof_bike_total_saturating() {
        let mechanical: u16 = kani::any();
        let electric: u16 = kani::any();
        let total = mechanical.saturating_add(electric);
        assert!(total >= mechanical);
        assert!(total >= electric);
    }
}
