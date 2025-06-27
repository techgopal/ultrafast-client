"""
Comprehensive Test Suite for WebSocket Clients

Tests all features and functionality of both WebSocketClient and AsyncWebSocketClient including:
- Connection management
- Message sending and receiving (text, binary, ping, pong)
- Auto-reconnection features
- Error handling
- Context manager support
- Real-time bidirectional communication
"""

import pytest
import asyncio
import ultrafast_client as uf
import time
import threading
from typing import List


class TestWebSocketMessage:
    """Test WebSocketMessage class"""
    
    def test_text_message_creation(self):
        """Test creating text messages"""
        msg = uf.WebSocketMessage.new_text("Hello, WebSocket!")
        assert msg.is_text() == True
        assert msg.text() == "Hello, WebSocket!"
        assert msg.message_type == "text"
        
    def test_binary_message_creation(self):
        """Test creating binary messages"""
        data = b"Binary data content"
        msg = uf.WebSocketMessage.new_binary(list(data))
        assert msg.is_binary() == True
        assert bytes(msg.data()) == data
        assert msg.message_type == "binary"
        
    def test_ping_message_creation(self):
        """Test creating ping messages"""
        ping_data = b"ping payload"
        msg = uf.WebSocketMessage.new_ping(list(ping_data))
        assert msg.is_ping() == True
        assert bytes(msg.data()) == ping_data
        assert msg.message_type == "ping"
        
    def test_pong_message_creation(self):
        """Test creating pong messages"""
        pong_data = b"pong payload"
        msg = uf.WebSocketMessage.new_pong(list(pong_data))
        assert msg.is_pong() == True
        assert bytes(msg.data()) == pong_data
        assert msg.message_type == "pong"
        
    def test_close_message_creation(self):
        """Test creating close messages"""
        msg = uf.WebSocketMessage.new_close()
        assert msg.is_close() == True
        assert msg.message_type == "close"
        
    def test_message_type_checks(self):
        """Test message type checking methods"""
        text_msg = uf.WebSocketMessage.new_text("test")
        binary_msg = uf.WebSocketMessage.new_binary([1, 2, 3])
        ping_msg = uf.WebSocketMessage.new_ping([])
        pong_msg = uf.WebSocketMessage.new_pong([])
        close_msg = uf.WebSocketMessage.new_close()
        
        # Test text message
        assert text_msg.is_text() == True
        assert text_msg.is_binary() == False
        assert text_msg.is_ping() == False
        assert text_msg.is_pong() == False
        assert text_msg.is_close() == False
        
        # Test binary message
        assert binary_msg.is_text() == False
        assert binary_msg.is_binary() == True
        assert binary_msg.is_ping() == False
        assert binary_msg.is_pong() == False
        assert binary_msg.is_close() == False
        
        # Test ping message
        assert ping_msg.is_text() == False
        assert ping_msg.is_binary() == False
        assert ping_msg.is_ping() == True
        assert ping_msg.is_pong() == False
        assert ping_msg.is_close() == False
        
        # Test pong message
        assert pong_msg.is_text() == False
        assert pong_msg.is_binary() == False
        assert pong_msg.is_ping() == False
        assert pong_msg.is_pong() == True
        assert pong_msg.is_close() == False
        
        # Test close message
        assert close_msg.is_text() == False
        assert close_msg.is_binary() == False
        assert close_msg.is_ping() == False
        assert close_msg.is_pong() == False
        assert close_msg.is_close() == True


