use crate::{Error, Result};
use std::future::Future;
use tokio::time::{sleep, timeout, Duration};
use tracing::{debug, warn};

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub timeout_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 1000,
            max_delay_ms: 30000,
            timeout_ms: 60000,
        }
    }
}

impl RetryConfig {
    /// Create a configuration for HTTP requests
    pub fn for_http() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 1000,
            max_delay_ms: 10000,
            timeout_ms: 30000,
        }
    }

    /// Create a configuration for cache operations
    pub fn for_cache() -> Self {
        Self {
            max_attempts: 2,
            base_delay_ms: 500,
            max_delay_ms: 2000,
            timeout_ms: 5000,
        }
    }

    /// Create a configuration for critical operations
    pub fn for_critical() -> Self {
        Self {
            max_attempts: 5,
            base_delay_ms: 2000,
            max_delay_ms: 60000,
            timeout_ms: 120000,
        }
    }
}

/// Retry a future with exponential backoff
pub async fn retry_with_backoff<F, Fut, T>(
    operation_name: &str,
    mut operation: F,
    config: RetryConfig,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut _last_error = None;

    for attempt in 0..config.max_attempts {
        let operation_future = operation();

        match timeout(Duration::from_millis(config.timeout_ms), operation_future).await {
            Ok(Ok(result)) => {
                if attempt > 0 {
                    debug!(
                        "Operation '{}' succeeded on attempt {}",
                        operation_name,
                        attempt + 1
                    );
                }
                return Ok(result);
            }
            Ok(Err(error)) => {
                _last_error = Some(error);

                if let Some(ref err) = _last_error {
                    if !err.is_retryable() {
                        warn!(
                            "Operation '{}' failed with non-retryable error on attempt {}: {}",
                            operation_name,
                            attempt + 1,
                            err
                        );
                        err.increment_metric();
                        return Err(err.clone());
                    }
                }

                // Don't sleep after the last attempt
                if attempt + 1 < config.max_attempts {
                    let delay_ms = if let Some(ref err) = _last_error {
                        err.retry_delay_ms(attempt).min(config.max_delay_ms)
                    } else {
                        config.base_delay_ms * (2_u64.pow(attempt.min(10)))
                    };

                    debug!(
                        "Operation '{}' failed on attempt {}, retrying in {}ms: {}",
                        operation_name,
                        attempt + 1,
                        delay_ms,
                        _last_error.as_ref().unwrap()
                    );

                    sleep(Duration::from_millis(delay_ms)).await;
                }
            }
            Err(_) => {
                let timeout_error = Error::Timeout {
                    operation: operation_name.to_string(),
                    timeout_ms: config.timeout_ms,
                };

                warn!(
                    "Operation '{}' timed out on attempt {} ({}ms)",
                    operation_name,
                    attempt + 1,
                    config.timeout_ms
                );

                if !timeout_error.is_retryable() || attempt + 1 >= config.max_attempts {
                    timeout_error.increment_metric();
                    return Err(timeout_error);
                }

                _last_error = Some(timeout_error);

                // Don't sleep after the last attempt
                if attempt + 1 < config.max_attempts {
                    let delay_ms = config.base_delay_ms * (2_u64.pow(attempt.min(10)));
                    sleep(Duration::from_millis(delay_ms.min(config.max_delay_ms))).await;
                }
            }
        }
    }

    let final_error = Error::RetryExhausted {
        operation: operation_name.to_string(),
        attempts: config.max_attempts,
    };

    warn!(
        "Operation '{}' exhausted all {} retry attempts",
        operation_name, config.max_attempts
    );

    final_error.increment_metric();
    Err(final_error)
}

/// Retry an HTTP operation with appropriate configuration
pub async fn retry_http<F, Fut, T>(operation_name: &str, operation: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    retry_with_backoff(operation_name, operation, RetryConfig::for_http()).await
}

/// Retry a cache operation with appropriate configuration
pub async fn retry_cache<F, Fut, T>(operation_name: &str, operation: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    retry_with_backoff(operation_name, operation, RetryConfig::for_cache()).await
}

/// Circuit breaker state for advanced error handling
#[derive(Debug, Clone)]
pub enum CircuitState {
    Closed,
    Open { until: std::time::Instant },
    HalfOpen,
}

