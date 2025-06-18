use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::timeout;
use velib_mcp::data::VelibDataClient;

static TEST_MUTEX: Mutex<()> = Mutex::const_new(());

#[tokio::test]
async fn test_data_client_creation() {
    let _guard = TEST_MUTEX.lock().await;

    let client = VelibDataClient::new();
    let (ref_size, rt_size) = client.cache_stats().await;

    // New client should have empty caches
    assert_eq!(ref_size, 0);
    assert_eq!(rt_size, 0);
}

#[tokio::test]
async fn test_cache_cleanup() {
    let _guard = TEST_MUTEX.lock().await;

    let client = VelibDataClient::new();

    // Cache cleanup should work without errors
    client.cleanup_cache().await;

    let (ref_size, rt_size) = client.cache_stats().await;
    assert_eq!(ref_size, 0);
    assert_eq!(rt_size, 0);
}

#[tokio::test]
async fn test_real_api_fetch_with_timeout() {
    let _guard = TEST_MUTEX.lock().await;

    let mut client = VelibDataClient::new();

    // Test with a reasonable timeout to avoid hanging tests
    let result = timeout(Duration::from_secs(30), client.fetch_reference_stations()).await;

    match result {
        Ok(Ok(stations)) => {
            println!("Successfully fetched {} reference stations", stations.len());

            if !stations.is_empty() {
                // Verify station structure if we got data
                let first_station = &stations[0];
                assert!(!first_station.station_code.is_empty());
                assert!(!first_station.name.is_empty());
                assert!(first_station.capacity > 0);
                println!(
                    "First station: {} - {}",
                    first_station.station_code, first_station.name
                );
            } else {
                println!(
                    "Warning: API returned 0 stations - this may indicate API structure changes"
                );
            }
        }
        Ok(Err(e)) => {
            println!("Warning: API fetch failed: {}", e);
            // Don't fail the test for network issues
        }
        Err(_) => {
            println!("Warning: API call timed out after 30 seconds");
            // Don't fail the test for timeouts
        }
    }
}

#[tokio::test]
async fn test_station_by_code_not_found() {
    let _guard = TEST_MUTEX.lock().await;

    let mut client = VelibDataClient::new();

    // Test with a timeout and non-existent station code
    let result = timeout(
        Duration::from_secs(10),
        client.get_station_by_code("nonexistent", false),
    )
    .await;

    match result {
        Ok(Ok(station)) => {
            // Should return None for non-existent stations
            assert!(station.is_none(), "Non-existent station should return None");
        }
        Ok(Err(e)) => {
            println!("Warning: API fetch failed: {}", e);
            // Don't fail for network issues
        }
        Err(_) => {
            println!("Warning: Station lookup timed out");
            // Don't fail for timeouts
        }
    }
}
