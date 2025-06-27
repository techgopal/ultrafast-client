use pyo3::prelude::*;
use pyo3::types::PyAny;
use pyo3::{PyRef, PyRefMut};
use reqwest::{Client, Method};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex as TokioMutex;
use crate::config::{
    AuthConfig, RetryConfig, TimeoutConfig, PoolConfig, SSLConfig, 
    ProxyConfig, CompressionConfig, ProtocolConfig, RateLimitConfig
};
use crate::response::Response;
use crate::middleware::{MiddlewareManager, LoggingMiddleware, HeadersMiddleware, RateLimitMiddleware};
use crate::rate_limit_common::{AsyncRateLimitManager, RateLimitManager};
use crate::protocol_stats_common::{AsyncProtocolStatsManager, ProtocolStatsManager};
use crate::http3::{AsyncHttp3Client, AsyncHttp3ConnectionPool};
use crate::connection_pool::{FastConnectionPool, ConnectionMultiplexer};
use crate::performance_common::{HeaderCache};
use crate::protocol_enhanced::EnhancedProtocolNegotiator;
use pyo3_asyncio::tokio::future_into_py;
use crate::error::map_reqwest_error;
use base64::engine::general_purpose;
use base64::Engine;
use crate::auth_common;

/// Async HTTP Client for Python asyncio integration
#[pyclass]
#[derive(Clone)]
pub struct AsyncHttpClient {
    client: Client,
    base_url: Option<String>,
    headers: HashMap<String, String>,
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
    middleware_manager: Arc<tokio::sync::Mutex<MiddlewareManager>>,
    // OAuth2 token storage
    oauth2_token: Arc<TokioMutex<Option<crate::config::OAuth2Token>>>,
    // Shared rate limiting manager (async-compatible)
    rate_limit_manager: Arc<AsyncRateLimitManager>,
    // Shared protocol statistics manager (async-compatible)
    protocol_stats_manager: Arc<AsyncProtocolStatsManager>,
    // HTTP/3 client for QUIC connections
    http3_client: Arc<tokio::sync::Mutex<Option<AsyncHttp3Client>>>,
    // HTTP/3 connection pool
    http3_pool: Arc<AsyncHttp3ConnectionPool>,
    // Performance optimizations
    header_cache: Arc<HeaderCache>,
    connection_pool: Arc<FastConnectionPool>,
    connection_multiplexer: Arc<ConnectionMultiplexer>,
    // Request statistics
    request_count: Arc<TokioMutex<u64>>,
    total_request_time: Arc<TokioMutex<f64>>,
    last_request_time: Arc<TokioMutex<Option<std::time::Instant>>>,
}

#[pymethods]
impl AsyncHttpClient {
    /// Create a new AsyncHttpClient
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
        let timeout_cfg = timeout_config.unwrap_or_else(|| TimeoutConfig::new(
            Some(10.0), Some(timeout), Some(timeout), Some(30.0)
        ));
        
        let pool_cfg = pool_config.unwrap_or_else(|| PoolConfig::new(100, 10, 90.0, 30.0));
        let ssl_cfg = ssl_config.unwrap_or_else(|| SSLConfig::new(true, None, None, None, None));
        let compression_cfg = compression_config.unwrap_or_else(|| CompressionConfig::new(false, true, None, None, 1024));
        let protocol_cfg = protocol_config.clone().unwrap_or_else(|| ProtocolConfig::default());
        // Note: HTTP/3 pool settings would be used for HTTP/3 client initialization if implemented
        let http3_pool_settings = protocol_cfg.http3_settings.clone();
        
        // --- User-Agent fix: set at client builder level if present in headers ---
        let mut client_builder = Client::builder()
            .timeout(Duration::from_secs_f64(timeout))
            .pool_max_idle_per_host(pool_cfg.max_idle_per_host)
            .pool_idle_timeout(Duration::from_secs_f64(pool_cfg.idle_timeout))
            .user_agent(""); // Suppress reqwest default User-Agent
        
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
        
        // Set User-Agent at client level if present in headers
        // let mut headers = headers.unwrap_or_default();
        // if let Some(user_agent) = headers.remove("User-Agent").or_else(|| headers.remove("user-agent")) {
        //     client_builder = client_builder.user_agent(user_agent);
        // }
        let mut headers = headers.unwrap_or_default();
        
