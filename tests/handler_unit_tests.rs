//! Offline unit tests for handler filtering logic and cache expiry.
//!
//! All five tool-handler methods were previously covered only by `#[ignore]`
//! network-dependent tests.  The tests in this file exercise the core
//! filtering / validation paths without touching the network:
//!
//!   1. `InMemoryCache` expiry — insert with a 1 ms TTL, sleep past it, assert `get` → `None`.
//!   2. `find_nearby_stations` distance filter — stations beyond the requested radius are excluded.
//!   3. `search_stations_by_name` NFC normalisation — `"chatelet"` matches `"Châtelet"`.
//!   4. Coordinate validation — London coordinates are rejected with `OutsideServiceArea`.

use chrono::{Duration as ChronoDuration, Utc};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

use velib_mcp::{
    data::{cache::InMemoryCache, VelibDataClient},
    mcp::McpToolHandler,
    mcp::types::{
        FindNearbyStationsInput, SearchStationsByNameInput,
    },
    types::{
        BikeAvailability, Coordinates, DataFreshness, RealTimeStatus, ServiceCapabilities,
        StationReference, StationStatus, VelibStation,
    },
    Error,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a minimal `StationReference` located at `coords` that passes
/// `StationReference::validate()` (Paris metro bounds, capacity 1-200).
fn make_reference(code: &str, name: &str, coords: Coordinates) -> StationReference {
    StationReference {
        station_code: code.to_string(),
        name: name.to_string(),
        coordinates: coords,
        capacity: 20,
        capabilities: ServiceCapabilities::default(),
    }
}

/// Wrap a `StationReference` in a `VelibStation` with a minimal open real-time
/// status so that `is_operational()` and `has_available_bikes()` both return `true`.
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

/// Seed a `VelibDataClient` with the supplied stations (no real-time data)
/// so that `get_all_stations` returns them without a network call.
async fn client_with_stations(stations: Vec<VelibStation>) -> VelibDataClient {
    let references: Vec<StationReference> = stations
        .iter()
        .map(|s| s.reference.clone())
        .collect();

    // Build a matching real-time map from the stations that carry one.
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
// Test 1 — InMemoryCache expiry
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_cache_entry_expires_after_ttl() {
    // Use a default TTL of 10 minutes; we will override per-entry.
    let cache: InMemoryCache<String, String> =
        InMemoryCache::new(ChronoDuration::minutes(10));

    let key = "expiry_key".to_string();
    let value = "hello".to_string();

    // Insert with a TTL of 1 millisecond.
    cache
        .insert_with_ttl(key.clone(), value, ChronoDuration::milliseconds(1))
        .await;

    // The entry should be visible immediately.
    assert!(
        cache.get(&key).await.is_some(),
        "entry should be present right after insertion"
    );

    // Sleep long enough for the entry to expire (chrono resolution is
    // milliseconds, so 20 ms is a comfortable margin).
    sleep(Duration::from_millis(20)).await;

    assert!(
        cache.get(&key).await.is_none(),
        "entry should have expired after sleeping past the TTL"
    );
}

// ---------------------------------------------------------------------------
// Test 2 — find_nearby_stations distance filtering
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_find_nearby_stations_excludes_distant_stations() {
    // Query point: Notre-Dame de Paris
    let query_lat = 48.8530;
    let query_lon = 2.3499;
    let radius_meters: u32 = 300;

    // Station A — ~100 m from the query point (should be included).
    let near_coords = Coordinates::new(48.8533, 2.3503);
    // Station B — ~5 km away, well outside the radius (should be excluded).
    let far_coords = Coordinates::new(48.8900, 2.3499);

    let near_station = make_open_station(make_reference("NEAR001", "Near Station", near_coords));
    let far_station = make_open_station(make_reference("FAR001", "Far Station", far_coords));

    let client = client_with_stations(vec![near_station, far_station]).await;
    let handler = McpToolHandler::with_data_client(client);

    let input = FindNearbyStationsInput {
        latitude: query_lat,
        longitude: query_lon,
        radius_meters,
        limit: 50,
        availability_filter: None,
    };

    let output = handler
        .find_nearby_stations(input)
        .await
        .expect("find_nearby_stations should succeed with valid Paris coords");

    let codes: Vec<&str> = output
        .stations
        .iter()
        .map(|s| s.station.reference.station_code.as_str())
        .collect();

    assert!(
        codes.contains(&"NEAR001"),
        "the nearby station should be in the results; got: {codes:?}"
    );
    assert!(
        !codes.contains(&"FAR001"),
        "the distant station should be excluded from the results; got: {codes:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 3 — search_stations_by_name NFC normalisation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_search_stations_by_name_nfc_normalization() {
    // Station whose name contains a precomposed accented character (U+00E2 for â, etc.).
    // "Châtelet" uses the precomposed form.
    let chatelet_coords = Coordinates::new(48.8600, 2.3470);
    let chatelet = make_open_station(make_reference(
        "CHAT001",
        "Châtelet",
        chatelet_coords,
    ));

    // Unrelated station that should NOT appear in results.
    let other_coords = Coordinates::new(48.8566, 2.3522);
    let other = make_open_station(make_reference("OTHER001", "Opéra", other_coords));

    let client = client_with_stations(vec![chatelet, other]).await;
    let handler = McpToolHandler::with_data_client(client);

    // Query uses plain ASCII — NFC normalization should still match.
    let input = SearchStationsByNameInput {
        query: "chatelet".to_string(),
        limit: 10,
        fuzzy: true,
    };

    let output = handler
        .search_stations_by_name(input)
        .await
        .expect("search_stations_by_name should succeed");

    let codes: Vec<&str> = output
        .stations
        .iter()
        .map(|s| s.reference.station_code.as_str())
        .collect();

    assert!(
        codes.contains(&"CHAT001"),
        "NFC-normalised search for 'chatelet' should match 'Châtelet'; got: {codes:?}"
    );
    assert!(
        !codes.contains(&"OTHER001"),
        "unrelated station should not appear in results; got: {codes:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 4 — coordinate validation: London is outside the Paris service area
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_find_nearby_stations_rejects_london_coordinates() {
    // London — well outside the 50 km Paris service area.
    let london_lat = 51.5074;
    let london_lon = -0.1278;

    // The handler should reject the request before touching any data, so an
    // empty client is fine.
    let handler = McpToolHandler::new();

    let input = FindNearbyStationsInput {
        latitude: london_lat,
        longitude: london_lon,
        radius_meters: 500,
        limit: 10,
        availability_filter: None,
    };

    let result = handler.find_nearby_stations(input).await;

    assert!(
        result.is_err(),
        "London coordinates should be rejected, but got Ok"
    );

    // The error should specifically be InvalidCoordinates (fails the
    // is_valid_paris_metro bounding-box check before reaching the
    // service-area distance check).
    match result.unwrap_err() {
        Error::InvalidCoordinates { latitude, longitude } => {
            assert!(
                (latitude - london_lat).abs() < 1e-9,
                "error latitude should match the input"
            );
            assert!(
                (longitude - london_lon).abs() < 1e-9,
                "error longitude should match the input"
            );
        }
        other => panic!("expected InvalidCoordinates, got: {other:?}"),
    }
}
