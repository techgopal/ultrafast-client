"""
UltraFast HTTP Client

A blazingly fast HTTP client for Python, built with Rust and Tokio.
"""

from ._ultrafast_client import (  # Configuration classes; Protocol configuration; Middleware classes; Rate limiting; Benchmarking
    AsyncHttpClient,
    AsyncSession,
    AsyncSSEClient,
    AsyncWebSocketClient,
    AuthConfig,
    AuthType,
    Benchmark,
    CompressionConfig,
    HeadersMiddleware,
    Http2Settings,
    Http3Settings,
    HttpClient,
    HttpVersion,
    InterceptorMiddleware,
    LoggingMiddleware,
    MemoryProfiler,
    MetricsMiddleware,
    Middleware,
    OAuth2Token,
    PoolConfig,
    ProtocolConfig,
    ProtocolFallback,
    ProxyConfig,
    RateLimitAlgorithm,
    RateLimitConfig,
    RateLimitMiddleware,
    Response,
    RetryConfig,
    RetryMiddleware,
    Session,
    SSEClient,
    SSEEvent,
    SSEEventIterator,
    SSLConfig,
    TimeoutConfig,
    WebSocketClient,
    WebSocketMessage,
)


# Convenience functions for quick usage
def get(url, **kwargs):
    """Perform a GET request."""
    client = HttpClient()
    return client.get(url, **kwargs)


def post(url, **kwargs):
    """Perform a POST request."""
    client = HttpClient()
    return client.post(url, **kwargs)


def put(url, **kwargs):
    """Perform a PUT request."""
    client = HttpClient()
    return client.put(url, **kwargs)


def delete(url, **kwargs):
    """Perform a DELETE request."""
    client = HttpClient()
    return client.delete(url, **kwargs)


def patch(url, **kwargs):
    """Perform a PATCH request."""
    client = HttpClient()
    return client.patch(url, **kwargs)


def head(url, **kwargs):
    """Perform a HEAD request."""
    client = HttpClient()
    return client.head(url, **kwargs)


def options(url, **kwargs):
    """Perform an OPTIONS request."""
    client = HttpClient()
    return client.options(url, **kwargs)


__version__ = "0.1.2"
__author__ = "UltraFast Team"

__all__ = [
    # Core classes
    "HttpClient",
    "AsyncHttpClient",
    "Session",
    "AsyncSession",
    "WebSocketClient",
    "AsyncWebSocketClient",
    "WebSocketMessage",
    "SSEClient",
    "AsyncSSEClient",
    "SSEEvent",
    "SSEEventIterator",
    "Response",
    # Configuration classes
    "RetryConfig",
    "SSLConfig",
    "PoolConfig",
    "AuthConfig",
    "AuthType",
    "OAuth2Token",
    "TimeoutConfig",
    "ProxyConfig",
    "CompressionConfig",
    # Protocol configuration
    "ProtocolConfig",
    "HttpVersion",
    "Http2Settings",
    "Http3Settings",
    "ProtocolFallback",
    # Middleware classes
    "Middleware",
    "LoggingMiddleware",
    "HeadersMiddleware",
    "RetryMiddleware",
    "MetricsMiddleware",
    "InterceptorMiddleware",
    # Rate limiting
    "RateLimitConfig",
    "RateLimitAlgorithm",
    "RateLimitMiddleware",
    # Benchmarking
    "Benchmark",
    "MemoryProfiler",
    # Convenience functions
    "get",
    "post",
    "put",
    "delete",
    "patch",
    "head",
    "options",
]
