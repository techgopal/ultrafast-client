use crate::async_client::AsyncHttpClient;
use crate::config::{AuthConfig, RetryConfig, TimeoutConfig};
use pyo3::prelude::*;
use pyo3::types::PyAny;
use reqwest::cookie::Jar;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Async Session for managing HTTP requests with shared state
#[pyclass]
pub struct AsyncSession {
    client: Arc<Mutex<AsyncHttpClient>>,
    session_headers: HashMap<String, String>,
    cookies: Arc<Jar>,
    base_url: Option<String>,
    auth_config: Option<AuthConfig>,
    retry_config: Option<RetryConfig>,
    timeout_config: Option<TimeoutConfig>,
    session_data: HashMap<String, String>,
    persist_cookies: bool,
}

#[pymethods]
impl AsyncSession {
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
        let client = AsyncHttpClient::new(
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
        Ok(AsyncSession {
            client: Arc::new(Mutex::new(client)),
            session_headers: headers.clone().unwrap_or_default(),
            cookies: Arc::new(Jar::default()),
            base_url,
            auth_config,
            retry_config,
            timeout_config,
            session_data: HashMap::new(),
            persist_cookies,
        })
    }

    /// GET request with session state
    #[pyo3(signature = (url, params = None, headers = None))]
    pub fn get<'py>(
        &self,
        py: Python<'py>,
        url: &str,
        params: Option<HashMap<String, String>>,
        headers: Option<HashMap<String, String>>,
    ) -> PyResult<&'py PyAny> {
        let client_guard = self.client.lock().unwrap();
        let client_py = Py::new(py, client_guard.clone())?;
        drop(client_guard);
        let mut merged_headers = self.session_headers.clone();
        if let Some(request_headers) = headers {
            merged_headers.extend(request_headers);
        }
        AsyncHttpClient::get(client_py, py, url, params, Some(merged_headers))
    }

    /// POST request with session state
    #[pyo3(signature = (url, json = None, data = None, files = None, headers = None))]
    pub fn post<'py>(
        &self,
        py: Python<'py>,
        url: &str,
        json: Option<&PyAny>,
        data: Option<HashMap<String, String>>,
        files: Option<HashMap<String, Vec<u8>>>,
        headers: Option<HashMap<String, String>>,
    ) -> PyResult<&'py PyAny> {
        let client_guard = self.client.lock().unwrap();
        let client_py = Py::new(py, client_guard.clone())?;
        drop(client_guard);
        let mut merged_headers = self.session_headers.clone();
        if let Some(request_headers) = headers {
            merged_headers.extend(request_headers);
        }
        AsyncHttpClient::post(client_py, py, url, json, data, files, Some(merged_headers))
    }

    /// PUT request with session state
    #[pyo3(signature = (url, json = None, data = None, headers = None, files = None))]
    pub fn put<'py>(
        &self,
        py: Python<'py>,
        url: &str,
        json: Option<&PyAny>,
        data: Option<HashMap<String, String>>,
        headers: Option<HashMap<String, String>>,
        files: Option<HashMap<String, Vec<u8>>>,
    ) -> PyResult<&'py PyAny> {
        let client_guard = self.client.lock().unwrap();
        let client_py = Py::new(py, client_guard.clone())?;
        drop(client_guard);
        let mut merged_headers = self.session_headers.clone();
        if let Some(request_headers) = headers {
            merged_headers.extend(request_headers);
        }
        AsyncHttpClient::put(client_py, py, url, json, data, files, Some(merged_headers))
    }

    /// DELETE request with session state
    #[pyo3(signature = (url, headers = None))]
    pub fn delete<'py>(
        &self,
        py: Python<'py>,
        url: &str,
        headers: Option<HashMap<String, String>>,
    ) -> PyResult<&'py PyAny> {
        let client_guard = self.client.lock().unwrap();
        let client_py = Py::new(py, client_guard.clone())?;
        drop(client_guard);
        let mut merged_headers = self.session_headers.clone();
        if let Some(request_headers) = headers {
            merged_headers.extend(request_headers);
        }
        AsyncHttpClient::delete(client_py, py, url, Some(merged_headers))
    }

    /// PATCH request with session state
    #[pyo3(signature = (url, json = None, data = None, files = None, headers = None))]
    pub fn patch<'py>(
        &self,
        py: Python<'py>,
        url: &str,
        json: Option<&PyAny>,
        data: Option<HashMap<String, String>>,
        files: Option<HashMap<String, Vec<u8>>>,
        headers: Option<HashMap<String, String>>,
    ) -> PyResult<&'py PyAny> {
        let client_guard = self.client.lock().unwrap();
        let client_py = Py::new(py, client_guard.clone())?;
        drop(client_guard);
        let mut merged_headers = self.session_headers.clone();
        if let Some(request_headers) = headers {
            merged_headers.extend(request_headers);
        }
        AsyncHttpClient::patch(client_py, py, url, json, data, files, Some(merged_headers))
    }

    /// HEAD request with session state
    #[pyo3(signature = (url, headers = None))]
    pub fn head<'py>(
        &self,
        py: Python<'py>,
        url: &str,
        headers: Option<HashMap<String, String>>,
    ) -> PyResult<&'py PyAny> {
        let client_guard = self.client.lock().unwrap();
        let client_py = Py::new(py, client_guard.clone())?;
        drop(client_guard);
        let mut merged_headers = self.session_headers.clone();
        if let Some(request_headers) = headers {
            merged_headers.extend(request_headers);
        }
        AsyncHttpClient::head(client_py, py, url, Some(merged_headers))
    }

    /// OPTIONS request with session state
    #[pyo3(signature = (url, headers = None))]
    pub fn options<'py>(
        &self,
        py: Python<'py>,
        url: &str,
        headers: Option<HashMap<String, String>>,
    ) -> PyResult<&'py PyAny> {
        let client_guard = self.client.lock().unwrap();
        let client_py = Py::new(py, client_guard.clone())?;
        drop(client_guard);
        let mut merged_headers = self.session_headers.clone();
        if let Some(request_headers) = headers {
            merged_headers.extend(request_headers);
        }
        AsyncHttpClient::options(client_py, py, url, Some(merged_headers))
    }

    /// Get the session's base URL
    #[getter]
    pub fn base_url(&self) -> Option<String> {
        self.base_url.clone()
    }
    /// Set the session's base URL
    pub fn set_base_url(&mut self, base_url: Option<String>) {
        self.base_url = base_url.clone();
        if let Ok(mut client) = self.client.lock() {
            client.set_base_url(base_url);
        }
    }
    /// Get the session's authentication config
    #[getter]
    pub fn auth_config(&self) -> Option<AuthConfig> {
        self.auth_config.clone()
    }
    /// Set the session's authentication config
    pub fn set_auth(&mut self, auth_config: AuthConfig) -> PyResult<()> {
        self.auth_config = Some(auth_config.clone());
        if let Ok(mut client) = self.client.lock() {
            client.set_auth(auth_config)?;
        }
        Ok(())
    }
    /// Clear authentication
    pub fn clear_auth(&mut self) -> PyResult<()> {
        self.auth_config = None;
        if let Ok(mut client) = self.client.lock() {
            client.clear_auth();
        }
        Ok(())
    }
    /// Get the session's retry config
    #[getter]
    pub fn retry_config(&self) -> Option<RetryConfig> {
        self.retry_config.clone()
    }
    /// Set the session's retry config
    pub fn set_retry(&mut self, retry_config: RetryConfig) -> PyResult<()> {
        self.retry_config = Some(retry_config.clone());
        if let Ok(mut client) = self.client.lock() {
            client.set_retry_config(retry_config);
        }
        Ok(())
    }

    /// Set the session's retry config (alias for compatibility)
    pub fn set_retry_config(&mut self, retry_config: RetryConfig) {
        let _ = self.set_retry(retry_config);
    }
    /// Store session data
    pub fn set_session_data(&mut self, key: String, value: String) {
        self.session_data.insert(key, value);
    }
    /// Get session data
    pub fn get_session_data(&self, key: &str) -> Option<String> {
        self.session_data.get(key).cloned()
    }
    /// Remove session data
    pub fn remove_session_data(&mut self, key: &str) -> Option<String> {
        self.session_data.remove(key)
    }

    /// Store session data (alias for compatibility)
    pub fn set_data(&mut self, key: String, value: String) {
        self.set_session_data(key, value);
    }
    /// Get session data (alias for compatibility)
    pub fn get_data(&self, key: &str) -> Option<String> {
        self.get_session_data(key)
    }
    /// Remove session data (alias for compatibility)
    pub fn remove_data(&mut self, key: &str) -> Option<String> {
        self.remove_session_data(key)
    }
    /// Clear all session data
    pub fn clear_data(&mut self) {
        self.session_data.clear();
    }
    /// Expose session headers as a property
    #[getter]
    pub fn headers(&self) -> HashMap<String, String> {
        self.session_headers.clone()
    }

    /// Clear all cookies
    pub fn clear_cookies(&mut self) {
        // Cookie clearing would require access to the internal cookie store
        // This is a simplified implementation since reqwest::Jar doesn't have a public clear method
        self.cookies = Arc::new(Jar::default());
    }
    /// Check if cookies are persisted
    #[getter]
    pub fn persist_cookies(&self) -> bool {
        self.persist_cookies
    }

    /// Set session header
    pub fn set_header(&mut self, key: String, value: String) {
        self.session_headers.insert(key, value);
    }

    /// Remove session header
    pub fn remove_header(&mut self, key: &str) -> Option<String> {
        self.session_headers.remove(key)
    }

    /// Set cookie (using a dummy URL for cookie store)
    pub fn set_cookie(&mut self, name: &str, value: &str) {
        // Create a simple cookie string for the cookie jar
        let cookie_str = format!("{}={}", name, value);
        let url = self.base_url.as_deref().unwrap_or("http://localhost");
        if let Ok(parsed_url) = url.parse() {
            self.cookies.add_cookie_str(&cookie_str, &parsed_url);
        }
    }

    /// Get cookie (simplified implementation)
    pub fn get_cookie(&self, name: &str) -> Option<String> {
        // This is a simplified implementation since reqwest::Jar doesn't provide
        // easy access to individual cookies. In practice, cookies are handled
        // automatically by the HTTP client.
        // For now, return None as cookies are managed internally by reqwest
        None
    }

    /// Close session (cleanup resources)
    pub fn close(&self) {
        // No special cleanup needed for now
    }

    /// Context manager entry - return Python object reference directly
    pub fn __aenter__(slf: PyRef<Self>) -> PyResult<PyRef<Self>> {
        Ok(slf)
    }

    /// Context manager exit - return a synchronous result
    pub fn __aexit__(
        &self,
        _exc_type: Option<&PyAny>,
        _exc_value: Option<&PyAny>,
        _traceback: Option<&PyAny>,
    ) -> PyResult<bool> {
        self.close();
        Ok(false) // Don't suppress exceptions
    }

    /// Apply session configuration to client
    fn apply_session_config(&self) -> PyResult<()> {
        if let Ok(mut client) = self.client.lock() {
            // Apply auth if set
            if let Some(ref auth) = self.auth_config {
                client.set_auth(auth.clone())?;
            }

            // Apply retry config if set
            if let Some(ref retry) = self.retry_config {
                client.set_retry_config(retry.clone());
            }

            // Apply session headers
            for (key, value) in self.session_headers.iter() {
                client.set_header(key.clone(), value.clone());
            }
        }
        Ok(())
    }
}

/// Helper function to merge session headers with request headers
fn merge_headers(
    session_headers: &HashMap<String, String>,
    request_headers: Option<HashMap<String, String>>,
) -> HashMap<String, String> {
    let mut merged = session_headers.clone();
    if let Some(req_headers) = request_headers {
        merged.extend(req_headers);
    }
    merged
}
