//! UltraFast HTTP Client
//!
//! A high-performance HTTP client library for Python, built with Rust and PyO3.
//! Provides both synchronous and asynchronous interfaces with comprehensive features.

#![allow(clippy::too_many_arguments)]
#![allow(clippy::enum_variant_names)]
#![allow(clippy::type_complexity)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::declare_interior_mutable_const)]
#![allow(clippy::needless_lifetimes)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::unwrap_or_default)]
#![allow(clippy::assign_op_pattern)]
#![allow(clippy::iter_next_slice)]
#![allow(clippy::while_let_loop)]
#![allow(clippy::implicit_saturating_add)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::unnecessary_map_or)]
#![allow(non_local_definitions)]

use pyo3::prelude::*;

mod async_client;
mod async_session;
mod benchmark;
mod client;
mod config;
mod connection_pool;
mod error;
mod http3;
mod middleware;
mod protocol_enhanced;
mod response;
mod session;
mod sse;
mod websocket;

// Shared modules to eliminate code duplication
mod auth_common;
mod performance_advanced;
mod performance_common;
mod protocol_stats_common;
mod rate_limit_common;

use async_client::AsyncHttpClient;
use async_session::AsyncSession;
use benchmark::{Benchmark, MemoryProfiler};
use client::HttpClient;
use config::{
    AuthConfig, AuthType, CompressionConfig, Http2Settings, Http3Settings, HttpVersion,
    OAuth2Token, PoolConfig, ProtocolConfig, ProtocolFallback, ProxyConfig, RateLimitAlgorithm,
    RateLimitConfig, RetryConfig, SSLConfig, TimeoutConfig,
};
use middleware::{
    HeadersMiddleware, InterceptorMiddleware, LoggingMiddleware, MetricsMiddleware, Middleware,
    RateLimitMiddleware, RetryMiddleware,
};
use response::Response;
use session::Session;
use sse::{AsyncSSEClient, SSEClient, SSEEvent, SSEEventIterator};
use websocket::{AsyncWebSocketClient, WebSocketClient, WebSocketMessage};

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
            let module = PyModule::new(py, "_ultrafast_client").map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create module: {}", e))
            })?;
            assert!(_ultrafast_client(py, module).is_ok());
            Ok(())
        })
    }
}
