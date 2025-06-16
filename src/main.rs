use velib_mcp::{parse_server_address, Server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Parse server address from environment variables
    let addr = parse_server_address()
        .expect("Failed to parse server address from IP and PORT environment variables");

    // Create and run server
    let server = Server::new(addr);
    server.run().await?;

    Ok(())
}
