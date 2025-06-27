use pyo3::prelude::*;
use pyo3::PyObject;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

use bytes::Bytes;
use futures_util::StreamExt;
use pyo3_asyncio::tokio::future_into_py;

/// Server-Sent Events client for real-time event streaming
#[pyclass]
pub struct SSEClient {
    url: Option<String>,
    client: Client,
    headers: HashMap<String, String>,
    reconnect_timeout: f64,
    max_reconnect_attempts: u32,
    connected: Arc<Mutex<bool>>,
    event_receiver: Arc<Mutex<Option<Receiver<Result<Bytes, String>>>>>,
    _connection_handle: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
}

#[pymethods]
impl SSEClient {
    #[new]
    #[pyo3(signature = (
        reconnect_timeout = 5.0,
        max_reconnect_attempts = 10,
        headers = None
    ))]
    pub fn new(
        reconnect_timeout: f64,
        max_reconnect_attempts: u32,
        headers: Option<HashMap<String, String>>,
    ) -> PyResult<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(60)) // 60 second request timeout
            .connect_timeout(std::time::Duration::from_secs(10)) // 10 second connect timeout
            .pool_max_idle_per_host(1) // Limit connection pooling
            .http1_title_case_headers() // Use HTTP/1.1 explicitly
            .http2_keep_alive_interval(None) // Disable HTTP/2 keep-alive
            .build()
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "Failed to create HTTP client: {}",
                    e
                ))
            })?;

        Ok(SSEClient {
            url: None,
            client,
            headers: headers.unwrap_or_default(),
            reconnect_timeout,
            max_reconnect_attempts,
            connected: Arc::new(Mutex::new(false)),
            event_receiver: Arc::new(Mutex::new(None)),
            _connection_handle: Arc::new(Mutex::new(None)),
        })
    }

    /// Connect to an SSE endpoint
    pub fn connect(&mut self, url: &str) -> PyResult<()> {
        // Close any existing connection
        self.close();

        self.url = Some(url.to_string());

        let client = self.client.clone();
        let url = url.to_string();
        let headers = self.headers.clone();
        let connected = self.connected.clone();
        let event_receiver_arc = self.event_receiver.clone();

        // Create a channel for streaming events
        let (tx, rx) = mpsc::channel::<Result<Bytes, String>>();

        // Create a channel to signal when connection is established
        let (conn_tx, conn_rx) = mpsc::channel::<Result<(), String>>();

        // Store the receiver
        *event_receiver_arc.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire lock: {}", e))
        })? = Some(rx);

        // Spawn a background thread to handle the connection
        let handle = thread::spawn(move || {
            // SSE background thread started

            // Create a new runtime for this thread
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => {
                    // SSE Tokio runtime created
                    rt
                }
                Err(e) => {
                    // Failed to create SSE runtime
                    let _ = conn_tx.send(Err(format!("Failed to create runtime: {}", e)));
                    return;
                }
            };

            rt.block_on(async move {
                // Starting SSE HTTP request
                let mut request = client.get(&url);

                // Add SSE-specific headers
                request = request.header("Accept", "text/event-stream");
                request = request.header("Cache-Control", "no-cache");

                // Add custom headers
                for (key, value) in headers {
                    request = request.header(&key, &value);
                }

                // Sending SSE HTTP request
                match request.send().await {
                    Ok(mut response) => {
                        if response.status().is_success() {
                            // SSE HTTP connection successful
                            if let Ok(mut connected_guard) = connected.lock() {
                                *connected_guard = true;
                            }

                            // Signal that connection is established
                            let _ = conn_tx.send(Ok(()));

                            // Stream response chunks
                            while let Ok(Some(chunk)) = response.chunk().await {
                                eprintln!("SSE: Received chunk of {} bytes", chunk.len());
                                if tx.send(Ok(chunk)).is_err() {
                                    // Receiver has been dropped, stop streaming
                                    // SSE debug statement removed
                                    break;
                                }
                            }

                            // Connection ended
                            // SSE debug statement removed
                            if let Ok(mut connected_guard) = connected.lock() {
                                *connected_guard = false;
                            }
                            let _ = tx.send(Err("Connection ended".to_string()));
                        } else {
                            eprintln!("SSE: HTTP error: {}", response.status());
                            if let Ok(mut connected_guard) = connected.lock() {
                                *connected_guard = false;
                            }
                            let _ = conn_tx.send(Err(format!("HTTP error: {}", response.status())));
                        }
                    }
                    Err(e) => {
                        eprintln!("SSE: Connection error: {}", e);
                        if let Ok(mut connected_guard) = connected.lock() {
                            *connected_guard = false;
                        }
                        let _ = conn_tx.send(Err(format!("Connection error: {}", e)));
                    }
                }
            });
        });

        // Store the connection handle
        if let Ok(mut handle_guard) = self._connection_handle.lock() {
            *handle_guard = Some(handle);
        }

        // Wait for connection establishment signal with timeout
        // SSE debug statement removed

        match conn_rx.recv_timeout(std::time::Duration::from_secs(30)) {
            Ok(Ok(())) => {
                // SSE debug statement removed
                Ok(())
            }
            Ok(Err(error_msg)) => {
                eprintln!("SSE: Connection failed: {}", error_msg);
                Err(pyo3::exceptions::PyConnectionError::new_err(error_msg))
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // SSE debug statement removed
                Err(pyo3::exceptions::PyConnectionError::new_err(
                    "Connection timeout - failed to connect within 30 seconds",
                ))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // SSE debug statement removed
                Err(pyo3::exceptions::PyConnectionError::new_err(
                    "Connection channel disconnected",
                ))
            }
        }
    }

    /// Listen for events (returns an iterator)
    pub fn listen(&self) -> PyResult<SSEEventIterator> {
        if let Ok(connected_guard) = self.connected.lock() {
            if !*connected_guard {
                return Err(pyo3::exceptions::PyConnectionError::new_err(
                    "Not connected to SSE endpoint",
                ));
            }
        }

        SSEEventIterator::new(self.event_receiver.clone())
    }

    /// Set a header for the SSE connection
    pub fn set_header(&mut self, key: String, value: String) {
        self.headers.insert(key, value);
    }

    /// Remove a header
    pub fn remove_header(&mut self, key: &str) -> Option<String> {
        self.headers.remove(key)
    }

    /// Get current headers
    #[getter]
    pub fn headers(&self) -> HashMap<String, String> {
        self.headers.clone()
    }

    /// Close the SSE connection
    pub fn close(&mut self) {
        // Update connection status
        if let Ok(mut connected_guard) = self.connected.lock() {
            *connected_guard = false;
        }
        if let Ok(mut receiver_guard) = self.event_receiver.lock() {
            *receiver_guard = None;
        }

        // The background thread will naturally terminate when the receiver is dropped
    }

    /// Check if connected
    #[getter]
    pub fn is_connected(&self) -> bool {
        if let Ok(connected_guard) = self.connected.lock() {
            *connected_guard
        } else {
            false
        }
    }

    /// Get the connection URL
    #[getter]
    pub fn url(&self) -> Option<String> {
        self.url.clone()
    }

    /// Context manager entry
    fn __enter__(slf: PyRefMut<Self>) -> PyResult<PyRefMut<Self>> {
        Ok(slf)
    }

    /// Context manager exit
    fn __exit__(
        &mut self,
        _exc_type: Option<&PyAny>,
        _exc_value: Option<&PyAny>,
        _traceback: Option<&PyAny>,
    ) {
        self.close();
    }
}

