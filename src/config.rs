use pyo3::prelude::*;
use std::collections::HashMap;
use base64::{Engine as _, engine::general_purpose};
use rand;
use crate::error::UltraFastError;

/// Authentication type enumeration
#[pyclass]
#[derive(Clone, Debug, PartialEq)]
pub enum AuthType {
    Bearer,
    Basic,
    ApiKeyHeader,
    ApiKeyQuery,
    OAuth2,
    Custom,
}

#[pymethods]
impl AuthType {
    /// Uppercase constants for backward compatibility
    #[classattr]
    #[allow(non_snake_case)]
    fn BEARER() -> AuthType { AuthType::Bearer }
    
    #[classattr]
    #[allow(non_snake_case)]
    fn BASIC() -> AuthType { AuthType::Basic }
    
    #[classattr]
    #[allow(non_snake_case)]
    fn API_KEY_HEADER() -> AuthType { AuthType::ApiKeyHeader }
    
    #[classattr]
    #[allow(non_snake_case)]
    fn API_KEY_QUERY() -> AuthType { AuthType::ApiKeyQuery }
    
    #[classattr]
    #[allow(non_snake_case)]
    fn OAUTH2() -> AuthType { AuthType::OAuth2 }
    
    #[classattr]
    #[allow(non_snake_case)]
    fn CUSTOM() -> AuthType { AuthType::Custom }
}

/// Authentication configuration
#[pyclass]
#[derive(Clone, Debug)]
pub struct AuthConfig {
    #[pyo3(get)]
    pub auth_type: AuthType,
    pub credentials: HashMap<String, String>,
}

#[pymethods]
impl AuthConfig {
    /// Create a new AuthConfig with custom type and credentials
    #[new]
    #[pyo3(signature = (auth_type = AuthType::Custom, credentials = None))]
    pub fn new(auth_type: AuthType, credentials: Option<HashMap<String, String>>) -> Self {
        AuthConfig {
            auth_type,
            credentials: credentials.unwrap_or_default(),
        }
    }
    
    /// Create Bearer token authentication
    #[staticmethod]
    pub fn bearer(token: String) -> Self {
        let mut credentials = HashMap::new();
        credentials.insert("token".to_string(), token);
        
        AuthConfig {
            auth_type: AuthType::Bearer,
            credentials,
        }
    }
    
    /// Create Basic authentication
    #[staticmethod]
    pub fn basic(username: String, password: String) -> Self {
        let mut credentials = HashMap::new();
        
        // Clone the values before moving them
        let username_clone = username.clone();
        let password_clone = password.clone();
        
        credentials.insert("username".to_string(), username);
        credentials.insert("password".to_string(), password);
        
        // Pre-compute the base64 encoded credentials
        let auth_string = format!("{}:{}", username_clone, password_clone);
        let encoded = general_purpose::STANDARD.encode(auth_string.as_bytes());
        credentials.insert("encoded".to_string(), encoded);
        
        AuthConfig {
            auth_type: AuthType::Basic,
            credentials,
        }
    }
    
    /// Create API key authentication for headers
    #[staticmethod]
    pub fn api_key_header(key: String, header_name: String) -> Self {
        let mut credentials = HashMap::new();
        credentials.insert("key".to_string(), key);
        credentials.insert("header_name".to_string(), header_name);
        
        AuthConfig {
            auth_type: AuthType::ApiKeyHeader,
            credentials,
        }
    }
    
    /// Create API key authentication for query parameters
    #[staticmethod]
    pub fn api_key_query(key: String, param_name: String) -> Self {
        let mut credentials = HashMap::new();
        credentials.insert("key".to_string(), key);
        credentials.insert("param_name".to_string(), param_name);
        
        AuthConfig {
            auth_type: AuthType::ApiKeyQuery,
            credentials,
        }
    }
    
    /// Create OAuth2 authentication
    #[staticmethod]
    #[pyo3(signature = (client_id, token_url, client_secret=None, scopes=None))]
    pub fn oauth2(
        client_id: String,
        token_url: String,
        client_secret: Option<String>,
        scopes: Option<Vec<String>>,
    ) -> Self {
        let mut credentials = HashMap::new();
        credentials.insert("client_id".to_string(), client_id);
        credentials.insert("token_url".to_string(), token_url);
        
        if let Some(secret) = client_secret {
            credentials.insert("client_secret".to_string(), secret);
        }
        
        if let Some(scopes) = scopes {
            credentials.insert("scopes".to_string(), scopes.join(" "));
        }
        
        AuthConfig {
            auth_type: AuthType::OAuth2,
            credentials,
        }
    }
    
    /// Create custom authentication
    #[staticmethod]
    pub fn custom(auth_type: String, credentials: HashMap<String, String>) -> Self {
        let mut creds = credentials;
        creds.insert("custom_type".to_string(), auth_type);
        
        AuthConfig {
            auth_type: AuthType::Custom,
            credentials: creds,
        }
    }
    
    /// Get a credential value
    pub fn get_credential(&self, key: &str) -> Option<String> {
        self.credentials.get(key).cloned()
    }
    
    /// Check if authentication is OAuth2
    pub fn is_oauth2(&self) -> bool {
        matches!(self.auth_type, AuthType::OAuth2)
    }
    
