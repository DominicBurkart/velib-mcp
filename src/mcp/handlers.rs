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

/// Find stations near a point, filtering by distance and a custom predicate,
/// sorted by distance and truncated to `limit` results.
///
/// Each matched station is cloned into the returned `StationWithDistance`. This
/// is intentional: callers that hold the original slice (e.g. `plan_bike_journey`,
/// which needs `all_stations` for both the pickup and the dropoff pass) must not
/// have their data consumed. For `find_nearby_stations`, which previously used
/// `into_iter()`, this introduces one extra clone per matched station; given the
/// Paris Velib network of ~1,400 stations this overhead is negligible.
fn find_stations_within_radius(
    stations: &[VelibStation],
    origin: &Coordinates,
    radius_meters: u32,
    limit: usize,
    predicate: impl Fn(&VelibStation) -> bool,
) -> Vec<StationWithDistance> {
    let mut results: Vec<StationWithDistance> = stations
        .iter()
        .filter_map(|station| {
            let distance = origin.distance_to(&station.reference.coordinates) as u32;
            if distance <= radius_meters && predicate(station) {
                Some(StationWithDistance {
                    station: station.clone(),
                    straight_line_distance_meters: distance,
                })
            } else {
                None
            }
        })
        .collect();

    results.sort_by_key(|s| s.straight_line_distance_meters);
    results.truncate(limit);
    results
}