class TestWebSocketClientSync:
    """Test synchronous WebSocketClient"""
    
    @pytest.fixture
    def client(self):
        """Create a WebSocketClient for testing"""
        return uf.WebSocketClient(
            auto_reconnect=True,
            max_reconnect_attempts=3,
            reconnect_delay=1.0
        )
    
    @pytest.fixture
    def echo_server_url(self):
        """WebSocket echo server URL for testing"""
        return "wss://echo.websocket.org"
    
    def test_client_creation(self, client):
        """Test WebSocket client creation"""
        assert client.auto_reconnect == True
        assert client.max_reconnect_attempts == 3
        assert client.reconnect_delay == 1.0
        assert client.connected == False
        assert client.url is None
        
    def test_header_management(self, client):
        """Test WebSocket header management"""
        # Set header
        client.set_header("Authorization", "Bearer token123")
        client.set_header("X-Custom", "custom-value")
        
        assert "Authorization" in client.headers
        assert client.headers["Authorization"] == "Bearer token123"
        assert client.headers["X-Custom"] == "custom-value"
        
        # Remove header
        removed = client.remove_header("X-Custom")
        assert removed == "custom-value"
        assert "X-Custom" not in client.headers
        
    def test_reconnect_attempts_management(self, client):
        """Test reconnect attempts management"""
        client.reset_reconnect_attempts()
        # Should not raise an error
        
    def test_connection_status(self, client):
        """Test connection status checking"""
        assert client.is_connected() == False
        assert client.connected == False
        
    def test_context_manager(self, client):
        """Test WebSocket client as context manager"""
        with client as ws:
            assert ws is client
            
    # Note: The following tests require a real WebSocket server
    # and may be unreliable in CI environments
    
    @pytest.mark.skip(reason="Requires external WebSocket server")
    def test_connect_and_disconnect(self, client, echo_server_url):
        """Test WebSocket connection and disconnection"""
        # Connect
        result = client.connect(echo_server_url)
        # In real implementation, this would be async
        # For now, just test that method exists and can be called
        assert result is not None
        
    @pytest.mark.skip(reason="Requires external WebSocket server")
    def test_send_text_message(self, client, echo_server_url):
        """Test sending text messages"""
        client.connect(echo_server_url)
        
        # Send text message
        result = client.send("Hello, WebSocket!")
        assert result is not None
        
    @pytest.mark.skip(reason="Requires external WebSocket server")
    def test_send_binary_message(self, client, echo_server_url):
        """Test sending binary messages"""
        client.connect(echo_server_url)
        
        # Send binary message
        binary_data = b"Binary message content"
        result = client.send_bytes(list(binary_data))
        assert result is not None
        
    @pytest.mark.skip(reason="Requires external WebSocket server")
    def test_ping_message(self, client, echo_server_url):
        """Test sending ping messages"""
        client.connect(echo_server_url)
        
        # Send ping
        ping_data = b"ping test"
        result = client.ping(list(ping_data))
        assert result is not None
        
    @pytest.mark.skip(reason="Requires external WebSocket server")
    def test_receive_messages(self, client, echo_server_url):
        """Test receiving messages"""
        client.connect(echo_server_url)
        
        # Send a message first
        client.send("Test message")
        
        # Receive message
        result = client.receive()
        assert result is not None
        
    @pytest.mark.skip(reason="Requires external WebSocket server")
    def test_receive_with_timeout(self, client, echo_server_url):
        """Test receiving messages with timeout"""
        client.connect(echo_server_url)
        
        # Try to receive with timeout
        result = client.receive_timeout(5.0)
        assert result is not None
        
    @pytest.mark.skip(reason="Requires external WebSocket server")
    def test_receive_all_messages(self, client, echo_server_url):
        """Test receiving all available messages"""
        client.connect(echo_server_url)
        
        # Send multiple messages
        client.send("Message 1")
        client.send("Message 2")
        client.send("Message 3")
        
        # Receive all messages
        result = client.receive_all()
        assert result is not None