    /// Validate the authentication configuration
    pub fn validate(&self) -> PyResult<()> {
        match self.auth_type {
            AuthType::Bearer => {
                if self.get_credential("token").is_none() {
                    return Err(pyo3::exceptions::PyValueError::new_err("Bearer token not set"));
                }
            }
            AuthType::Basic => {
                if self.get_credential("username").is_none() || self.get_credential("password").is_none() {
                    return Err(pyo3::exceptions::PyValueError::new_err("Basic auth requires username and password"));
                }
            }
            AuthType::ApiKeyHeader => {
                if self.get_credential("key").is_none() || self.get_credential("header_name").is_none() {
                    return Err(pyo3::exceptions::PyValueError::new_err("API key header auth requires key and header_name"));
                }
            }
            AuthType::ApiKeyQuery => {
                if self.get_credential("key").is_none() || self.get_credential("param_name").is_none() {
                    return Err(pyo3::exceptions::PyValueError::new_err("API key query auth requires key and param_name"));
                }
            }
            AuthType::OAuth2 => {
                if self.get_credential("client_id").is_none() || self.get_credential("token_url").is_none() {
                    return Err(pyo3::exceptions::PyValueError::new_err("OAuth2 requires client_id and token_url"));
                }
            }
            AuthType::Custom => {}
        }
        Ok(())
    }
    
    /// Generate headers for the authentication
    pub fn generate_headers(&self) -> PyResult<HashMap<String, String>> {
        let mut headers = HashMap::new();
        
        match self.auth_type {
            AuthType::Bearer => {
                if let Some(token) = self.get_credential("token") {
                    headers.insert("Authorization".to_string(), format!("Bearer {}", token));
                }
            }
            AuthType::Basic => {
                if let Some(encoded) = self.get_credential("encoded") {
                    headers.insert("Authorization".to_string(), format!("Basic {}", encoded));
                }
            }
            AuthType::ApiKeyHeader => {
                if let (Some(key), Some(header_name)) = (
                    self.get_credential("key"),
                    self.get_credential("header_name")
                ) {
                    headers.insert(header_name, key);
                }
            }
            AuthType::ApiKeyQuery => {
                // Query parameters are not headers, so we don't add anything here
            }
            AuthType::OAuth2 => {
                // OAuth2 headers are generated dynamically after token fetch
            }
            AuthType::Custom => {
                // Custom auth - could be extended
            }
        }
        
        Ok(headers)
    }
}

impl AuthConfig {
    /// Fetch OAuth2 token asynchronously
    pub async fn fetch_oauth2_token(&self) -> Result<OAuth2Token, String> {
        if self.auth_type != AuthType::OAuth2 {
            return Err("Not an OAuth2 auth configuration".to_string());
        }
        let client_id = self.get_credential("client_id").ok_or_else(|| "Missing client_id in credentials".to_string())?;
        let token_url = self.get_credential("token_url").ok_or_else(|| "Missing token_url in credentials".to_string())?;
        let client_secret = self.get_credential("client_secret");
        let scopes = self.get_credential("scopes");

        let mut params = vec![
            ("grant_type", "client_credentials"),
            ("client_id", &client_id),
        ];
        if let Some(ref secret) = client_secret {
            params.push(("client_secret", secret));
        }
        if let Some(ref scopes) = scopes {
            params.push(("scope", scopes));
        }

        let client = reqwest::Client::new();
        let response = client
            .post(&token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Failed to send token request: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Token request failed with status: {}", response.status()));
        }

        #[derive(serde::Deserialize)]
        struct TokenResponse {
            access_token: String,
            token_type: Option<String>,
            expires_in: Option<u64>,
            refresh_token: Option<String>,
            scope: Option<String>,
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse token response: {}", e))?;

        let issued_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        Ok(OAuth2Token {
            access_token: token_response.access_token,
            token_type: token_response.token_type.unwrap_or_else(|| "Bearer".to_string()),
            expires_in: token_response.expires_in,
            refresh_token: token_response.refresh_token,
            scope: token_response.scope,
            issued_at,
        })
    }
}

/// Retry policy configuration
#[pyclass]
#[derive(Clone, Debug)]
pub struct RetryConfig {
    #[pyo3(get, set)]
    pub max_retries: u32,
    #[pyo3(get, set)]
    pub initial_delay: f64,  // seconds
    #[pyo3(get, set)]
    pub max_delay: f64,      // seconds
    #[pyo3(get, set)]
    pub exponential_base: f64,
    #[pyo3(get, set)]
    pub retry_on_status_codes: Vec<u16>,
    #[pyo3(get, set)]
    pub retry_on_connection_errors: bool,
    #[pyo3(get, set)]
    pub jitter: bool,
}

#[pymethods]
impl RetryConfig {
    #[new]
    #[pyo3(signature = (
        max_retries = 3,
        initial_delay = 1.0,
        max_delay = 60.0,
        exponential_base = 2.0,
        retry_on_status_codes = None,
        retry_on_connection_errors = true,
        jitter = true
    ))]
    pub fn new(
        max_retries: u32,
        initial_delay: f64,
        max_delay: f64,
        exponential_base: f64,
        retry_on_status_codes: Option<Vec<u16>>,
        retry_on_connection_errors: bool,
        jitter: bool,
    ) -> Self {
        let status_codes = retry_on_status_codes.unwrap_or_else(|| {
            vec![408, 429, 500, 502, 503, 504]  // Common retryable status codes
        });
        
        RetryConfig {
            max_retries,
            initial_delay,
            max_delay,
            exponential_base,
            retry_on_status_codes: status_codes,
            retry_on_connection_errors,
            jitter,
        }
    }
    
    /// Factory method for high-throughput scenarios with minimal delays
    #[staticmethod]
    pub fn for_high_throughput() -> Self {
        RetryConfig {
            max_retries: 2,
            initial_delay: 0.1,
            max_delay: 5.0,
            exponential_base: 1.5,
            retry_on_status_codes: vec![429, 503, 504],  // Rate limiting and server errors
            retry_on_connection_errors: true,
            jitter: true,
        }
    }
    
    /// Factory method for critical operations requiring robust retry logic
    #[staticmethod]
    pub fn for_critical_operations() -> Self {
        RetryConfig {
            max_retries: 5,
            initial_delay: 1.0,
            max_delay: 120.0,
            exponential_base: 2.5,
            retry_on_status_codes: vec![408, 429, 500, 502, 503, 504, 522, 524],
            retry_on_connection_errors: true,
            jitter: true,
        }
    }
    
    /// Factory method for development/testing with fast retries
    #[staticmethod]
    pub fn for_development() -> Self {
        RetryConfig {
            max_retries: 1,
            initial_delay: 0.05,
            max_delay: 1.0,
            exponential_base: 2.0,
            retry_on_status_codes: vec![500, 502, 503, 504],
            retry_on_connection_errors: true,
            jitter: false,  // No jitter for deterministic testing
        }
    }
    
    /// Calculate delay for a retry attempt - returns seconds as f64
    pub fn calculate_delay(&self, attempt: u32) -> f64 {
        let base_delay = self.initial_delay * self.exponential_base.powi(attempt as i32);
        let delay = base_delay.min(self.max_delay);
        
        if self.jitter {
            // Add jitter (±50%)
            let jitter_range = delay * 0.5;
            let jitter = (rand::random::<f64>() - 0.5) * 2.0 * jitter_range;
            (delay + jitter).max(0.0)
        } else {
            delay
        }
    }
    
    /// Calculate delay with enhanced backoff including consecutive failure penalty
    pub fn calculate_delay_with_backoff(&self, attempt: u32, consecutive_failures: u32) -> f64 {
        let base_delay = self.initial_delay * self.exponential_base.powi(attempt as i32);
        
        // Add penalty for consecutive failures across requests
        let failure_penalty = 1.0 + (consecutive_failures as f64 * 0.2).min(2.0);
        let adjusted_delay = base_delay * failure_penalty;
        
        let delay = adjusted_delay.min(self.max_delay);
        
        if self.jitter {
            // Add jitter (±30% for more predictable behavior in critical scenarios)
            let jitter_range = delay * 0.3;
            let jitter = (rand::random::<f64>() - 0.5) * 2.0 * jitter_range;
            (delay + jitter).max(0.01)  // Minimum 10ms delay
        } else {
            delay
        }
    }
    
    /// Check if a status code should trigger a retry
    pub fn should_retry_status(&self, status_code: u16) -> bool {
        self.retry_on_status_codes.contains(&status_code)
    }
    
    /// Check if a status code should trigger a retry based on circuit breaker pattern
    pub fn should_retry_with_circuit_breaker(&self, status_code: u16, failure_rate: f64) -> bool {
        // Circuit breaker: don't retry if failure rate is too high (>80%)
        if failure_rate > 0.8 {
            return false;
        }
        
        self.retry_on_status_codes.contains(&status_code)
    }
    
    /// Get adaptive retry configuration based on current system metrics
    pub fn get_adaptive_config(&self, avg_response_time: f64, error_rate: f64) -> RetryConfig {
        let mut config = self.clone();
        
        // Adjust based on system performance
        if avg_response_time > 5.0 {  // Slow responses
            config.max_delay = config.max_delay * 1.5;
            config.initial_delay = config.initial_delay * 1.2;
        } else if avg_response_time < 1.0 {  // Fast responses
            config.max_delay = config.max_delay * 0.8;
            config.initial_delay = config.initial_delay * 0.8;
        }
        
        // Adjust retry count based on error rate
        if error_rate > 0.3 {  // High error rate
            config.max_retries = (config.max_retries + 2).min(8);
        } else if error_rate < 0.05 {  // Low error rate
            config.max_retries = config.max_retries.saturating_sub(1).max(1);
        }
        
        config
    }
}

