#[cfg(test)]
mod tests {
    use super::super::Error;

    fn create_test_json_error() -> serde_json::Error {
        serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err()
    }

    fn create_test_http_error() -> Error {
        Error::Http {
            message: "Connection failed".to_string(),
            endpoint: "https://api.example.com".to_string(),
        }
    }

    #[test]
    fn test_error_construction() {
        let http_error = create_test_http_error();
        assert!(matches!(http_error, Error::Http { .. }));
        assert_eq!(http_error.error_type(), "http_error");

        let json_error = Error::json_error(create_test_json_error(), "parsing station data");
        assert!(matches!(json_error, Error::Json { .. }));
        assert_eq!(json_error.error_type(), "json_error");

        let mcp_error = Error::mcp_protocol_error("Invalid method", "get_stations");
        assert!(matches!(mcp_error, Error::McpProtocol { .. }));
        assert_eq!(mcp_error.error_type(), "mcp_protocol_error");

        let validation_error = Error::validation_error("Invalid latitude", "coordinates.lat");
        assert!(matches!(validation_error, Error::Validation { .. }));
        assert_eq!(validation_error.error_type(), "validation_error");

        let cache_error = Error::cache_error("Cache miss", "get");
        assert!(matches!(cache_error, Error::Cache { .. }));
        assert_eq!(cache_error.error_type(), "cache_error");
    }

    #[test]
    fn test_mcp_error_codes() {
        assert_eq!(
            Error::InvalidCoordinates {
                latitude: 0.0,
                longitude: 0.0
            }
            .mcp_error_code(),
            -32602
        );
        assert_eq!(
            Error::OutsideServiceArea { distance_km: 60.0 }.mcp_error_code(),
            -32602
        );
        assert_eq!(
            Error::SearchRadiusTooLarge {
                radius: 10000,
                max: 5000
            }
            .mcp_error_code(),
            -32602
        );
        assert_eq!(
            Error::ResultLimitExceeded {
                limit: 1000,
                max: 100
            }
            .mcp_error_code(),
            -32602
        );
        assert_eq!(
            Error::StationNotFound {
                station_code: "123".to_string()
            }
            .mcp_error_code(),
            -32600
        );
        assert_eq!(
            Error::RetryExhausted {
                operation: "test".to_string(),
                attempts: 3
            }
            .mcp_error_code(),
            -32001
        );
        assert_eq!(
            Error::Timeout {
                operation: "test".to_string(),
                timeout_ms: 5000
            }
            .mcp_error_code(),
            -32001
        );
        assert_eq!(
            Error::RateLimit {
                operation: "test".to_string(),
                retry_after_ms: 1000
            }
            .mcp_error_code(),
            -32000
        );
    }

    #[test]
    fn test_retryable_errors() {
        let http_error = create_test_http_error();
        assert!(http_error.is_retryable());

        let timeout_error = Error::Timeout {
            operation: "test".to_string(),
            timeout_ms: 5000,
        };
        assert!(timeout_error.is_retryable());

        let cache_error = Error::cache_error("Cache error", "get");
        assert!(cache_error.is_retryable());

        let validation_error = Error::InvalidCoordinates {
            latitude: 0.0,
            longitude: 0.0,
        };
        assert!(!validation_error.is_retryable());

        let mcp_error = Error::mcp_protocol_error("Invalid method", "test");
        assert!(!mcp_error.is_retryable());
    }

    #[test]
    fn test_retry_delay_calculation() {
        let http_error = create_test_http_error();

        // Test exponential backoff
        let delay_0 = http_error.retry_delay_ms(0);
        let delay_1 = http_error.retry_delay_ms(1);
        let delay_2 = http_error.retry_delay_ms(2);

        // Delays should increase exponentially (accounting for jitter)
        assert!(delay_0 >= 1000 && delay_0 <= 2000); // Base 1000 + jitter 0-1000
        assert!(delay_1 >= 2000 && delay_1 <= 3000); // Base 2000 + jitter 0-1000
        assert!(delay_2 >= 4000 && delay_2 <= 5000); // Base 4000 + jitter 0-1000

        // Test maximum delay cap
        let delay_large = http_error.retry_delay_ms(20);
        assert!(delay_large <= 30000); // Capped at 30 seconds

        // Test different error types have different base delays
        let timeout_error = Error::Timeout {
            operation: "test".to_string(),
            timeout_ms: 5000,
        };
        let timeout_delay = timeout_error.retry_delay_ms(0);
        assert!(timeout_delay >= 2000 && timeout_delay <= 3000); // Base 2000 + jitter

        let cache_error = Error::cache_error("Cache error", "get");
        let cache_delay = cache_error.retry_delay_ms(0);
        assert!(cache_delay >= 500 && cache_delay <= 1500); // Base 500 + jitter
    }

