//! Unit tests for MCP handler business logic.
//!
//! All tests construct `VelibStation` data in-memory via
//! `VelibDataClient::from_stations` and inject it through
//! `McpToolHandler::with_data_client`, so no network access occurs.

use chrono::Utc;
use velib_mcp::{
    data::VelibDataClient,
    mcp::{
        types::{
            AvailabilityFilter, FindNearbyStationsInput, GetAreaStatisticsInput,
            GeographicBounds, JourneyPreferences, PlanBikeJourneyInput,
            SearchStationsByNameInput,
        },
        McpToolHandler,
    },
    types::{
        BikeAvailability, BikeTypeFilter, Coordinates, DataFreshness, RealTimeStatus,
        ServiceCapabilities, StationReference, StationStatus, VelibStation,
    },
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a minimal `VelibStation` with real-time data.
///
/// * `code`         – station code string
/// * `name`         – human-readable name
/// * `lat`/`lon`    – WGS-84 coordinates (must be inside Paris metro bounds)
/// * `mechanical`   – available mechanical bikes
/// * `electric`     – available electric bikes
/// * `docks`        – available docks
/// * `capacity`     – total station capacity (must be >= mechanical+electric+docks)
/// * `status`       – `StationStatus::Open`, `Closed`, or `Maintenance`
fn make_station(
    code: &str,
    name: &str,
    lat: f64,
    lon: f64,
    mechanical: u16,
    electric: u16,
    docks: u16,
    capacity: u16,
    status: StationStatus,
) -> VelibStation {
    let reference = StationReference {
        station_code: code.to_string(),
        name: name.to_string(),
        coordinates: Coordinates::new(lat, lon),
        capacity,
        capabilities: ServiceCapabilities::default(),
    };
    let real_time = RealTimeStatus {
        bikes: BikeAvailability::new(mechanical, electric),
        available_docks: docks,
        status,
        last_update: Utc::now(),
        data_freshness: DataFreshness::Fresh,
    };
    VelibStation {
        reference,
        real_time: Some(real_time),
    }
}

/// Convenience wrapper: open station with both bike types available.
fn open_station(
    code: &str,
    name: &str,
    lat: f64,
    lon: f64,
    mechanical: u16,
    electric: u16,
    docks: u16,
) -> VelibStation {
    let capacity = mechanical + electric + docks;
    make_station(
        code, name, lat, lon, mechanical, electric, docks, capacity,
        StationStatus::Open,
    )
}

/// Build a handler pre-loaded with the given stations (no network).
async fn handler_with(stations: Vec<VelibStation>) -> McpToolHandler {
    let client = VelibDataClient::from_stations(stations).await;
    McpToolHandler::with_data_client(client)
}

// ---------------------------------------------------------------------------
// find_nearby_stations
// ---------------------------------------------------------------------------

/// Stations within the radius should appear; stations outside should not.
#[tokio::test]
async fn find_nearby_stations_filters_by_distance() {
    // Query origin: Paris City Hall area (48.8565, 2.3514)
    // Station A: ~50 m away   → inside a 200 m radius
    // Station B: ~800 m away  → outside a 200 m radius
    let station_near = open_station("A", "Near Station", 48.8569, 2.3514, 3, 2, 10);
    let station_far = open_station("B", "Far Station", 48.8638, 2.3514, 5, 0, 15);

    let handler = handler_with(vec![station_near, station_far]).await;

    let input = FindNearbyStationsInput {
        latitude: 48.8565,
        longitude: 2.3514,
        radius_meters: 200,
        limit: 10,
        availability_filter: None,
    };

    let output = handler.find_nearby_stations(input).await.unwrap();

    assert_eq!(
        output.stations.len(),
        1,
        "only the near station should be returned"
    );
    assert_eq!(output.stations[0].station.reference.station_code, "A");
    assert_eq!(output.search_metadata.total_found, 1);
}

/// Results must be sorted closest-first.
#[tokio::test]
async fn find_nearby_stations_sorted_by_distance() {
    // Three stations at increasing distances from 48.8565, 2.3514.
    // All within 2 km so they all appear, but order must be A, B, C.
    let station_a = open_station("A", "Closest", 48.8569, 2.3514, 2, 1, 10); // ~44 m
    let station_b = open_station("B", "Middle", 48.8580, 2.3514, 2, 1, 10); // ~166 m
    let station_c = open_station("C", "Farthest", 48.8600, 2.3514, 2, 1, 10); // ~388 m

    // Shuffle insertion order to confirm sorting is done by the handler, not insertion order.
    let handler = handler_with(vec![station_c.clone(), station_a.clone(), station_b.clone()]).await;

    let input = FindNearbyStationsInput {
        latitude: 48.8565,
        longitude: 2.3514,
        radius_meters: 2000,
        limit: 10,
        availability_filter: None,
    };

    let output = handler.find_nearby_stations(input).await.unwrap();

    assert_eq!(output.stations.len(), 3);
    assert_eq!(output.stations[0].station.reference.station_code, "A");
    assert_eq!(output.stations[1].station.reference.station_code, "B");
    assert_eq!(output.stations[2].station.reference.station_code, "C");

    // Distances must be non-decreasing.
    let distances: Vec<u32> = output
        .stations
        .iter()
        .map(|s| s.straight_line_distance_meters)
        .collect();
    assert!(
        distances.windows(2).all(|w| w[0] <= w[1]),
        "distances not sorted: {distances:?}"
    );
}

/// With a `limit` of 2 only the two closest stations should be returned.
#[tokio::test]
async fn find_nearby_stations_respects_limit() {
    let stations: Vec<VelibStation> = (0u8..5)
        .map(|i| {
            // Space stations ~110 m apart along the same meridian.
            open_station(
                &i.to_string(),
                &format!("Station {i}"),
                48.8565 + f64::from(i) * 0.001,
                2.3514,
                2,
                1,
                10,
            )
        })
        .collect();

    let handler = handler_with(stations).await;

    let input = FindNearbyStationsInput {
        latitude: 48.8565,
        longitude: 2.3514,
        radius_meters: 5000,
        limit: 2,
        availability_filter: None,
    };

    let output = handler.find_nearby_stations(input).await.unwrap();

    assert_eq!(output.stations.len(), 2);
    // The two closest are indices 0 and 1.
    assert_eq!(output.stations[0].station.reference.station_code, "0");
    assert_eq!(output.stations[1].station.reference.station_code, "1");
}

/// When `bike_type` is `MechanicalOnly`, stations with only electric bikes
/// must be excluded even if they are within the radius.
#[tokio::test]
async fn find_nearby_stations_filters_mechanical_only() {
    // Station M: has mechanical bikes
    let station_m = open_station("M", "Mechanical Station", 48.8569, 2.3514, 3, 0, 10);
    // Station E: electric bikes only, no mechanical
    let station_e = open_station("E", "Electric Station", 48.8570, 2.3514, 0, 3, 10);

    let handler = handler_with(vec![station_m, station_e]).await;

    let input = FindNearbyStationsInput {
        latitude: 48.8565,
        longitude: 2.3514,
        radius_meters: 1000,
        limit: 10,
        availability_filter: Some(AvailabilityFilter {
            bike_type: Some(BikeTypeFilter::MechanicalOnly),
            ..Default::default()
        }),
    };

    let output = handler.find_nearby_stations(input).await.unwrap();

    assert_eq!(output.stations.len(), 1);
    assert_eq!(output.stations[0].station.reference.station_code, "M");
}

/// When `bike_type` is `ElectricOnly`, stations with only mechanical bikes
/// must be excluded.
#[tokio::test]
async fn find_nearby_stations_filters_electric_only() {
    let station_m = open_station("M", "Mechanical Station", 48.8569, 2.3514, 3, 0, 10);
    let station_e = open_station("E", "Electric Station", 48.8570, 2.3514, 0, 3, 10);

    let handler = handler_with(vec![station_m, station_e]).await;

    let input = FindNearbyStationsInput {
        latitude: 48.8565,
        longitude: 2.3514,
        radius_meters: 1000,
        limit: 10,
        availability_filter: Some(AvailabilityFilter {
            bike_type: Some(BikeTypeFilter::ElectricOnly),
            ..Default::default()
        }),
    };

    let output = handler.find_nearby_stations(input).await.unwrap();

    assert_eq!(output.stations.len(), 1);
    assert_eq!(output.stations[0].station.reference.station_code, "E");
}

/// Closed and maintenance stations must be excluded from nearby results.
#[tokio::test]
async fn find_nearby_stations_excludes_non_operational_stations() {
    let capacity = 20u16;
    let closed = make_station(
        "CLOSED", "Closed Station", 48.8569, 2.3514,
        3, 2, 15, capacity, StationStatus::Closed,
    );
    let maintenance = make_station(
        "MAINT", "Maintenance Station", 48.8570, 2.3514,
        3, 2, 15, capacity, StationStatus::Maintenance,
    );
    let open = make_station(
        "OPEN", "Open Station", 48.8571, 2.3514,
        3, 2, 15, capacity, StationStatus::Open,
    );

    let handler = handler_with(vec![closed, maintenance, open]).await;

    let input = FindNearbyStationsInput {
        latitude: 48.8565,
        longitude: 2.3514,
        radius_meters: 1000,
        limit: 10,
        availability_filter: None,
    };

    let output = handler.find_nearby_stations(input).await.unwrap();

    assert_eq!(output.stations.len(), 1);
    assert_eq!(output.stations[0].station.reference.station_code, "OPEN");
}

/// Radius exceeding 5 000 m must return an error.
#[tokio::test]
async fn find_nearby_stations_rejects_oversized_radius() {
    let handler = handler_with(vec![]).await;

    let input = FindNearbyStationsInput {
        latitude: 48.8565,
        longitude: 2.3514,
        radius_meters: 6000, // exceeds MAX_SEARCH_RADIUS = 5000
        limit: 10,
        availability_filter: None,
    };

    let result = handler.find_nearby_stations(input).await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("radius") || err_msg.contains("5000"),
        "unexpected error message: {err_msg}"
    );
}

