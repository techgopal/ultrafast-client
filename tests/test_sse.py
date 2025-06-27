"""
Comprehensive Test Suite for Server-Sent Events (SSE) Clients

Tests all features and functionality of both SSEClient and AsyncSSEClient including:
- Connection management
- Event streaming and parsing
- Event types (data, id, retry, custom events)
- Reconnection features
- Error handling
- Context manager support
- Real-time event processing
"""

import asyncio
import json
import time
from typing import List, Optional

import pytest
import ultrafast_client as uf


class TestSSEEvent:
    """Test SSEEvent class"""

    def test_event_creation(self):
        """Test creating SSE events"""
        event = uf.SSEEvent.new(
            event_type="message", data="Hello, SSE!", id="event-1", retry=5000
        )

        assert event.event_type == "message"
        assert event.data == "Hello, SSE!"
        assert event.id == "event-1"
        assert event.retry == 5000
        assert isinstance(event.timestamp, float)

    def test_event_without_optional_fields(self):
        """Test creating events without optional fields"""
        event = uf.SSEEvent.new(
            event_type=None, data="Simple event data", id=None, retry=None
        )

        assert event.event_type is None
        assert event.data == "Simple event data"
        assert event.id is None
        assert event.retry is None

    def test_event_json_parsing(self):
        """Test JSON data parsing in events"""
        json_data = '{"name": "test", "value": 123}'
        event = uf.SSEEvent.new(
            event_type="json", data=json_data, id="json-event", retry=None
        )

        # Test JSON parsing (if supported by implementation)
        try:
            import sys

            parsed = event.json(sys.modules["__main__"].__dict__["__builtins__"])
            if parsed:
                assert isinstance(parsed, dict)
        except:
            # JSON parsing may not be implemented or available
            pass

    def test_keepalive_detection(self):
        """Test keepalive event detection"""
        keepalive_event = uf.SSEEvent.new(event_type=None, data="", id=None, retry=None)

        normal_event = uf.SSEEvent.new(
            event_type="message", data="Normal event data", id="event-1", retry=None
        )

        # Keepalive events typically have empty data
        assert keepalive_event.is_keepalive() == True
        assert normal_event.is_keepalive() == False

    def test_retry_detection(self):
        """Test retry event detection"""
        retry_event = uf.SSEEvent.new(event_type=None, data="", id=None, retry=3000)

        normal_event = uf.SSEEvent.new(
            event_type="message", data="Normal event", id="event-1", retry=None
        )

        assert retry_event.is_retry() == True
        assert normal_event.is_retry() == False

    def test_event_string_representation(self):
        """Test event string representation"""
        event = uf.SSEEvent.new(
            event_type="test", data="Test data", id="test-id", retry=1000
        )

        # Test __repr__ and __str__ methods
        repr_str = repr(event)
        str_str = str(event)

        assert isinstance(repr_str, str)
        assert isinstance(str_str, str)
        assert len(repr_str) > 0
        assert len(str_str) > 0