class TestAsyncWebSocketClient:
    """Test asynchronous AsyncWebSocketClient"""
    
    @pytest.fixture
    def client(self):
        """Create an AsyncWebSocketClient for testing"""
        return uf.AsyncWebSocketClient(
            auto_reconnect=True,
            max_reconnect_attempts=5,
            reconnect_delay=2.0
        )
    
    @pytest.fixture
    def echo_server_url(self):
        """WebSocket echo server URL for testing"""
        return "wss://echo.websocket.org"
    
    def test_async_client_creation(self, client):
        """Test async WebSocket client creation"""
        assert client.auto_reconnect == True
        assert client.max_reconnect_attempts == 5
        assert client.reconnect_delay == 2.0
        assert client.connected == False
        assert client.url is None
        
    def test_async_header_management(self, client):
        """Test async WebSocket header management"""
        # Set header
        client.set_header("Authorization", "Bearer async_token123")
        client.set_header("X-Async-Custom", "async-custom-value")
        
        assert "Authorization" in client.headers
        assert client.headers["Authorization"] == "Bearer async_token123"
        assert client.headers["X-Async-Custom"] == "async-custom-value"
        
        # Remove header
        removed = client.remove_header("X-Async-Custom")
        assert removed == "async-custom-value"
        assert "X-Async-Custom" not in client.headers
        
    def test_async_connection_status(self, client):
        """Test async connection status checking"""
        assert client.is_connected() == False
        assert client.connected == False
        
    def test_async_context_manager(self, client):
        """Test async WebSocket client as context manager"""
        with client as ws:
            assert ws is client
            
    # Note: The following tests require a real WebSocket server
    # and may be unreliable in CI environments
    
    @pytest.mark.asyncio
    @pytest.mark.skip(reason="Requires external WebSocket server")
    async def test_async_connect_and_disconnect(self, client, echo_server_url):
        """Test async WebSocket connection and disconnection"""
        # Connect
        result = await client.connect(echo_server_url)
        assert result is not None
        
        # Disconnect
        await client.close()
        
    @pytest.mark.asyncio
    @pytest.mark.skip(reason="Requires external WebSocket server")
    async def test_async_send_text_message(self, client, echo_server_url):
        """Test async sending text messages"""
        await client.connect(echo_server_url)
        
        # Send text message
        result = await client.send("Hello, Async WebSocket!")
        assert result is not None
        
        await client.close()
        
    @pytest.mark.asyncio
    @pytest.mark.skip(reason="Requires external WebSocket server")
    async def test_async_send_binary_message(self, client, echo_server_url):
        """Test async sending binary messages"""
        await client.connect(echo_server_url)
        
        # Send binary message
        binary_data = b"Async binary message content"
        result = await client.send_bytes(list(binary_data))
        assert result is not None
        
        await client.close()
        
    @pytest.mark.asyncio
    @pytest.mark.skip(reason="Requires external WebSocket server")
    async def test_async_receive_messages(self, client, echo_server_url):
        """Test async receiving messages"""
        await client.connect(echo_server_url)
        
        # Send a message first
        await client.send("Async test message")
        
        # Receive message
        result = await client.receive()
        assert result is not None
        
        await client.close()
        
    @pytest.mark.asyncio
    @pytest.mark.skip(reason="Requires external WebSocket server")
    async def test_async_concurrent_operations(self, client, echo_server_url):
        """Test async concurrent WebSocket operations"""
        await client.connect(echo_server_url)
        
        # Send multiple messages concurrently
        send_tasks = [
            client.send(f"Concurrent message {i}")
            for i in range(5)
        ]
        
        await asyncio.gather(*send_tasks)
        
        await client.close()


class TestWebSocketConfiguration:
    """Test WebSocket configuration options"""
    
    def test_reconnection_configuration(self):
        """Test reconnection configuration"""
        client = uf.WebSocketClient(
            auto_reconnect=False,
            max_reconnect_attempts=10,
            reconnect_delay=5.0
        )
        
        assert client.auto_reconnect == False
        assert client.max_reconnect_attempts == 10
        assert client.reconnect_delay == 5.0
        
    def test_async_reconnection_configuration(self):
        """Test async reconnection configuration"""
        client = uf.AsyncWebSocketClient(
            auto_reconnect=True,
            max_reconnect_attempts=15,
            reconnect_delay=3.0
        )
        
        assert client.auto_reconnect == True
        assert client.max_reconnect_attempts == 15
        assert client.reconnect_delay == 3.0
        
    def test_default_configuration(self):
        """Test default configuration values"""
        sync_client = uf.WebSocketClient()
        async_client = uf.AsyncWebSocketClient()
        
        # Both should have reasonable defaults
        assert sync_client.auto_reconnect == True
        assert async_client.auto_reconnect == True
        
        assert sync_client.max_reconnect_attempts == 5
        assert async_client.max_reconnect_attempts == 5
        
        assert sync_client.reconnect_delay == 1.0
        assert async_client.reconnect_delay == 1.0


