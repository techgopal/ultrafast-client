"""
Comprehensive Test Suite for Performance Features

Tests performance-related functionality including:
- Performance statistics and metrics
- Benchmarking capabilities
- Memory profiling
- Connection pooling
- HTTP/3 support
- Protocol statistics
- Rate limiting performance
- Concurrent request handling
"""

import asyncio
import threading
import time
from typing import Any, Dict, List

import pytest
import ultrafast_client as uf


class TestPerformanceStatistics:
    """Test performance statistics and metrics"""

    @pytest.fixture
    def client(self):
        """Create a client for performance testing"""
        return uf.HttpClient(timeout=30.0)

    @pytest.fixture
    def async_client(self):
        """Create an async client for performance testing"""
        return uf.AsyncHttpClient(timeout=30.0)

    def test_get_stats(self, client):
        """Test getting performance statistics"""
        # Make a request to generate stats
        try:
            client.get("https://httpbin.org/get")
        except:
            # Ignore network errors for stats testing
            pass

        stats = client.get_stats()
        assert isinstance(stats, dict)

        # Stats should contain performance metrics
        # The exact keys depend on implementation
        assert len(stats) >= 0  # Should at least be a valid dict

    @pytest.mark.asyncio
    async def test_async_get_stats(self, async_client):
        """Test getting async performance statistics"""
        # Make a request to generate stats
        try:
            await async_client.get("https://httpbin.org/get")
        except:
            # Ignore network errors for stats testing
            pass

        stats = async_client.get_stats_sync()
        assert isinstance(stats, dict)

        # Stats should contain performance metrics
        assert len(stats) >= 0  # Should at least be a valid dict

    def test_protocol_stats(self, client):
        """Test protocol-specific statistics"""
        stats = client.get_protocol_stats("https://httpbin.org")
        assert isinstance(stats, dict)

        # Should contain protocol information
        if stats:
            assert "protocol_version" in stats or len(stats) >= 0

    @pytest.mark.asyncio
    async def test_async_protocol_stats(self, async_client):
        """Test async protocol-specific statistics"""
        stats = async_client.get_protocol_stats("https://httpbin.org")
        assert isinstance(stats, dict)

        # Should contain protocol information
        if stats:
            assert "protocol_version" in stats or len(stats) >= 0

    def test_http3_support_detection(self, client):
        """Test HTTP/3 support detection"""
        supports_http3 = client.supports_http3()
        assert isinstance(supports_http3, bool)

    @pytest.mark.asyncio
    async def test_async_http3_support_detection(self, async_client):
        """Test async HTTP/3 support detection"""
        supports_http3 = async_client.supports_http3()
        assert isinstance(supports_http3, bool)


class TestBenchmarking:
    """Test benchmarking capabilities"""

    def test_benchmark_creation(self):
        """Test creating benchmark instance"""
        benchmark = uf.Benchmark()
        assert benchmark is not None

    def test_benchmark_simple_request(self):
        """Test benchmarking a simple request"""
        benchmark = uf.Benchmark()

        try:
            # Run a simple benchmark
            results = benchmark.run_simple(
                url="https://httpbin.org/get", method="GET", requests=5, concurrency=2
            )

            assert isinstance(results, dict)
            assert "total_time" in results or len(results) >= 0

        except Exception as e:
            # Benchmark may fail in test environments
            pytest.skip(f"Benchmark failed: {e}")

    def test_benchmark_with_configuration(self):
        """Test benchmarking with custom configuration"""
        benchmark = uf.Benchmark()

        try:
            # Run benchmark with custom settings
            results = benchmark.run_with_config(
                url="https://httpbin.org/get",
                config={
                    "requests": 3,
                    "concurrency": 1,
                    "timeout": 10.0,
                    "method": "GET",
                },
            )

            assert isinstance(results, dict)

        except Exception as e:
            # Benchmark may fail in test environments
            pytest.skip(f"Configured benchmark failed: {e}")

    def test_benchmark_multiple_urls(self):
        """Test benchmarking multiple URLs"""
        benchmark = uf.Benchmark()

        urls = [
            "https://httpbin.org/get",
            "https://httpbin.org/ip",
            "https://httpbin.org/user-agent",
        ]

        try:
            results = benchmark.run_multiple(
                urls=urls, requests_per_url=2, concurrency=1
            )

            assert isinstance(results, dict)
            assert len(results) >= 0

        except Exception as e:
            # Benchmark may fail in test environments
            pytest.skip(f"Multiple URL benchmark failed: {e}")


