use unicode_normalization::UnicodeNormalization;

use crate::data::VelibDataClient;
use crate::mcp::types::{
    AreaStatistics, AvailableBikesStats, BikeJourney, FindNearbyStationsInput,
    FindNearbyStationsOutput, GetAreaStatisticsInput, GetAreaStatisticsOutput,
    GetStationByCodeInput, GetStationByCodeOutput, JourneyPreferences, JourneyRecommendation,
    PlanBikeJourneyInput, PlanBikeJourneyOutput, SearchMetadata, SearchStationsByNameInput,
    SearchStationsByNameOutput, StationWithDistance, TextSearchMetadata,
};
use crate::types::{BikeTypeFilter, Coordinates, VelibStation};
use crate::{Error, Result};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

const MAX_SEARCH_RADIUS: u32 = 5000; // 5km
const MAX_RESULT_LIMIT: u16 = 100;

// Paris City Hall coordinates - reference point for service area validation
const PARIS_CITY_HALL: Coordinates = Coordinates {
    latitude: 48.8565,
    longitude: 2.3514,
};

pub struct McpToolHandler {
    data_client: Arc<RwLock<VelibDataClient>>,
}

impl Default for McpToolHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl McpToolHandler {
    #[must_use]
    pub fn new() -> Self {
        Self {
            data_client: Arc::new(RwLock::new(VelibDataClient::new())),
        }
    }

    #[must_use]
    pub fn with_data_client(data_client: VelibDataClient) -> Self {
        Self {
            data_client: Arc::new(RwLock::new(data_client)),
        }
    }

    /// Validate that coordinates are within the Paris metro bounding box and the
    /// 50 km service radius around Paris City Hall.  Returns an appropriate error
    /// variant on failure so callers can return it directly.
    fn validate_paris_coordinates(coords: &Coordinates) -> Result<()> {
        if !coords.is_valid_paris_metro() {
            return Err(Error::InvalidCoordinates {
                latitude: coords.latitude,
                longitude: coords.longitude,
            });
        }
        if !coords.is_within_paris_service_area() {
            let distance_km = coords.distance_to(&PARIS_CITY_HALL) / 1000.0;
            return Err(Error::OutsideServiceArea { distance_km });
        }
        Ok(())
    }

    pub async fn find_nearby_stations(
        &self,
        input: FindNearbyStationsInput,
    ) -> Result<FindNearbyStationsOutput> {
        let start_time = Instant::now();

        if input.radius_meters > MAX_SEARCH_RADIUS {
            return Err(Error::SearchRadiusTooLarge {
                radius: input.radius_meters,
                max: MAX_SEARCH_RADIUS,
            });
        }

        if input.limit > MAX_RESULT_LIMIT {
            return Err(Error::ResultLimitExceeded {
                limit: input.limit,
                max: MAX_RESULT_LIMIT,
            });
        }

        let query_point = Coordinates::new(input.latitude, input.longitude);
        Self::validate_paris_coordinates(&query_point)?;

        let mut data_client = self.data_client.write().await;
        let all_stations = data_client.get_all_stations(true).await?;

        let mut nearby_stations: Vec<StationWithDistance> = all_stations
            .into_iter()
            .filter_map(|station| {
                let distance = query_point.distance_to(&station.reference.coordinates) as u32;

                if distance > input.radius_meters {
                    return None;
                }

                let has_requested_bikes = match &input.availability_filter {
                    Some(filter) => match &filter.bike_type {
                        Some(bike_type) => station.has_available_bikes(bike_type),
                        None => true,
                    },
                    None => true,
                };

                if has_requested_bikes && station.is_operational() {
                    Some(StationWithDistance {
                        station,
                        straight_line_distance_meters: distance,
                    })
                } else {
                    None
                }
            })
            .collect();

        nearby_stations.sort_by_key(|s| s.straight_line_distance_meters);
        nearby_stations.truncate(input.limit as usize);

        let search_time = start_time.elapsed().as_millis() as u64;

        Ok(FindNearbyStationsOutput {
            search_metadata: SearchMetadata {
                query_point,
                radius_meters: input.radius_meters,
                total_found: nearby_stations.len() as u32,
                search_time_ms: search_time,
            },
            stations: nearby_stations,
        })
    }

