use pyo3::prelude::*;
use std::collections::HashMap;
use url::Url;
use base64::{Engine as _, engine::general_purpose};
use regex::Regex;

/// Utility functions for the HTTP client
pub struct Utils;

impl Utils {
    /// Parse a URL and extract components
    pub fn parse_url(url: &str) -> PyResult<HashMap<String, Option<String>>> {
        let parsed = Url::parse(url).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid URL: {}", e))
        })?;
        
        let mut components = HashMap::new();
        components.insert("scheme".to_string(), Some(parsed.scheme().to_string()));
        components.insert("host".to_string(), parsed.host_str().map(|s| s.to_string()));
        components.insert("port".to_string(), parsed.port().map(|p| p.to_string()));
        components.insert("path".to_string(), Some(parsed.path().to_string()));
        components.insert("query".to_string(), parsed.query().map(|s| s.to_string()));
        components.insert("fragment".to_string(), parsed.fragment().map(|s| s.to_string()));
        
        Ok(components)
    }
    
    /// Encode credentials for Basic authentication
    pub fn encode_basic_auth(username: &str, password: &str) -> String {
        let credentials = format!("{}:{}", username, password);
        format!("Basic {}", general_purpose::STANDARD.encode(credentials))
    }
    
    /// Decode Basic authentication header
    pub fn decode_basic_auth(auth_header: &str) -> PyResult<(String, String)> {
        if !auth_header.starts_with("Basic ") {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Not a Basic authentication header"
            ));
        }
        
        let encoded = &auth_header[6..];
        let decoded = general_purpose::STANDARD.decode(encoded).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid base64: {}", e))
        })?;
        
        let credentials = String::from_utf8(decoded).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid UTF-8: {}", e))
        })?;
        
        if let Some(colon_pos) = credentials.find(':') {
            let username = credentials[..colon_pos].to_string();
            let password = credentials[colon_pos + 1..].to_string();
            Ok((username, password))
        } else {
            Err(pyo3::exceptions::PyValueError::new_err(
                "Invalid credentials format"
            ))
        }
    }
    
    /// Format headers for display
    pub fn format_headers(headers: &HashMap<String, String>) -> String {
        let mut formatted = Vec::new();
        for (key, value) in headers {
            formatted.push(format!("{}: {}", key, value));
        }
        formatted.join("\n")
    }
    
    /// Parse Content-Type header
    pub fn parse_content_type(content_type: &str) -> (String, HashMap<String, String>) {
        let mut parts = content_type.split(';');
        let media_type = parts.next().unwrap_or("").trim().to_string();
        
        let mut parameters = HashMap::new();
        for part in parts {
            if let Some(eq_pos) = part.find('=') {
                let key = part[..eq_pos].trim().to_string();
                let value = part[eq_pos + 1..].trim().trim_matches('"').to_string();
                parameters.insert(key, value);
            }
        }
        
        (media_type, parameters)
    }
    
    /// Build query string from parameters
    pub fn build_query_string(params: &HashMap<String, String>) -> String {
        let mut query_parts = Vec::new();
        for (key, value) in params {
            query_parts.push(format!("{}={}", 
                urlencoding::encode(key), 
                urlencoding::encode(value)
            ));
        }
        query_parts.join("&")
    }
    
    /// Parse query string into parameters
    pub fn parse_query_string(query: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        
        if query.is_empty() {
            return params;
        }
        
        for pair in query.split('&') {
            if let Some(eq_pos) = pair.find('=') {
                let key = urlencoding::decode(&pair[..eq_pos])
                    .unwrap_or_else(|_| pair[..eq_pos].into())
                    .into_owned();
                let value = urlencoding::decode(&pair[eq_pos + 1..])
                    .unwrap_or_else(|_| pair[eq_pos + 1..].into())
                    .into_owned();
                params.insert(key, value);
            } else {
                let key = urlencoding::decode(pair)
                    .unwrap_or_else(|_| pair.into())
                    .into_owned();
                params.insert(key, String::new());
            }
        }
        
        params
    }
    
    /// Validate email address format
    pub fn is_valid_email(email: &str) -> PyResult<bool> {
        let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid regex pattern: {}", e)))?;
        Ok(email_regex.is_match(email))
    }
    
    /// Validate URL format
    pub fn is_valid_url(url: &str) -> bool {
        Url::parse(url).is_ok()
    }
    
    /// Get file extension from URL or filename
    pub fn get_file_extension(path: &str) -> Option<String> {
        if let Some(dot_pos) = path.rfind('.') {
            if let Some(slash_pos) = path.rfind('/') {
                if dot_pos > slash_pos {
                    return Some(path[dot_pos + 1..].to_string());
                }
            } else {
                return Some(path[dot_pos + 1..].to_string());
            }
        }
        None
    }
    
    /// Guess MIME type from file extension
    pub fn guess_mime_type(extension: &str) -> &'static str {
        match extension.to_lowercase().as_str() {
            "html" | "htm" => "text/html",
            "css" => "text/css",
            "js" => "application/javascript",
            "json" => "application/json",
            "xml" => "application/xml",
            "txt" => "text/plain",
            "csv" => "text/csv",
            "pdf" => "application/pdf",
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "svg" => "image/svg+xml",
            "webp" => "image/webp",
            "mp4" => "video/mp4",
            "webm" => "video/webm",
            "mp3" => "audio/mpeg",
            "wav" => "audio/wav",
            "zip" => "application/zip",
            "tar" => "application/x-tar",
            "gz" => "application/gzip",
            _ => "application/octet-stream",
        }
    }
    
    /// Format bytes in human-readable format
    pub fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;
        
        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }
        
        if unit_index == 0 {
            format!("{} {}", bytes, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }
    
    /// Convert duration to human-readable format
    pub fn format_duration(duration_secs: f64) -> String {
        if duration_secs < 1.0 {
            format!("{:.0}ms", duration_secs * 1000.0)
        } else if duration_secs < 60.0 {
            format!("{:.1}s", duration_secs)
        } else if duration_secs < 3600.0 {
            let minutes = (duration_secs / 60.0) as u32;
            let seconds = duration_secs % 60.0;
            format!("{}m {:.1}s", minutes, seconds)
        } else {
            let hours = (duration_secs / 3600.0) as u32;
            let minutes = ((duration_secs % 3600.0) / 60.0) as u32;
            format!("{}h {}m", hours, minutes)
        }
    }
}

