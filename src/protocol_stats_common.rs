use pyo3::prelude::*;
use pythonize;
use std::collections::HashMap;
#[cfg(test)]
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Shared protocol statistics management for both sync and async clients
/// This eliminates code duplication between HttpClient and AsyncHttpClient
#[derive(Debug, Clone)]
pub struct ProtocolStatsManager {
    stats: Arc<RwLock<HashMap<String, ProtocolStats>>>,
}

#[derive(Debug, Clone)]
pub struct ProtocolStats {
    pub protocol_version: String,
    pub connection_time: Option<Duration>,
    pub request_count: u64,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub average_response_time: Option<Duration>,
    pub last_used: Instant,
    pub error_count: u64,
    pub connection_reused: bool,
    pub tls_version: Option<String>,
    pub cipher_suite: Option<String>,
    pub custom_fields: HashMap<String, String>,
}

impl Default for ProtocolStats {
    fn default() -> Self {
        Self {
            protocol_version: "HTTP/1.1".to_string(),
            connection_time: None,
            request_count: 0,
            total_bytes_sent: 0,
            total_bytes_received: 0,
            average_response_time: None,
            last_used: Instant::now(),
            error_count: 0,
            connection_reused: false,
            tls_version: None,
            cipher_suite: None,
            custom_fields: HashMap::new(),
        }
    }
}

impl ProtocolStatsManager {
    /// Create a new protocol statistics manager
    pub fn new() -> Self {
        Self {
            stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Update statistics for a specific host/URL
    pub fn update_stats(&self, url: &str, new_stats: ProtocolStats) {
        if let Ok(mut stats) = self.stats.write() {
            stats.insert(url.to_string(), new_stats);
        }
    }
    
    /// Get statistics for a specific host/URL
    pub fn get_stats(&self, url: &str) -> Option<ProtocolStats> {
        self.stats.read().ok()?.get(url).cloned()
    }
    
    /// Get statistics as a Python dictionary
    pub fn get_stats_py<'py>(&self, py: Python<'py>, url: &str) -> PyResult<Py<PyAny>> {
        let stats_dict = if let Some(stats) = self.get_stats(url) {
            self.stats_to_dict(&stats)
        } else {
            HashMap::new()
        };
        
        pythonize::pythonize(py, &stats_dict)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Failed to convert stats: {}", e)))
    }
    
    /// Get all statistics as a Python dictionary
    pub fn get_all_stats_py<'py>(&self, py: Python<'py>) -> PyResult<Py<PyAny>> {
        let all_stats: HashMap<String, HashMap<String, serde_json::Value>> = if let Ok(stats) = self.stats.read() {
            stats.iter()
                .map(|(url, protocol_stats)| {
                    (url.clone(), self.stats_to_dict(protocol_stats))
                })
                .collect()
        } else {
            HashMap::new()
        };
        
        pythonize::pythonize(py, &all_stats)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Failed to convert stats: {}", e)))
    }
    
    /// Increment request count for a host
    pub fn increment_request_count(&self, url: &str) {
        if let Ok(mut stats) = self.stats.write() {
            let entry = stats.entry(url.to_string()).or_insert_with(ProtocolStats::default);
            entry.request_count += 1;
            entry.last_used = Instant::now();
        }
    }
    
    /// Update response time statistics
    pub fn update_response_time(&self, url: &str, response_time: Duration) {
        if let Ok(mut stats) = self.stats.write() {
            let entry = stats.entry(url.to_string()).or_insert_with(ProtocolStats::default);
            
            // Calculate rolling average
            if let Some(current_avg) = entry.average_response_time {
                let count = entry.request_count as f64;
                let new_avg_nanos = ((current_avg.as_nanos() as f64 * (count - 1.0)) + response_time.as_nanos() as f64) / count;
                entry.average_response_time = Some(Duration::from_nanos(new_avg_nanos as u64));
            } else {
                entry.average_response_time = Some(response_time);
            }
            
            entry.last_used = Instant::now();
        }
    }
    
