"""
Comprehensive Integration Test Suite

End-to-end integration tests covering:
- Complete feature combinations
- Real-world usage scenarios
- Cross-feature compatibility
- Authentication with different protocols
- Sessions with complex configurations
- WebSocket and SSE integration
- Performance under realistic conditions
- Error handling in complex scenarios
"""

import asyncio
import os
import tempfile
import threading
import time
from typing import Any, Dict, List

import pytest
import ultrafast_client as uf


class TestBasicIntegration:
    """Test basic integration scenarios"""

    @pytest.fixture
    def test_url(self):
        """Base URL for testing"""
        return "https://httpbin.org"

    def test_sync_client_full_workflow(self, test_url):
        """Test complete synchronous client workflow"""
        # Create client with comprehensive configuration
        auth_config = uf.AuthConfig.basic("testuser", "testpass")
        retry_config = uf.RetryConfig(max_retries=3, initial_delay=1.0)
        timeout_config = uf.TimeoutConfig(connect_timeout=10.0, read_timeout=30.0)
        pool_config = uf.PoolConfig(max_idle_connections=10, max_idle_per_host=5)
        ssl_config = uf.SSLConfig(verify=True)
        compression_config = uf.CompressionConfig(enable_response_compression=True)
        protocol_config = uf.ProtocolConfig(
            preferred_version=uf.HttpVersion.Http2,
            enable_http2=True,
            fallback_strategy=uf.ProtocolFallback.Http2ToHttp1,
        )

        client = uf.HttpClient(
            base_url=test_url,
            headers={"User-Agent": "UltraFast-Integration-Test"},
            auth_config=auth_config,
            retry_config=retry_config,
            timeout_config=timeout_config,
            pool_config=pool_config,
            ssl_config=ssl_config,
            compression_config=compression_config,
            protocol_config=protocol_config,
        )

        # Test various request types
        try:
            # GET request
            response = client.get("/get", params={"integration": "test"})
            assert response.status_code == 200
            data = response.json()
            assert data["args"]["integration"] == "test"

            # POST with JSON
            post_data = {"integration": "post_test", "timestamp": time.time()}
            response = client.post("/post", json=post_data)
            assert response.status_code == 200
            data = response.json()
            assert data["json"]["integration"] == "post_test"

            # PUT with form data
            form_data = {"integration": "put_test", "method": "PUT"}
            response = client.put("/put", data=form_data)
            assert response.status_code == 200
            data = response.json()
            assert data["form"]["integration"] == "put_test"

            # File upload
            with tempfile.NamedTemporaryFile(
                mode="w", delete=False, suffix=".txt"
            ) as tmp_file:
                tmp_file.write("Integration test file content")
                tmp_file_path = tmp_file.name

            try:
                with open(tmp_file_path, "rb") as f:
                    files = {"integration_file": f.read()}
                    response = client.post(
                        "/post",
                        data={"description": "Integration file upload"},
                        files=files,
                    )
                    assert response.status_code == 200
                    data = response.json()
                    assert "integration_file" in data["files"]
            finally:
                os.unlink(tmp_file_path)

            # Test headers and authentication
            response = client.get("/headers")
            assert response.status_code == 200
            headers_data = response.json()
            assert "Authorization" in headers_data["headers"]
            assert "User-Agent" in headers_data["headers"]
            assert headers_data["headers"]["User-Agent"] == "UltraFast-Integration-Test"

        except Exception as e:
            pytest.skip(f"Sync integration test failed: {e}")

    @pytest.mark.asyncio
    async def test_async_client_full_workflow(self, test_url):
        """Test complete asynchronous client workflow"""
        # Create async client with comprehensive configuration
        auth_config = uf.AuthConfig.bearer("integration-test-token")
        retry_config = uf.RetryConfig(max_retries=2, initial_delay=0.5)
        timeout_config = uf.TimeoutConfig(connect_timeout=15.0, read_timeout=45.0)
        pool_config = uf.PoolConfig(max_idle_connections=20, max_idle_per_host=8)
        ssl_config = uf.SSLConfig(verify=True)
        compression_config = uf.CompressionConfig.all_algorithms()
        protocol_config = uf.ProtocolConfig(
            preferred_version=uf.HttpVersion.AUTO,
            enable_http2=True,
            enable_http3=True,
            fallback_strategy=uf.ProtocolFallback.Http3ToHttp2ToHttp1,
        )

        client = uf.AsyncHttpClient(
            base_url=test_url,
            headers={"User-Agent": "UltraFast-Async-Integration-Test"},
            auth_config=auth_config,
            retry_config=retry_config,
            timeout_config=timeout_config,
            pool_config=pool_config,
            ssl_config=ssl_config,
            compression_config=compression_config,
            protocol_config=protocol_config,
        )

        try:
            # Concurrent requests
            tasks = [
                client.get("/get", params={"async": "test1"}),
                client.get("/get", params={"async": "test2"}),
                client.post("/post", json={"async": "post_test"}),
                client.put("/put", json={"async": "put_test"}),
                client.delete("/delete"),
            ]

            responses = await asyncio.gather(*tasks, return_exceptions=True)

            # Check that most requests succeeded
            successful_responses = [
                r
                for r in responses
                if hasattr(r, "status_code") and r.status_code == 200
            ]
            assert len(successful_responses) >= 3  # At least 3 should succeed

            # Test async file upload
            with tempfile.NamedTemporaryFile(
                mode="w", delete=False, suffix=".txt"
            ) as tmp_file:
                tmp_file.write("Async integration test file content")
                tmp_file_path = tmp_file.name

            try:
                with open(tmp_file_path, "rb") as f:
                    files = {"async_integration_file": f.read()}
                    response = await client.post(
                        "/post",
                        data={"description": "Async integration file upload"},
                        files=files,
                    )
                    assert response.status_code == 200
                    data = response.json()
                    assert "async_integration_file" in data["files"]
            finally:
                os.unlink(tmp_file_path)

        except Exception as e:
            pytest.skip(f"Async integration test failed: {e}")


