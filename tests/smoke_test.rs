use std::net::TcpListener;
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

/// Find an available port by binding to a random port and returning it
fn find_available_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("Failed to bind to ephemeral port")
        .local_addr()
        .expect("Failed to get local address")
        .port()
}

#[tokio::test]
async fn test_server_starts_and_responds() {
    // Build the server
    let build_output = Command::new("cargo")
        .args(["build", "--release"])
        .output()
        .expect("Failed to build server");
    
    assert!(build_output.status.success(), "Server build failed");
    
    // Start the server in background
    let port = find_available_port();
    let mut server = Command::new("./target/release/velib-mcp")
        .env("IP", "127.0.0.1")
        .env("PORT", port.to_string())
        .spawn()
        .expect("Failed to start server");
    
    // Give server time to start
    sleep(Duration::from_secs(2)).await;
    
    // Test health endpoint
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{}/health", port))
        .timeout(Duration::from_secs(5))
        .send()
        .await;
    
    // Clean up
    server.kill().expect("Failed to kill server");
    let _ = server.wait();
    
    // Verify response
    assert!(response.is_ok(), "Health check failed: {:?}", response.err());
    let response = response.unwrap();
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "healthy");
    assert_eq!(body["service"], "velib-mcp");
}

#[tokio::test]
async fn test_mcp_tools_endpoint() {
    // Start the server
    let port = find_available_port();
    let mut server = Command::new("./target/release/velib-mcp")
        .env("IP", "127.0.0.1")
        .env("PORT", port.to_string())
        .spawn()
        .expect("Failed to start server");
    
    // Give server time to start
    sleep(Duration::from_secs(2)).await;
    
    // Test MCP tools/list endpoint
    let client = reqwest::Client::new();
    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });
    
    let response = client
        .post(format!("http://127.0.0.1:{}/mcp", port))
        .json(&request_body)
        .timeout(Duration::from_secs(5))
        .send()
        .await;
    
    // Clean up
    server.kill().expect("Failed to kill server");
    let _ = server.wait();
    
    // Verify response
    assert!(response.is_ok(), "MCP tools/list failed: {:?}", response.err());
    let response = response.unwrap();
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], 1);
    assert!(body["result"]["tools"].is_array());
    
    // Verify expected tools are present
    let tools = body["result"]["tools"].as_array().unwrap();
    let tool_names: Vec<String> = tools
        .iter()
        .map(|tool| tool["name"].as_str().unwrap().to_string())
        .collect();
    
    assert!(tool_names.contains(&"find_nearby_stations".to_string()));
    assert!(tool_names.contains(&"get_station_by_code".to_string()));
    assert!(tool_names.contains(&"search_stations_by_name".to_string()));
}