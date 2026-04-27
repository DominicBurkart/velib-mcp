use velib_mcp::{parse_server_address, Server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let addr = parse_server_address()?;

    let server = Server::new(addr);
    server.run().await?;

    Ok(())
}