        let client = client_builder.build().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create async HTTP client: {}", e))
        })?;

        // Create middleware manager and add rate limiting if configured
        let mut middleware_manager = MiddlewareManager::new();
        if let Some(rate_limit_cfg) = &rate_limit_config {
            let rate_limit_middleware = crate::middleware::RateLimitMiddleware::new("default_rate_limit".to_string(), rate_limit_cfg.clone(), true)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create rate limit middleware: {}", e)))?;
            middleware_manager.add_rate_limit_middleware(rate_limit_middleware);
        }
        
        // Initialize shared managers
        let rate_limit_manager = Arc::new(AsyncRateLimitManager::new(rate_limit_config.clone()));
        let protocol_stats_manager = Arc::new(AsyncProtocolStatsManager::new());
        
        Ok(AsyncHttpClient {
            client,
            base_url,
            headers,
            auth_config,
            retry_config,
            timeout_config: timeout_cfg,
            pool_config: pool_cfg.clone(),
            ssl_config: ssl_cfg,
            proxy_config,
            compression_config: compression_cfg,
            protocol_config: protocol_cfg.clone(),
            rate_limit_config,
            protocol_negotiator: Arc::new(EnhancedProtocolNegotiator::new(protocol_cfg.fallback_strategy.clone())),
            middleware_manager: Arc::new(tokio::sync::Mutex::new(middleware_manager)),
            oauth2_token: Arc::new(TokioMutex::new(None)),
            rate_limit_manager,
            protocol_stats_manager,
            http3_client: Arc::new(tokio::sync::Mutex::new(None)),
            http3_pool: Arc::new(AsyncHttp3ConnectionPool::new(
                http3_pool_settings.connection_pool_size.unwrap_or(10), 
                http3_pool_settings.pool_timeout_seconds.unwrap_or(300)
            )),
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
            request_count: Arc::new(TokioMutex::new(0)),
            total_request_time: Arc::new(TokioMutex::new(0.0)),
            last_request_time: Arc::new(TokioMutex::new(None)),
        })
    }


    
    /// Async GET request with enhanced retry logic
    pub fn get<'py>(slf: Py<Self>, py: Python<'py>, url: &str, params: Option<HashMap<String, String>>, headers: Option<HashMap<String, String>>) -> PyResult<&'py PyAny> {
        let this = slf.borrow(py).clone();
        let url = url.to_string();
        let params = params.clone();
        let headers = headers.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            this.execute_request_with_retry(Method::GET, &url, params, None, headers).await
        })
    }

    /// Async POST request with enhanced retry logic
    #[pyo3(signature = (url, json = None, data = None, files = None, headers = None))]
    pub fn post<'py>(slf: Py<Self>, py: Python<'py>, url: &str, json: Option<&PyAny>, data: Option<HashMap<String, String>>, files: Option<HashMap<String, Vec<u8>>>, headers: Option<HashMap<String, String>>) -> PyResult<&'py PyAny> {
        let this = slf.borrow(py).clone();
        let url = url.to_string();
        let data = data.clone();
        let files = files.clone();
        let mut headers = headers.clone().unwrap_or_default();
        
        // Convert JSON to bytes directly here in the synchronous context
        let json_body = if let Some(json_obj) = json {
            // Set Content-Type header for JSON
            headers.insert("Content-Type".to_string(), "application/json".to_string());
            let json_value: serde_json::Value = pythonize::depythonize(json_obj)?;
            Some(serde_json::to_vec(&json_value)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("JSON serialization error: {}", e)))?)
        } else {
            None
        };
        
        pyo3_asyncio::tokio::future_into_py(py, async move {
            let body = if let Some(json_bytes) = json_body {
                Some(json_bytes)
            } else {
                let has_files = files.is_some();
                let has_data = data.is_some();
                let prepared_body = this.prepare_body(None, data, files)?;
                // Set Content-Type header for form data if body was prepared
                if prepared_body.is_some() {
                    if has_files {
                        // Multipart form data - Content-Type will be set in execute_single_request
                    } else if has_data {
                        // Regular form data
                        headers.insert("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string());
                    }
                }
                prepared_body
            };
            this.execute_request_with_retry(Method::POST, &url, None, body, Some(headers)).await
        })
    }

    /// Async PUT request with enhanced retry logic
    #[pyo3(signature = (url, json = None, data = None, files = None, headers = None))]
    pub fn put<'py>(slf: Py<Self>, py: Python<'py>, url: &str, json: Option<&PyAny>, data: Option<HashMap<String, String>>, files: Option<HashMap<String, Vec<u8>>>, headers: Option<HashMap<String, String>>) -> PyResult<&'py PyAny> {
        let this = slf.borrow(py).clone();
        let url = url.to_string();
        let data = data.clone();
        let files = files.clone();
        let mut headers = headers.clone().unwrap_or_default();
        
        // Convert JSON to bytes directly here in the synchronous context
        let json_body = if let Some(json_obj) = json {
            // Set Content-Type header for JSON
            headers.insert("Content-Type".to_string(), "application/json".to_string());
            let json_value: serde_json::Value = pythonize::depythonize(json_obj)?;
            Some(serde_json::to_vec(&json_value)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("JSON serialization error: {}", e)))?)
        } else {
            None
        };
        
        pyo3_asyncio::tokio::future_into_py(py, async move {
            let body = if let Some(json_bytes) = json_body {
                Some(json_bytes)
            } else {
                let has_files = files.is_some();
                let has_data = data.is_some();
                let prepared_body = this.prepare_body(None, data, files)?;
                // Set Content-Type header for form data if body was prepared
                if prepared_body.is_some() {
                    if has_files {
                        // Multipart form data - Content-Type will be set in execute_single_request
                    } else if has_data {
                        // Regular form data
                        headers.insert("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string());
                    }
                }
                prepared_body
            };
            this.execute_request_with_retry(Method::PUT, &url, None, body, Some(headers)).await
        })
    }

    /// Async DELETE request with enhanced retry logic
    pub fn delete<'py>(slf: Py<Self>, py: Python<'py>, url: &str, headers: Option<HashMap<String, String>>) -> PyResult<&'py PyAny> {
        let this = slf.borrow(py).clone();
        let url = url.to_string();
        let headers = headers.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            this.execute_request_with_retry(Method::DELETE, &url, None, None, headers).await
        })
    }

    /// Async PATCH request with enhanced retry logic
    #[pyo3(signature = (url, json = None, data = None, files = None, headers = None))]
    pub fn patch<'py>(slf: Py<Self>, py: Python<'py>, url: &str, json: Option<&PyAny>, data: Option<HashMap<String, String>>, files: Option<HashMap<String, Vec<u8>>>, headers: Option<HashMap<String, String>>) -> PyResult<&'py PyAny> {
        let this = slf.borrow(py).clone();
        let url = url.to_string();
        let data = data.clone();
        let files = files.clone();
        let mut headers = headers.clone().unwrap_or_default();
        
        // Convert JSON to bytes directly here in the synchronous context
        let json_body = if let Some(json_obj) = json {
            // Set Content-Type header for JSON
            headers.insert("Content-Type".to_string(), "application/json".to_string());
            let json_value: serde_json::Value = pythonize::depythonize(json_obj)?;
            Some(serde_json::to_vec(&json_value)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("JSON serialization error: {}", e)))?)
        } else {
            None
        };
        
        pyo3_asyncio::tokio::future_into_py(py, async move {
            let body = if let Some(json_bytes) = json_body {
                Some(json_bytes)
            } else {
                let has_files = files.is_some();
                let has_data = data.is_some();
                let prepared_body = this.prepare_body(None, data, files)?;
                // Set Content-Type header for form data if body was prepared
                if prepared_body.is_some() {
                    if has_files {
                        // Multipart form data - Content-Type will be set in execute_single_request
                    } else if has_data {
                        // Regular form data
                        headers.insert("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string());
                    }
                }
                prepared_body
            };
            this.execute_request_with_retry(Method::PATCH, &url, None, body, Some(headers)).await
        })
    }

    /// Async HEAD request with enhanced retry logic
    pub fn head<'py>(slf: Py<Self>, py: Python<'py>, url: &str, headers: Option<HashMap<String, String>>) -> PyResult<&'py PyAny> {
        let this = slf.borrow(py).clone();
        let url = url.to_string();
        let headers = headers.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            this.execute_request_with_retry(Method::HEAD, &url, None, None, headers).await
        })
    }

    /// Async OPTIONS request with enhanced retry logic
    pub fn options<'py>(slf: Py<Self>, py: Python<'py>, url: &str, headers: Option<HashMap<String, String>>) -> PyResult<&'py PyAny> {
        let this = slf.borrow(py).clone();
        let url = url.to_string();
        let headers = headers.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            this.execute_request_with_retry(Method::OPTIONS, &url, None, None, headers).await
        })
    }
    
    /// Set a header for the client
    pub fn set_header(&mut self, key: String, value: String) {
        self.headers.insert(key, value);
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

    /// Set authentication configuration
    pub fn set_auth(&mut self, auth_config: AuthConfig) -> PyResult<()> {
        // If OAuth2, we might need to fetch initial token
        if auth_config.is_oauth2() {
            // Token will be fetched on first request
            self.oauth2_token = Arc::new(TokioMutex::new(None));
        }
        self.auth_config = Some(auth_config);
        Ok(())
    }
    
    /// Clear authentication
    pub fn clear_auth(&mut self) {
        self.auth_config = None;
        // Clear OAuth2 token synchronously by creating a new empty Arc
        self.oauth2_token = Arc::new(TokioMutex::new(None));
    }
    
    /// Get current authentication configuration
    pub fn get_auth(&self) -> Option<AuthConfig> {
        self.auth_config.clone()
    }
    
    /// Check if authentication is configured
    pub fn has_auth(&self) -> bool {
        self.auth_config.is_some()
    }
    
    /// Set retry configuration
    pub fn set_retry_config(&mut self, retry_config: RetryConfig) {
        self.retry_config = Some(retry_config);
    }
    
    /// Set timeout configuration
    pub fn set_timeout_config(&mut self, timeout_config: TimeoutConfig) -> PyResult<()> {
        self.timeout_config = timeout_config;
        self.rebuild_client()?;
        Ok(())
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
    
    /// Get protocol configuration
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
    
    /// Remove a header from the client
    pub fn remove_header(&mut self, key: &str) -> Option<String> {
        self.headers.remove(key)
    }
    
    #[pyo3(name = "add_middleware")]
    pub fn add_middleware<'p>(mut slf: PyRefMut<'p, Self>, py: Python<'p>, middleware: &'p PyAny) -> PyResult<&'p PyAny> {
        let manager = slf.middleware_manager.clone();
        if let Ok(logging) = middleware.extract::<PyRef<LoggingMiddleware>>() {
            let logging_val = (*logging).clone();
            pyo3_asyncio::tokio::future_into_py(py, async move {
                let mut mgr = manager.lock().await;
                mgr.add_logging_middleware(logging_val)
                    .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to add logging middleware: {}", e)))?;
                Ok(Python::with_gil(|py| py.None()))
            })
        } else if let Ok(headers) = middleware.extract::<PyRef<HeadersMiddleware>>() {
            let headers_val = (*headers).clone();
            pyo3_asyncio::tokio::future_into_py(py, async move {
                let mut mgr = manager.lock().await;
                mgr.add_headers_middleware(headers_val)
                    .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to add headers middleware: {}", e)))?;
                Ok(Python::with_gil(|py| py.None()))
            })
        } else if let Ok(rate_limit) = middleware.extract::<PyRef<RateLimitMiddleware>>() {
            let rate_limit_val = (*rate_limit).clone();
            pyo3_asyncio::tokio::future_into_py(py, async move {
                let mut mgr = manager.lock().await;
                mgr.add_rate_limit_middleware(rate_limit_val);
                Ok(Python::with_gil(|py| py.None()))
            })
        } else {
            Err(pyo3::exceptions::PyValueError::new_err("Unsupported middleware type"))
        }
    }

    #[pyo3(name = "remove_middleware")]
    pub fn remove_middleware<'p>(mut slf: PyRefMut<'p, Self>, py: Python<'p>, name: String) -> PyResult<&'p PyAny> {
        let manager = slf.middleware_manager.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            let mut mgr = manager.lock().await;
            let mut stack = mgr.middleware_stack.write().map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire middleware stack lock: {}", e)))?;
            let mut removed = false;
            // Logging
            let before = stack.logging_middleware.len();
            stack.logging_middleware.retain(|m| m.name != name);
            if stack.logging_middleware.len() < before { removed = true; }
            // Headers
            let before = stack.headers_middleware.len();
            stack.headers_middleware.retain(|m| m.name != name);
            if stack.headers_middleware.len() < before { removed = true; }
            // RateLimit
            let before = stack.rate_limit_middleware.len();
            stack.rate_limit_middleware.retain(|m| m.name != name);
            if stack.rate_limit_middleware.len() < before { removed = true; }
            Ok(Python::with_gil(|py| removed.into_py(py)))
        })
    }
    
    /// Prepare request body (similar to HttpClient) - synchronous version
    pub fn prepare_body(&self, json: Option<&PyAny>, data: Option<HashMap<String, String>>, files: Option<HashMap<String, Vec<u8>>>) -> PyResult<Option<Vec<u8>>> {
        if let Some(json_data) = json {
            // Serialize JSON
            let json_value: serde_json::Value = pythonize::depythonize(json_data)?;
            let json_string = serde_json::to_string(&json_value)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("JSON serialization error: {}", e)))?;
            return Ok(Some(json_string.into_bytes()));
        }
        
        if let Some(files) = &files {
            // Always use multipart if files are present
            let boundary = format!("----UltraFastBoundary{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to get system time: {}", e)))?
                .as_millis());
            let mut body = String::new();
            let mut body_bytes = Vec::new();
            // Helper function to get MIME type from filename
            fn get_mime_type(filename: &str) -> &'static str {
                let ext = filename.split('.').last().unwrap_or("").to_lowercase();
                match ext.as_str() {
                    "pdf" => "application/pdf",
                    "jpg" | "jpeg" => "image/jpeg",
                    "png" => "image/png",
                    "gif" => "image/gif",
                    "webp" => "image/webp",
                    "svg" => "image/svg+xml",
                    "json" => "application/json",
                    "xml" => "application/xml",
                    "txt" => "text/plain",
                    "html" | "htm" => "text/html",
                    "css" => "text/css",
                    "js" => "application/javascript",
                    "zip" => "application/zip",
                    "tar" => "application/x-tar",
                    "gz" => "application/gzip",
                    _ => "application/octet-stream"
                }
            }
            // Add form fields
            if let Some(data) = &data {
                for (key, value) in data.iter() {
                    let content_length = value.len();
                    let part = format!(
                        "--{}\r\nContent-Disposition: form-data; name=\"{}\"\r\nContent-Length: {}\r\n\r\n{}\r\n",
                        boundary, key, content_length, value
                    );
                    body_bytes.extend_from_slice(part.as_bytes());
                }
            }
            // Add files
            for (filename, file_bytes) in files.iter() {
                let mime_type = get_mime_type(filename);
                let content_length = file_bytes.len();
                let part_header = format!(
                    "--{}\r\nContent-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n",
                    boundary, filename, filename, mime_type, content_length
                );
                body_bytes.extend_from_slice(part_header.as_bytes());
                body_bytes.extend_from_slice(file_bytes);
                body_bytes.extend_from_slice(b"\r\n");
            }
            body_bytes.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());
            return Ok(Some(body_bytes));
        }
        
        if let Some(form_data) = data {
            // Handle form data
            let form_string = serde_urlencoded::to_string(form_data)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Form encoding error: {}", e)))?;
            return Ok(Some(form_string.into_bytes()));
        }
        Ok(None)
    }
    
    /// Process request body with compression if enabled
    #[pyo3(signature = (body, content_type))]
    pub fn process_request_body(&self, body: Option<Vec<u8>>, content_type: &str) -> PyResult<Option<Vec<u8>>> {
        // Check if compression is enabled and should be applied
        if let Some(body_data) = body {
            if self.compression_config.enable_request_compression && 
               body_data.len() >= self.compression_config.min_compression_size {
                
                // Check if content type is compressible
                let compressible_types = [
                    "text/", "application/json", "application/xml", 
                    "application/javascript", "application/x-www-form-urlencoded"
                ];
                
                let should_compress = compressible_types.iter().any(|&t| content_type.starts_with(t));
                
                if should_compress {
                    // Use flate2 for gzip compression
                    use flate2::write::GzEncoder;
                    use flate2::Compression;
                    use std::io::Write;
                    
                    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                    if encoder.write_all(&body_data).is_ok() {
                        if let Ok(compressed_data) = encoder.finish() {
                            return Ok(Some(compressed_data));
                        }
                    }
                }
                
                return Ok(Some(body_data));
            }
            return Ok(Some(body_data));
        }
        Ok(body)
    }
    
    /// Set rate limiting configuration (async)
    pub fn set_rate_limit_config<'py>(&mut self, py: Python<'py>, rate_limit_config: Option<RateLimitConfig>) -> PyResult<&'py PyAny> {
        // Validate config if provided
        if let Some(ref config) = rate_limit_config {
            config.validate()?;
        }
        
        self.rate_limit_config = rate_limit_config.clone();
        
        let rate_limit_manager = self.rate_limit_manager.clone();
        let middleware_manager = self.middleware_manager.clone();
        
        pyo3_asyncio::tokio::future_into_py(py, async move {
            // Update the async rate limit manager
            rate_limit_manager.update_config(rate_limit_config.clone()).await
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e))?;
            
            // Also update middleware manager for backward compatibility
            if let Ok(mut middleware_manager_guard) = middleware_manager.try_lock() {
                if let Some(config) = rate_limit_config {
                    let rate_limit_middleware = crate::middleware::RateLimitMiddleware::new("config_rate_limit".to_string(), config, true)
                        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create rate limit middleware: {}", e)))?;
                    middleware_manager_guard.add_rate_limit_middleware(rate_limit_middleware);
                } else {
                    // Remove rate limiting by creating a disabled middleware
                    let disabled_config = RateLimitConfig::disabled();
                    let rate_limit_middleware = crate::middleware::RateLimitMiddleware::new("disabled_rate_limit".to_string(), disabled_config, false)
                        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create rate limit middleware: {}", e)))?;
                    middleware_manager_guard.add_rate_limit_middleware(rate_limit_middleware);
                }
            } else {
                return Err(pyo3::exceptions::PyRuntimeError::new_err("Failed to acquire middleware manager lock"));
            }
            
            Ok(())
        })
    }
    
    /// Get current rate limiting configuration (async)
    pub fn get_rate_limit_config<'py>(&self, py: Python<'py>) -> PyResult<&'py PyAny> {
        let rate_limit_manager = self.rate_limit_manager.clone();
        
        pyo3_asyncio::tokio::future_into_py(py, async move {
            Ok(rate_limit_manager.get_config().await)
        })
    }
    
    /// Check if rate limiting is enabled (async)
    pub fn is_rate_limiting_enabled<'py>(&self, py: Python<'py>) -> PyResult<&'py PyAny> {
        let rate_limit_manager = self.rate_limit_manager.clone();
        
        pyo3_asyncio::tokio::future_into_py(py, async move {
            Ok(rate_limit_manager.is_enabled().await)
        })
    }
    
    /// Get rate limiting status for a specific host (async)
    pub fn get_rate_limit_status<'py>(&self, py: Python<'py>, host: &str) -> PyResult<&'py PyAny> {
        let rate_limit_manager = self.rate_limit_manager.clone();
        let host = host.to_string();
        
        pyo3_asyncio::tokio::future_into_py(py, async move {
            Ok(rate_limit_manager.get_status(&host).await)
        })
    }
    
    /// Reset rate limiting for all hosts (async)
    pub fn reset_rate_limits<'py>(&self, py: Python<'py>) -> PyResult<&'py PyAny> {
        let rate_limit_manager = self.rate_limit_manager.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            rate_limit_manager.reset().await;
            Ok(())
        })
    }
    
    /// Get performance statistics (async)
    pub fn get_stats<'py>(&self, py: Python<'py>) -> PyResult<&'py PyAny> {
        let protocol_stats_manager = self.protocol_stats_manager.clone();
        let rate_limit_manager = self.rate_limit_manager.clone();
        
        pyo3_asyncio::tokio::future_into_py(py, async move {
            // Get all protocol stats 
            let all_stats = protocol_stats_manager.get_all_stats().await;
            
            // Calculate summary statistics
            let mut summary = HashMap::new();
            let mut total_requests = 0u64;
            let mut total_errors = 0u64;
            let mut total_bytes_sent = 0u64;
            let mut total_bytes_received = 0u64;
            let mut total_response_time = 0.0;
            let mut response_count = 0;
            
            for stats in all_stats.values() {
                total_requests += stats.request_count;
                total_errors += stats.error_count;
                total_bytes_sent += stats.total_bytes_sent;
                total_bytes_received += stats.total_bytes_received;
                
                if let Some(avg_time) = stats.average_response_time {
                    total_response_time += avg_time.as_secs_f64();
                    response_count += 1;
                }
            }
            
            // Calculate averages
            let error_rate = if total_requests > 0 {
                (total_errors as f64 / total_requests as f64) * 100.0
            } else {
                0.0
            };
            
            let average_response_time = if response_count > 0 {
                total_response_time / response_count as f64
            } else {
                0.0
            };
            
            // Build summary
            summary.insert("request_count".to_string(), total_requests as f64);
            summary.insert("error_count".to_string(), total_errors as f64);
            summary.insert("total_bytes_sent".to_string(), total_bytes_sent as f64);
            summary.insert("total_bytes_received".to_string(), total_bytes_received as f64);
            summary.insert("error_rate_percent".to_string(), error_rate);
            summary.insert("average_request_time".to_string(), average_response_time);
            summary.insert("total_request_time".to_string(), total_response_time);
            
            // Add rate limiting stats
            let rate_limit_stats = rate_limit_manager.get_stats().await;
            for (key, value) in rate_limit_stats {
                summary.insert(format!("rate_limit_{}", key), value);
            }
            
            Ok(summary)
        })
    }
    
    /// Reset performance statistics (async)
    pub fn reset_stats<'py>(&self, py: Python<'py>) -> PyResult<&'py PyAny> {
        let protocol_stats_manager = self.protocol_stats_manager.clone();
        
        pyo3_asyncio::tokio::future_into_py(py, async move {
            protocol_stats_manager.clear_all_stats().await;
            Ok(())
        })
    }

    /// Check if HTTP/3 support is available
    pub fn supports_http3(&self) -> PyResult<bool> {
        // Check if HTTP/3 is enabled in protocol configuration
        Ok(self.protocol_config.is_http3_enabled())
    }

    /// Get HTTP/3 protocol statistics for a specific URL
    pub fn get_protocol_stats(&self, url: &str) -> PyResult<HashMap<String, String>> {
        use url::Url;

        // Parse URL to get host and port
        let parsed_url = Url::parse(url).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid URL: {}", e))
        })?;

        let host = parsed_url.host_str().ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("No host in URL")
        })?;

        let port = parsed_url.port_or_known_default().unwrap_or(443);

        // Create connection key
        let connection_key = format!("{}:{}", host, port);

        // For now, return basic placeholder stats since we can't access async stats synchronously
        let mut result = HashMap::new();
        result.insert("connection_key".to_string(), connection_key);
        result.insert("requests_sent".to_string(), "0".to_string());
        result.insert("bytes_sent".to_string(), "0".to_string());
        result.insert("bytes_received".to_string(), "0".to_string());
        result.insert("error_count".to_string(), "0".to_string());
        result.insert("protocol_version".to_string(), "HTTP/3".to_string());
        result.insert("average_response_time_ms".to_string(), "0.0".to_string());
        result.insert("last_used".to_string(), "0".to_string());

        Ok(result)
    }

    /// Set the base URL for the client
    pub fn set_base_url(&mut self, base_url: Option<String>) {
        self.base_url = base_url;
    }

    /// Get all headers (for debugging) - matches HttpClient interface
    pub fn get_headers(&self) -> PyResult<HashMap<String, String>> {
        Ok(self.headers.clone())
    }
    
    /// Get rate limit configuration - direct return (matches HttpClient)
    pub fn get_rate_limit_config_sync(&self) -> Option<RateLimitConfig> {
        self.rate_limit_config.clone()
    }
    
    /// Check if rate limiting is enabled - direct return (matches HttpClient)
    pub fn is_rate_limiting_enabled_sync(&self) -> bool {
        self.rate_limit_config.as_ref().map(|cfg| cfg.enabled).unwrap_or(false)
    }
    
    /// Get rate limit status for host - direct return (matches HttpClient)
    pub fn get_rate_limit_status_sync(&self, _host: &str) -> f64 {
        // This is a simplified version that returns the remaining quota
        // For more detailed status, use the async version
        if let Some(config) = &self.rate_limit_config {
            if config.enabled {
                // Return a placeholder value - in practice this would need async access
                return config.requests_per_second;
            }
        }
        0.0
    }
    
    /// Reset rate limits - direct return (matches HttpClient)
    pub fn reset_rate_limits_sync(&self) -> PyResult<()> {
        // This is a simplified version - for full functionality use the async version
        Ok(())
    }
    
    /// Get performance statistics - direct return (matches HttpClient)
    pub fn get_stats_sync(&self) -> PyResult<HashMap<String, f64>> {
        // This is a simplified version that returns basic stats
        // For detailed stats, use the async version
        let mut stats = HashMap::new();
        stats.insert("request_count".to_string(), 0.0);
        stats.insert("total_request_time".to_string(), 0.0);
        stats.insert("average_request_time".to_string(), 0.0);
        stats.insert("error_count".to_string(), 0.0);
        stats.insert("error_rate_percent".to_string(), 0.0);
        Ok(stats)
    }
    
    /// Reset performance statistics - direct return (matches HttpClient)
    pub fn reset_stats_sync(&mut self) -> PyResult<()> {
        // This is a simplified version - for full functionality use the async version
        Ok(())
    }


}