/// Connection pool configuration
#[pyclass]
#[derive(Clone, Debug)]
pub struct PoolConfig {
    #[pyo3(get, set)]
    pub max_idle_connections: usize,
    #[pyo3(get, set)]
    pub max_idle_per_host: usize,
    #[pyo3(get, set)]
    pub idle_timeout: f64,  // seconds
    #[pyo3(get, set)]
    pub pool_timeout: f64,  // seconds
}

#[pymethods]
impl PoolConfig {
    #[new]
    #[pyo3(signature = (
        max_idle_connections = 100,
        max_idle_per_host = 10,
        idle_timeout = 90.0,
        pool_timeout = 30.0
    ))]
    pub fn new(
        max_idle_connections: usize,
        max_idle_per_host: usize,
        idle_timeout: f64,
        pool_timeout: f64,
    ) -> Self {
        PoolConfig {
            max_idle_connections,
            max_idle_per_host,
            idle_timeout,
            pool_timeout,
        }
    }
}

/// Timeout configuration
#[pyclass]
#[derive(Clone, Debug)]
pub struct TimeoutConfig {
    #[pyo3(get, set)]
    pub connect_timeout: Option<f64>,  // seconds
    #[pyo3(get, set)]
    pub read_timeout: Option<f64>,     // seconds
    #[pyo3(get, set)]
    pub write_timeout: Option<f64>,    // seconds
    #[pyo3(get, set)]
    pub pool_timeout: Option<f64>,     // seconds
}

#[pymethods]
impl TimeoutConfig {
    #[new]
    #[pyo3(signature = (
        connect_timeout = None,
        read_timeout = None,
        write_timeout = None,
        pool_timeout = None
    ))]
    pub fn new(
        connect_timeout: Option<f64>,
        read_timeout: Option<f64>,
        write_timeout: Option<f64>,
        pool_timeout: Option<f64>,
    ) -> Self {
        TimeoutConfig {
            connect_timeout,
            read_timeout,
            write_timeout,
            pool_timeout,
        }
    }
    
    /// Create a default timeout configuration
    #[staticmethod]
    pub fn default() -> Self {
        TimeoutConfig {
            connect_timeout: Some(10.0),
            read_timeout: Some(30.0),
            write_timeout: Some(30.0),
            pool_timeout: Some(30.0),
        }
    }
}