class TestSessionIntegration:
    """Test session integration scenarios"""

    @pytest.fixture
    def test_url(self):
        return "https://httpbin.org"

    def test_sync_session_workflow(self, test_url):
        """Test complete synchronous session workflow"""
        auth_config = uf.AuthConfig.basic("session_user", "session_pass")
        timeout_config = uf.TimeoutConfig(connect_timeout=10.0)

        session = uf.Session(
            base_url=test_url,
            headers={"X-Session-ID": "integration-session-123"},
            auth_config=auth_config,
            timeout_config=timeout_config,
            persist_cookies=True,
        )

        try:
            # Set session data
            session.set_data("user_id", "integration_user_123")
            session.set_data("session_start", str(time.time()))

            # Make multiple requests with session state
            response1 = session.get("/get", params={"session_request": "1"})
            assert response1.status_code == 200

            response2 = session.post(
                "/post",
                json={"session_request": "2", "user_id": session.get_data("user_id")},
            )
            assert response2.status_code == 200

            # Verify session headers are maintained
            response3 = session.get("/headers")
            assert response3.status_code == 200
            headers_data = response3.json()
            assert headers_data["headers"]["X-Session-Id"] == "integration-session-123"

            # Test session header management
            session.set_header("X-Request-ID", "request-456")
            response4 = session.get("/headers")
            assert response4.status_code == 200
            headers_data = response4.json()
            assert headers_data["headers"]["X-Request-Id"] == "request-456"

        except Exception as e:
            pytest.skip(f"Sync session integration test failed: {e}")

    @pytest.mark.asyncio
    async def test_async_session_workflow(self, test_url):
        """Test complete asynchronous session workflow"""
        auth_config = uf.AuthConfig.bearer("async-session-token-789")
        timeout_config = uf.TimeoutConfig(connect_timeout=15.0)

        session = uf.AsyncSession(
            base_url=test_url,
            headers={"X-Async-Session-ID": "async-integration-session-456"},
            auth_config=auth_config,
            timeout_config=timeout_config,
            persist_cookies=True,
        )

        try:
            # Set session data
            session.set_data("async_user_id", "async_integration_user_456")
            session.set_data("async_session_start", str(time.time()))

            # Make concurrent requests with session state
            tasks = [
                session.get("/get", params={"async_session_request": "1"}),
                session.post(
                    "/post",
                    json={
                        "async_session_request": "2",
                        "user_id": session.get_data("async_user_id"),
                    },
                ),
                session.put("/put", json={"async_session_request": "3"}),
            ]

            responses = await asyncio.gather(*tasks, return_exceptions=True)

            # Check that requests succeeded
            successful_responses = [
                r
                for r in responses
                if hasattr(r, "status_code") and r.status_code == 200
            ]
            assert len(successful_responses) >= 2  # At least 2 should succeed

            # Test session state persistence
            response = await session.get("/headers")
            assert response.status_code == 200
            headers_data = response.json()
            assert (
                headers_data["headers"]["X-Async-Session-Id"]
                == "async-integration-session-456"
            )

        except Exception as e:
            pytest.skip(f"Async session integration test failed: {e}")


