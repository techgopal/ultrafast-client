//! HTTP Client Middleware
//!
//! This module provides a middleware system for intercepting and modifying
//! HTTP requests and responses in the UltraFast HTTP Client.

use crate::config::{RateLimitAlgorithm, RateLimitConfig};
use crate::error::UltraFastError;
use crate::response::Response;
use ahash::AHashMap;
use pyo3::prelude::*;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Base middleware trait for HTTP request/response processing
#[pyclass(subclass)]
pub struct Middleware {
    pub name: String,
}

impl Middleware {
    pub fn new(name: String) -> Self {
        Middleware { name }
    }
}

/// Comprehensive middleware stack with improved error handling
pub struct MiddlewareStack {
    pub logging_middleware: Vec<LoggingMiddleware>,
    pub headers_middleware: Vec<HeadersMiddleware>,
    pub retry_middleware: Vec<RetryMiddleware>,
    pub metrics_middleware: Vec<MetricsMiddleware>,
    pub interceptor_middleware: Vec<InterceptorMiddleware>,
    pub rate_limit_middleware: Vec<RateLimitMiddleware>,
}

impl MiddlewareStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.logging_middleware.len()
            + self.headers_middleware.len()
            + self.retry_middleware.len()
            + self.metrics_middleware.len()
            + self.interceptor_middleware.len()
            + self.rate_limit_middleware.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // Safe middleware addition methods
    pub fn add_logging_middleware(&mut self, middleware: LoggingMiddleware) {
        if self.logging_middleware.len() < 100 {
            // Prevent unbounded growth
            self.logging_middleware.push(middleware);
        }
    }

    pub fn add_headers_middleware(&mut self, middleware: HeadersMiddleware) {
        if self.headers_middleware.len() < 100 {
            self.headers_middleware.push(middleware);
        }
    }

    pub fn add_retry_middleware(&mut self, middleware: RetryMiddleware) {
        if self.retry_middleware.len() < 100 {
            self.retry_middleware.push(middleware);
        }
    }

    pub fn add_metrics_middleware(&mut self, middleware: MetricsMiddleware) {
        if self.metrics_middleware.len() < 100 {
            self.metrics_middleware.push(middleware);
        }
    }

    pub fn add_interceptor_middleware(&mut self, middleware: InterceptorMiddleware) {
        if self.interceptor_middleware.len() < 100 {
            self.interceptor_middleware.push(middleware);
        }
    }

    pub fn add_rate_limit_middleware(&mut self, middleware: RateLimitMiddleware) {
        if self.rate_limit_middleware.len() < 100 {
            self.rate_limit_middleware.push(middleware);
        }
    }
}

impl Default for MiddlewareStack {
    fn default() -> Self {
        Self {
            logging_middleware: Vec::new(),
            headers_middleware: Vec::new(),
            retry_middleware: Vec::new(),
            metrics_middleware: Vec::new(),
            interceptor_middleware: Vec::new(),
            rate_limit_middleware: Vec::new(),
        }
    }
}

/// Middleware manager with improved thread safety
pub struct MiddlewareManager {
    pub middleware_stack: Arc<RwLock<MiddlewareStack>>,
}

impl MiddlewareManager {
    pub fn new() -> Self {
        Self {
            middleware_stack: Arc::new(RwLock::new(MiddlewareStack::new())),
        }
    }

    // Improved methods with proper error handling
    pub fn add_logging_middleware(&self, middleware: LoggingMiddleware) -> Result<(), String> {
        match self.middleware_stack.write() {
            Ok(mut stack) => {
                stack.add_logging_middleware(middleware);
                Ok(())
            }
            Err(_) => Err("Failed to acquire middleware stack lock".to_string()),
        }
    }

    pub fn add_headers_middleware(&self, middleware: HeadersMiddleware) -> Result<(), String> {
        match self.middleware_stack.write() {
            Ok(mut stack) => {
                stack.add_headers_middleware(middleware);
                Ok(())
            }
            Err(_) => Err("Failed to acquire middleware stack lock".to_string()),
        }
    }

    pub fn add_retry_middleware(&self, middleware: RetryMiddleware) -> Result<(), String> {
        match self.middleware_stack.write() {
            Ok(mut stack) => {
                stack.add_retry_middleware(middleware);
                Ok(())
            }
            Err(_) => Err("Failed to acquire middleware stack lock".to_string()),
        }
    }

    pub fn add_metrics_middleware(&self, middleware: MetricsMiddleware) -> Result<(), String> {
        match self.middleware_stack.write() {
            Ok(mut stack) => {
                stack.add_metrics_middleware(middleware);
                Ok(())
            }
            Err(_) => Err("Failed to acquire middleware stack lock".to_string()),
        }
    }

    pub fn add_interceptor_middleware(
        &self,
        middleware: InterceptorMiddleware,
    ) -> Result<(), String> {
        match self.middleware_stack.write() {
            Ok(mut stack) => {
                stack.add_interceptor_middleware(middleware);
                Ok(())
            }
            Err(_) => Err("Failed to acquire middleware stack lock".to_string()),
        }
    }

    pub fn add_rate_limit_middleware(&self, middleware: RateLimitMiddleware) {
        if let Ok(mut stack) = self.middleware_stack.write() {
            stack.add_rate_limit_middleware(middleware);
        }
    }