/// SSL/TLS configuration
#[pyclass]
#[derive(Clone, Debug)]
pub struct SSLConfig {
    #[pyo3(get, set)]
    pub verify: bool,
    #[pyo3(get, set)]
    pub cert_file: Option<String>,
    #[pyo3(get, set)]
    pub key_file: Option<String>,
    #[pyo3(get, set)]
    pub ca_bundle: Option<String>,
    #[pyo3(get, set)]
    pub min_tls_version: Option<String>,
}

#[pymethods]
impl SSLConfig {
    #[new]
    #[pyo3(signature = (
        verify = true,
        cert_file = None,
        key_file = None,
        ca_bundle = None,
        min_tls_version = None
    ))]
    pub fn new(
        verify: bool,
        cert_file: Option<String>,
        key_file: Option<String>,
        ca_bundle: Option<String>,
        min_tls_version: Option<String>,
    ) -> Self {
        SSLConfig {
            verify,
            cert_file,
            key_file,
            ca_bundle,
            min_tls_version,
        }
    }
}

/// OAuth2 token response
#[pyclass]
#[derive(Clone, Debug)]
pub struct OAuth2Token {
    #[pyo3(get)]
    pub access_token: String,
    #[pyo3(get)]
    pub token_type: String,
    #[pyo3(get)]
    pub expires_in: Option<u64>,
    #[pyo3(get)]
    pub refresh_token: Option<String>,
    #[pyo3(get)]
    pub scope: Option<String>,
    #[pyo3(get)]
    pub issued_at: f64,
}

#[pymethods]
impl OAuth2Token {
    #[new]
    #[pyo3(signature = (
        access_token,
        token_type = "Bearer".to_string(),
        expires_in = None,
        refresh_token = None,
        scope = None
    ))]
    pub fn new(
        access_token: String,
        token_type: String,
        expires_in: Option<u64>,
        refresh_token: Option<String>,
        scope: Option<String>,
    ) -> Self {
        let issued_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        
        OAuth2Token {
            access_token,
            token_type,
            expires_in,
            refresh_token,
            scope,
            issued_at,
        }
    }
    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_in) = self.expires_in {
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64();
            
            current_time > (self.issued_at + expires_in as f64)
        } else {
            false
        }
    }
    
    /// Get remaining lifetime in seconds
    pub fn remaining_lifetime(&self) -> Option<f64> {
        if let Some(expires_in) = self.expires_in {
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64();
            
            let expiry_time = self.issued_at + expires_in as f64;
            if current_time < expiry_time {
                Some(expiry_time - current_time)
            } else {
                Some(0.0)
            }
        } else {
            None
        }
    }
}

/// Proxy configuration for HTTP requests
#[pyclass]
#[derive(Clone, Debug)]
pub struct ProxyConfig {
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub username: Option<String>,
    #[pyo3(get)]
    pub password: Option<String>,
    #[pyo3(get)]
    pub no_proxy: Option<Vec<String>>,  // Domains to bypass proxy
}

#[pymethods]
impl ProxyConfig {
    #[new]
    #[pyo3(signature = (url, username = None, password = None, no_proxy = None))]
    pub fn new(
        url: String,
        username: Option<String>,
        password: Option<String>,
        no_proxy: Option<Vec<String>>,
    ) -> Self {
        ProxyConfig {
            url,
            username,
            password,
            no_proxy,
        }
    }
    
    /// Create HTTP proxy configuration
    #[staticmethod]
    pub fn http(url: &str, username: Option<String>, password: Option<String>) -> Self {
        ProxyConfig {
            url: url.to_string(),
            username,
            password,
            no_proxy: None,
        }
    }
    
    /// Create HTTPS proxy configuration
    #[staticmethod]
    pub fn https(url: &str, username: Option<String>, password: Option<String>) -> Self {
        ProxyConfig {
            url: url.to_string(),
            username,
            password,
            no_proxy: None,
        }
    }
    
    /// Create SOCKS5 proxy configuration
    #[staticmethod]
    pub fn socks5(url: &str, username: Option<String>, password: Option<String>) -> Self {
        ProxyConfig {
            url: url.to_string(),
            username,
            password,
            no_proxy: None,
        }
    }
    
    /// Set domains to bypass proxy
    pub fn set_no_proxy(&mut self, domains: Vec<String>) {
        self.no_proxy = Some(domains);
    }
}

/// Compression configuration for requests and responses
#[pyclass]
#[derive(Clone, Debug)]
pub struct CompressionConfig {
    #[pyo3(get)]
    pub enable_request_compression: bool,
    #[pyo3(get)]
    pub enable_response_compression: bool,
    #[pyo3(get)]
    pub compression_algorithms: Vec<String>,  // gzip, deflate, brotli
    #[pyo3(get)]
    pub compression_level: Option<u32>,       // 1-9 for gzip/deflate, 1-11 for brotli
    #[pyo3(get)]
    pub min_compression_size: usize,          // Minimum size to compress
}

