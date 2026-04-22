use velib_mcp::{parse_server_address, Server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing with a sensible default if RUST_LOG is not set
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Parse server address from environment variables
    let addr = parse_server_address()?;

    // Create and run server
    let server = Server::new(addr);
    server.run().await?;

    Ok(())
}
