# Error Handling System

This document describes the comprehensive error handling system implemented in the Velib MCP server.

## Overview

The error handling system provides robust, structured error management with:
- **Comprehensive error types** covering all failure modes
- **Retry logic** with exponential backoff for transient failures
- **Circuit breaker pattern** for preventing cascading failures
- **Error metrics** for monitoring and observability
- **MCP protocol compliance** with structured error responses
- **Graceful degradation** for improved user experience

## Error Types

The system defines a comprehensive `Error` enum with context-rich variants:

### Core Error Categories

#### Network & HTTP Errors
```rust
Error::Http { message: String, endpoint: String }
```
- **Purpose**: HTTP request failures (network issues, server errors)
- **Context**: Includes endpoint URL and error details
- **Retryable**: Yes
- **MCP Code**: -32001 (Transport error)

#### Data Parsing Errors
```rust
Error::Json { message: String, context: String }
```
- **Purpose**: JSON parsing and serialization failures
- **Context**: Includes parsing context information
- **Retryable**: No
- **MCP Code**: -32700 (Parse error)

#### Input Validation Errors
```rust
Error::InvalidCoordinates { latitude: f64, longitude: f64 }
Error::OutsideServiceArea { distance_km: f64 }
Error::SearchRadiusTooLarge { radius: u32, max: u32 }
Error::ResultLimitExceeded { limit: u16, max: u16 }
```
- **Purpose**: Input parameter validation failures
- **Context**: Includes specific invalid values and limits
- **Retryable**: No
- **MCP Code**: -32602 (Invalid params)

#### Business Logic Errors
```rust
Error::StationNotFound { station_code: String }
```
- **Purpose**: Resource not found errors
- **Context**: Includes specific resource identifier
- **Retryable**: No
- **MCP Code**: -32600 (Invalid request)

#### Protocol & System Errors
```rust
Error::McpProtocol { message: String, method: String }
Error::Validation { message: String, field: String }
Error::Cache { message: String, operation: String }
```
- **Purpose**: Protocol violations and system errors
- **Context**: Includes method/field/operation context
- **Retryable**: Cache errors only
- **MCP Code**: -32603 (Internal error) or -32602 (Invalid params)

#### Retry & Timeout Errors
```rust
Error::RetryExhausted { operation: String, attempts: u32 }
Error::Timeout { operation: String, timeout_ms: u64 }
Error::RateLimit { operation: String, retry_after_ms: u64 }
```
- **Purpose**: Retry system and rate limiting failures
- **Context**: Includes operation details and timing information
- **Retryable**: Rate limit errors only
- **MCP Code**: -32001 (Transport error) or -32000 (Rate limit)

### Error Construction Helpers

Convenient constructors for common error patterns:

```rust
// HTTP errors with endpoint context
Error::http_error(reqwest_error, "https://api.example.com")

// JSON errors with parsing context  
Error::json_error(serde_error, "parsing station data")

// Protocol errors with method context
Error::mcp_protocol_error("Unknown method", "tools/call")

// Validation errors with field context
Error::validation_error("Invalid latitude", "coordinates.lat")

// Cache errors with operation context
Error::cache_error("Cache miss", "get_stations")
```

## Retry System

The retry system provides intelligent retry logic with exponential backoff:

### Retry Configuration

```rust
pub struct RetryConfig {
    pub max_attempts: u32,     // Maximum retry attempts
    pub base_delay_ms: u64,    // Base delay between retries
    pub max_delay_ms: u64,     // Maximum delay cap
    pub timeout_ms: u64,       // Per-attempt timeout
}

// Predefined configurations
RetryConfig::for_http()      // 3 attempts, 1s base delay
RetryConfig::for_cache()     // 2 attempts, 500ms base delay  
RetryConfig::for_critical()  // 5 attempts, 2s base delay
```

### Retry Behavior

- **Exponential backoff**: `base_delay * 2^attempt + jitter`
- **Jitter**: Random 0-1000ms to prevent thundering herd
- **Delay caps**: Maximum 30 seconds per retry
- **Timeout handling**: Per-attempt timeouts with retry on timeout
- **Error filtering**: Only retryable errors are retried

### Usage Examples

```rust
// Simple HTTP retry
let result = retry_http("fetch_stations", || async {
    client.get("https://api.example.com").send().await
}).await?;

// Custom retry configuration
let result = retry_with_backoff(
    "critical_operation",
    || async { perform_operation().await },
    RetryConfig::for_critical()
).await?;
```

### Retryable Error Detection

The system automatically determines if errors are retryable:

```rust
impl Error {
    pub fn is_retryable(&self) -> bool {
        matches!(self,
            Error::Http { .. } |      // Network issues
            Error::Timeout { .. } |   // Timeouts
            Error::Cache { .. } |     // Cache issues
            Error::Internal { .. }    // Internal errors
        )
    }
}
```

