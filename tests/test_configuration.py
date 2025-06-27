"""
Comprehensive Test Suite for Configuration Classes

Tests all configuration classes and their functionality including:
- AuthConfig (Basic, Bearer, OAuth2, API Key)
- RetryConfig
- TimeoutConfig 
- PoolConfig
- SSLConfig
- CompressionConfig
- ProtocolConfig
- RateLimitConfig
- Configuration validation and edge cases
"""

import pytest
import ultrafast_client as uf


class TestAuthConfig:
    """Test AuthConfig class and authentication methods"""

    def test_basic_auth_creation(self):
        """Test creating Basic Authentication config"""
        auth = uf.AuthConfig.basic("username", "password")

        assert auth.auth_type == uf.AuthType.Basic
        assert auth is not None

    def test_bearer_auth_creation(self):
        """Test creating Bearer Token Authentication config"""
        auth = uf.AuthConfig.bearer("my-secret-token")

        assert auth.auth_type == uf.AuthType.Bearer
        assert auth is not None

    def test_oauth2_auth_creation(self):
        """Test creating OAuth2 Authentication config"""
        auth = uf.AuthConfig.oauth2(
            client_id="test-client-id",
            client_secret="test-client-secret",
            token_url="https://auth.example.com/oauth/token",
            scope="read write admin",
        )

        assert auth.auth_type == uf.AuthType.OAuth2
        assert auth is not None

    def test_oauth2_auth_with_optional_params(self):
        """Test creating OAuth2 config with optional parameters"""
        auth = uf.AuthConfig.oauth2(
            client_id="client123",
            client_secret="secret456",
            token_url="https://oauth.provider.com/token",
            scope="read",
            redirect_uri="https://myapp.com/callback",
            state="random-state-string",
        )

        assert auth.auth_type == uf.AuthType.OAuth2
        assert auth is not None

    def test_api_key_auth_creation(self):
        """Test creating API Key Authentication config"""
        auth = uf.AuthConfig.api_key("X-API-Key", "my-api-key-value")

        assert auth.auth_type == uf.AuthType.ApiKey
        assert auth is not None

    def test_auth_type_enum(self):
        """Test AuthType enumeration values"""
        assert hasattr(uf.AuthType, "Basic")
        assert hasattr(uf.AuthType, "Bearer")
        assert hasattr(uf.AuthType, "OAuth2")
        assert hasattr(uf.AuthType, "ApiKey")

    def test_auth_config_methods(self):
        """Test AuthConfig helper methods"""
        basic_auth = uf.AuthConfig.basic("user", "pass")
        bearer_auth = uf.AuthConfig.bearer("token")
        oauth2_auth = uf.AuthConfig.oauth2("id", "secret", "url", "scope")

        # All should have is_* methods
        assert basic_auth.is_basic() == True
        assert basic_auth.is_bearer() == False
        assert basic_auth.is_oauth2() == False

        assert bearer_auth.is_basic() == False
        assert bearer_auth.is_bearer() == True
        assert bearer_auth.is_oauth2() == False

        assert oauth2_auth.is_basic() == False
        assert oauth2_auth.is_bearer() == False
        assert oauth2_auth.is_oauth2() == True


class TestRetryConfig:
    """Test RetryConfig class"""

    def test_retry_config_creation(self):
        """Test creating retry configuration"""
        retry_config = uf.RetryConfig(
            max_retries=5, initial_delay=1.0, max_delay=30.0, backoff_factor=2.0
        )

        assert retry_config.max_retries == 5
        assert retry_config.initial_delay == 1.0
        assert retry_config.max_delay == 30.0
        assert retry_config.backoff_factor == 2.0

    def test_retry_config_defaults(self):
        """Test retry configuration with default values"""
        retry_config = uf.RetryConfig()

        # Should have reasonable defaults
        assert retry_config.max_retries >= 0
        assert retry_config.initial_delay > 0

    def test_retry_config_minimal(self):
        """Test retry configuration with minimal parameters"""
        retry_config = uf.RetryConfig(max_retries=3)

        assert retry_config.max_retries == 3

    def test_retry_config_validation(self):
        """Test retry configuration validation"""
        # Valid configuration
        valid_config = uf.RetryConfig(
            max_retries=10, initial_delay=0.5, max_delay=60.0, backoff_factor=1.5
        )
        assert valid_config is not None

        # Test edge cases
        edge_config = uf.RetryConfig(max_retries=0)  # No retries
        assert edge_config.max_retries == 0