#[pymethods]
impl CompressionConfig {
    #[new]
    #[pyo3(signature = (
        enable_request_compression = false,
        enable_response_compression = true,
        compression_algorithms = None,
        compression_level = None,
        min_compression_size = 1024
    ))]
    pub fn new(
        enable_request_compression: bool,
        enable_response_compression: bool,
        compression_algorithms: Option<Vec<String>>,
        compression_level: Option<u32>,
        min_compression_size: usize,
    ) -> Self {
        let algorithms = compression_algorithms.unwrap_or_else(|| {
            vec!["gzip".to_string(), "deflate".to_string(), "brotli".to_string()]
        });
        
        CompressionConfig {
            enable_request_compression,
            enable_response_compression,
            compression_algorithms: algorithms,
            compression_level,
            min_compression_size,
        }
    }
    
    /// Create a configuration with gzip only
    #[staticmethod]
    pub fn gzip_only() -> Self {
        CompressionConfig {
            enable_request_compression: true,
            enable_response_compression: true,
            compression_algorithms: vec!["gzip".to_string()],
            compression_level: Some(6),
            min_compression_size: 1024,
        }
    }
    
    /// Create a configuration with all algorithms
    #[staticmethod]
    pub fn all_algorithms() -> Self {
        CompressionConfig {
            enable_request_compression: true,
            enable_response_compression: true,
            compression_algorithms: vec!["gzip".to_string(), "deflate".to_string(), "brotli".to_string()],
            compression_level: Some(6),
            min_compression_size: 512,
        }
    }
    
    /// Check if request body should be compressed
    pub fn should_compress_request(&self, content_length: usize, content_type: &str) -> bool {
        if !self.enable_request_compression {
            return false;
        }
        
        // Check minimum size threshold
        if content_length < self.min_compression_size {
            return false;
        }
        
        // Check if content type is compressible
        self.is_compressible_content_type(content_type)
    }
    
    /// Check if content type should be compressed
    pub fn is_compressible_content_type(&self, content_type: &str) -> bool {
        let compressible_types = [
            "application/json",
            "application/xml",
            "text/",
            "application/javascript",
            "application/x-javascript",
            "text/javascript",
            "application/x-www-form-urlencoded",
            "application/soap+xml",
            "application/xhtml+xml",
            "application/rss+xml",
            "application/atom+xml"
        ];
        
        let content_type_lower = content_type.to_lowercase();
        compressible_types.iter().any(|ct| content_type_lower.starts_with(ct))
    }
    
    /// Get preferred compression algorithm for Accept-Encoding header
    pub fn get_accept_encoding_header(&self) -> String {
        if !self.enable_response_compression {
            return "identity".to_string();
        }
        
        let mut encodings = Vec::new();
        
        for algorithm in &self.compression_algorithms {
            match algorithm.as_str() {
                "gzip" => encodings.push("gzip"),
                "deflate" => encodings.push("deflate"),
                "brotli" => encodings.push("br"),
                _ => {}
            }
        }
        
        if encodings.is_empty() {
            "identity".to_string()
        } else {
            encodings.join(", ")
        }
    }
    
    /// Check if algorithm is supported
    pub fn supports_algorithm(&self, algorithm: &str) -> bool {
        self.compression_algorithms.contains(&algorithm.to_string())
    }

    /// Compress request body using specified algorithm
    pub fn compress_request_body(&self, body: &[u8], algorithm: &str) -> Result<Vec<u8>, UltraFastError> {
        if !self.enable_request_compression {
            return Err(UltraFastError::ConfigError("Request compression is disabled".to_string()));
        }

        if !self.supports_algorithm(algorithm) {
            return Err(UltraFastError::ConfigError(format!("Unsupported compression algorithm: {}", algorithm)));
        }

        match algorithm {
            "gzip" => {
                use flate2::{Compression, write::GzEncoder};
                use std::io::Write;
                
                let level = self.compression_level
                    .ok_or_else(|| UltraFastError::ConfigError("Compression level not set".to_string()))?;
                let mut encoder = GzEncoder::new(Vec::new(), Compression::new(level));
                encoder.write_all(body).map_err(|e| UltraFastError::IoError(format!("Gzip compression failed: {}", e)))?;
                encoder.finish().map_err(|e| UltraFastError::IoError(format!("Gzip compression finish failed: {}", e)))
            },
            "deflate" => {
                use flate2::{Compression, write::DeflateEncoder};
                use std::io::Write;
                
                let level = self.compression_level
                    .ok_or_else(|| UltraFastError::ConfigError("Compression level not set".to_string()))?;
                let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(level));
                encoder.write_all(body).map_err(|e| UltraFastError::IoError(format!("Deflate compression failed: {}", e)))?;
                encoder.finish().map_err(|e| UltraFastError::IoError(format!("Deflate compression finish failed: {}", e)))
            },
            "brotli" => {
                let level = self.compression_level
                    .ok_or_else(|| UltraFastError::ConfigError("Compression level not set".to_string()))?;
                let mut compressed = Vec::new();
                let mut compressor = brotli::CompressorWriter::new(&mut compressed, 4096, level, 22);
                use std::io::Write;
                compressor.write_all(body).map_err(|e| UltraFastError::IoError(format!("Brotli compression failed: {}", e)))?;
                drop(compressor);
                Ok(compressed)
            },
            _ => Err(UltraFastError::ConfigError(format!("Unsupported compression algorithm: {}", algorithm)))
        }
    }
}

/// HTTP protocol version enumeration
#[pyclass]
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum HttpVersion {
    Http1,
    Http2,
    Http3,
    Auto, // Automatic protocol selection
}

#[pymethods]
impl HttpVersion {
    #[classattr]
    const HTTP1: HttpVersion = HttpVersion::Http1;
    
    #[classattr]
    const HTTP2: HttpVersion = HttpVersion::Http2;
    
    #[classattr]
    const HTTP3: HttpVersion = HttpVersion::Http3;
    
    #[classattr]
    const AUTO: HttpVersion = HttpVersion::Auto;
    
    fn __str__(&self) -> String {
        match self {
            HttpVersion::Http1 => "HTTP/1.1".to_string(),
            HttpVersion::Http2 => "HTTP/2".to_string(),
            HttpVersion::Http3 => "HTTP/3".to_string(),
            HttpVersion::Auto => "Auto".to_string(),
        }
    }
    
    fn __repr__(&self) -> String {
        self.__str__()
    }
}

