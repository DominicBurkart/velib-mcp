/// Unit tests covering components that previously lacked direct test coverage:
///
/// - `InMemoryCache`: insert, get, TTL expiry, `cleanup_expired`, `remove`, `clear`
/// - `GeographicBounds::contains`: boundary edge cases
/// - `McpToolHandler` input-validation paths (no network required):
///   radius too large, result limit exceeded, out-of-bounds coordinates,
///   search query too short
/// - `BikeAvailability` saturation arithmetic
/// - `StationReference::validate` — each distinct error path
use chrono::{Duration, Utc};
use velib_mcp::data::cache::InMemoryCache;
use velib_mcp::mcp::types::{
    AvailabilityFilter, FindNearbyStationsInput, GeographicBounds, SearchStationsByNameInput,
};
use velib_mcp::mcp::McpToolHandler;
use velib_mcp::types::{
    BikeAvailability, Coordinates, DataFreshness, RealTimeStatus, ServiceCapabilities,
    StationReference, StationStatus,
};

// ---------------------------------------------------------------------------
// InMemoryCache
// ---------------------------------------------------------------------------

#[tokio::test]
async fn cache_insert_and_get_returns_value() {
    let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::minutes(5));
    cache.insert("key".to_string(), 42).await;
    assert_eq!(cache.get(&"key".to_string()).await, Some(42));
}

#[tokio::test]
async fn cache_get_missing_key_returns_none() {
    let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::minutes(5));
    assert_eq!(cache.get(&"missing".to_string()).await, None);
}

#[tokio::test]
async fn cache_expired_entry_is_not_returned() {
    let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::minutes(5));
    // Insert with a TTL that has already elapsed
    cache
        .insert_with_ttl("key".to_string(), 99, Duration::seconds(-1))
        .await;
    assert_eq!(cache.get(&"key".to_string()).await, None);
}

#[tokio::test]
async fn cache_cleanup_expired_removes_stale_entries() {
    let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::minutes(5));
    cache
        .insert_with_ttl("stale".to_string(), 1, Duration::seconds(-1))
        .await;
    cache.insert("fresh".to_string(), 2).await;

    assert_eq!(cache.size().await, 2);
    cache.cleanup_expired().await;
    assert_eq!(cache.size().await, 1);
    assert_eq!(cache.get(&"fresh".to_string()).await, Some(2));
    assert_eq!(cache.get(&"stale".to_string()).await, None);
}

#[tokio::test]
async fn cache_remove_returns_value_and_decreases_size() {
    let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::minutes(5));
    cache.insert("k".to_string(), 7).await;

    let removed = cache.remove(&"k".to_string()).await;
    assert_eq!(removed, Some(7));
    assert_eq!(cache.size().await, 0);
}

#[tokio::test]
async fn cache_clear_empties_all_entries() {
    let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::minutes(5));
    cache.insert("a".to_string(), 1).await;
    cache.insert("b".to_string(), 2).await;

    cache.clear().await;
    assert_eq!(cache.size().await, 0);
}

#[tokio::test]
async fn cache_insert_overwrites_existing_key() {
    let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::minutes(5));
    cache.insert("k".to_string(), 1).await;
    cache.insert("k".to_string(), 2).await;

    assert_eq!(cache.get(&"k".to_string()).await, Some(2));
    assert_eq!(cache.size().await, 1);
}

// ---------------------------------------------------------------------------
// GeographicBounds::contains
// ---------------------------------------------------------------------------

fn paris_center_bounds() -> GeographicBounds {
    GeographicBounds {
        north: 48.90,
        south: 48.80,
        east: 2.40,
        west: 2.30,
    }
}

#[test]
fn bounds_contains_interior_point() {
    let bounds = paris_center_bounds();
    let inside = Coordinates::new(48.85, 2.35);
    assert!(bounds.contains(&inside));
}

#[test]
fn bounds_rejects_point_north_of_bounds() {
    let bounds = paris_center_bounds();
    assert!(!bounds.contains(&Coordinates::new(48.91, 2.35)));
}

#[test]
fn bounds_rejects_point_south_of_bounds() {
    let bounds = paris_center_bounds();
    assert!(!bounds.contains(&Coordinates::new(48.79, 2.35)));
}