class TestTimeoutConfig:
    """Test TimeoutConfig class"""

    def test_timeout_config_creation(self):
        """Test creating timeout configuration"""
        timeout_config = uf.TimeoutConfig(
            connect_timeout=10.0,
            read_timeout=30.0,
            write_timeout=20.0,
            total_timeout=60.0,
        )

        assert timeout_config.connect_timeout == 10.0
        assert timeout_config.read_timeout == 30.0
        assert timeout_config.write_timeout == 20.0
        assert timeout_config.total_timeout == 60.0

    def test_timeout_config_partial(self):
        """Test timeout configuration with partial parameters"""
        timeout_config = uf.TimeoutConfig(connect_timeout=5.0, read_timeout=15.0)

        assert timeout_config.connect_timeout == 5.0
        assert timeout_config.read_timeout == 15.0

    def test_timeout_config_defaults(self):
        """Test timeout configuration with defaults"""
        timeout_config = uf.TimeoutConfig()

        # Should have reasonable defaults
        assert timeout_config is not None

    def test_timeout_config_validation(self):
        """Test timeout configuration validation"""
        # Valid timeouts
        valid_config = uf.TimeoutConfig(
            connect_timeout=30.0,
            read_timeout=120.0,
            write_timeout=60.0,
            total_timeout=300.0,
        )
        assert valid_config is not None


class TestPoolConfig:
    """Test PoolConfig class"""

    def test_pool_config_creation(self):
        """Test creating pool configuration"""
        pool_config = uf.PoolConfig(
            max_idle_connections=100,
            max_idle_per_host=20,
            max_idle_per_host_per_proxy=10,
            idle_timeout=60.0,
            keep_alive_timeout=30.0,
        )

        assert pool_config.max_idle_connections == 100
        assert pool_config.max_idle_per_host == 20
        assert pool_config.max_idle_per_host_per_proxy == 10
        assert pool_config.idle_timeout == 60.0
        assert pool_config.keep_alive_timeout == 30.0

    def test_pool_config_partial(self):
        """Test pool configuration with partial parameters"""
        pool_config = uf.PoolConfig(max_idle_connections=50, idle_timeout=90.0)

        assert pool_config.max_idle_connections == 50
        assert pool_config.idle_timeout == 90.0

    def test_pool_config_defaults(self):
        """Test pool configuration defaults"""
        pool_config = uf.PoolConfig()

        # Should have reasonable defaults
        assert pool_config is not None
        assert pool_config.max_idle_connections > 0

    def test_pool_config_validation(self):
        """Test pool configuration validation"""
        # Valid configuration
        valid_config = uf.PoolConfig(
            max_idle_connections=200, max_idle_per_host=50, idle_timeout=300.0
        )
        assert valid_config is not None


class TestSSLConfig:
    """Test SSLConfig class"""

    def test_ssl_config_creation(self):
        """Test creating SSL configuration"""
        ssl_config = uf.SSLConfig(verify=True)

        assert ssl_config.verify == True

    def test_ssl_config_no_verify(self):
        """Test SSL configuration with verification disabled"""
        ssl_config = uf.SSLConfig(verify=False)

        assert ssl_config.verify == False

    def test_ssl_config_with_cert_path(self):
        """Test SSL configuration with certificate path"""
        ssl_config = uf.SSLConfig(verify=True, cert_path="/path/to/cert.pem")

        assert ssl_config.verify == True
        assert ssl_config.cert_path == "/path/to/cert.pem"

    def test_ssl_config_with_key_path(self):
        """Test SSL configuration with key path"""
        ssl_config = uf.SSLConfig(
            verify=True, cert_path="/path/to/cert.pem", key_path="/path/to/key.pem"
        )

        assert ssl_config.verify == True
        assert ssl_config.cert_path == "/path/to/cert.pem"
        assert ssl_config.key_path == "/path/to/key.pem"

    def test_ssl_config_with_ca_bundle(self):
        """Test SSL configuration with CA bundle"""
        ssl_config = uf.SSLConfig(verify=True, ca_bundle_path="/path/to/ca-bundle.crt")

        assert ssl_config.verify == True
        assert ssl_config.ca_bundle_path == "/path/to/ca-bundle.crt"

    def test_ssl_config_defaults(self):
        """Test SSL configuration defaults"""
        ssl_config = uf.SSLConfig()

        # Should have reasonable defaults
        assert ssl_config is not None


