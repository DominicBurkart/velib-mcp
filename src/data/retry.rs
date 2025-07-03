use crate::{Error, Result};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// Configuration for retry behavior
///
/// This struct allows fine-tuning of the retry logic used for HTTP requests
/// that may encounter rate limiting or temporary failures.
///
/// # Examples
///
/// Create a conservative retry configuration:
/// ```
/// use velib_mcp::data::RetryConfig;
///
/// let conservative = RetryConfig {
///     max_attempts: 2,
///     base_delay_seconds: 1,
///     max_delay_seconds: 30,
///     use_jitter: true,
/// };
/// ```
///
/// Create an aggressive retry configuration for high-availability scenarios:
/// ```
/// use velib_mcp::data::RetryConfig;
///
/// let aggressive = RetryConfig {
///     max_attempts: 5,
///     base_delay_seconds: 1,
///     max_delay_seconds: 120,
///     use_jitter: true,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (excluding initial attempt)
    ///
    /// For example, if set to 3, the total number of requests will be 4
    /// (1 initial + 3 retries).
    pub max_attempts: u32,

    /// Base delay for exponential backoff (in seconds)
    ///
    /// The actual delay for attempt N will be: base_delay * 2^N
    /// (subject to max_delay_seconds and jitter).
    pub base_delay_seconds: u64,

    /// Maximum delay between retries (in seconds)
    ///
    /// Prevents exponential backoff from creating excessively long delays.
    pub max_delay_seconds: u64,

    /// Whether to add jitter to prevent thundering herd
    ///
    /// When true, adds up to 25% random variation to the calculated delay
    /// to prevent multiple clients from retrying at exactly the same time.
    pub use_jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_seconds: 1,
            max_delay_seconds: 60,
            use_jitter: true,
        }
    }
}

/// Strategy for calculating retry delays
#[derive(Debug, Clone)]
pub enum RetryStrategy {
    /// Exponential backoff with optional jitter
    ExponentialBackoff {
        /// Base delay in seconds
        base_delay: u64,
        /// Maximum delay in seconds
        max_delay: u64,
        /// Whether to add jitter (up to 25% of calculated delay)
        use_jitter: bool,
    },
    /// Fixed delay between retries
    FixedDelay {
        /// Delay in seconds
        delay: u64,
    },
}

impl RetryStrategy {
    /// Calculate delay for a given attempt number (0-based)
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        match self {
            RetryStrategy::ExponentialBackoff {
                base_delay,
                max_delay,
                use_jitter,
            } => {
                let delay = base_delay * 2_u64.pow(attempt);
                let delay = delay.min(*max_delay);

                if *use_jitter {
                    // Add jitter up to 25% of delay
                    let jitter = (delay as f64 * 0.25 * fastrand::f64()).round() as u64;
                    Duration::from_secs(delay + jitter)
                } else {
                    Duration::from_secs(delay)
                }
            }
            RetryStrategy::FixedDelay { delay } => Duration::from_secs(*delay),
        }
    }
}

/// Retry policy for handling failed HTTP requests
#[derive(Debug)]
pub struct RetryPolicy {
    config: RetryConfig,
    strategy: RetryStrategy,
}

impl RetryPolicy {
    /// Create a new retry policy with default configuration
    pub fn new() -> Self {
        Self::with_config(RetryConfig::default())
    }

    /// Create a new retry policy with custom configuration
    pub fn with_config(config: RetryConfig) -> Self {
        let strategy = RetryStrategy::ExponentialBackoff {
            base_delay: config.base_delay_seconds,
            max_delay: config.max_delay_seconds,
            use_jitter: config.use_jitter,
        };

        Self { config, strategy }
    }