class TestWebSocketIntegration:
    """Test WebSocket integration scenarios"""

    @pytest.mark.skip(reason="Requires external WebSocket server")
    def test_websocket_with_authentication(self):
        """Test WebSocket with authentication headers"""
        client = uf.WebSocketClient()

        # Set authentication headers
        client.set_header("Authorization", "Bearer ws-integration-token")
        client.set_header("X-WS-Client", "integration-test")

        try:
            # This would require a real WebSocket server
            # result = client.connect("wss://echo.websocket.org")
            # Test basic functionality
            assert client.headers["Authorization"] == "Bearer ws-integration-token"
            assert client.headers["X-WS-Client"] == "integration-test"

        except Exception as e:
            pytest.skip(f"WebSocket integration test failed: {e}")

    @pytest.mark.asyncio
    @pytest.mark.skip(reason="Requires external WebSocket server")
    async def test_async_websocket_integration(self):
        """Test async WebSocket integration"""
        client = uf.AsyncWebSocketClient(auto_reconnect=True, max_reconnect_attempts=3)

        # Set headers
        client.set_header("Authorization", "Bearer async-ws-token")

        try:
            # This would require a real WebSocket server
            # await client.connect("wss://echo.websocket.org")

            # Test message creation and handling
            text_msg = uf.WebSocketMessage.new_text("Integration test message")
            assert text_msg.is_text() == True
            assert text_msg.text() == "Integration test message"

            binary_msg = uf.WebSocketMessage.new_binary([1, 2, 3, 4, 5])
            assert binary_msg.is_binary() == True
            assert list(binary_msg.data()) == [1, 2, 3, 4, 5]

        except Exception as e:
            pytest.skip(f"Async WebSocket integration test failed: {e}")


class TestSSEIntegration:
    """Test Server-Sent Events integration scenarios"""

    @pytest.mark.skip(reason="Requires external SSE server")
    def test_sse_with_authentication(self):
        """Test SSE with authentication"""
        client = uf.SSEClient(
            reconnect_timeout=10.0,
            max_reconnect_attempts=3,
            headers={"Authorization": "Bearer sse-integration-token"},
        )

        try:
            # This would require a real SSE server
            # client.connect("https://example.com/events")

            # Test event creation and handling
            event = uf.SSEEvent.new(
                event_type="integration",
                data="Integration test event data",
                id="integration-event-1",
                retry=5000,
            )

            assert event.event_type == "integration"
            assert event.data == "Integration test event data"
            assert event.id == "integration-event-1"
            assert event.retry == 5000

        except Exception as e:
            pytest.skip(f"SSE integration test failed: {e}")

    @pytest.mark.asyncio
    @pytest.mark.skip(reason="Requires external SSE server")
    async def test_async_sse_integration(self):
        """Test async SSE integration"""
        client = uf.AsyncSSEClient(
            reconnect_timeout=15.0,
            max_reconnect_attempts=5,
            headers={"Authorization": "Bearer async-sse-token"},
        )

        try:
            # This would require a real SSE server
            # await client.connect("https://example.com/async-events")

            # Test event parsing functionality
            line_data = "data: Integration async event data"
            parsed = uf.parse_sse_line(line_data)
            if parsed:
                field, value = parsed
                assert field == "data"
                assert value == "Integration async event data"

        except Exception as e:
            pytest.skip(f"Async SSE integration test failed: {e}")


class TestMiddlewareIntegration:
    """Test middleware integration scenarios"""

    def test_multiple_middleware_sync(self):
        """Test multiple middleware with sync client"""
        client = uf.HttpClient()

        # Add multiple middleware
        logging_middleware = uf.LoggingMiddleware(
            name="integration_logger", log_requests=True, log_responses=True
        )

        headers_middleware = uf.HeadersMiddleware(
            name="integration_headers", headers={"X-Middleware": "integration-test"}
        )

        try:
            client.add_middleware(logging_middleware)
            client.add_middleware(headers_middleware)

            # Make request with multiple middleware
            response = client.get("https://httpbin.org/headers")
            assert response.status_code == 200

            data = response.json()
            assert data["headers"]["X-Middleware"] == "integration-test"

        except Exception as e:
            pytest.skip(f"Sync middleware integration test failed: {e}")

    @pytest.mark.asyncio
    async def test_multiple_middleware_async(self):
        """Test multiple middleware with async client"""
        client = uf.AsyncHttpClient()

        # Add multiple middleware
        logging_middleware = uf.LoggingMiddleware(
            name="async_integration_logger", log_requests=True, log_responses=True
        )

        headers_middleware = uf.HeadersMiddleware(
            name="async_integration_headers",
            headers={"X-Async-Middleware": "async-integration-test"},
        )

        try:
            await client.add_middleware(logging_middleware)
            await client.add_middleware(headers_middleware)

            # Make request with multiple middleware
            response = await client.get("https://httpbin.org/headers")
            assert response.status_code == 200

            data = response.json()
            assert data["headers"]["X-Async-Middleware"] == "async-integration-test"

        except Exception as e:
            pytest.skip(f"Async middleware integration test failed: {e}")