    pub async fn get_station_by_code(
        &self,
        input: GetStationByCodeInput,
    ) -> Result<GetStationByCodeOutput> {
        let mut data_client = self.data_client.write().await;
        let station = data_client
            .get_station_by_code(&input.station_code, input.include_real_time)
            .await?;

        Ok(GetStationByCodeOutput {
            found: station.is_some(),
            station,
        })
    }

    pub async fn search_stations_by_name(
        &self,
        input: SearchStationsByNameInput,
    ) -> Result<SearchStationsByNameOutput> {
        let start_time = Instant::now();

        if input.query.len() < 2 {
            return Err(Error::Internal(anyhow::anyhow!("Search query too short")));
        }

        if input.limit > MAX_RESULT_LIMIT {
            return Err(Error::ResultLimitExceeded {
                limit: input.limit,
                max: MAX_RESULT_LIMIT,
            });
        }

        let mut data_client = self.data_client.write().await;
        let all_stations = data_client.get_all_stations(true).await?;

        let query_normalized = input.query.to_lowercase().nfc().collect::<String>();
        let mut matching_stations: Vec<VelibStation> = all_stations
            .into_iter()
            .filter(|station| {
                let name_normalized = station
                    .reference
                    .name
                    .to_lowercase()
                    .nfc()
                    .collect::<String>();
                if input.fuzzy {
                    name_normalized.contains(&query_normalized)
                } else {
                    name_normalized.starts_with(&query_normalized)
                }
            })
            .collect();

        matching_stations.sort_by(|a, b| a.reference.name.cmp(&b.reference.name));
        matching_stations.truncate(input.limit as usize);

        let search_time = start_time.elapsed().as_millis() as u64;

        Ok(SearchStationsByNameOutput {
            search_metadata: TextSearchMetadata {
                query: input.query,
                total_found: matching_stations.len() as u32,
                fuzzy_enabled: input.fuzzy,
                search_time_ms: search_time,
            },
            stations: matching_stations,
        })
    }

    pub async fn get_area_statistics(
        &self,
        input: GetAreaStatisticsInput,
    ) -> Result<GetAreaStatisticsOutput> {
        let mut data_client = self.data_client.write().await;
        let all_stations = data_client.get_all_stations(true).await?;

        let area_stations: Vec<&VelibStation> = all_stations
            .iter()
            .filter(|station| input.bounds.contains(&station.reference.coordinates))
            .collect();

        let total_stations = area_stations.len() as u32;
        let operational_stations = area_stations
            .iter()
            .filter(|station| station.is_operational())
            .count() as u32;

        let mut total_capacity = 0u32;
        let mut total_mechanical = 0u32;
        let mut total_electric = 0u32;
        let mut total_available_docks = 0u32;

        for station in &area_stations {
            total_capacity += u32::from(station.reference.capacity);

            if let Some(rt) = &station.real_time {
                total_mechanical += u32::from(rt.bikes.mechanical);
                total_electric += u32::from(rt.bikes.electric);
                total_available_docks += u32::from(rt.available_docks);
            }
        }

        let total_bikes = total_mechanical + total_electric;
        let occupancy_rate = if total_capacity > 0 {
            f64::from(total_bikes) / f64::from(total_capacity)
        } else {
            0.0
        };

        Ok(GetAreaStatisticsOutput {
            area_stats: AreaStatistics {
                total_stations,
                operational_stations,
                total_capacity,
                available_bikes: AvailableBikesStats {
                    mechanical: total_mechanical,
                    electric: total_electric,
                    total: total_bikes,
                },
                available_docks: total_available_docks,
                occupancy_rate,
            },
            bounds: input.bounds,
        })
    }