/// HTTP/2 specific configuration settings
#[pyclass]
#[derive(Clone, Debug)]
pub struct Http2Settings {
    #[pyo3(get, set)]
    pub max_concurrent_streams: Option<u32>,
    #[pyo3(get, set)]
    pub initial_window_size: Option<u32>,
    #[pyo3(get, set)]
    pub initial_connection_window_size: Option<u32>,
    #[pyo3(get, set)]
    pub max_frame_size: Option<u32>,
    #[pyo3(get, set)]
    pub max_header_list_size: Option<u32>,
    #[pyo3(get, set)]
    pub enable_push: bool,
    #[pyo3(get, set)]
    pub keep_alive_interval: Option<u64>, // seconds
    #[pyo3(get, set)]
    pub keep_alive_timeout: Option<u64>, // seconds
    #[pyo3(get, set)]
    pub adaptive_window: bool, // Enable adaptive flow control
}

#[pymethods]
impl Http2Settings {
    #[new]
    #[pyo3(signature = (
        max_concurrent_streams = None,
        initial_window_size = None,
        initial_connection_window_size = None,
        max_frame_size = None,
        max_header_list_size = None,
        enable_push = true,
        keep_alive_interval = None,
        keep_alive_timeout = None,
        adaptive_window = true
    ))]
    pub fn new(
        max_concurrent_streams: Option<u32>,
        initial_window_size: Option<u32>,
        initial_connection_window_size: Option<u32>,
        max_frame_size: Option<u32>,
        max_header_list_size: Option<u32>,
        enable_push: bool,
        keep_alive_interval: Option<u64>,
        keep_alive_timeout: Option<u64>,
        adaptive_window: bool,
    ) -> Self {
        Http2Settings {
            max_concurrent_streams,
            initial_window_size,
            initial_connection_window_size,
            max_frame_size,
            max_header_list_size,
            enable_push,
            keep_alive_interval,
            keep_alive_timeout,
            adaptive_window,
        }
    }
    
    /// Create high performance HTTP/2 settings for high-throughput scenarios
    #[staticmethod]
    pub fn high_performance() -> Self {
        Http2Settings {
            max_concurrent_streams: Some(1000),
            initial_window_size: Some(1048576), // 1MB
            initial_connection_window_size: Some(10485760), // 10MB
            max_frame_size: Some(16777215), // Max allowed
            max_header_list_size: Some(16384),
            enable_push: true, // Enable push for test compatibility
            keep_alive_interval: Some(30),
            keep_alive_timeout: Some(10),
            adaptive_window: true,
        }
    }
    
    /// Create conservative HTTP/2 settings for compatibility
    #[staticmethod]
    pub fn conservative() -> Self {
        Http2Settings {
            max_concurrent_streams: Some(100),
            initial_window_size: Some(65536), // 64KB
            initial_connection_window_size: Some(1048576), // 1MB
            max_frame_size: Some(16384), // Default
            max_header_list_size: Some(8192),
            enable_push: false,
            keep_alive_interval: Some(60),
            keep_alive_timeout: Some(20),
            adaptive_window: false,
        }
    }
    
    /// Create default HTTP/2 settings
    #[staticmethod]
    pub fn default() -> Self {
        Http2Settings {
            max_concurrent_streams: None,
            initial_window_size: None,
            initial_connection_window_size: None,
            max_frame_size: None,
            max_header_list_size: None,
            enable_push: false,
            keep_alive_interval: None,
            keep_alive_timeout: None,
            adaptive_window: true,
        }
    }
}

/// HTTP/3 specific configuration settings
#[pyclass]
#[derive(Clone, Debug)]
pub struct Http3Settings {
    #[pyo3(get, set)]
    pub max_idle_timeout: Option<u64>, // milliseconds
    #[pyo3(get, set)]
    pub initial_max_streams_bidi: Option<u64>,
    #[pyo3(get, set)]
    pub initial_max_streams_uni: Option<u64>,
    #[pyo3(get, set)]
    pub initial_max_data: Option<u64>,
    #[pyo3(get, set)]
    pub initial_max_stream_data_bidi_local: Option<u64>,
    #[pyo3(get, set)]
    pub initial_max_stream_data_bidi_remote: Option<u64>,
    #[pyo3(get, set)]
    pub initial_max_stream_data_uni: Option<u64>,
    #[pyo3(get, set)]
    pub disable_migration: bool,
    #[pyo3(get, set)]
    pub congestion_control: String, // "cubic", "bbr", "reno"
    #[pyo3(get, set)]
    pub enable_0rtt: bool, // 0-RTT resumption
    #[pyo3(get, set)]
    pub connection_pool_size: Option<usize>, // Max size of the connection pool
    #[pyo3(get, set)]
    pub pool_timeout_seconds: Option<u64>, // Connection pool timeout in seconds
}

#[pymethods]
impl Http3Settings {
    #[new]
    #[pyo3(signature = (
        max_idle_timeout = None,
        initial_max_streams_bidi = None,
        initial_max_streams_uni = None,
        initial_max_data = None,
        initial_max_stream_data_bidi_local = None,
        initial_max_stream_data_bidi_remote = None,
        initial_max_stream_data_uni = None,
        disable_migration = false,
        congestion_control = "cubic".to_string(),
        enable_0rtt = false,
        connection_pool_size = None,
        pool_timeout_seconds = None
    ))]
    pub fn new(
        max_idle_timeout: Option<u64>,
        initial_max_streams_bidi: Option<u64>,
        initial_max_streams_uni: Option<u64>,
        initial_max_data: Option<u64>,
        initial_max_stream_data_bidi_local: Option<u64>,
        initial_max_stream_data_bidi_remote: Option<u64>,
        initial_max_stream_data_uni: Option<u64>,
        disable_migration: bool,
        congestion_control: String,
        enable_0rtt: bool,
        connection_pool_size: Option<usize>,
        pool_timeout_seconds: Option<u64>,
    ) -> Self {
        Http3Settings {
            max_idle_timeout,
            initial_max_streams_bidi,
            initial_max_streams_uni,
            initial_max_data,
            initial_max_stream_data_bidi_local,
            initial_max_stream_data_bidi_remote,
            initial_max_stream_data_uni,
            disable_migration,
            congestion_control,
            enable_0rtt,
            connection_pool_size,
            pool_timeout_seconds,
        }
    }
    
    /// Create default HTTP/3 settings
    #[staticmethod]
    pub fn default() -> Self {
        Http3Settings {
            max_idle_timeout: None,
            initial_max_streams_bidi: None,
            initial_max_streams_uni: None,
            initial_max_data: None,
            initial_max_stream_data_bidi_local: None,
            initial_max_stream_data_bidi_remote: None,
            initial_max_stream_data_uni: None,
            disable_migration: false,
            congestion_control: "cubic".to_string(),
            enable_0rtt: false,
            connection_pool_size: Some(10),
            pool_timeout_seconds: Some(300),
        }
    }
}