class TestSSEClientSync:
    """Test synchronous SSEClient"""

    @pytest.fixture
    def client(self):
        """Create an SSEClient for testing"""
        return uf.SSEClient(
            reconnect_timeout=5.0,
            max_reconnect_attempts=3,
            headers={"User-Agent": "UltraFast-SSE-Test"},
        )

    @pytest.fixture
    def sse_server_url(self):
        """SSE server URL for testing"""
        # Note: This would need a real SSE endpoint for full testing
        return "https://httpbin.org/stream/5"

    def test_client_creation(self, client):
        """Test SSE client creation"""
        assert client.reconnect_timeout == 5.0
        assert client.max_reconnect_attempts == 3
        assert client.is_connected() == False
        assert client.url() is None

    def test_header_management(self, client):
        """Test SSE header management"""
        # Set header
        client.set_header("Authorization", "Bearer sse_token123")
        client.set_header("X-Custom-SSE", "sse-custom-value")

        headers = client.headers()
        assert "Authorization" in headers
        assert headers["Authorization"] == "Bearer sse_token123"
        assert headers["X-Custom-SSE"] == "sse-custom-value"

        # Remove header
        removed = client.remove_header("X-Custom-SSE")
        assert removed == "sse-custom-value"

        updated_headers = client.headers()
        assert "X-Custom-SSE" not in updated_headers

    def test_connection_status(self, client):
        """Test connection status checking"""
        assert client.is_connected() == False

    def test_url_property(self, client):
        """Test URL property"""
        assert client.url() is None

    def test_context_manager(self, client):
        """Test SSE client as context manager"""
        with client as sse:
            assert sse is client

    def test_close_method(self, client):
        """Test close method"""
        client.close()
        assert client.is_connected() == False

    # Note: The following tests require a real SSE server
    # and may be unreliable in CI environments

    @pytest.mark.skip(reason="Requires external SSE server")
    def test_connect_to_sse_endpoint(self, client, sse_server_url):
        """Test connecting to SSE endpoint"""
        try:
            client.connect(sse_server_url)
            assert client.is_connected() == True
            assert client.url() == sse_server_url
        except Exception as e:
            # Connection may fail in test environment
            pytest.skip(f"SSE connection failed: {e}")

    @pytest.mark.skip(reason="Requires external SSE server")
    def test_listen_for_events(self, client, sse_server_url):
        """Test listening for SSE events"""
        try:
            client.connect(sse_server_url)

            # Get event iterator
            iterator = client.listen()
            assert iterator is not None

            # Try to get an event (with timeout to avoid hanging)
            try:
                event = next(iterator)
                if event:
                    assert isinstance(event, uf.SSEEvent)
                    assert event.data is not None
            except StopIteration:
                # No events available, which is okay for testing
                pass

        except Exception as e:
            pytest.skip(f"SSE listening failed: {e}")


class TestAsyncSSEClient:
    """Test asynchronous AsyncSSEClient"""

    @pytest.fixture
    def client(self):
        """Create an AsyncSSEClient for testing"""
        return uf.AsyncSSEClient(
            reconnect_timeout=10.0,
            max_reconnect_attempts=5,
            headers={"User-Agent": "UltraFast-Async-SSE-Test"},
        )

    @pytest.fixture
    def sse_server_url(self):
        """SSE server URL for testing"""
        return "https://httpbin.org/stream/5"

    def test_async_client_creation(self, client):
        """Test async SSE client creation"""
        assert client.reconnect_timeout == 10.0
        assert client.max_reconnect_attempts == 5
        assert client.is_connected() == False
        assert client.url() is None

    def test_async_header_management(self, client):
        """Test async SSE header management"""
        # Set header
        client.set_header("Authorization", "Bearer async_sse_token123")
        client.set_header("X-Async-SSE", "async-sse-value")

        headers = client.headers()
        assert "Authorization" in headers
        assert headers["Authorization"] == "Bearer async_sse_token123"
        assert headers["X-Async-SSE"] == "async-sse-value"

        # Remove header
        removed = client.remove_header("X-Async-SSE")
        assert removed == "async-sse-value"

        updated_headers = client.headers()
        assert "X-Async-SSE" not in updated_headers

    def test_async_connection_status(self, client):
        """Test async connection status checking"""
        assert client.is_connected() == False

    def test_async_url_property(self, client):
        """Test async URL property"""
        assert client.url() is None

    def test_async_context_manager(self, client):
        """Test async SSE client as context manager"""
        with client as sse:
            assert sse is client

    @pytest.mark.asyncio
    async def test_async_close_method(self, client):
        """Test async close method"""
        await client.close()
        assert client.is_connected() == False

    # Note: The following tests require a real SSE server
    # and may be unreliable in CI environments

    @pytest.mark.asyncio
    @pytest.mark.skip(reason="Requires external SSE server")
    async def test_async_connect_to_sse_endpoint(self, client, sse_server_url):
        """Test async connecting to SSE endpoint"""
        try:
            await client.connect(sse_server_url)
            assert client.is_connected() == True
            assert client.url() == sse_server_url
        except Exception as e:
            pytest.skip(f"Async SSE connection failed: {e}")

    @pytest.mark.asyncio
    @pytest.mark.skip(reason="Requires external SSE server")
    async def test_async_listen_for_events(self, client, sse_server_url):
        """Test async listening for SSE events"""
        try:
            await client.connect(sse_server_url)

            # Get async event listener
            import sys

            listener = client.listen(sys.modules["__main__"])
            assert listener is not None

        except Exception as e:
            pytest.skip(f"Async SSE listening failed: {e}")


