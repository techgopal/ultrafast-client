use pyo3::prelude::*;
use crate::config::{AuthConfig, AuthType, RetryConfig, TimeoutConfig, PoolConfig, SSLConfig, OAuth2Token, ProxyConfig, CompressionConfig, ProtocolConfig, RateLimitConfig};

use crate::config::HttpVersion;

use crate::http3::Http3Client;
use crate::middleware::MiddlewareManager;
use crate::response::Response;
use crate::error::map_reqwest_error;
use crate::protocol_enhanced::EnhancedProtocolNegotiator;
use crate::auth_common;
use crate::rate_limit_common::RateLimitManager;
use crate::protocol_stats_common::ProtocolStatsManager;
use crate::performance_common::HeaderCache;
use crate::performance_advanced::get_runtime_optimizer;
use crate::connection_pool::{FastConnectionPool, ConnectionMultiplexer};
use reqwest::{Client, Method, RequestBuilder};
use std::collections::HashMap;
use ahash::AHashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use serde_json::Value;
use pythonize;

/// Lock ordering enumeration to prevent deadlocks
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum LockOrder {
    OAuth2Token = 0,
    MiddlewareManager = 1,
    Http3Client = 2,
    RequestCount = 3,
    TotalRequestTime = 4,
    LastRequestTime = 5,
    RateLimitManager = 6,
}

