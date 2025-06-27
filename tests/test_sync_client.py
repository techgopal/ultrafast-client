"""
Comprehensive Test Suite for Synchronous HttpClient

Tests all features and functionality of the HttpClient including:
- HTTP methods (GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS)
- Authentication (Basic, Bearer, OAuth2, API Key)
- Request/Response handling (JSON, form data, multipart, files)
- Headers and parameters
- Configuration (timeouts, retries, SSL, compression, protocols)
- Error handling and edge cases
- Performance features
"""

import pytest
import ultrafast_client as uf
import json
import tempfile
import os
from typing import Dict, Any


class TestHttpClientBasicRequests:
    """Test basic HTTP request methods"""
    
    @pytest.fixture
    def client(self):
        """Create a basic HttpClient for testing"""
        return uf.HttpClient(timeout=30.0)
    
    @pytest.fixture
    def test_url(self):
        """Base URL for testing"""
        return "https://httpbin.org"
    
    def test_get_request(self, client, test_url):
        """Test GET request"""
        response = client.get(f"{test_url}/get")
        assert response.status_code == 200
        data = response.json()
        assert "url" in data
        assert "headers" in data
        
    def test_get_with_params(self, client, test_url):
        """Test GET request with query parameters"""
        params = {"key1": "value1", "key2": "value2"}
        response = client.get(f"{test_url}/get", params=params)
        assert response.status_code == 200
        data = response.json()
        assert data["args"]["key1"] == "value1"
        assert data["args"]["key2"] == "value2"
        
    def test_get_with_headers(self, client, test_url):
        """Test GET request with custom headers"""
        headers = {"X-Custom-Header": "custom-value", "User-Agent": "UltraFast-Test"}
        response = client.get(f"{test_url}/get", headers=headers)
        assert response.status_code == 200
        data = response.json()
        assert data["headers"]["X-Custom-Header"] == "custom-value"
        assert data["headers"]["User-Agent"] == "UltraFast-Test"
        
    def test_post_request_json(self, client, test_url):
        """Test POST request with JSON data"""
        payload = {"name": "test", "value": 123, "active": True}
        response = client.post(f"{test_url}/post", json=payload)
        assert response.status_code == 200
        data = response.json()
        assert data["json"] == payload
        assert "application/json" in data["headers"]["Content-Type"]
        
    def test_post_request_form_data(self, client, test_url):
        """Test POST request with form data"""
        form_data = {"username": "testuser", "password": "testpass"}
        response = client.post(f"{test_url}/post", data=form_data)
        assert response.status_code == 200
        data = response.json()
        assert data["form"]["username"] == "testuser"
        assert data["form"]["password"] == "testpass"
        
    def test_post_request_multipart_files(self, client, test_url):
        """Test POST request with file upload"""
        # Create a temporary file
        with tempfile.NamedTemporaryFile(mode='w', delete=False, suffix='.txt') as tmp_file:
            tmp_file.write("This is test file content")
            tmp_file_path = tmp_file.name
        
        try:
            with open(tmp_file_path, 'rb') as f:
                files = {"file": f.read()}
                form_data = {"description": "Test file upload"}
                
                response = client.post(f"{test_url}/post", data=form_data, files=files)
                assert response.status_code == 200
                data = response.json()
                assert data["form"]["description"] == "Test file upload"
                assert "file" in data["files"]
        finally:
            os.unlink(tmp_file_path)
            
    def test_put_request(self, client, test_url):
        """Test PUT request"""
        payload = {"name": "updated", "value": 456}
        response = client.put(f"{test_url}/put", json=payload)
        assert response.status_code == 200
        data = response.json()
        assert data["json"] == payload
        
    def test_patch_request(self, client, test_url):
        """Test PATCH request"""
        payload = {"status": "active"}
        response = client.patch(f"{test_url}/patch", json=payload)
        assert response.status_code == 200
        data = response.json()
        assert data["json"] == payload
        
    def test_delete_request(self, client, test_url):
        """Test DELETE request"""
        response = client.delete(f"{test_url}/delete")
        assert response.status_code == 200
        data = response.json()
        assert "url" in data
        
    def test_head_request(self, client, test_url):
        """Test HEAD request"""
        response = client.head(f"{test_url}/get")
        assert response.status_code == 200
        # HEAD requests should not return body content
        assert len(response.text()) == 0 or response.text() == ""
        
    def test_options_request(self, client, test_url):
        """Test OPTIONS request"""
        response = client.options(f"{test_url}/get")
        assert response.status_code == 200


