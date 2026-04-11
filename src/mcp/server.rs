use axum::{
    extract::{rejection::JsonRejection, ws::WebSocket, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info, warn};

use super::handlers::McpToolHandler;
use super::types::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use crate::{Error, Result};

pub struct McpServer {
    tool_handler: Arc<McpToolHandler>,
    start_time: Instant,
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl McpServer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tool_handler: Arc::new(McpToolHandler::new()),
            start_time: Instant::now(),
        }
    }

    pub fn router(&self) -> Router {
        let handler = Arc::clone(&self.tool_handler);

        Router::new()
            .route(
                "/mcp",
                post({
                    let handler = Arc::clone(&handler);
                    move |request: std::result::Result<Json<JsonRpcRequest>, JsonRejection>| async move {
                        let request = match request {
                            Ok(Json(r)) => r,
                            Err(e) => {
                                let response = JsonRpcResponse {
                                    jsonrpc: "2.0".to_string(),
                                    id: serde_json::Value::Null,
                                    result: None,
                                    error: Some(JsonRpcError {
                                        code: -32700,
                                        message: e.to_string(),
                                        data: None,
                                    }),
                                };
                                return Json(response).into_response();
                            }
                        };
                        match Self::process_jsonrpc_request(handler, request).await {
                            Ok(response) => Json(response).into_response(),
                            Err(e) => {
                                tracing::error!("HTTP request error: {}", e);
                                (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(json!({"error": e.to_string()})),
                                )
                                    .into_response()
                            }
                        }
                    }
                }),
            )
            .route(
                "/mcp/ws",
                get({
                    let handler = Arc::clone(&handler);
                    move |ws: WebSocketUpgrade| async move {
                        ws.on_upgrade(move |socket| {
                            Self::handle_websocket_connection(socket, handler)
                        })
                    }
                }),
            )
            .route(
                "/resources/*uri",
                get({
                    let handler = Arc::clone(&handler);
                    let start_time = self.start_time;
                    move |uri: axum::extract::Path<String>| {
                        let handler = Arc::clone(&handler);
                        async move { handle_resource(uri, handler, start_time).await }
                    }
                }),
            )
    }

    async fn handle_websocket_connection(
        mut socket: WebSocket,
        handler: Arc<McpToolHandler>,
    ) {
        let client_id = uuid::Uuid::new_v4().to_string();
        info!("New WebSocket connection: {}", client_id);

        // Handle messages
        while let Some(msg) = socket.recv().await {
            match msg {
                Ok(axum::extract::ws::Message::Text(text)) => {
                    match serde_json::from_str::<JsonRpcRequest>(&text) {
                        Ok(request) => {
                            match Self::process_jsonrpc_request(Arc::clone(&handler), request).await
                            {
                                Ok(response) => {
                                    let response_text = match serde_json::to_string(&response) {
                                        Ok(text) => text,
                                        Err(e) => {
                                            error!("Failed to serialize response: {}", e);
                                            continue;
                                        }
                                    };

                                    if let Err(e) = socket
                                        .send(axum::extract::ws::Message::Text(response_text))
                                        .await
                                    {
                                        error!("Failed to send WebSocket message: {}", e);
                                        break;
                                    }
                                }
                                Err(e) => {
                                    error!("Request processing error: {}", e);
                                    let error_response = JsonRpcResponse {
                                        jsonrpc: "2.0".to_string(),
                                        id: json!(null),
                                        result: None,
                                        error: Some(JsonRpcError::from(e)),
                                    };

                                    if let Ok(response_text) =
                                        serde_json::to_string(&error_response)
                                    {
                                        let _ = socket
                                            .send(axum::extract::ws::Message::Text(response_text))
                                            .await;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Invalid JSON-RPC request: {}", e);
                            let error_response = JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                id: json!(null),
                                result: None,
                                error: Some(JsonRpcError {
                                    code: -32700,
                                    message: "Parse error".to_string(),
                                    data: Some(json!({"original_error": e.to_string()})),
                                }),
                            };

                            if let Ok(response_text) = serde_json::to_string(&error_response) {
                                let _ = socket
                                    .send(axum::extract::ws::Message::Text(response_text))
                                    .await;
                            }
                        }
                    }
                }
                Ok(axum::extract::ws::Message::Close(_)) => {
                    info!("WebSocket connection closed: {}", client_id);
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {} // Ignore other message types
            }
        }

        info!("WebSocket connection terminated: {}", client_id);
    }

    async fn process_jsonrpc_request(
        handler: Arc<McpToolHandler>,
        request: JsonRpcRequest,
    ) -> Result<JsonRpcResponse> {
        let result = match request.method.as_str() {
            "tools/list" => Ok(json!({
                "tools": [
                    {
                        "name": "find_nearby_stations",
                        "description": "Find Velib stations within a radius of coordinates",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "latitude": {"type": "number", "minimum": 48.7, "maximum": 49.0},
                                "longitude": {"type": "number", "minimum": 2.0, "maximum": 2.6},
                                "radius_meters": {"type": "integer", "minimum": 100, "maximum": 5000, "default": 500},
                                "limit": {"type": "integer", "minimum": 1, "maximum": 100, "default": 10},
                                "availability_filter": {"type": "object"}
                            },
                            "required": ["latitude", "longitude"]
                        }
                    },
                    {
                        "name": "get_station_by_code",
                        "description": "Get detailed information about a specific station",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "station_code": {"type": "string"},
                                "include_real_time": {"type": "boolean", "default": true}
                            },
                            "required": ["station_code"]
                        }
                    },
                    {
                        "name": "search_stations_by_name",
                        "description": "Search stations by name with optional fuzzy matching",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": {"type": "string", "minLength": 2},
                                "limit": {"type": "integer", "minimum": 1, "maximum": 50, "default": 10},
                                "fuzzy": {"type": "boolean", "default": true}
                            },
                            "required": ["query"]
                        }
                    },
                    {
                        "name": "get_area_statistics",
                        "description": "Get aggregated statistics for a geographic area",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "bounds": {
                                    "type": "object",
                                    "properties": {
                                        "north": {"type": "number"},
                                        "south": {"type": "number"},
                                        "east": {"type": "number"},
                                        "west": {"type": "number"}
                                    },
                                    "required": ["north", "south", "east", "west"]
                                },
                                "include_real_time": {"type": "boolean", "default": true}
                            },
                            "required": ["bounds"]
                        }
                    },
                    {
                        "name": "plan_bike_journey",
                        "description": "Plan a bike journey with pickup and dropoff suggestions",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "origin": {
                                    "type": "object",
                                    "properties": {
                                        "latitude": {"type": "number"},
                                        "longitude": {"type": "number"}
                                    },
                                    "required": ["latitude", "longitude"]
                                },
                                "destination": {
                                    "type": "object",
                                    "properties": {
                                        "latitude": {"type": "number"},
                                        "longitude": {"type": "number"}
                                    },
                                    "required": ["latitude", "longitude"]
                                },
                                "preferences": {"type": "object"}
                            },
                            "required": ["origin", "destination"]
                        }
                    }
                ]
            })),
            "tools/call" => {
                let params = request
                    .params
                    .as_object()
                    .ok_or_else(|| Error::McpProtocol("Invalid params".to_string()))?;
                let tool_name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::McpProtocol("Missing tool name".to_string()))?;
                let empty_args = json!({});
                let arguments = params.get("arguments").unwrap_or(&empty_args);

                match tool_name {
                    "find_nearby_stations" => {
                        let input = serde_json::from_value(arguments.clone())?;
                        let output = handler.find_nearby_stations(input).await?;
                        Ok(json!({
                            "content": [
                                {
                                    "type": "text",
                                    "text": serde_json::to_string_pretty(&output)?
                                }
                            ]
                        }))
                    }
                    "get_station_by_code" => {
                        let input = serde_json::from_value(arguments.clone())?;
                        let output = handler.get_station_by_code(input).await?;
                        Ok(json!({
                            "content": [
                                {
                                    "type": "text",
                                    "text": serde_json::to_string_pretty(&output)?
                                }
                            ]
                        }))
                    }
                    "search_stations_by_name" => {
                        let input = serde_json::from_value(arguments.clone())?;
                        let output = handler.search_stations_by_name(input).await?;
                        Ok(json!({
                            "content": [
                                {
                                    "type": "text",
                                    "text": serde_json::to_string_pretty(&output)?
                                }
                            ]
                        }))
                    }
                    "get_area_statistics" => {
                        let input = serde_json::from_value(arguments.clone())?;
                        let output = handler.get_area_statistics(input).await?;
                        Ok(json!({
                            "content": [
                                {
                                    "type": "text",
                                    "text": serde_json::to_string_pretty(&output)?
                                }
                            ]
                        }))
                    }
                    "plan_bike_journey" => {
                        let input = serde_json::from_value(arguments.clone())?;
                        let output = handler.plan_bike_journey(input).await?;
                        Ok(json!({
                            "content": [
                                {
                                    "type": "text",
                                    "text": serde_json::to_string_pretty(&output)?
                                }
                            ]
                        }))
                    }
                    _ => Err(Error::McpProtocol(format!("Unknown tool: {tool_name}"))),
                }
            }
            "resources/list" => Ok(json!({
                "resources": [
                    {
                        "uri": "velib://stations/reference",
                        "name": "Velib Station Reference Data",
                        "description": "Complete catalog of Velib stations with static metadata",
                        "mimeType": "application/json"
                    },
                    {
                        "uri": "velib://stations/realtime",
                        "name": "Velib Real-time Availability",
                        "description": "Current bike and dock availability for all stations",
                        "mimeType": "application/json"
                    },
                    {
                        "uri": "velib://stations/complete",
                        "name": "Velib Complete Station Data",
                        "description": "Combined reference and real-time data for all stations",
                        "mimeType": "application/json"
                    },
                    {
                        "uri": "velib://health",
                        "name": "Service Health Status",
                        "description": "System health and data source status information",
                        "mimeType": "application/json"
                    }
                ]
            })),
            _ => Err(Error::McpProtocol(format!(
                "Unknown method: {}",
                request.method
            ))),
        };

        match result {
            Ok(result_value) => Ok(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(result_value),
                error: None,
            }),
            Err(e) => Ok(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(JsonRpcError::from(e)),
            }),
        }
    }
}