/// SSE Event representation
#[pyclass]
#[derive(Clone, Debug)]
pub struct SSEEvent {
    #[pyo3(get)]
    pub event_type: Option<String>,
    #[pyo3(get)]
    pub data: Option<String>,
    #[pyo3(get)]
    pub id: Option<String>,
    #[pyo3(get)]
    pub retry: Option<u32>,
    #[pyo3(get)]
    pub timestamp: f64,
}

#[pymethods]
impl SSEEvent {
    #[new]
    #[pyo3(signature = (event_type, data, id=None, retry=None))]
    pub fn new(
        event_type: Option<String>,
        data: String,
        id: Option<String>,
        retry: Option<u32>,
    ) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();

        SSEEvent {
            event_type,
            data: Some(data),
            id,
            retry,
            timestamp,
        }
    }

    /// Parse data as JSON
    pub fn json(&self, py: Python) -> PyResult<PyObject> {
        match &self.data {
            Some(data) => {
                let value: serde_json::Value = serde_json::from_str(data).map_err(|e| {
                    pyo3::exceptions::PyValueError::new_err(format!("Invalid JSON: {}", e))
                })?;
                crate::client::json_to_python(py, &value)
            }
            None => Ok(py.None()),
        }
    }

    /// Check if this is a keep-alive event
    pub fn is_keepalive(&self) -> bool {
        self.data
            .as_ref()
            .map(|d| d.trim().is_empty())
            .unwrap_or(true)
            && self.event_type.is_none()
    }

    /// Check if this is a retry event
    pub fn is_retry(&self) -> bool {
        self.retry.is_some()
    }

    fn __repr__(&self) -> String {
        format!(
            "SSEEvent(type={:?}, id={:?}, data_len={})",
            self.event_type,
            self.id,
            self.data.as_ref().map(|d| d.len()).unwrap_or(0)
        )
    }

    fn __str__(&self) -> String {
        format!(
            "SSE Event: {}",
            self.data.as_ref().unwrap_or(&"<empty>".to_string())
        )
    }
}