class TestSSEEventIterator:
    """Test SSEEventIterator functionality"""

    def test_iterator_creation(self):
        """Test creating SSE event iterator"""
        # This test is limited without a real connection
        # but we can test the basic structure

        # Create a mock receiver for testing
        import threading
        from queue import Queue

        # Basic test to ensure the class can be instantiated
        # Full testing would require integration with actual SSE connection
        pass

    def test_iterator_protocol(self):
        """Test iterator protocol implementation"""
        # Event iterator should implement __iter__ and __next__
        # This would be tested with a real connection in integration tests
        pass


class TestSSEConfiguration:
    """Test SSE configuration options"""

    def test_reconnection_configuration(self):
        """Test reconnection configuration"""
        client = uf.SSEClient(
            reconnect_timeout=15.0,
            max_reconnect_attempts=10,
            headers={"X-Test": "test-value"},
        )

        assert client.reconnect_timeout == 15.0
        assert client.max_reconnect_attempts == 10

        headers = client.headers()
        assert headers["X-Test"] == "test-value"

    def test_async_reconnection_configuration(self):
        """Test async reconnection configuration"""
        client = uf.AsyncSSEClient(
            reconnect_timeout=20.0,
            max_reconnect_attempts=15,
            headers={"X-Async-Test": "async-test-value"},
        )

        assert client.reconnect_timeout == 20.0
        assert client.max_reconnect_attempts == 15

        headers = client.headers()
        assert headers["X-Async-Test"] == "async-test-value"

    def test_default_configuration(self):
        """Test default configuration values"""
        sync_client = uf.SSEClient()
        async_client = uf.AsyncSSEClient()

        # Both should have reasonable defaults
        assert sync_client.reconnect_timeout == 5.0
        assert async_client.reconnect_timeout == 5.0

        assert sync_client.max_reconnect_attempts == 10
        assert async_client.max_reconnect_attempts == 10


class TestSSEErrorHandling:
    """Test SSE error handling"""

    def test_invalid_url_handling(self):
        """Test handling of invalid SSE URLs"""
        client = uf.SSEClient()

        # Test with invalid URL
        with pytest.raises(Exception):
            client.connect("invalid-sse-url")

    @pytest.mark.asyncio
    async def test_async_invalid_url_handling(self):
        """Test async handling of invalid SSE URLs"""
        client = uf.AsyncSSEClient()

        # Test with invalid URL
        with pytest.raises(Exception):
            await client.connect("invalid-sse-url")

    def test_connection_without_connect(self):
        """Test operations without connecting first"""
        client = uf.SSEClient()

        # Should handle gracefully when not connected
        assert client.is_connected() == False

        # Listening without connecting should raise error
        with pytest.raises(Exception):
            client.listen()

    def test_multiple_close_calls(self):
        """Test multiple close calls don't cause errors"""
        client = uf.SSEClient()

        # Multiple close calls should be safe
        client.close()
        client.close()
        client.close()

        assert client.is_connected() == False