    pub async fn plan_bike_journey(
        &self,
        input: PlanBikeJourneyInput,
    ) -> Result<PlanBikeJourneyOutput> {
        Self::validate_paris_coordinates(&input.origin)?;
        Self::validate_paris_coordinates(&input.destination)?;

        let mut data_client = self.data_client.write().await;
        let all_stations = data_client.get_all_stations(true).await?;

        let preferences = input.preferences.unwrap_or_default();

        let mut pickup_candidates: Vec<StationWithDistance> = all_stations
            .iter()
            .filter_map(|station| {
                let distance = input.origin.distance_to(&station.reference.coordinates) as u32;

                if distance <= preferences.max_walk_distance
                    && station.is_operational()
                    && station.has_available_bikes(&preferences.bike_type)
                {
                    Some(StationWithDistance {
                        station: station.clone(),
                        straight_line_distance_meters: distance,
                    })
                } else {
                    None
                }
            })
            .collect();

        pickup_candidates.sort_by_key(|s| s.straight_line_distance_meters);
        pickup_candidates.truncate(3);

        let mut dropoff_candidates: Vec<StationWithDistance> = all_stations
            .iter()
            .filter_map(|station| {
                let distance = input
                    .destination
                    .distance_to(&station.reference.coordinates)
                    as u32;

                if distance <= preferences.max_walk_distance
                    && station.is_operational()
                    && station.has_available_docks(1)
                {
                    Some(StationWithDistance {
                        station: station.clone(),
                        straight_line_distance_meters: distance,
                    })
                } else {
                    None
                }
            })
            .collect();

        dropoff_candidates.sort_by_key(|s| s.straight_line_distance_meters);
        dropoff_candidates.truncate(3);

        let mut recommendations = Vec::new();

        if !pickup_candidates.is_empty() && !dropoff_candidates.is_empty() {
            let best_pickup = &pickup_candidates[0];
            let best_dropoff = &dropoff_candidates[0];

            let max_walk = f64::from(preferences.max_walk_distance);
            let pickup_walk_ratio = f64::from(best_pickup.straight_line_distance_meters) / max_walk;
            let dropoff_walk_ratio =
                f64::from(best_dropoff.straight_line_distance_meters) / max_walk;
            let confidence_score = 1.0 - f64::midpoint(pickup_walk_ratio, dropoff_walk_ratio) * 0.5;

            recommendations.push(JourneyRecommendation {
                pickup_station: best_pickup.station.clone(),
                dropoff_station: best_dropoff.station.clone(),
                straight_line_to_pickup_meters: best_pickup.straight_line_distance_meters,
                straight_line_from_dropoff_meters: best_dropoff.straight_line_distance_meters,
                confidence_score: confidence_score.clamp(0.1, 1.0),
            });
        }

        Ok(PlanBikeJourneyOutput {
            journey: BikeJourney {
                pickup_stations: pickup_candidates,
                dropoff_stations: dropoff_candidates,
                recommendations,
            },
        })
    }

    /// Clean up expired cache entries in the data client
    pub async fn cleanup_cache(&self) {
        let data_client = self.data_client.read().await;
        data_client.cleanup_cache().await;
    }

    /// Get cache statistics from the data client
    pub async fn cache_stats(&self) -> (usize, usize) {
        let data_client = self.data_client.read().await;
        data_client.cache_stats().await
    }

    /// Get reference stations for resource endpoints
    pub async fn get_reference_stations(&self) -> Result<Vec<crate::types::StationReference>> {
        let mut data_client = self.data_client.write().await;
        data_client.fetch_reference_stations().await
    }

    /// Get real-time status for resource endpoints
    pub async fn get_realtime_status(
        &self,
    ) -> Result<std::collections::HashMap<String, crate::types::RealTimeStatus>> {
        let mut data_client = self.data_client.write().await;
        data_client.fetch_realtime_status().await
    }

    /// Get complete stations data for resource endpoints
    pub async fn get_complete_stations(
        &self,
        include_realtime: bool,
    ) -> Result<Vec<crate::types::VelibStation>> {
        let mut data_client = self.data_client.write().await;
        data_client.get_all_stations(include_realtime).await
    }

    /// Test connectivity to data sources for health checks
    pub async fn test_connectivity(&self) -> Result<()> {
        let mut data_client = self.data_client.write().await;
        data_client.get_all_stations(false).await?;
        Ok(())
    }
}

impl Default for JourneyPreferences {
    fn default() -> Self {
        Self {
            bike_type: BikeTypeFilter::AnyType,
            max_walk_distance: 500,
        }
    }
}