    /// Update bytes transferred
    pub fn update_bytes_transferred(&self, url: &str, bytes_sent: u64, bytes_received: u64) {
        if let Ok(mut stats) = self.stats.write() {
            let entry = stats.entry(url.to_string()).or_insert_with(ProtocolStats::default);
            entry.total_bytes_sent += bytes_sent;
            entry.total_bytes_received += bytes_received;
            entry.last_used = Instant::now();
        }
    }
    
    /// Record an error for a host
    pub fn record_error(&self, url: &str) {
        if let Ok(mut stats) = self.stats.write() {
            let entry = stats.entry(url.to_string()).or_insert_with(ProtocolStats::default);
            entry.error_count += 1;
            entry.last_used = Instant::now();
        }
    }
    
    /// Clear statistics for all hosts
    pub fn clear_all_stats(&self) {
        if let Ok(mut stats) = self.stats.write() {
            stats.clear();
        }
    }
    
    /// Clear statistics for a specific host
    pub fn clear_stats(&self, url: &str) {
        if let Ok(mut stats) = self.stats.write() {
            stats.remove(url);
        }
    }
    
    /// Get a summary of all statistics
    pub fn get_summary(&self) -> HashMap<String, serde_json::Value> {
        let mut summary = HashMap::new();
        
        if let Ok(stats) = self.stats.read() {
            let total_hosts = stats.len();
            let total_requests: u64 = stats.values().map(|s| s.request_count).sum();
            let total_errors: u64 = stats.values().map(|s| s.error_count).sum();
            let total_bytes_sent: u64 = stats.values().map(|s| s.total_bytes_sent).sum();
            let total_bytes_received: u64 = stats.values().map(|s| s.total_bytes_received).sum();
            
            summary.insert("total_hosts".to_string(), serde_json::Value::Number(serde_json::Number::from(total_hosts)));
            summary.insert("total_requests".to_string(), serde_json::Value::Number(serde_json::Number::from(total_requests)));
            summary.insert("total_errors".to_string(), serde_json::Value::Number(serde_json::Number::from(total_errors)));
            summary.insert("total_bytes_sent".to_string(), serde_json::Value::Number(serde_json::Number::from(total_bytes_sent)));
            summary.insert("total_bytes_received".to_string(), serde_json::Value::Number(serde_json::Number::from(total_bytes_received)));
            
            if total_requests > 0 {
                let error_rate = (total_errors as f64 / total_requests as f64) * 100.0;
                summary.insert("error_rate_percent".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(error_rate).unwrap_or(serde_json::Number::from(0))));
            }
        }
        
        summary
    }
    
    /// Convert ProtocolStats to a dictionary for Python serialization
    fn stats_to_dict(&self, stats: &ProtocolStats) -> HashMap<String, serde_json::Value> {
        let mut dict = HashMap::new();
        
        dict.insert("protocol_version".to_string(), serde_json::Value::String(stats.protocol_version.clone()));
        dict.insert("request_count".to_string(), serde_json::Value::Number(serde_json::Number::from(stats.request_count)));
        dict.insert("total_bytes_sent".to_string(), serde_json::Value::Number(serde_json::Number::from(stats.total_bytes_sent)));
        dict.insert("total_bytes_received".to_string(), serde_json::Value::Number(serde_json::Number::from(stats.total_bytes_received)));
        dict.insert("error_count".to_string(), serde_json::Value::Number(serde_json::Number::from(stats.error_count)));
        dict.insert("connection_reused".to_string(), serde_json::Value::Bool(stats.connection_reused));
        
        if let Some(connection_time) = stats.connection_time {
            dict.insert("connection_time_ms".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(connection_time.as_secs_f64() * 1000.0).unwrap_or(serde_json::Number::from(0))));
        }
        
        if let Some(response_time) = stats.average_response_time {
            dict.insert("average_response_time_ms".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(response_time.as_secs_f64() * 1000.0).unwrap_or(serde_json::Number::from(0))));
        }
        
        if let Some(ref tls_version) = stats.tls_version {
            dict.insert("tls_version".to_string(), serde_json::Value::String(tls_version.clone()));
        }
        
        if let Some(ref cipher_suite) = stats.cipher_suite {
            dict.insert("cipher_suite".to_string(), serde_json::Value::String(cipher_suite.clone()));
        }
        
        // Add custom fields
        for (key, value) in &stats.custom_fields {
            dict.insert(key.clone(), serde_json::Value::String(value.clone()));
        }
        
        dict
    }
    
    /// Get summary statistics as HashMap<String, f64> for compatibility with existing client interfaces
    pub fn get_summary_f64(&self) -> HashMap<String, f64> {
        let mut summary = HashMap::new();
        
        if let Ok(stats) = self.stats.read() {
            let total_hosts = stats.len() as f64;
            let total_requests: u64 = stats.values().map(|s| s.request_count).sum();
            let total_errors: u64 = stats.values().map(|s| s.error_count).sum();
            let total_bytes_sent: u64 = stats.values().map(|s| s.total_bytes_sent).sum();
            let total_bytes_received: u64 = stats.values().map(|s| s.total_bytes_received).sum();
            
            summary.insert("total_hosts".to_string(), total_hosts);
            summary.insert("total_requests".to_string(), total_requests as f64);
            summary.insert("total_errors".to_string(), total_errors as f64);
            summary.insert("total_bytes_sent".to_string(), total_bytes_sent as f64);
            summary.insert("total_bytes_received".to_string(), total_bytes_received as f64);
            
            if total_requests > 0 {
                let error_rate = (total_errors as f64 / total_requests as f64) * 100.0;
                summary.insert("error_rate_percent".to_string(), error_rate);
            }
            
            // Calculate average response time across all hosts
            let response_times: Vec<f64> = stats.values()
                .filter_map(|s| s.average_response_time.map(|t| t.as_secs_f64() * 1000.0))
                .collect();
            
            if !response_times.is_empty() {
                let avg_response_time = response_times.iter().sum::<f64>() / response_times.len() as f64;
                summary.insert("overall_average_response_time_ms".to_string(), avg_response_time);
            }
        }
        
        summary
    }
}