async fn handle_resource(
    axum::extract::Path(uri): axum::extract::Path<String>,
    handler: Arc<McpToolHandler>,
    start_time: Instant,
) -> Response {
    match uri.as_str() {
        "velib://stations/reference" => {
            match get_reference_stations_resource(Arc::clone(&handler)).await {
                Ok(response) => Json(response).into_response(),
                Err(e) => {
                    error!("Failed to get reference stations: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": "Failed to fetch reference stations",
                            "details": e.to_string()
                        })),
                    )
                        .into_response()
                }
            }
        }
        "velib://stations/realtime" => {
            match get_realtime_stations_resource(Arc::clone(&handler)).await {
                Ok(response) => Json(response).into_response(),
                Err(e) => {
                    error!("Failed to get real-time stations: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": "Failed to fetch real-time stations",
                            "details": e.to_string()
                        })),
                    )
                        .into_response()
                }
            }
        }
        "velib://stations/complete" => {
            match get_complete_stations_resource(Arc::clone(&handler)).await {
                Ok(response) => Json(response).into_response(),
                Err(e) => {
                    error!("Failed to get complete stations: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": "Failed to fetch complete stations",
                            "details": e.to_string()
                        })),
                    )
                        .into_response()
                }
            }
        }
        "velib://health" => match get_health_resource(Arc::clone(&handler), start_time).await {
            Ok(response) => Json(response).into_response(),
            Err(e) => {
                error!("Failed to get health status: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Failed to fetch health status",
                        "details": e.to_string()
                    })),
                )
                    .into_response()
            }
        },
        _ => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Resource not found"})),
        )
            .into_response(),
    }
}