    #[test]
    fn test_error_metrics() {
        // Reset metrics before test
        Error::reset_metrics();

        let http_error = create_test_http_error();
        let validation_error = Error::InvalidCoordinates {
            latitude: 0.0,
            longitude: 0.0,
        };

        // Increment metrics
        http_error.increment_metric();
        http_error.increment_metric();
        validation_error.increment_metric();

        // Check metrics
        let metrics = Error::get_metrics();
        assert_eq!(metrics.get("http_error"), Some(&2));
        assert_eq!(metrics.get("invalid_coordinates"), Some(&1));

        // Reset and verify
        Error::reset_metrics();
        let metrics_after_reset = Error::get_metrics();
        assert!(metrics_after_reset.is_empty());
    }

    #[test]
    fn test_error_display() {
        let http_error = create_test_http_error();
        let error_string = format!("{}", http_error);
        assert!(error_string.contains("HTTP request failed"));
        assert!(error_string.contains("https://api.example.com"));

        let json_error = Error::json_error(create_test_json_error(), "parsing station data");
        let json_string = format!("{}", json_error);
        assert!(json_string.contains("JSON parsing error"));
        assert!(json_string.contains("parsing station data"));

        let validation_error = Error::validation_error("Invalid latitude value", "coordinates.lat");
        let validation_string = format!("{}", validation_error);
        assert!(validation_string.contains("Data validation error"));
        assert!(validation_string.contains("Invalid latitude value"));
        assert!(validation_string.contains("coordinates.lat"));

        let coordinates_error = Error::InvalidCoordinates {
            latitude: 91.0,
            longitude: 181.0,
        };
        let coords_string = format!("{}", coordinates_error);
        assert!(coords_string.contains("Invalid coordinates"));
        assert!(coords_string.contains("91"));
        assert!(coords_string.contains("181"));

        let outside_area_error = Error::OutsideServiceArea { distance_km: 75.5 };
        let area_string = format!("{}", outside_area_error);
        assert!(area_string.contains("outside service area"));
        assert!(area_string.contains("75.5"));

        let retry_error = Error::RetryExhausted {
            operation: "fetch_stations".to_string(),
            attempts: 3,
        };
        let retry_string = format!("{}", retry_error);
        assert!(retry_string.contains("Retry exhausted"));
        assert!(retry_string.contains("fetch_stations"));
        assert!(retry_string.contains("3 attempts"));
    }

    #[test]
    fn test_error_debug() {
        let error = Error::mcp_protocol_error("Method not found", "unknown_method");
        let debug_string = format!("{:?}", error);
        assert!(debug_string.contains("McpProtocol"));
        assert!(debug_string.contains("Method not found"));
        assert!(debug_string.contains("unknown_method"));
    }

    #[test]
    fn test_error_from_conversions() {
        // Test anyhow conversion
        let anyhow_error = anyhow::anyhow!("Generic error");
        let error: Error = anyhow_error.into();
        assert!(matches!(error, Error::Internal { .. }));

        // Test that the conversion preserves the source error
        assert_eq!(error.error_type(), "internal_error");
        assert_eq!(error.mcp_error_code(), -32603);
    }

    #[test]
    fn test_all_error_variants_coverage() {
        // Ensure all error variants have proper error types and codes
        let test_cases = vec![
            (create_test_http_error(), "http_error", -32001),
            (
                Error::json_error(create_test_json_error(), "test"),
                "json_error",
                -32700,
            ),
            (
                Error::InvalidCoordinates {
                    latitude: 0.0,
                    longitude: 0.0,
                },
                "invalid_coordinates",
                -32602,
            ),
            (
                Error::OutsideServiceArea { distance_km: 60.0 },
                "outside_service_area",
                -32602,
            ),
            (
                Error::SearchRadiusTooLarge {
                    radius: 10000,
                    max: 5000,
                },
                "search_radius_too_large",
                -32602,
            ),
            (
                Error::ResultLimitExceeded {
                    limit: 1000,
                    max: 100,
                },
                "result_limit_exceeded",
                -32602,
            ),
            (
                Error::StationNotFound {
                    station_code: "123".to_string(),
                },
                "station_not_found",
                -32600,
            ),
            (
                Error::mcp_protocol_error("test", "method"),
                "mcp_protocol_error",
                -32603,
            ),
            (
                Error::validation_error("test", "field"),
                "validation_error",
                -32602,
            ),
            (
                Error::cache_error("test", "operation"),
                "cache_error",
                -32603,
            ),
            (
                Error::RetryExhausted {
                    operation: "test".to_string(),
                    attempts: 3,
                },
                "retry_exhausted",
                -32001,
            ),
            (
                Error::Timeout {
                    operation: "test".to_string(),
                    timeout_ms: 5000,
                },
                "timeout",
                -32001,
            ),
            (
                Error::RateLimit {
                    operation: "test".to_string(),
                    retry_after_ms: 1000,
                },
                "rate_limit",
                -32000,
            ),
            (
                Error::Internal {
                    message: "test".to_string(),
                },
                "internal_error",
                -32603,
            ),
        ];

        for (error, expected_type, expected_code) in test_cases {
            assert_eq!(error.error_type(), expected_type);
            assert_eq!(error.mcp_error_code(), expected_code);
        }
    }
}