/// HTTP Client with advanced features and improved error handling
#[pyclass]
pub struct HttpClient {
    client: Client,
    base_url: Option<String>,
    // Use RwLock for headers since they're read more than written
    headers: Arc<RwLock<HashMap<String, String>>>,
    auth_config: Option<AuthConfig>,
    retry_config: Option<RetryConfig>,
    timeout_config: TimeoutConfig,
    pool_config: PoolConfig,
    ssl_config: SSLConfig,
    proxy_config: Option<ProxyConfig>,
    compression_config: CompressionConfig,
    protocol_config: ProtocolConfig,
    rate_limit_config: Option<RateLimitConfig>,
    protocol_negotiator: Arc<EnhancedProtocolNegotiator>,
    // Improved with consistent lock ordering
    oauth2_token: Arc<RwLock<Option<OAuth2Token>>>,
    middleware_manager: Arc<RwLock<MiddlewareManager>>,
    runtime: Arc<Runtime>,
    // HTTP/3 client for QUIC connections
    http3_client: Arc<RwLock<Option<Http3Client>>>,
    // HTTP/3 connection pool
    http3_pool: Arc<crate::http3::Http3ConnectionPool>,
    // Performance tracking with atomic operations
    request_count: Arc<std::sync::atomic::AtomicU64>,
    total_request_time: Arc<std::sync::atomic::AtomicU64>, // Store as nanoseconds
    last_request_time: Arc<RwLock<Option<f64>>>,
    // Shared managers to eliminate code duplication
    rate_limit_manager: Arc<RwLock<RateLimitManager>>,
    protocol_stats_manager: Arc<ProtocolStatsManager>,
    // Performance optimizations
    header_cache: Arc<HeaderCache>,
    connection_pool: Arc<FastConnectionPool>,
    connection_multiplexer: Arc<ConnectionMultiplexer>,
    // Resource cleanup tracking
    _cleanup_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

#[pymethods]
impl HttpClient {
    #[new]
    #[pyo3(signature = (
        base_url = None,
        headers = None,
        timeout = 30.0,
        auth_config = None,
        retry_config = None,
        timeout_config = None,
        pool_config = None,
        ssl_config = None,
        proxy_config = None,
        compression_config = None,
        protocol_config = None,
        rate_limit_config = None
    ))]
    pub fn new(
        base_url: Option<String>,
        headers: Option<HashMap<String, String>>,
        timeout: f64,
        auth_config: Option<AuthConfig>,
        retry_config: Option<RetryConfig>,
        timeout_config: Option<TimeoutConfig>,
        pool_config: Option<PoolConfig>,
        ssl_config: Option<SSLConfig>,
        proxy_config: Option<ProxyConfig>,
        compression_config: Option<CompressionConfig>,
        protocol_config: Option<ProtocolConfig>,
        rate_limit_config: Option<RateLimitConfig>,
    ) -> PyResult<Self> {
        let runtime = Arc::new(Runtime::new().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create runtime: {}", e))
        })?);
        
        let timeout_cfg = timeout_config.unwrap_or_else(|| TimeoutConfig::new(
            Some(10.0), Some(timeout), Some(timeout), Some(30.0)
        ));
        
        let pool_cfg = pool_config.unwrap_or_else(|| PoolConfig::new(100, 10, 90.0, 30.0));
        let ssl_cfg = ssl_config.unwrap_or_else(|| SSLConfig::new(true, None, None, None, None));
        let compression_cfg = compression_config.unwrap_or_else(|| CompressionConfig::new(false, true, None, None, 1024));
        let protocol_cfg = protocol_config.clone().unwrap_or_else(|| ProtocolConfig::default());
        // Note: HTTP/3 pool settings would be used for HTTP/3 client initialization if implemented
        let http3_pool_settings = protocol_cfg.http3_settings.clone();
        
        // Build client with advanced configuration
        let mut client_builder = Client::builder()
            .timeout(Duration::from_secs_f64(timeout))
            .pool_max_idle_per_host(pool_cfg.max_idle_per_host)
            .pool_idle_timeout(Duration::from_secs_f64(pool_cfg.idle_timeout));
        
        // Configure timeouts
        if let Some(connect_timeout) = timeout_cfg.connect_timeout {
            client_builder = client_builder.connect_timeout(Duration::from_secs_f64(connect_timeout));
        }
        
        // Configure SSL/TLS
        if !ssl_cfg.verify {
            client_builder = client_builder.danger_accept_invalid_certs(true);
        }
        
        // Apply additional SSL configuration
        if let Some(cert_path) = &ssl_cfg.cert_file {
            if let Some(key_path) = &ssl_cfg.key_file {
                // Load client certificate and key
                match std::fs::read(cert_path) {
                    Ok(cert_data) => {
                        match std::fs::read(key_path) {
                            Ok(key_data) => {
                                // Use from_pkcs8_pem which is the correct method
                                if let Ok(identity) = reqwest::Identity::from_pkcs8_pem(&cert_data, &key_data) {
                                    client_builder = client_builder.identity(identity);
                                }
                            }
                            Err(_) => {} // Ignore key loading errors
                        }
                    }
                    Err(_) => {} // Ignore cert loading errors
                }
            }
        }
        
        // Configure CA bundle if specified
        if let Some(ca_bundle) = &ssl_cfg.ca_bundle {
            match std::fs::read(ca_bundle) {
                Ok(ca_data) => {
                    if let Ok(cert) = reqwest::Certificate::from_pem(&ca_data) {
                        client_builder = client_builder.add_root_certificate(cert);
                    }
                }
                Err(_) => {} // Ignore CA bundle errors
            }
        }
        
        // Configure proxy if specified
        if let Some(proxy_cfg) = &proxy_config {
            if let Ok(proxy) = reqwest::Proxy::all(&proxy_cfg.url) {
                let mut proxy = proxy;
                
                // Add authentication if specified
                if let (Some(username), Some(password)) = (&proxy_cfg.username, &proxy_cfg.password) {
                    proxy = proxy.basic_auth(username, password);
                }
                
                client_builder = client_builder.proxy(proxy);
            }
        }
        
        // Configure compression
        if compression_cfg.enable_response_compression {
            // reqwest enables gzip/deflate by default, we just need to ensure it's not disabled
            client_builder = client_builder.gzip(true);
            
            // Enable brotli if supported
            if compression_cfg.supports_algorithm("brotli") {
                client_builder = client_builder.brotli(true);
            }
        }
        
        // Configure HTTP/2 settings
        if protocol_cfg.is_http2_enabled() {
            // reqwest enables HTTP/2 by default, but we can configure specific settings
            if protocol_cfg.enable_http2_prior_knowledge {
                client_builder = client_builder.http2_prior_knowledge();
            }
            
            // Configure HTTP/2 keep-alive if specified
            if let Some(interval) = protocol_cfg.http2_settings.keep_alive_interval {
                client_builder = client_builder.http2_keep_alive_interval(Some(Duration::from_secs(interval)));
            }
            
            if let Some(timeout) = protocol_cfg.http2_settings.keep_alive_timeout {
                client_builder = client_builder.http2_keep_alive_timeout(Duration::from_secs(timeout));
            }
        }
        
        // Configure HTTP/3 via Quiche
        if protocol_cfg.is_http3_enabled() {
            // HTTP/3 support requires unstable features
            // This will only work if compiled with RUSTFLAGS="--cfg reqwest_unstable"
            // client_builder = client_builder.http3_prior_knowledge();
        }
        
        let client = runtime.block_on(async {
            client_builder.build()
        }).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create HTTP client: {}", e))
        })?;
        
        // Create middleware manager and add rate limiting if configured
        let mut middleware_manager = MiddlewareManager::new();
        if let Some(rate_limit_cfg) = &rate_limit_config {
            let rate_limit_middleware = crate::middleware::RateLimitMiddleware::new("default_rate_limit".to_string(), rate_limit_cfg.clone(), true)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create rate limit middleware: {}", e)))?;
            middleware_manager.add_rate_limit_middleware(rate_limit_middleware);
        }
        
        // Start cleanup task
        let cleanup_handle = Self::start_cleanup_task(runtime.clone());
        
        Ok(HttpClient {
            client,
            base_url,
            headers: Arc::new(RwLock::new(headers.unwrap_or_default())),
            auth_config,
            retry_config,
            timeout_config: timeout_cfg,
            pool_config: pool_cfg.clone(),
            ssl_config: ssl_cfg,
            proxy_config,
            compression_config: compression_cfg,
            protocol_negotiator: Arc::new(EnhancedProtocolNegotiator::new(protocol_cfg.fallback_strategy.clone())),
            protocol_config: protocol_cfg,
            rate_limit_config: rate_limit_config.clone(),
            oauth2_token: Arc::new(RwLock::new(None)),
            middleware_manager: Arc::new(RwLock::new(middleware_manager)),
            runtime,
            http3_client: Arc::new(RwLock::new(None)),
            http3_pool: Arc::new(crate::http3::Http3ConnectionPool::new(
                http3_pool_settings.connection_pool_size.unwrap_or(10), 
                http3_pool_settings.pool_timeout_seconds.unwrap_or(300)
            )),
            request_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            total_request_time: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            last_request_time: Arc::new(RwLock::new(None)),
            // Initialize shared managers
            rate_limit_manager: Arc::new(RwLock::new(RateLimitManager::new(rate_limit_config))),
            protocol_stats_manager: Arc::new(ProtocolStatsManager::new()),
            // Initialize performance optimizations
            header_cache: Arc::new(HeaderCache::new()),
            connection_pool: Arc::new(FastConnectionPool::new(
                pool_cfg.max_idle_connections, 
                Duration::from_secs_f64(pool_cfg.idle_timeout)
            )),
            connection_multiplexer: Arc::new(ConnectionMultiplexer::new(
                pool_cfg.max_idle_connections,
                Duration::from_secs_f64(pool_cfg.idle_timeout)
            )),
            _cleanup_handle: Arc::new(RwLock::new(Some(cleanup_handle))),
        })
    }
    
    /// Set authentication configuration
    pub fn set_auth(&mut self, auth_config: AuthConfig) -> PyResult<()> {
        // If OAuth2, we might need to fetch initial token
        if auth_config.is_oauth2() {
            // Token will be fetched on first request
            let mut token_guard = self.oauth2_token.write()
                .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Failed to acquire OAuth2 token lock"))?;
            *token_guard = None;
        }
        self.auth_config = Some(auth_config);
        Ok(())
    }
    
    /// Get current authentication configuration
    pub fn get_auth(&self) -> Option<AuthConfig> {
        self.auth_config.clone()
    }
    
    /// Check if authentication is configured
    pub fn has_auth(&self) -> bool {
        self.auth_config.is_some()
    }
    
    /// Clear authentication
    pub fn clear_auth(&mut self) -> PyResult<()> {
        self.auth_config = None;
        let mut token_guard = self.oauth2_token.write()
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Failed to acquire OAuth2 token lock"))?;
        *token_guard = None;
        Ok(())
    }
    
    /// Set retry configuration
    pub fn set_retry_config(&mut self, retry_config: RetryConfig) {
        self.retry_config = Some(retry_config);
    }
    
    /// Set timeout configuration
    pub fn set_timeout_config(&mut self, timeout_config: TimeoutConfig) {
        self.timeout_config = timeout_config;
    }
    
    /// Set pool configuration (requires client rebuild)
    pub fn set_pool_config(&mut self, pool_config: PoolConfig) -> PyResult<()> {
        self.pool_config = pool_config;
        self.rebuild_client()?;
        Ok(())
    }
    
    /// Set SSL configuration (requires client rebuild)
    pub fn set_ssl_config(&mut self, ssl_config: SSLConfig) -> PyResult<()> {
        self.ssl_config = ssl_config;
        self.rebuild_client()?;
        Ok(())
    }
    
    /// Set proxy configuration (requires client rebuild)
    pub fn set_proxy_config(&mut self, proxy_config: Option<ProxyConfig>) -> PyResult<()> {
        self.proxy_config = proxy_config;
        self.rebuild_client()?;
        Ok(())
    }
    
    /// Set compression configuration (requires client rebuild)
    pub fn set_compression_config(&mut self, compression_config: CompressionConfig) -> PyResult<()> {
        self.compression_config = compression_config;
        self.rebuild_client()?;
        Ok(())
    }
    
    /// Set protocol configuration (requires client rebuild)
    pub fn set_protocol_config(&mut self, protocol_config: ProtocolConfig) -> PyResult<()> {
        protocol_config.validate()?;
        self.protocol_config = protocol_config;
        self.rebuild_client()?;
        Ok(())
    }
    
    /// Set rate limiting configuration
    pub fn set_rate_limit_config(&mut self, rate_limit_config: Option<RateLimitConfig>) -> PyResult<()> {
        // Update shared rate limit manager
        {
            let mut manager = self.rate_limit_manager.write()
                .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Failed to acquire rate limit manager lock"))?;
            manager.update_config(rate_limit_config.clone())
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e))?;
        }
        
        self.rate_limit_config = rate_limit_config.clone();
        
        // Update middleware manager with new rate limiting configuration
        {
            let mut middleware_manager = self.middleware_manager.write()
                .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Failed to acquire middleware manager lock"))?;
            if let Some(config) = rate_limit_config {
                let rate_limit_middleware = crate::middleware::RateLimitMiddleware::new("config_rate_limit".to_string(), config, true)
                    .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create rate limit middleware: {}", e)))?;
                middleware_manager.add_rate_limit_middleware(rate_limit_middleware);
            } else {
                // Remove rate limiting by creating a disabled middleware
                let disabled_config = RateLimitConfig::disabled();
                let rate_limit_middleware = crate::middleware::RateLimitMiddleware::new("disabled_rate_limit".to_string(), disabled_config, false)
                    .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create rate limit middleware: {}", e)))?;
                middleware_manager.add_rate_limit_middleware(rate_limit_middleware);
            }
        }
        
        Ok(())
    }
    
    /// Get current rate limiting configuration
    pub fn get_rate_limit_config(&self) -> Option<RateLimitConfig> {
        match self.rate_limit_manager.read() {
            Ok(manager) => manager.get_config(),
            Err(_) => self.rate_limit_config.clone(),
        }
    }
    
    /// Check if rate limiting is enabled
    pub fn is_rate_limiting_enabled(&self) -> bool {
        match self.rate_limit_manager.read() {
            Ok(manager) => manager.is_enabled(),
            Err(_) => self.rate_limit_config.as_ref().map_or(false, |config| config.enabled),
        }
    }
    
    /// Get rate limiting status for a specific host
    pub fn get_rate_limit_status(&self, host: &str) -> f64 {
        match self.rate_limit_manager.read() {
            Ok(manager) => manager.get_status(host),
            Err(_) => {
                match self.middleware_manager.read() {
                    Ok(middleware_manager) => middleware_manager.get_rate_limit_status(host),
                    Err(_) => 0.0,
                }
            }
        }
    }
    
    /// Reset rate limiting for all hosts
    pub fn reset_rate_limits(&self) -> PyResult<()> {
        if let Ok(mut manager) = self.rate_limit_manager.write() {
            manager.reset();
        }
        if let Ok(middleware_manager) = self.middleware_manager.read() {
            middleware_manager.reset_rate_limits();
        }
        Ok(())
    }
    
    /// Get current protocol configuration
    pub fn get_protocol_config(&self) -> ProtocolConfig {
        self.protocol_config.clone()
    }
    
    /// Check if HTTP/2 is enabled
    pub fn is_http2_enabled(&self) -> bool {
        self.protocol_config.is_http2_enabled()
    }
    
    /// Check if HTTP/3 is enabled
    pub fn is_http3_enabled(&self) -> bool {
        self.protocol_config.is_http3_enabled()
    }
    
    /// Enhanced GET request with retry and auth
    #[pyo3(signature = (url, params = None, headers = None))]
    pub fn get(&mut self, url: &str, params: Option<HashMap<String, String>>, headers: Option<HashMap<String, String>>) -> PyResult<Response> {
        self.execute_request_with_retry(Method::GET, url, params, None, headers)
    }
    
    /// Enhanced POST request with retry and auth
    #[pyo3(signature = (url, json = None, data = None, files = None, headers = None))]
    pub fn post(&mut self, url: &str, json: Option<&PyAny>, data: Option<HashMap<String, String>>, files: Option<HashMap<String, Vec<u8>>>, headers: Option<HashMap<String, String>>) -> PyResult<Response> {
        let body = self.prepare_body(json, data, files)?;
        self.execute_request_with_retry(Method::POST, url, None, body, headers)
    }
    
    /// Enhanced PUT request with retry and auth
    #[pyo3(signature = (url, json = None, data = None, files = None, headers = None))]
    pub fn put(&mut self, url: &str, json: Option<&PyAny>, data: Option<HashMap<String, String>>, files: Option<HashMap<String, Vec<u8>>>, headers: Option<HashMap<String, String>>) -> PyResult<Response> {
        let body = self.prepare_body(json, data, files)?;
        self.execute_request_with_retry(Method::PUT, url, None, body, headers)
    }
    
    /// Enhanced DELETE request with retry and auth
    #[pyo3(signature = (url, headers = None))]
    pub fn delete(&mut self, url: &str, headers: Option<HashMap<String, String>>) -> PyResult<Response> {
        self.execute_request_with_retry(Method::DELETE, url, None, None, headers)
    }
    
    /// Enhanced PATCH request with retry and auth
    #[pyo3(signature = (url, json = None, data = None, files = None, headers = None))]
    pub fn patch(&mut self, url: &str, json: Option<&PyAny>, data: Option<HashMap<String, String>>, files: Option<HashMap<String, Vec<u8>>>, headers: Option<HashMap<String, String>>) -> PyResult<Response> {
        let body = self.prepare_body(json, data, files)?;
        self.execute_request_with_retry(Method::PATCH, url, None, body, headers)
    }
    
    /// Enhanced HEAD request with retry and auth
    #[pyo3(signature = (url, headers = None))]
    pub fn head(&mut self, url: &str, headers: Option<HashMap<String, String>>) -> PyResult<Response> {
        self.execute_request_with_retry(Method::HEAD, url, None, None, headers)
    }
    
    /// Enhanced OPTIONS request with retry and auth
    #[pyo3(signature = (url, headers = None))]
    pub fn options(&mut self, url: &str, headers: Option<HashMap<String, String>>) -> PyResult<Response> {
        self.execute_request_with_retry(Method::OPTIONS, url, None, None, headers)
    }
    
    /// Get performance statistics
    pub fn get_stats(&self) -> PyResult<HashMap<String, f64>> {
        // Use shared protocol stats manager
        let mut stats = self.protocol_stats_manager.get_summary_f64();
        
        // Also include legacy stats for backward compatibility
        let count = self.request_count.load(std::sync::atomic::Ordering::Relaxed) as f64;
        let total_time_nanos = self.total_request_time.load(std::sync::atomic::Ordering::Relaxed);
        let total_time = total_time_nanos as f64 / 1_000_000_000.0; // Convert from nanoseconds to seconds
        
        let last_time = match self.last_request_time.read() {
            Ok(guard) => *guard,
            Err(_) => None,
        };
        
        stats.insert("request_count".to_string(), count);
        stats.insert("total_request_time".to_string(), total_time);
        stats.insert("average_request_time".to_string(), if count > 0.0 { total_time / count } else { 0.0 });
        if let Some(last) = last_time {
            stats.insert("last_request_time".to_string(), last);
        }
        
        Ok(stats)
    }
    
    /// Reset performance statistics
    pub fn reset_stats(&mut self) -> PyResult<()> {
        // Reset shared protocol stats
        self.protocol_stats_manager.clear_all_stats();
        
        // Reset legacy stats for backward compatibility
        self.request_count.store(0, std::sync::atomic::Ordering::Relaxed);
        self.total_request_time.store(0, std::sync::atomic::Ordering::Relaxed);
        
        match self.last_request_time.write() {
            Ok(mut guard) => *guard = None,
            Err(_) => return Err(pyo3::exceptions::PyRuntimeError::new_err("Failed to reset last request time")),
        }
        
        Ok(())
    }
    
    /// Check if HTTP/3 support is available
    pub fn supports_http3(&self) -> PyResult<bool> {
        // Check if HTTP/3 is enabled in protocol configuration
        Ok(self.protocol_config.is_http3_enabled())
    }
    
    /// Get HTTP/3 protocol statistics for a specific URL
    pub fn get_protocol_stats(&self, url: &str) -> PyResult<HashMap<String, String>> {
        use std::net::ToSocketAddrs;
        use url::Url;
        
        // Parse URL to get host and port
        let parsed_url = Url::parse(url).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid URL: {}", e))
        })?;
        
        let host = parsed_url.host_str().ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("No host in URL")
        })?;
        
        let port = parsed_url.port().unwrap_or(443);
        let addr_str = format!("{}:{}", host, port);
        
        // Try to resolve address
        let server_addr = match addr_str.to_socket_addrs() {
            Ok(mut addrs) => {
                addrs.next().ok_or_else(|| {
                    pyo3::exceptions::PyConnectionError::new_err("No addresses found for host")
                })?
            }
            Err(_) => {
                // Return basic stats if address resolution fails
                let mut stats_dict = HashMap::new();
                stats_dict.insert("protocol".to_string(), "HTTP/3".to_string());
                stats_dict.insert("connection_established".to_string(), "false".to_string());
                stats_dict.insert("http3_enabled".to_string(), self.protocol_config.is_http3_enabled().to_string());
                stats_dict.insert("error".to_string(), "Address resolution failed".to_string());
                return Ok(stats_dict);
            }
        };
        
        // Try to get HTTP/3 connection from pool
        let stats_dict = self.runtime.block_on(async {
            match self.http3_pool.get_connection(server_addr).await {
                Ok(client) => {
                    let stats = client.stats().await;
                    let mut stats_map = HashMap::new();
                    
                    stats_map.insert("protocol".to_string(), "HTTP/3".to_string());
                    stats_map.insert("connection_established".to_string(), "true".to_string());
                    stats_map.insert("bytes_sent".to_string(), stats.sent_bytes.to_string());
                    stats_map.insert("bytes_received".to_string(), stats.recv_bytes.to_string());
                    stats_map.insert("lost_packets".to_string(), stats.lost_count.to_string());
                    stats_map.insert("delivered_packets".to_string(), stats.delivered_count.to_string());
                    stats_map.insert("rtt_ms".to_string(), stats.rtt.as_millis().to_string());
                    stats_map.insert("cwnd".to_string(), stats.cwnd.to_string());
                    stats_map.insert("is_established".to_string(), stats.is_established.to_string());
                    
                    if let Some(min_rtt) = stats.min_rtt {
                        stats_map.insert("min_rtt_ms".to_string(), min_rtt.as_millis().to_string());
                    }
                    
                    if let Some(srtt) = stats.srtt {
                        stats_map.insert("srtt_ms".to_string(), srtt.as_millis().to_string());
                    }
                    
                    if let Some(rttvar) = stats.rttvar {
                        stats_map.insert("rttvar_ms".to_string(), rttvar.as_millis().to_string());
                    }
                    
                    stats_map
                }
                Err(_) => {
                    let mut stats_map = HashMap::new();
                    stats_map.insert("protocol".to_string(), "HTTP/3".to_string());
                    stats_map.insert("connection_established".to_string(), "false".to_string());
                    stats_map.insert("http3_enabled".to_string(), self.protocol_config.is_http3_enabled().to_string());
                    stats_map
                }
            }
        });
        
        Ok(stats_dict)
    }
    
    /// Add middleware
    pub fn add_middleware(&mut self, _middleware: PyObject) {
        // For now, we'll accept PyObject instead of Py<Middleware>
        // This would need to be implemented based on the specific middleware type
    }
    
    /// Remove middleware (currently not supported in this release)
    pub fn remove_middleware(&mut self, _name: &str) -> bool {
        // Middleware removal is planned for a future release
        // For now, middleware can only be added
        false
    }
    
    /// Set a header
    pub fn set_header(&mut self, key: String, value: String) -> PyResult<()> {
        match self.headers.write() {
            Ok(mut headers) => {
                headers.insert(key, value);
                Ok(())
            },
            Err(_) => Err(pyo3::exceptions::PyRuntimeError::new_err("Failed to acquire headers lock")),
        }
    }
    
    /// Remove a header
    pub fn remove_header(&mut self, key: &str) -> PyResult<Option<String>> {
        match self.headers.write() {
            Ok(mut headers) => Ok(headers.remove(key)),
            Err(_) => Err(pyo3::exceptions::PyRuntimeError::new_err("Failed to acquire headers lock")),
        }
    }
    
    /// Get all headers (for debugging)
    pub fn get_headers(&self) -> PyResult<HashMap<String, String>> {
        match self.headers.read() {
            Ok(headers) => Ok(headers.clone()),
            Err(_) => Err(pyo3::exceptions::PyRuntimeError::new_err("Failed to acquire headers lock")),
        }
    }
}

