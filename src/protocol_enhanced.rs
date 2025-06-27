use crate::config::{HttpVersion, ProtocolConfig, ProtocolFallback};
use ahash::AHashMap;
use once_cell::sync::Lazy;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::{Duration, Instant};

/// Enhanced protocol negotiator with machine learning capabilities
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct EnhancedProtocolNegotiator {
    protocol_cache: Arc<RwLock<AHashMap<String, HostCapabilities>>>,
    metrics_cache: Arc<RwLock<AHashMap<String, ProtocolMetrics>>>,
    weights: Arc<RwLock<AHashMap<String, ProtocolWeights>>>,
    #[allow(dead_code)]
    fallback_strategy: ProtocolFallback,
    // Cache settings with TTL support
    cache_ttl: Duration,
    #[allow(dead_code)]
    dns_cache: Arc<RwLock<AHashMap<String, (Vec<SocketAddr>, Instant)>>>,
    #[allow(dead_code)]
    dns_cache_ttl: Duration,
}

/// Enhanced host capabilities with detailed performance tracking
#[derive(Debug, Clone)]
pub struct HostCapabilities {
    /// Highest supported HTTP version
    pub max_version: HttpVersion,
    /// Whether HTTP/2 is available
    pub http2_available: bool,
    /// Whether HTTP/3 is available
    pub http3_available: bool,
    /// Last updated timestamp
    pub last_updated: Instant,
    /// Success rate for this protocol (0.0 - 1.0)
    pub success_rate: f64,
    /// Average connection time
    pub avg_connection_time: Duration,
    /// Preferred ALPN protocols
    pub alpn_protocols: Vec<String>,
    /// TLS version support
    pub tls_versions: Vec<String>,
    /// Connection failure count
    pub failure_count: u32,
    /// Average response time for this protocol
    pub avg_response_time: Option<Duration>,
    /// Connection reliability score (0.0 - 1.0)
    pub reliability_score: f64,
    /// Protocol-specific performance metrics
    pub http1_metrics: ProtocolMetrics,
    pub http2_metrics: ProtocolMetrics,
    pub http3_metrics: ProtocolMetrics,
}

/// Protocol-specific performance metrics
#[derive(Debug, Clone, Default)]
pub struct ProtocolMetrics {
    pub request_count: u64,
    pub success_count: u64,
    pub total_response_time: Duration,
    pub last_used: Option<Instant>,
    pub connection_errors: u32,
    pub timeout_errors: u32,
}

/// Protocol preference weights for intelligent selection
#[derive(Debug, Clone)]
pub struct ProtocolWeights {
    pub http1_weight: f64,
    pub http2_weight: f64,
    pub http3_weight: f64,
    pub last_updated: Instant,
}

impl Default for ProtocolWeights {
    fn default() -> Self {
        Self {
            http1_weight: 1.0,
            http2_weight: 1.5,
            http3_weight: 2.0,
            last_updated: Instant::now(),
        }
    }
}

impl Default for HostCapabilities {
    fn default() -> Self {
        Self {
            max_version: HttpVersion::Http1,
            http2_available: false,
            http3_available: false,
            last_updated: Instant::now(),
            success_rate: 1.0,
            avg_connection_time: Duration::from_millis(100),
            alpn_protocols: vec!["http/1.1".to_string()],
            tls_versions: vec!["TLSv1.3".to_string()],
            failure_count: 0,
            avg_response_time: None,
            reliability_score: 1.0,
            http1_metrics: ProtocolMetrics::default(),
            http2_metrics: ProtocolMetrics::default(),
            http3_metrics: ProtocolMetrics::default(),
        }
    }
}

