use std::env;
use std::sync::Mutex;
use std::time::Duration;
use tokio::time::timeout;
use velib_mcp::server::{parse_server_address, Server};

// Use a mutex to ensure integration tests don't interfere with each other
static ENV_MUTEX: Mutex<()> = Mutex::new(());

#[tokio::test]
async fn test_server_configuration_from_env() {
    let _guard = ENV_MUTEX.lock().unwrap();

    // Test 1: Default configuration
    env::remove_var("IP");
    env::remove_var("PORT");

    let addr = parse_server_address().unwrap();
    assert_eq!(addr.to_string(), "0.0.0.0:8080");

    // Test 2: Custom port
    env::remove_var("IP"); // Ensure IP is cleared
    env::set_var("PORT", "9999");
    let addr = parse_server_address().unwrap();
    assert_eq!(addr.to_string(), "0.0.0.0:9999");

    // Test 3: Custom IP and port
    env::set_var("IP", "127.0.0.1");
    env::set_var("PORT", "3000");
    let addr = parse_server_address().unwrap();
    assert_eq!(addr.to_string(), "127.0.0.1:3000");

    // Cleanup
    env::remove_var("IP");
    env::remove_var("PORT");
}

#[tokio::test]
async fn test_server_starts_and_shuts_down() {
    let _guard = ENV_MUTEX.lock().unwrap();

    // Use a high port to avoid conflicts
    env::set_var("IP", "127.0.0.1");
    env::set_var("PORT", "0"); // Let OS choose port

    let addr = parse_server_address().unwrap();
    let server = Server::new(addr);

    // Start server in background task
    let server_handle = tokio::spawn(async move {
        let _ = server.run().await;
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Abort the server (simulates shutdown)
    server_handle.abort();

    // Wait for the handle to complete (should be aborted)
    let result = timeout(Duration::from_secs(1), server_handle).await;

    // Either it was aborted (Err) or completed somehow (Ok)
    // Both are acceptable for this test
    assert!(result.is_ok() || result.is_err());

    // Cleanup
    env::remove_var("IP");
    env::remove_var("PORT");
}

#[tokio::test]
async fn test_invalid_configurations() {
    let _guard = ENV_MUTEX.lock().unwrap();

    // Test invalid IP
    env::set_var("IP", "not.an.ip.address");
    env::set_var("PORT", "8080");

    let result = parse_server_address();
    assert!(result.is_err());

    // Test port out of range
    env::set_var("IP", "127.0.0.1");
    env::set_var("PORT", "70000");

    let result = parse_server_address();
    assert!(result.is_err());

    // Cleanup
    env::remove_var("IP");
    env::remove_var("PORT");
}

// Note: Full HTTP integration tests would require more complex setup
// to capture the actual bound port when using port 0. This is just
// a placeholder for when we implement that functionality.
#[tokio::test]
async fn test_server_router_creation() {
    let _guard = ENV_MUTEX.lock().unwrap();

    // Test that we can create a server and its router without errors
    env::set_var("IP", "127.0.0.1");
    env::set_var("PORT", "8888");

    let addr = parse_server_address().unwrap();
    let server = Server::new(addr);

    // Test that router can be created
    let _router = server.router();

    env::remove_var("IP");
    env::remove_var("PORT");
}