/// Python-exposed utility functions
#[pyclass]
pub struct PyUtils;

#[pymethods]
impl PyUtils {
    #[new]
    pub fn new() -> Self {
        PyUtils
    }

    #[staticmethod]
    pub fn parse_url(url: &str) -> PyResult<HashMap<String, Option<String>>> {
        Utils::parse_url(url)
    }
    
    #[staticmethod]
    pub fn encode_basic_auth(username: &str, password: &str) -> String {
        Utils::encode_basic_auth(username, password)
    }
    
    #[staticmethod]
    pub fn decode_basic_auth(auth_header: &str) -> PyResult<(String, String)> {
        Utils::decode_basic_auth(auth_header)
    }
    
    #[staticmethod]
    pub fn build_query_string(params: HashMap<String, String>) -> String {
        Utils::build_query_string(&params)
    }
    
    #[staticmethod]
    pub fn parse_query_string(query: &str) -> HashMap<String, String> {
        Utils::parse_query_string(query)
    }
    
    #[staticmethod]
    pub fn is_valid_email(email: &str) -> PyResult<bool> {
        Utils::is_valid_email(email)
    }
    
    #[staticmethod]
    pub fn is_valid_url(url: &str) -> bool {
        Utils::is_valid_url(url)
    }
    
    #[staticmethod]
    pub fn format_bytes(bytes: u64) -> String {
        Utils::format_bytes(bytes)
    }
    
    #[staticmethod]
    pub fn format_duration(duration_secs: f64) -> String {
        Utils::format_duration(duration_secs)
    }
}
