use velib_mcp::server::{parse_server_address, Server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "velib_mcp=info,tower_http=debug".into()),
        )
        .init();

    // Parse address from environment or use defaults
    let addr = parse_server_address()
        .expect("Failed to parse server address from IP and PORT environment variables");

    // Create and run server
    let server = Server::new(addr);
    server.run().await?;

    Ok(())
}
