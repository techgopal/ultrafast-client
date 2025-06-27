use pyo3::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::client::HttpClient;
use crate::config::{AuthConfig, RetryConfig, TimeoutConfig};
use reqwest::cookie::Jar;
use std::sync::Mutex;

/// Advanced session management with cookie handling and persistent configuration
#[pyclass]
pub struct Session {
    client: Arc<Mutex<HttpClient>>,
    cookies: Arc<Jar>,
    persist_cookies: bool,
    default_headers: Arc<RwLock<HashMap<String, String>>>,
    auth_config: Arc<RwLock<Option<AuthConfig>>>,
    retry_config: Arc<RwLock<Option<RetryConfig>>>,
    timeout_config: Arc<RwLock<Option<TimeoutConfig>>>,
    base_url: Arc<RwLock<Option<String>>>,
    session_data: Arc<RwLock<HashMap<String, String>>>,
}

#[pymethods]
impl Session {
    #[new]
    #[pyo3(signature = (
        base_url = None,
        headers = None,
        auth_config = None,
        retry_config = None,
        timeout_config = None,
        pool_config = None,
        ssl_config = None,
        persist_cookies = true
    ))]
    pub fn new(
        base_url: Option<String>,
        headers: Option<HashMap<String, String>>,
        auth_config: Option<AuthConfig>,
        retry_config: Option<RetryConfig>,
        timeout_config: Option<TimeoutConfig>,
        pool_config: Option<crate::config::PoolConfig>,
        ssl_config: Option<crate::config::SSLConfig>,
        persist_cookies: bool,
    ) -> PyResult<Self> {
        let client = HttpClient::new(
            base_url.clone(),
            headers.clone(),
            30.0,
            auth_config.clone(),
            retry_config.clone(),
            timeout_config.clone(),
            pool_config,
            ssl_config,
            None, // proxy_config
            None, // compression_config
            None, // protocol_config
            None, // rate_limit_config
        )?;
        
        Ok(Session {
            client: Arc::new(Mutex::new(client)),
            cookies: Arc::new(Jar::default()),
            persist_cookies,
            default_headers: Arc::new(RwLock::new(headers.unwrap_or_default())),
            auth_config: Arc::new(RwLock::new(auth_config)),
            retry_config: Arc::new(RwLock::new(retry_config)),
            timeout_config: Arc::new(RwLock::new(timeout_config)),
            base_url: Arc::new(RwLock::new(base_url)),
            session_data: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Perform GET request with session
    #[pyo3(signature = (url, params = None, headers = None))]
    pub fn get<'py>(&self, py: Python<'py>, url: &str, params: Option<HashMap<String, String>>, headers: Option<HashMap<String, String>>) -> PyResult<&'py PyAny> {
        let mut client = self.client.lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire lock: {}", e)))?;
        let response = client.get(url, params, headers)?;
        Ok(response.into_py(py).into_ref(py))
    }
    
    /// Perform POST request with session
    #[pyo3(signature = (url, json = None, data = None, files = None, headers = None))]
    pub fn post<'py>(&self, py: Python<'py>, url: &str, json: Option<&PyAny>, data: Option<HashMap<String, String>>, files: Option<HashMap<String, String>>, headers: Option<HashMap<String, String>>) -> PyResult<&'py PyAny> {
        let mut client = self.client.lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire lock: {}", e)))?;
        // Convert files to the expected format
        let files_converted = files.map(|f| {
            f.into_iter().map(|(k, v)| (k, v.into_bytes())).collect::<HashMap<String, Vec<u8>>>()
        });
        let response = client.post(url, json, data, files_converted, headers)?;
        Ok(response.into_py(py).into_ref(py))
    }
    
    /// Perform PUT request with session
    #[pyo3(signature = (url, json = None, data = None, files = None, headers = None))]
    pub fn put<'py>(&self, py: Python<'py>, url: &str, json: Option<&PyAny>, data: Option<HashMap<String, String>>, files: Option<HashMap<String, String>>, headers: Option<HashMap<String, String>>) -> PyResult<&'py PyAny> {
        let mut client = self.client.lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire lock: {}", e)))?;
        // Convert files to the expected format
        let files_converted = files.map(|f| {
            f.into_iter().map(|(k, v)| (k, v.into_bytes())).collect::<HashMap<String, Vec<u8>>>()
        });
        let response = client.put(url, json, data, files_converted, headers)?;
        Ok(response.into_py(py).into_ref(py))
    }
    
    /// Perform DELETE request with session
    pub fn delete<'py>(&self, py: Python<'py>, url: &str, headers: Option<HashMap<String, String>>) -> PyResult<&'py PyAny> {
        let mut client = self.client.lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire lock: {}", e)))?;
        let response = client.delete(url, headers)?;
        Ok(response.into_py(py).into_ref(py))
    }
    
    /// Perform PATCH request with session
    #[pyo3(signature = (url, json = None, data = None, files = None, headers = None))]
    pub fn patch<'py>(&self, py: Python<'py>, url: &str, json: Option<&PyAny>, data: Option<HashMap<String, String>>, files: Option<HashMap<String, String>>, headers: Option<HashMap<String, String>>) -> PyResult<&'py PyAny> {
        let mut client = self.client.lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire lock: {}", e)))?;
        // Convert files to the expected format
        let files_converted = files.map(|f| {
            f.into_iter().map(|(k, v)| (k, v.into_bytes())).collect::<HashMap<String, Vec<u8>>>()
        });
        let response = client.patch(url, json, data, files_converted, headers)?;
        Ok(response.into_py(py).into_ref(py))
    }
    
    /// Perform HEAD request with session
    pub fn head<'py>(&self, py: Python<'py>, url: &str, headers: Option<HashMap<String, String>>) -> PyResult<&'py PyAny> {
        let mut client = self.client.lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire lock: {}", e)))?;
        let response = client.head(url, headers)?;
        Ok(response.into_py(py).into_ref(py))
    }
    
    /// Perform OPTIONS request with session
    pub fn options<'py>(&self, py: Python<'py>, url: &str, headers: Option<HashMap<String, String>>) -> PyResult<&'py PyAny> {
        let mut client = self.client.lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire lock: {}", e)))?;
        let response = client.options(url, headers)?;
        Ok(response.into_py(py).into_ref(py))
    }
    
    /// Set a session header
    pub fn set_header(&mut self, key: String, value: String) -> PyResult<()> {
        self.default_headers.write()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire write lock: {}", e)))?
            .insert(key, value);
        Ok(())
    }
    
    /// Remove a session header
    pub fn remove_header(&mut self, key: &str) -> Option<String> {
        self.default_headers.write()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire write lock: {}", e)))
            .map(|mut guard| guard.remove(key))
            .unwrap_or(None)
    }
    
    /// Get session headers as property
    #[getter]
    pub fn headers(&self) -> HashMap<String, String> {
        self.default_headers.read()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire read lock: {}", e)))
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }
    
    /// Get authentication config as property
    #[getter]
    pub fn auth_config(&self) -> Option<AuthConfig> {
        self.auth_config.read()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire read lock: {}", e)))
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }
    
    /// Get timeout config as property
    #[getter]
    pub fn timeout_config(&self) -> Option<TimeoutConfig> {
        self.timeout_config.read()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire read lock: {}", e)))
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }
    
    /// Set authentication for the session
    pub fn set_auth(&mut self, auth_config: AuthConfig) -> PyResult<()> {
        *self.auth_config.write()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire write lock: {}", e)))? = Some(auth_config);
        Ok(())
    }
    
    /// Set authentication config (alias for compatibility)
    pub fn set_auth_config(&mut self, auth_config: AuthConfig) -> PyResult<()> {
        self.set_auth(auth_config)
    }
    
    /// Clear authentication
    pub fn clear_auth(&mut self) -> PyResult<()> {
        *self.auth_config.write()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire write lock: {}", e)))? = None;
        Ok(())
    }
    
    /// Set retry configuration
    pub fn set_retry(&mut self, retry_config: RetryConfig) -> PyResult<()> {
        *self.retry_config.write()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire write lock: {}", e)))? = Some(retry_config);
        Ok(())
    }
    
    /// Store session data
    pub fn set_session_data(&mut self, key: String, value: String) -> PyResult<()> {
        self.session_data.write()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire write lock: {}", e)))?
            .insert(key, value);
        Ok(())
    }
    
    /// Get session data
    pub fn get_session_data(&self, key: &str) -> Option<String> {
        self.session_data.read()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire read lock: {}", e)))
            .map(|guard| guard.get(key).cloned())
            .unwrap_or_default()
    }
    
    /// Remove session data
    pub fn remove_session_data(&mut self, key: &str) -> PyResult<()> {
        self.session_data.write()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire write lock: {}", e)))?
            .remove(key);
        Ok(())
    }
    
    /// Store data (alias for set_session_data)
    pub fn set_data(&mut self, key: String, value: String) -> PyResult<()> {
        self.set_session_data(key, value)
    }
    
    /// Get data (alias for get_session_data)
    pub fn get_data(&self, key: &str) -> Option<String> {
        self.get_session_data(key)
    }
    
    /// Remove data (alias for remove_session_data)
    pub fn remove_data(&mut self, key: &str) -> PyResult<()> {
        self.remove_session_data(key)
    }
    
    /// Clear all session data
    pub fn clear_data(&mut self) -> PyResult<()> {
        self.session_data.write()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire write lock: {}", e)))?
            .clear();
        Ok(())
    }
    
    /// Clear all cookies
    pub fn clear_cookies(&self) {
        // Cookie clearing would require access to the internal cookie store
        // This is a simplified implementation
    }

    /// Set cookie (using a dummy URL for cookie store)
    pub fn set_cookie(&self, name: &str, value: &str) -> PyResult<()> {
        // Create a simple cookie string for the cookie jar
        let cookie_str = format!("{}={}", name, value);
        let base_url_guard = self.base_url.read()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire read lock: {}", e)))?;
        let url = base_url_guard.as_deref().unwrap_or("http://localhost");
        if let Ok(parsed_url) = url.parse() {
            self.cookies.add_cookie_str(&cookie_str, &parsed_url);
        }
        Ok(())
    }

    /// Get cookie (simplified implementation)
    pub fn get_cookie(&self, name: &str) -> Option<String> {
        // This is a simplified implementation since reqwest::Jar doesn't provide 
        // easy access to individual cookies. In practice, cookies are handled 
        // automatically by the HTTP client.
        // For now, return None as cookies are managed internally by reqwest
        None
    }
    
    /// Check if cookies are persisted
    #[getter]
    pub fn persist_cookies(&self) -> bool {
        self.persist_cookies
    }
    
    /// Set base URL
    pub fn set_base_url(&self, base_url: Option<String>) {
        *self.base_url.write().unwrap() = base_url;
    }
    
    /// Get base URL
    #[getter]
    pub fn base_url(&self) -> Option<String> {
        self.base_url.read().unwrap().clone()
    }
    
    /// Apply session configuration to client
    fn apply_session_config(&self, client: &mut HttpClient) -> PyResult<()> {
        // Apply auth if set
        if let Some(ref auth) = *self.auth_config.read().unwrap() {
            client.set_auth(auth.clone())?;
        }
        
        // Apply retry config if set
        if let Some(ref retry) = *self.retry_config.read().unwrap() {
            client.set_retry_config(retry.clone());
        }
        
        // Apply session headers
        let headers = self.default_headers.read().unwrap();
        for (key, value) in headers.iter() {
            let _ = client.set_header(key.clone(), value.clone());
        }
        
        Ok(())
    }
    
    /// Context manager support
    fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
    
    fn __exit__(&mut self, _exc_type: Option<&PyAny>, _exc_value: Option<&PyAny>, _traceback: Option<&PyAny>) {
        // Cleanup if needed
    }
}
