//! Tests for Error::mcp_error_code() and Error::error_type().
//!
//! These pin the MCP error-code contract so that accidental renumbering
//! is caught immediately.  All tests are offline.

use velib_mcp::Error;

// ── mcp_error_code() ─────────────────────────────────────────────────────────

#[test]
fn rate_limited_is_server_error_code() {
    let e = Error::RateLimited {
        retry_after_seconds: Some(30),
    };
    assert_eq!(e.mcp_error_code(), -32001);
}

#[test]
fn json_parse_error_code() {
    let e: Error = serde_json::from_str::<serde_json::Value>("{").unwrap_err().into();
    assert_eq!(e.mcp_error_code(), -32700);
}

#[test]
fn invalid_coordinates_is_invalid_params_code() {
    let e = Error::InvalidCoordinates {
        latitude: 0.0,
        longitude: 0.0,
    };
    assert_eq!(e.mcp_error_code(), -32602);
}

#[test]
fn outside_service_area_is_invalid_params_code() {
    let e = Error::OutsideServiceArea { distance_km: 100.0 };
    assert_eq!(e.mcp_error_code(), -32602);
}

#[test]
fn search_radius_too_large_is_invalid_params_code() {
    let e = Error::SearchRadiusTooLarge {
        radius: 10_000,
        max: 5_000,
    };
    assert_eq!(e.mcp_error_code(), -32602);
}

#[test]
fn result_limit_exceeded_is_invalid_params_code() {
    let e = Error::ResultLimitExceeded { limit: 200, max: 100 };
    assert_eq!(e.mcp_error_code(), -32602);
}

#[test]
fn station_not_found_is_invalid_request_code() {
    let e = Error::StationNotFound {
        station_code: "99999".to_string(),
    };
    assert_eq!(e.mcp_error_code(), -32600);
}

#[test]
fn validation_error_is_invalid_params_code() {
    let e = Error::Validation("bad input".to_string());
    assert_eq!(e.mcp_error_code(), -32602);
}

#[test]
fn cache_error_is_internal_code() {
    let e = Error::Cache("disk full".to_string());
    assert_eq!(e.mcp_error_code(), -32603);
}

#[test]
fn internal_error_is_internal_code() {
    let e = Error::Internal(anyhow::anyhow!("oops"));
    assert_eq!(e.mcp_error_code(), -32603);
}

// ── error_type() ─────────────────────────────────────────────────────────────

#[test]
fn error_type_strings_are_stable() {
    assert_eq!(
        Error::RateLimited { retry_after_seconds: None }.error_type(),
        "rate_limited"
    );
    assert_eq!(
        Error::InvalidCoordinates { latitude: 0.0, longitude: 0.0 }.error_type(),
        "invalid_coordinates"
    );
    assert_eq!(
        Error::OutsideServiceArea { distance_km: 1.0 }.error_type(),
        "outside_service_area"
    );
    assert_eq!(
        Error::SearchRadiusTooLarge { radius: 1, max: 1 }.error_type(),
        "search_radius_too_large"
    );
    assert_eq!(
        Error::ResultLimitExceeded { limit: 1, max: 1 }.error_type(),
        "result_limit_exceeded"
    );
    assert_eq!(
        Error::StationNotFound { station_code: "x".to_string() }.error_type(),
        "station_not_found"
    );
    assert_eq!(
        Error::McpProtocol("oops".to_string()).error_type(),
        "mcp_protocol_error"
    );
    assert_eq!(
        Error::Validation("oops".to_string()).error_type(),
        "validation_error"
    );
    assert_eq!(
        Error::Cache("oops".to_string()).error_type(),
        "cache_error"
    );
    assert_eq!(
        Error::Internal(anyhow::anyhow!("oops")).error_type(),
        "internal_error"
    );
}

// ── Display messages ──────────────────────────────────────────────────────────

#[test]
fn rate_limited_display_includes_seconds_when_present() {
    let e = Error::RateLimited {
        retry_after_seconds: Some(60),
    };
    assert!(e.to_string().contains("60s"), "display: {e}");
}

#[test]
fn rate_limited_display_omits_seconds_when_absent() {
    let e = Error::RateLimited {
        retry_after_seconds: None,
    };
    let s = e.to_string();
    assert!(s.contains("Rate limited"), "display: {s}");
    assert!(!s.contains("retry after"), "should not mention retry after when None: {s}");
}

#[test]
fn outside_service_area_display_includes_distance() {
    let e = Error::OutsideServiceArea { distance_km: 73.5 };
    assert!(e.to_string().contains("73.5"), "display: {e}");
}