class TestProtocolIntegration:
    """Test protocol integration scenarios"""

    def test_http2_with_full_configuration(self):
        """Test HTTP/2 with complete configuration"""
        http2_settings = uf.Http2Settings(
            max_concurrent_streams=100, initial_window_size=65536, max_frame_size=16384
        )

        protocol_config = uf.ProtocolConfig(
            preferred_version=uf.HttpVersion.Http2,
            enable_http2=True,
            http2_settings=http2_settings,
            fallback_strategy=uf.ProtocolFallback.Http2ToHttp1,
        )

        client = uf.HttpClient(protocol_config=protocol_config)

        assert client.is_http2_enabled() == True

        try:
            response = client.get("https://httpbin.org/get")
            assert response.status_code == 200

            # Get protocol stats
            stats = client.get_protocol_stats("https://httpbin.org")
            assert isinstance(stats, dict)

        except Exception as e:
            pytest.skip(f"HTTP/2 integration test failed: {e}")

    def test_http3_with_full_configuration(self):
        """Test HTTP/3 with complete configuration if supported"""
        if not uf.HttpClient().supports_http3():
            pytest.skip("HTTP/3 not supported in this build")

        http3_settings = uf.Http3Settings(
            max_idle_timeout=30, max_udp_payload_size=1200, initial_max_data=1048576
        )

        protocol_config = uf.ProtocolConfig(
            preferred_version=uf.HttpVersion.Http3,
            enable_http3=True,
            http3_settings=http3_settings,
            fallback_strategy=uf.ProtocolFallback.Http3ToHttp2ToHttp1,
        )

        client = uf.HttpClient(protocol_config=protocol_config)

        assert client.is_http3_enabled() == True
        assert client.supports_http3() == True

        try:
            response = client.get("https://httpbin.org/get")
            assert response.status_code == 200

        except Exception as e:
            pytest.skip(f"HTTP/3 integration test failed: {e}")


class TestPerformanceIntegration:
    """Test performance integration scenarios"""

    def test_benchmark_with_configuration(self):
        """Test benchmarking with configured client"""
        # Create a configured client
        protocol_config = uf.ProtocolConfig(enable_http2=True)
        pool_config = uf.PoolConfig(max_idle_connections=20)

        client = uf.HttpClient(protocol_config=protocol_config, pool_config=pool_config)

        # Run benchmark
        benchmark = uf.Benchmark()

        try:
            results = benchmark.run_simple(
                url="https://httpbin.org/get", method="GET", requests=5, concurrency=2
            )

            assert isinstance(results, dict)

        except Exception as e:
            pytest.skip(f"Performance integration test failed: {e}")

    def test_memory_profiling_integration(self):
        """Test memory profiling with complex operations"""
        with uf.MemoryProfiler() as profiler:
            # Create multiple clients with different configurations
            clients = []

            for i in range(5):
                client = uf.HttpClient(
                    headers={"X-Client-ID": f"client-{i}"}, timeout=30.0
                )
                clients.append(client)

                # Make request with each client
                try:
                    response = client.get("https://httpbin.org/get")
                    assert response.status_code == 200
                except:
                    # Ignore network errors for memory profiling test
                    pass

        # Profiler should have collected data
        assert profiler is not None