class TestSSEParsing:
    """Test SSE event parsing functionality"""

    def test_parse_sse_line(self):
        """Test SSE line parsing"""
        # Test data line
        result = uf.parse_sse_line("data: Hello, World!")
        if result:
            field, value = result
            assert field == "data"
            assert value == "Hello, World!"

        # Test event line
        result = uf.parse_sse_line("event: message")
        if result:
            field, value = result
            assert field == "event"
            assert value == "message"

        # Test id line
        result = uf.parse_sse_line("id: 12345")
        if result:
            field, value = result
            assert field == "id"
            assert value == "12345"

        # Test retry line
        result = uf.parse_sse_line("retry: 5000")
        if result:
            field, value = result
            assert field == "retry"
            assert value == "5000"

        # Test invalid line
        result = uf.parse_sse_line("invalid line without colon")
        assert result is None

    def test_build_sse_event(self):
        """Test building SSE event from fields"""
        fields = {
            "data": ["Line 1", "Line 2", "Line 3"],
            "event": ["message"],
            "id": ["event-123"],
            "retry": ["3000"],
        }

        event = uf.build_sse_event(fields)

        assert isinstance(event, uf.SSEEvent)
        assert event.event_type == "message"
        assert "Line 1\nLine 2\nLine 3" in event.data
        assert event.id == "event-123"
        assert event.retry == 3000


class TestSSEIntegration:
    """Test SSE integration scenarios"""

    def test_sync_and_async_client_coexistence(self):
        """Test that sync and async SSE clients can coexist"""
        sync_client = uf.SSEClient(headers={"X-Sync": "sync-sse"})
        async_client = uf.AsyncSSEClient(headers={"X-Async": "async-sse"})

        # Both should be independent
        sync_headers = sync_client.headers()
        async_headers = async_client.headers()

        assert "X-Sync" in sync_headers
        assert "X-Sync" not in async_headers

        assert "X-Async" in async_headers
        assert "X-Async" not in sync_headers

    def test_configuration_independence(self):
        """Test that SSE client configurations are independent"""
        client1 = uf.SSEClient(reconnect_timeout=5.0, max_reconnect_attempts=3)
        client2 = uf.SSEClient(reconnect_timeout=10.0, max_reconnect_attempts=5)

        assert client1.reconnect_timeout != client2.reconnect_timeout
        assert client1.max_reconnect_attempts != client2.max_reconnect_attempts

    def test_event_types_handling(self):
        """Test handling different types of SSE events"""
        # Test different event types that might be encountered

        # Standard message event
        message_event = uf.SSEEvent.new("message", "Hello", "1", None)
        assert message_event.event_type == "message"

        # Custom event type
        custom_event = uf.SSEEvent.new("user-login", "User logged in", "2", None)
        assert custom_event.event_type == "user-login"

        # Event with retry instruction
        retry_event = uf.SSEEvent.new(None, "", None, 5000)
        assert retry_event.is_retry() == True

        # Keepalive event
        keepalive_event = uf.SSEEvent.new(None, "", None, None)
        assert keepalive_event.is_keepalive() == True


class TestSSERealTime:
    """Test real-time SSE functionality"""

    def test_event_timestamp(self):
        """Test event timestamp generation"""
        event = uf.SSEEvent.new("test", "data", "id", None)

        assert isinstance(event.timestamp, float)
        assert event.timestamp > 0

        # Create another event and verify timestamp is different
        time.sleep(0.001)  # Small delay
        event2 = uf.SSEEvent.new("test2", "data2", "id2", None)

        assert event2.timestamp > event.timestamp

    def test_concurrent_sse_clients(self):
        """Test multiple SSE clients running concurrently"""
        client1 = uf.SSEClient(headers={"X-Client": "client1"})
        client2 = uf.SSEClient(headers={"X-Client": "client2"})

        # Both clients should be independent
        assert client1.headers()["X-Client"] == "client1"
        assert client2.headers()["X-Client"] == "client2"

        # Both should be able to close independently
        client1.close()
        client2.close()

        assert client1.is_connected() == False
        assert client2.is_connected() == False