## Circuit Breaker Pattern

Prevents cascading failures when downstream services are unhealthy:

```rust
let circuit = CircuitBreaker::new(
    5,      // Failure threshold
    30000   // Recovery timeout (30s)
);

let result = circuit.call(|| async {
    risky_operation().await
}).await?;
```

### Circuit States

- **Closed**: Normal operation, requests proceed
- **Open**: Failure threshold exceeded, requests fail fast
- **Half-Open**: Testing recovery, single request allowed

## Error Metrics & Monitoring

Built-in error tracking for monitoring and alerting:

```rust
// Automatic metric increment on error creation
error.increment_metric();

// Get current metrics
let metrics = Error::get_metrics();
// Returns: HashMap<String, usize> with error type counts

// Reset metrics (useful for testing)
Error::reset_metrics();
```

### Metric Categories

Tracks counts by error type:
- `http_error`: Network/HTTP failures
- `json_error`: Parsing failures  
- `invalid_coordinates`: Input validation failures
- `timeout`: Operation timeouts
- `retry_exhausted`: Retry failures
- And all other error variants...

## MCP Protocol Integration

Seamless integration with MCP protocol error responses:

```rust
impl Error {
    pub fn mcp_error_code(&self) -> i32 {
        match self {
            Error::Http { .. } => -32001,        // Transport error
            Error::Json { .. } => -32700,        // Parse error
            Error::InvalidCoordinates { .. } => -32602,  // Invalid params
            // ... all variants mapped to appropriate codes
        }
    }
    
    pub fn error_type(&self) -> &'static str {
        // Returns structured error type string for MCP responses
    }
}
```

## Graceful Degradation

The data client implements graceful degradation patterns:

### Cache Fallback
```rust
// Try fresh data, fallback to cache if unavailable
match self.fetch_reference_stations().await {
    Ok(stations) => Ok(stations),
    Err(e) => {
        warn!("Failed to fetch fresh data, trying cache: {}", e);
        self.get_cached_stations().await
    }
}
```

### Partial Failure Handling
```rust
// Continue processing even if some records fail
for record in records {
    if let Ok(station) = self.parse_reference_station(record) {
        all_stations.push(station);
    }
    // Skip invalid records rather than failing entirely
}
```

## Best Practices

### Error Construction
```rust
// ✅ Use specific error types with context
Error::validation_error("Latitude must be between -90 and 90", "coordinates.latitude")

// ❌ Avoid generic errors without context  
Error::Internal { message: "Invalid input".to_string() }
```

### Error Propagation
```rust
// ✅ Use ? operator for clean propagation
let stations = self.fetch_stations().await?;

// ✅ Add context when converting errors
.map_err(|e| Error::http_error(e, endpoint))?
```

### Retry Usage
```rust
// ✅ Use appropriate retry configurations
retry_http("fetch_data", operation).await?         // For HTTP requests
retry_cache("cache_operation", operation).await?   // For cache operations

// ✅ Ensure operations are idempotent before retrying
// ❌ Don't retry operations with side effects
```

### Error Handling in Handlers
```rust
// ✅ Convert to appropriate error types early
let input: ValidatedInput = params.try_into()
    .map_err(|e| Error::validation_error(&e.to_string(), "input"))?;

// ✅ Use structured errors for better debugging
if input.radius > MAX_RADIUS {
    return Err(Error::SearchRadiusTooLarge { 
        radius: input.radius, 
        max: MAX_RADIUS 
    });
}
```

## Testing

Comprehensive test coverage for all error scenarios:

```rust
#[test]
fn test_retry_with_transient_failures() {
    // Test retry behavior with failing then succeeding operations
}

#[test] 
fn test_non_retryable_errors() {
    // Ensure validation errors are not retried
}

#[test]
fn test_error_metrics() {
    // Verify error counting and metrics collection
}

#[test]
fn test_circuit_breaker() {
    // Test circuit breaker state transitions
}
```

## Future Enhancements

Potential improvements to consider:

1. **Structured Logging**: Add tracing integration for error correlation
2. **Error Aggregation**: Batch similar errors to reduce noise
3. **Adaptive Timeouts**: Adjust timeouts based on historical performance
4. **Error Classification**: Machine learning-based error categorization
5. **Distributed Tracing**: Cross-service error tracking

## Conclusion

This error handling system provides a robust foundation for reliable service operation with:
- Comprehensive error coverage
- Intelligent retry strategies  
- Observability and monitoring
- Graceful degradation
- MCP protocol compliance

The system follows Rust best practices and provides excellent developer experience while ensuring production reliability.