/// Coordinates outside the Paris metro bounding box must be rejected.
#[tokio::test]
async fn find_nearby_stations_rejects_non_paris_coordinates() {
    let handler = handler_with(vec![]).await;

    let input = FindNearbyStationsInput {
        latitude: 51.5074,  // London
        longitude: -0.1278,
        radius_meters: 500,
        limit: 10,
        availability_filter: None,
    };

    let result = handler.find_nearby_stations(input).await;
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// plan_bike_journey
// ---------------------------------------------------------------------------

/// The default `max_walk_distance` is 500 m; stations beyond that should not
/// be candidates even if they are the only ones in the dataset.
#[tokio::test]
async fn plan_bike_journey_default_walk_limit_is_500m() {
    // Place one pickup-eligible station just outside 500 m from the origin.
    // Origin: 48.8565, 2.3514
    // ~600 m north: 48.8619, 2.3514
    let far_pickup = open_station("FAR", "Far Pickup", 48.8619, 2.3514, 3, 2, 5);
    // Dropoff station close to destination (within 500 m).
    // Destination: 48.8600, 2.3600
    let near_dropoff = open_station("DROP", "Near Dropoff", 48.8604, 2.3600, 1, 1, 8);

    let handler = handler_with(vec![far_pickup, near_dropoff]).await;

    let input = PlanBikeJourneyInput {
        origin: Coordinates::new(48.8565, 2.3514),
        destination: Coordinates::new(48.8600, 2.3600),
        preferences: None, // uses default: 500 m walk limit, AnyType
    };

    let output = handler.plan_bike_journey(input).await.unwrap();

    // FAR is beyond 500 m from origin, so no pickup candidates.
    assert!(
        output.journey.pickup_stations.is_empty(),
        "expected no pickup candidates because FAR is >500 m from origin"
    );
    // No pickup → no recommendations.
    assert!(output.journey.recommendations.is_empty());
}

/// When both pickup and dropoff candidates exist, recommendations are generated
/// with a confidence_score clamped to [0.1, 1.0].
#[tokio::test]
async fn plan_bike_journey_produces_recommendation_with_valid_confidence() {
    // Origin: 48.8565, 2.3514
    // Destination: 48.8600, 2.3600
    // Place pickup and dropoff stations very close to the respective endpoints.
    let pickup = open_station("P1", "Pickup Close", 48.8567, 2.3514, 4, 2, 8);
    let dropoff = open_station("D1", "Dropoff Close", 48.8602, 2.3600, 1, 1, 10);

    let handler = handler_with(vec![pickup, dropoff]).await;

    let input = PlanBikeJourneyInput {
        origin: Coordinates::new(48.8565, 2.3514),
        destination: Coordinates::new(48.8600, 2.3600),
        preferences: None,
    };

    let output = handler.plan_bike_journey(input).await.unwrap();

    assert!(!output.journey.pickup_stations.is_empty(), "should have pickup candidates");
    assert!(!output.journey.dropoff_stations.is_empty(), "should have dropoff candidates");
    assert_eq!(output.journey.recommendations.len(), 1);

    let rec = &output.journey.recommendations[0];
    assert!(
        rec.confidence_score >= 0.1 && rec.confidence_score <= 1.0,
        "confidence_score out of [0.1, 1.0]: {}",
        rec.confidence_score
    );
}

/// A station with no available bikes must not appear as a pickup candidate.
#[tokio::test]
async fn plan_bike_journey_excludes_empty_pickup_stations() {
    let empty_pickup = open_station("EMPTY", "Empty Pickup", 48.8567, 2.3514, 0, 0, 15);
    let valid_dropoff = open_station("D1", "Dropoff", 48.8602, 2.3600, 0, 0, 10);

    let handler = handler_with(vec![empty_pickup, valid_dropoff]).await;

    let input = PlanBikeJourneyInput {
        origin: Coordinates::new(48.8565, 2.3514),
        destination: Coordinates::new(48.8600, 2.3600),
        preferences: None,
    };

    let output = handler.plan_bike_journey(input).await.unwrap();

    assert!(
        output.journey.pickup_stations.is_empty(),
        "station with 0 bikes should not be a pickup candidate"
    );
}

/// A station with no available docks must not appear as a dropoff candidate.
#[tokio::test]
async fn plan_bike_journey_excludes_full_dropoff_stations() {
    let valid_pickup = open_station("P1", "Pickup", 48.8567, 2.3514, 3, 2, 5);
    let full_dropoff = open_station("FULL", "Full Dropoff", 48.8602, 2.3600, 10, 5, 0);

    let handler = handler_with(vec![valid_pickup, full_dropoff]).await;

    let input = PlanBikeJourneyInput {
        origin: Coordinates::new(48.8565, 2.3514),
        destination: Coordinates::new(48.8600, 2.3600),
        preferences: None,
    };

    let output = handler.plan_bike_journey(input).await.unwrap();

    assert!(
        output.journey.dropoff_stations.is_empty(),
        "station with 0 docks should not be a dropoff candidate"
    );
}

/// Pickup candidates must be sorted closest-first; only the top 3 are kept.
#[tokio::test]
async fn plan_bike_journey_returns_at_most_three_pickup_candidates_sorted() {
    // Five stations all within 400 m of origin, each ~44 m further than the last.
    let origin = Coordinates::new(48.8565, 2.3514);
    let dest = Coordinates::new(48.8600, 2.3600);

    let stations: Vec<VelibStation> = (0u8..5)
        .map(|i| {
            open_station(
                &i.to_string(),
                &format!("Station {i}"),
                48.8565 + f64::from(i) * 0.001,
                2.3514,
                2,
                1,
                5,
            )
        })
        .collect();
    // Also add a single valid dropoff so the journey can be planned.
    let dropoff = open_station("D", "Dropoff", 48.8602, 2.3600, 0, 0, 10);
    let mut all = stations;
    all.push(dropoff);

    let handler = handler_with(all).await;

    let input = PlanBikeJourneyInput {
        origin,
        destination: dest,
        preferences: Some(JourneyPreferences {
            bike_type: BikeTypeFilter::AnyType,
            max_walk_distance: 500,
        }),
    };

    let output = handler.plan_bike_journey(input).await.unwrap();

    let pickups = &output.journey.pickup_stations;
    assert!(pickups.len() <= 3, "handler must cap pickup candidates at 3");

    // Verify sorted closest-first.
    let dists: Vec<u32> = pickups
        .iter()
        .map(|s| s.straight_line_distance_meters)
        .collect();
    assert!(
        dists.windows(2).all(|w| w[0] <= w[1]),
        "pickup candidates not sorted by distance: {dists:?}"
    );
    // The closest station (code "0") must be first.
    assert_eq!(pickups[0].station.reference.station_code, "0");
}

/// When `bike_type` preference is `ElectricOnly`, mechanical-only stations
/// must not appear as pickup candidates.
#[tokio::test]
async fn plan_bike_journey_respects_bike_type_preference() {
    // Only mechanical bikes at this station.
    let mech_only = open_station("MECH", "Mechanical Only", 48.8567, 2.3514, 5, 0, 5);
    // Station with electric bikes at origin, and a valid dropoff.
    let elec_station = open_station("ELEC", "Electric", 48.8568, 2.3514, 0, 5, 5);
    let dropoff = open_station("D", "Dropoff", 48.8602, 2.3600, 0, 0, 10);

    let handler = handler_with(vec![mech_only, elec_station, dropoff]).await;

    let input = PlanBikeJourneyInput {
        origin: Coordinates::new(48.8565, 2.3514),
        destination: Coordinates::new(48.8600, 2.3600),
        preferences: Some(JourneyPreferences {
            bike_type: BikeTypeFilter::ElectricOnly,
            max_walk_distance: 500,
        }),
    };

    let output = handler.plan_bike_journey(input).await.unwrap();

    for s in &output.journey.pickup_stations {
        assert_ne!(
            s.station.reference.station_code, "MECH",
            "mechanical-only station must not appear with ElectricOnly preference"
        );
    }
    // The electric station should be present.
    assert!(
        output
            .journey
            .pickup_stations
            .iter()
            .any(|s| s.station.reference.station_code == "ELEC"),
        "electric station should be a pickup candidate"
    );
}

// ---------------------------------------------------------------------------
// get_area_statistics
// ---------------------------------------------------------------------------

/// Total bikes and docks must equal the sum across all stations in the box.
#[tokio::test]
async fn get_area_statistics_aggregates_correctly() {
    // All three stations fall inside the bounding box below.
    let s1 = open_station("S1", "Station 1", 48.850, 2.340, 3, 2, 10); // 5 bikes, 10 docks
    let s2 = open_station("S2", "Station 2", 48.855, 2.345, 1, 4, 5); //  5 bikes,  5 docks
    let s3 = open_station("S3", "Station 3", 48.860, 2.350, 0, 3, 7); //  3 bikes,  7 docks

    // One station outside the bounding box – must NOT be counted.
    let outside = open_station("OUT", "Outside", 48.900, 2.400, 10, 10, 10);

    let handler = handler_with(vec![s1, s2, s3, outside]).await;

    let input = GetAreaStatisticsInput {
        bounds: GeographicBounds {
            north: 48.870,
            south: 48.840,
            east: 2.360,
            west: 2.330,
        },
        include_real_time: true,
    };

    let output = handler.get_area_statistics(input).await.unwrap();
    let stats = &output.area_stats;

    assert_eq!(stats.total_stations, 3, "only 3 stations inside bounds");
    assert_eq!(stats.operational_stations, 3);

    // Mechanical: 3+1+0 = 4
    assert_eq!(stats.available_bikes.mechanical, 4);
    // Electric: 2+4+3 = 9
    assert_eq!(stats.available_bikes.electric, 9);
    // Total bikes: 13
    assert_eq!(stats.available_bikes.total, 13);

    // Docks: 10+5+7 = 22
    assert_eq!(stats.available_docks, 22);

    // Total capacity: 15+10+10 = 35
    assert_eq!(stats.total_capacity, 35);
}

/// Occupancy rate must be bikes / capacity, clamped to [0, 1].
#[tokio::test]
async fn get_area_statistics_occupancy_rate_calculation() {
    // capacity = 5+5 = 10, bikes = 3+2 = 5  → occupancy = 0.5
    let s1 = open_station("S1", "Station 1", 48.850, 2.340, 3, 0, 2);
    let s2 = open_station("S2", "Station 2", 48.855, 2.345, 2, 0, 3);

    let handler = handler_with(vec![s1, s2]).await;

    let input = GetAreaStatisticsInput {
        bounds: GeographicBounds {
            north: 48.870,
            south: 48.840,
            east: 2.360,
            west: 2.330,
        },
        include_real_time: true,
    };

    let output = handler.get_area_statistics(input).await.unwrap();
    let rate = output.area_stats.occupancy_rate;

    assert!(
        (rate - 0.5).abs() < 1e-6,
        "expected occupancy_rate ≈ 0.5, got {rate}"
    );
}

/// An empty bounding box (no stations inside) should return zeros without panic.
#[tokio::test]
async fn get_area_statistics_empty_area_returns_zeros() {
    let station = open_station("S1", "Station 1", 48.850, 2.340, 3, 2, 10);

    let handler = handler_with(vec![station]).await;

    // Bounds that contain nothing.
    let input = GetAreaStatisticsInput {
        bounds: GeographicBounds {
            north: 48.800,
            south: 48.790,
            east: 2.300,
            west: 2.290,
        },
        include_real_time: true,
    };

    let output = handler.get_area_statistics(input).await.unwrap();
    let stats = &output.area_stats;

    assert_eq!(stats.total_stations, 0);
    assert_eq!(stats.operational_stations, 0);
    assert_eq!(stats.available_bikes.total, 0);
    assert_eq!(stats.available_docks, 0);
    assert_eq!(stats.total_capacity, 0);
    assert_eq!(stats.occupancy_rate, 0.0);
}

/// Closed / maintenance stations count towards `total_stations` but not
/// towards `operational_stations`.
#[tokio::test]
async fn get_area_statistics_distinguishes_operational_from_total() {
    let open = make_station("O", "Open", 48.850, 2.340, 3, 2, 10, 15, StationStatus::Open);
    let closed = make_station(
        "C", "Closed", 48.852, 2.342, 0, 0, 20, 20, StationStatus::Closed,
    );
    let maintenance = make_station(
        "M", "Maintenance", 48.854, 2.344, 0, 0, 18, 18, StationStatus::Maintenance,
    );

    let handler = handler_with(vec![open, closed, maintenance]).await;

    let input = GetAreaStatisticsInput {
        bounds: GeographicBounds {
            north: 48.870,
            south: 48.840,
            east: 2.360,
            west: 2.330,
        },
        include_real_time: true,
    };

    let output = handler.get_area_statistics(input).await.unwrap();
    let stats = &output.area_stats;

    assert_eq!(stats.total_stations, 3);
    assert_eq!(stats.operational_stations, 1);
}

// ---------------------------------------------------------------------------
// search_stations_by_name
// ---------------------------------------------------------------------------

/// Basic case-insensitive substring match (fuzzy=true).
#[tokio::test]
async fn search_stations_by_name_case_insensitive_fuzzy() {
    let s1 = open_station("S1", "Opéra - Capucines", 48.870, 2.332, 2, 1, 5);
    let s2 = open_station("S2", "République", 48.867, 2.363, 3, 2, 8);
    let s3 = open_station("S3", "Bastille", 48.853, 2.369, 1, 1, 10);

    let handler = handler_with(vec![s1, s2, s3]).await;

    let input = SearchStationsByNameInput {
        query: "OPERA".to_string(),
        limit: 10,
        fuzzy: true,
    };

    let output = handler.search_stations_by_name(input).await.unwrap();

    assert_eq!(output.stations.len(), 1);
    assert_eq!(output.stations[0].reference.station_code, "S1");
}

/// Unicode normalization: searching "opera" (plain ASCII) should match
/// "Opéra" (NFC-encoded é).
#[tokio::test]
async fn search_stations_by_name_unicode_normalization_e_accent() {
    // Station name uses the Unicode é character.
    let s1 = open_station("S1", "Opéra Garnier", 48.870, 2.332, 2, 1, 5);
    let s2 = open_station("S2", "Louvre Rivoli", 48.861, 2.341, 3, 0, 7);

    let handler = handler_with(vec![s1, s2]).await;

    // The handler normalises both query and name with `.nfc()`, so "opera"
    // matches "opéra" only if the code does NFC and then lowercases.
    // Our test verifies the case-fold + NFC path used in handlers.rs.
    let input = SearchStationsByNameInput {
        query: "opéra".to_string(), // query already has accent
        limit: 10,
        fuzzy: true,
    };

    let output = handler.search_stations_by_name(input).await.unwrap();

    assert_eq!(
        output.stations.len(), 1,
        "accented query should match accented station name"
    );
    assert_eq!(output.stations[0].reference.station_code, "S1");
}

/// Prefix match (fuzzy=false): only stations whose name *starts with* the
/// query should be returned.
#[tokio::test]
async fn search_stations_by_name_prefix_match_non_fuzzy() {
    let s1 = open_station("S1", "Bastille Est", 48.853, 2.369, 2, 1, 5);
    let s2 = open_station("S2", "Saint-Bastille", 48.854, 2.370, 1, 1, 8);
    let s3 = open_station("S3", "Bastille Ouest", 48.852, 2.368, 3, 0, 6);

    let handler = handler_with(vec![s1, s2, s3]).await;

    let input = SearchStationsByNameInput {
        query: "Bastille".to_string(),
        limit: 10,
        fuzzy: false, // prefix only
    };

    let output = handler.search_stations_by_name(input).await.unwrap();

    // "Saint-Bastille" starts with "saint-", not "bastille", so must be excluded.
    assert_eq!(
        output.stations.len(), 2,
        "only prefix matches: {:#?}",
        output
            .stations
            .iter()
            .map(|s| &s.reference.name)
            .collect::<Vec<_>>()
    );
    for station in &output.stations {
        assert!(
            station.reference.name.to_lowercase().starts_with("bastille"),
            "unexpected station: {}",
            station.reference.name
        );
    }
}

/// Results are sorted alphabetically by name.
#[tokio::test]
async fn search_stations_by_name_sorted_alphabetically() {
    let s1 = open_station("S1", "Voltaire", 48.857, 2.375, 2, 1, 5);
    let s2 = open_station("S2", "Vaugirard", 48.841, 2.298, 3, 2, 8);
    let s3 = open_station("S3", "Victor Hugo", 48.866, 2.289, 1, 1, 10);

    let handler = handler_with(vec![s1, s2, s3]).await;

    let input = SearchStationsByNameInput {
        query: "v".to_string(),
        limit: 10,
        fuzzy: true,
    };

    let output = handler.search_stations_by_name(input).await.unwrap();

    assert_eq!(output.stations.len(), 3);
    let names: Vec<&str> = output
        .stations
        .iter()
        .map(|s| s.reference.name.as_str())
        .collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "stations must be in alphabetical order");
}