#[test]
fn bounds_rejects_point_east_of_bounds() {
    let bounds = paris_center_bounds();
    assert!(!bounds.contains(&Coordinates::new(48.85, 2.41)));
}

#[test]
fn bounds_rejects_point_west_of_bounds() {
    let bounds = paris_center_bounds();
    assert!(!bounds.contains(&Coordinates::new(48.85, 2.29)));
}

#[test]
fn bounds_contains_point_on_north_edge() {
    let bounds = paris_center_bounds();
    // Boundary is inclusive (>= south, <= north)
    assert!(bounds.contains(&Coordinates::new(48.90, 2.35)));
}

#[test]
fn bounds_contains_point_on_south_edge() {
    let bounds = paris_center_bounds();
    assert!(bounds.contains(&Coordinates::new(48.80, 2.35)));
}

// ---------------------------------------------------------------------------
// McpToolHandler — input-validation (no network calls required)
// ---------------------------------------------------------------------------

fn make_handler() -> McpToolHandler {
    McpToolHandler::new()
}

#[tokio::test]
async fn find_nearby_stations_rejects_radius_too_large() {
    let handler = make_handler();
    let input = FindNearbyStationsInput {
        latitude: 48.8566,
        longitude: 2.3522,
        radius_meters: 6_000, // > 5 000 m limit
        limit: 10,
        availability_filter: None,
    };
    let err = handler.find_nearby_stations(input).await.unwrap_err();
    assert!(
        matches!(err, velib_mcp::Error::SearchRadiusTooLarge { .. }),
        "expected SearchRadiusTooLarge, got {err:?}"
    );
}

#[tokio::test]
async fn find_nearby_stations_rejects_limit_too_large() {
    let handler = make_handler();
    let input = FindNearbyStationsInput {
        latitude: 48.8566,
        longitude: 2.3522,
        radius_meters: 500,
        limit: 200, // > 100 limit
        availability_filter: None,
    };
    let err = handler.find_nearby_stations(input).await.unwrap_err();
    assert!(
        matches!(err, velib_mcp::Error::ResultLimitExceeded { .. }),
        "expected ResultLimitExceeded, got {err:?}"
    );
}

#[tokio::test]
async fn find_nearby_stations_rejects_coordinates_outside_paris_metro() {
    let handler = make_handler();
    // New York City — clearly outside Paris bounding box
    let input = FindNearbyStationsInput {
        latitude: 40.7128,
        longitude: -74.0060,
        radius_meters: 500,
        limit: 10,
        availability_filter: None,
    };
    let err = handler.find_nearby_stations(input).await.unwrap_err();
    assert!(
        matches!(err, velib_mcp::Error::InvalidCoordinates { .. }),
        "expected InvalidCoordinates, got {err:?}"
    );
}

#[tokio::test]
async fn search_stations_by_name_rejects_short_query() {
    let handler = make_handler();
    let input = SearchStationsByNameInput {
        query: "a".to_string(), // < 2 chars
        limit: 10,
        fuzzy: true,
    };
    let err = handler
        .search_stations_by_name(input)
        .await
        .unwrap_err();
    // The handler returns Error::Internal for this case
    assert!(
        err.to_string().to_lowercase().contains("short")
            || matches!(err, velib_mcp::Error::Internal(_)),
        "expected an error about short query, got {err:?}"
    );
}