class TestHttpClientAuthentication:
    """Test authentication methods"""
    
    @pytest.fixture
    def test_url(self):
        return "https://httpbin.org"
        
    def test_basic_auth(self, test_url):
        """Test Basic Authentication"""
        auth_config = uf.AuthConfig.basic("testuser", "testpass")
        client = uf.HttpClient(auth_config=auth_config)
        
        response = client.get(f"{test_url}/basic-auth/testuser/testpass")
        assert response.status_code == 200
        data = response.json()
        assert data["authenticated"] == True
        assert data["user"] == "testuser"
        
    def test_bearer_token_auth(self, test_url):
        """Test Bearer Token Authentication"""
        auth_config = uf.AuthConfig.bearer("test-token-123")
        client = uf.HttpClient(auth_config=auth_config)
        
        response = client.get(f"{test_url}/bearer")
        assert response.status_code == 200
        data = response.json()
        assert data["authenticated"] == True
        assert data["token"] == "test-token-123"
        
    def test_api_key_auth(self, test_url):
        """Test API Key Authentication via headers"""
        client = uf.HttpClient()
        headers = {"X-API-Key": "api-key-123"}
        
        response = client.get(f"{test_url}/headers", headers=headers)
        assert response.status_code == 200
        data = response.json()
        assert data["headers"]["X-Api-Key"] == "api-key-123"
        
    def test_oauth2_auth(self, test_url):
        """Test OAuth2 Authentication setup"""
        # Test OAuth2 configuration creation
        oauth2_config = uf.AuthConfig.oauth2(
            client_id="test-client-id",
            client_secret="test-client-secret",
            token_url="https://auth.example.com/token",
            scope="read write"
        )
        
        client = uf.HttpClient(auth_config=oauth2_config)
        assert client.has_auth() == True
        
        auth = client.get_auth()
        assert auth is not None
        assert auth.auth_type == uf.AuthType.OAuth2
        
    def test_auth_configuration_methods(self):
        """Test authentication configuration methods"""
        client = uf.HttpClient()
        
        # Initially no auth
        assert client.has_auth() == False
        assert client.get_auth() is None
        
        # Set basic auth
        basic_auth = uf.AuthConfig.basic("user", "pass")
        client.set_auth(basic_auth)
        assert client.has_auth() == True
        assert client.get_auth().auth_type == uf.AuthType.Basic
        
        # Clear auth
        client.clear_auth()
        assert client.has_auth() == False
        assert client.get_auth() is None