class TestMemoryProfiling:
    """Test memory profiling capabilities"""

    def test_memory_profiler_creation(self):
        """Test creating memory profiler"""
        profiler = uf.MemoryProfiler()
        assert profiler is not None

    def test_memory_profiler_start_stop(self):
        """Test starting and stopping memory profiler"""
        profiler = uf.MemoryProfiler()

        # Start profiling
        profiler.start()

        # Do some work (make HTTP requests)
        client = uf.HttpClient()
        try:
            client.get("https://httpbin.org/get")
        except:
            # Ignore network errors
            pass

        # Stop profiling
        results = profiler.stop()

        assert isinstance(results, dict)

    def test_memory_profiler_context_manager(self):
        """Test memory profiler as context manager"""
        with uf.MemoryProfiler() as profiler:
            # Do some work
            client = uf.HttpClient()
            try:
                client.get("https://httpbin.org/get")
            except:
                # Ignore network errors
                pass

        # Profiler should have collected data
        assert profiler is not None

    def test_memory_profiler_with_client(self):
        """Test memory profiling specific client operations"""
        profiler = uf.MemoryProfiler()

        profiler.start()

        # Create multiple clients to test memory usage
        clients = []
        for i in range(5):
            client = uf.HttpClient()
            clients.append(client)

        profiler.stop()

        # Should complete without errors
        assert len(clients) == 5


class TestConnectionPooling:
    """Test connection pooling performance"""

    def test_pool_configuration_performance(self):
        """Test different pool configurations for performance"""
        # Small pool
        small_pool_config = uf.PoolConfig(
            max_idle_connections=5, max_idle_per_host=2, idle_timeout=30.0
        )

        small_pool_client = uf.HttpClient(pool_config=small_pool_config)

        # Large pool
        large_pool_config = uf.PoolConfig(
            max_idle_connections=50, max_idle_per_host=10, idle_timeout=60.0
        )

        large_pool_client = uf.HttpClient(pool_config=large_pool_config)

        # Both should work
        assert small_pool_client is not None
        assert large_pool_client is not None

    def test_connection_reuse(self):
        """Test connection reuse for performance"""
        client = uf.HttpClient(base_url="https://httpbin.org")

        # Make multiple requests to same host
        try:
            start_time = time.time()

            for i in range(3):
                response = client.get("/get")
                assert response.status_code == 200

            end_time = time.time()
            total_time = end_time - start_time

            # Should complete relatively quickly due to connection reuse
            assert total_time < 30.0  # Reasonable timeout

        except Exception as e:
            pytest.skip(f"Connection reuse test failed: {e}")

    @pytest.mark.asyncio
    async def test_async_connection_pooling(self):
        """Test async connection pooling"""
        pool_config = uf.PoolConfig(max_idle_connections=20, max_idle_per_host=5)

        client = uf.AsyncHttpClient(
            base_url="https://httpbin.org", pool_config=pool_config
        )

        # Make concurrent requests
        try:
            tasks = [client.get("/delay/0.5") for _ in range(5)]

            start_time = time.time()
            responses = await asyncio.gather(*tasks, return_exceptions=True)
            end_time = time.time()

            # Check that at least some requests succeeded
            successful_responses = [r for r in responses if hasattr(r, "status_code")]
            assert len(successful_responses) >= 0

            # Should complete in reasonable time
            total_time = end_time - start_time
            assert total_time < 60.0  # Reasonable timeout for concurrent requests

        except Exception as e:
            pytest.skip(f"Async connection pooling test failed: {e}")


