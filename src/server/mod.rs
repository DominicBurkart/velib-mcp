pub mod config;

pub use config::parse_server_address;

use axum::{response::Json, routing::get, Router};
use serde_json::{json, Value};
use std::net::SocketAddr;
use tracing::info;

use crate::mcp::McpServer;

pub struct Server {
    addr: SocketAddr,
}

impl Server {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }

    pub fn router(&self) -> Router {
        let mcp_server = McpServer::new();

        Router::new()
            .route("/health", get(health_check))
            .merge(mcp_server.router())
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let app = self.router();

        info!("Starting server on {}", self.addr);

        let listener = tokio::net::TcpListener::bind(self.addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now(),
        "service": "velib-mcp"
    }))
}