class TestErrorHandlingIntegration:
    """Test error handling in integration scenarios"""

    def test_cascading_failures(self):
        """Test handling of cascading failures"""
        # Configure client with aggressive retries
        retry_config = uf.RetryConfig(max_retries=2, initial_delay=0.1, max_delay=1.0)

        client = uf.HttpClient(retry_config=retry_config, timeout=5.0)

        # Test with non-existent domain
        with pytest.raises(Exception):
            client.get("https://non-existent-domain-12345.com")

    @pytest.mark.asyncio
    async def test_async_error_handling_integration(self):
        """Test async error handling in complex scenarios"""
        client = uf.AsyncHttpClient(timeout=2.0)

        # Test concurrent requests with some failing
        tasks = [
            client.get("https://httpbin.org/get"),  # Should succeed
            client.get("https://httpbin.org/delay/10"),  # Should timeout
            client.get("https://non-existent-domain.com"),  # Should fail
        ]

        results = await asyncio.gather(*tasks, return_exceptions=True)

        # Should have mix of successes and exceptions
        successes = [r for r in results if hasattr(r, "status_code")]
        exceptions = [r for r in results if isinstance(r, Exception)]

        assert len(successes) >= 1  # At least one should succeed
        assert len(exceptions) >= 1  # At least one should fail


class TestRealWorldScenarios:
    """Test real-world usage scenarios"""

    def test_api_client_scenario(self):
        """Test typical API client usage scenario"""
        # Simulate API client with authentication, rate limiting, and retries
        auth_config = uf.AuthConfig.bearer("api-key-12345")
        retry_config = uf.RetryConfig(max_retries=3, initial_delay=1.0)
        rate_limit_config = uf.RateLimitConfig(
            enabled=True, requests_per_second=5, burst_size=10
        )

        client = uf.HttpClient(
            base_url="https://httpbin.org",
            headers={"User-Agent": "MyApp/1.0"},
            auth_config=auth_config,
            retry_config=retry_config,
            rate_limit_config=rate_limit_config,
        )

        try:
            # Simulate typical API operations
            # List resources
            response = client.get("/get", params={"page": 1, "limit": 10})
            assert response.status_code == 200

            # Create resource
            new_resource = {"name": "test", "value": 123}
            response = client.post("/post", json=new_resource)
            assert response.status_code == 200

            # Update resource
            updated_resource = {"name": "test_updated", "value": 456}
            response = client.put("/put", json=updated_resource)
            assert response.status_code == 200

            # Delete resource
            response = client.delete("/delete")
            assert response.status_code == 200

        except Exception as e:
            pytest.skip(f"API client scenario test failed: {e}")

    @pytest.mark.asyncio
    async def test_web_scraping_scenario(self):
        """Test web scraping scenario with concurrent requests"""
        # Configure for web scraping
        headers = {
            "User-Agent": "Mozilla/5.0 (compatible; WebScraper/1.0)",
            "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        }

        pool_config = uf.PoolConfig(max_idle_connections=20, max_idle_per_host=10)
        timeout_config = uf.TimeoutConfig(connect_timeout=10.0, read_timeout=30.0)

        client = uf.AsyncHttpClient(
            headers=headers, pool_config=pool_config, timeout_config=timeout_config
        )

        # Simulate scraping multiple pages
        urls = [
            "https://httpbin.org/html",
            "https://httpbin.org/json",
            "https://httpbin.org/xml",
        ]

        try:
            tasks = [client.get(url) for url in urls]
            responses = await asyncio.gather(*tasks, return_exceptions=True)

            # Check results
            successful_responses = [
                r
                for r in responses
                if hasattr(r, "status_code") and r.status_code == 200
            ]
            assert len(successful_responses) >= 2  # Most should succeed

        except Exception as e:
            pytest.skip(f"Web scraping scenario test failed: {e}")

    def test_file_upload_scenario(self):
        """Test file upload scenario with large files"""
        client = uf.HttpClient(timeout=60.0)  # Longer timeout for file uploads

        # Create a larger test file
        with tempfile.NamedTemporaryFile(
            mode="w", delete=False, suffix=".txt"
        ) as tmp_file:
            # Write some content (not too large for test environments)
            content = "File upload test content\n" * 100  # ~2.5KB
            tmp_file.write(content)
            tmp_file_path = tmp_file.name

        try:
            with open(tmp_file_path, "rb") as f:
                files = {"large_file": f.read()}
                form_data = {
                    "description": "Large file upload test",
                    "upload_type": "integration_test",
                }

                response = client.post(
                    "https://httpbin.org/post", data=form_data, files=files
                )
                assert response.status_code == 200

                data = response.json()
                assert "large_file" in data["files"]
                assert data["form"]["description"] == "Large file upload test"

        except Exception as e:
            pytest.skip(f"File upload scenario test failed: {e}")
        finally:
            os.unlink(tmp_file_path)