/// Validate that coordinates are within the 50km Paris service area.
/// Returns an appropriate error if they are not.
fn validate_service_area(coords: &Coordinates) -> Result<()> {
    if !coords.is_within_paris_service_area() {
        let distance_km = coords.distance_to(&PARIS_CITY_HALL) / 1000.0;
        return Err(Error::OutsideServiceArea { distance_km });
    }
    Ok(())
}

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

        // Validate coordinates are within 50km Paris service area
        validate_service_area(&query_point)?;

        // Fetch live station data
        let mut data_client = self.data_client.write().await;
        let all_stations = data_client.get_all_stations(true).await?;

        // Filter stations by distance and bike type
        let stations = find_stations_within_radius(
            &all_stations,
            &query_point,
            input.radius_meters,
            input.limit as usize,
            |station| {
                let has_requested_bikes = match &input.availability_filter {
                    Some(filter) => match &filter.bike_type {
                        Some(bike_type) => station.has_available_bikes(bike_type),
                        None => true,
                    },
                    None => true,
                };
                has_requested_bikes && station.is_operational()
            },
        );

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
        let mut data_client = self.data_client.write().await;
        let station = data_client
            .get_station_by_code(&input.station_code, true)
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

        // Fetch live station data and search by name
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

        // Sort by name for consistent results
        matching_stations.sort_by(|a, b| a.reference.name.cmp(&b.reference.name));

        // Limit results
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
        // Fetch live station data
        let mut data_client = self.data_client.write().await;
        let all_stations = data_client.get_all_stations(true).await?;

        // Filter stations within the specified bounds
        let area_stations: Vec<&VelibStation> = all_stations
            .iter()
            .filter(|station| input.bounds.contains(&station.reference.coordinates))
            .collect();

        // Calculate area statistics from live data
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

        let stats = AreaStatistics {
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
        // Validate both origin and destination are within 50km Paris service area
        validate_service_area(&input.origin)?;
        validate_service_area(&input.destination)?;

        // Find nearby stations for pickup and dropoff using live data
        let mut data_client = self.data_client.write().await;
        let all_stations = data_client.get_all_stations(true).await?;

        // Get preferences or use defaults
        let preferences = input.preferences.unwrap_or_default();

        // Find pickup stations near origin
        let pickup_stations = find_stations_within_radius(
            &all_stations,
            &input.origin,
            preferences.max_walk_distance,
            3,
            |station| {
                station.is_operational() && station.has_available_bikes(&preferences.bike_type)
            },
        );

        // Find dropoff stations near destination
        let dropoff_stations = find_stations_within_radius(
            &all_stations,
            &input.destination,
            preferences.max_walk_distance,
            3,
            |station| station.is_operational() && station.has_available_docks(1),
        );

        // Generate journey recommendations
        let mut recommendations = Vec::new();

        if !pickup_stations.is_empty() && !dropoff_stations.is_empty() {
            // Create recommendations by pairing closest pickup with closest dropoff
            let best_pickup = &pickup_stations[0];
            let best_dropoff = &dropoff_stations[0];

            // Calculate confidence score based on walking distances
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
                pickup_stations,
                dropoff_stations,
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
        // Simple connectivity test by fetching reference data
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        BikeAvailability, DataFreshness, RealTimeStatus, ServiceCapabilities, StationReference,
        StationStatus,
    };
    use chrono::Utc;

    /// Build a minimal operational station with the given code, coordinates, and bike counts.
    fn make_station(
        code: &str,
        lat: f64,
        lon: f64,
        mechanical: u16,
        electric: u16,
    ) -> VelibStation {
        VelibStation {
            reference: StationReference {
                station_code: code.to_string(),
                name: format!("Station {code}"),
                coordinates: Coordinates::new(lat, lon),
                capacity: 20,
                capabilities: ServiceCapabilities::default(),
            },
            real_time: Some(RealTimeStatus {
                bikes: BikeAvailability::new(mechanical, electric),
                available_docks: 20 - mechanical - electric,
                status: StationStatus::Open,
                last_update: Utc::now(),
                data_freshness: DataFreshness::Fresh,
            }),
        }
    }

    /// Paris City Hall as a convenient well-known origin.
    fn paris_city_hall() -> Coordinates {
        Coordinates::new(48.8565, 2.3514)
    }

    // --- filtering by radius ---

    #[test]
    fn test_radius_filtering_excludes_distant_stations() {
        let origin = paris_city_hall();
        // ~111 m per 0.001° latitude; place one station ~200 m away and one ~2 km away.
        let near = make_station("near", 48.8565, 2.3514, 2, 0); // same point
        let far = make_station("far", 48.875, 2.3514, 2, 0); // ~2 km north
        let stations = vec![near, far];

        let results = find_stations_within_radius(&stations, &origin, 500, 10, |_| true);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].station.reference.station_code, "near");
    }

    #[test]
    fn test_empty_result_when_all_stations_out_of_radius() {
        let origin = paris_city_hall();
        let far1 = make_station("far1", 48.875, 2.3514, 2, 0);
        let far2 = make_station("far2", 48.880, 2.3514, 2, 0);
        let stations = vec![far1, far2];

        let results = find_stations_within_radius(&stations, &origin, 100, 10, |_| true);

        assert!(results.is_empty());
    }

    // --- sort order ---

    #[test]
    fn test_results_sorted_by_distance_ascending() {
        let origin = paris_city_hall();
        // Build stations in reverse distance order so we can confirm sorting flips them.
        let closest = make_station("closest", 48.8566, 2.3514, 1, 0); // ~11 m
        let middle = make_station("middle", 48.8575, 2.3514, 1, 0); // ~110 m
        let farthest = make_station("farthest", 48.8585, 2.3514, 1, 0); // ~220 m
        let stations = vec![farthest.clone(), middle.clone(), closest.clone()];

        let results = find_stations_within_radius(&stations, &origin, 500, 10, |_| true);

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].station.reference.station_code, "closest");
        assert_eq!(results[1].station.reference.station_code, "middle");
        assert_eq!(results[2].station.reference.station_code, "farthest");
        // Distances must be non-decreasing.
        assert!(
            results[0].straight_line_distance_meters <= results[1].straight_line_distance_meters
        );
        assert!(
            results[1].straight_line_distance_meters <= results[2].straight_line_distance_meters
        );
    }

    // --- truncation ---

    #[test]
    fn test_truncation_to_limit() {
        let origin = paris_city_hall();
        let stations: Vec<VelibStation> = (0..10)
            .map(|i| {
                // Space them out within radius but at different distances.
                make_station(
                    &format!("s{i}"),
                    48.8565 + f64::from(i) * 0.0001,
                    2.3514,
                    1,
                    0,
                )
            })
            .collect();

        let results = find_stations_within_radius(&stations, &origin, 5000, 3, |_| true);

        assert_eq!(results.len(), 3);
        // The 3 returned must be the 3 closest (limit applied after sort).
        assert!(
            results[0].straight_line_distance_meters <= results[1].straight_line_distance_meters
        );
        assert!(
            results[1].straight_line_distance_meters <= results[2].straight_line_distance_meters
        );
    }

    #[test]
    fn test_limit_larger_than_matches_returns_all_matches() {
        let origin = paris_city_hall();
        let stations = vec![
            make_station("a", 48.8565, 2.3514, 1, 0),
            make_station("b", 48.8566, 2.3514, 1, 0),
        ];

        let results = find_stations_within_radius(&stations, &origin, 500, 100, |_| true);

        assert_eq!(results.len(), 2);
    }

    // --- predicate ---

    #[test]
    fn test_predicate_filters_stations() {
        let origin = paris_city_hall();
        // One station has mechanical bikes, one has only electric.
        let mech = make_station("mech", 48.8565, 2.3514, 3, 0);
        let elec = make_station("elec", 48.8566, 2.3514, 0, 3);
        let stations = vec![mech, elec];

        let results = find_stations_within_radius(&stations, &origin, 500, 10, |s| {
            s.has_available_bikes(&BikeTypeFilter::MechanicalOnly)
        });

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].station.reference.station_code, "mech");
    }

    #[test]
    fn test_predicate_rejects_all_returns_empty() {
        let origin = paris_city_hall();
        let stations = vec![
            make_station("a", 48.8565, 2.3514, 1, 0),
            make_station("b", 48.8566, 2.3514, 1, 0),
        ];

        let results = find_stations_within_radius(&stations, &origin, 500, 10, |_| false);

        assert!(results.is_empty());
    }

    // --- combined: predicate + radius + truncation ---

    #[test]
    fn test_combined_radius_predicate_and_limit() {
        let origin = paris_city_hall();
        // 5 stations in radius with mechanical bikes, 2 outside radius, 1 in radius without bikes.
        let mut stations: Vec<VelibStation> = (0..5)
            .map(|i| {
                make_station(
                    &format!("m{i}"),
                    48.8565 + f64::from(i) * 0.0001,
                    2.3514,
                    2,
                    0,
                )
            })
            .collect();
        stations.push(make_station("far", 48.875, 2.3514, 2, 0)); // outside radius
        stations.push(make_station("nobike", 48.8566, 2.3520, 0, 0)); // in radius, no bikes

        let results = find_stations_within_radius(&stations, &origin, 500, 3, |s| {
            s.has_available_bikes(&BikeTypeFilter::MechanicalOnly)
        });

        assert_eq!(results.len(), 3);
        // All returned stations must be within radius.
        for r in &results {
            assert!(r.straight_line_distance_meters <= 500);
        }
        // Must be sorted by distance.
        assert!(
            results[0].straight_line_distance_meters <= results[1].straight_line_distance_meters
        );
        assert!(
            results[1].straight_line_distance_meters <= results[2].straight_line_distance_meters
        );
    }
}