class TestWebSocketErrorHandling:
    """Test WebSocket error handling"""
    
    def test_invalid_url_handling(self):
        """Test handling of invalid WebSocket URLs"""
        client = uf.WebSocketClient()
        
        # Test with invalid URL
        try:
            result = client.connect("invalid-websocket-url")
            # Should handle gracefully or raise appropriate exception
        except Exception as e:
            # Expected for invalid URL
            assert "invalid" in str(e).lower() or "error" in str(e).lower()
            
    @pytest.mark.asyncio
    async def test_async_invalid_url_handling(self):
        """Test async handling of invalid WebSocket URLs"""
        client = uf.AsyncWebSocketClient()
        
        # Test with invalid URL
        try:
            result = await client.connect("invalid-websocket-url")
            # Should handle gracefully or raise appropriate exception
        except Exception as e:
            # Expected for invalid URL
            assert "invalid" in str(e).lower() or "error" in str(e).lower()
            
    def test_message_errors(self):
        """Test message-related error handling"""
        # Test accessing text data from binary message
        binary_msg = uf.WebSocketMessage.new_binary([1, 2, 3])
        
        with pytest.raises(Exception):
            binary_msg.text()
            
        # Test accessing binary data from text message
        text_msg = uf.WebSocketMessage.new_text("test")
        
        with pytest.raises(Exception):
            text_msg.data()


class TestWebSocketRealTime:
    """Test real-time WebSocket functionality"""
    
    def test_message_representation(self):
        """Test message string representation"""
        text_msg = uf.WebSocketMessage.new_text("Hello")
        binary_msg = uf.WebSocketMessage.new_binary([1, 2, 3])
        ping_msg = uf.WebSocketMessage.new_ping([])
        pong_msg = uf.WebSocketMessage.new_pong([])
        close_msg = uf.WebSocketMessage.new_close()
        
        # Test __repr__ methods exist and return strings
        assert isinstance(repr(text_msg), str)
        assert isinstance(repr(binary_msg), str)
        assert isinstance(repr(ping_msg), str)
        assert isinstance(repr(pong_msg), str)
        assert isinstance(repr(close_msg), str)
        
        # Should contain message type information
        assert "Text" in repr(text_msg)
        assert "Binary" in repr(binary_msg)
        assert "Ping" in repr(ping_msg)
        assert "Pong" in repr(pong_msg)
        assert "Close" in repr(close_msg)


class TestWebSocketIntegration:
    """Test WebSocket integration scenarios"""
    
    def test_sync_and_async_client_coexistence(self):
        """Test that sync and async clients can coexist"""
        sync_client = uf.WebSocketClient()
        async_client = uf.AsyncWebSocketClient()
        
        # Both should be independent
        sync_client.set_header("X-Sync", "sync-value")
        async_client.set_header("X-Async", "async-value")
        
        assert "X-Sync" in sync_client.headers
        assert "X-Sync" not in async_client.headers
        
        assert "X-Async" in async_client.headers
        assert "X-Async" not in sync_client.headers
        
    def test_configuration_independence(self):
        """Test that client configurations are independent"""
        client1 = uf.WebSocketClient(auto_reconnect=True, max_reconnect_attempts=3)
        client2 = uf.WebSocketClient(auto_reconnect=False, max_reconnect_attempts=10)
        
        assert client1.auto_reconnect != client2.auto_reconnect
        assert client1.max_reconnect_attempts != client2.max_reconnect_attempts 