#[tokio::test]
async fn search_stations_by_name_rejects_limit_too_large() {
    let handler = make_handler();
    let input = SearchStationsByNameInput {
        query: "chat".to_string(),
        limit: 200, // > 100
        fuzzy: true,
    };
    let err = handler
        .search_stations_by_name(input)
        .await
        .unwrap_err();
    assert!(
        matches!(err, velib_mcp::Error::ResultLimitExceeded { .. }),
        "expected ResultLimitExceeded, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// StationReference::validate — all error paths
// ---------------------------------------------------------------------------

fn base_reference() -> StationReference {
    StationReference {
        station_code: "12345".to_string(),
        name: "Test Station".to_string(),
        coordinates: Coordinates::new(48.8566, 2.3522),
        capacity: 20,
        capabilities: ServiceCapabilities::default(),
    }
}

#[test]
fn station_reference_validate_ok_for_valid_station() {
    assert!(base_reference().validate().is_ok());
}

#[test]
fn station_reference_validate_rejects_empty_code() {
    let mut r = base_reference();
    r.station_code = String::new();
    let err = r.validate().unwrap_err();
    assert!(err.contains("code"), "error should mention 'code': {err}");
}

#[test]
fn station_reference_validate_rejects_empty_name() {
    let mut r = base_reference();
    r.name = String::new();
    let err = r.validate().unwrap_err();
    assert!(err.contains("name"), "error should mention 'name': {err}");
}

#[test]
fn station_reference_validate_rejects_zero_capacity() {
    let mut r = base_reference();
    r.capacity = 0;
    assert!(r.validate().is_err());
}

#[test]
fn station_reference_validate_rejects_unreasonably_large_capacity() {
    let mut r = base_reference();
    r.capacity = 201;
    assert!(r.validate().is_err());
}

#[test]
fn station_reference_validate_rejects_non_paris_coordinates() {
    let mut r = base_reference();
    r.coordinates = Coordinates::new(51.5074, -0.1278); // London
    assert!(r.validate().is_err());
}

// ---------------------------------------------------------------------------
// BikeAvailability — saturation on total()
// ---------------------------------------------------------------------------

#[test]
fn bike_availability_total_saturates_at_u16_max() {
    // Both at u16::MAX — saturating_add should not panic or wrap
    let bikes = BikeAvailability::new(u16::MAX, u16::MAX);
    assert_eq!(bikes.total(), u16::MAX);
}

#[test]
fn bike_availability_has_bikes_false_when_zero() {
    let bikes = BikeAvailability::new(0, 0);
    assert!(!bikes.has_bikes());
    assert!(!bikes.has_mechanical());
    assert!(!bikes.has_electric());
}

// ---------------------------------------------------------------------------
// DataFreshness — boundary values for age thresholds
// ---------------------------------------------------------------------------

#[test]
fn data_freshness_boundary_at_exactly_10_minutes() {
    // < 10 → Fresh; == 10 → Recent
    assert_eq!(DataFreshness::from_age(9.99), DataFreshness::Fresh);
    assert_eq!(DataFreshness::from_age(10.0), DataFreshness::Recent);
}

#[test]
fn data_freshness_boundary_at_exactly_30_minutes() {
    assert_eq!(DataFreshness::from_age(29.99), DataFreshness::Recent);
    assert_eq!(DataFreshness::from_age(30.0), DataFreshness::Stale);
}

#[test]
fn data_freshness_boundary_at_exactly_120_minutes() {
    assert_eq!(DataFreshness::from_age(119.99), DataFreshness::Stale);
    assert_eq!(DataFreshness::from_age(120.0), DataFreshness::VeryStale);
}

// ---------------------------------------------------------------------------
// RealTimeStatus::new — freshness computed from last_update
// ---------------------------------------------------------------------------

#[test]
fn real_time_status_computes_freshness_from_last_update() {
    // A last_update 5 minutes ago → Fresh
    let five_min_ago = Utc::now() - Duration::minutes(5);
    let bikes = BikeAvailability::new(3, 2);
    let status = RealTimeStatus::new(bikes, 10, StationStatus::Open, five_min_ago);
    assert_eq!(status.data_freshness, DataFreshness::Fresh);

    // A last_update 45 minutes ago → Stale
    let forty_five_min_ago = Utc::now() - Duration::minutes(45);
    let status2 = RealTimeStatus::new(bikes, 10, StationStatus::Open, forty_five_min_ago);
    assert_eq!(status2.data_freshness, DataFreshness::Stale);
}

// ---------------------------------------------------------------------------
// AvailabilityFilter — default values via serde deserialization
// ---------------------------------------------------------------------------

#[test]
fn availability_filter_default_excludes_out_of_service() {
    let filter: AvailabilityFilter = serde_json::from_str("{}").unwrap();
    assert!(filter.exclude_out_of_service);
    assert!(filter.min_bikes.is_none());
    assert!(filter.min_docks.is_none());
    assert!(filter.bike_type.is_none());
}