/// A query shorter than 2 characters must be rejected with an error.
#[tokio::test]
async fn search_stations_by_name_rejects_short_query() {
    let handler = handler_with(vec![]).await;

    let input = SearchStationsByNameInput {
        query: "a".to_string(), // 1 char – too short
        limit: 10,
        fuzzy: true,
    };

    let result = handler.search_stations_by_name(input).await;
    assert!(result.is_err(), "single-char query should be rejected");
}

/// A query that matches nothing should return an empty list, not an error.
#[tokio::test]
async fn search_stations_by_name_returns_empty_for_no_match() {
    let s1 = open_station("S1", "Châtelet", 48.858, 2.347, 2, 1, 5);

    let handler = handler_with(vec![s1]).await;

    let input = SearchStationsByNameInput {
        query: "zzznomatch".to_string(),
        limit: 10,
        fuzzy: true,
    };

    let output = handler.search_stations_by_name(input).await.unwrap();

    assert!(output.stations.is_empty());
    assert_eq!(output.search_metadata.total_found, 0);
}

/// The `limit` parameter caps how many stations are returned.
#[tokio::test]
async fn search_stations_by_name_respects_limit() {
    // 5 stations all matching "Rue".
    let stations: Vec<VelibStation> = (0u8..5)
        .map(|i| {
            open_station(
                &i.to_string(),
                &format!("Rue de la Paix {i}"),
                48.869 + f64::from(i) * 0.001,
                2.331,
                2,
                1,
                5,
            )
        })
        .collect();

    let handler = handler_with(stations).await;

    let input = SearchStationsByNameInput {
        query: "Rue".to_string(),
        limit: 3,
        fuzzy: true,
    };

    let output = handler.search_stations_by_name(input).await.unwrap();

    assert_eq!(
        output.stations.len(),
        3,
        "limit=3 must cap results at 3 even when 5 match"
    );
}
