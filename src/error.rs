use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Rate limited by API (HTTP 429){}", match retry_after_seconds {
        Some(seconds) => format!(": retry after {seconds}s"),
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
    #[must_use]
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
    #[must_use]
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

#[cfg(test)]
mod tests {
    use super::*;

    // Each variant maps to a stable MCP JSON-RPC error code.  These codes are
    // part of the public protocol contract – an AI client that receives a
    // -32602 knows the *caller* made a bad request, whereas -32603 signals a
    // server-side fault.  We pin every variant so a refactor can't silently
    // change the semantics.

    #[test]
    fn invalid_coords_is_invalid_params() {
        let e = Error::InvalidCoordinates {
            latitude: 0.0,
            longitude: 0.0,
        };
        assert_eq!(e.mcp_error_code(), -32602);
        assert_eq!(e.error_type(), "invalid_coordinates");
    }

    #[test]
    fn outside_service_area_is_invalid_params() {
        let e = Error::OutsideServiceArea { distance_km: 60.0 };
        assert_eq!(e.mcp_error_code(), -32602);
        assert_eq!(e.error_type(), "outside_service_area");
        // Display message should quote the distance
        assert!(e.to_string().contains("60.0"));
    }

    #[test]
    fn search_radius_too_large_is_invalid_params() {
        let e = Error::SearchRadiusTooLarge {
            radius: 6000,
            max: 5000,
        };
        assert_eq!(e.mcp_error_code(), -32602);
        assert_eq!(e.error_type(), "search_radius_too_large");
        assert!(e.to_string().contains("6000"));
        assert!(e.to_string().contains("5000"));
    }

    #[test]
    fn result_limit_exceeded_is_invalid_params() {
        let e = Error::ResultLimitExceeded { limit: 150, max: 100 };
        assert_eq!(e.mcp_error_code(), -32602);
        assert_eq!(e.error_type(), "result_limit_exceeded");
        assert!(e.to_string().contains("150"));
    }

    #[test]
    fn station_not_found_is_invalid_request() {
        let e = Error::StationNotFound {
            station_code: "99999".to_string(),
        };
        assert_eq!(e.mcp_error_code(), -32600);
        assert_eq!(e.error_type(), "station_not_found");
        assert!(e.to_string().contains("99999"));
    }

    #[test]
    fn rate_limited_without_retry_after() {
        let e = Error::RateLimited {
            retry_after_seconds: None,
        };
        assert_eq!(e.mcp_error_code(), -32001);
        assert_eq!(e.error_type(), "rate_limited");
        // Should not mention "retry after" when value is absent
        assert!(!e.to_string().contains("retry after"));
    }

    #[test]
    fn rate_limited_with_retry_after() {
        let e = Error::RateLimited {
            retry_after_seconds: Some(30),
        };
        assert_eq!(e.mcp_error_code(), -32001);
        assert!(e.to_string().contains("retry after 30s"));
    }

    #[test]
    fn validation_error_is_invalid_params() {
        let e = Error::Validation("bad input".to_string());
        assert_eq!(e.mcp_error_code(), -32602);
        assert_eq!(e.error_type(), "validation_error");
    }

    #[test]
    fn mcp_protocol_error_is_internal() {
        let e = Error::McpProtocol("unknown method".to_string());
        assert_eq!(e.mcp_error_code(), -32603);
        assert_eq!(e.error_type(), "mcp_protocol_error");
    }

    #[test]
    fn cache_error_is_internal() {
        let e = Error::Cache("lock poisoned".to_string());
        assert_eq!(e.mcp_error_code(), -32603);
        assert_eq!(e.error_type(), "cache_error");
    }

    #[test]
    fn internal_error_is_internal() {
        let e = Error::Internal(anyhow::anyhow!("unexpected failure"));
        assert_eq!(e.mcp_error_code(), -32603);
        assert_eq!(e.error_type(), "internal_error");
        assert!(e.to_string().contains("unexpected failure"));
    }

    #[test]
    fn json_parse_error_is_parse_error() {
        let raw: std::result::Result<serde_json::Value, serde_json::Error> =
            serde_json::from_str("not json");
        let e: Error = raw.unwrap_err().into();
        assert_eq!(e.mcp_error_code(), -32700);
        assert_eq!(e.error_type(), "json_error");
    }
}