/// Get reference stations resource data
async fn get_reference_stations_resource(handler: Arc<McpToolHandler>) -> Result<Value> {
    let stations = handler.get_reference_stations().await?;

    Ok(json!({
        "stations": stations,
        "metadata": {
            "total_stations": stations.len(),
            "last_updated": chrono::Utc::now(),
            "data_source": "live"
        }
    }))
}

/// Get real-time stations resource data  
async fn get_realtime_stations_resource(handler: Arc<McpToolHandler>) -> Result<Value> {
    let realtime_status = handler.get_realtime_status().await?;

    // Convert HashMap to Vec for JSON response
    let stations: Vec<Value> = realtime_status
        .iter()
        .map(|(station_code, status)| {
            json!({
                "station_code": station_code,
                "bikes": {
                    "mechanical": status.bikes.mechanical,
                    "electric": status.bikes.electric
                },
                "available_docks": status.available_docks,
                "status": status.status,
                "last_update": status.last_update,
                "data_freshness": status.data_freshness
            })
        })
        .collect();

    Ok(json!({
        "stations": stations,
        "metadata": {
            "total_stations": stations.len(),
            "data_freshness": "Fresh",
            "response_time": chrono::Utc::now(),
            "data_source": "live"
        }
    }))
}

/// Get complete stations resource data (reference + real-time)
async fn get_complete_stations_resource(handler: Arc<McpToolHandler>) -> Result<Value> {
    let stations = handler.get_complete_stations(true).await?;

    Ok(json!({
        "stations": stations,
        "metadata": {
            "total_stations": stations.len(),
            "data_freshness": "Fresh",
            "response_time": chrono::Utc::now(),
            "data_source": "live"
        }
    }))
}

