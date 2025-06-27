"""
Comprehensive Test Suite for Asynchronous AsyncHttpClient

Tests all features and functionality of the AsyncHttpClient including:
- Async HTTP methods (GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS)
- Authentication (Basic, Bearer, OAuth2, API Key)
- Request/Response handling (JSON, form data, multipart, files)
- Headers and parameters
- Configuration (timeouts, retries, SSL, compression, protocols)
- Error handling and edge cases
- Performance features
- Async-specific functionality
"""

import asyncio
import os
import tempfile
from typing import Any, Dict

import pytest
import ultrafast_client as uf


class TestAsyncHttpClientBasicRequests:
    """Test basic async HTTP request methods"""

    @pytest.fixture
    def client(self):
        """Create a basic AsyncHttpClient for testing"""
        return uf.AsyncHttpClient(timeout=30.0)

    @pytest.fixture
    def test_url(self):
        """Base URL for testing"""
        return "https://httpbin.org"

    @pytest.mark.asyncio
    async def test_get_request(self, client, test_url):
        """Test async GET request"""
        response = await client.get(f"{test_url}/get")
        assert response.status_code == 200
        data = response.json()
        assert "url" in data
        assert "headers" in data

    @pytest.mark.asyncio
    async def test_get_with_params(self, client, test_url):
        """Test async GET request with query parameters"""
        params = {"key1": "value1", "key2": "value2"}
        response = await client.get(f"{test_url}/get", params=params)
        assert response.status_code == 200
        data = response.json()
        assert data["args"]["key1"] == "value1"
        assert data["args"]["key2"] == "value2"

    @pytest.mark.asyncio
    async def test_get_with_headers(self, client, test_url):
        """Test async GET request with custom headers"""
        headers = {
            "X-Custom-Header": "custom-value",
            "User-Agent": "UltraFast-Async-Test",
        }
        response = await client.get(f"{test_url}/get", headers=headers)
        assert response.status_code == 200
        data = response.json()
        assert data["headers"]["X-Custom-Header"] == "custom-value"
        assert data["headers"]["User-Agent"] == "UltraFast-Async-Test"

    @pytest.mark.asyncio
    async def test_post_request_json(self, client, test_url):
        """Test async POST request with JSON data"""
        payload = {"name": "async_test", "value": 123, "active": True}
        response = await client.post(f"{test_url}/post", json=payload)
        assert response.status_code == 200
        data = response.json()
        assert data["json"] == payload
        assert "application/json" in data["headers"]["Content-Type"]

    @pytest.mark.asyncio
    async def test_post_request_form_data(self, client, test_url):
        """Test async POST request with form data"""
        form_data = {"username": "async_user", "password": "async_pass"}
        response = await client.post(f"{test_url}/post", data=form_data)
        assert response.status_code == 200
        data = response.json()
        assert data["form"]["username"] == "async_user"
        assert data["form"]["password"] == "async_pass"

    @pytest.mark.asyncio
    async def test_post_request_multipart_files(self, client, test_url):
        """Test async POST request with file upload"""
        # Create a temporary file
        with tempfile.NamedTemporaryFile(
            mode="w", delete=False, suffix=".txt"
        ) as tmp_file:
            tmp_file.write("This is async test file content")
            tmp_file_path = tmp_file.name

        try:
            with open(tmp_file_path, "rb") as f:
                files = {"file": f.read()}
                form_data = {"description": "Async test file upload"}

                response = await client.post(
                    f"{test_url}/post", data=form_data, files=files
                )
                assert response.status_code == 200
                data = response.json()
                assert data["form"]["description"] == "Async test file upload"
                assert "file" in data["files"]
        finally:
            os.unlink(tmp_file_path)

    @pytest.mark.asyncio
    async def test_put_request(self, client, test_url):
        """Test async PUT request"""
        payload = {"name": "async_updated", "value": 456}
        response = await client.put(f"{test_url}/put", json=payload)
        assert response.status_code == 200
        data = response.json()
        assert data["json"] == payload

    @pytest.mark.asyncio
    async def test_patch_request(self, client, test_url):
        """Test async PATCH request"""
        payload = {"status": "async_active"}
        response = await client.patch(f"{test_url}/patch", json=payload)
        assert response.status_code == 200
        data = response.json()
        assert data["json"] == payload

    @pytest.mark.asyncio
    async def test_delete_request(self, client, test_url):
        """Test async DELETE request"""
        response = await client.delete(f"{test_url}/delete")
        assert response.status_code == 200
        data = response.json()
        assert "url" in data

    @pytest.mark.asyncio
    async def test_head_request(self, client, test_url):
        """Test async HEAD request"""
        response = await client.head(f"{test_url}/get")
        assert response.status_code == 200
        # HEAD requests should not return body content
        assert len(response.text()) == 0 or response.text() == ""

    @pytest.mark.asyncio
    async def test_options_request(self, client, test_url):
        """Test async OPTIONS request"""
        response = await client.options(f"{test_url}/get")
        assert response.status_code == 200


