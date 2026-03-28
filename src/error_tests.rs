#[cfg(test)]
mod tests {
    use crate::error::Error;

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
            (
                Error::McpProtocol("protocol issue".to_string()),
                -32603,
            ),
            (Error::Cache("cache miss".to_string()), -32603),
            (
                Error::Internal(anyhow::anyhow!("internal failure")),
                -32603,
            ),
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
        let json_err: Result<serde_json::Value, _> = serde_json::from_str("not json");
        let error = Error::Json(json_err.unwrap_err());
        assert_eq!(error.mcp_error_code(), -32700);
    }

    #[test]
    fn error_type_covers_all_variants() {
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
            (
                Error::McpProtocol("err".to_string()),
                "mcp_protocol_error",
            ),
            (Error::Validation("err".to_string()), "validation_error"),
            (Error::Cache("err".to_string()), "cache_error"),
            (
                Error::Internal(anyhow::anyhow!("err")),
                "internal_error",
            ),
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
}
