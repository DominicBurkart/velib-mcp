use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

/// Global error metrics for monitoring
static ERROR_COUNTS: std::sync::LazyLock<Arc<Mutex<HashMap<String, AtomicUsize>>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

#[derive(Error, Debug, Clone)]
pub enum Error {
    #[error("HTTP request failed: {message} (endpoint: {endpoint})")]
    Http { message: String, endpoint: String },

    #[error("Rate limited by API (HTTP 429){}", match retry_after_seconds {
        Some(seconds) => format!(": retry after {seconds}s"),
        None => String::new(),
    })]
    RateLimited { retry_after_seconds: Option<u64> },

    #[error("JSON parsing error: {message} (context: {context})")]
    Json { message: String, context: String },

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

    #[error("MCP protocol error: {message} (method: {method})")]
    McpProtocol { message: String, method: String },

    #[error("Data validation error: {message} (field: {field})")]
    Validation { message: String, field: String },

    #[error("Cache error: {message} (operation: {operation})")]
    Cache { message: String, operation: String },

    #[error("Retry exhausted: {operation} failed after {attempts} attempts")]
    RetryExhausted { operation: String, attempts: u32 },

    #[error("Timeout: {operation} timed out after {timeout_ms}ms")]
    Timeout { operation: String, timeout_ms: u64 },

    #[error("Rate limit exceeded: {operation} rate limited (retry after {retry_after_ms}ms)")]
    RateLimit {
        operation: String,
        retry_after_ms: u64,
    },

    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Self::Http {
            message: err.to_string(),
            endpoint: "unknown".to_string(),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::Json {
            message: err.to_string(),
            context: "unknown".to_string(),
        }
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal {
            message: err.to_string(),
        }
    }
}

impl Error {
    /// Create HTTP error with context
    pub fn http_error(source: reqwest::Error, endpoint: &str) -> Self {
        Self::Http {
            message: source.to_string(),
            endpoint: endpoint.to_string(),
        }
    }

    /// Create JSON error with context
    pub fn json_error(source: serde_json::Error, context: &str) -> Self {
        Self::Json {
            message: source.to_string(),
            context: context.to_string(),
        }
    }

    /// Create MCP protocol error with method context
    pub fn mcp_protocol_error(message: &str, method: &str) -> Self {
        Self::McpProtocol {
            message: message.to_string(),
            method: method.to_string(),
        }
    }

    /// Create validation error with field context
    pub fn validation_error(message: &str, field: &str) -> Self {
        Self::Validation {
            message: message.to_string(),
            field: field.to_string(),
        }
    }

    /// Create cache error with operation context
    pub fn cache_error(message: &str, operation: &str) -> Self {
        Self::Cache {
            message: message.to_string(),
            operation: operation.to_string(),
        }
    }

    /// Get MCP-compatible error code
    #[must_use]
    pub fn mcp_error_code(&self) -> i32 {
        match self {
            Error::Http { .. } => -32001,                 // Transport error
            Error::RateLimited { .. } => -32001,          // Server error (rate limit)
            Error::Json { .. } => -32700,                 // Parse error
            Error::InvalidCoordinates { .. } => -32602,   // Invalid params
            Error::OutsideServiceArea { .. } => -32602,   // Invalid params
            Error::SearchRadiusTooLarge { .. } => -32602, // Invalid params
            Error::ResultLimitExceeded { .. } => -32602,  // Invalid params
            Error::StationNotFound { .. } => -32600,      // Invalid request
            Error::McpProtocol { .. } => -32603,          // Internal error
            Error::Validation { .. } => -32602,           // Invalid params
            Error::Cache { .. } => -32603,                // Internal error
            Error::RetryExhausted { .. } => -32001,       // Transport error
            Error::Timeout { .. } => -32001,              // Transport error
            Error::RateLimit { .. } => -32000,            // Rate limit
            Error::Internal { .. } => -32603,             // Internal error
        }
    }

    /// Get error type string for structured error data
    #[must_use]
    pub fn error_type(&self) -> &'static str {
        match self {
            Error::Http { .. } => "http_error",
            Error::RateLimited { .. } => "rate_limited",
            Error::Json { .. } => "json_error",
            Error::InvalidCoordinates { .. } => "invalid_coordinates",
            Error::OutsideServiceArea { .. } => "outside_service_area",
            Error::SearchRadiusTooLarge { .. } => "search_radius_too_large",
            Error::ResultLimitExceeded { .. } => "result_limit_exceeded",
            Error::StationNotFound { .. } => "station_not_found",
            Error::McpProtocol { .. } => "mcp_protocol_error",
            Error::Validation { .. } => "validation_error",
            Error::Cache { .. } => "cache_error",
            Error::RetryExhausted { .. } => "retry_exhausted",
            Error::Timeout { .. } => "timeout",
            Error::RateLimit { .. } => "rate_limit",
            Error::Internal { .. } => "internal_error",
        }
    }

    /// Increment error count for monitoring
    pub fn increment_metric(&self) {
        let error_type = self.error_type();
        if let Ok(mut counts) = ERROR_COUNTS.lock() {
            let counter = counts
                .entry(error_type.to_string())
                .or_insert_with(|| AtomicUsize::new(0));
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get error metrics for monitoring
    pub fn get_metrics() -> HashMap<String, usize> {
        if let Ok(counts) = ERROR_COUNTS.lock() {
            counts
                .iter()
                .map(|(k, v)| (k.clone(), v.load(Ordering::Relaxed)))
                .collect()
        } else {
            HashMap::new()
        }
    }

    /// Reset error metrics
    pub fn reset_metrics() {
        if let Ok(mut counts) = ERROR_COUNTS.lock() {
            counts.clear();
        }
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::Http { .. }
                | Error::Timeout { .. }
                | Error::Cache { .. }
                | Error::Internal { .. }
                | Error::RateLimited { .. }
        )
    }

    /// Get retry delay in milliseconds for exponential backoff
    pub fn retry_delay_ms(&self, attempt: u32) -> u64 {
        let base_delay = match self {
            Error::Http { .. } => 1000,     // 1 second base
            Error::Timeout { .. } => 2000,  // 2 seconds base
            Error::Cache { .. } => 500,     // 500ms base
            Error::Internal { .. } => 1500, // 1.5 seconds base
            Error::RateLimited {
                retry_after_seconds,
            } => {
                // Use API-provided retry-after if available, otherwise use base delay
                retry_after_seconds.map_or(5000, |s| s * 1000)
            }
            _ => 1000, // Default 1 second
        };

        // Exponential backoff with jitter: base * 2^attempt + random(0, 1000)
        let exponential = base_delay * (2_u64.pow(attempt.min(10))); // Cap at 2^10
        let jitter = fastrand::u64(0..=1000);
        (exponential + jitter).min(30000) // Cap at 30 seconds
    }
}

#[cfg(test)]
mod tests;