    pub fn len(&self) -> usize {
        match self.middleware_stack.read() {
            Ok(stack) => stack.len(),
            Err(_) => 0,
        }
    }

    /// Apply headers middleware with error handling
    pub fn apply_headers_middleware(&self, headers: &mut HashMap<String, String>) {
        if let Ok(stack) = self.middleware_stack.read() {
            for middleware in &stack.headers_middleware {
                *headers = middleware.apply_headers(headers.clone());
            }
        }
    }

    /// Log request with error handling
    pub fn log_request(&self, method: &str, url: &str) {
        if let Ok(stack) = self.middleware_stack.read() {
            for middleware in &stack.logging_middleware {
                middleware.log_request(method, url);
            }
        }
    }

    /// Check rate limit with proper error handling
    pub fn check_rate_limit(&self, host: &str) -> Result<(), String> {
        match self.middleware_stack.read() {
            Ok(stack) => {
                for middleware in &stack.rate_limit_middleware {
                    if let Err(e) = middleware.check_rate_limit(host) {
                        return Err(format!("Rate limit check failed: {}", e));
                    }
                }
                Ok(())
            }
            Err(_) => Err("Failed to acquire middleware stack lock".to_string()),
        }
    }

    /// Log response with error handling
    pub fn log_response(&self, status_code: u16, response_time_ms: f64) {
        if let Ok(stack) = self.middleware_stack.read() {
            for middleware in &stack.logging_middleware {
                middleware.log_response(status_code, response_time_ms);
            }
        }
    }

    /// Update metrics with error handling
    pub fn update_metrics(&self, response_time: f64, is_error: bool) {
        if let Ok(stack) = self.middleware_stack.read() {
            for middleware in &stack.metrics_middleware {
                middleware.update_metrics(response_time, is_error);
            }
        }
    }

    /// Get rate limit status with error handling
    pub fn get_rate_limit_status(&self, host: &str) -> f64 {
        match self.middleware_stack.read() {
            Ok(stack) => {
                for middleware in &stack.rate_limit_middleware {
                    return middleware.get_status(host);
                }
                0.0
            }
            Err(_) => 0.0,
        }
    }

    /// Reset rate limits with error handling
    pub fn reset_rate_limits(&self) {
        if let Ok(stack) = self.middleware_stack.read() {
            for middleware in &stack.rate_limit_middleware {
                middleware.reset();
            }
        }
    }

    pub fn get_middleware(&self, name: &str) -> PyResult<Option<PyObject>> {
        let stack = self.middleware_stack.read().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire read lock: {}", e))
        })?;

        // Check each middleware type for the given name
        // This is a simplified implementation - in a real system you'd have a unified lookup
        Ok(None) // Placeholder - middleware lookup would need to be implemented properly
    }
}

/// Default implementation for MiddlewareManager
impl Default for MiddlewareManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Logging middleware for request/response logging
#[pyclass(subclass)]
#[derive(Clone)]
pub struct LoggingMiddleware {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub enabled: bool,
    #[pyo3(get)]
    pub log_requests: bool,
    #[pyo3(get)]
    pub log_responses: bool,
    #[pyo3(get)]
    pub log_errors: bool,
}

#[pymethods]
impl LoggingMiddleware {
    #[new]
    pub fn new(
        name: String,
        enabled: bool,
        log_requests: bool,
        log_responses: bool,
        log_errors: bool,
    ) -> Self {
        Self {
            name,
            enabled,
            log_requests,
            log_responses,
            log_errors,
        }
    }

    pub fn log_request(&self, method: &str, url: &str) {
        if self.enabled && self.log_requests {
            println!("[LOG] {} {}", method, url);
        }
    }

    pub fn log_response(&self, status_code: u16, response_time_ms: f64) {
        if self.enabled && self.log_responses {
            println!("[LOG] Response: {} ({}ms)", status_code, response_time_ms);
        }
    }

    pub fn log_error(&self, error: &str) {
        if self.enabled && self.log_errors {
            eprintln!("[LOG] Error: {}", error);
        }
    }
}

/// Headers middleware for request header management
#[pyclass(subclass)]
#[derive(Clone)]
pub struct HeadersMiddleware {
    #[pyo3(get)]
    pub name: String,
    pub default_headers: HashMap<String, String>,
    #[pyo3(get)]
    pub max_header_size: usize,
}

#[pymethods]
impl HeadersMiddleware {
    #[new]
    pub fn new(
        name: String,
        default_headers: HashMap<String, String>,
        max_header_size: Option<usize>,
    ) -> Self {
        Self {
            name,
            default_headers,
            max_header_size: max_header_size.unwrap_or(8192),
        }
    }

    pub fn validate_header(&self, key: &str, value: &str) -> bool {
        !key.is_empty()
            && !value.is_empty()
            && key.len() + value.len() <= self.max_header_size
            && !key.contains('\0')
            && !value.contains('\0')
    }
    pub fn apply_headers(&self, headers: HashMap<String, String>) -> HashMap<String, String> {
        let mut result = headers;
        for (key, value) in &self.default_headers {
            if !result.contains_key(key) && self.validate_header(key, value) {
                result.insert(key.clone(), value.clone());
            }
        }
        result
    }

    #[getter]
    pub fn default_headers(&self, py: pyo3::Python) -> pyo3::PyObject {
        self.default_headers.clone().into_py(py)
    }
}