    /// Execute a closure with retry logic
    pub async fn execute<T, F, Fut>(&self, mut operation: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut last_error = None;

        debug!(
            "Starting operation with retry policy: max_attempts={}, base_delay={}s, max_delay={}s",
            self.config.max_attempts, self.config.base_delay_seconds, self.config.max_delay_seconds
        );

        for attempt in 0..=self.config.max_attempts {
            match operation().await {
                Ok(result) => {
                    if attempt > 0 {
                        debug!("Operation succeeded after {} retry attempts", attempt);
                    }
                    return Ok(result);
                }
                Err(error) => {
                    last_error = Some(error);

                    // Don't retry on the last attempt
                    if attempt == self.config.max_attempts {
                        break;
                    }

                    // Check if this is a retryable error
                    if let Some(last_error_ref) = last_error.as_ref() {
                        if !Self::is_retryable_error(last_error_ref) {
                            info!(
                                "Error is not retryable, failing immediately after attempt {}: {}",
                                attempt + 1,
                                last_error_ref
                            );
                            break;
                        }
                    }

                    let delay = self.strategy.calculate_delay(attempt);
                    warn!(
                        "Attempt {} failed, retrying in {:.2}s: {}",
                        attempt + 1,
                        delay.as_secs_f64(),
                        last_error.as_ref().unwrap()
                    );

                    sleep(delay).await;
                }
            }
        }

        // Return the last error if all attempts failed
        let final_error = last_error.unwrap();
        info!(
            "All retry attempts exhausted ({} total attempts). Final error: {}",
            self.config.max_attempts + 1,
            final_error
        );
        Err(final_error)
    }