class TestAsyncHttpClientAuthentication:
    """Test async authentication methods"""

    @pytest.fixture
    def test_url(self):
        return "https://httpbin.org"

    @pytest.mark.asyncio
    async def test_basic_auth(self, test_url):
        """Test async Basic Authentication"""
        auth_config = uf.AuthConfig.basic("async_user", "async_pass")
        client = uf.AsyncHttpClient(auth_config=auth_config)

        response = await client.get(f"{test_url}/basic-auth/async_user/async_pass")
        assert response.status_code == 200
        data = response.json()
        assert data["authenticated"] == True
        assert data["user"] == "async_user"

    @pytest.mark.asyncio
    async def test_bearer_token_auth(self, test_url):
        """Test async Bearer Token Authentication"""
        auth_config = uf.AuthConfig.bearer("async-token-123")
        client = uf.AsyncHttpClient(auth_config=auth_config)

        response = await client.get(f"{test_url}/bearer")
        assert response.status_code == 200
        data = response.json()
        assert data["authenticated"] == True
        assert data["token"] == "async-token-123"

    @pytest.mark.asyncio
    async def test_api_key_auth(self, test_url):
        """Test async API Key Authentication via headers"""
        client = uf.AsyncHttpClient()
        headers = {"X-API-Key": "async-api-key-123"}

        response = await client.get(f"{test_url}/headers", headers=headers)
        assert response.status_code == 200
        data = response.json()
        assert data["headers"]["X-Api-Key"] == "async-api-key-123"

    @pytest.mark.asyncio
    async def test_oauth2_auth(self, test_url):
        """Test async OAuth2 Authentication setup"""
        # Test OAuth2 configuration creation
        oauth2_config = uf.AuthConfig.oauth2(
            client_id="async-client-id",
            client_secret="async-client-secret",
            token_url="https://auth.example.com/token",
            scope="read write admin",
        )

        client = uf.AsyncHttpClient(auth_config=oauth2_config)
        assert client.has_auth() == True

        auth = client.get_auth()
        assert auth is not None
        assert auth.auth_type == uf.AuthType.OAuth2

    def test_auth_configuration_methods(self):
        """Test async authentication configuration methods"""
        client = uf.AsyncHttpClient()

        # Initially no auth
        assert client.has_auth() == False
        assert client.get_auth() is None

        # Set basic auth
        basic_auth = uf.AuthConfig.basic("async_user", "async_pass")
        client.set_auth(basic_auth)
        assert client.has_auth() == True
        assert client.get_auth().auth_type == uf.AuthType.Basic

        # Clear auth
        client.clear_auth()
        assert client.has_auth() == False
        assert client.get_auth() is None