class TestHttpClientConfiguration:
    """Test client configuration options"""
    
    def test_timeout_configuration(self):
        """Test timeout configuration"""
        timeout_config = uf.TimeoutConfig(
            connect_timeout=5.0,
            read_timeout=10.0,
            write_timeout=8.0,
            total_timeout=20.0
        )
        
        client = uf.HttpClient(timeout_config=timeout_config)
        assert client is not None
        
    def test_retry_configuration(self):
        """Test retry configuration"""
        retry_config = uf.RetryConfig(
            max_retries=3,
            initial_delay=1.0,
            max_delay=10.0,
            backoff_factor=2.0
        )
        
        client = uf.HttpClient(retry_config=retry_config)
        assert client is not None
        
        # Test setting retry config after creation
        new_retry_config = uf.RetryConfig(max_retries=5, initial_delay=0.5)
        client.set_retry_config(new_retry_config)
        
    def test_ssl_configuration(self):
        """Test SSL configuration"""
        ssl_config = uf.SSLConfig(verify=True)
        client = uf.HttpClient(ssl_config=ssl_config)
        assert client is not None
        
        # Test disabling SSL verification
        ssl_config_no_verify = uf.SSLConfig(verify=False)
        client.set_ssl_config(ssl_config_no_verify)
        
    def test_compression_configuration(self):
        """Test compression configuration"""
        compression_config = uf.CompressionConfig(
            enable_response_compression=True,
            enable_request_compression=True
        )
        
        client = uf.HttpClient(compression_config=compression_config)
        assert client is not None
        
        # Test different compression algorithms
        gzip_config = uf.CompressionConfig.gzip_only()
        client.set_compression_config(gzip_config)
        
        all_algorithms_config = uf.CompressionConfig.all_algorithms()
        client.set_compression_config(all_algorithms_config)
        
    def test_protocol_configuration(self):
        """Test protocol configuration"""
        protocol_config = uf.ProtocolConfig(
            preferred_version=uf.HttpVersion.Http2,
            enable_http2=True,
            enable_http3=False,
            fallback_strategy=uf.ProtocolFallback.Http2ToHttp1
        )
        
        client = uf.HttpClient(protocol_config=protocol_config)
        assert client is not None
        assert client.is_http2_enabled() == True
        assert client.is_http3_enabled() == False
        
        # Test HTTP/3 configuration
        http3_config = uf.ProtocolConfig(
            preferred_version=uf.HttpVersion.Http3,
            enable_http3=True,
            fallback_strategy=uf.ProtocolFallback.Http3ToHttp2ToHttp1
        )
        client.set_protocol_config(http3_config)
        assert client.is_http3_enabled() == True
        
    def test_pool_configuration(self):
        """Test connection pool configuration"""
        pool_config = uf.PoolConfig(
            max_idle_connections=20,
            max_idle_per_host=10,
            idle_timeout=60.0
        )
        
        client = uf.HttpClient(pool_config=pool_config)
        assert client is not None
        
    def test_rate_limit_configuration(self):
        """Test rate limiting configuration"""
        rate_limit_config = uf.RateLimitConfig(
            enabled=True,
            requests_per_second=10,
            burst_size=20,
            algorithm=uf.RateLimitAlgorithm.TokenBucket
        )
        
        client = uf.HttpClient(rate_limit_config=rate_limit_config)
        assert client is not None
        assert client.is_rate_limit_enabled() == True
        
        rate_limit = client.get_rate_limit_config()
        assert rate_limit is not None
        assert rate_limit.requests_per_second == 10


class TestHttpClientHeaders:
    """Test header management"""
    
    def test_set_and_get_headers(self):
        """Test setting and getting headers"""
        client = uf.HttpClient()
        
        # Set header
        client.set_header("X-Custom", "value1")
        
        # Get headers
        headers = client.get_headers()
        assert "X-Custom" in headers
        assert headers["X-Custom"] == "value1"
        
    def test_global_headers(self):
        """Test global headers applied to all requests"""
        headers = {"X-Global": "global-value", "User-Agent": "UltraFast-Global"}
        client = uf.HttpClient(headers=headers)
        
        response = client.get("https://httpbin.org/headers")
        assert response.status_code == 200
        data = response.json()
        assert data["headers"]["X-Global"] == "global-value"
        assert data["headers"]["User-Agent"] == "UltraFast-Global"


class TestHttpClientBaseUrl:
    """Test base URL functionality"""
    
    def test_base_url_configuration(self):
        """Test base URL configuration"""
        client = uf.HttpClient(base_url="https://httpbin.org")
        
        # Relative URL should use base URL
        response = client.get("/get")
        assert response.status_code == 200
        data = response.json()
        assert "httpbin.org" in data["url"]
        
    def test_base_url_override(self):
        """Test overriding base URL with absolute URL"""
        client = uf.HttpClient(base_url="https://httpbin.org")
        
        # Absolute URL should override base URL
        response = client.get("https://httpbin.org/ip")
        assert response.status_code == 200
        data = response.json()
        assert "origin" in data