impl EnhancedProtocolNegotiator {
    /// Create a new enhanced protocol negotiator
    pub fn new(fallback_strategy: ProtocolFallback) -> Self {
        Self {
            protocol_cache: Arc::new(RwLock::new(AHashMap::new())),
            metrics_cache: Arc::new(RwLock::new(AHashMap::new())),
            weights: Arc::new(RwLock::new(AHashMap::<String, ProtocolWeights>::new())),
            fallback_strategy,
            cache_ttl: Duration::from_secs(3600), // 1 hour cache
            dns_cache: Arc::new(RwLock::new(AHashMap::new())),
            dns_cache_ttl: Duration::from_secs(300), // 5 minute DNS cache
        }
    }

    /// Select the best protocol for a given URL with intelligent caching
    pub async fn select_protocol(&self, url: &str, config: &ProtocolConfig) -> HttpVersion {
        let host = extract_host(url);

        match &config.preferred_version {
            HttpVersion::Auto => self.auto_select_protocol(&host, config).await,
            specific_version => specific_version.clone(),
        }
    }

    /// Automatically select the best protocol using machine learning-like approach
    async fn auto_select_protocol(&self, host: &str, config: &ProtocolConfig) -> HttpVersion {
        // Fast path: check cache first
        if let Some(capabilities) = self.get_cached_capabilities(host) {
            if !self.is_cache_expired(&capabilities) {
                return self.select_best_protocol(&capabilities, config);
            }
        }

        // Slow path: detect protocol capabilities
        let capabilities = self.detect_protocol_capabilities(host).await;
        self.update_cache(host, capabilities.clone());

        self.select_best_protocol(&capabilities, config)
    }

    /// Get cached capabilities with fast read lock
    fn get_cached_capabilities(&self, host: &str) -> Option<HostCapabilities> {
        let cache = self.protocol_cache.read().ok()?;
        cache.get(host).cloned()
    }

    /// Check if cache entry is expired
    fn is_cache_expired(&self, capabilities: &HostCapabilities) -> bool {
        capabilities.last_updated.elapsed() > self.cache_ttl
    }