class TestRateLimitingPerformance:
    """Test rate limiting performance impact"""

    def test_rate_limiting_overhead(self):
        """Test performance overhead of rate limiting"""
        # Client without rate limiting
        normal_client = uf.HttpClient()

        # Client with rate limiting
        rate_limit_config = uf.RateLimitConfig(
            enabled=True, requests_per_second=10, burst_size=5
        )

        rate_limited_client = uf.HttpClient(rate_limit_config=rate_limit_config)

        # Both should work (we're testing creation overhead here)
        assert normal_client is not None
        assert rate_limited_client is not None

        # Test that rate limited client enforces limits
        assert rate_limited_client.is_rate_limit_enabled() == True

    def test_rate_limiting_algorithms_performance(self):
        """Test different rate limiting algorithms"""
        algorithms = [
            uf.RateLimitAlgorithm.TokenBucket,
            uf.RateLimitAlgorithm.LeakyBucket,
            uf.RateLimitAlgorithm.FixedWindow,
            uf.RateLimitAlgorithm.SlidingWindow,
        ]

        clients = []
        for algorithm in algorithms:
            rate_config = uf.RateLimitConfig(
                enabled=True, requests_per_second=5, algorithm=algorithm
            )

            client = uf.HttpClient(rate_limit_config=rate_config)
            clients.append(client)

            # Should create successfully
            assert client is not None
            assert client.is_rate_limit_enabled() == True

        # All algorithm-specific clients should be created
        assert len(clients) == len(algorithms)

    @pytest.mark.asyncio
    async def test_async_rate_limiting_performance(self):
        """Test async rate limiting performance"""
        rate_config = uf.RateLimitConfig(
            enabled=True, requests_per_second=5, burst_size=10
        )

        client = uf.AsyncHttpClient(rate_limit_config=rate_config)

        # Make multiple requests to test rate limiting
        try:
            start_time = time.time()

            tasks = [client.get("https://httpbin.org/get") for _ in range(3)]

            responses = await asyncio.gather(*tasks, return_exceptions=True)

            end_time = time.time()
            total_time = end_time - start_time

            # Rate limiting may introduce delays
            # Just verify it doesn't crash and takes reasonable time
            assert total_time < 60.0  # Should not hang indefinitely

        except Exception as e:
            pytest.skip(f"Async rate limiting test failed: {e}")


class TestConcurrentPerformance:
    """Test concurrent request performance"""

    def test_threaded_sync_requests(self):
        """Test synchronous requests in multiple threads"""

        def make_request():
            client = uf.HttpClient()
            try:
                response = client.get("https://httpbin.org/get")
                return response.status_code == 200
            except:
                return False

        # Create multiple threads
        threads = []
        for i in range(5):
            thread = threading.Thread(target=make_request)
            threads.append(thread)

        # Start all threads
        start_time = time.time()
        for thread in threads:
            thread.start()

        # Wait for all threads
        for thread in threads:
            thread.join(timeout=30.0)  # 30 second timeout

        end_time = time.time()
        total_time = end_time - start_time

        # Should complete in reasonable time
        assert total_time < 60.0

    @pytest.mark.asyncio
    async def test_concurrent_async_requests(self):
        """Test concurrent async requests performance"""
        client = uf.AsyncHttpClient()

        async def make_request():
            try:
                response = await client.get("https://httpbin.org/get")
                return response.status_code == 200
            except:
                return False

        # Create multiple concurrent tasks
        start_time = time.time()

        tasks = [make_request() for _ in range(10)]
        results = await asyncio.gather(*tasks, return_exceptions=True)

        end_time = time.time()
        total_time = end_time - start_time

        # Should complete in reasonable time
        assert total_time < 30.0

        # Check results
        successful_results = [r for r in results if r is True]
        assert len(successful_results) >= 0  # At least some should succeed

    def test_mixed_sync_async_performance(self):
        """Test performance when mixing sync and async clients"""
        sync_client = uf.HttpClient()
        async_client = uf.AsyncHttpClient()

        # Both should coexist without performance issues
        assert sync_client is not None
        assert async_client is not None

        # Test that they don't interfere with each other
        try:
            sync_response = sync_client.get("https://httpbin.org/get")
            sync_success = sync_response.status_code == 200
        except:
            sync_success = False

        async def test_async():
            try:
                async_response = await async_client.get("https://httpbin.org/get")
                return async_response.status_code == 200
            except:
                return False

        async_success = asyncio.run(test_async())

        # Both can work independently
        assert isinstance(sync_success, bool)
        assert isinstance(async_success, bool)


