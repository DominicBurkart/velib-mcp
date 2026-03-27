//! Tests for error module: MCP error code mapping and error type strings.

use velib_mcp::Error;

#[test]
fn mcp_error_codes_follow_jsonrpc_spec() {
    // -32602: Invalid params
    let invalid_coords = Error::InvalidCoordinates {
        latitude: 999.0,
        longitude: 999.0,
    };
    assert_eq!(invalid_coords.mcp_error_code(), -32602);

    let outside = Error::OutsideServiceArea { distance_km: 100.0 };
    assert_eq!(outside.mcp_error_code(), -32602);

    let radius = Error::SearchRadiusTooLarge {
        radius: 9999,
        max: 5000,
    };
    assert_eq!(radius.mcp_error_code(), -32602);

    let limit = Error::ResultLimitExceeded { limit: 200, max: 100 };
    assert_eq!(limit.mcp_error_code(), -32602);

    let validation = Error::Validation("bad input".into());
    assert_eq!(validation.mcp_error_code(), -32602);

    // -32700: Parse error
    let json_err = Error::Json(serde_json::from_str::<i32>("not json").unwrap_err());
    assert_eq!(json_err.mcp_error_code(), -32700);

    // -32600: Invalid request
    let not_found = Error::StationNotFound {
        station_code: "XYZ".into(),
    };
    assert_eq!(not_found.mcp_error_code(), -32600);

    // -32603: Internal error
    let mcp = Error::McpProtocol("bad".into());
    assert_eq!(mcp.mcp_error_code(), -32603);

    let cache = Error::Cache("fail".into());
    assert_eq!(cache.mcp_error_code(), -32603);

    let internal = Error::Internal(anyhow::anyhow!("boom"));
    assert_eq!(internal.mcp_error_code(), -32603);

    // -32001: Server error (HTTP / rate limit)
    let rate = Error::RateLimited {
        retry_after_seconds: Some(30),
    };
    assert_eq!(rate.mcp_error_code(), -32001);

    let rate_none = Error::RateLimited {
        retry_after_seconds: None,
    };
    assert_eq!(rate_none.mcp_error_code(), -32001);
}

#[test]
fn error_type_strings_are_unique_and_snake_case() {
    let errors: Vec<Box<dyn Fn() -> Error>> = vec![
        Box::new(|| Error::InvalidCoordinates {
            latitude: 0.0,
            longitude: 0.0,
        }),
        Box::new(|| Error::OutsideServiceArea { distance_km: 1.0 }),
        Box::new(|| Error::SearchRadiusTooLarge {
            radius: 1,
            max: 1,
        }),
        Box::new(|| Error::ResultLimitExceeded { limit: 1, max: 1 }),
        Box::new(|| Error::Validation("x".into())),
        Box::new(|| Error::StationNotFound {
            station_code: "x".into(),
        }),
        Box::new(|| Error::McpProtocol("x".into())),
        Box::new(|| Error::Cache("x".into())),
        Box::new(|| Error::Internal(anyhow::anyhow!("x"))),
        Box::new(|| Error::RateLimited {
            retry_after_seconds: None,
        }),
    ];

    let mut seen = std::collections::HashSet::new();
    for make_err in &errors {
        let err = make_err();
        let ty = err.error_type();

        // Must be snake_case (lowercase + underscores only)
        assert!(
            ty.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "error_type '{ty}' is not snake_case"
        );

        // Must be unique
        assert!(
            seen.insert(ty),
            "duplicate error_type '{ty}'"
        );
    }
}

#[test]
fn error_display_messages_are_non_empty() {
    let errors: Vec<Error> = vec![
        Error::InvalidCoordinates {
            latitude: 1.0,
            longitude: 2.0,
        },
        Error::OutsideServiceArea { distance_km: 55.0 },
        Error::SearchRadiusTooLarge {
            radius: 6000,
            max: 5000,
        },
        Error::ResultLimitExceeded { limit: 200, max: 100 },
        Error::Validation("test".into()),
        Error::StationNotFound {
            station_code: "ABC".into(),
        },
        Error::McpProtocol("proto".into()),
        Error::Cache("cache issue".into()),
        Error::Internal(anyhow::anyhow!("internal")),
        Error::RateLimited {
            retry_after_seconds: Some(10),
        },
        Error::RateLimited {
            retry_after_seconds: None,
        },
    ];

    for err in &errors {
        let msg = err.to_string();
        assert!(!msg.is_empty(), "Error display should not be empty");
    }
}
