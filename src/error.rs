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

    #[test]
    fn mcp_error_code_invalid_params_variants() {
        let cases: Vec<(Error, i32)> = vec![
            (
                Error::InvalidCoordinates {
                    latitude: 0.0,
                    longitude: 0.0,
                },
                -32602,
            ),
            (Error::OutsideServiceArea { distance_km: 100.0 }, -32602),
            (
                Error::SearchRadiusTooLarge {
                    radius: 10000,
                    max: 5000,
                },
                -32602,
            ),
            (
                Error::ResultLimitExceeded {
                    limit: 200,
                    max: 100,
                },
                -32602,
            ),
            (Error::Validation("bad input".to_string()), -32602),
        ];

        for (error, expected_code) in cases {
            assert_eq!(
                error.mcp_error_code(),
                expected_code,
                "Wrong code for {}",
                error
            );
        }
    }

    #[test]
    fn mcp_error_code_internal_variants() {
        let cases: Vec<(Error, i32)> = vec![
            (Error::McpProtocol("protocol issue".to_string()), -32603),
            (Error::Cache("cache miss".to_string()), -32603),
            (Error::Internal(anyhow::anyhow!("internal failure")), -32603),
        ];

        for (error, expected_code) in cases {
            assert_eq!(
                error.mcp_error_code(),
                expected_code,
                "Wrong code for {}",
                error
            );
        }
    }

    #[test]
    fn mcp_error_code_http_and_rate_limit() {
        let rate_limited = Error::RateLimited {
            retry_after_seconds: Some(30),
        };
        assert_eq!(rate_limited.mcp_error_code(), -32001);

        let rate_limited_no_header = Error::RateLimited {
            retry_after_seconds: None,
        };
        assert_eq!(rate_limited_no_header.mcp_error_code(), -32001);
    }

    #[test]
    fn mcp_error_code_station_not_found() {
        let error = Error::StationNotFound {
            station_code: "12345".to_string(),
        };
        assert_eq!(error.mcp_error_code(), -32600);
    }

    #[test]
    fn mcp_error_code_json_parse() {
        let json_err: std::result::Result<serde_json::Value, _> = serde_json::from_str("not json");
        let error = Error::Json(json_err.unwrap_err());
        assert_eq!(error.mcp_error_code(), -32700);
    }

    #[test]
    fn error_type_covers_all_non_http_variants() {
        let cases: Vec<(Error, &str)> = vec![
            (
                Error::RateLimited {
                    retry_after_seconds: None,
                },
                "rate_limited",
            ),
            (
                Error::InvalidCoordinates {
                    latitude: 0.0,
                    longitude: 0.0,
                },
                "invalid_coordinates",
            ),
            (
                Error::OutsideServiceArea { distance_km: 100.0 },
                "outside_service_area",
            ),
            (
                Error::SearchRadiusTooLarge {
                    radius: 10000,
                    max: 5000,
                },
                "search_radius_too_large",
            ),
            (
                Error::ResultLimitExceeded {
                    limit: 200,
                    max: 100,
                },
                "result_limit_exceeded",
            ),
            (
                Error::StationNotFound {
                    station_code: "X".to_string(),
                },
                "station_not_found",
            ),
            (Error::McpProtocol("err".to_string()), "mcp_protocol_error"),
            (Error::Validation("err".to_string()), "validation_error"),
            (Error::Cache("err".to_string()), "cache_error"),
            (Error::Internal(anyhow::anyhow!("err")), "internal_error"),
        ];

        for (error, expected_type) in cases {
            assert_eq!(
                error.error_type(),
                expected_type,
                "Wrong type for {}",
                error
            );
        }
    }

    #[test]
    fn error_display_messages_are_descriptive() {
        let err = Error::InvalidCoordinates {
            latitude: 91.0,
            longitude: 181.0,
        };
        let msg = err.to_string();
        assert!(msg.contains("91"), "Should contain latitude: {msg}");
        assert!(msg.contains("181"), "Should contain longitude: {msg}");

        let err = Error::OutsideServiceArea { distance_km: 75.3 };
        let msg = err.to_string();
        assert!(msg.contains("75.3"), "Should contain distance: {msg}");
        assert!(msg.contains("50km"), "Should mention limit: {msg}");

        let err = Error::SearchRadiusTooLarge {
            radius: 10000,
            max: 5000,
        };
        let msg = err.to_string();
        assert!(msg.contains("10000"), "Should contain radius: {msg}");
        assert!(msg.contains("5000"), "Should contain max: {msg}");

        let err = Error::StationNotFound {
            station_code: "ABC123".to_string(),
        };
        assert!(err.to_string().contains("ABC123"));
    }

    #[test]
    fn rate_limited_display_with_retry_after() {
        let err = Error::RateLimited {
            retry_after_seconds: Some(30),
        };
        assert!(err.to_string().contains("retry after 30s"));
    }

    #[test]
    fn rate_limited_display_without_retry_after() {
        let err = Error::RateLimited {
            retry_after_seconds: None,
        };
        let msg = err.to_string();
        assert!(msg.contains("Rate limited"));
        assert!(!msg.contains("retry after"));
    }
}