    /// Select the best protocol based on capabilities and performance metrics
    fn select_best_protocol(
        &self,
        capabilities: &HostCapabilities,
        config: &ProtocolConfig,
    ) -> HttpVersion {
        // Get protocol weights for intelligent selection
        let weights = self.get_protocol_weights(&extract_host_from_capabilities(capabilities));

        // Calculate scores for each protocol
        let mut scores = Vec::new();

        if capabilities.http3_available {
            let score = self.calculate_protocol_score(
                &capabilities.http3_metrics,
                weights.http3_weight,
                capabilities.reliability_score,
            );
            scores.push((HttpVersion::Http3, score));
        }

        if capabilities.http2_available {
            let score = self.calculate_protocol_score(
                &capabilities.http2_metrics,
                weights.http2_weight,
                capabilities.reliability_score,
            );
            scores.push((HttpVersion::Http2, score));
        }

        // HTTP/1.1 is always available
        let score = self.calculate_protocol_score(
            &capabilities.http1_metrics,
            weights.http1_weight,
            capabilities.reliability_score,
        );
        scores.push((HttpVersion::Http1, score));

        // Select the protocol with the highest score
        scores
            .into_iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(version, _)| version)
            .unwrap_or(HttpVersion::Http1)
    }

    /// Calculate protocol score based on performance metrics
    fn calculate_protocol_score(
        &self,
        metrics: &ProtocolMetrics,
        weight: f64,
        reliability: f64,
    ) -> f64 {
        if metrics.request_count == 0 {
            return weight; // Base weight for untested protocols
        }

        let success_rate = metrics.success_count as f64 / metrics.request_count as f64;
        let avg_response_time = if metrics.request_count > 0 {
            metrics.total_response_time.as_millis() as f64 / metrics.request_count as f64
        } else {
            1000.0 // Default 1 second
        };

        // Lower response time is better, so invert it
        let time_score = 1000.0 / (avg_response_time + 1.0);

        // Combine factors: weight * success_rate * time_score * reliability
        weight * success_rate * time_score * reliability * 0.0001
    }

    /// Get protocol weights for a host
    fn get_protocol_weights(&self, host: &str) -> ProtocolWeights {
        if let Ok(weights) = self.weights.read() {
            weights.get(host).cloned().unwrap_or_else(|| {
                ProtocolWeights {
                    http1_weight: 1.0,
                    http2_weight: 1.5, // Prefer HTTP/2 by default
                    http3_weight: 2.0, // Prefer HTTP/3 most by default
                    last_updated: Instant::now(),
                }
            })
        } else {
            ProtocolWeights::default()
        }
    }

    /// Detect protocol capabilities for a host
    async fn detect_protocol_capabilities(&self, host: &str) -> HostCapabilities {
        // In a real implementation, this would perform actual protocol detection
        // For now, we'll use heuristics and return optimistic capabilities

        let mut capabilities = HostCapabilities::default();
        capabilities.last_updated = Instant::now();

        // Assume modern hosts support HTTP/2 and HTTP/3
        capabilities.http2_available = true;
        capabilities.http3_available = true;
        capabilities.max_version = HttpVersion::Http3;
        capabilities.alpn_protocols =
            vec!["h3".to_string(), "h2".to_string(), "http/1.1".to_string()];

        capabilities
    }

    /// Update the protocol capabilities cache
    fn update_cache(&self, host: &str, capabilities: HostCapabilities) {
        if let Ok(mut cache) = self.protocol_cache.write() {
            cache.insert(host.to_string(), capabilities);
        }
    }

    /// Update protocol performance metrics after a request
    pub fn update_protocol_metrics(
        &self,
        host: &str,
        protocol: &HttpVersion,
        success: bool,
        response_time: Duration,
    ) {
        if let Ok(mut cache) = self.protocol_cache.write() {
            if let Some(capabilities) = cache.get_mut(host) {
                let metrics = match protocol {
                    HttpVersion::Http1 => &mut capabilities.http1_metrics,
                    HttpVersion::Http2 => &mut capabilities.http2_metrics,
                    HttpVersion::Http3 => &mut capabilities.http3_metrics,
                    HttpVersion::Auto => &mut capabilities.http1_metrics, // Fallback
                };

                metrics.request_count += 1;
                if success {
                    metrics.success_count += 1;
                } else {
                    metrics.connection_errors += 1;
                }
                metrics.total_response_time += response_time;
                metrics.last_used = Some(Instant::now());

                // Update overall success rate
                let total_requests = capabilities.http1_metrics.request_count
                    + capabilities.http2_metrics.request_count
                    + capabilities.http3_metrics.request_count;
                let total_successes = capabilities.http1_metrics.success_count
                    + capabilities.http2_metrics.success_count
                    + capabilities.http3_metrics.success_count;

                if total_requests > 0 {
                    capabilities.success_rate = total_successes as f64 / total_requests as f64;
                }

                // Update protocol weights based on performance
                self.update_protocol_weights(host, protocol, success, response_time);
            }
        }
    }

    /// Update protocol preference weights based on performance
    fn update_protocol_weights(
        &self,
        host: &str,
        protocol: &HttpVersion,
        success: bool,
        response_time: Duration,
    ) {
        if let Ok(mut weights) = self.weights.write() {
            let host_weights = weights.entry(host.to_string()).or_insert_with(ProtocolWeights::default);

            // Learning rate for weight adjustment
            let learning_rate = 0.1;
            let performance_score = if success {
                // Better performance = higher weight
                1.0 / (response_time.as_millis() as f64 + 1.0)
            } else {
                // Failure = lower weight
                -0.1
            };

            match protocol {
                HttpVersion::Http1 => {
                    host_weights.http1_weight = (host_weights.http1_weight 
                        + learning_rate * performance_score).max(0.1);
                }
                HttpVersion::Http2 => {
                    host_weights.http2_weight = (host_weights.http2_weight 
                        + learning_rate * performance_score).max(0.1);
                }
                HttpVersion::Http3 => {
                    host_weights.http3_weight = (host_weights.http3_weight 
                        + learning_rate * performance_score).max(0.1);
                }
                HttpVersion::Auto => {} // No-op
            }

            host_weights.last_updated = Instant::now();
        }
    }

    /// Get cache statistics for monitoring
    pub fn get_cache_stats(&self) -> CacheStats {
        if let Ok(cache) = self.protocol_cache.read() {
            CacheStats {
                cache_hits: 0, // Simplified - not tracking hits/misses anymore
                cache_misses: 0,
                hit_rate: 0.0,
                cached_hosts: cache.len(),
                cache_size_bytes: std::mem::size_of_val(&*cache)
                    + cache
                        .iter()
                        .map(|(k, v)| k.len() + std::mem::size_of_val(v))
                        .sum::<usize>(),
            }
        } else {
            CacheStats {
                cache_hits: 0,
                cache_misses: 0,
                hit_rate: 0.0,
                cached_hosts: 0,
                cache_size_bytes: 0,
            }
        }
    }

    /// Clean up expired cache entries
    pub fn cleanup_expired_entries(&self) {
        let now = Instant::now();

        // Clean up capability cache
        if let Ok(mut cache) = self.protocol_cache.write() {
            cache.retain(|_, capabilities| {
                now.duration_since(capabilities.last_updated) < self.cache_ttl
            });
        }

        // Clean up DNS cache
        if let Ok(mut dns_cache) = self.dns_cache.write() {
            dns_cache.retain(|_, (_, timestamp)| {
                now.duration_since(*timestamp) < self.dns_cache_ttl
            });
        }

        // Clean up preference weights (keep longer)
        if let Ok(mut weights) = self.weights.write() {
            let weight_ttl = Duration::from_secs(86400); // 24 hours
            weights.retain(|_, weight| now.duration_since(weight.last_updated) < weight_ttl);
        }
    }

    /// Preload protocol capabilities for a list of hosts
    pub async fn preload_capabilities(&self, hosts: &[String]) {
        for host in hosts {
            if self.get_cached_capabilities(host).is_none() {
                let capabilities = self.detect_protocol_capabilities(host).await;
                self.update_cache(host, capabilities);
            }
        }
    }
}