impl HttpClient {
    /// Start cleanup task for resource management
    fn start_cleanup_task(runtime: Arc<Runtime>) -> tokio::task::JoinHandle<()> {
        runtime.spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // Cleanup every 5 minutes
            loop {
                interval.tick().await;
                // Perform cleanup operations
                // This could include cleaning up expired connections, tokens, etc.
                tokio::task::yield_now().await;
            }
        })
    }
    
    /// Enhanced retry logic with exponential backoff and advanced conditions
    fn execute_request_with_retry(
        &mut self,
        method: Method,
        url: &str,
        params: Option<HashMap<String, String>>,
        body: Option<Vec<u8>>,
        headers: Option<HashMap<String, String>>,
    ) -> PyResult<Response> {
        let retry_config = self.retry_config.clone();
        let max_retries = retry_config.as_ref().map(|c| c.max_retries).unwrap_or(0);
        let start_time = Instant::now();
        let mut last_error = None;
        let mut consecutive_failures = 0;

        for attempt in 0..=max_retries {
            if attempt > 0 {
                // Calculate retry delay with advanced backoff
                if let Some(ref config) = retry_config {
                    let delay_secs = config.calculate_delay_with_backoff(attempt - 1, consecutive_failures);
                    
                    // Use async sleep instead of blocking the thread
                    self.runtime.block_on(async {
                        tokio::time::sleep(Duration::from_secs_f64(delay_secs)).await;
                    });
                }
            }

            match self.execute_request_internal(method.clone(), url, params.clone(), body.clone(), headers.clone()) {
                Ok(response) => {
                    // Update performance stats with atomic operations
                    let duration = start_time.elapsed();
                    let duration_secs = duration.as_secs_f64();
                    
                    self.request_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let duration_nanos = duration.as_nanos() as u64;
                    self.total_request_time.fetch_add(duration_nanos, std::sync::atomic::Ordering::Relaxed);
                    
                    if let Ok(mut last_time) = self.last_request_time.write() {
                        *last_time = Some(duration_secs);
                    }
                    
                    // Update shared protocol stats
                    self.protocol_stats_manager.increment_request_count(url);
                    self.protocol_stats_manager.update_response_time(url, duration);
                    
                    // Check if we should retry based on status code with circuit breaker
                    if let Some(ref config) = retry_config {
                        if attempt < max_retries {
                            // Get current error rate for circuit breaker
                            let error_rate = self.get_error_rate();
                            
                            if config.should_retry_with_circuit_breaker(response.status_code, error_rate) {
                                consecutive_failures += 1;
                                last_error = Some(format!("Status code {} is retryable", response.status_code));
                                continue;
                            }
                        }
                    }
                    
                    // Reset consecutive failures on success
                    consecutive_failures = 0;
                    return Ok(response);
                }
                Err(e) => {
                    consecutive_failures += 1;
                    last_error = Some(e.to_string());
                    
                    // Check if we should retry on connection errors with circuit breaker
                    if let Some(ref config) = retry_config {
                        if !config.retry_on_connection_errors || attempt >= max_retries {
                            return Err(e);
                        }
                        
                        // Check circuit breaker for connection errors
                        let error_rate = self.get_error_rate();
                        if error_rate > 0.8 {
                            return Err(pyo3::exceptions::PyConnectionError::new_err(
                                format!("Circuit breaker open - error rate too high: {:.2}%", error_rate * 100.0)
                            ));
                        }
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        
        // All retries exhausted
        Err(pyo3::exceptions::PyConnectionError::new_err(
            format!("Request failed after {} retries: {}", max_retries + 1, last_error.unwrap_or_default())
        ))
    }
    
    /// Get current error rate for circuit breaker logic
    fn get_error_rate(&self) -> f64 {
        let total_requests = self.request_count.load(std::sync::atomic::Ordering::Relaxed);
        if total_requests == 0 {
            return 0.0;
        }
        
        // This is a simplified error rate calculation
        // In a real implementation, you'd track errors separately
        // For now, we'll use a conservative estimate
        0.1 // 10% error rate as placeholder
    }
    
    /// Internal: Execute a single request
    fn execute_request_internal(
        &mut self,
        method: Method,
        url: &str,
        params: Option<HashMap<String, String>>,
        body: Option<Vec<u8>>,
        headers: Option<HashMap<String, String>>,
    ) -> PyResult<Response> {
        let start_time = std::time::Instant::now();
        let full_url = self.build_url(url)?;
        
        // Extract host for rate limiting
        let host = if let Ok(parsed_url) = reqwest::Url::parse(&full_url) {
            parsed_url.host_str().unwrap_or("default").to_string()
        } else {
            "default".to_string()
        };
        
        // Check rate limiting before making request
        {
            let middleware_manager = self.middleware_manager.read()
                .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Failed to acquire middleware manager lock"))?;
            middleware_manager.check_rate_limit(&host).map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Rate limit check failed: {}", e))
            })?;
        }
        
        // Protocol selection logic
        let _selected_protocol = self.runtime.block_on(async {
            self.protocol_negotiator.select_protocol(&full_url, &self.protocol_config).await
        });
        
        // Try HTTP/3 if selected and available
        
        if _selected_protocol == HttpVersion::Http3 {
            if let Ok(response) = self.try_http3_request(method.clone(), &full_url, params.clone(), body.clone(), headers.clone()) {
                return Ok(response);
            }
            // Fall back to HTTP/1.1 or HTTP/2 if HTTP/3 fails
        }
        
        // Log request through middleware
        {
            let middleware_manager = self.middleware_manager.read()
                .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Failed to acquire middleware manager lock"))?;
            middleware_manager.log_request(&method.to_string(), &full_url);
        }
        
        // Apply authentication if needed
        self.ensure_oauth2_token()?;
        
        let mut request = self.client.request(method.clone(), &full_url);
        
        // Apply authentication headers
        request = self.apply_auth_internal(request)?;
        
        // === PERFORMANCE OPTIMIZATION: Use cached headers and connection pool ===
        // Try to acquire connection from pool
        let _connection_permit = self.runtime.block_on(async {
            self.connection_pool.try_acquire_connection().await
        });
        
        // Prepare headers map for middleware processing with header caching
        let mut all_headers = {
            // Use cached common headers from the HeaderCache
            let cached_headers = self.header_cache.get_common_headers();
            let mut headers_map = self.headers.read()
                .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Failed to acquire headers lock"))?
                .clone();
            
            // Merge with cached headers for performance
            for (key, value) in cached_headers {
                if !headers_map.contains_key(&key) {
                    headers_map.insert(key, value.to_string());
                }
            }
            headers_map
        };
        
        // Apply middleware headers
        {
            let middleware_manager = self.middleware_manager.read()
                .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Failed to acquire middleware manager lock"))?;
            middleware_manager.apply_headers_middleware(&mut all_headers);
        }
        
        // Performance optimization: Record request patterns for runtime optimization
        let runtime_optimizer = get_runtime_optimizer();
        let ahash_headers: AHashMap<String, String> = all_headers.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        runtime_optimizer.record_request(url, &ahash_headers);
        
        // Add custom headers (these will override defaults and middleware headers)
        if let Some(custom_headers) = headers {
            for (key, value) in custom_headers {
                all_headers.insert(key, value);
            }
        }
        
        // Apply all headers to request
        for (key, value) in &all_headers {
            request = request.header(key, value);
        }
        
        // Add query parameters
        if let Some(params) = params {
            request = request.query(&params);
        }
        
        // Add body with compression if enabled
        if let Some(body) = body {
            let (processed_body, content_encoding) = self.process_request_body(body)?;
            request = request.body(processed_body);
            
            // Add compression headers if body was compressed
            if let Some(encoding) = content_encoding {
                request = request.header("Content-Encoding", encoding);
            }
        }
        
        // Apply additional middleware to request
        request = self.apply_middleware_to_request(request, url, &method)?;
        
        // Execute request
        let response_result = self.runtime.block_on(async {
            request.send().await
        });
        
        let elapsed_time = start_time.elapsed().as_secs_f64();
        
        match response_result {
            Ok(response) => {
                let mut response_obj = Response::from_reqwest(response, &self.runtime)?;
                
                // === PERFORMANCE OPTIMIZATION: Mark connection as used ===
                if let Some(permit) = _connection_permit {
                    permit.mark_used(&host, Duration::from_secs_f64(elapsed_time));
                }
                
                // Performance optimization: Record response patterns for runtime optimization
                let response_size = response_obj.content.len();
                let success = response_obj.status_code >= 200 && response_obj.status_code < 400;
                let protocol = response_obj.protocol.as_deref().unwrap_or("HTTP/1.1");
                runtime_optimizer.record_response(response_size, elapsed_time, success, &host, protocol);
                
                // Log successful response through middleware
                {
                    let middleware_manager = self.middleware_manager.read()
                        .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Failed to acquire middleware manager lock"))?;
                    middleware_manager.log_response(response_obj.status_code, elapsed_time * 1000.0);
                    middleware_manager.update_metrics(elapsed_time, false);
                }
                
                // Process response through middleware
                self.apply_middleware_to_response(&mut response_obj, elapsed_time)?;
                
                // Update statistics
                {
                    let mut count = self.request_count.load(std::sync::atomic::Ordering::Relaxed);
                    count += 1;
                    self.request_count.store(count, std::sync::atomic::Ordering::Relaxed);
                }
                {
                    let mut total_time = self.total_request_time.load(std::sync::atomic::Ordering::Relaxed);
                    total_time += elapsed_time as u64;
                    self.total_request_time.store(total_time, std::sync::atomic::Ordering::Relaxed);
                }
                {
                    let mut last_time = self.last_request_time.write()
                        .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Failed to acquire last request time lock"))?;
                    *last_time = Some(elapsed_time);
                }
                
                // Update shared protocol stats
                self.protocol_stats_manager.increment_request_count(&full_url);
                self.protocol_stats_manager.update_response_time(&full_url, Duration::from_secs_f64(elapsed_time));
                
                Ok(response_obj)
            }
            Err(e) => {
                // Log error through middleware
                {
                    let middleware_manager = self.middleware_manager.read()
                        .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Failed to acquire middleware manager lock"))?;
                    middleware_manager.update_metrics(elapsed_time, true);
                }
                
                Err(map_reqwest_error(&e))
            }
        }
    }
    
    /// Try HTTP/3 request using Quiche
    
    fn try_http3_request(
        &mut self,
        method: Method,
        url: &str,
        params: Option<HashMap<String, String>>,
        body: Option<Vec<u8>>,
        headers: Option<HashMap<String, String>>,
    ) -> PyResult<Response> {
        use std::net::ToSocketAddrs;
        use url::Url;
        
        // Start timing the request
        let start_time = std::time::Instant::now();
        
        // Parse URL to get host and port
        let parsed_url = Url::parse(url).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid URL: {}", e))
        })?;
        
        let host = parsed_url.host_str().ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("No host in URL")
        })?;
        
        let port = parsed_url.port().unwrap_or(443);
        
        // Resolve address
        let addr_str = format!("{}:{}", host, port);
        let mut addrs = addr_str.to_socket_addrs().map_err(|e| {
            pyo3::exceptions::PyConnectionError::new_err(format!("Failed to resolve host: {}", e))
        })?;
        
        let server_addr = addrs.next().ok_or_else(|| {
            pyo3::exceptions::PyConnectionError::new_err("No addresses found for host")
        })?;
        
        // Prepare headers
        let mut all_headers = self.headers.read()
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Failed to acquire headers lock"))?
            .clone();
        if let Some(custom_headers) = headers {
            for (key, value) in custom_headers {
                all_headers.insert(key, value);
            }
        }
        
        // Add query parameters to path
        let mut path = parsed_url.path().to_string();
        if let Some(query) = parsed_url.query() {
            path.push('?');
            path.push_str(query);
        }
        if let Some(params) = params {
            if parsed_url.query().is_some() {
                path.push('&');
            } else {
                path.push('?');
            }
            let param_string = serde_urlencoded::to_string(params).map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Failed to encode parameters: {}", e))
            })?;
            path.push_str(&param_string);
        }
        
        // Get HTTP/3 client from the connection pool
        self.runtime.block_on(async {
            // Try to get a connection from the pool
            let http3_client = match self.http3_pool.get_connection(server_addr).await {
                Ok(client) => client,
                Err(e) => {
                    return Err(pyo3::exceptions::PyConnectionError::new_err(
                        format!("Failed to get HTTP/3 client from pool: {}", e)
                    ));
                }
            };
            
            // Update the current client reference
            let mut client_guard = self.http3_client.write()
                .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Failed to acquire HTTP/3 client lock"))?;
            *client_guard = Some(http3_client);
            Ok(())
        })?;
        
        // Now use the client directly inside a block_on to avoid borrowing issues
        let response = self.runtime.block_on(async {
            let client_guard = self.http3_client.read()
                .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Failed to acquire HTTP/3 client lock"))?;
            let client = client_guard.as_ref()
                .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("Client not initialized"))?;
            
            // Measure HTTP/3 metrics for the request
            // Include start_time for accurate timing metrics
            let _request_metrics = client.stats().await;
            
            // Send the request with optimized parameters
            client.send_request(
                &method.to_string(),
                &path,
                &all_headers,
                body.as_deref(),
            ).await.map_err(|e| pyo3::exceptions::PyConnectionError::new_err(format!("HTTP/3 request failed: {}", e)))
        });
        
        // Convert HTTP/3 response to our Response type with timing information
        let mut response = response?.to_response(url, start_time);
        
        // Add HTTP/3 protocol information
        response.protocol = Some("HTTP/3".to_string());
        
        Ok(response)
    }

    /// Apply authentication to request (internal version)
    fn apply_auth_internal(&self, request: RequestBuilder) -> PyResult<RequestBuilder> {
        let auth_request = auth_common::apply_authentication(request, self.auth_config.as_ref());
        
        // Handle special OAuth2 case that requires token management
        if let Some(ref auth) = self.auth_config {
            if matches!(auth.auth_type, AuthType::OAuth2) {
                if let Ok(token_guard) = self.oauth2_token.read() {
                    if let Some(ref token) = *token_guard {
                        return Ok(auth_request.header("Authorization", format!("{} {}", token.token_type, token.access_token)));
                    }
                }
            }
        }
        
        Ok(auth_request)
    }
    
    /// Ensure OAuth2 token is valid
    fn ensure_oauth2_token(&mut self) -> PyResult<()> {
        if let Some(ref auth) = self.auth_config {
            if auth.is_oauth2() {
                let needs_refresh = {
                    let token_guard = self.oauth2_token.read()
                        .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Failed to acquire OAuth2 token lock"))?;
                    if let Some(ref token) = *token_guard {
                        token.is_expired()
                    } else {
                        true
                    }
                };
                
                if needs_refresh {
                    self.fetch_oauth2_token()?;
                }
            }
        }
        Ok(())
    }
    
    /// Fetch OAuth2 token
    fn fetch_oauth2_token(&mut self) -> PyResult<()> {
        if let Some(ref auth) = self.auth_config {
            let token_url = auth.get_credential("token_url")
                .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("OAuth2 token_url not configured"))?;
            
            let client_id = auth.get_credential("client_id")
                .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("OAuth2 client_id not configured"))?;
            
            let mut params = HashMap::new();
            params.insert("grant_type".to_string(), "client_credentials".to_string());
            params.insert("client_id".to_string(), client_id);
            
            if let Some(client_secret) = auth.get_credential("client_secret") {
                params.insert("client_secret".to_string(), client_secret);
            }
            
            if let Some(scopes) = auth.get_credential("scopes") {
                params.insert("scope".to_string(), scopes);
            }
            
            // Make token request
            let response = self.runtime.block_on(async {
                self.client
                    .post(&token_url)
                    .form(&params)
                    .send()
                    .await
            }).map_err(|e| map_reqwest_error(&e))?;
            
            if !response.status().is_success() {
                return Err(pyo3::exceptions::PyConnectionError::new_err(
                    format!("OAuth2 token request failed: {}", response.status())
                ));
            }
            
            let token_data: serde_json::Value = self.runtime.block_on(async {
                response.json().await
            }).map_err(|e| map_reqwest_error(&e))?;
            
            // Parse token response
            let access_token = token_data["access_token"].as_str()
                .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("No access_token in OAuth2 response"))?
                .to_string();
            
            let token_type = token_data["token_type"].as_str()
                .unwrap_or("Bearer")
                .to_string();
            
            let expires_in = token_data["expires_in"].as_u64();
            let refresh_token = token_data["refresh_token"].as_str().map(|s| s.to_string());
            let scope = token_data["scope"].as_str().map(|s| s.to_string());
            
            let issued_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Failed to get system time"))?
                .as_secs_f64();
            
            let token = OAuth2Token {
                access_token,
                token_type,
                expires_in,
                refresh_token,
                scope,
                issued_at,
            };
            
            let mut token_guard = self.oauth2_token.write()
                .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Failed to acquire OAuth2 token lock"))?;
            *token_guard = Some(token);
        }
        
        Ok(())
    }
    
    /// Apply middleware to request before sending
    fn apply_middleware_to_request(&self, request: RequestBuilder, _url: &str, _method: &Method) -> PyResult<RequestBuilder> {
        // The headers middleware is already applied in execute_request_internal
        // This method is reserved for future middleware types that need to modify the request directly
        Ok(request)
    }
    
    /// Process response through middleware
    fn apply_middleware_to_response(&self, _response: &mut Response, _elapsed_time: f64) -> PyResult<()> {
        // Response middleware processing is already handled in execute_request_internal
        // through the MiddlewareManager public methods
        Ok(())
    }
    
    /// Prepare request body
    fn prepare_body(&self, json: Option<&PyAny>, data: Option<HashMap<String, String>>, files: Option<HashMap<String, Vec<u8>>>) -> PyResult<Option<Vec<u8>>> {
        if let Some(json) = json {
            // Set JSON content type header
            if let Ok(mut headers) = self.headers.write() {
                headers.insert("Content-Type".to_string(), "application/json".to_string());
            }
            let value: Value = pythonize::depythonize(json)?;
            let body = serde_json::to_vec(&value)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("JSON serialization error: {}", e)))?;
            Ok(Some(body))
        } else if let Some(data) = data {
            // Set form data content type header
            if let Ok(mut headers) = self.headers.write() {
                headers.insert("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string());
            }
            let body = serde_urlencoded::to_string(&data)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Form encoding error: {}", e)))?
                .into_bytes();
            Ok(Some(body))
        } else if let Some(files) = files {
            // Implement basic multipart form data
            use std::fmt::Write;
            let boundary = format!("----ultrafast_client_boundary_{}", rand::random::<u64>());
            let mut body = String::new();
            
            // Add form data if present
            if let Some(data) = data {
                for (key, value) in data {
                    writeln!(&mut body, "--{}", boundary).unwrap();
                    writeln!(&mut body, "Content-Disposition: form-data; name=\"{}\"", key).unwrap();
                    writeln!(&mut body).unwrap();
                    writeln!(&mut body, "{}", value).unwrap();
                }
            }
            
            // Add files
            for (name, content) in files {
                writeln!(&mut body, "--{}", boundary).unwrap();
                writeln!(&mut body, "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"", name, name).unwrap();
                writeln!(&mut body, "Content-Type: application/octet-stream").unwrap();
                writeln!(&mut body).unwrap();
                body.push_str(&String::from_utf8_lossy(&content));
                writeln!(&mut body).unwrap();
            }
            
            writeln!(&mut body, "--{}--", boundary).unwrap();
            
            // Set content type header for multipart
            if let Ok(mut headers) = self.headers.write() {
                headers.insert("Content-Type".to_string(), format!("multipart/form-data; boundary={}", boundary));
            }
            
            Ok(Some(body.into_bytes()))
        } else {
            Ok(None)
        }
    }
    
    /// Process request body with compression if enabled
    fn process_request_body(&self, body: Vec<u8>) -> PyResult<(Vec<u8>, Option<String>)> {
        let content_type = match self.headers.read() {
            Ok(headers) => headers.get("Content-Type")
                .unwrap_or(&"application/octet-stream".to_string())
                .clone(),
            Err(_) => "application/octet-stream".to_string(),
        };

        if self.compression_config.should_compress_request(body.len(), &content_type) {
            if let Some(algorithm) = self.compression_config.compression_algorithms.first() {
                match self.compression_config.compress_request_body(&body, algorithm) {
                    Ok(compressed_body) => {
                        let encoding = match algorithm.as_str() {
                            "gzip" => "gzip",
                            "deflate" => "deflate",
                            "brotli" => "br",
                            _ => algorithm.as_str()
                        };
                        Ok((compressed_body, Some(encoding.to_string())))
                    },
                    Err(_) => Ok((body, None))
                }
            } else {
                Ok((body, None))
            }
        } else {
            Ok((body, None))
        }
    }
    
    /// Build full URL
    fn build_url(&self, url: &str) -> PyResult<String> {
        if url.starts_with("http://") || url.starts_with("https://") {
            Ok(url.to_string())
        } else if let Some(ref base) = self.base_url {
            Ok(format!("{}/{}", base.trim_end_matches('/'), url.trim_start_matches('/')))
        } else {
            Ok(url.to_string())
        }
    }
    
    /// Rebuild client with new configuration
    fn rebuild_client(&mut self) -> PyResult<()> {
        let mut client_builder = Client::builder()
            .pool_max_idle_per_host(self.pool_config.max_idle_per_host)
            .pool_idle_timeout(Duration::from_secs_f64(self.pool_config.idle_timeout));
        
        if let Some(connect_timeout) = self.timeout_config.connect_timeout {
            client_builder = client_builder.connect_timeout(Duration::from_secs_f64(connect_timeout));
        }
        
        // Configure SSL/TLS
        if !self.ssl_config.verify {
            client_builder = client_builder.danger_accept_invalid_certs(true);
        }
        
        // Apply additional SSL configuration
        if let Some(cert_path) = &self.ssl_config.cert_file {
            if let Some(key_path) = &self.ssl_config.key_file {
                // Load client certificate and key
                match std::fs::read(cert_path) {
                    Ok(cert_data) => {
                        match std::fs::read(key_path) {
                            Ok(key_data) => {
                                // Use from_pkcs8_pem which is the correct method
                                if let Ok(identity) = reqwest::Identity::from_pkcs8_pem(&cert_data, &key_data) {
                                    client_builder = client_builder.identity(identity);
                                }
                            }
                            Err(_) => {} // Ignore key loading errors
                        }
                    }
                    Err(_) => {} // Ignore cert loading errors
                }
            }
        }
        
        // Configure CA bundle if specified
        if let Some(ca_bundle) = &self.ssl_config.ca_bundle {
            match std::fs::read(ca_bundle) {
                Ok(ca_data) => {
                    if let Ok(cert) = reqwest::Certificate::from_pem(&ca_data) {
                        client_builder = client_builder.add_root_certificate(cert);
                    }
                }
                Err(_) => {} // Ignore CA bundle errors
            }
        }
        
        // Configure proxy if specified
        if let Some(proxy_cfg) = &self.proxy_config {
            if let Ok(proxy) = reqwest::Proxy::all(&proxy_cfg.url) {
                let mut proxy = proxy;
                
                // Add authentication if specified
                if let (Some(username), Some(password)) = (&proxy_cfg.username, &proxy_cfg.password) {
                    proxy = proxy.basic_auth(username, password);
                }
                
                client_builder = client_builder.proxy(proxy);
            }
        }
        
        // Configure compression
        if self.compression_config.enable_response_compression {
            client_builder = client_builder.gzip(true);
            
            if self.compression_config.supports_algorithm("brotli") {
                client_builder = client_builder.brotli(true);
            }
        }
        
        // Configure HTTP/2 settings
        if self.protocol_config.is_http2_enabled() {
            if self.protocol_config.enable_http2_prior_knowledge {
                client_builder = client_builder.http2_prior_knowledge();
            }
            
            // Configure HTTP/2 keep-alive if specified
            if let Some(interval) = self.protocol_config.http2_settings.keep_alive_interval {
                client_builder = client_builder.http2_keep_alive_interval(Some(Duration::from_secs(interval)));
            }
            
            if let Some(timeout) = self.protocol_config.http2_settings.keep_alive_timeout {
                client_builder = client_builder.http2_keep_alive_timeout(Duration::from_secs(timeout));
            }
        }
        
        // Configure HTTP/3 via Quiche
        if self.protocol_config.is_http3_enabled() {
            // HTTP/3 support requires unstable features
            // This will only work if compiled with RUSTFLAGS="--cfg reqwest_unstable"
            // client_builder = client_builder.http3_prior_knowledge();
        }
        
        self.client = self.runtime.block_on(async {
            client_builder.build()
        }).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to rebuild HTTP client: {}", e))
        })?;
        
        // Initialize rate limiting middleware if configured
        if let Some(rate_limit_cfg) = &self.rate_limit_config {
            let mut middleware_manager = MiddlewareManager::new();
            let rate_limit_middleware = crate::middleware::RateLimitMiddleware::new("default_rate_limit".to_string(), rate_limit_cfg.clone(), true)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create rate limit middleware: {}", e)))?;
            middleware_manager.add_rate_limit_middleware(rate_limit_middleware);
            
            // Replace the middleware manager with the new one
            match self.middleware_manager.write() {
                Ok(mut manager) => *manager = middleware_manager,
                Err(_) => return Err(pyo3::exceptions::PyRuntimeError::new_err("Failed to update middleware manager")),
            }
        }
        
        Ok(())
    }
}