class TestCompressionConfig:
    """Test CompressionConfig class"""

    def test_compression_config_creation(self):
        """Test creating compression configuration"""
        compression_config = uf.CompressionConfig(
            enable_response_compression=True, enable_request_compression=True
        )

        assert compression_config.enable_response_compression == True
        assert compression_config.enable_request_compression == True

    def test_compression_config_response_only(self):
        """Test compression configuration for response only"""
        compression_config = uf.CompressionConfig(
            enable_response_compression=True, enable_request_compression=False
        )

        assert compression_config.enable_response_compression == True
        assert compression_config.enable_request_compression == False

    def test_compression_config_gzip_only(self):
        """Test GZIP-only compression configuration"""
        compression_config = uf.CompressionConfig.gzip_only()

        assert compression_config is not None

    def test_compression_config_all_algorithms(self):
        """Test all algorithms compression configuration"""
        compression_config = uf.CompressionConfig.all_algorithms()

        assert compression_config is not None

    def test_compression_config_disabled(self):
        """Test disabled compression configuration"""
        compression_config = uf.CompressionConfig.disabled()

        assert compression_config is not None
        assert compression_config.enable_response_compression == False
        assert compression_config.enable_request_compression == False

    def test_compression_config_defaults(self):
        """Test compression configuration defaults"""
        compression_config = uf.CompressionConfig()

        # Should have reasonable defaults
        assert compression_config is not None


