use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Rate limited by API (HTTP 429){}", match retry_after_seconds {
        Some(seconds) => format!(": retry after {}s", seconds),
        None => String::new(),
    })]
    RateLimited { retry_after_seconds: Option<u64> },

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid coordinates: latitude {latitude}, longitude {longitude}")]
    InvalidCoordinates { latitude: f64, longitude: f64 },

    #[error("Coordinates outside service area: {distance_km:.1}km from Paris (max: 50km)")]
    OutsideServiceArea { distance_km: f64 },
    #[error("Search radius too large: {radius}m (max: {max}m)")]
    SearchRadiusTooLarge { radius: u32, max: u32 },

    #[error("Result limit exceeded: {limit} (max: {max})")]
    ResultLimitExceeded { limit: u16, max: u16 },

    #[error("Station not found: {station_code}")]
    StationNotFound { station_code: String },

    #[error("MCP protocol error: {0}")]
    McpProtocol(String),

    #[error("Data validation error: {0}")]
    Validation(String),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl Error {
    /// Get MCP-compatible error code
    pub fn mcp_error_code(&self) -> i32 {
        match self {
            Error::Http(_) => -32001,
            Error::RateLimited { .. } => -32001, // Server error (rate limit)
            Error::Json(_) => -32700,            // Parse error
            Error::InvalidCoordinates { .. } => -32602, // Invalid params
            Error::OutsideServiceArea { .. } => -32602, // Invalid params
            Error::SearchRadiusTooLarge { .. } => -32602, // Invalid params
            Error::ResultLimitExceeded { .. } => -32602, // Invalid params
            Error::StationNotFound { .. } => -32600, // Invalid request
            Error::McpProtocol(_) => -32603,     // Internal error
            Error::Validation(_) => -32602,      // Invalid params
            Error::Cache(_) => -32603,           // Internal error
            Error::Internal(_) => -32603,        // Internal error
        }
    }

    /// Get error type string for structured error data
    pub fn error_type(&self) -> &'static str {
        match self {
            Error::Http(_) => "http_error",
            Error::RateLimited { .. } => "rate_limited",
            Error::Json(_) => "json_error",
            Error::InvalidCoordinates { .. } => "invalid_coordinates",
            Error::OutsideServiceArea { .. } => "outside_service_area",
            Error::SearchRadiusTooLarge { .. } => "search_radius_too_large",
            Error::ResultLimitExceeded { .. } => "result_limit_exceeded",
            Error::StationNotFound { .. } => "station_not_found",
            Error::McpProtocol(_) => "mcp_protocol_error",
            Error::Validation(_) => "validation_error",
            Error::Cache(_) => "cache_error",
            Error::Internal(_) => "internal_error",
        }
    }
}