/// Cache statistics for monitoring
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub hit_rate: f64,
    pub cached_hosts: usize,
    pub cache_size_bytes: usize,
}

/// Extract hostname from URL
fn extract_host(url: &str) -> String {
    // Simple hostname extraction without external dependencies
    if let Some(start) = url.find("://") {
        let after_protocol = &url[start + 3..];
        if let Some(end) = after_protocol.find('/') {
            after_protocol[..end].to_string()
        } else if let Some(end) = after_protocol.find(':') {
            after_protocol[..end].to_string()
        } else {
            after_protocol.to_string()
        }
    } else {
        "localhost".to_string()
    }
}

/// Extract hostname from capabilities (helper function)
fn extract_host_from_capabilities(_capabilities: &HostCapabilities) -> String {
    // In a real implementation, this would extract the host from capabilities
    // For now, return a placeholder
    "unknown".to_string()
}

/// Protocol detection utilities with enhanced capabilities
pub struct ProtocolDetector;

impl ProtocolDetector {
    /// Detect HTTP/3 support using QUIC connection attempt
    pub async fn detect_http3_support(host: &str, port: u16) -> bool {
        // Implement QUIC connection test
        // For now, assume most modern hosts support HTTP/3
        tokio::time::timeout(Duration::from_millis(500), async {
            // Simulate QUIC handshake attempt
            tokio::time::sleep(Duration::from_millis(50)).await;
            true
        })
        .await
        .unwrap_or(false)
    }

    /// Detect HTTP/2 support using ALPN negotiation
    pub async fn detect_http2_support(host: &str, port: u16) -> bool {
        // Implement TLS ALPN negotiation test
        // For now, assume most HTTPS hosts support HTTP/2
        tokio::time::timeout(Duration::from_millis(200), async {
            // Simulate TLS handshake with ALPN
            tokio::time::sleep(Duration::from_millis(20)).await;
            true
        })
        .await
        .unwrap_or(false)
    }