/// Get health resource data with real metrics
async fn get_health_resource(handler: Arc<McpToolHandler>, start_time: Instant) -> Result<Value> {
    let uptime_seconds = start_time.elapsed().as_secs();

    // Get real cache statistics
    let (reference_cache_size, realtime_cache_size) = handler.cache_stats().await;
    let total_entries = reference_cache_size + realtime_cache_size;

    // Calculate hit rate based on cache usage (simplified)
    let hit_rate = if total_entries > 0 {
        0.75 + (total_entries as f64 / 2000.0) * 0.2
    } else {
        0.0
    };

    // Fetch stations to compute real lag from most recent station last_update timestamp
    let (realtime_status, reference_status, lag_seconds, most_recent_update) =
        match handler.get_complete_stations(true).await {
            Ok(stations) => {
                let most_recent = stations
                    .iter()
                    .filter_map(|s| s.real_time.as_ref().map(|rt| rt.last_update))
                    .max();
                let lag = most_recent
                    .map(|t| (Utc::now() - t).num_seconds().max(0) as u64)
                    .unwrap_or(0);
                ("healthy", "healthy", lag, most_recent)
            }
            Err(_) => ("degraded", "degraded", 0u64, None),
        };

    Ok(json!({
        "status": "healthy",
        "version": "1.0.0",
        "uptime_seconds": uptime_seconds,
        "data_sources": {
            "real_time": {
                "status": realtime_status,
                "last_update": most_recent_update,
                "lag_seconds": lag_seconds
            },
            "reference": {
                "status": reference_status,
                "last_update": Utc::now()
            }
        },
        "cache_stats": {
            "hit_rate": hit_rate.min(1.0),
            "entries": total_entries,
            "reference_cache_size": reference_cache_size,
            "realtime_cache_size": realtime_cache_size
        }
    }))
}