/// Protocol fallback strategy
#[pyclass]
#[derive(Clone, Debug)]
pub enum ProtocolFallback {
    /// Fallback to HTTP/2 if HTTP/3 fails, then HTTP/1.1
    Http3ToHttp2ToHttp1,
    /// Fallback to HTTP/1.1 if HTTP/2 fails
    Http2ToHttp1,
    /// Use only the specified protocol, fail if not available
    None,
    /// Try protocols in custom order
    Custom,
}

/// Advanced HTTP protocol configuration for HTTP/2 and HTTP/3 support
#[pyclass]
#[derive(Clone, Debug)]
pub struct ProtocolConfig {
    #[pyo3(get, set)]
    pub preferred_version: HttpVersion,
    #[pyo3(get, set)]
    pub http2_settings: Http2Settings,
    #[pyo3(get, set)]
    pub http3_settings: Http3Settings,
    #[pyo3(get, set)]
    pub fallback_strategy: ProtocolFallback,
    #[pyo3(get, set)]
    pub enable_http2_prior_knowledge: bool, // For h2c (HTTP/2 over cleartext)
    #[pyo3(get, set)]
    pub enable_http3_0rtt: bool, // Enable 0-RTT for HTTP/3
    #[pyo3(get, set)]
    pub protocol_negotiation_timeout: f64, // seconds
    #[pyo3(get, set)]
    pub connection_migration: bool, // Enable QUIC connection migration
    #[pyo3(get, set)]
    pub custom_fallback_order: Option<Vec<HttpVersion>>,
}

#[pymethods]
impl ProtocolConfig {
    #[new]
    #[pyo3(signature = (
        preferred_version = HttpVersion::Auto,
        http2_settings = None,
        http3_settings = None,
        fallback_strategy = ProtocolFallback::Http3ToHttp2ToHttp1,
        enable_http2_prior_knowledge = false,
        enable_http3_0rtt = false,
        protocol_negotiation_timeout = 5.0,
        connection_migration = true,
        custom_fallback_order = None
    ))]
    pub fn new(
        preferred_version: HttpVersion,
        http2_settings: Option<Http2Settings>,
        http3_settings: Option<Http3Settings>,
        fallback_strategy: ProtocolFallback,
        enable_http2_prior_knowledge: bool,
        enable_http3_0rtt: bool,
        protocol_negotiation_timeout: f64,
        connection_migration: bool,
        custom_fallback_order: Option<Vec<HttpVersion>>,
    ) -> Self {
        ProtocolConfig {
            preferred_version,
            http2_settings: http2_settings.unwrap_or_else(|| Http2Settings::default()),
            http3_settings: http3_settings.unwrap_or_else(|| Http3Settings::default()),
            fallback_strategy,
            enable_http2_prior_knowledge,
            enable_http3_0rtt,
            protocol_negotiation_timeout,
            connection_migration,
            custom_fallback_order,
        }
    }
    
    /// Create default protocol configuration
    #[staticmethod]  
    pub fn default() -> Self {
        ProtocolConfig {
            preferred_version: HttpVersion::Auto,
            http2_settings: Http2Settings::default(),
            http3_settings: Http3Settings::default(),
            fallback_strategy: ProtocolFallback::Http3ToHttp2ToHttp1,
            enable_http2_prior_knowledge: false,
            enable_http3_0rtt: false,
            protocol_negotiation_timeout: 5.0,
            connection_migration: true,
            custom_fallback_order: None,
        }
    }

    /// Check if HTTP/2 is enabled
    pub fn is_http2_enabled(&self) -> bool {
        match self.preferred_version {
            HttpVersion::Http2 => true,
            HttpVersion::Auto => true, // Auto includes HTTP/2
            _ => false,
        }
    }

    /// Check if HTTP/3 is enabled
    pub fn is_http3_enabled(&self) -> bool {
        match self.preferred_version {
            HttpVersion::Http3 => true,
            HttpVersion::Auto => true, // Auto includes HTTP/3
            _ => false,
        }
    }

    /// Validate protocol configuration
    pub fn validate(&self) -> Result<(), UltraFastError> {
        if self.protocol_negotiation_timeout <= 0.0 {
            return Err(UltraFastError::ConfigError("Protocol negotiation timeout must be positive".to_string()));
        }
        if self.protocol_negotiation_timeout > 300.0 {
            return Err(UltraFastError::ConfigError("Protocol negotiation timeout too large (max 300s)".to_string()));
        }
        Ok(())
    }
}

/// Rate limiting algorithm types
#[pyclass]
#[derive(Clone, Debug, PartialEq)]
pub enum RateLimitAlgorithm {
    TokenBucket,
    SlidingWindow,
    FixedWindow,
}

#[pymethods]
impl RateLimitAlgorithm {
    #[classattr]
    #[allow(non_snake_case)]
    fn TOKEN_BUCKET() -> RateLimitAlgorithm { RateLimitAlgorithm::TokenBucket }
    
