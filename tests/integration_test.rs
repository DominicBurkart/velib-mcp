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
async fn test_claude_installation_workflow() {
    // Test that we can build the server (simulating `cargo install`)
    let build_output = Command::new("cargo")
        .args(["build"])
        .output()
        .expect("Failed to build server");

    assert!(build_output.status.success(), "Server build failed");

    // Test that the binary runs with custom port
    let port = find_available_port();
    let mut server = Command::new("./target/debug/velib-mcp")
        .env("IP", "127.0.0.1")
        .env("PORT", port.to_string())
        .spawn()
        .expect("Failed to start server");

    // Give server time to start
    sleep(Duration::from_secs(3)).await;

    // Test that Claude can communicate with the server
    let client = reqwest::Client::new();

    // Test health endpoint
    let health_response = client
        .get(format!("http://127.0.0.1:{port}/health"))
        .timeout(Duration::from_secs(5))
        .send()
        .await;

    assert!(health_response.is_ok(), "Health check failed");

    // Test MCP protocol endpoint
    let mcp_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let mcp_response = client
        .post(format!("http://127.0.0.1:{port}/mcp"))
        .json(&mcp_request)
        .timeout(Duration::from_secs(5))
        .send()
        .await;

    // Clean up
    server.kill().expect("Failed to kill server");
    let _ = server.wait();
    // Verify MCP response
    assert!(mcp_response.is_ok(), "MCP request failed");
    let mcp_response = mcp_response.unwrap();
    assert_eq!(mcp_response.status(), 200);

    let body: serde_json::Value = mcp_response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["jsonrpc"], "2.0");
    assert!(body["result"]["tools"].is_array());
}

#[tokio::test]
async fn test_velib_tool_functionality() {
    // Start server
    let port = find_available_port();
    let mut server = Command::new("./target/debug/velib-mcp")
        .env("IP", "127.0.0.1")
        .env("PORT", port.to_string())
        .spawn()
        .expect("Failed to start server");

    sleep(Duration::from_secs(3)).await;

    let client = reqwest::Client::new();

    // Test find_nearby_stations tool
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "find_nearby_stations",
            "arguments": {
                "latitude": 48.8566,
                "longitude": 2.3522,
                "radius_meters": 500,
                "limit": 5
            }
        }
    });

    let response = client
        .post(format!("http://127.0.0.1:{port}/mcp"))
        .json(&request)
        .timeout(Duration::from_secs(10))
        .send()
        .await;

    // Clean up
    server.kill().expect("Failed to kill server");
    let _ = server.wait();
    // Verify response
    assert!(response.is_ok(), "Tool call failed");
    let response = response.unwrap();
    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["jsonrpc"], "2.0");
    assert!(body["result"]["content"].is_array());
}
