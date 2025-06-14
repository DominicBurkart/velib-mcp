use chrono::{DateTime, Duration, Utc};
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use tracing::{info, warn};

use crate::types::{
    BikeAvailability, Coordinates, RealTimeStatus, ServiceCapabilities, StationReference,
    StationStatus, VelibStation,
};
use crate::{Error, Result};

const VELIB_REALTIME_URL: &str = "https://opendata.paris.fr/api/explore/v2.1/catalog/datasets/velib-disponibilite-en-temps-reel/records";
const VELIB_STATIONS_URL: &str = "https://opendata.paris.fr/api/explore/v2.1/catalog/datasets/velib-emplacement-des-stations/records";

pub struct VelibDataClient {
    client: Client,
    stations_cache: Option<(HashMap<String, StationReference>, DateTime<Utc>)>,
    realtime_cache: Option<(HashMap<String, RealTimeStatus>, DateTime<Utc>)>,
    cache_duration: Duration,
}

impl Default for VelibDataClient {
    fn default() -> Self {
        Self::new()
    }
}

impl VelibDataClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            stations_cache: None,
            realtime_cache: None,
            cache_duration: Duration::minutes(5), // Cache for 5 minutes
        }
    }

    pub async fn get_all_stations(&mut self, include_real_time: bool) -> Result<Vec<VelibStation>> {
        let reference_stations = self.get_reference_stations().await?;

        if !include_real_time {
            return Ok(reference_stations
                .into_values()
                .map(VelibStation::new)
                .collect());
        }

        let real_time_data = self.get_realtime_data().await?;

        let stations = reference_stations
            .into_iter()
            .map(|(code, reference)| {
                let mut station = VelibStation::new(reference);
                if let Some(real_time) = real_time_data.get(&code) {
                    station = station.with_real_time(real_time.clone());
                }
                station
            })
            .collect();

        Ok(stations)
    }

    pub async fn get_station_by_code(
        &mut self,
        station_code: &str,
        include_real_time: bool,
    ) -> Result<Option<VelibStation>> {
        let reference_stations = self.get_reference_stations().await?;

        if let Some(reference) = reference_stations.get(station_code) {
            let mut station = VelibStation::new(reference.clone());

            if include_real_time {
                let real_time_data = self.get_realtime_data().await?;
                if let Some(real_time) = real_time_data.get(station_code) {
                    station = station.with_real_time(real_time.clone());
                }
            }

            Ok(Some(station))
        } else {
            Ok(None)
        }
    }

    async fn get_reference_stations(&mut self) -> Result<HashMap<String, StationReference>> {
        // Check cache
        if let Some((ref cached_data, cached_time)) = &self.stations_cache {
            if Utc::now().signed_duration_since(*cached_time) < self.cache_duration {
                return Ok(cached_data.clone());
            }
        }

        info!("Fetching reference stations from API");

        // Fetch data with pagination (API limit is 100 records per request)
        let mut stations = HashMap::new();
        let mut offset = 0;
        let limit = 100;

        loop {
            let response = self
                .client
                .get(VELIB_STATIONS_URL)
                .query(&[
                    ("limit", &limit.to_string()),
                    ("offset", &offset.to_string()),
                ])
                .send()
                .await?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "No body".to_string());
                return Err(Error::Internal(anyhow::anyhow!(
                    "API request failed with status: {}, body: {}",
                    status,
                    body
                )));
            }

            let json: Value = response.json().await?;
            let results = json
                .get("results")
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    Error::Internal(anyhow::anyhow!("Invalid API response structure"))
                })?;

            if results.is_empty() {
                break; // No more records
            }

            for record in results {
                if let Some(station) = self.parse_reference_station(record) {
                    stations.insert(station.station_code.clone(), station);
                }
            }

            offset += limit;

            // Safety check to prevent infinite loops
            if offset > 10000 {
                warn!(
                    "Hit pagination safety limit, stopping at {} stations",
                    stations.len()
                );
                break;
            }
        }

        info!("Fetched {} reference stations", stations.len());

        // Update cache
        self.stations_cache = Some((stations.clone(), Utc::now()));

        Ok(stations)
    }

    async fn get_realtime_data(&mut self) -> Result<HashMap<String, RealTimeStatus>> {
        // Check cache
        if let Some((ref cached_data, cached_time)) = &self.realtime_cache {
            if Utc::now().signed_duration_since(*cached_time) < Duration::minutes(2) {
                // Shorter cache for real-time
                return Ok(cached_data.clone());
            }
        }

        info!("Fetching real-time data from API");

        // Fetch data with pagination (API limit is 100 records per request)
        let mut real_time_data = HashMap::new();
        let mut offset = 0;
        let limit = 100;

        loop {
            let response = self
                .client
                .get(VELIB_REALTIME_URL)
                .query(&[
                    ("limit", &limit.to_string()),
                    ("offset", &offset.to_string()),
                ])
                .send()
                .await?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "No body".to_string());
                return Err(Error::Internal(anyhow::anyhow!(
                    "API request failed with status: {}, body: {}",
                    status,
                    body
                )));
            }

            let json: Value = response.json().await?;
            let results = json
                .get("results")
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    Error::Internal(anyhow::anyhow!("Invalid API response structure"))
                })?;

            if results.is_empty() {
                break; // No more records
            }

            for record in results {
                if let Some(status) = self.parse_realtime_status(record) {
                    real_time_data.insert(status.station_code.clone(), status);
                }
            }

            offset += limit;

            // Safety check to prevent infinite loops
            if offset > 10000 {
                warn!(
                    "Hit pagination safety limit, stopping at {} real-time records",
                    real_time_data.len()
                );
                break;
            }
        }

        info!("Fetched {} real-time records", real_time_data.len());

        // Update cache
        self.realtime_cache = Some((real_time_data.clone(), Utc::now()));

        Ok(real_time_data)
    }

    fn parse_reference_station(&self, record: &Value) -> Option<StationReference> {
        let station_code = record.get("stationcode")?.as_str()?.to_string();
        let name = record.get("name")?.as_str()?.to_string();
        let capacity = record.get("capacity")?.as_u64()? as u16;

        let coordgeo = record.get("coordonnees_geo")?;
        let lat = coordgeo.get("lat")?.as_f64()?;
        let lon = coordgeo.get("lon")?.as_f64()?;

        // These fields are not in the reference endpoint
        let commune = None;
        let insee_code = None;

        Some(StationReference {
            station_code,
            name,
            coordinates: Coordinates::new(lat, lon),
            capacity,
            commune,
            insee_code,
        })
    }

    fn parse_realtime_status(&self, record: &Value) -> Option<RealTimeStatus> {
        let station_code = record.get("stationcode")?.as_str()?.to_string();

        let mechanical = record.get("mechanical")?.as_u64().unwrap_or(0) as u16;
        let electric = record.get("ebike")?.as_u64().unwrap_or(0) as u16;
        let bikes = BikeAvailability::new(mechanical, electric);

        let available_docks = record.get("numdocksavailable")?.as_u64()? as u16;

        let is_renting = record.get("is_renting")?.as_str()? == "OUI";
        let is_returning = record.get("is_returning")?.as_str()? == "OUI";
        let is_installed = record.get("is_installed")?.as_str()? == "OUI";

        let service = ServiceCapabilities {
            renting_enabled: is_renting,
            returning_enabled: is_returning,
            installed: is_installed,
        };

        let status = if is_installed && is_renting && is_returning {
            StationStatus::Operational
        } else if is_installed {
            StationStatus::Installed
        } else {
            StationStatus::OutOfService
        };

        // Parse timestamp
        let last_updated = record
            .get("duedate")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let valid_until = last_updated + Duration::minutes(5);

        Some(RealTimeStatus {
            station_code,
            bikes,
            available_docks,
            service,
            status,
            last_updated,
            valid_until,
        })
    }
}
