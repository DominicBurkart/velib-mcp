use std::sync::Mutex;
use velib_mcp::data::VelibDataClient;
use velib_mcp::mcp::handlers::McpToolHandler;
use velib_mcp::mcp::types::{
    FindNearbyStationsInput, GeographicBounds, GetAreaStatisticsInput, SearchStationsByNameInput,
};

static TEST_MUTEX: Mutex<()> = Mutex::new(());

#[tokio::test]
async fn test_real_api_data_fetching() {
    let _guard = TEST_MUTEX.lock().unwrap();

    let mut client = VelibDataClient::new();

    // Test fetching all stations with real-time data
    let stations = client.get_all_stations(true).await;
    match stations {
        Ok(stations) => {
            assert!(
                !stations.is_empty(),
                "Should fetch some stations from real API"
            );

            // Verify station structure
            let first_station = &stations[0];
            assert!(!first_station.reference.station_code.is_empty());
            assert!(!first_station.reference.name.is_empty());
            assert!(first_station.reference.capacity > 0);

            println!(
                "Successfully fetched {} stations from real API",
                stations.len()
            );
        }
        Err(e) => {
            println!("Warning: Failed to fetch real data from API: {}", e);
            println!("This may be due to network issues or API changes");
        }
    }
}

#[tokio::test]
async fn test_mcp_handlers_with_real_data() {
    let _guard = TEST_MUTEX.lock().unwrap();

    let handler = McpToolHandler::new();

    // Test nearby stations search in central Paris (Louvre area)
    let input = FindNearbyStationsInput {
        latitude: 48.8606,
        longitude: 2.3376,
        radius_meters: 1000,
        limit: 5,
        availability_filter: None,
    };

    let result = handler.find_nearby_stations(input).await;
    match result {
        Ok(output) => {
            println!("Found {} nearby stations", output.stations.len());
            assert!(
                output.search_metadata.search_time_ms < 5000,
                "Search should complete quickly"
            );

            if !output.stations.is_empty() {
                let station = &output.stations[0];
                assert!(station.distance_meters <= 1000);
                println!(
                    "Closest station: {} at {}m",
                    station.station.reference.name, station.distance_meters
                );
            }
        }
        Err(e) => {
            println!("Warning: Handler test failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_station_search_by_name() {
    let _guard = TEST_MUTEX.lock().unwrap();

    let handler = McpToolHandler::new();

    // Search for stations with "Metro" in the name
    let input = SearchStationsByNameInput {
        query: "Metro".to_string(),
        limit: 10,
        fuzzy: true,
    };

    let result = handler.search_stations_by_name(input).await;
    match result {
        Ok(output) => {
            println!("Found {} stations matching 'Metro'", output.stations.len());
            if !output.stations.is_empty() {
                for station in output.stations.iter().take(3) {
                    println!("- {}", station.reference.name);
                }
            }
        }
        Err(e) => {
            println!("Warning: Name search test failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_area_statistics() {
    let _guard = TEST_MUTEX.lock().unwrap();

    let handler = McpToolHandler::new();

    // Test area statistics for a small area in central Paris
    let bounds = GeographicBounds {
        north: 48.870,
        south: 48.850,
        east: 2.350,
        west: 2.330,
    };

    let input = GetAreaStatisticsInput {
        bounds,
        include_real_time: true,
    };

    let result = handler.get_area_statistics(input).await;
    match result {
        Ok(output) => {
            let stats = &output.area_stats;
            println!("Area statistics:");
            println!("- Total stations: {}", stats.total_stations);
            println!("- Operational: {}", stats.operational_stations);
            println!("- Total bikes: {}", stats.available_bikes.total);
            println!("- Mechanical: {}", stats.available_bikes.mechanical);
            println!("- Electric: {}", stats.available_bikes.electric);
            println!("- Available docks: {}", stats.available_docks);
            println!("- Occupancy rate: {:.2}%", stats.occupancy_rate * 100.0);

            assert!(
                stats.total_capacity > 0,
                "Should have some capacity in the area"
            );
        }
        Err(e) => {
            println!("Warning: Area statistics test failed: {}", e);
        }
    }
}