/// Circuit breaker for preventing cascading failures
#[derive(Debug)]
pub struct CircuitBreaker {
    state: std::sync::Arc<std::sync::Mutex<CircuitState>>,
    failure_threshold: u32,
    recovery_timeout_ms: u64,
    consecutive_failures: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, recovery_timeout_ms: u64) -> Self {
        Self {
            state: std::sync::Arc::new(std::sync::Mutex::new(CircuitState::Closed)),
            failure_threshold,
            recovery_timeout_ms,
            consecutive_failures: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)),
        }
    }

    pub async fn call<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        // Check circuit state
        {
            let mut state = self.state.lock().unwrap();
            match *state {
                CircuitState::Open { until } => {
                    if std::time::Instant::now() >= until {
                        *state = CircuitState::HalfOpen;
                    } else {
                        return Err(Error::Internal {
                            message: "Circuit breaker is open".to_string(),
                        });
                    }
                }
                CircuitState::HalfOpen | CircuitState::Closed => {}
            }
        }

        // Execute operation
        match operation().await {
            Ok(result) => {
                // Reset on success
                self.consecutive_failures
                    .store(0, std::sync::atomic::Ordering::Relaxed);
                {
                    let mut state = self.state.lock().unwrap();
                    *state = CircuitState::Closed;
                }
                Ok(result)
            }
            Err(error) => {
                // Increment failure count
                let failures = self
                    .consecutive_failures
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                    + 1;

                if failures >= self.failure_threshold {
                    // Open circuit
                    let mut state = self.state.lock().unwrap();
                    *state = CircuitState::Open {
                        until: std::time::Instant::now()
                            + std::time::Duration::from_millis(self.recovery_timeout_ms),
                    };
                }

                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_success_on_first_attempt() {
        let result = retry_with_backoff(
            "test_operation",
            || async { Ok::<i32, Error>(42) },
            RetryConfig {
                max_attempts: 3,
                base_delay_ms: 10,
                max_delay_ms: 100,
                timeout_ms: 1000,
            },
        )
        .await;

        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = attempt_count.clone();

        let result = retry_with_backoff(
            "test_operation",
            move || {
                let count = attempt_count_clone.fetch_add(1, Ordering::Relaxed);
                async move {
                    if count < 2 {
                        Err(Error::Internal {
                            message: "Simulated failure".to_string(),
                        })
                    } else {
                        Ok(42)
                    }
                }
            },
            RetryConfig {
                max_attempts: 3,
                base_delay_ms: 1,
                max_delay_ms: 10,
                timeout_ms: 1000,
            },
        )
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempt_count.load(Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn test_retry_exhausted() {
        let result = retry_with_backoff(
            "test_operation",
            || async {
                Err::<i32, Error>(Error::Internal {
                    message: "Always fails".to_string(),
                })
            },
            RetryConfig {
                max_attempts: 2,
                base_delay_ms: 1,
                max_delay_ms: 10,
                timeout_ms: 1000,
            },
        )
        .await;

        assert!(matches!(result, Err(Error::RetryExhausted { .. })));
    }

    #[tokio::test]
    async fn test_non_retryable_error() {
        let result = retry_with_backoff(
            "test_operation",
            || async {
                Err::<i32, Error>(Error::InvalidCoordinates {
                    latitude: 0.0,
                    longitude: 0.0,
                })
            },
            RetryConfig {
                max_attempts: 3,
                base_delay_ms: 1,
                max_delay_ms: 10,
                timeout_ms: 1000,
            },
        )
        .await;

        assert!(matches!(result, Err(Error::InvalidCoordinates { .. })));
    }

    #[tokio::test]
    async fn test_circuit_breaker() {
        let circuit = CircuitBreaker::new(2, 100);

        // First failure
        let result1 = circuit
            .call(|| async {
                Err::<(), Error>(Error::Internal {
                    message: "Failure".to_string(),
                })
            })
            .await;
        assert!(result1.is_err());

        // Second failure should open circuit
        let result2 = circuit
            .call(|| async {
                Err::<(), Error>(Error::Internal {
                    message: "Failure".to_string(),
                })
            })
            .await;
        assert!(result2.is_err());

        // Third call should be rejected by open circuit
        let result3 = circuit.call(|| async { Ok::<(), Error>(()) }).await;
        assert!(result3.is_err());
    }
}