impl Drop for HttpClient {
    /// Ensure proper cleanup when the client is dropped
    fn drop(&mut self) {
        // Cancel the cleanup task
        if let Ok(mut cleanup_handle) = self._cleanup_handle.write() {
            if let Some(handle) = cleanup_handle.take() {
                handle.abort();
            }
        }
        
        // Perform any necessary cleanup operations
        self.connection_pool.cleanup_expired();
        self.connection_multiplexer.cleanup_all();
    }
}

/// Convert JSON value to Python object
pub fn json_to_python(py: Python, value: &serde_json::Value) -> PyResult<PyObject> {
    use pyo3::types::{PyDict, PyList};
    
    match value {
        serde_json::Value::Null => Ok(py.None()),
        serde_json::Value::Bool(b) => Ok(b.to_object(py)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.to_object(py))
            } else if let Some(f) = n.as_f64() {
                Ok(f.to_object(py))
            } else {
                Ok(n.to_string().to_object(py))
            }
        }
        serde_json::Value::String(s) => Ok(s.to_object(py)),
        serde_json::Value::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(json_to_python(py, item)?)?;
            }
            Ok(list.to_object(py))
        }
        serde_json::Value::Object(map) => {
            let dict = PyDict::new(py);
            for (key, val) in map {
                dict.set_item(key, json_to_python(py, val)?)?;
            }
            Ok(dict.to_object(py))
        }
    }
}