/// Iterator for SSE events
#[pyclass]
pub struct SSEEventIterator {
    event_receiver: Arc<Mutex<Option<Receiver<Result<Bytes, String>>>>>,
    buffer: String,
    current_event: HashMap<String, Vec<String>>,
}

impl SSEEventIterator {
    pub fn new(
        event_receiver: Arc<Mutex<Option<Receiver<Result<Bytes, String>>>>>,
    ) -> PyResult<Self> {
        Ok(SSEEventIterator {
            event_receiver,
            buffer: String::new(),
            current_event: HashMap::new(),
        })
    }
}
#[pymethods]
impl SSEEventIterator {
    /// Python iterator protocol
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    /// Python iterator next method
    fn __next__(&mut self) -> PyResult<Option<SSEEvent>> {
        let receiver_arc = self.event_receiver.clone();

        // First, check if we can process any existing buffered data
        if !self.buffer.is_empty() {
            if let Some(event) = self.try_parse_event_from_buffer()? {
                return Ok(Some(event));
            }
        }

        // If no buffered event is ready, try to receive more data
        loop {
            let chunk_result = {
                let receiver_guard = receiver_arc.lock().map_err(|e| {
                    pyo3::exceptions::PyRuntimeError::new_err(format!(
                        "Failed to acquire lock: {}",
                        e
                    ))
                })?;
                if let Some(receiver) = receiver_guard.as_ref() {
                    // Use a longer timeout for better reliability
                    match receiver.recv_timeout(std::time::Duration::from_secs(5)) {
                        Ok(result) => Some(result),
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            // Timeout - check if we have any partial data to process
                            if !self.buffer.is_empty() {
                                if let Some(event) = self.try_parse_event_from_buffer()? {
                                    return Ok(Some(event));
                                }
                            }
                            continue; // Keep waiting for more data
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => {
                            // Connection ended - process any remaining data
                            if !self.buffer.is_empty() {
                                if let Some(event) = self.try_parse_event_from_buffer()? {
                                    return Ok(Some(event));
                                }
                            }
                            return Ok(None);
                        }
                    }
                } else {
                    return Ok(None); // No receiver available
                }
            };

            match chunk_result {
                Some(Ok(chunk)) => {
                    // Convert chunk to string and add to buffer
                    let chunk_str = String::from_utf8_lossy(&chunk);
                    self.buffer.push_str(&chunk_str);

                    // Try to parse events from the updated buffer
                    if let Some(event) = self.try_parse_event_from_buffer()? {
                        return Ok(Some(event));
                    }
                }
                Some(Err(e)) => {
                    // Only treat this as an error if it's not a normal connection end
                    if e == "Connection ended" {
                        // Connection ended gracefully - process any remaining data
                        if !self.buffer.is_empty() {
                            if let Some(event) = self.try_parse_event_from_buffer()? {
                                return Ok(Some(event));
                            }
                        }
                        return Ok(None);
                    } else {
                        return Err(pyo3::exceptions::PyConnectionError::new_err(format!(
                            "Error reading SSE stream: {}",
                            e
                        )));
                    }
                }
                None => {
                    // Should not happen in this context
                    return Ok(None);
                }
            }
        }
    }