class TestAsyncHttpClientConfiguration:
    """Test async client configuration options"""

    def test_timeout_configuration(self):
        """Test async timeout configuration"""
        timeout_config = uf.TimeoutConfig(
            connect_timeout=5.0,
            read_timeout=10.0,
            write_timeout=8.0,
            total_timeout=20.0,
        )

        client = uf.AsyncHttpClient(timeout_config=timeout_config)
        assert client is not None

    def test_retry_configuration(self):
        """Test async retry configuration"""
        retry_config = uf.RetryConfig(
            max_retries=3, initial_delay=1.0, max_delay=10.0, backoff_factor=2.0
        )

        client = uf.AsyncHttpClient(retry_config=retry_config)
        assert client is not None

        # Test setting retry config after creation
        new_retry_config = uf.RetryConfig(max_retries=5, initial_delay=0.5)
        client.set_retry_config(new_retry_config)

    def test_ssl_configuration(self):
        """Test async SSL configuration"""
        ssl_config = uf.SSLConfig(verify=True)
        client = uf.AsyncHttpClient(ssl_config=ssl_config)
        assert client is not None

        # Test disabling SSL verification
        ssl_config_no_verify = uf.SSLConfig(verify=False)
        client.set_ssl_config(ssl_config_no_verify)

    def test_compression_configuration(self):
        """Test async compression configuration"""
        compression_config = uf.CompressionConfig(
            enable_response_compression=True, enable_request_compression=True
        )

        client = uf.AsyncHttpClient(compression_config=compression_config)
        assert client is not None

        # Test different compression algorithms
        gzip_config = uf.CompressionConfig.gzip_only()
        client.set_compression_config(gzip_config)

        all_algorithms_config = uf.CompressionConfig.all_algorithms()
        client.set_compression_config(all_algorithms_config)

    def test_protocol_configuration(self):
        """Test async protocol configuration"""
        protocol_config = uf.ProtocolConfig(
            preferred_version=uf.HttpVersion.Http2,
            enable_http2=True,
            enable_http3=False,
            fallback_strategy=uf.ProtocolFallback.Http2ToHttp1,
        )

        client = uf.AsyncHttpClient(protocol_config=protocol_config)
        assert client is not None
        assert client.is_http2_enabled() == True
        assert client.is_http3_enabled() == False

        # Test HTTP/3 configuration
        http3_config = uf.ProtocolConfig(
            preferred_version=uf.HttpVersion.Http3,
            enable_http3=True,
            fallback_strategy=uf.ProtocolFallback.Http3ToHttp2ToHttp1,
        )
        client.set_protocol_config(http3_config)
        assert client.is_http3_enabled() == True

    def test_pool_configuration(self):
        """Test async connection pool configuration"""
        pool_config = uf.PoolConfig(
            max_idle_connections=20, max_idle_per_host=10, idle_timeout=60.0
        )

        client = uf.AsyncHttpClient(pool_config=pool_config)
        assert client is not None

    def test_rate_limit_configuration(self):
        """Test async rate limiting configuration"""
        rate_limit_config = uf.RateLimitConfig(
            enabled=True,
            requests_per_second=10,
            burst_size=20,
            algorithm=uf.RateLimitAlgorithm.TokenBucket,
        )

        client = uf.AsyncHttpClient(rate_limit_config=rate_limit_config)
        assert client is not None
        assert client.is_rate_limit_enabled() == True

        rate_limit = client.get_rate_limit_config_sync()
        assert rate_limit is not None
        assert rate_limit.requests_per_second == 10


class TestAsyncHttpClientHeaders:
    """Test async header management"""

    def test_set_and_get_headers(self):
        """Test async setting and getting headers"""
        client = uf.AsyncHttpClient()

        # Set header
        client.set_header("X-Async-Custom", "async_value1")

        # Get headers
        headers = client.get_headers()
        assert "X-Async-Custom" in headers
        assert headers["X-Async-Custom"] == "async_value1"

    @pytest.mark.asyncio
    async def test_global_headers(self):
        """Test async global headers applied to all requests"""
        headers = {
            "X-Async-Global": "async-global-value",
            "User-Agent": "UltraFast-Async-Global",
        }
        client = uf.AsyncHttpClient(headers=headers)

        response = await client.get("https://httpbin.org/headers")
        assert response.status_code == 200
        data = response.json()
        assert data["headers"]["X-Async-Global"] == "async-global-value"
        assert data["headers"]["User-Agent"] == "UltraFast-Async-Global"


