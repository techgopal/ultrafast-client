use pyo3::prelude::*;

mod client;
mod response;
mod config;
mod error;
mod middleware;
mod session;
mod async_session;
mod websocket;
mod sse;
mod async_client;
mod benchmark;
mod protocol_enhanced;
mod http3;
mod connection_pool;

// Shared modules to eliminate code duplication
mod auth_common;
mod rate_limit_common;
mod protocol_stats_common;
mod performance_common;
mod performance_advanced;

use client::HttpClient;
use response::Response;
use session::Session;
use async_session::AsyncSession;
use config::{AuthConfig, AuthType, RetryConfig, TimeoutConfig, PoolConfig, SSLConfig, OAuth2Token, ProxyConfig, CompressionConfig, 
           ProtocolConfig, HttpVersion, Http2Settings, Http3Settings, ProtocolFallback, RateLimitConfig, RateLimitAlgorithm};
use middleware::{Middleware, LoggingMiddleware, RetryMiddleware, HeadersMiddleware, MetricsMiddleware, InterceptorMiddleware, RateLimitMiddleware};
use websocket::{WebSocketClient, AsyncWebSocketClient, WebSocketMessage};
use sse::{SSEClient, AsyncSSEClient, SSEEvent, SSEEventIterator};
use async_client::AsyncHttpClient;
use benchmark::{Benchmark, MemoryProfiler};

// Import HTTP/3 types - keep only what's needed
// use http3::{Http3Client, Http3Response, Http3Stats, Http3ConnectionPool, AsyncHttp3ConnectionPool};
// Import shared manager types - unused currently
// use rate_limit_common::{RateLimitManager, AsyncRateLimitManager};
// use protocol_stats_common::{ProtocolStatsManager, AsyncProtocolStatsManager};
// use protocol_enhanced::EnhancedProtocolNegotiator;
// use performance_common::{PooledHeaders, HeaderCache, FastHeaderBuilder};
// use connection_pool::{FastConnectionPool, ConnectionMultiplexer};

/// UltraFast HTTP Client - Production-Ready HTTP Client for Python
/// 
/// A blazingly fast HTTP client built with Rust and Tokio, providing:
/// 
/// ## Features
/// - **High Performance**: 2-7x faster than popular Python HTTP libraries
/// - **Protocol Support**: HTTP/1.1, HTTP/2, HTTP/3 (QUIC), WebSocket, Server-Sent Events
/// - **Async/Sync APIs**: Both synchronous and asynchronous interfaces
/// - **Advanced Features**: Connection pooling, middleware, rate limiting, compression
/// - **Enterprise Ready**: Authentication, retries, circuit breakers, observability
/// 
/// ## Quick Start
/// ```python
/// import ultrafast_client as uc
/// 
/// # Synchronous usage
/// client = uc.HttpClient()
/// response = client.get("https://api.example.com/data")
/// print(response.text())
/// 
/// # Asynchronous usage  
/// async def main():
///     client = uc.AsyncHttpClient()
///     response = await client.get("https://api.example.com/data")
///     data = response.json()
/// ```
#[pymodule]
fn _ultrafast_client(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    // Core client classes
    m.add_class::<HttpClient>()?;
    m.add_class::<AsyncHttpClient>()?;
    
    // Response and session
    m.add_class::<Response>()?;
    m.add_class::<Session>()?;
    m.add_class::<AsyncSession>()?;
    
    // Configuration classes
    m.add_class::<AuthConfig>()?;
    m.add_class::<AuthType>()?;
    m.add_class::<RetryConfig>()?;
    m.add_class::<TimeoutConfig>()?;
    m.add_class::<PoolConfig>()?;
    m.add_class::<SSLConfig>()?;
    m.add_class::<OAuth2Token>()?;
    m.add_class::<ProxyConfig>()?;
    m.add_class::<CompressionConfig>()?;
    
    // Protocol configuration classes (Phase 5)
    m.add_class::<ProtocolConfig>()?;
    m.add_class::<HttpVersion>()?;
    m.add_class::<Http2Settings>()?;
    m.add_class::<Http3Settings>()?;
    m.add_class::<ProtocolFallback>()?;
    
    // Real-time communication
    m.add_class::<WebSocketClient>()?;
    m.add_class::<AsyncWebSocketClient>()?;
    m.add_class::<WebSocketMessage>()?;
    m.add_class::<SSEClient>()?;
    m.add_class::<AsyncSSEClient>()?;
    m.add_class::<SSEEvent>()?;
    m.add_class::<SSEEventIterator>()?;
    
    // Middleware
    m.add_class::<Middleware>()?;
    m.add_class::<LoggingMiddleware>()?;
    m.add_class::<HeadersMiddleware>()?;
    m.add_class::<RetryMiddleware>()?;
    m.add_class::<MetricsMiddleware>()?;
    m.add_class::<InterceptorMiddleware>()?;
    m.add_class::<RateLimitMiddleware>()?;
    
    // Performance tools
    m.add_class::<Benchmark>()?;
    m.add_class::<MemoryProfiler>()?;
    
    // Rate limiting (Phase 5)
    m.add_class::<RateLimitConfig>()?;
    m.add_class::<RateLimitAlgorithm>()?;
    
    // Add version
    m.add("__version__", "0.1.0")?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_module_creation() -> PyResult<()> {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let module = PyModule::new(py, "_ultrafast_client")
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create module: {}", e)))?;
            assert!(_ultrafast_client(py, module).is_ok());
            Ok(())
        })
    }
}
