use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3::types::PyDict;
use std::collections::HashMap;

/// HTTP Response object
#[pyclass]
#[derive(Clone, Debug)]
pub struct Response {
    #[pyo3(get)]
    pub status_code: u16,
    #[pyo3(get)]
    pub headers: HashMap<String, String>,
    pub content: Vec<u8>,
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub elapsed: f64,
    #[pyo3(get)]
    pub protocol: Option<String>,
    #[pyo3(get)]
    pub protocol_version: Option<f32>,
    pub protocol_stats: Option<HashMap<String, String>>,
    #[pyo3(get)]
    pub request_time: f64, // Request time in seconds
    #[pyo3(get)]
    pub response_time: f64, // Response time in seconds
    #[pyo3(get)]
    pub total_time: f64, // Total time in seconds
    #[pyo3(get)]
    pub start_time: f64, // Start timestamp
    #[pyo3(get)]
    pub end_time: f64, // End timestamp
    pub timing: Option<f64>,
}

#[pymethods]
impl Response {
    /// Get response body as text
    pub fn text(&self) -> PyResult<String> {
        String::from_utf8(self.content.clone()).map_err(|e| {
            pyo3::exceptions::PyUnicodeDecodeError::new_err(format!(
                "Failed to decode response as UTF-8: {}",
                e
            ))
        })
    }

    /// Get response body as bytes
    pub fn bytes<'py>(&self, py: Python<'py>) -> PyResult<&'py PyBytes> {
        Ok(PyBytes::new(py, &self.content))
    }

    /// Parse response as JSON
    pub fn json(&self, py: Python) -> PyResult<PyObject> {
        let text = self.text()?;
        let value: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid JSON: {}", e)))?;

        // Import from client module where it's defined
        crate::client::json_to_python(py, &value)
    }

    /// Check if response status is successful (2xx)
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status_code)
    }

    /// Raise an exception if status is not successful
    pub fn raise_for_status(&self) -> PyResult<()> {
        if !self.ok() {
            let msg = format!("HTTP {} Error: {}", self.status_code, self.status_text());
            match self.status_code {
                400..=499 => Err(pyo3::exceptions::PyValueError::new_err(msg)),
                500..=599 => Err(pyo3::exceptions::PyRuntimeError::new_err(msg)),
                _ => Err(pyo3::exceptions::PyException::new_err(msg)),
            }
        } else {
            Ok(())
        }
    }

    /// Get status text description
    pub fn status_text(&self) -> String {
        match self.status_code {
            200 => "OK",
            201 => "Created",
            204 => "No Content",
            301 => "Moved Permanently",
            302 => "Found",
            304 => "Not Modified",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            429 => "Too Many Requests",
            500 => "Internal Server Error",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            504 => "Gateway Timeout",
            _ => "Unknown Status",
        }
        .to_string()
    }

    /// Get a specific header value
    pub fn get_header(&self, name: &str) -> Option<String> {
        self.headers.get(&name.to_lowercase()).cloned()
    }

    /// Get content length
    #[getter]
    pub fn content_length(&self) -> usize {
        self.content.len()
    }

    /// Iterate over response content chunks (simplified synchronous implementation)
    pub fn iter_chunks(&self, chunk_size: Option<usize>) -> PyResult<Vec<PyObject>> {
        let chunk_size = chunk_size.unwrap_or(8192);
        let mut chunks = Vec::new();

        Python::with_gil(|py| {
            for chunk in self.content.chunks(chunk_size) {
                chunks.push(PyBytes::new(py, chunk).to_object(py));
            }
            Ok(chunks)
        })
    }

    /// Iterate over response content lines (simplified synchronous implementation)
    pub fn iter_lines(&self) -> PyResult<Vec<PyObject>> {
        let content_str = String::from_utf8_lossy(&self.content);
        let mut lines = Vec::new();

        Python::with_gil(|py| {
            for line in content_str.lines() {
                lines.push(line.to_string().into_py(py));
            }
            Ok(lines)
        })
    }

    /// String representation
    fn __str__(&self) -> String {
        format!("<Response [{}]>", self.status_code)
    }

    /// Detailed representation
    fn __repr__(&self) -> String {
        format!(
            "Response(status_code={}, url='{}', content_length={})",
            self.status_code,
            self.url,
            self.content.len()
        )
    }

    /// Get protocol statistics if available
    #[pyo3(name = "get_protocol_stats")]
    pub fn get_protocol_stats_py(&self, py: Python) -> PyObject {
        match &self.protocol_stats {
            Some(stats) => {
                // Convert HashMap to Python dict
                let dict = pyo3::types::PyDict::new(py);
                for (key, value) in stats {
                    dict.set_item(key, value).unwrap_or(());
                }
                dict.into()
            }
            None => {
                // Return empty dict if no stats
                pyo3::types::PyDict::new(py).into()
            }
        }
    }

    /// Set timing information for the response
    pub fn set_timing(&mut self, request_start: f64, request_end: f64) {
        self.timing = Some(request_end - request_start);
    }

    /// Get detailed timing information as a dictionary
    pub fn get_timing_info(&self, py: Python) -> PyObject {
        let dict = PyDict::new(py);
        dict.set_item("total_time", self.total_time).unwrap_or(());
        dict.set_item("request_time", self.request_time)
            .unwrap_or(());
        dict.set_item("response_time", self.response_time)
            .unwrap_or(());
        dict.set_item("start_time", self.start_time).unwrap_or(());
        dict.set_item("end_time", self.end_time).unwrap_or(());
        dict.set_item("elapsed", self.elapsed).unwrap_or(());
        dict.into()
    }

    /// Check if response was fast (under threshold)
    pub fn is_fast(&self, threshold_seconds: f64) -> bool {
        self.total_time < threshold_seconds
    }

    /// Get performance rating based on response time
    pub fn get_performance_rating(&self) -> String {
        match self.total_time {
            t if t < 0.1 => "Excellent".to_string(),
            t if t < 0.5 => "Good".to_string(),
            t if t < 1.0 => "Average".to_string(),
            t if t < 2.0 => "Slow".to_string(),
            _ => "Very Slow".to_string(),
        }
    }
}

