pub mod config;

use axum::Router;
use std::net::SocketAddr;
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::mcp::server::McpServer;
pub use config::parse_server_address;

pub struct Server {
    mcp_server: McpServer,
    addr: SocketAddr,
}

impl Server {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            mcp_server: McpServer::new(),
            addr,
        }
    }

    pub fn router(&self) -> Router {
        self.mcp_server.router().layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()), // For development - should be more restrictive in production
        )
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let app = self.router();

        info!("Starting Velib MCP server on {}", self.addr);

        let listener = tokio::net::TcpListener::bind(self.addr).await?;

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        Ok(())
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down gracefully");
        },
        _ = terminate => {
            info!("Received SIGTERM, shutting down gracefully");
        },
    }
}