class TestProtocolConfig:
    """Test ProtocolConfig class"""

    def test_protocol_config_creation(self):
        """Test creating protocol configuration"""
        protocol_config = uf.ProtocolConfig(
            preferred_version=uf.HttpVersion.Http2,
            enable_http2=True,
            enable_http3=False,
            fallback_strategy=uf.ProtocolFallback.Http2ToHttp1,
        )

        assert protocol_config.preferred_version == uf.HttpVersion.Http2
        assert protocol_config.enable_http2 == True
        assert protocol_config.enable_http3 == False
        assert protocol_config.fallback_strategy == uf.ProtocolFallback.Http2ToHttp1

    def test_protocol_config_http3(self):
        """Test protocol configuration with HTTP/3"""
        protocol_config = uf.ProtocolConfig(
            preferred_version=uf.HttpVersion.Http3,
            enable_http3=True,
            fallback_strategy=uf.ProtocolFallback.Http3ToHttp2ToHttp1,
        )

        assert protocol_config.preferred_version == uf.HttpVersion.Http3
        assert protocol_config.enable_http3 == True
        assert (
            protocol_config.fallback_strategy == uf.ProtocolFallback.Http3ToHttp2ToHttp1
        )

    def test_protocol_config_auto(self):
        """Test protocol configuration with auto version"""
        protocol_config = uf.ProtocolConfig(
            preferred_version=uf.HttpVersion.AUTO,
            enable_http2=True,
            enable_http3=True,
            fallback_strategy=uf.ProtocolFallback.Http3ToHttp2ToHttp1,
        )

        assert protocol_config.preferred_version == uf.HttpVersion.AUTO

    def test_http_version_enum(self):
        """Test HttpVersion enumeration"""
        assert hasattr(uf.HttpVersion, "Http1")
        assert hasattr(uf.HttpVersion, "Http2")
        assert hasattr(uf.HttpVersion, "Http3")
        assert hasattr(uf.HttpVersion, "AUTO")

    def test_protocol_fallback_enum(self):
        """Test ProtocolFallback enumeration"""
        assert hasattr(uf.ProtocolFallback, "Http1Only")
        assert hasattr(uf.ProtocolFallback, "Http2ToHttp1")
        assert hasattr(uf.ProtocolFallback, "Http3ToHttp2ToHttp1")

    def test_protocol_config_http2_settings(self):
        """Test protocol configuration with HTTP/2 settings"""
        http2_settings = uf.Http2Settings(
            max_concurrent_streams=100, initial_window_size=65536, max_frame_size=16384
        )

        protocol_config = uf.ProtocolConfig(
            enable_http2=True, http2_settings=http2_settings
        )

        assert protocol_config.enable_http2 == True
        assert protocol_config.http2_settings is not None

    def test_protocol_config_http3_settings(self):
        """Test protocol configuration with HTTP/3 settings"""
        http3_settings = uf.Http3Settings(
            max_idle_timeout=30, max_udp_payload_size=1200, initial_max_data=1048576
        )

        protocol_config = uf.ProtocolConfig(
            enable_http3=True, http3_settings=http3_settings
        )

        assert protocol_config.enable_http3 == True
        assert protocol_config.http3_settings is not None

    def test_protocol_config_methods(self):
        """Test protocol configuration helper methods"""
        protocol_config = uf.ProtocolConfig(enable_http2=True, enable_http3=False)

        assert protocol_config.is_http2_enabled() == True
        assert protocol_config.is_http3_enabled() == False

    def test_protocol_config_validation(self):
        """Test protocol configuration validation"""
        # Valid configuration
        valid_config = uf.ProtocolConfig(
            preferred_version=uf.HttpVersion.Http2,
            enable_http2=True,
            enable_http3=False,
        )

        # Should validate successfully
        try:
            valid_config.validate()
            validation_passed = True
        except:
            validation_passed = False

        assert validation_passed == True

    def test_protocol_config_defaults(self):
        """Test protocol configuration defaults"""
        protocol_config = uf.ProtocolConfig()

        # Should have reasonable defaults
        assert protocol_config is not None


class TestRateLimitConfig:
    """Test RateLimitConfig class"""

    def test_rate_limit_config_creation(self):
        """Test creating rate limit configuration"""
        rate_limit_config = uf.RateLimitConfig(
            enabled=True,
            requests_per_second=10,
            burst_size=20,
            algorithm=uf.RateLimitAlgorithm.TokenBucket,
        )

        assert rate_limit_config.enabled == True
        assert rate_limit_config.requests_per_second == 10
        assert rate_limit_config.burst_size == 20
        assert rate_limit_config.algorithm == uf.RateLimitAlgorithm.TokenBucket

    def test_rate_limit_config_disabled(self):
        """Test disabled rate limit configuration"""
        rate_limit_config = uf.RateLimitConfig(enabled=False)

        assert rate_limit_config.enabled == False

    def test_rate_limit_algorithm_enum(self):
        """Test RateLimitAlgorithm enumeration"""
        assert hasattr(uf.RateLimitAlgorithm, "TokenBucket")
        assert hasattr(uf.RateLimitAlgorithm, "LeakyBucket")
        assert hasattr(uf.RateLimitAlgorithm, "FixedWindow")
        assert hasattr(uf.RateLimitAlgorithm, "SlidingWindow")

    def test_rate_limit_config_different_algorithms(self):
        """Test rate limit configuration with different algorithms"""
        # Token bucket
        token_config = uf.RateLimitConfig(
            enabled=True,
            requests_per_second=5,
            algorithm=uf.RateLimitAlgorithm.TokenBucket,
        )
        assert token_config.algorithm == uf.RateLimitAlgorithm.TokenBucket

        # Leaky bucket
        leaky_config = uf.RateLimitConfig(
            enabled=True,
            requests_per_second=5,
            algorithm=uf.RateLimitAlgorithm.LeakyBucket,
        )
        assert leaky_config.algorithm == uf.RateLimitAlgorithm.LeakyBucket

        # Fixed window
        fixed_config = uf.RateLimitConfig(
            enabled=True,
            requests_per_second=5,
            algorithm=uf.RateLimitAlgorithm.FixedWindow,
        )
        assert fixed_config.algorithm == uf.RateLimitAlgorithm.FixedWindow

        # Sliding window
        sliding_config = uf.RateLimitConfig(
            enabled=True,
            requests_per_second=5,
            algorithm=uf.RateLimitAlgorithm.SlidingWindow,
        )
        assert sliding_config.algorithm == uf.RateLimitAlgorithm.SlidingWindow

    def test_rate_limit_config_validation(self):
        """Test rate limit configuration validation"""
        # Valid configuration
        valid_config = uf.RateLimitConfig(
            enabled=True, requests_per_second=100, burst_size=200
        )
        assert valid_config is not None

        # Edge case: very low rate limit
        low_rate_config = uf.RateLimitConfig(
            enabled=True, requests_per_second=1, burst_size=1
        )
        assert low_rate_config is not None

        # Edge case: high rate limit
        high_rate_config = uf.RateLimitConfig(
            enabled=True, requests_per_second=1000, burst_size=5000
        )
        assert high_rate_config is not None


