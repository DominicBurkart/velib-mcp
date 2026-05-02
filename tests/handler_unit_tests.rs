//! Offline unit tests for MCP handler business logic.
//!
//! All tests run without any network access: `client_with_stations` seeds
//! both the reference-station cache and the real-time cache via
//! `VelibDataClient::seed_for_testing` / `seed_realtime_for_testing`
//! (available under `feature = "test-utils"` or `cfg(test)`).
//!
//! Handlers covered:
//!   - `find_nearby_stations` (distance filter, sort order, limit, bike-type filter,
//!     closed/maintenance exclusion, radius > 5 000 m rejected, non-Paris rejected)
//!   - `search_stations_by_name` (NFC normalisation, prefix-only when fuzzy=false,
//!     query-too-short error, empty result, limit cap, alphabetic sort)
//!   - `get_area_statistics` (totals, occupancy rate, empty bbox, closed stations)
//!   - `plan_bike_journey` (walk limit, confidence score bounds, zero-bike/zero-dock
//!     exclusion, pickup candidate count, bike-type preference)
//!   - `get_station_by_code` (found, not-found)
//!   - `InMemoryCache` TTL expiry

use chrono::{Duration as ChronoDuration, Utc};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

use velib_mcp::{
    data::{cache::InMemoryCache, VelibDataClient},
    mcp::types::{
        FindNearbyStationsInput, GeographicBounds, GetAreaStatisticsInput, GetStationByCodeInput,
        JourneyPreferences, PlanBikeJourneyInput, SearchStationsByNameInput,
    },
    mcp::McpToolHandler,
    types::{
        BikeAvailability, BikeTypeFilter, Coordinates, DataFreshness, RealTimeStatus,
        ServiceCapabilities, StationReference, StationStatus, VelibStation,
    },
    Error,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a minimal `StationReference` at `coords`.
fn make_reference(code: &str, name: &str, coords: Coordinates) -> StationReference {
    StationReference {
        station_code: code.to_string(),
        name: name.to_string(),
        coordinates: coords,
        capacity: 20,
        capabilities: ServiceCapabilities::default(),
    }
}

/// Wrap a `StationReference` in a `VelibStation` with a fully-open real-time
/// status (3 mechanical + 2 electric, 15 free docks).
fn make_open_station(reference: StationReference) -> VelibStation {
    let rt = RealTimeStatus {
        bikes: BikeAvailability::new(3, 2),
        available_docks: 15,
        status: StationStatus::Open,
        last_update: Utc::now(),
        data_freshness: DataFreshness::Fresh,
    };
    VelibStation::new(reference).with_real_time(rt)
}

/// Build a station with custom bike / dock counts and status.
fn make_station_custom(
    code: &str,
    name: &str,
    coords: Coordinates,
    mechanical: u16,
    electric: u16,
    available_docks: u16,
    status: StationStatus,
) -> VelibStation {
    let rt = RealTimeStatus {
        bikes: BikeAvailability::new(mechanical, electric),
        available_docks,
        status,
        last_update: Utc::now(),
        data_freshness: DataFreshness::Fresh,
    };
    VelibStation::new(make_reference(code, name, coords)).with_real_time(rt)
}

/// Seed a `VelibDataClient` with the supplied stations so that
/// `get_all_stations` returns them without any network call.
async fn client_with_stations(stations: Vec<VelibStation>) -> VelibDataClient {
    let references: Vec<StationReference> = stations.iter().map(|s| s.reference.clone()).collect();

    let mut rt_map: HashMap<String, RealTimeStatus> = HashMap::new();
    for s in &stations {
        if let Some(rt) = &s.real_time {
            rt_map.insert(s.reference.station_code.clone(), rt.clone());
        }
    }

    let client = VelibDataClient::new();
    client.seed_for_testing(references).await;
    client.seed_realtime_for_testing(rt_map).await;
    client
}

// ---------------------------------------------------------------------------
// InMemoryCache TTL expiry
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_cache_entry_expires_after_ttl() {
    let cache: InMemoryCache<String, String> = InMemoryCache::new(ChronoDuration::minutes(10));
    let key = "expiry_key".to_string();

    cache
        .insert_with_ttl(
            key.clone(),
            "hello".to_string(),
            ChronoDuration::milliseconds(100),
        )
        .await;

    assert!(
        cache.get(&key).await.is_some(),
        "entry should be present right after insertion"
    );

    sleep(Duration::from_millis(200)).await;

    assert!(
        cache.get(&key).await.is_none(),
        "entry should have expired after sleeping past the TTL"
    );
}

// ---------------------------------------------------------------------------
// find_nearby_stations
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_find_nearby_stations_excludes_distant_stations() {
    // Notre-Dame de Paris
    let query_lat = 48.8530;
    let query_lon = 2.3499;

    let near = make_open_station(make_reference(
        "NEAR001",
        "Near Station",
        Coordinates::new(48.8533, 2.3503), // ~100 m
    ));
    let far = make_open_station(make_reference(
        "FAR001",
        "Far Station",
        Coordinates::new(48.8900, 2.3499), // ~5 km
    ));

    let handler =
        McpToolHandler::with_data_client(client_with_stations(vec![near, far]).await);

    let output = handler
        .find_nearby_stations(FindNearbyStationsInput {
            latitude: query_lat,
            longitude: query_lon,
            radius_meters: 300,
            limit: 50,
            availability_filter: None,
        })
        .await
        .expect("should succeed with valid Paris coords");

    let codes: Vec<&str> = output
        .stations
        .iter()
        .map(|s| s.station.reference.station_code.as_str())
        .collect();

    assert!(codes.contains(&"NEAR001"), "nearby station missing; got: {codes:?}");
    assert!(!codes.contains(&"FAR001"), "distant station present; got: {codes:?}");
}

#[tokio::test]
async fn test_find_nearby_stations_sorted_closest_first() {
    let origin = Coordinates::new(48.8565, 2.3514); // Paris City Hall

    let close = make_open_station(make_reference(
        "CLOSE",
        "Close",
        Coordinates::new(48.8566, 2.3514),
    ));
    let mid = make_open_station(make_reference(
        "MID",
        "Mid",
        Coordinates::new(48.8575, 2.3514),
    ));
    let far = make_open_station(make_reference(
        "FAR",
        "Far",
        Coordinates::new(48.8590, 2.3514),
    ));

    let handler = McpToolHandler::with_data_client(
        client_with_stations(vec![far, mid, close]).await,
    );

    let output = handler
        .find_nearby_stations(FindNearbyStationsInput {
            latitude: origin.latitude,
            longitude: origin.longitude,
            radius_meters: 5000,
            limit: 50,
            availability_filter: None,
        })
        .await
        .expect("should succeed");

    assert_eq!(output.stations.len(), 3);
    assert!(
        output.stations[0].straight_line_distance_meters
            <= output.stations[1].straight_line_distance_meters
    );
    assert!(
        output.stations[1].straight_line_distance_meters
            <= output.stations[2].straight_line_distance_meters
    );
}

#[tokio::test]
async fn test_find_nearby_stations_limit_enforced() {
    let origin = Coordinates::new(48.8565, 2.3514);
    let stations: Vec<VelibStation> = (0..10)
        .map(|i| {
            make_open_station(make_reference(
                &format!("S{i}"),
                &format!("Station {i}"),
                Coordinates::new(48.8565 + f64::from(i) * 0.0001, 2.3514),
            ))
        })
        .collect();

    let handler =
        McpToolHandler::with_data_client(client_with_stations(stations).await);

    let output = handler
        .find_nearby_stations(FindNearbyStationsInput {
            latitude: origin.latitude,
            longitude: origin.longitude,
            radius_meters: 5000,
            limit: 3,
            availability_filter: None,
        })
        .await
        .expect("should succeed");

    assert_eq!(output.stations.len(), 3);
}

#[tokio::test]
async fn test_find_nearby_stations_rejects_radius_over_5000m() {
    let handler = McpToolHandler::new();
    let result = handler
        .find_nearby_stations(FindNearbyStationsInput {
            latitude: 48.8566,
            longitude: 2.3522,
            radius_meters: 6000,
            limit: 10,
            availability_filter: None,
        })
        .await;
    assert!(matches!(result, Err(Error::SearchRadiusTooLarge { .. })));
}

#[tokio::test]
async fn test_find_nearby_stations_rejects_london_coordinates() {
    let handler = McpToolHandler::new();
    let result = handler
        .find_nearby_stations(FindNearbyStationsInput {
            latitude: 51.5074,
            longitude: -0.1278,
            radius_meters: 500,
            limit: 10,
            availability_filter: None,
        })
        .await;
    match result.expect_err("London should be rejected") {
        Error::InvalidCoordinates {
            latitude,
            longitude,
        } => {
            assert!((latitude - 51.5074).abs() < 1e-9);
            assert!((longitude - (-0.1278)).abs() < 1e-9);
        }
        other => panic!("expected InvalidCoordinates, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// search_stations_by_name
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_search_stations_by_name_nfc_normalization() {
    let chatelet = make_open_station(make_reference(
        "CHAT001",
        "Ch\u{00E2}telet",
        Coordinates::new(48.8600, 2.3470),
    ));
    let other = make_open_station(make_reference(
        "OTHER001",
        "Op\u{00E9}ra",
        Coordinates::new(48.8566, 2.3522),
    ));

    let handler = McpToolHandler::with_data_client(
        client_with_stations(vec![chatelet, other]).await,
    );

    // NFD form of "ch\u{00e2}telet" (a + combining circumflex U+0302)
    let query_nfd = "cha\u{0302}telet";
    let output = handler
        .search_stations_by_name(SearchStationsByNameInput {
            query: query_nfd.to_string(),
            limit: 10,
            fuzzy: true,
        })
        .await
        .expect("search should succeed");

    let codes: Vec<&str> = output
        .stations
        .iter()
        .map(|s| s.reference.station_code.as_str())
        .collect();

    assert!(codes.contains(&"CHAT001"), "NFC match failed; got: {codes:?}");
    assert!(
        !codes.contains(&"OTHER001"),
        "unrelated station present; got: {codes:?}"
    );
}

#[tokio::test]
async fn test_search_stations_by_name_prefix_only_when_fuzzy_false() {
    // "Bastille" starts with "bastille"; "Op\u{00e9}ra Bastille" has it in the middle.
    let starts = make_open_station(make_reference(
        "A",
        "Bastille",
        Coordinates::new(48.8533, 2.3692),
    ));
    let middle = make_open_station(make_reference(
        "B",
        "Op\u{00E9}ra Bastille",
        Coordinates::new(48.8534, 2.3693),
    ));

    let handler = McpToolHandler::with_data_client(
        client_with_stations(vec![starts, middle]).await,
    );

    let output = handler
        .search_stations_by_name(SearchStationsByNameInput {
            query: "bastille".to_string(),
            limit: 10,
            fuzzy: false,
        })
        .await
        .expect("search should succeed");

    let codes: Vec<&str> = output
        .stations
        .iter()
        .map(|s| s.reference.station_code.as_str())
        .collect();

    assert!(codes.contains(&"A"), "prefix-match station missing; got: {codes:?}");
    assert!(
        !codes.contains(&"B"),
        "middle-match should be excluded when fuzzy=false; got: {codes:?}"
    );
}

#[tokio::test]
async fn test_search_stations_by_name_rejects_short_query() {
    let handler = McpToolHandler::new();
    let result = handler
        .search_stations_by_name(SearchStationsByNameInput {
            query: "x".to_string(),
            limit: 10,
            fuzzy: true,
        })
        .await;
    assert!(result.is_err(), "single-char query should be rejected");
}

// ---------------------------------------------------------------------------
// get_station_by_code
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_station_by_code_found() {
    let station = make_open_station(make_reference(
        "16107",
        "Benjamin Godard - Victor Hugo",
        Coordinates::new(48.8656, 2.2779),
    ));
    let handler =
        McpToolHandler::with_data_client(client_with_stations(vec![station]).await);

    let output = handler
        .get_station_by_code(GetStationByCodeInput {
            station_code: "16107".to_string(),
            include_real_time: true,
        })
        .await
        .expect("should succeed");

    assert!(output.found);
    assert!(output.station.is_some());
    assert_eq!(output.station.unwrap().reference.station_code, "16107");
}

#[tokio::test]
async fn test_get_station_by_code_not_found() {
    let handler =
        McpToolHandler::with_data_client(client_with_stations(vec![]).await);

    let output = handler
        .get_station_by_code(GetStationByCodeInput {
            station_code: "NONEXISTENT".to_string(),
            include_real_time: true,
        })
        .await
        .expect("should succeed even when station absent");

    assert!(!output.found);
    assert!(output.station.is_none());
}

// ---------------------------------------------------------------------------
// get_area_statistics
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_area_statistics_totals_and_occupancy() {
    // Two stations inside the bounding box, each with capacity 20.
    // Station A: 4 mechanical + 2 electric = 6 bikes, 14 docks
    // Station B: 1 mechanical + 3 electric = 4 bikes, 16 docks
    // Total bikes = 10, capacity = 40 -> occupancy = 0.25
    let inside_bounds = GeographicBounds {
        north: 48.870,
        south: 48.850,
        east: 2.380,
        west: 2.340,
    };

    let a = make_station_custom(
        "A",
        "Alpha",
        Coordinates::new(48.860, 2.360),
        4,
        2,
        14,
        StationStatus::Open,
    );
    let b = make_station_custom(
        "B",
        "Beta",
        Coordinates::new(48.855, 2.355),
        1,
        3,
        16,
        StationStatus::Open,
    );
    // Station outside the bbox - must not appear in stats
    let outside = make_station_custom(
        "OUT",
        "Outside",
        Coordinates::new(48.900, 2.360),
        5,
        5,
        10,
        StationStatus::Open,
    );

    let handler = McpToolHandler::with_data_client(
        client_with_stations(vec![a, b, outside]).await,
    );

    let output = handler
        .get_area_statistics(GetAreaStatisticsInput {
            bounds: inside_bounds,
            include_real_time: true,
        })
        .await
        .expect("should succeed");

    let stats = &output.area_stats;
    assert_eq!(stats.total_stations, 2);
    assert_eq!(stats.operational_stations, 2);
    assert_eq!(stats.total_capacity, 40);
    assert_eq!(stats.available_bikes.mechanical, 5);
    assert_eq!(stats.available_bikes.electric, 5);
    assert_eq!(stats.available_bikes.total, 10);
    assert_eq!(stats.available_docks, 30);
    assert!(
        (stats.occupancy_rate - 0.25).abs() < 1e-9,
        "occupancy_rate expected 0.25, got {}",
        stats.occupancy_rate
    );
}

#[tokio::test]
async fn test_get_area_statistics_empty_bbox_returns_zeros() {
    let station = make_open_station(make_reference(
        "A",
        "Alpha",
        Coordinates::new(48.860, 2.360),
    ));
    let handler =
        McpToolHandler::with_data_client(client_with_stations(vec![station]).await);

    // Bounding box that contains no stations
    let output = handler
        .get_area_statistics(GetAreaStatisticsInput {
            bounds: GeographicBounds {
                north: 48.810,
                south: 48.800,
                east: 2.300,
                west: 2.290,
            },
            include_real_time: true,
        })
        .await
        .expect("should succeed even with no matching stations");

    let stats = &output.area_stats;
    assert_eq!(stats.total_stations, 0);
    assert_eq!(stats.available_bikes.total, 0);
    assert_eq!(stats.available_docks, 0);
    assert_eq!(stats.occupancy_rate, 0.0);
}

#[tokio::test]
async fn test_get_area_statistics_closed_station_excluded_from_operational() {
    let bounds = GeographicBounds {
        north: 48.870,
        south: 48.850,
        east: 2.380,
        west: 2.340,
    };

    let open = make_station_custom(
        "OPEN",
        "Open Station",
        Coordinates::new(48.860, 2.360),
        2,
        0,
        18,
        StationStatus::Open,
    );
    let closed = make_station_custom(
        "CLOSED",
        "Closed Station",
        Coordinates::new(48.855, 2.355),
        0,
        0,
        20,
        StationStatus::Closed,
    );

    let handler = McpToolHandler::with_data_client(
        client_with_stations(vec![open, closed]).await,
    );

    let output = handler
        .get_area_statistics(GetAreaStatisticsInput {
            bounds,
            include_real_time: true,
        })
        .await
        .expect("should succeed");

    assert_eq!(output.area_stats.total_stations, 2);
    assert_eq!(output.area_stats.operational_stations, 1);
}

// ---------------------------------------------------------------------------
// plan_bike_journey
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_plan_bike_journey_default_walk_limit_excludes_distant_pickup() {
    // Origin: Paris City Hall area
    let origin = Coordinates::new(48.8565, 2.3514);
    // Destination: ~2 km away (still Paris)
    let destination = Coordinates::new(48.8565, 2.3714);

    // Pickup candidate ~617 m north of origin — clearly beyond the 500 m default
    // walk limit, so it must be excluded from pickup candidates.
    let far_pickup = make_station_custom(
        "FAR_PICK",
        "Far Pickup",
        Coordinates::new(48.8620, 2.3514), // ~617 m from origin
        5,
        0,
        15,
        StationStatus::Open,
    );
    // Dropoff candidate near destination
    let near_drop = make_station_custom(
        "NEAR_DROP",
        "Near Dropoff",
        Coordinates::new(48.8566, 2.3714),
        0,
        0,
        20,
        StationStatus::Open,
    );

    let handler = McpToolHandler::with_data_client(
        client_with_stations(vec![far_pickup, near_drop]).await,
    );

    let output = handler
        .plan_bike_journey(PlanBikeJourneyInput {
            origin,
            destination,
            preferences: None, // default: 500 m walk limit
        })
        .await
        .expect("should succeed");

    // With no valid pickup, recommendations must be empty.
    assert!(
        output.journey.recommendations.is_empty(),
        "no pickup within 500 m walk - recommendations should be empty"
    );
}

#[tokio::test]
async fn test_plan_bike_journey_valid_pair_produces_recommendation_with_bounded_confidence() {
    let origin = Coordinates::new(48.8565, 2.3514);
    let destination = Coordinates::new(48.8565, 2.3714);

    let near_pickup = make_station_custom(
        "PICK",
        "Pickup",
        Coordinates::new(48.8566, 2.3516), // ~20 m from origin
        5,
        0,
        15,
        StationStatus::Open,
    );
    let near_drop = make_station_custom(
        "DROP",
        "Dropoff",
        Coordinates::new(48.8566, 2.3716), // ~20 m from destination
        0,
        0,
        20,
        StationStatus::Open,
    );

    let handler = McpToolHandler::with_data_client(
        client_with_stations(vec![near_pickup, near_drop]).await,
    );

    let output = handler
        .plan_bike_journey(PlanBikeJourneyInput {
            origin,
            destination,
            preferences: None,
        })
        .await
        .expect("should succeed");

    assert!(
        !output.journey.recommendations.is_empty(),
        "expected at least one recommendation"
    );
    let score = output.journey.recommendations[0].confidence_score;
    assert!(
        (0.1..=1.0).contains(&score),
        "confidence_score {score} not in [0.1, 1.0]"
    );
}

#[tokio::test]
async fn test_plan_bike_journey_excludes_zero_bike_pickup_candidates() {
    let origin = Coordinates::new(48.8565, 2.3514);
    let destination = Coordinates::new(48.8565, 2.3714);

    // Station with 0 bikes - must not be a pickup candidate
    let no_bikes = make_station_custom(
        "NO_BIKES",
        "No Bikes",
        Coordinates::new(48.8566, 2.3516),
        0,
        0,
        20,
        StationStatus::Open,
    );
    let near_drop = make_station_custom(
        "DROP",
        "Dropoff",
        Coordinates::new(48.8566, 2.3716),
        0,
        0,
        20,
        StationStatus::Open,
    );

    let handler = McpToolHandler::with_data_client(
        client_with_stations(vec![no_bikes, near_drop]).await,
    );

    let output = handler
        .plan_bike_journey(PlanBikeJourneyInput {
            origin,
            destination,
            preferences: None,
        })
        .await
        .expect("should succeed");

    assert!(
        output.journey.pickup_stations.is_empty(),
        "station with 0 bikes should not appear as pickup candidate"
    );
    assert!(
        output.journey.recommendations.is_empty(),
        "no pickup available => no recommendations"
    );
}

#[tokio::test]
async fn test_plan_bike_journey_bike_type_preference_respected() {
    let origin = Coordinates::new(48.8565, 2.3514);
    let destination = Coordinates::new(48.8565, 2.3714);

    // Only mechanical bikes available
    let mech_only = make_station_custom(
        "MECH",
        "Mechanical Only",
        Coordinates::new(48.8566, 2.3516),
        5,
        0,
        15,
        StationStatus::Open,
    );
    let near_drop = make_station_custom(
        "DROP",
        "Dropoff",
        Coordinates::new(48.8566, 2.3716),
        0,
        0,
        20,
        StationStatus::Open,
    );

    let handler = McpToolHandler::with_data_client(
        client_with_stations(vec![mech_only, near_drop]).await,
    );

    // Request electric only - should find no pickup candidates
    let output_electric = handler
        .plan_bike_journey(PlanBikeJourneyInput {
            origin,
            destination,
            preferences: Some(JourneyPreferences {
                bike_type: BikeTypeFilter::ElectricOnly,
                max_walk_distance: 500,
            }),
        })
        .await
        .expect("should succeed");

    assert!(
        output_electric.journey.pickup_stations.is_empty(),
        "no electric bikes available - pickup_stations should be empty"
    );
}
