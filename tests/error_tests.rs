use velib_mcp::Error;

/// Every error variant must map to a valid JSON-RPC error code.
#[test]
fn all_error_variants_have_valid_mcp_codes() {
    let variants: Vec<Error> = vec![
        Error::RateLimited {
            retry_after_seconds: Some(10),
        },
        Error::RateLimited {
            retry_after_seconds: None,
        },
        Error::InvalidCoordinates {
            latitude: 0.0,
            longitude: 0.0,
        },
        Error::OutsideServiceArea { distance_km: 99.0 },
        Error::SearchRadiusTooLarge {
            radius: 10000,
            max: 5000,
        },
        Error::ResultLimitExceeded {
            limit: 200,
            max: 100,
        },
        Error::StationNotFound {
            station_code: "XYZ".into(),
        },
        Error::McpProtocol("bad".into()),
        Error::Validation("bad".into()),
        Error::Cache("bad".into()),
        Error::Internal(anyhow::anyhow!("boom")),
    ];

    for err in &variants {
        let code = err.mcp_error_code();
        // JSON-RPC error codes are negative
        assert!(code < 0, "Error code for {err} should be negative, got {code}");
    }
}

/// Every error variant must return a non-empty error_type string.
#[test]
fn all_error_variants_have_nonempty_error_type() {
    let variants: Vec<Error> = vec![
        Error::RateLimited {
            retry_after_seconds: None,
        },
        Error::InvalidCoordinates {
            latitude: 0.0,
            longitude: 0.0,
        },
        Error::OutsideServiceArea { distance_km: 99.0 },
        Error::SearchRadiusTooLarge {
            radius: 10000,
            max: 5000,
        },
        Error::ResultLimitExceeded {
            limit: 200,
            max: 100,
        },
        Error::StationNotFound {
            station_code: "XYZ".into(),
        },
        Error::McpProtocol("bad".into()),
        Error::Validation("bad".into()),
        Error::Cache("bad".into()),
        Error::Internal(anyhow::anyhow!("boom")),
    ];

    for err in &variants {
        let t = err.error_type();
        assert!(!t.is_empty(), "error_type for {err} should not be empty");
    }
}

/// Invalid-params variants should all map to -32602.
#[test]
fn invalid_params_errors_use_correct_code() {
    let params_errors: Vec<Error> = vec![
        Error::InvalidCoordinates {
            latitude: 0.0,
            longitude: 0.0,
        },
        Error::OutsideServiceArea { distance_km: 99.0 },
        Error::SearchRadiusTooLarge {
            radius: 10000,
            max: 5000,
        },
        Error::ResultLimitExceeded {
            limit: 200,
            max: 100,
        },
        Error::Validation("x".into()),
    ];

    for err in &params_errors {
        assert_eq!(
            err.mcp_error_code(),
            -32602,
            "Expected -32602 for {err}"
        );
    }
}

/// Display output for RateLimited should differ based on retry_after.
#[test]
fn rate_limited_display_with_and_without_retry() {
    let with = Error::RateLimited {
        retry_after_seconds: Some(30),
    };
    assert!(with.to_string().contains("30"));

    let without = Error::RateLimited {
        retry_after_seconds: None,
    };
    let msg = without.to_string();
    assert!(msg.contains("Rate limited"));
    assert!(!msg.contains("retry after"));
}