class TestHttpClientResponse:
    """Test response handling"""
    
    @pytest.fixture
    def client(self):
        return uf.HttpClient()
        
    @pytest.fixture
    def test_url(self):
        return "https://httpbin.org"
        
    def test_response_properties(self, client, test_url):
        """Test response object properties"""
        response = client.get(f"{test_url}/get")
        
        assert response.status_code == 200
        assert response.ok() == True
        assert response.url() is not None
        assert len(response.headers()) > 0
        
    def test_response_text(self, client, test_url):
        """Test response text content"""
        response = client.get(f"{test_url}/get")
        text = response.text()
        assert isinstance(text, str)
        assert len(text) > 0
        
    def test_response_json(self, client, test_url):
        """Test response JSON parsing"""
        response = client.get(f"{test_url}/json")
        json_data = response.json()
        assert isinstance(json_data, dict)
        
    def test_response_bytes(self, client, test_url):
        """Test response binary content"""
        response = client.get(f"{test_url}/bytes/1024")
        content = response.content()
        assert isinstance(content, bytes)
        assert len(content) == 1024


class TestHttpClientErrorHandling:
    """Test error handling and edge cases"""
    
    def test_invalid_url(self):
        """Test handling of invalid URLs"""
        client = uf.HttpClient(timeout=5.0)
        
        with pytest.raises(Exception):
            client.get("invalid-url-format")
            
    def test_timeout_error(self):
        """Test timeout handling"""
        client = uf.HttpClient(timeout=1.0)  # Very short timeout
        
        with pytest.raises(Exception):
            # This should timeout
            client.get("https://httpbin.org/delay/5")
            
    def test_network_error(self):
        """Test network error handling"""
        client = uf.HttpClient(timeout=5.0)
        
        with pytest.raises(Exception):
            client.get("https://non-existent-domain-12345.com")
            
    def test_http_error_codes(self):
        """Test handling of HTTP error status codes"""
        client = uf.HttpClient()
        
        # Test 404
        response = client.get("https://httpbin.org/status/404")
        assert response.status_code == 404
        assert not response.ok()
        
        # Test 500
        response = client.get("https://httpbin.org/status/500")
        assert response.status_code == 500
        assert not response.ok()


class TestHttpClientPerformance:
    """Test performance features"""
    
    def test_get_stats(self):
        """Test client statistics"""
        client = uf.HttpClient()
        
        # Make a request to generate stats
        client.get("https://httpbin.org/get")
        
        stats = client.get_stats()
        assert isinstance(stats, dict)
        # Stats should contain performance metrics
        
    def test_protocol_stats(self):
        """Test protocol statistics"""
        client = uf.HttpClient()
        
        stats = client.get_protocol_stats("https://httpbin.org")
        assert isinstance(stats, dict)
        
    def test_http3_support(self):
        """Test HTTP/3 support detection"""
        client = uf.HttpClient()
        
        supports_http3 = client.supports_http3()
        assert isinstance(supports_http3, bool)


class TestHttpClientMiddleware:
    """Test middleware functionality"""
    
    def test_logging_middleware(self):
        """Test logging middleware"""
        client = uf.HttpClient()
        
        # Add logging middleware
        logging_middleware = uf.LoggingMiddleware(
            name="test_logger",
            log_requests=True,
            log_responses=True,
            log_request_body=True,
            log_response_body=True
        )
        
        client.add_middleware(logging_middleware)
        
        # Make request with middleware
        response = client.get("https://httpbin.org/get")
        assert response.status_code == 200
        
    def test_headers_middleware(self):
        """Test headers middleware"""
        client = uf.HttpClient()
        
        # Add headers middleware
        headers_middleware = uf.HeadersMiddleware(
            name="test_headers",
            headers={"X-Middleware": "test"}
        )
        
        client.add_middleware(headers_middleware)
        
        # Make request with middleware
        response = client.get("https://httpbin.org/headers")
        assert response.status_code == 200
        data = response.json()
        assert data["headers"]["X-Middleware"] == "test"
        
    def test_rate_limit_middleware(self):
        """Test rate limiting middleware"""
        client = uf.HttpClient()
        
        # Add rate limit middleware
        rate_limit_middleware = uf.RateLimitMiddleware(
            name="test_rate_limit",
            requests_per_second=10,
            burst_size=20
        )
        
        client.add_middleware(rate_limit_middleware)
        
        # Make request with middleware
        response = client.get("https://httpbin.org/get")
        assert response.status_code == 200


class TestHttpClientContextManager:
    """Test context manager functionality"""
    
    def test_context_manager(self):
        """Test client as context manager"""
        with uf.HttpClient() as client:
            response = client.get("https://httpbin.org/get")
            assert response.status_code == 200 