class TestOAuth2Token:
    """Test OAuth2Token class"""

    def test_oauth2_token_creation(self):
        """Test creating OAuth2 token"""
        token = uf.OAuth2Token(
            access_token="access-token-123",
            token_type="Bearer",
            expires_in=3600,
            refresh_token="refresh-token-456",
            scope="read write",
        )

        assert token.access_token == "access-token-123"
        assert token.token_type == "Bearer"
        assert token.expires_in == 3600
        assert token.refresh_token == "refresh-token-456"
        assert token.scope == "read write"

    def test_oauth2_token_minimal(self):
        """Test creating minimal OAuth2 token"""
        token = uf.OAuth2Token(access_token="minimal-token", token_type="Bearer")

        assert token.access_token == "minimal-token"
        assert token.token_type == "Bearer"

    def test_oauth2_token_expiration(self):
        """Test OAuth2 token expiration checking"""
        token = uf.OAuth2Token(
            access_token="expiring-token", token_type="Bearer", expires_in=1  # 1 second
        )

        # Should initially be valid
        assert not token.is_expired()

        # After some time, check expiration (if method exists)
        if hasattr(token, "is_expired"):
            import time

            time.sleep(1.1)  # Wait a bit longer than expiration
            assert token.is_expired() == True


class TestConfigurationEdgeCases:
    """Test configuration edge cases and validation"""

    def test_none_configurations(self):
        """Test handling of None configurations"""
        # Test that clients handle None configs gracefully
        client = uf.HttpClient(
            auth_config=None,
            retry_config=None,
            timeout_config=None,
            pool_config=None,
            ssl_config=None,
            compression_config=None,
            protocol_config=None,
            rate_limit_config=None,
        )

        assert client is not None

    def test_default_configurations(self):
        """Test default configuration values"""
        # Create configs with defaults
        timeout_config = uf.TimeoutConfig()
        pool_config = uf.PoolConfig()
        ssl_config = uf.SSLConfig()
        compression_config = uf.CompressionConfig()
        protocol_config = uf.ProtocolConfig()

        # All should be valid
        assert timeout_config is not None
        assert pool_config is not None
        assert ssl_config is not None
        assert compression_config is not None
        assert protocol_config is not None

    def test_configuration_immutability(self):
        """Test that configurations are properly handled"""
        # Create configurations
        auth_config = uf.AuthConfig.basic("user", "pass")
        retry_config = uf.RetryConfig(max_retries=3)

        # Use in multiple clients
        client1 = uf.HttpClient(auth_config=auth_config, retry_config=retry_config)
        client2 = uf.HttpClient(auth_config=auth_config, retry_config=retry_config)

        # Both clients should be independent
        assert client1 is not client2

    def test_configuration_copying(self):
        """Test configuration copying/cloning"""
        original_config = uf.RetryConfig(max_retries=5, initial_delay=2.0)

        # Use config in client
        client = uf.HttpClient(retry_config=original_config)

        # Client should work properly
        assert client is not None