impl Response {
    /// Create response from reqwest response - internal method
    pub(crate) async fn from_reqwest_response_async(response: reqwest::Response) -> PyResult<Self> {
        let status_code = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or_default().to_string()))
            .collect();

        let url = response.url().to_string();

        // Get protocol information before consuming the response
        // Determine protocol from HTTP version
        let version = response.version();
        let (protocol, protocol_version) = match version {
            reqwest::Version::HTTP_10 => (Some("HTTP/1.0".to_string()), Some(1.0)),
            reqwest::Version::HTTP_11 => (Some("HTTP/1.1".to_string()), Some(1.1)),
            reqwest::Version::HTTP_2 => (Some("HTTP/2".to_string()), Some(2.0)),
            reqwest::Version::HTTP_3 => (Some("HTTP/3".to_string()), Some(3.0)),
            _ => (None, None),
        };

        // For streaming, we might not want to read the whole body here immediately.
        // However, the current structure of Response holds Vec<u8>.
        // For true async streaming into Response, its structure would need to change,
        // or these iter_chunks/iter_lines methods become the primary way to access body.
        let content = response
            .bytes()
            .await
            .map_err(|e| {
                pyo3::exceptions::PyIOError::new_err(format!("Failed to read response body: {}", e))
            })?
            .to_vec();

        Ok(Response {
            status_code,
            headers,
            content,
            url,
            elapsed: 0.0, // Will be set by the client when timing information is available
            protocol,
            protocol_version,
            protocol_stats: None,
            request_time: 0.0,
            response_time: 0.0,
            total_time: 0.0,
            start_time: 0.0,
            end_time: 0.0,
            timing: None,
        })
    }

    /// Create a Response from a reqwest::Response
    pub fn from_reqwest(
        response: reqwest::Response,
        runtime: &tokio::runtime::Runtime,
    ) -> PyResult<Self> {
        let status_code = response.status().as_u16();
        let url = response.url().to_string();

        // Convert headers
        let mut headers = HashMap::new();
        let version = response.version();
        for (key, value) in response.headers() {
            headers.insert(key.to_string(), value.to_str().unwrap_or("").to_string());
        }

        // Get the response body
        let content = runtime
            .block_on(async { response.bytes().await })
            .map_err(|e| {
                pyo3::exceptions::PyIOError::new_err(format!("Failed to read response body: {}", e))
            })?;

        // Determine protocol from HTTP version
        let (protocol, protocol_version) = match version {
            reqwest::Version::HTTP_10 => (Some("HTTP/1.0".to_string()), Some(1.0)),
            reqwest::Version::HTTP_11 => (Some("HTTP/1.1".to_string()), Some(1.1)),
            reqwest::Version::HTTP_2 => (Some("HTTP/2".to_string()), Some(2.0)),
            reqwest::Version::HTTP_3 => (Some("HTTP/3".to_string()), Some(3.0)),
            _ => (None, None),
        };

        Ok(Response {
            status_code,
            url,
            headers,
            content: content.to_vec(),
            elapsed: 0.0,
            protocol,
            protocol_version,
            protocol_stats: None,
            request_time: 0.0,
            response_time: 0.0,
            total_time: 0.0,
            start_time: 0.0,
            end_time: 0.0,
            timing: None,
        })
    }
}

// Protocol stats methods are part of the main Response implementation
// They are added to the existing #[pymethods] block above