class TestAsyncHttpClientBaseUrl:
    """Test async base URL functionality"""

    @pytest.mark.asyncio
    async def test_base_url_configuration(self):
        """Test async base URL configuration"""
        client = uf.AsyncHttpClient(base_url="https://httpbin.org")

        # Relative URL should use base URL
        response = await client.get("/get")
        assert response.status_code == 200
        data = response.json()
        assert "httpbin.org" in data["url"]

    @pytest.mark.asyncio
    async def test_base_url_override(self):
        """Test async overriding base URL with absolute URL"""
        client = uf.AsyncHttpClient(base_url="https://httpbin.org")

        # Absolute URL should override base URL
        response = await client.get("https://httpbin.org/ip")
        assert response.status_code == 200
        data = response.json()
        assert "origin" in data


class TestAsyncHttpClientResponse:
    """Test async response handling"""

    @pytest.fixture
    def client(self):
        return uf.AsyncHttpClient()

    @pytest.fixture
    def test_url(self):
        return "https://httpbin.org"

    @pytest.mark.asyncio
    async def test_response_properties(self, client, test_url):
        """Test async response object properties"""
        response = await client.get(f"{test_url}/get")

        assert response.status_code == 200
        assert response.ok() == True
        assert response.url() is not None
        assert len(response.headers()) > 0

    @pytest.mark.asyncio
    async def test_response_text(self, client, test_url):
        """Test async response text content"""
        response = await client.get(f"{test_url}/get")
        text = response.text()
        assert isinstance(text, str)
        assert len(text) > 0

    @pytest.mark.asyncio
    async def test_response_json(self, client, test_url):
        """Test async response JSON parsing"""
        response = await client.get(f"{test_url}/json")
        json_data = response.json()
        assert isinstance(json_data, dict)

    @pytest.mark.asyncio
    async def test_response_bytes(self, client, test_url):
        """Test async response binary content"""
        response = await client.get(f"{test_url}/bytes/1024")
        content = response.content()
        assert isinstance(content, bytes)
        assert len(content) == 1024


class TestAsyncHttpClientErrorHandling:
    """Test async error handling and edge cases"""

    @pytest.mark.asyncio
    async def test_invalid_url(self):
        """Test async handling of invalid URLs"""
        client = uf.AsyncHttpClient(timeout=5.0)

        with pytest.raises(Exception):
            await client.get("invalid-url-format")

    @pytest.mark.asyncio
    async def test_timeout_error(self):
        """Test async timeout handling"""
        client = uf.AsyncHttpClient(timeout=1.0)  # Very short timeout

        with pytest.raises(Exception):
            # This should timeout
            await client.get("https://httpbin.org/delay/5")

    @pytest.mark.asyncio
    async def test_network_error(self):
        """Test async network error handling"""
        client = uf.AsyncHttpClient(timeout=5.0)

        with pytest.raises(Exception):
            await client.get("https://non-existent-domain-12345.com")

    @pytest.mark.asyncio
    async def test_http_error_codes(self):
        """Test async handling of HTTP error status codes"""
        client = uf.AsyncHttpClient()

        # Test 404
        response = await client.get("https://httpbin.org/status/404")
        assert response.status_code == 404
        assert not response.ok()

        # Test 500
        response = await client.get("https://httpbin.org/status/500")
        assert response.status_code == 500
        assert not response.ok()


class TestAsyncHttpClientPerformance:
    """Test async performance features"""

    @pytest.mark.asyncio
    async def test_get_stats(self):
        """Test async client statistics"""
        client = uf.AsyncHttpClient()

        # Make a request to generate stats
        await client.get("https://httpbin.org/get")

        stats = client.get_stats_sync()
        assert isinstance(stats, dict)
        # Stats should contain performance metrics

    def test_protocol_stats(self):
        """Test async protocol statistics"""
        client = uf.AsyncHttpClient()

        stats = client.get_protocol_stats("https://httpbin.org")
        assert isinstance(stats, dict)

    def test_http3_support(self):
        """Test async HTTP/3 support detection"""
        client = uf.AsyncHttpClient()

        supports_http3 = client.supports_http3()
        assert isinstance(supports_http3, bool)