    /// Helper method to try parsing an event from the current buffer
    fn try_parse_event_from_buffer(&mut self) -> PyResult<Option<SSEEvent>> {
        // Process lines in buffer
        loop {
            if let Some(line_end) = self.buffer.find('\n') {
                let line = self.buffer[..line_end].trim_end_matches('\r').to_string();
                self.buffer.drain(..line_end + 1);

                if line.is_empty() {
                    // Empty line indicates end of event
                    if !self.current_event.is_empty() {
                        let event = build_sse_event(&self.current_event);
                        self.current_event.clear();
                        return Ok(Some(event));
                    }
                } else if let Some((field, value)) = parse_sse_line(&line) {
                    // Add field to current event
                    self.current_event
                        .entry(field)
                        .or_insert_with(Vec::new)
                        .push(value);
                }
            } else {
                // No complete line in buffer
                break;
            }
        }

        // No complete event found in buffer
        Ok(None)
    }
}

/// Parse SSE data from a text stream
pub fn parse_sse_line(line: &str) -> Option<(String, String)> {
    if line.is_empty() || line.starts_with(':') {
        return None; // Empty line or comment
    }

    if let Some(colon_pos) = line.find(':') {
        let field = line[..colon_pos].trim();
        let value = if line.len() > colon_pos + 1 && &line[colon_pos + 1..colon_pos + 2] == " " {
            &line[colon_pos + 2..]
        } else {
            &line[colon_pos + 1..]
        };
        Some((field.to_string(), value.to_string()))
    } else {
        Some((line.trim().to_string(), String::new()))
    }
}

/// Build an SSE event from parsed fields
pub fn build_sse_event(fields: &HashMap<String, Vec<String>>) -> SSEEvent {
    let event_type = fields.get("event").and_then(|v| v.last()).cloned();
    let data = fields.get("data").map(|v| v.join("\n")).unwrap_or_default();
    let id = fields.get("id").and_then(|v| v.last()).cloned();
    let retry = fields
        .get("retry")
        .and_then(|v| v.last())
        .and_then(|s| s.parse().ok());

    SSEEvent::new(event_type, data, id, retry)
}

/// Async SSE client for real-time event streaming over HTTP/1.1
#[pyclass]
pub struct AsyncSSEClient {
    url: Option<String>,
    client: Client,
    headers: HashMap<String, String>,
    reconnect_timeout: f64,
    max_reconnect_attempts: u32,
    connected: Arc<Mutex<bool>>,
    event_receiver: Arc<Mutex<Option<Receiver<Result<Bytes, String>>>>>,
}

