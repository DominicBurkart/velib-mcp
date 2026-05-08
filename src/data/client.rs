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

    // --- duedate parsing ---
    //
    // The `duedate` field drives `RealTimeStatus::last_update`, which in turn
    // determines `data_freshness`. The parser must:
    //   1. Accept RFC3339 timestamps and preserve their instant in UTC.
    //   2. Normalize non-UTC offsets to UTC.
    //   3. Fall back to "now" when `duedate` is missing or malformed, so a
    //      single bad row doesn't break the whole batch.
    //   4. Produce a `data_freshness` consistent with the parsed timestamp.

    #[test]
    fn parse_realtime_status_preserves_utc_duedate() {
        let record = json!({
            "stationcode": "1",
            "is_installed": "OUI",
            "is_renting": "OUI",
            "is_returning": "OUI",
            "duedate": "2026-04-23T10:00:00+00:00"
        });
        let (_, status) = parse_realtime_status(&record).expect("parses");
        let expected = DateTime::parse_from_rfc3339("2026-04-23T10:00:00+00:00")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(status.last_update, expected);
    }

    #[test]
    fn parse_realtime_status_normalizes_non_utc_offset_to_utc() {
        // 12:00 in +02:00 is 10:00 UTC. The parsed `last_update` must be the
        // same instant, expressed in UTC, regardless of the wire offset.
        let record = json!({
            "stationcode": "1",
            "is_installed": "OUI",
            "is_renting": "OUI",
            "is_returning": "OUI",
            "duedate": "2026-04-23T12:00:00+02:00"
        });
        let (_, status) = parse_realtime_status(&record).expect("parses");
        let expected = DateTime::parse_from_rfc3339("2026-04-23T10:00:00+00:00")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(status.last_update, expected);
    }

    #[test]
    fn parse_realtime_status_missing_duedate_falls_back_to_now() {
        // No `duedate` -> last_update should be ~now, not an error and not the
        // unix epoch. We allow generous slack to keep this test non-flaky on
        // slow CI.
        let before = Utc::now();
        let record = json!({
            "stationcode": "1",
            "is_installed": "OUI",
            "is_renting": "OUI",
            "is_returning": "OUI",
        });
        let (_, status) = parse_realtime_status(&record).expect("parses");
        let after = Utc::now();
        assert!(
            status.last_update >= before - chrono::Duration::seconds(1)
                && status.last_update <= after + chrono::Duration::seconds(1),
            "last_update {:?} not within [{:?}, {:?}]",
            status.last_update,
            before,
            after
        );
    }

    #[test]
    fn parse_realtime_status_malformed_duedate_falls_back_to_now() {
        // Non-RFC3339 strings should not surface as errors. The fallback path
        // must succeed and produce a "now" timestamp so a single corrupt row
        // doesn't poison the whole batch.
        let cases = [
            "not-a-date",
            "2026-04-23",           // date without time
            "2026/04/23 10:00:00",  // wrong separator
            "23-04-2026T10:00:00Z", // dd-mm-yyyy
            "",                     // empty
        ];
        for duedate in cases {
            let before = Utc::now();
            let record = json!({
                "stationcode": "1",
                "is_installed": "OUI",
                "is_renting": "OUI",
                "is_returning": "OUI",
                "duedate": duedate,
            });
            let (_, status) = parse_realtime_status(&record)
                .unwrap_or_else(|e| panic!("malformed duedate {duedate:?} should not error: {e}"));
            let after = Utc::now();
            assert!(
                status.last_update >= before - chrono::Duration::seconds(1)
                    && status.last_update <= after + chrono::Duration::seconds(1),
                "malformed duedate {duedate:?} produced last_update {:?}, expected ~now",
                status.last_update
            );
        }
    }

    #[test]
    fn parse_realtime_status_non_string_duedate_falls_back_to_now() {
        // `as_str()` returns None for non-string values; the fallback must
        // still kick in instead of panicking or erroring.
        let before = Utc::now();
        let record = json!({
            "stationcode": "1",
            "is_installed": "OUI",
            "is_renting": "OUI",
            "is_returning": "OUI",
            "duedate": 1_700_000_000,
        });
        let (_, status) = parse_realtime_status(&record).expect("parses");
        let after = Utc::now();
        assert!(
            status.last_update >= before - chrono::Duration::seconds(1)
                && status.last_update <= after + chrono::Duration::seconds(1)
        );
    }

    #[test]
    fn parse_realtime_status_old_duedate_yields_stale_freshness() {
        // A duedate well in the past must surface as a stale-or-worse
        // `data_freshness` so downstream consumers can warn the user. We use
        // a 6-hour-old timestamp -- past the 120-minute `VeryStale` boundary.
        let six_hours_ago = Utc::now() - chrono::Duration::hours(6);
        let record = json!({
            "stationcode": "1",
            "is_installed": "OUI",
            "is_renting": "OUI",
            "is_returning": "OUI",
            "duedate": six_hours_ago.to_rfc3339(),
        });
        let (_, status) = parse_realtime_status(&record).expect("parses");
        assert_eq!(
            status.data_freshness,
            crate::types::DataFreshness::VeryStale
        );
    }

    #[test]
    fn parse_realtime_status_recent_duedate_yields_fresh_freshness() {
        // A duedate just a few seconds old must classify as Fresh.
        let just_now = Utc::now() - chrono::Duration::seconds(5);
        let record = json!({
            "stationcode": "1",
            "is_installed": "OUI",
            "is_renting": "OUI",
            "is_returning": "OUI",
            "duedate": just_now.to_rfc3339(),
        });
        let (_, status) = parse_realtime_status(&record).expect("parses");
        assert_eq!(status.data_freshness, crate::types::DataFreshness::Fresh);
    }

    // --- type-mismatch handling for parse_reference_station ---
    //
    // Existing tests cover *missing* fields. These cover *wrong-typed* fields,
    // which the upstream API can produce when an operator pushes corrupt
    // metadata (e.g., `capacity` arriving as the string "35" rather than an
    // integer). Each case must surface as Err rather than panic.

    #[test]
    fn parse_reference_station_rejects_string_capacity() {
        let record = json!({
            "stationcode": "1",
            "name": "x",
            "capacity": "35",
            "coordonnees_geo": {"lat": 48.85, "lon": 2.35}
        });
        assert!(parse_reference_station(&record).is_err());
    }

    #[test]
    fn parse_reference_station_rejects_non_object_geo() {
        let record = json!({
            "stationcode": "1",
            "name": "x",
            "capacity": 35,
            "coordonnees_geo": "48.85,2.35"
        });
        assert!(parse_reference_station(&record).is_err());
    }

    #[test]
    fn parse_reference_station_rejects_string_coordinates() {
        let record = json!({
            "stationcode": "1",
            "name": "x",
            "capacity": 35,
            "coordonnees_geo": {"lat": "48.85", "lon": "2.35"}
        });
        assert!(parse_reference_station(&record).is_err());
    }

    #[test]
    fn parse_reference_station_rejects_numeric_station_code() {
        // The upstream payload uses string codes like "16107"; a numeric
        // `stationcode` must be rejected rather than coerced.
        let record = json!({
            "stationcode": 16107,
            "name": "x",
            "capacity": 35,
            "coordonnees_geo": {"lat": 48.85, "lon": 2.35}
        });
        assert!(parse_reference_station(&record).is_err());
    }

    // --- saturating field truncation in parse_realtime_status ---
    //
    // The parser converts u64 JSON values to u16 via `as u16`, which silently
    // wraps. Ensure realistic upstream values (always <= a few thousand)
    // round-trip correctly, and pin down the conversion behavior so a future
    // refactor to `try_into` doesn't break callers without warning.

    #[test]
    fn parse_realtime_status_preserves_in_range_counts() {
        let record = json!({
            "stationcode": "1",
            "mechanical": 30,
            "ebike": 25,
            "numdocksavailable": 100,
            "is_installed": "OUI",
            "is_renting": "OUI",
            "is_returning": "OUI",
        });
        let (_, status) = parse_realtime_status(&record).expect("parses");
        assert_eq!(status.bikes.mechanical, 30);
        assert_eq!(status.bikes.electric, 25);
        assert_eq!(status.available_docks, 100);
    }

    #[test]
    fn parse_realtime_status_negative_counts_treated_as_zero() {
        // `as_u64()` returns None for negative integers, so they fall through
        // to the `unwrap_or(0)` branch -- documented permissive behavior.
        let record = json!({
            "stationcode": "1",
            "mechanical": -5,
            "ebike": -1,
            "numdocksavailable": -3,
            "is_installed": "OUI",
            "is_renting": "OUI",
            "is_returning": "OUI",
        });
        let (_, status) = parse_realtime_status(&record).expect("parses");
        assert_eq!(status.bikes.mechanical, 0);
        assert_eq!(status.bikes.electric, 0);
        assert_eq!(status.available_docks, 0);
    }
}