class TestAsyncHttpClientMiddleware:
    """Test async middleware functionality"""

    @pytest.mark.asyncio
    async def test_logging_middleware(self):
        """Test async logging middleware"""
        client = uf.AsyncHttpClient()

        # Add logging middleware
        logging_middleware = uf.LoggingMiddleware(
            name="async_test_logger",
            log_requests=True,
            log_responses=True,
            log_request_body=True,
            log_response_body=True,
        )

        await client.add_middleware(logging_middleware)

        # Make request with middleware
        response = await client.get("https://httpbin.org/get")
        assert response.status_code == 200

    @pytest.mark.asyncio
    async def test_headers_middleware(self):
        """Test async headers middleware"""
        client = uf.AsyncHttpClient()

        # Add headers middleware
        headers_middleware = uf.HeadersMiddleware(
            name="async_test_headers", headers={"X-Async-Middleware": "async_test"}
        )

        await client.add_middleware(headers_middleware)

        # Make request with middleware
        response = await client.get("https://httpbin.org/headers")
        assert response.status_code == 200
        data = response.json()
        assert data["headers"]["X-Async-Middleware"] == "async_test"

    @pytest.mark.asyncio
    async def test_rate_limit_middleware(self):
        """Test async rate limiting middleware"""
        client = uf.AsyncHttpClient()

        # Add rate limit middleware
        rate_limit_middleware = uf.RateLimitMiddleware(
            name="async_test_rate_limit", requests_per_second=10, burst_size=20
        )

        await client.add_middleware(rate_limit_middleware)

        # Make request with middleware
        response = await client.get("https://httpbin.org/get")
        assert response.status_code == 200


class TestAsyncHttpClientConcurrency:
    """Test async concurrency features"""

    @pytest.mark.asyncio
    async def test_concurrent_requests(self):
        """Test making multiple concurrent async requests"""
        client = uf.AsyncHttpClient()

        # Make multiple concurrent requests
        tasks = [
            client.get("https://httpbin.org/delay/1"),
            client.get("https://httpbin.org/delay/1"),
            client.get("https://httpbin.org/delay/1"),
        ]

        responses = await asyncio.gather(*tasks)

        # All responses should be successful
        for response in responses:
            assert response.status_code == 200

    @pytest.mark.asyncio
    async def test_async_with_context_manager(self):
        """Test async client with context manager"""
        async with uf.AsyncHttpClient() as client:
            response = await client.get("https://httpbin.org/get")
            assert response.status_code == 200

    @pytest.mark.asyncio
    async def test_async_stream_handling(self):
        """Test async stream handling capabilities"""
        client = uf.AsyncHttpClient()

        # Test streaming response
        response = await client.get("https://httpbin.org/stream/5")
        assert response.status_code == 200

        # Response should contain stream data
        text = response.text()
        assert len(text) > 0


class TestAsyncHttpClientCompatibility:
    """Test async client compatibility with sync features"""

    def test_sync_methods_available(self):
        """Test that sync-compatible methods are available on async client"""
        client = uf.AsyncHttpClient()

        # These methods should work synchronously
        assert hasattr(client, "get_headers")
        assert hasattr(client, "set_header")
        assert hasattr(client, "has_auth")
        assert hasattr(client, "get_auth")
        assert hasattr(client, "clear_auth")
        assert hasattr(client, "supports_http3")
        assert hasattr(client, "is_http2_enabled")
        assert hasattr(client, "is_http3_enabled")

        # Test that these methods work
        headers = client.get_headers()
        assert isinstance(headers, dict)

        supports_http3 = client.supports_http3()
        assert isinstance(supports_http3, bool)

    def test_configuration_parity(self):
        """Test that async client has same configuration options as sync client"""
        # Test that both clients can be configured identically
        auth_config = uf.AuthConfig.basic("user", "pass")
        retry_config = uf.RetryConfig(max_retries=3)
        timeout_config = uf.TimeoutConfig(connect_timeout=10.0)

        sync_client = uf.HttpClient(
            auth_config=auth_config,
            retry_config=retry_config,
            timeout_config=timeout_config,
        )

        async_client = uf.AsyncHttpClient(
            auth_config=auth_config,
            retry_config=retry_config,
            timeout_config=timeout_config,
        )

        assert sync_client is not None
        assert async_client is not None

        # Both should have auth configured
        assert sync_client.has_auth() == True
        assert async_client.has_auth() == True