#[pymethods]
impl AsyncSSEClient {
    #[new]
    #[pyo3(signature = (
        reconnect_timeout = 5.0,
        max_reconnect_attempts = 10,
        headers = None
    ))]
    pub fn new(
        reconnect_timeout: f64,
        max_reconnect_attempts: u32,
        headers: Option<HashMap<String, String>>,
    ) -> Self {
        let client = Client::new();

        AsyncSSEClient {
            url: None,
            client,
            headers: headers.unwrap_or_default(),
            reconnect_timeout,
            max_reconnect_attempts,
            connected: Arc::new(Mutex::new(false)),
            event_receiver: Arc::new(Mutex::new(None)),
        }
    }

    /// Connect to SSE endpoint
    pub fn connect<'py>(&mut self, py: Python<'py>, url: &str) -> PyResult<&'py PyAny> {
        self.url = Some(url.to_string());

        let client = self.client.clone();
        let url_clone = url.to_string();
        let headers = self.headers.clone();
        let connected = self.connected.clone();
        let event_receiver_arc = self.event_receiver.clone();

        future_into_py(py, async move {
            // Create a channel for streaming events
            let (tx, rx) = mpsc::channel::<Result<Bytes, String>>();

            // Store the receiver
            *event_receiver_arc.lock().map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire lock: {}", e))
            })? = Some(rx);

            let mut request = client.get(&url_clone);

            // Add SSE-specific headers
            request = request.header("Accept", "text/event-stream");
            request = request.header("Cache-Control", "no-cache");

            // Add custom headers
            for (key, value) in headers {
                request = request.header(&key, &value);
            }

            // Make the request and get streaming response
            let response = request.send().await.map_err(|e| {
                pyo3::exceptions::PyConnectionError::new_err(format!(
                    "SSE connection failed: {}",
                    e
                ))
            })?;

            if !response.status().is_success() {
                return Err(pyo3::exceptions::PyConnectionError::new_err(format!(
                    "SSE connection failed with status: {}",
                    response.status()
                )));
            }

            if let Ok(mut connected_guard) = connected.lock() {
                *connected_guard = true;
            }

            // Spawn a task to handle the SSE stream
            let url_clone = url_clone.clone();
            tokio::spawn(async move {
                let mut stream = response.bytes_stream();
                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            if tx.send(Ok(chunk)).is_err() {
                                eprintln!("SSE receiver disconnected for {}", url_clone);
                                break;
                            }
                        }
                        Err(e) => {
                            if tx.send(Err(e.to_string())).is_err() {
                                eprintln!("SSE receiver disconnected for {}", url_clone);
                            }
                            break;
                        }
                    }
                }
            });

            Ok(())
        })
    }

    /// Listen for events
    pub fn listen(&self, py: Python) -> PyResult<PyObject> {
        if let Ok(connected_guard) = self.connected.lock() {
            if !*connected_guard {
                return Err(pyo3::exceptions::PyConnectionError::new_err(
                    "Not connected to SSE endpoint",
                ));
            }
        }

        let iterator = SSEEventIterator::new(self.event_receiver.clone())?;
        Ok(iterator.into_py(py))
    }

    /// Set header
    pub fn set_header(&mut self, key: String, value: String) {
        self.headers.insert(key, value);
    }

    /// Remove a header
    pub fn remove_header(&mut self, key: &str) -> Option<String> {
        self.headers.remove(key)
    }

    /// Get current headers
    #[getter]
    pub fn headers(&self) -> HashMap<String, String> {
        self.headers.clone()
    }

    /// Close connection
    pub fn close<'py>(&mut self, py: Python<'py>) -> PyResult<&'py PyAny> {
        let connected = self.connected.clone();
        let event_receiver = self.event_receiver.clone();

        future_into_py(py, async move {
            if let Ok(mut connected_guard) = connected.lock() {
                *connected_guard = false;
            }
            if let Ok(mut receiver_guard) = event_receiver.lock() {
                *receiver_guard = None;
            }
            Ok(())
        })
    }

    /// Check if connected
    #[getter]
    pub fn is_connected(&self) -> bool {
        if let Ok(connected_guard) = self.connected.lock() {
            *connected_guard
        } else {
            false
        }
    }

    /// Get the connection URL
    #[getter]
    pub fn url(&self) -> Option<String> {
        self.url.clone()
    }

    /// Context manager entry
    fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    /// Context manager exit
    fn __exit__(
        &mut self,
        _exc_type: Option<&PyAny>,
        _exc_value: Option<&PyAny>,
        _traceback: Option<&PyAny>,
    ) -> PyResult<bool> {
        // Update connection status
        if let Ok(mut connected_guard) = self.connected.lock() {
            *connected_guard = false;
        }
        if let Ok(mut receiver_guard) = self.event_receiver.lock() {
            *receiver_guard = None;
        }
        Ok(false)
    }
}