    /// Check if an error is retryable
    fn is_retryable_error(error: &Error) -> bool {
        match error {
            Error::Http(reqwest_error) => {
                if let Some(status) = reqwest_error.status() {
                    // Retry on 429 (Rate Limited), 500, 502, 503, 504
                    matches!(status.as_u16(), 429 | 500 | 502 | 503 | 504)
                } else {
                    // Retry on network errors (no status code)
                    true
                }
            }
            Error::RateLimited { .. } => true,
            // Don't retry on validation errors or other client errors
            _ => false,
        }
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to extract retry-after header from reqwest error
pub fn extract_retry_after_from_response(response: &reqwest::Response) -> Option<u64> {
    response
        .headers()
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
}

/// Helper function to create rate limited error from HTTP response
pub fn create_rate_limited_error(response: &reqwest::Response) -> Error {
    let retry_after = extract_retry_after_from_response(response);
    Error::RateLimited {
        retry_after_seconds: retry_after,
    }
}

/// Wrapper for making HTTP requests with retry logic
#[derive(Debug)]
pub struct RetryableHttpClient {
    client: reqwest::Client,
    retry_policy: RetryPolicy,
}

impl RetryableHttpClient {
    /// Create a new retryable HTTP client with default retry policy
    pub fn new() -> Self {
        Self::with_retry_policy(RetryPolicy::new())
    }

    /// Create a new retryable HTTP client with custom retry policy
    pub fn with_retry_policy(retry_policy: RetryPolicy) -> Self {
        Self {
            client: reqwest::Client::new(),
            retry_policy,
        }
    }

    /// Make a GET request with retry logic
    pub async fn get(&self, url: &str) -> Result<reqwest::Response> {
        debug!("Making GET request to: {}", url);

        self.retry_policy
            .execute(|| async {
                let response = self.client.get(url).send().await?;

                debug!("Received response: {} {}", response.status(), url);

                // Check for rate limiting
                if response.status() == 429 {
                    let retry_after = extract_retry_after_from_response(&response);
                    warn!(
                        "Rate limited (429) for {}{}",
                        url,
                        retry_after.map_or_else(
                            String::new,
                            |seconds| format!(", retry after {}s", seconds)
                        )
                    );
                    return Err(create_rate_limited_error(&response));
                }

                // Check for other HTTP errors
                if !response.status().is_success() {
                    warn!("HTTP error {} for {}", response.status(), url);
                    return Err(Error::Http(
                        response.error_for_status().unwrap_err(),
                    ));
                }

                Ok(response)
            })
            .await
    }

    /// Make a GET request with query parameters and retry logic
    pub async fn get_with_query<T>(&self, url: &str, query: &T) -> Result<reqwest::Response>
    where
        T: serde::Serialize + ?Sized,
    {
        debug!("Making GET request with query params to: {}", url);

        self.retry_policy
            .execute(|| async {
                let response = self.client.get(url).query(query).send().await?;

                debug!("Received response: {} {}", response.status(), url);

                // Check for rate limiting
                if response.status() == 429 {
                    let retry_after = extract_retry_after_from_response(&response);
                    warn!(
                        "Rate limited (429) for {}{}",
                        url,
                        retry_after.map_or_else(
                            String::new,
                            |seconds| format!(", retry after {}s", seconds)
                        )
                    );
                    return Err(create_rate_limited_error(&response));
                }

                // Check for other HTTP errors
                if !response.status().is_success() {
                    warn!("HTTP error {} for {}", response.status(), url);
                    return Err(Error::Http(
                        response.error_for_status().unwrap_err(),
                    ));
                }

                Ok(response)
            })
            .await
    }

    /// Get the underlying reqwest client
    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }
}

impl Default for RetryableHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tokio::time::Instant;

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.base_delay_seconds, 1);
        assert_eq!(config.max_delay_seconds, 60);
        assert!(config.use_jitter);
    }

    #[test]
    fn test_exponential_backoff_calculation() {
        let strategy = RetryStrategy::ExponentialBackoff {
            base_delay: 1,
            max_delay: 10,
            use_jitter: false,
        };

        assert_eq!(strategy.calculate_delay(0), Duration::from_secs(1));
        assert_eq!(strategy.calculate_delay(1), Duration::from_secs(2));
        assert_eq!(strategy.calculate_delay(2), Duration::from_secs(4));
        assert_eq!(strategy.calculate_delay(3), Duration::from_secs(8));
        assert_eq!(strategy.calculate_delay(4), Duration::from_secs(10)); // Capped at max_delay
    }

    #[test]
    fn test_fixed_delay_calculation() {
        let strategy = RetryStrategy::FixedDelay { delay: 5 };

        assert_eq!(strategy.calculate_delay(0), Duration::from_secs(5));
        assert_eq!(strategy.calculate_delay(1), Duration::from_secs(5));
        assert_eq!(strategy.calculate_delay(2), Duration::from_secs(5));
    }

    #[test]
    fn test_exponential_backoff_with_jitter() {
        let strategy = RetryStrategy::ExponentialBackoff {
            base_delay: 1,
            max_delay: 10,
            use_jitter: true,
        };

        // Test that jitter produces different results
        let delay1 = strategy.calculate_delay(2);
        let delay2 = strategy.calculate_delay(2);

        // Base delay should be 4 seconds
        assert!(delay1 >= Duration::from_secs(4));
        assert!(delay2 >= Duration::from_secs(4));

        // With jitter, should be <= 4 + 25% = 5 seconds
        assert!(delay1 <= Duration::from_secs(5));
        assert!(delay2 <= Duration::from_secs(5));
    }

    #[test]
    fn test_is_retryable_error() {
        // Test rate limited error
        let rate_limited = Error::RateLimited {
            retry_after_seconds: Some(30),
        };
        assert!(RetryPolicy::is_retryable_error(&rate_limited));

        // Test validation error (should not retry)
        let validation = Error::Validation("Invalid input".to_string());
        assert!(!RetryPolicy::is_retryable_error(&validation));

        // Test station not found (should not retry)
        let not_found = Error::StationNotFound {
            station_code: "12345".to_string(),
        };
        assert!(!RetryPolicy::is_retryable_error(&not_found));

        // Test internal error (should not retry)
        let internal = Error::Internal(anyhow::anyhow!("Internal error"));
        assert!(!RetryPolicy::is_retryable_error(&internal));
    }

    #[tokio::test]
    async fn test_retry_policy_success_on_first_attempt() {
        let policy = RetryPolicy::new();
        let call_count = Arc::new(Mutex::new(0));
        let call_count_clone = call_count.clone();

        let result = policy
            .execute(|| {
                let count = call_count_clone.clone();
                async move {
                    *count.lock().unwrap() += 1;
                    Ok::<i32, Error>(42)
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(*call_count.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_retry_policy_success_after_retries() {
        let policy = RetryPolicy::with_config(RetryConfig {
            max_attempts: 2,
            base_delay_seconds: 0, // No delay for faster tests
            max_delay_seconds: 0,
            use_jitter: false,
        });

        let call_count = Arc::new(Mutex::new(0));
        let call_count_clone = call_count.clone();

        let result = policy
            .execute(|| {
                let count = call_count_clone.clone();
                async move {
                    let current_count = {
                        let mut guard = count.lock().unwrap();
                        *guard += 1;
                        *guard
                    };

                    if current_count < 2 {
                        Err(Error::RateLimited {
                            retry_after_seconds: Some(1),
                        })
                    } else {
                        Ok::<i32, Error>(42)
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(*call_count.lock().unwrap(), 2);
    }

    #[tokio::test]
    async fn test_retry_policy_exhausted_attempts() {
        let policy = RetryPolicy::with_config(RetryConfig {
            max_attempts: 2,
            base_delay_seconds: 0, // No delay for faster tests
            max_delay_seconds: 0,
            use_jitter: false,
        });

        let call_count = Arc::new(Mutex::new(0));
        let call_count_clone = call_count.clone();

        let result = policy
            .execute(|| {
                let count = call_count_clone.clone();
                async move {
                    *count.lock().unwrap() += 1;
                    Err::<i32, Error>(Error::RateLimited {
                        retry_after_seconds: Some(1),
                    })
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(*call_count.lock().unwrap(), 3); // Initial + 2 retries

        match result.unwrap_err() {
            Error::RateLimited { .. } => {} // Expected
            _ => panic!("Expected RateLimited error"),
        }
    }

    #[tokio::test]
    async fn test_retry_policy_non_retryable_error() {
        let policy = RetryPolicy::new();
        let call_count = Arc::new(Mutex::new(0));
        let call_count_clone = call_count.clone();

        let result = policy
            .execute(|| {
                let count = call_count_clone.clone();
                async move {
                    *count.lock().unwrap() += 1;
                    Err::<i32, Error>(Error::Validation("Invalid input".to_string()))
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(*call_count.lock().unwrap(), 1); // Should not retry
    }

    #[tokio::test]
    async fn test_retry_policy_timing() {
        let policy = RetryPolicy::with_config(RetryConfig {
            max_attempts: 1,
            base_delay_seconds: 1,
            max_delay_seconds: 1,
            use_jitter: false,
        });

        let start = Instant::now();
        let call_count = Arc::new(Mutex::new(0));
        let call_count_clone = call_count.clone();

        let result = policy
            .execute(|| {
                let count = call_count_clone.clone();
                async move {
                    let current_count = {
                        let mut guard = count.lock().unwrap();
                        *guard += 1;
                        *guard
                    };

                    if current_count == 1 {
                        Err(Error::RateLimited {
                            retry_after_seconds: Some(1),
                        })
                    } else {
                        Ok::<i32, Error>(42)
                    }
                }
            })
            .await;

        let duration = start.elapsed();

        assert!(result.is_ok());
        assert_eq!(*call_count.lock().unwrap(), 2);
        // Should have waited approximately 1 second between attempts
        assert!(duration >= Duration::from_millis(950));
        assert!(duration < Duration::from_millis(1200));
    }

    #[test]
    fn test_retry_after_parsing() {
        // Test the header parsing logic directly
        assert_eq!(Some(30), "30".parse::<u64>().ok());
        assert_eq!(None, "invalid".parse::<u64>().ok());
        assert_eq!(Some(0), "0".parse::<u64>().ok());
    }

    #[test]
    fn test_error_display() {
        let error_with_retry = Error::RateLimited {
            retry_after_seconds: Some(30),
        };
        assert!(error_with_retry.to_string().contains("retry after 30s"));

        let error_without_retry = Error::RateLimited {
            retry_after_seconds: None,
        };
        assert!(error_without_retry.to_string().contains("Rate limited"));
        assert!(!error_without_retry.to_string().contains("retry after"));
    }

    #[test]
    fn test_retry_config_custom() {
        let config = RetryConfig {
            max_attempts: 5,
            base_delay_seconds: 2,
            max_delay_seconds: 120,
            use_jitter: false,
        };

        let policy = RetryPolicy::with_config(config);
        assert_eq!(policy.config.max_attempts, 5);
        assert_eq!(policy.config.base_delay_seconds, 2);
        assert_eq!(policy.config.max_delay_seconds, 120);
        assert!(!policy.config.use_jitter);
    }
}
