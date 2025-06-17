use crate::mcp::types::*;
use crate::types::{
    BikeAvailability, BikeTypeFilter, Coordinates, DataFreshness, RealTimeStatus,
    ServiceCapabilities, StationReference, StationStatus, VelibStation,
};
use crate::{Error, Result};
use chrono::Utc;
use std::time::Instant;

const MAX_SEARCH_RADIUS: u32 = 5000; // 5km
const MAX_RESULT_LIMIT: u16 = 100;

pub struct McpToolHandler {
    // In Phase 2B, we use placeholder data
    // In Phase 3, this will be replaced with a real data client
}

impl Default for McpToolHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl McpToolHandler {
    pub fn new() -> Self {
        Self {}
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

        // Generate placeholder stations for demonstration
        let stations = self.generate_placeholder_stations(&query_point, input.limit as usize);

        let search_time = start_time.elapsed().as_millis() as u64;

        Ok(FindNearbyStationsOutput {
            search_metadata: SearchMetadata {
                query_point,
                radius_meters: input.radius_meters,
                total_found: stations.len() as u32,
                search_time_ms: search_time,
            },
            stations,
        })
    }

    pub async fn get_station_by_code(
        &self,
        input: GetStationByCodeInput,
    ) -> Result<GetStationByCodeOutput> {
        // Placeholder implementation - returns a demo station for valid codes
        let station = if input.station_code.starts_with("demo") {
            Some(self.create_demo_station(&input.station_code))
        } else {
            None
        };

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

        // Generate placeholder search results
        let stations = self.generate_search_results(&input.query, input.limit as usize);
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
        // Generate placeholder area statistics
        let stats = AreaStatistics {
            total_stations: 25,
            operational_stations: 23,
            total_capacity: 500,
            available_bikes: AvailableBikesStats {
                mechanical: 120,
                electric: 85,
                total: 205,
            },
            available_docks: 295,
            occupancy_rate: 0.41, // 205/500
        };

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

        // Generate placeholder journey recommendations
        let pickup_stations = self.generate_placeholder_stations(&input.origin, 3);
        let dropoff_stations = self.generate_placeholder_stations(&input.destination, 3);

        let recommendations = vec![JourneyRecommendation {
            pickup_station: pickup_stations[0].station.clone(),
            dropoff_station: dropoff_stations[0].station.clone(),
            walk_to_pickup: pickup_stations[0].distance_meters,
            walk_from_dropoff: dropoff_stations[0].distance_meters,
            confidence_score: 0.85,
        }];

        Ok(PlanBikeJourneyOutput {
            journey: BikeJourney {
                pickup_stations,
                dropoff_stations,
                recommendations,
            },
        })
    }

    // Helper methods for generating placeholder data
    fn generate_placeholder_stations(
        &self,
        center: &Coordinates,
        count: usize,
    ) -> Vec<StationWithDistance> {
        (0..count)
            .map(|i| {
                let offset_lat = (i as f64) * 0.001;
                let offset_lon = (i as f64) * 0.001;

                let station_coords =
                    Coordinates::new(center.latitude + offset_lat, center.longitude + offset_lon);

                let distance = center.distance_to(&station_coords) as u32;

                StationWithDistance {
                    station: VelibStation {
                        reference: StationReference {
                            station_code: format!("demo_{:03}", i + 1),
                            name: format!("Demo Station {}", i + 1),
                            coordinates: station_coords,
                            capacity: 20,
                            capabilities: ServiceCapabilities::default(),
                        },
                        real_time: Some(RealTimeStatus {
                            bikes: BikeAvailability::new(((i + 1) * 2) as u16, (i + 1) as u16),
                            available_docks: (20 - (i + 1) * 3) as u16,
                            status: StationStatus::Open,
                            last_update: Utc::now(),
                            data_freshness: DataFreshness::Fresh,
                        }),
                    },
                    distance_meters: distance,
                }
            })
            .collect()
    }

    fn create_demo_station(&self, code: &str) -> VelibStation {
        VelibStation {
            reference: StationReference {
                station_code: code.to_string(),
                name: format!("Demo Station for {}", code),
                coordinates: Coordinates::new(48.8566, 2.3522), // Paris center
                capacity: 25,
                capabilities: ServiceCapabilities::default(),
            },
            real_time: Some(RealTimeStatus {
                bikes: BikeAvailability::new(8, 5),
                available_docks: 12,
                status: StationStatus::Open,
                last_update: Utc::now(),
                data_freshness: DataFreshness::Fresh,
            }),
        }
    }

    fn generate_search_results(&self, query: &str, limit: usize) -> Vec<VelibStation> {
        // Generate a few demo stations that "match" the query
        let demo_names = vec![
            format!("{} Metro Station", query),
            format!("{} Place", query),
            format!("Station {}", query),
        ];

        demo_names
            .into_iter()
            .take(limit)
            .enumerate()
            .map(|(i, name)| VelibStation {
                reference: StationReference {
                    station_code: format!("search_{:03}", i + 1),
                    name,
                    coordinates: Coordinates::new(
                        48.8566 + (i as f64) * 0.001,
                        2.3522 + (i as f64) * 0.001,
                    ),
                    capacity: 20,
                    capabilities: ServiceCapabilities::default(),
                },
                real_time: Some(RealTimeStatus {
                    bikes: BikeAvailability::new(5, 3),
                    available_docks: 12,
                    status: StationStatus::Open,
                    last_update: Utc::now(),
                    data_freshness: DataFreshness::Fresh,
                }),
            })
            .collect()
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
