use crate::data::VelibDataClient;
use crate::mcp::types::{
    AreaStatistics, AvailableBikesStats, BikeJourney, FindNearbyStationsInput,
    FindNearbyStationsOutput, GetAreaStatisticsInput, GetAreaStatisticsOutput,
    GetStationByCodeInput, GetStationByCodeOutput, JourneyRecommendation, PlanBikeJourneyInput,
    PlanBikeJourneyOutput, SearchMetadata, SearchStationsByNameInput, SearchStationsByNameOutput,
    StationWithDistance, TextSearchMetadata,
};
use crate::types::{BikeTypeFilter, Coordinates, VelibStation};
use crate::{Error, Result};
use std::time::Instant;
use tokio::sync::Mutex;

const MAX_SEARCH_RADIUS: u32 = 5000; // 5km
const MAX_RESULT_LIMIT: u16 = 100;

pub struct McpToolHandler {
    data_client: Mutex<VelibDataClient>,
}

impl Default for McpToolHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl McpToolHandler {
    pub fn new() -> Self {
        Self {
            data_client: Mutex::new(VelibDataClient::new()),
        }
    }

    pub async fn find_nearby_stations(
        &self,
        input: FindNearbyStationsInput,
    ) -> Result<FindNearbyStationsOutput> {
        let start_time = Instant::now();

        // Validate input parameters
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
        if !query_point.is_valid_paris_metro() {
            return Err(Error::InvalidCoordinates {
                latitude: input.latitude,
                longitude: input.longitude,
            });
        }

        // Get all stations with real-time data
        let mut data_client = self.data_client.lock().await;
        let all_stations = data_client.get_all_stations(true).await?;

        // Filter stations within radius
        let stations: Vec<VelibStation> = all_stations
            .into_iter()
            .filter(|station| {
                let distance = query_point.distance_to(&station.reference.coordinates);
                distance <= input.radius_meters as f64
            })
            .collect();

        // Apply availability filters if provided
        let filtered_stations = if let Some(filter) = &input.availability_filter {
            self.apply_availability_filter(stations, filter)
        } else {
            stations
        };

        // Calculate distances and sort by distance
        let mut stations_with_distance: Vec<StationWithDistance> = filtered_stations
            .into_iter()
            .map(|station| {
                let distance = query_point.distance_to(&station.reference.coordinates) as u32;
                StationWithDistance {
                    station,
                    distance_meters: distance,
                }
            })
            .collect();

        stations_with_distance.sort_by_key(|s| s.distance_meters);
        stations_with_distance.truncate(input.limit as usize);

        let search_time = start_time.elapsed().as_millis() as u64;

        Ok(FindNearbyStationsOutput {
            search_metadata: SearchMetadata {
                query_point,
                radius_meters: input.radius_meters,
                total_found: stations_with_distance.len() as u32,
                search_time_ms: search_time,
            },
            stations: stations_with_distance,
        })
    }

    pub async fn get_station_by_code(
        &self,
        input: GetStationByCodeInput,
    ) -> Result<GetStationByCodeOutput> {
        let mut data_client = self.data_client.lock().await;
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

        // Get all stations and perform text search
        let mut data_client = self.data_client.lock().await;
        let all_stations = data_client.get_all_stations(true).await?;

        let query_lower = input.query.to_lowercase();
        let mut matching_stations: Vec<VelibStation> = all_stations
            .into_iter()
            .filter(|station| {
                let name_lower = station.reference.name.to_lowercase();
                if input.fuzzy {
                    // Fuzzy search: contains the query
                    name_lower.contains(&query_lower)
                } else {
                    // Exact search: starts with or equals the query
                    name_lower.starts_with(&query_lower) || name_lower == query_lower
                }
            })
            .collect();

        // Sort by relevance (exact matches first, then by name)
        matching_stations.sort_by(|a, b| {
            let a_name = a.reference.name.to_lowercase();
            let b_name = b.reference.name.to_lowercase();

            let a_exact = a_name.starts_with(&query_lower);
            let b_exact = b_name.starts_with(&query_lower);

            match (a_exact, b_exact) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a_name.cmp(&b_name),
            }
        });

        matching_stations.truncate(input.limit as usize);
        let stations = matching_stations;

        let search_time = start_time.elapsed().as_millis() as u64;