/// Retry middleware with exponential backoff
#[pyclass(subclass)]
#[derive(Clone)]
pub struct RetryMiddleware {
    #[pyo3(get)]
    pub name: String,
    pub max_retries: u32,
    pub retry_on_status: Vec<u16>,
    pub retry_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub max_delay_ms: u64,
    pub jitter: bool,
}

#[pymethods]
impl RetryMiddleware {
    #[new]
    pub fn new(
        name: String,
        max_retries: u32,
        retry_on_status: Vec<u16>,
        retry_delay_ms: u64,
        backoff_multiplier: f64,
        max_delay_ms: u64,
        jitter: bool,
    ) -> Self {
        Self {
            name,
            max_retries,
            retry_on_status,
            retry_delay_ms,
            backoff_multiplier,
            max_delay_ms,
            jitter,
        }
    }
}

impl Default for RetryMiddleware {
    fn default() -> Self {
        Self {
            name: "default_retry".to_string(),
            max_retries: 3,
            retry_on_status: vec![500, 502, 503, 504],
            retry_delay_ms: 1000,
            backoff_multiplier: 2.0,
            max_delay_ms: 30000,
            jitter: true,
        }
    }
}

/// Metrics middleware with atomic operations
#[pyclass(subclass)]
pub struct MetricsMiddleware {
    pub name: String,
    pub enabled: bool,
    // Use thread-safe storage
    pub start_times: Arc<RwLock<AHashMap<String, Instant>>>,
    // Store metrics atomically
    pub total_requests: std::sync::atomic::AtomicU64,
    pub error_count: std::sync::atomic::AtomicU64,
    pub total_response_time: std::sync::atomic::AtomicU64, // Store as nanoseconds
}