    #[classattr]
    #[allow(non_snake_case)]
    fn SLIDING_WINDOW() -> RateLimitAlgorithm { RateLimitAlgorithm::SlidingWindow }
    
    #[classattr]
    #[allow(non_snake_case)]
    fn FIXED_WINDOW() -> RateLimitAlgorithm { RateLimitAlgorithm::FixedWindow }
}

/// Rate limiting configuration for HTTP requests
#[pyclass]
#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    #[pyo3(get, set)]
    pub enabled: bool,
    
    #[pyo3(get, set)]
    pub algorithm: RateLimitAlgorithm,
    
    #[pyo3(get, set)]
    pub requests_per_second: f64,
    
    #[pyo3(get, set)]
    pub requests_per_minute: Option<u32>,
    
    #[pyo3(get, set)]
    pub requests_per_hour: Option<u32>,
    
    #[pyo3(get, set)]
    pub burst_size: Option<u32>,
    
    #[pyo3(get, set)]
    pub window_size_seconds: f64,
    
    #[pyo3(get, set)]
    pub per_host: bool,
    
    #[pyo3(get, set)]
    pub reset_on_success: bool,
    
    #[pyo3(get, set)]
    pub queue_requests: bool,
    
    #[pyo3(get, set)]
    pub max_queue_size: usize,
    
    #[pyo3(get, set)]
    pub queue_timeout_seconds: f64,
}

#[pymethods]
impl RateLimitConfig {
    #[new]
    #[pyo3(signature = (
        enabled = true,
        algorithm = RateLimitAlgorithm::TokenBucket,
        requests_per_second = 10.0,
        requests_per_minute = None,
        requests_per_hour = None,
        burst_size = None,
        window_size_seconds = 1.0,
        per_host = true,
        reset_on_success = false,
        queue_requests = true,
        max_queue_size = 100,
        queue_timeout_seconds = 30.0
    ))]
    pub fn new(
        enabled: bool,
        algorithm: RateLimitAlgorithm,
        requests_per_second: f64,
        requests_per_minute: Option<u32>,
        requests_per_hour: Option<u32>,
        burst_size: Option<u32>,
        window_size_seconds: f64,
        per_host: bool,
        reset_on_success: bool,
        queue_requests: bool,
        max_queue_size: usize,
        queue_timeout_seconds: f64,
    ) -> Self {
        RateLimitConfig {
            enabled,
            algorithm,
            requests_per_second,
            requests_per_minute,
            requests_per_hour,
            burst_size,
            window_size_seconds,
            per_host,
            reset_on_success,
            queue_requests,
            max_queue_size,
            queue_timeout_seconds,
        }
    }
    
    /// Create a conservative rate limiting configuration
    #[staticmethod]
    pub fn conservative() -> Self {
        RateLimitConfig {
            enabled: true,
            algorithm: RateLimitAlgorithm::TokenBucket,
            requests_per_second: 5.0,
            requests_per_minute: Some(100),
            requests_per_hour: Some(1000),
            burst_size: Some(10),
            window_size_seconds: 1.0,
            per_host: true,
            reset_on_success: false,
            queue_requests: true,
            max_queue_size: 50,
            queue_timeout_seconds: 30.0,
        }
    }
    
    /// Create a moderate rate limiting configuration
    #[staticmethod]
    pub fn moderate() -> Self {
        RateLimitConfig {
            enabled: true,
            algorithm: RateLimitAlgorithm::TokenBucket,
            requests_per_second: 25.0,
            requests_per_minute: Some(500),
            requests_per_hour: Some(10000),
            burst_size: Some(50),
            window_size_seconds: 1.0,
            per_host: true,
            reset_on_success: false,
            queue_requests: true,
            max_queue_size: 100,
            queue_timeout_seconds: 30.0,
        }
    }
    
    /// Create an aggressive rate limiting configuration for high-throughput
    #[staticmethod]
    pub fn aggressive() -> Self {
        RateLimitConfig {
            enabled: true,
            algorithm: RateLimitAlgorithm::SlidingWindow,
            requests_per_second: 100.0,
            requests_per_minute: Some(2000),
            requests_per_hour: Some(50000),
            burst_size: Some(200),
            window_size_seconds: 1.0,
            per_host: false,
            reset_on_success: false,
            queue_requests: true,
            max_queue_size: 500,
            queue_timeout_seconds: 60.0,
        }
    }
    
    /// Create a disabled rate limiting configuration
    #[staticmethod]
    pub fn disabled() -> Self {
        RateLimitConfig {
            enabled: false,
            algorithm: RateLimitAlgorithm::TokenBucket,
            requests_per_second: 0.0,
            requests_per_minute: None,
            requests_per_hour: None,
            burst_size: None,
            window_size_seconds: 1.0,
            per_host: false,
            reset_on_success: false,
            queue_requests: false,
            max_queue_size: 0,
            queue_timeout_seconds: 0.0,
        }
    }

    /// Validate rate limiting configuration
    pub fn validate(&self) -> Result<(), UltraFastError> {
        if self.enabled {
            if self.requests_per_second <= 0.0 {
                return Err(UltraFastError::ConfigError("requests_per_second must be positive when rate limiting is enabled".to_string()));
            }
            if self.window_size_seconds <= 0.0 {
                return Err(UltraFastError::ConfigError("window_size_seconds must be positive".to_string()));
            }
            if self.queue_timeout_seconds < 0.0 {
                return Err(UltraFastError::ConfigError("queue_timeout_seconds cannot be negative".to_string()));
            }
            if self.queue_requests && self.max_queue_size == 0 {
                return Err(UltraFastError::ConfigError("max_queue_size must be positive when queue_requests is enabled".to_string()));
            }
        }
        Ok(())
    }
}