impl AsyncHttpClient {
    /// Enhanced async retry logic with exponential backoff and circuit breaker
    pub(crate) async fn execute_request_with_retry(
        &self,
        method: Method,
        url: &str,
        params: Option<HashMap<String, String>>,
        body: Option<Vec<u8>>,
        headers: Option<HashMap<String, String>>,
    ) -> PyResult<Response> {
        let start_time = Instant::now();
        let max_retries = self.retry_config.as_ref().map(|c| c.max_retries).unwrap_or(3);
        let mut last_error = None;
        
        for attempt in 0..=max_retries {
            match self.execute_single_request(&method, url, params.as_ref(), body.as_ref(), headers.as_ref()).await {
                Ok(response) => {
                    let elapsed = start_time.elapsed().as_secs_f64();
                    
                    // Update request statistics
                    {
                        let mut request_count = self.request_count.lock().await;
                        *request_count += 1;
                    }
                    {
                        let mut total_time = self.total_request_time.lock().await;
                        *total_time += elapsed;
                    }
                    {
                        let mut last_time = self.last_request_time.lock().await;
                        *last_time = Some(start_time);
                    }
                    
                    return Ok(response);
                }
                Err(e) => {
                    last_error = Some(e);
                    
                    // If this is not the last attempt, wait before retrying
                    if attempt < max_retries {
                        let delay = if let Some(retry_config) = &self.retry_config {
                            // Exponential backoff
                            let base_delay = retry_config.initial_delay;
                            let backoff_factor = retry_config.exponential_base;
                            let max_delay = retry_config.max_delay;
                            
                            let calculated_delay = base_delay * backoff_factor.powi(attempt as i32);
                            calculated_delay.min(max_delay)
                        } else {
                            2.0_f64.powi(attempt as i32).min(10.0) // Default exponential backoff
                        };
                        
                        tokio::time::sleep(Duration::from_secs_f64(delay)).await;
                    }
                }
            }
        }
        
        // All retries failed, return the last error
        Err(last_error.unwrap_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Request failed after all retry attempts")
        }))
    }
    
    /// Execute a single HTTP request
    async fn execute_single_request(
        &self,
        method: &Method,
        url: &str,
        params: Option<&HashMap<String, String>>,
        body: Option<&Vec<u8>>,
        headers: Option<&HashMap<String, String>>,
    ) -> PyResult<Response> {
        let request_start = Instant::now();
        
        // Build the full URL
        let full_url = self.build_url(url)?;
        
        // Create the request builder
        let mut request_builder = self.client.request(method.clone(), &full_url);
        
        // Add query parameters
        if let Some(params) = params {
            for (key, value) in params {
                request_builder = request_builder.query(&[(key, value)]);
            }
        }
        
        // Add default headers
        for (key, value) in &self.headers {
            request_builder = request_builder.header(key, value);
        }
        
        // Add request-specific headers
        if let Some(headers) = headers {
            for (key, value) in headers {
                request_builder = request_builder.header(key, value);
            }
        }
        
        // Add body if present
        if let Some(body_data) = body {
            // Auto-detect Content-Type if not already set
            let mut has_content_type = false;
            if let Some(headers) = headers {
                has_content_type = headers.keys().any(|k| k.to_lowercase() == "content-type");
            }
            if !has_content_type {
                has_content_type = self.headers.keys().any(|k| k.to_lowercase() == "content-type");
            }
            
            if !has_content_type {
                // Try to detect content type from body
                if let Ok(json_test) = std::str::from_utf8(body_data) {
                    if json_test.trim_start().starts_with('{') || json_test.trim_start().starts_with('[') {
                        request_builder = request_builder.header("Content-Type", "application/json");
                    } else {
                        request_builder = request_builder.header("Content-Type", "application/x-www-form-urlencoded");
                    }
                } else {
                    request_builder = request_builder.header("Content-Type", "application/octet-stream");
                }
            }
            
            request_builder = request_builder.body(body_data.clone());
        }
        
        // Apply authentication
        request_builder = self.apply_oauth2_auth(request_builder).await;
        
        // Execute the request
        let response = request_builder.send().await
            .map_err(|e| map_reqwest_error(&e))?;
        
        let status_code = response.status().as_u16();
        let headers: HashMap<String, String> = response.headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        
        let protocol = response.version();
        let protocol_str = match protocol {
            reqwest::Version::HTTP_09 => "HTTP/0.9",
            reqwest::Version::HTTP_10 => "HTTP/1.0", 
            reqwest::Version::HTTP_11 => "HTTP/1.1",
            reqwest::Version::HTTP_2 => "HTTP/2",
            reqwest::Version::HTTP_3 => "HTTP/3",
            _ => "HTTP/1.1",
        }.to_string();
        
        let protocol_version = match protocol {
            reqwest::Version::HTTP_09 => 0.9,
            reqwest::Version::HTTP_10 => 1.0,
            reqwest::Version::HTTP_11 => 1.1,
            reqwest::Version::HTTP_2 => 2.0,
            reqwest::Version::HTTP_3 => 3.0,
            _ => 1.1,
        };
        
        // Read response body
        let content = response.bytes().await
            .map_err(|e| map_reqwest_error(&e))?
            .to_vec();
        
        let elapsed = request_start.elapsed().as_secs_f64();
        
        Ok(Response {
            status_code,
            headers,
            content,
            url: full_url,
            elapsed,
            protocol: Some(protocol_str),
            protocol_version: Some(protocol_version),
            protocol_stats: None,
            request_time: elapsed,
            response_time: elapsed,
            total_time: elapsed,
            start_time: elapsed,
            end_time: elapsed,
            timing: Some(elapsed),
        })
    }

    /// Apply OAuth2 authentication asynchronously if needed
    async fn apply_oauth2_auth(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(ref auth_config) = self.auth_config {
            if auth_config.is_oauth2() {
                // Check if we have a valid token
                let mut token_guard = self.oauth2_token.lock().await;
                
                // Check if token needs refresh
                let needs_refresh = if let Some(ref token) = *token_guard {
                    token.is_expired()
                } else {
                    true
                };
                
                if needs_refresh {
                    // Fetch new token
                    if let Ok(new_token) = auth_config.fetch_oauth2_token().await {
                        *token_guard = Some(new_token);
                    }
                }
                
                // Apply token to request
                if let Some(ref token) = *token_guard {
                    return request.bearer_auth(&token.access_token);
                }
            }
        }
        
        // For non-OAuth2 auth, use the synchronous method
        auth_common::apply_authentication(request, self.auth_config.as_ref())
    }

    /// Rebuild the HTTP client with current configuration (internal method)
    fn rebuild_client(&mut self) -> PyResult<()> {
        let mut client_builder = Client::builder()
            .timeout(Duration::from_secs_f64(self.timeout_config.read_timeout.unwrap_or(30.0)))
            .pool_max_idle_per_host(self.pool_config.max_idle_per_host)
            .pool_idle_timeout(Duration::from_secs_f64(self.pool_config.idle_timeout));
        
        // Configure timeouts
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
        if let Some(ref proxy_cfg) = self.proxy_config {
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
        
        self.client = client_builder.build().map_err(|e| map_reqwest_error(&e))?;
        Ok(())
    }
}