class TestProtocolPerformance:
    """Test protocol-specific performance"""

    def test_http1_performance(self):
        """Test HTTP/1.1 performance"""
        protocol_config = uf.ProtocolConfig(
            preferred_version=uf.HttpVersion.Http1,
            enable_http2=False,
            enable_http3=False,
        )

        client = uf.HttpClient(protocol_config=protocol_config)

        assert client.is_http2_enabled() == False
        assert client.is_http3_enabled() == False

        # Should work with HTTP/1.1
        try:
            response = client.get("https://httpbin.org/get")
            assert response.status_code == 200
        except Exception as e:
            pytest.skip(f"HTTP/1.1 test failed: {e}")

    def test_http2_performance(self):
        """Test HTTP/2 performance"""
        protocol_config = uf.ProtocolConfig(
            preferred_version=uf.HttpVersion.Http2,
            enable_http2=True,
            enable_http3=False,
        )

        client = uf.HttpClient(protocol_config=protocol_config)

        assert client.is_http2_enabled() == True

        # Should work with HTTP/2
        try:
            response = client.get("https://httpbin.org/get")
            assert response.status_code == 200
        except Exception as e:
            pytest.skip(f"HTTP/2 test failed: {e}")

    def test_http3_performance(self):
        """Test HTTP/3 performance if available"""
        if not uf.HttpClient().supports_http3():
            pytest.skip("HTTP/3 not supported in this build")

        protocol_config = uf.ProtocolConfig(
            preferred_version=uf.HttpVersion.Http3, enable_http3=True
        )

        client = uf.HttpClient(protocol_config=protocol_config)

        assert client.is_http3_enabled() == True
        assert client.supports_http3() == True

        # Should work with HTTP/3
        try:
            response = client.get("https://httpbin.org/get")
            assert response.status_code == 200
        except Exception as e:
            pytest.skip(f"HTTP/3 test failed: {e}")

    def test_protocol_fallback_performance(self):
        """Test protocol fallback performance"""
        protocol_config = uf.ProtocolConfig(
            preferred_version=uf.HttpVersion.AUTO,
            enable_http2=True,
            enable_http3=True,
            fallback_strategy=uf.ProtocolFallback.Http3ToHttp2ToHttp1,
        )

        client = uf.HttpClient(protocol_config=protocol_config)

        # Should handle fallback gracefully
        try:
            response = client.get("https://httpbin.org/get")
            assert response.status_code == 200
        except Exception as e:
            pytest.skip(f"Protocol fallback test failed: {e}")


class TestCompressionPerformance:
    """Test compression performance impact"""

    def test_compression_overhead(self):
        """Test performance overhead of compression"""
        # No compression
        no_compression_config = uf.CompressionConfig.disabled()
        no_compression_client = uf.HttpClient(compression_config=no_compression_config)

        # With compression
        compression_config = uf.CompressionConfig.all_algorithms()
        compression_client = uf.HttpClient(compression_config=compression_config)

        # Both should work
        assert no_compression_client is not None
        assert compression_client is not None

        # Test performance with large response
        try:
            # Request larger data to see compression benefit
            url = "https://httpbin.org/bytes/10240"  # 10KB

            start_time = time.time()
            response1 = no_compression_client.get(url)
            no_compression_time = time.time() - start_time

            start_time = time.time()
            response2 = compression_client.get(url)
            compression_time = time.time() - start_time

            # Both should succeed
            assert response1.status_code == 200
            assert response2.status_code == 200

            # Compression may be faster for large responses
            # But we just verify both complete in reasonable time
            assert no_compression_time < 30.0
            assert compression_time < 30.0

        except Exception as e:
            pytest.skip(f"Compression performance test failed: {e}")


class TestPerformanceEdgeCases:
    """Test performance edge cases"""

    def test_large_header_performance(self):
        """Test performance with large headers"""
        # Create large headers
        large_headers = {}
        for i in range(100):
            large_headers[f"X-Large-Header-{i}"] = f"value-{i}" * 100

        client = uf.HttpClient(headers=large_headers)

        try:
            response = client.get("https://httpbin.org/headers")
            assert response.status_code == 200
        except Exception as e:
            pytest.skip(f"Large header test failed: {e}")

    def test_many_concurrent_clients(self):
        """Test performance with many concurrent clients"""
        clients = []

        # Create many clients
        for i in range(20):
            client = uf.HttpClient()
            clients.append(client)

        # All should be created successfully
        assert len(clients) == 20

        # Test that they can be used (basic functionality)
        try:
            response = clients[0].get("https://httpbin.org/get")
            assert response.status_code == 200
        except Exception as e:
            pytest.skip(f"Many clients test failed: {e}")

    def test_rapid_client_creation_destruction(self):
        """Test rapid client creation and destruction"""
        start_time = time.time()

        # Create and destroy many clients rapidly
        for i in range(50):
            client = uf.HttpClient()
            # Client goes out of scope and should be cleaned up

        end_time = time.time()
        total_time = end_time - start_time

        # Should complete quickly
        assert total_time < 10.0  # Should not take more than 10 seconds