/// Async-compatible protocol statistics manager
pub struct AsyncProtocolStatsManager {
    inner: Arc<tokio::sync::RwLock<HashMap<String, ProtocolStats>>>,
}

impl AsyncProtocolStatsManager {
    /// Create a new async protocol statistics manager
    pub fn new() -> Self {
        Self {
            inner: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }
    
    /// Update statistics for a specific host/URL asynchronously
    pub async fn update_stats(&self, url: &str, new_stats: ProtocolStats) {
        let mut stats = self.inner.write().await;
        stats.insert(url.to_string(), new_stats);
    }
    
    /// Get statistics for a specific host/URL asynchronously
    pub async fn get_stats(&self, url: &str) -> Option<ProtocolStats> {
        let stats = self.inner.read().await;
        stats.get(url).cloned()
    }
    
    /// Get statistics as a Python dictionary asynchronously
    pub(crate) async fn get_stats_py<'py>(&self, py: Python<'py>, url: &str) -> PyResult<Py<PyAny>> {
        let stats_dict = if let Some(stats) = self.get_stats(url).await {
            self.stats_to_dict(&stats).await
        } else {
            HashMap::new()
        };
        
        pythonize::pythonize(py, &stats_dict)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Failed to convert stats: {}", e)))
    }
    
    /// Increment request count for a host asynchronously
    pub async fn increment_request_count(&self, url: &str) {
        let mut stats = self.inner.write().await;
        let entry = stats.entry(url.to_string()).or_insert_with(ProtocolStats::default);
        entry.request_count += 1;
        entry.last_used = Instant::now();
    }
    
    /// Update response time statistics asynchronously
    pub async fn update_response_time(&self, url: &str, response_time: Duration) {
        let mut stats = self.inner.write().await;
        let entry = stats.entry(url.to_string()).or_insert_with(ProtocolStats::default);
        
        // Calculate rolling average
        if let Some(current_avg) = entry.average_response_time {
            let count = entry.request_count as f64;
            let new_avg_nanos = ((current_avg.as_nanos() as f64 * (count - 1.0)) + response_time.as_nanos() as f64) / count;
            entry.average_response_time = Some(Duration::from_nanos(new_avg_nanos as u64));
        } else {
            entry.average_response_time = Some(response_time);
        }
        
        entry.last_used = Instant::now();
    }
    