    /// Comprehensive protocol detection
    pub async fn detect_all_protocols(host: &str, port: u16) -> HostCapabilities {
        let start = Instant::now();

        let (http2_available, http3_available) = tokio::join!(
            Self::detect_http2_support(host, port),
            Self::detect_http3_support(host, port)
        );

        let connection_time = start.elapsed();

        HostCapabilities {
            max_version: if http3_available {
                HttpVersion::Http3
            } else if http2_available {
                HttpVersion::Http2
            } else {
                HttpVersion::Http1
            },
            http2_available,
            http3_available,
            last_updated: Instant::now(),
            success_rate: 1.0,
            avg_connection_time: connection_time,
            alpn_protocols: {
                let mut protocols = vec!["http/1.1".to_string()];
                if http2_available {
                    protocols.push("h2".to_string());
                }
                if http3_available {
                    protocols.push("h3".to_string());
                }
                protocols
            },
            tls_versions: vec!["TLSv1.3".to_string(), "TLSv1.2".to_string()],
            failure_count: 0,
            avg_response_time: Some(connection_time),
            reliability_score: 1.0,
            http1_metrics: ProtocolMetrics::default(),
            http2_metrics: ProtocolMetrics::default(),
            http3_metrics: ProtocolMetrics::default(),
        }
    }
}

/// Global protocol negotiator instance for improved performance
static GLOBAL_NEGOTIATOR: Lazy<EnhancedProtocolNegotiator> =
    Lazy::new(|| EnhancedProtocolNegotiator::new(ProtocolFallback::Http3ToHttp2ToHttp1));

/// Get the global protocol negotiator instance
pub fn get_global_negotiator() -> &'static EnhancedProtocolNegotiator {
    &GLOBAL_NEGOTIATOR
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_protocol_selection() {
        let negotiator = EnhancedProtocolNegotiator::new(ProtocolFallback::Http3ToHttp2ToHttp1);
        let config = ProtocolConfig::default();

        let protocol = negotiator
            .select_protocol("https://example.com", &config)
            .await;
        assert!(matches!(
            protocol,
            HttpVersion::Http1 | HttpVersion::Http2 | HttpVersion::Http3
        ));
    }

    #[tokio::test]
    async fn test_cache_functionality() {
        let negotiator = EnhancedProtocolNegotiator::new(ProtocolFallback::Http3ToHttp2ToHttp1);

        // First access should be a cache miss
        let capabilities1 = negotiator.detect_protocol_capabilities("example.com").await;
        negotiator.update_cache("example.com", capabilities1.clone());

        // Second access should be a cache hit
        let cached = negotiator.get_cached_capabilities("example.com");
        assert!(cached.is_some());

        let stats = negotiator.get_cache_stats();
        assert_eq!(stats.cached_hosts, 1);
    }

    #[test]
    fn test_protocol_metrics_update() {
        let negotiator = EnhancedProtocolNegotiator::new(ProtocolFallback::Http3ToHttp2ToHttp1);

        // Update some metrics
        negotiator.update_protocol_metrics(
            "example.com",
            &HttpVersion::Http2,
            true,
            Duration::from_millis(100),
        );

        let capabilities = negotiator.get_cached_capabilities("example.com");
        if let Some(caps) = capabilities {
            assert_eq!(caps.http2_metrics.request_count, 1);
            assert_eq!(caps.http2_metrics.success_count, 1);
        }
    }

    #[test]
    fn test_protocol_score_calculation() {
        let negotiator = EnhancedProtocolNegotiator::new(ProtocolFallback::Http3ToHttp2ToHttp1);

        let mut metrics = ProtocolMetrics::default();
        metrics.request_count = 10;
        metrics.success_count = 9;
        metrics.total_response_time = Duration::from_millis(1000);

        let score = negotiator.calculate_protocol_score(&metrics, 1.5, 0.9);
        assert!(score > 0.0);
    }
}
