use crate::config::RateLimitConfig;
use crate::middleware::RateLimitMiddleware;
use std::sync::Arc;

/// Shared rate limiting logic for both sync and async clients
/// This eliminates code duplication between HttpClient and AsyncHttpClient
pub struct RateLimitManager {
    config: Option<RateLimitConfig>,
    middleware: Option<RateLimitMiddleware>,
}

impl RateLimitManager {
    /// Create a new rate limit manager with optional configuration
    pub fn new(config: Option<RateLimitConfig>) -> Self {
        let middleware = config.as_ref().and_then(|cfg| {
            RateLimitMiddleware::new("default_rate_limit".to_string(), cfg.clone(), true).ok()
        });
        Self { config, middleware }
    }

    /// Update the rate limiting configuration
    pub fn update_config(&mut self, config: Option<RateLimitConfig>) -> Result<(), String> {
        // Validate configuration if provided
        if let Some(ref cfg) = config {
            if cfg.requests_per_second <= 0.0 {
                return Err("requests_per_second must be greater than 0".to_string());
            }
            if cfg.burst_size.unwrap_or(1) == 0 {
                return Err("burst_size must be greater than 0".to_string());
            }
        }

        self.config = config.clone();
        self.middleware = config.and_then(|cfg| {
            RateLimitMiddleware::new("config_rate_limit".to_string(), cfg, true).ok()
        });
        Ok(())
    }

    /// Get the current rate limiting configuration
    pub fn get_config(&self) -> Option<RateLimitConfig> {
        self.config.clone()
    }

    /// Check if rate limiting is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.as_ref().map_or(false, |config| config.enabled)
    }

    /// Get the current rate limit status for a host (0.0 = blocked, 1.0 = full capacity)
    pub fn get_status(&self, host: &str) -> f64 {
        self.middleware
            .as_ref()
            .map(|m| m.time_until_available(host))
            .unwrap_or(0.0)
    }

    /// Reset rate limiting state for all hosts
    pub fn reset(&mut self) {
        if let Some(middleware) = &self.middleware {
            middleware.reset();
        }
    }

    /// Check if a request to the given host should be allowed
    pub fn check_rate_limit(&self, host: &str) -> bool {
        self.middleware
            .as_ref()
            .map(|m| m.can_proceed(host))
            .unwrap_or(true) // Allow if no rate limiting is configured
    }

    /// Get rate limiting statistics
    pub fn get_stats(&self) -> std::collections::HashMap<String, f64> {
        let mut stats = std::collections::HashMap::new();

        if let Some(ref config) = self.config {
            stats.insert(
                "enabled".to_string(),
                if config.enabled { 1.0 } else { 0.0 },
            );
            stats.insert(
                "requests_per_second".to_string(),
                config.requests_per_second,
            );
            stats.insert(
                "burst_size".to_string(),
                config.burst_size.unwrap_or(1) as f64,
            );
            stats.insert(
                "window_size_seconds".to_string(),
                config.window_size_seconds,
            );
        } else {
            stats.insert("enabled".to_string(), 0.0);
        }

        stats
    }
}

impl Clone for RateLimitManager {
    fn clone(&self) -> Self {
        Self::new(self.config.clone())
    }
}

/// Async-compatible rate limit manager using tokio::sync::Mutex
pub struct AsyncRateLimitManager {
    inner: Arc<tokio::sync::Mutex<RateLimitManager>>,
}

impl AsyncRateLimitManager {
    /// Create a new async rate limit manager
    pub fn new(config: Option<RateLimitConfig>) -> Self {
        Self {
            inner: Arc::new(tokio::sync::Mutex::new(RateLimitManager::new(config))),
        }
    }

    /// Update the rate limiting configuration asynchronously
    pub async fn update_config(&self, config: Option<RateLimitConfig>) -> Result<(), String> {
        let mut manager = self.inner.lock().await;
        manager.update_config(config)
    }

    /// Get the current rate limiting configuration
    pub async fn get_config(&self) -> Option<RateLimitConfig> {
        let manager = self.inner.lock().await;
        manager.get_config()
    }

    /// Check if rate limiting is enabled
    pub async fn is_enabled(&self) -> bool {
        let manager = self.inner.lock().await;
        manager.is_enabled()
    }

    /// Get the current rate limit status for a host
    pub async fn get_status(&self, host: &str) -> f64 {
        let manager = self.inner.lock().await;
        manager.get_status(host)
    }

    /// Reset rate limiting state for all hosts
    pub async fn reset(&self) {
        let mut manager = self.inner.lock().await;
        manager.reset();
    }

    /// Check if a request to the given host should be allowed
    pub async fn check_rate_limit(&self, host: &str) -> bool {
        let manager = self.inner.lock().await;
        manager.check_rate_limit(host)
    }

    /// Get rate limiting statistics
    pub async fn get_stats(&self) -> std::collections::HashMap<String, f64> {
        let manager = self.inner.lock().await;
        manager.get_stats()
    }
}

impl Clone for AsyncRateLimitManager {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{RateLimitAlgorithm, RateLimitConfig};

    #[test]
    fn test_rate_limit_manager_creation() {
        let config = RateLimitConfig {
            enabled: true,
            algorithm: RateLimitAlgorithm::TokenBucket,
            requests_per_second: 10.0,
            requests_per_minute: Some(600),
            requests_per_hour: Some(36000),
            burst_size: Some(5),
            window_size_seconds: 1.0,
            per_host: true,
            reset_on_success: false,
            queue_requests: false,
            max_queue_size: 100,
            queue_timeout_seconds: 5.0,
        };

        let manager = RateLimitManager::new(Some(config));
        assert!(manager.is_enabled());
        assert_eq!(manager.get_status("example.com"), 0.0); // Time until available
    }

    #[test]
    fn test_rate_limit_manager_disabled() {
        let manager = RateLimitManager::new(None);
        assert!(!manager.is_enabled());
        assert_eq!(manager.get_status("example.com"), 0.0); // Time until available
        assert!(manager.check_rate_limit("example.com"));
    }

    #[test]
    fn test_invalid_config() {
        let invalid_config = RateLimitConfig {
            enabled: true,
            algorithm: RateLimitAlgorithm::TokenBucket,
            requests_per_second: 0.0, // Invalid: must be > 0
            requests_per_minute: Some(0),
            requests_per_hour: Some(0),
            burst_size: Some(5),
            window_size_seconds: 1.0,
            per_host: true,
            reset_on_success: false,
            queue_requests: false,
            max_queue_size: 100,
            queue_timeout_seconds: 5.0,
        };

        let mut manager = RateLimitManager::new(None);
        assert!(manager.update_config(Some(invalid_config)).is_err());
    }

    #[tokio::test]
    async fn test_async_rate_limit_manager() {
        let config = RateLimitConfig {
            enabled: true,
            algorithm: RateLimitAlgorithm::TokenBucket,
            requests_per_second: 10.0,
            requests_per_minute: Some(600),
            requests_per_hour: Some(36000),
            burst_size: Some(5),
            window_size_seconds: 1.0,
            per_host: true,
            reset_on_success: false,
            queue_requests: false,
            max_queue_size: 100,
            queue_timeout_seconds: 5.0,
        };

        let manager = AsyncRateLimitManager::new(Some(config));
        assert!(manager.is_enabled().await);
        assert_eq!(manager.get_status("example.com").await, 0.0);
        assert!(manager.check_rate_limit("example.com").await);
    }
}