    /// Record an error for a host asynchronously
    pub async fn record_error(&self, url: &str) {
        let mut stats = self.inner.write().await;
        let entry = stats.entry(url.to_string()).or_insert_with(ProtocolStats::default);
        entry.error_count += 1;
        entry.last_used = Instant::now();
    }
    
    /// Clear statistics for all hosts asynchronously
    pub async fn clear_all_stats(&self) {
        let mut stats = self.inner.write().await;
        stats.clear();
    }
    
    /// Clear statistics for a specific host asynchronously
    pub async fn clear_stats(&self, url: &str) {
        let mut stats = self.inner.write().await;
        stats.remove(url);
    }
    
    /// Get all statistics as HashMap (for async clients)
    pub async fn get_all_stats(&self) -> HashMap<String, ProtocolStats> {
        let stats = self.inner.read().await;
        stats.clone()
    }

    /// Convert ProtocolStats to a dictionary for Python serialization
    async fn stats_to_dict(&self, stats: &ProtocolStats) -> HashMap<String, serde_json::Value> {
        let mut dict = HashMap::new();
        
        dict.insert("protocol_version".to_string(), serde_json::Value::String(stats.protocol_version.clone()));
        dict.insert("request_count".to_string(), serde_json::Value::Number(serde_json::Number::from(stats.request_count)));
        dict.insert("total_bytes_sent".to_string(), serde_json::Value::Number(serde_json::Number::from(stats.total_bytes_sent)));
        dict.insert("total_bytes_received".to_string(), serde_json::Value::Number(serde_json::Number::from(stats.total_bytes_received)));
        dict.insert("error_count".to_string(), serde_json::Value::Number(serde_json::Number::from(stats.error_count)));
        dict.insert("connection_reused".to_string(), serde_json::Value::Bool(stats.connection_reused));
        
        if let Some(connection_time) = stats.connection_time {
            dict.insert("connection_time_ms".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(connection_time.as_secs_f64() * 1000.0).unwrap_or(serde_json::Number::from(0))));
        }
        
        if let Some(response_time) = stats.average_response_time {
            dict.insert("average_response_time_ms".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(response_time.as_secs_f64() * 1000.0).unwrap_or(serde_json::Number::from(0))));
        }
        
        if let Some(ref tls_version) = stats.tls_version {
            dict.insert("tls_version".to_string(), serde_json::Value::String(tls_version.clone()));
        }
        
        if let Some(ref cipher_suite) = stats.cipher_suite {
            dict.insert("cipher_suite".to_string(), serde_json::Value::String(cipher_suite.clone()));
        }
        
        // Add custom fields
        for (key, value) in &stats.custom_fields {
            dict.insert(key.clone(), serde_json::Value::String(value.clone()));
        }
        
        dict
    }
}

impl Clone for AsyncProtocolStatsManager {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_protocol_stats_manager() {
        let manager = ProtocolStatsManager::new();
        
        // Test updating stats
        let mut stats = ProtocolStats::default();
        stats.protocol_version = "HTTP/2".to_string();
        stats.request_count = 5;
        
        manager.update_stats("https://example.com", stats.clone());
        
        let retrieved = manager.get_stats("https://example.com");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().protocol_version, "HTTP/2");
    }

    #[test]
    fn test_increment_request_count() {
        let manager = ProtocolStatsManager::new();
        
        manager.increment_request_count("https://example.com");
        manager.increment_request_count("https://example.com");
        
        let stats = manager.get_stats("https://example.com").unwrap();
        assert_eq!(stats.request_count, 2);
    }

    #[test]
    fn test_update_response_time() {
        let manager = ProtocolStatsManager::new();
        
        manager.update_response_time("https://example.com", Duration::from_millis(100));
        manager.update_response_time("https://example.com", Duration::from_millis(200));
        
        let stats = manager.get_stats("https://example.com").unwrap();
        assert!(stats.average_response_time.is_some());
    }

    #[tokio::test]
    async fn test_async_protocol_stats_manager() {
        let manager = AsyncProtocolStatsManager::new();
        
        let mut stats = ProtocolStats::default();
        stats.protocol_version = "HTTP/3".to_string();
        stats.request_count = 10;
        
        manager.update_stats("https://example.com", stats.clone()).await;
        
        let retrieved = manager.get_stats("https://example.com").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().protocol_version, "HTTP/3");
    }

}
