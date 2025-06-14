use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Station not found: {station_code}")]
    StationNotFound { station_code: String },

    #[error("Invalid coordinates: lat={latitude}, lon={longitude}")]
    InvalidCoordinates { latitude: f64, longitude: f64 },

    #[error("Real-time data unavailable")]
    RealTimeUnavailable,

    #[error("Search radius too large: {radius} meters (max: {max})")]
    SearchRadiusTooLarge { radius: u32, max: u32 },

    #[error("Result limit exceeded: {limit} (max: {max})")]
    ResultLimitExceeded { limit: u16, max: u16 },

    #[error("Capacity overflow for station {station_code}")]
    CapacityOverflow { station_code: String },

    #[error("Dock count mismatch for station {station_code}")]
    DockCountMismatch { station_code: String },

    #[error("MCP protocol error: {0}")]
    McpProtocol(String),

    #[error("HTTP error: {0}")]
    Http(#[from] hyper::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn mcp_error_code(&self) -> i32 {
        match self {
            Error::StationNotFound { .. } => -32001,
            Error::InvalidCoordinates { .. } => -32002,
            Error::RealTimeUnavailable => -32003,
            Error::SearchRadiusTooLarge { .. } => -32004,
            Error::ResultLimitExceeded { .. } => -32005,
            _ => -32000, // Generic server error
        }
    }

    pub fn error_type(&self) -> &'static str {
        match self {
            Error::StationNotFound { .. } => "STATION_NOT_FOUND",
            Error::InvalidCoordinates { .. } => "INVALID_COORDINATES",
            Error::RealTimeUnavailable => "REAL_TIME_UNAVAILABLE",
            Error::SearchRadiusTooLarge { .. } => "SEARCH_RADIUS_TOO_LARGE",
            Error::ResultLimitExceeded { .. } => "RESULT_LIMIT_EXCEEDED",
            Error::CapacityOverflow { .. } => "CAPACITY_OVERFLOW",
            Error::DockCountMismatch { .. } => "DOCK_COUNT_MISMATCH",
            Error::McpProtocol(_) => "MCP_PROTOCOL_ERROR",
            Error::Http(_) => "HTTP_ERROR",
            Error::Json(_) => "JSON_ERROR",
            Error::Request(_) => "REQUEST_ERROR",
            Error::Internal(_) => "INTERNAL_ERROR",
        }
    }
}