        Ok(SearchStationsByNameOutput {
            search_metadata: TextSearchMetadata {
                query: input.query,
                total_found: stations.len() as u32,
                fuzzy_enabled: input.fuzzy,
                search_time_ms: search_time,
            },
            stations,
        })
    }

    pub async fn get_area_statistics(
        &self,
        input: GetAreaStatisticsInput,
    ) -> Result<GetAreaStatisticsOutput> {
        // Get all stations and filter by geographic bounds
        let mut data_client = self.data_client.lock().await;
        let all_stations = data_client
            .get_all_stations(input.include_real_time)
            .await?;

        let stations: Vec<VelibStation> = all_stations
            .into_iter()
            .filter(|station| input.bounds.contains(&station.reference.coordinates))
            .collect();

        let mut stats = AreaStatistics {
            total_stations: stations.len() as u32,
            operational_stations: 0,
            total_capacity: 0,
            available_bikes: AvailableBikesStats {
                mechanical: 0,
                electric: 0,
                total: 0,
            },
            available_docks: 0,
            occupancy_rate: 0.0,
        };

        for station in &stations {
            stats.total_capacity += station.reference.capacity as u32;

            if let Some(rt) = &station.real_time {
                if station.is_operational() {
                    stats.operational_stations += 1;
                }

                stats.available_bikes.mechanical += rt.bikes.mechanical as u32;
                stats.available_bikes.electric += rt.bikes.electric as u32;
                stats.available_docks += rt.available_docks as u32;
            }
        }

        stats.available_bikes.total =
            stats.available_bikes.mechanical + stats.available_bikes.electric;

        if stats.total_capacity > 0 {
            let occupied = stats.available_bikes.total;
            stats.occupancy_rate = occupied as f64 / stats.total_capacity as f64;
        }

        Ok(GetAreaStatisticsOutput {
            area_stats: stats,
            bounds: input.bounds,
        })
    }

    pub async fn plan_bike_journey(
        &self,
        input: PlanBikeJourneyInput,
    ) -> Result<PlanBikeJourneyOutput> {
        if !input.origin.is_valid_paris_metro() {
            return Err(Error::InvalidCoordinates {
                latitude: input.origin.latitude,
                longitude: input.origin.longitude,
            });
        }

        if !input.destination.is_valid_paris_metro() {
            return Err(Error::InvalidCoordinates {
                latitude: input.destination.latitude,
                longitude: input.destination.longitude,
            });
        }

        let preferences = input.preferences.unwrap_or_default();
        let max_walk = preferences.max_walk_distance;

        // Get all stations and find those near origin
        let mut data_client = self.data_client.lock().await;
        let all_stations = data_client.get_all_stations(true).await?;

        let pickup_stations: Vec<VelibStation> = all_stations
            .iter()
            .filter(|station| {
                let distance = input.origin.distance_to(&station.reference.coordinates);
                distance <= max_walk as f64
            })
            .cloned()
            .collect();
        let pickup_with_distance: Vec<StationWithDistance> = pickup_stations
            .into_iter()
            .filter(|s| s.has_available_bikes(&preferences.bike_type))
            .map(|station| {
                let distance = input.origin.distance_to(&station.reference.coordinates) as u32;
                StationWithDistance {
                    station,
                    distance_meters: distance,
                }
            })
            .collect();

        // Find dropoff stations near destination (reuse the same data)
        let dropoff_stations: Vec<VelibStation> = all_stations
            .into_iter()
            .filter(|station| {
                let distance = input
                    .destination
                    .distance_to(&station.reference.coordinates);
                distance <= max_walk as f64
            })
            .collect();
        let dropoff_with_distance: Vec<StationWithDistance> = dropoff_stations
            .into_iter()
            .filter(|s| s.has_available_docks(1))
            .map(|station| {
                let distance = input
                    .destination
                    .distance_to(&station.reference.coordinates)
                    as u32;
                StationWithDistance {
                    station,
                    distance_meters: distance,
                }
            })
            .collect();

        // Generate recommendations
        let mut recommendations = Vec::new();
        for pickup in pickup_with_distance.iter().take(3) {
            for dropoff in dropoff_with_distance.iter().take(3) {
                if pickup.station.reference.station_code != dropoff.station.reference.station_code {
                    let confidence = self.calculate_journey_confidence(pickup, dropoff);
                    recommendations.push(JourneyRecommendation {
                        pickup_station: pickup.station.clone(),
                        dropoff_station: dropoff.station.clone(),
                        walk_to_pickup: pickup.distance_meters,
                        walk_from_dropoff: dropoff.distance_meters,
                        confidence_score: confidence,
                    });
                }
            }
        }

        recommendations
            .sort_by(|a, b| b.confidence_score.partial_cmp(&a.confidence_score).unwrap());
        recommendations.truncate(5);

        Ok(PlanBikeJourneyOutput {
            journey: BikeJourney {
                pickup_stations: pickup_with_distance,
                dropoff_stations: dropoff_with_distance,
                recommendations,
            },
        })
    }

    fn apply_availability_filter(
        &self,
        stations: Vec<VelibStation>,
        filter: &crate::mcp::types::AvailabilityFilter,
    ) -> Vec<VelibStation> {
        stations
            .into_iter()
            .filter(|station| {
                if filter.exclude_out_of_service && !station.is_operational() {
                    return false;
                }

                if let Some(min_bikes) = filter.min_bikes {
                    if let Some(rt) = &station.real_time {
                        if rt.bikes.total() < min_bikes {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                if let Some(min_docks) = filter.min_docks {
                    if !station.has_available_docks(min_docks) {
                        return false;
                    }
                }

                if let Some(bike_type) = &filter.bike_type {
                    if !station.has_available_bikes(bike_type) {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    fn calculate_journey_confidence(
        &self,
        pickup: &StationWithDistance,
        dropoff: &StationWithDistance,
    ) -> f64 {
        let mut confidence = 1.0;

        // Penalize longer walks
        confidence -= (pickup.distance_meters as f64 / 1000.0) * 0.2;
        confidence -= (dropoff.distance_meters as f64 / 1000.0) * 0.2;

        // Bonus for more available bikes/docks
        if let Some(pickup_rt) = &pickup.station.real_time {
            confidence += (pickup_rt.bikes.total() as f64 / 10.0).min(0.2);
        }

        if let Some(dropoff_rt) = &dropoff.station.real_time {
            confidence += (dropoff_rt.available_docks as f64 / 10.0).min(0.2);
        }

        confidence.max(0.0).min(1.0)
    }
}

impl Default for crate::mcp::types::JourneyPreferences {
    fn default() -> Self {
        Self {
            bike_type: BikeTypeFilter::AnyType,
            max_walk_distance: 500,
        }
    }
}