impl Clone for MetricsMiddleware {
    fn clone(&self) -> Self {
        MetricsMiddleware {
            name: self.name.clone(),
            enabled: self.enabled,
            start_times: Arc::clone(&self.start_times),
            total_requests: std::sync::atomic::AtomicU64::new(
                self.total_requests
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            error_count: std::sync::atomic::AtomicU64::new(
                self.error_count.load(std::sync::atomic::Ordering::Relaxed),
            ),
            total_response_time: std::sync::atomic::AtomicU64::new(
                self.total_response_time
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
        }
    }
}

impl MetricsMiddleware {
    pub fn new(name: String, enabled: bool) -> Self {
        Self {
            name,
            enabled,
            start_times: Arc::new(RwLock::new(AHashMap::new())),
            total_requests: std::sync::atomic::AtomicU64::new(0),
            error_count: std::sync::atomic::AtomicU64::new(0),
            total_response_time: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn start_request(&self, request_id: String) {
        if !self.enabled {
            return;
        }

        if let Ok(mut times) = self.start_times.write() {
            // Limit the number of tracked requests to prevent memory leaks
            if times.len() < 10000 {
                times.insert(request_id, Instant::now());
            }
        }
    }

    pub fn end_request(&self, request_id: &str) -> Option<f64> {
        if !self.enabled {
            return None;
        }

        if let Ok(mut times) = self.start_times.write() {
            if let Some(start_time) = times.remove(request_id) {
                return Some(start_time.elapsed().as_secs_f64());
            }
        }
        None
    }

    pub fn update_metrics(&self, response_time: f64, is_error: bool) {
        if !self.enabled {
            return;
        }

        self.total_requests
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        if is_error {
            self.error_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        // Convert to nanoseconds for storage
        let response_time_nanos = (response_time * 1_000_000_000.0) as u64;
        self.total_response_time
            .fetch_add(response_time_nanos, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_metrics(&self) -> (u64, u64, f64) {
        let total = self
            .total_requests
            .load(std::sync::atomic::Ordering::Relaxed);
        let errors = self.error_count.load(std::sync::atomic::Ordering::Relaxed);
        let total_time_nanos = self
            .total_response_time
            .load(std::sync::atomic::Ordering::Relaxed);
        let avg_time = if total > 0 {
            (total_time_nanos as f64) / (total as f64) / 1_000_000_000.0 // Convert back to seconds
        } else {
            0.0
        };

        (total, errors, avg_time)
    }

    pub fn reset(&self) {
        if let Ok(mut times) = self.start_times.write() {
            times.clear();
        }
        self.total_requests
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.error_count
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.total_response_time
            .store(0, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Interceptor middleware for custom request/response processing
#[pyclass(subclass)]
#[derive(Clone)]
pub struct InterceptorMiddleware {
    #[pyo3(get)]
    pub name: String,

    pub enabled: bool,

    // Python callable for request interception
    pub request_interceptor: Option<PyObject>,

    // Python callable for response interception
    pub response_interceptor: Option<PyObject>,

    // Python callable for error interception
    pub error_interceptor: Option<PyObject>,
}

impl InterceptorMiddleware {
    pub fn new(
        name: String,
        request_interceptor: Option<PyObject>,
        response_interceptor: Option<PyObject>,
        error_interceptor: Option<PyObject>,
        enabled: bool,
    ) -> Self {
        Self {
            name,
            enabled,
            request_interceptor,
            response_interceptor,
            error_interceptor,
        }
    }

    /// Enable the interceptor
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable the interceptor
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Check if interceptor has request interceptor
    pub fn has_request_interceptor(&self) -> bool {
        self.request_interceptor.is_some()
    }

    /// Check if interceptor has response interceptor
    pub fn has_response_interceptor(&self) -> bool {
        self.response_interceptor.is_some()
    }

    /// Check if interceptor has error interceptor
    pub fn has_error_interceptor(&self) -> bool {
        self.error_interceptor.is_some()
    }
}

/// Token bucket for rate limiting
struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    fn try_consume(&mut self, tokens: f64) -> bool {
        self.refill();
        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }

    fn time_until_available(&self, tokens: f64) -> Duration {
        if self.tokens >= tokens {
            Duration::from_secs(0)
        } else {
            let needed_tokens = tokens - self.tokens;
            let time_to_refill = needed_tokens / self.refill_rate;
            Duration::from_secs_f64(time_to_refill)
        }
    }
}

/// Sliding window for rate limiting
struct SlidingWindow {
    window_size: Duration,
    max_requests: u32,
    requests: VecDeque<Instant>,
}

impl SlidingWindow {
    fn new(window_size: Duration, max_requests: u32) -> Self {
        Self {
            window_size,
            max_requests,
            requests: VecDeque::new(),
        }
    }

    fn try_consume(&mut self) -> bool {
        let now = Instant::now();
        self.cleanup_old_requests(now);

        if self.requests.len() < self.max_requests as usize {
            self.requests.push_back(now);
            true
        } else {
            false
        }
    }

    fn cleanup_old_requests(&mut self, now: Instant) {
        while let Some(&front) = self.requests.front() {
            if now.duration_since(front) > self.window_size {
                self.requests.pop_front();
            } else {
                break;
            }
        }
    }

    fn time_until_available(&self) -> Duration {
        if self.requests.len() < self.max_requests as usize {
            Duration::from_secs(0)
        } else if let Some(&oldest) = self.requests.front() {
            let elapsed = Instant::now().duration_since(oldest);
            if elapsed >= self.window_size {
                Duration::from_secs(0)
            } else {
                self.window_size - elapsed
            }
        } else {
            Duration::from_secs(0)
        }
    }
}

/// Fixed window for rate limiting
struct FixedWindow {
    window_size: Duration,
    max_requests: u32,
    current_count: u32,
    window_start: Instant,
}

impl FixedWindow {
    fn new(window_size: Duration, max_requests: u32) -> Self {
        Self {
            window_size,
            max_requests,
            current_count: 0,
            window_start: Instant::now(),
        }
    }

    fn try_consume(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.window_start) >= self.window_size {
            self.current_count = 0;
            self.window_start = now;
        }

        if self.current_count < self.max_requests {
            self.current_count += 1;
            true
        } else {
            false
        }
    }

    fn time_until_available(&self) -> Duration {
        let now = Instant::now();
        if self.current_count < self.max_requests {
            Duration::from_secs(0)
        } else {
            let elapsed = now.duration_since(self.window_start);
            if elapsed >= self.window_size {
                Duration::from_secs(0)
            } else {
                self.window_size - elapsed
            }
        }
    }
}

/// Request queue entry for rate limiting
struct QueuedRequest {
    enqueued_at: Instant,
    host: String,
}

/// Rate limiting middleware for controlling request rates
#[pyclass(subclass)]
pub struct RateLimitMiddleware {
    pub name: String,
    pub enabled: bool,

    pub config: RateLimitConfig,

    // Internal state - not exposed to Python
    pub token_buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
    pub sliding_windows: Arc<RwLock<HashMap<String, SlidingWindow>>>,
    pub fixed_windows: Arc<RwLock<HashMap<String, FixedWindow>>>,
    pub request_queue: Arc<RwLock<VecDeque<QueuedRequest>>>,
    pub global_bucket: Arc<RwLock<Option<TokenBucket>>>,
    pub global_sliding: Arc<RwLock<Option<SlidingWindow>>>,
    pub global_fixed: Arc<RwLock<Option<FixedWindow>>>,
}

impl Clone for RateLimitMiddleware {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            enabled: self.enabled,
            config: self.config.clone(),
            token_buckets: Arc::clone(&self.token_buckets),
            sliding_windows: Arc::clone(&self.sliding_windows),
            fixed_windows: Arc::clone(&self.fixed_windows),
            request_queue: Arc::clone(&self.request_queue),
            global_bucket: Arc::clone(&self.global_bucket),
            global_sliding: Arc::clone(&self.global_sliding),
            global_fixed: Arc::clone(&self.global_fixed),
        }
    }
}

impl RateLimitMiddleware {
    pub fn new(name: String, config: RateLimitConfig, enabled: bool) -> PyResult<Self> {
        config.validate().map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid rate limit config: {}", e))
        })?;

        let middleware = Self {
            name,
            enabled,
            config: config.clone(),
            token_buckets: Arc::new(RwLock::new(HashMap::new())),
            sliding_windows: Arc::new(RwLock::new(HashMap::new())),
            fixed_windows: Arc::new(RwLock::new(HashMap::new())),
            request_queue: Arc::new(RwLock::new(VecDeque::new())),
            global_bucket: Arc::new(RwLock::new(None)),
            global_sliding: Arc::new(RwLock::new(None)),
            global_fixed: Arc::new(RwLock::new(None)),
        };

        // Initialize global rate limiters if per_host is false
        if !config.per_host {
            middleware.init_global_limiters()?;
        }

        Ok(middleware)
    }

    /// Create from RateLimitConfig
    pub fn from_config(name: String, config: RateLimitConfig) -> PyResult<Self> {
        Self::new(name, config, true)
    }

    /// Check if request can proceed
    pub fn can_proceed(&self, host: &str) -> bool {
        if !self.enabled || !self.config.enabled {
            return true;
        }

        match self.config.algorithm {
            RateLimitAlgorithm::TokenBucket => self.check_token_bucket(host),
            RateLimitAlgorithm::SlidingWindow => self.check_sliding_window(host),
            RateLimitAlgorithm::FixedWindow => self.check_fixed_window(host),
        }
    }

    /// Check rate limit with error handling (new interface)
    pub fn check_rate_limit(&self, host: &str) -> PyResult<()> {
        if !self.enabled || !self.config.enabled {
            return Ok(());
        }

        if self.can_proceed(host) {
            Ok(())
        } else {
            let wait_time = self.time_until_available(host);
            Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Rate limit exceeded. Wait time: {:.2}s",
                wait_time
            )))
        }
    }

    /// Get rate limit status (wait time in seconds)
    pub fn get_status(&self, host: &str) -> f64 {
        if !self.enabled || !self.config.enabled {
            return 0.0;
        }

        self.time_until_available(host)
    }

    /// Wait for rate limit to be available (blocking)
    pub fn wait_for_rate_limit(&self, host: &str) -> PyResult<()> {
        if !self.enabled || !self.config.enabled {
            return Ok(());
        }

        let wait_time = self.time_until_available(host);
        if wait_time > 0.0 {
            std::thread::sleep(Duration::from_secs_f64(wait_time));
        }

        Ok(())
    }

    /// Get time until next request is allowed (in seconds)
    pub fn time_until_available(&self, host: &str) -> f64 {
        if !self.enabled || !self.config.enabled {
            return 0.0;
        }

        let duration = match self.config.algorithm {
            RateLimitAlgorithm::TokenBucket => self.time_until_token_bucket(host),
            RateLimitAlgorithm::SlidingWindow => self.time_until_sliding_window(host),
            RateLimitAlgorithm::FixedWindow => self.time_until_fixed_window(host),
        };

        duration.as_secs_f64()
    }

    /// Reset rate limiters
    pub fn reset(&self) {
        if let Ok(mut buckets) = self.token_buckets.write() {
            buckets.clear();
        }
        if let Ok(mut windows) = self.sliding_windows.write() {
            windows.clear();
        }
        if let Ok(mut windows) = self.fixed_windows.write() {
            windows.clear();
        }
        if let Ok(mut queue) = self.request_queue.write() {
            queue.clear();
        }
        if let Ok(mut bucket) = self.global_bucket.write() {
            *bucket = None;
        }
        if let Ok(mut sliding) = self.global_sliding.write() {
            *sliding = None;
        }
        if let Ok(mut fixed) = self.global_fixed.write() {
            *fixed = None;
        }

        if !self.config.per_host {
            let _ = self.init_global_limiters();
        }
    }

    /// Get current queue size
    pub fn get_queue_size(&self) -> usize {
        match self.request_queue.read() {
            Ok(queue) => queue.len(),
            Err(_) => 0,
        }
    }

    /// Check if queue is full
    pub fn is_queue_full(&self) -> bool {
        if !self.config.queue_requests {
            return false;
        }
        self.get_queue_size() >= self.config.max_queue_size
    }
}

impl RateLimitMiddleware {
    fn init_global_limiters(&self) -> PyResult<()> {
        match self.config.algorithm {
            RateLimitAlgorithm::TokenBucket => {
                let burst_size = self
                    .config
                    .burst_size
                    .unwrap_or(self.config.requests_per_second as u32)
                    as f64;
                match self.global_bucket.write() {
                    Ok(mut bucket) => {
                        *bucket = Some(TokenBucket::new(
                            burst_size,
                            self.config.requests_per_second,
                        ));
                    }
                    Err(_) => {
                        return Err(pyo3::exceptions::PyRuntimeError::new_err(
                            "Failed to initialize global token bucket",
                        ));
                    }
                }
            }
            RateLimitAlgorithm::SlidingWindow => {
                let window_requests =
                    (self.config.requests_per_second * self.config.window_size_seconds) as u32;
                match self.global_sliding.write() {
                    Ok(mut sliding) => {
                        *sliding = Some(SlidingWindow::new(
                            Duration::from_secs_f64(self.config.window_size_seconds),
                            window_requests,
                        ));
                    }
                    Err(_) => {
                        return Err(pyo3::exceptions::PyRuntimeError::new_err(
                            "Failed to initialize global sliding window",
                        ));
                    }
                }
            }
            RateLimitAlgorithm::FixedWindow => {
                let window_requests =
                    (self.config.requests_per_second * self.config.window_size_seconds) as u32;
                match self.global_fixed.write() {
                    Ok(mut fixed) => {
                        *fixed = Some(FixedWindow::new(
                            Duration::from_secs_f64(self.config.window_size_seconds),
                            window_requests,
                        ));
                    }
                    Err(_) => {
                        return Err(pyo3::exceptions::PyRuntimeError::new_err(
                            "Failed to initialize global fixed window",
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn check_token_bucket(&self, host: &str) -> bool {
        if self.config.per_host {
            match self.token_buckets.write() {
                Ok(mut buckets) => {
                    let bucket = buckets.entry(host.to_string()).or_insert_with(|| {
                        let burst_size = self
                            .config
                            .burst_size
                            .unwrap_or(self.config.requests_per_second as u32)
                            as f64;
                        TokenBucket::new(burst_size, self.config.requests_per_second)
                    });
                    bucket.try_consume(1.0)
                }
                Err(_) => false,
            }
        } else {
            match self.global_bucket.write() {
                Ok(mut global_bucket) => {
                    if let Some(bucket) = global_bucket.as_mut() {
                        bucket.try_consume(1.0)
                    } else {
                        false
                    }
                }
                Err(_) => false,
            }
        }
    }

    fn check_sliding_window(&self, host: &str) -> bool {
        if self.config.per_host {
            match self.sliding_windows.write() {
                Ok(mut windows) => {
                    let window = windows.entry(host.to_string()).or_insert_with(|| {
                        let window_requests = (self.config.requests_per_second
                            * self.config.window_size_seconds)
                            as u32;
                        SlidingWindow::new(
                            Duration::from_secs_f64(self.config.window_size_seconds),
                            window_requests,
                        )
                    });
                    window.try_consume()
                }
                Err(_) => false,
            }
        } else {
            match self.global_sliding.write() {
                Ok(mut global_sliding) => {
                    if let Some(window) = global_sliding.as_mut() {
                        window.try_consume()
                    } else {
                        false
                    }
                }
                Err(_) => false,
            }
        }
    }

    fn check_fixed_window(&self, host: &str) -> bool {
        if self.config.per_host {
            match self.fixed_windows.write() {
                Ok(mut windows) => {
                    let window = windows.entry(host.to_string()).or_insert_with(|| {
                        let window_requests = (self.config.requests_per_second
                            * self.config.window_size_seconds)
                            as u32;
                        FixedWindow::new(
                            Duration::from_secs_f64(self.config.window_size_seconds),
                            window_requests,
                        )
                    });
                    window.try_consume()
                }
                Err(_) => false,
            }
        } else {
            match self.global_fixed.write() {
                Ok(mut global_fixed) => {
                    if let Some(window) = global_fixed.as_mut() {
                        window.try_consume()
                    } else {
                        false
                    }
                }
                Err(_) => false,
            }
        }
    }

    fn time_until_token_bucket(&self, host: &str) -> Duration {
        if self.config.per_host {
            match self.token_buckets.read() {
                Ok(buckets) => {
                    if let Some(bucket) = buckets.get(host) {
                        bucket.time_until_available(1.0)
                    } else {
                        Duration::from_secs(0)
                    }
                }
                Err(_) => Duration::from_secs(60), // Default wait time on error
            }
        } else {
            match self.global_bucket.read() {
                Ok(global_bucket) => {
                    if let Some(bucket) = global_bucket.as_ref() {
                        bucket.time_until_available(1.0)
                    } else {
                        Duration::from_secs(0)
                    }
                }
                Err(_) => Duration::from_secs(60), // Default wait time on error
            }
        }
    }

    fn time_until_sliding_window(&self, host: &str) -> Duration {
        if self.config.per_host {
            match self.sliding_windows.read() {
                Ok(windows) => {
                    if let Some(window) = windows.get(host) {
                        window.time_until_available()
                    } else {
                        Duration::from_secs(0)
                    }
                }
                Err(_) => Duration::from_secs(60), // Default wait time on error
            }
        } else {
            match self.global_sliding.read() {
                Ok(global_sliding) => {
                    if let Some(window) = global_sliding.as_ref() {
                        window.time_until_available()
                    } else {
                        Duration::from_secs(0)
                    }
                }
                Err(_) => Duration::from_secs(60), // Default wait time on error
            }
        }
    }

    fn time_until_fixed_window(&self, host: &str) -> Duration {
        if self.config.per_host {
            match self.fixed_windows.read() {
                Ok(windows) => {
                    if let Some(window) = windows.get(host) {
                        window.time_until_available()
                    } else {
                        Duration::from_secs(0)
                    }
                }
                Err(_) => Duration::from_secs(60), // Default wait time on error
            }
        } else {
            match self.global_fixed.read() {
                Ok(global_fixed) => {
                    if let Some(window) = global_fixed.as_ref() {
                        window.time_until_available()
                    } else {
                        Duration::from_secs(0)
                    }
                }
                Err(_) => Duration::from_secs(60), // Default wait time on error
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{RateLimitAlgorithm, RateLimitConfig};

    #[test]
    fn test_middleware_stack_creation() {
        let stack = MiddlewareStack::new();
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_middleware_manager_creation() {
        let manager = MiddlewareManager::new();

        // Should start with empty middleware stack
        let stack = manager
            .middleware_stack
            .read()
            .expect("Failed to acquire read lock");
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_rate_limit_config_creation() {
        let config = RateLimitConfig::new(
            true,
            RateLimitAlgorithm::TokenBucket,
            10.0,
            Some(600),
            Some(3600),
            Some(20),
            1.0,
            true,
            false,
            true,
            100,
            30.0,
        );

        assert!(config.enabled);
        assert_eq!(config.requests_per_second, 10.0);
        assert_eq!(config.burst_size, Some(20));
        assert!(config.per_host);
    }

    #[test]
    fn test_rate_limit_middleware_creation() {
        let config = RateLimitConfig::new(
            true,
            RateLimitAlgorithm::TokenBucket,
            5.0,
            None,
            None,
            Some(10),
            1.0,
            true,
            false,
            false,
            0,
            0.0,
        );

        let middleware = RateLimitMiddleware::new("test_middleware".to_string(), config, true);
        assert!(middleware.is_ok());

        let middleware = middleware.expect("Failed to create middleware");
        assert!(middleware.enabled);
        assert_eq!(middleware.config.requests_per_second, 5.0);
    }

    #[test]
    fn test_token_bucket_rate_limiting() {
        let config = RateLimitConfig::new(
            true,
            RateLimitAlgorithm::TokenBucket,
            2.0, // 2 requests per second
            None,
            None,
            Some(5), // burst of 5
            1.0,
            true,
            false,
            false,
            0,
            0.0,
        );

        let middleware = RateLimitMiddleware::new("test_middleware".to_string(), config, true)
            .expect("Failed to create middleware");
        let host = "test.example.com";

        // Should allow burst requests initially
        for _ in 0..5 {
            assert!(middleware.can_proceed(host));
        }

        // Should be rate limited after burst
        assert!(!middleware.can_proceed(host));

        // Check that time until available is reasonable
        let wait_time = middleware.time_until_available(host);
        assert!(wait_time > 0.0);
        assert!(wait_time <= 1.0); // Should be less than 1 second for 2 RPS
    }

    #[test]
    fn test_sliding_window_rate_limiting() {
        let config = RateLimitConfig::new(
            true,
            RateLimitAlgorithm::SlidingWindow,
            3.0, // 3 requests per second
            None,
            None,
            None,
            1.0, // 1 second window
            true,
            false,
            false,
            0,
            0.0,
        );

        let middleware = RateLimitMiddleware::new("test_middleware".to_string(), config, true)
            .expect("Failed to create middleware");
        let host = "test.example.com";

        // Should allow requests within rate limit
        for _ in 0..3 {
            assert!(middleware.can_proceed(host));
        }

        // Should be rate limited after exceeding rate
        assert!(!middleware.can_proceed(host));
    }

    #[test]
    fn test_fixed_window_rate_limiting() {
        let config = RateLimitConfig::new(
            true,
            RateLimitAlgorithm::FixedWindow,
            2.0, // 2 requests per second
            None,
            None,
            None,
            1.0, // 1 second window
            true,
            false,
            false,
            0,
            0.0,
        );

        let middleware = RateLimitMiddleware::new("test_middleware".to_string(), config, true)
            .expect("Failed to create middleware");
        let host = "test.example.com";

        // Should allow requests within window limit
        for _ in 0..2 {
            assert!(middleware.can_proceed(host));
        }

        // Should be rate limited after exceeding window limit
        assert!(!middleware.can_proceed(host));
    }

    #[test]
    fn test_per_host_rate_limiting() {
        let config = RateLimitConfig::new(
            true,
            RateLimitAlgorithm::TokenBucket,
            1.0, // 1 request per second
            None,
            None,
            Some(1), // burst of 1
            1.0,
            true, // per_host = true
            false,
            false,
            0,
            0.0,
        );

        let middleware = RateLimitMiddleware::new("test_middleware".to_string(), config, true)
            .expect("Failed to create middleware");

        // Different hosts should have separate rate limits
        assert!(middleware.can_proceed("host1.example.com"));
        assert!(middleware.can_proceed("host2.example.com"));

        // Both hosts should now be rate limited
        assert!(!middleware.can_proceed("host1.example.com"));
        assert!(!middleware.can_proceed("host2.example.com"));
    }

    #[test]
    fn test_global_rate_limiting() {
        let config = RateLimitConfig::new(
            true,
            RateLimitAlgorithm::TokenBucket,
            1.0, // 1 request per second
            None,
            None,
            Some(2), // burst of 2
            1.0,
            false, // per_host = false (global)
            false,
            false,
            0,
            0.0,
        );

        let middleware = RateLimitMiddleware::new("test_middleware".to_string(), config, true)
            .expect("Failed to create middleware");

        // Global rate limiting - requests to different hosts share the limit
        assert!(middleware.can_proceed("host1.example.com"));
        assert!(middleware.can_proceed("host2.example.com"));

        // Now both hosts should be rate limited (shared global limit)
        assert!(!middleware.can_proceed("host1.example.com"));
        assert!(!middleware.can_proceed("host3.example.com"));
    }

    #[test]
    fn test_disabled_rate_limiting() {
        let config = RateLimitConfig::disabled();
        let middleware = RateLimitMiddleware::new("test_middleware".to_string(), config, false)
            .expect("Failed to create middleware");

        // Disabled middleware should always allow requests
        for _ in 0..100 {
            assert!(middleware.can_proceed("example.com"));
        }

        assert_eq!(middleware.time_until_available("example.com"), 0.0);
    }

    #[test]
    fn test_rate_limit_reset() {
        let config = RateLimitConfig::new(
            true,
            RateLimitAlgorithm::TokenBucket,
            1.0,
            None,
            None,
            Some(1),
            1.0,
            true,
            false,
            false,
            0,
            0.0,
        );

        let middleware = RateLimitMiddleware::new("test_middleware".to_string(), config, true)
            .expect("Failed to create middleware");
        let host = "test.example.com";

        // Consume the token
        assert!(middleware.can_proceed(host));
        assert!(!middleware.can_proceed(host));

        // Reset should restore the token
        middleware.reset();
        assert!(middleware.can_proceed(host));
    }

    #[test]
    fn test_queue_functionality() {
        let config = RateLimitConfig::new(
            true,
            RateLimitAlgorithm::TokenBucket,
            1.0,
            None,
            None,
            Some(1),
            1.0,
            true,
            false,
            true, // queue_requests = true
            5,    // max_queue_size = 5
            30.0,
        );

        let middleware = RateLimitMiddleware::new("test_middleware".to_string(), config, true)
            .expect("Failed to create middleware");

        // Initially queue should be empty
        assert_eq!(middleware.get_queue_size(), 0);
        assert!(!middleware.is_queue_full());
    }

    #[test]
    fn test_middleware_manager_rate_limiting() {
        let manager = MiddlewareManager::new();

        let config = RateLimitConfig::new(
            true,
            RateLimitAlgorithm::TokenBucket,
            2.0,
            None,
            None,
            Some(3),
            1.0,
            true,
            false,
            false,
            0,
            0.0,
        );

        let middleware = RateLimitMiddleware::new("test_middleware".to_string(), config, true)
            .expect("Failed to create middleware");

        // Add rate limiting middleware to manager
        {
            let mut stack = manager
                .middleware_stack
                .write()
                .expect("Failed to acquire write lock");
            stack.add_rate_limit_middleware(middleware);
        }

        let host = "test.example.com";

        // Should allow initial requests
        assert!(manager.check_rate_limit(host).is_ok());
        assert!(manager.check_rate_limit(host).is_ok());
        assert!(manager.check_rate_limit(host).is_ok());

        // Should be rate limited after burst
        assert!(manager.check_rate_limit(host).is_err());

        // Check status
        let status = manager.get_rate_limit_status(host);
        assert!(status > 0.0);
    }

    #[test]
    fn test_rate_limit_error_messages() {
        let manager = MiddlewareManager::new();

        let config = RateLimitConfig::new(
            true,
            RateLimitAlgorithm::TokenBucket,
            1.0,
            None,
            None,
            Some(1),
            1.0,
            true,
            false,
            false,
            0,
            0.0,
        );

        let middleware = RateLimitMiddleware::new("test_middleware".to_string(), config, true)
            .expect("Failed to create middleware");

        {
            let mut stack = manager
                .middleware_stack
                .write()
                .expect("Failed to acquire write lock");
            stack.add_rate_limit_middleware(middleware);
        }

        let host = "test.example.com";

        // First request should succeed
        assert!(manager.check_rate_limit(host).is_ok());

        // Second request should fail with appropriate error
        let result = manager.check_rate_limit(host);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.contains("Rate limit exceeded"));
    }

    #[test]
    fn test_concurrent_rate_limiting() -> Result<(), Box<dyn std::error::Error>> {
        use std::sync::Arc;
        use std::thread;

        let config = RateLimitConfig::new(
            true,
            RateLimitAlgorithm::TokenBucket,
            10.0,
            None,
            None,
            Some(20),
            1.0,
            true,
            false,
            false,
            0,
            0.0,
        );

        let middleware = Arc::new(
            RateLimitMiddleware::new("test_middleware".to_string(), config, true)
                .expect("Failed to create middleware"),
        );
        let host = "test.example.com";

        let mut handles = vec![];
        let results = Arc::new(std::sync::Mutex::new(Vec::new()));

        // Spawn multiple threads to test concurrent access
        for _ in 0..10 {
            let middleware_clone = Arc::clone(&middleware);
            let results_clone = Arc::clone(&results);
            let host_clone = host.to_string();

            let handle = thread::spawn(move || {
                let can_proceed = middleware_clone.can_proceed(&host_clone);
                results_clone
                    .lock()
                    .expect("Failed to acquire lock")
                    .push(can_proceed);
            });

            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to join thread: {:?}", e))
            })?;
        }

        let results = results.lock().expect("Failed to acquire lock");
        assert_eq!(results.len(), 10);

        // Some requests should be allowed, some might be denied
        let allowed_count = results.iter().filter(|&&x| x).count();
        assert!(allowed_count > 0);
        assert!(allowed_count <= 20); // Should not exceed burst size

        Ok(())
    }

    #[test]
    fn test_middleware_manager_reset() {
        let manager = MiddlewareManager::new();

        let config = RateLimitConfig::new(
            true,
            RateLimitAlgorithm::TokenBucket,
            1.0,
            None,
            None,
            Some(1),
            1.0,
            true,
            false,
            false,
            0,
            0.0,
        );

        let middleware = RateLimitMiddleware::new("test_middleware".to_string(), config, true)
            .expect("Failed to create middleware");

        {
            let mut stack = manager
                .middleware_stack
                .write()
                .expect("Failed to acquire write lock");
            stack.add_rate_limit_middleware(middleware);
        }

        let host = "test.example.com";

        // Consume rate limit
        assert!(manager.check_rate_limit(host).is_ok());
        assert!(manager.check_rate_limit(host).is_err());

        // Reset and check again
        manager.reset_rate_limits();
        assert!(manager.check_rate_limit(host).is_ok());
    }
}
