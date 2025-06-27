"""
Comprehensive Test Suite for Session Management

Tests all features and functionality of both Session and AsyncSession including:
- Session state management
- Cookie persistence
- Header inheritance
- Authentication persistence
- Request methods with session state
- Configuration inheritance
- Context manager support
"""

import asyncio
import os
import tempfile
from typing import Any, Dict

import pytest
import ultrafast_client as uf


class TestSessionSync:
    """Test synchronous Session class"""

    @pytest.fixture
    def session(self):
        """Create a basic Session for testing"""
        return uf.Session(
            base_url="https://httpbin.org",
            headers={"User-Agent": "UltraFast-Session-Test"},
            persist_cookies=True,
        )

    @pytest.fixture
    def test_url(self):
        """Base URL for testing"""
        return "https://httpbin.org"

    def test_session_creation(self, session):
        """Test session creation with configuration"""
        assert session.base_url == "https://httpbin.org"
        assert session.persist_cookies == True

        # Check headers are set
        headers = session.headers
        assert "User-Agent" in headers
        assert headers["User-Agent"] == "UltraFast-Session-Test"

    def test_session_creation_with_auth(self):
        """Test session creation with authentication"""
        auth_config = uf.AuthConfig.basic("session_user", "session_pass")
        session = uf.Session(base_url="https://httpbin.org", auth_config=auth_config)

        assert session.auth_config is not None
        assert session.auth_config.auth_type == uf.AuthType.Basic

    def test_session_creation_with_timeouts(self):
        """Test session creation with timeout configuration"""
        timeout_config = uf.TimeoutConfig(connect_timeout=10.0, read_timeout=30.0)

        session = uf.Session(
            base_url="https://httpbin.org", timeout_config=timeout_config
        )

        assert session.timeout_config is not None

    def test_session_creation_with_pool_config(self):
        """Test session creation with pool configuration"""
        pool_config = uf.PoolConfig(max_idle_connections=20, max_idle_per_host=10)

        session = uf.Session(base_url="https://httpbin.org", pool_config=pool_config)

        assert session is not None

    def test_session_creation_with_ssl_config(self):
        """Test session creation with SSL configuration"""
        ssl_config = uf.SSLConfig(verify=False)

        session = uf.Session(base_url="https://httpbin.org", ssl_config=ssl_config)

        assert session is not None

    def test_get_request(self, session):
        """Test GET request with session"""
        response = session.get("/get")
        assert response.status_code == 200
        data = response.json()
        assert "httpbin.org" in data["url"]

        # Check that session headers are included
        assert data["headers"]["User-Agent"] == "UltraFast-Session-Test"

    def test_get_with_params(self, session):
        """Test GET request with parameters"""
        params = {"session_key": "session_value", "test": "params"}
        response = session.get("/get", params=params)
        assert response.status_code == 200
        data = response.json()
        assert data["args"]["session_key"] == "session_value"
        assert data["args"]["test"] == "params"

    def test_get_with_additional_headers(self, session):
        """Test GET request with additional headers"""
        headers = {"X-Session-Test": "session-header"}
        response = session.get("/get", headers=headers)
        assert response.status_code == 200
        data = response.json()

        # Both session headers and request headers should be present
        assert data["headers"]["User-Agent"] == "UltraFast-Session-Test"
        assert data["headers"]["X-Session-Test"] == "session-header"

    def test_post_request_json(self, session):
        """Test POST request with JSON data"""
        payload = {"session_data": "test", "value": 456}
        response = session.post("/post", json=payload)
        assert response.status_code == 200
        data = response.json()
        assert data["json"] == payload

    def test_post_request_form_data(self, session):
        """Test POST request with form data"""
        form_data = {"session_form": "session_value", "form_test": "data"}
        response = session.post("/post", data=form_data)
        assert response.status_code == 200
        data = response.json()
        assert data["form"]["session_form"] == "session_value"
        assert data["form"]["form_test"] == "data"

    def test_post_request_with_files(self, session):
        """Test POST request with file upload"""
        # Create a temporary file
        with tempfile.NamedTemporaryFile(
            mode="w", delete=False, suffix=".txt"
        ) as tmp_file:
            tmp_file.write("Session file upload test content")
            tmp_file_path = tmp_file.name

        try:
            with open(tmp_file_path, "rb") as f:
                files = {"session_file": f.read()}
                form_data = {"session_description": "Session file upload"}

                response = session.post("/post", data=form_data, files=files)
                assert response.status_code == 200
                data = response.json()
                assert data["form"]["session_description"] == "Session file upload"
                assert "session_file" in data["files"]
        finally:
            os.unlink(tmp_file_path)

    def test_put_request(self, session):
        """Test PUT request with session"""
        payload = {"session_update": "put_test", "value": 789}
        response = session.put("/put", json=payload)
        assert response.status_code == 200
        data = response.json()
        assert data["json"] == payload

    def test_patch_request(self, session):
        """Test PATCH request with session"""
        payload = {"session_patch": "patch_test"}
        response = session.patch("/patch", json=payload)
        assert response.status_code == 200
        data = response.json()
        assert data["json"] == payload

    def test_delete_request(self, session):
        """Test DELETE request with session"""
        response = session.delete("/delete")
        assert response.status_code == 200
        data = response.json()
        assert "url" in data

    def test_head_request(self, session):
        """Test HEAD request with session"""
        response = session.head("/get")
        assert response.status_code == 200
        # HEAD requests should not return body content
        assert len(response.text()) == 0 or response.text() == ""

    def test_options_request(self, session):
        """Test OPTIONS request with session"""
        response = session.options("/get")
        assert response.status_code == 200

    def test_session_header_management(self, session):
        """Test session header management"""
        # Set session header
        session.set_header("X-Session-Custom", "custom-session-value")

        response = session.get("/headers")
        assert response.status_code == 200
        data = response.json()
        assert data["headers"]["X-Session-Custom"] == "custom-session-value"

        # Remove session header
        session.remove_header("X-Session-Custom")

        response = session.get("/headers")
        assert response.status_code == 200
        data = response.json()
        assert "X-Session-Custom" not in data["headers"]

    def test_base_url_property(self, session):
        """Test base URL property access"""
        assert session.base_url == "https://httpbin.org"

        # Test setting new base URL
        session.set_base_url("https://example.com")
        assert session.base_url == "https://example.com"

    def test_auth_config_property(self, session):
        """Test auth config property access"""
        # Initially no auth
        assert session.auth_config is None

        # Set auth config
        auth_config = uf.AuthConfig.bearer("session-token-123")
        session.set_auth_config(auth_config)
        assert session.auth_config is not None
        assert session.auth_config.auth_type == uf.AuthType.Bearer

    def test_session_data_management(self, session):
        """Test session data storage"""
        # Set session data
        session.set_data("user_id", "session_user_123")
        session.set_data("preferences", "dark_mode")

        # Get session data
        assert session.get_data("user_id") == "session_user_123"
        assert session.get_data("preferences") == "dark_mode"
        assert session.get_data("nonexistent") is None

        # Remove session data
        session.remove_data("preferences")
        assert session.get_data("preferences") is None

        # Clear all session data
        session.clear_data()
        assert session.get_data("user_id") is None

    def test_context_manager(self, session):
        """Test session as context manager"""
        with session as s:
            assert s is session
            response = s.get("/get")
            assert response.status_code == 200


class TestAsyncSession:
    """Test asynchronous AsyncSession class"""

    @pytest.fixture
    def session(self):
        """Create a basic AsyncSession for testing"""
        return uf.AsyncSession(
            base_url="https://httpbin.org",
            headers={"User-Agent": "UltraFast-Async-Session-Test"},
            persist_cookies=True,
        )

    @pytest.fixture
    def test_url(self):
        """Base URL for testing"""
        return "https://httpbin.org"

    def test_async_session_creation(self, session):
        """Test async session creation with configuration"""
        assert session.base_url == "https://httpbin.org"
        assert session.persist_cookies == True

        # Check headers are set
        headers = session.session_headers
        assert "User-Agent" in headers
        assert headers["User-Agent"] == "UltraFast-Async-Session-Test"

    def test_async_session_creation_with_auth(self):
        """Test async session creation with authentication"""
        auth_config = uf.AuthConfig.basic("async_session_user", "async_session_pass")
        session = uf.AsyncSession(
            base_url="https://httpbin.org", auth_config=auth_config
        )

        assert session.auth_config is not None
        assert session.auth_config.auth_type == uf.AuthType.Basic

    def test_async_session_creation_with_timeouts(self):
        """Test async session creation with timeout configuration"""
        timeout_config = uf.TimeoutConfig(connect_timeout=15.0, read_timeout=45.0)

        session = uf.AsyncSession(
            base_url="https://httpbin.org", timeout_config=timeout_config
        )

        assert session.timeout_config is not None

    @pytest.mark.asyncio
    async def test_async_get_request(self, session):
        """Test async GET request with session"""
        response = await session.get("/get")
        assert response.status_code == 200
        data = response.json()
        assert "httpbin.org" in data["url"]

        # Check that session headers are included
        assert data["headers"]["User-Agent"] == "UltraFast-Async-Session-Test"

    @pytest.mark.asyncio
    async def test_async_get_with_params(self, session):
        """Test async GET request with parameters"""
        params = {"async_session_key": "async_session_value", "async_test": "params"}
        response = await session.get("/get", params=params)
        assert response.status_code == 200
        data = response.json()
        assert data["args"]["async_session_key"] == "async_session_value"
        assert data["args"]["async_test"] == "params"

    @pytest.mark.asyncio
    async def test_async_get_with_additional_headers(self, session):
        """Test async GET request with additional headers"""
        headers = {"X-Async-Session-Test": "async-session-header"}
        response = await session.get("/get", headers=headers)
        assert response.status_code == 200
        data = response.json()

        # Both session headers and request headers should be present
        assert data["headers"]["User-Agent"] == "UltraFast-Async-Session-Test"
        assert data["headers"]["X-Async-Session-Test"] == "async-session-header"

    @pytest.mark.asyncio
    async def test_async_post_request_json(self, session):
        """Test async POST request with JSON data"""
        payload = {"async_session_data": "async_test", "async_value": 456}
        response = await session.post("/post", json=payload)
        assert response.status_code == 200
        data = response.json()
        assert data["json"] == payload

    @pytest.mark.asyncio
    async def test_async_post_request_form_data(self, session):
        """Test async POST request with form data"""
        form_data = {
            "async_session_form": "async_session_value",
            "async_form_test": "data",
        }
        response = await session.post("/post", data=form_data)
        assert response.status_code == 200
        data = response.json()
        assert data["form"]["async_session_form"] == "async_session_value"
        assert data["form"]["async_form_test"] == "data"

    @pytest.mark.asyncio
    async def test_async_post_request_with_files(self, session):
        """Test async POST request with file upload"""
        # Create a temporary file
        with tempfile.NamedTemporaryFile(
            mode="w", delete=False, suffix=".txt"
        ) as tmp_file:
            tmp_file.write("Async session file upload test content")
            tmp_file_path = tmp_file.name

        try:
            with open(tmp_file_path, "rb") as f:
                files = {"async_session_file": f.read()}
                form_data = {"async_session_description": "Async session file upload"}

                response = await session.post("/post", data=form_data, files=files)
                assert response.status_code == 200
                data = response.json()
                assert (
                    data["form"]["async_session_description"]
                    == "Async session file upload"
                )
                assert "async_session_file" in data["files"]
        finally:
            os.unlink(tmp_file_path)

    @pytest.mark.asyncio
    async def test_async_put_request(self, session):
        """Test async PUT request with session"""
        payload = {"async_session_update": "async_put_test", "async_value": 789}
        response = await session.put("/put", json=payload)
        assert response.status_code == 200
        data = response.json()
        assert data["json"] == payload

    @pytest.mark.asyncio
    async def test_async_patch_request(self, session):
        """Test async PATCH request with session"""
        payload = {"async_session_patch": "async_patch_test"}
        response = await session.patch("/patch", json=payload)
        assert response.status_code == 200
        data = response.json()
        assert data["json"] == payload

    @pytest.mark.asyncio
    async def test_async_delete_request(self, session):
        """Test async DELETE request with session"""
        response = await session.delete("/delete")
        assert response.status_code == 200
        data = response.json()
        assert "url" in data

    @pytest.mark.asyncio
    async def test_async_head_request(self, session):
        """Test async HEAD request with session"""
        response = await session.head("/get")
        assert response.status_code == 200
        # HEAD requests should not return body content
        assert len(response.text()) == 0 or response.text() == ""

    @pytest.mark.asyncio
    async def test_async_options_request(self, session):
        """Test async OPTIONS request with session"""
        response = await session.options("/get")
        assert response.status_code == 200

    def test_async_session_header_management(self, session):
        """Test async session header management"""
        # Set session header
        session.set_header("X-Async-Session-Custom", "async-custom-session-value")

        # Session headers should be updated
        assert "X-Async-Session-Custom" in session.session_headers
        assert (
            session.session_headers["X-Async-Session-Custom"]
            == "async-custom-session-value"
        )

        # Remove session header
        session.remove_header("X-Async-Session-Custom")
        assert "X-Async-Session-Custom" not in session.session_headers

    def test_async_base_url_property(self, session):
        """Test async base URL property access"""
        assert session.base_url == "https://httpbin.org"

        # Test setting new base URL
        session.set_base_url("https://async.example.com")
        assert session.base_url == "https://async.example.com"

    def test_async_auth_config_property(self, session):
        """Test async auth config property access"""
        # Initially no auth
        assert session.auth_config is None

        # Set auth config
        auth_config = uf.AuthConfig.bearer("async-session-token-123")
        session.set_auth_config(auth_config)
        assert session.auth_config is not None
        assert session.auth_config.auth_type == uf.AuthType.Bearer

    def test_async_session_data_management(self, session):
        """Test async session data storage"""
        # Set session data
        session.set_data("async_user_id", "async_session_user_123")
        session.set_data("async_preferences", "async_dark_mode")

        # Get session data
        assert session.get_data("async_user_id") == "async_session_user_123"
        assert session.get_data("async_preferences") == "async_dark_mode"
        assert session.get_data("async_nonexistent") is None

        # Remove session data
        session.remove_data("async_preferences")
        assert session.get_data("async_preferences") is None

        # Clear all session data
        session.clear_data()
        assert session.get_data("async_user_id") is None


class TestSessionCookies:
    """Test session cookie management"""

    def test_cookie_persistence_enabled(self):
        """Test session with cookie persistence enabled"""
        session = uf.Session(base_url="https://httpbin.org", persist_cookies=True)

        assert session.persist_cookies == True

    def test_cookie_persistence_disabled(self):
        """Test session with cookie persistence disabled"""
        session = uf.Session(base_url="https://httpbin.org", persist_cookies=False)

        assert session.persist_cookies == False

    @pytest.mark.asyncio
    async def test_async_cookie_persistence_enabled(self):
        """Test async session with cookie persistence enabled"""
        session = uf.AsyncSession(base_url="https://httpbin.org", persist_cookies=True)

        assert session.persist_cookies == True

    @pytest.mark.asyncio
    async def test_async_cookie_persistence_disabled(self):
        """Test async session with cookie persistence disabled"""
        session = uf.AsyncSession(base_url="https://httpbin.org", persist_cookies=False)

        assert session.persist_cookies == False


class TestSessionErrorHandling:
    """Test session error handling"""

    def test_invalid_base_url(self):
        """Test session with invalid base URL"""
        session = uf.Session(base_url="invalid-url")

        # Should handle gracefully when making requests
        with pytest.raises(Exception):
            session.get("/test")

    @pytest.mark.asyncio
    async def test_async_invalid_base_url(self):
        """Test async session with invalid base URL"""
        session = uf.AsyncSession(base_url="invalid-url")

        # Should handle gracefully when making requests
        with pytest.raises(Exception):
            await session.get("/test")

    def test_network_error_handling(self):
        """Test session network error handling"""
        session = uf.Session(base_url="https://non-existent-domain-12345.com")

        with pytest.raises(Exception):
            session.get("/test")

    @pytest.mark.asyncio
    async def test_async_network_error_handling(self):
        """Test async session network error handling"""
        session = uf.AsyncSession(base_url="https://non-existent-domain-12345.com")

        with pytest.raises(Exception):
            await session.get("/test")


class TestSessionIntegration:
    """Test session integration scenarios"""

    def test_sync_and_async_session_coexistence(self):
        """Test that sync and async sessions can coexist"""
        sync_session = uf.Session(
            base_url="https://httpbin.org", headers={"X-Sync-Session": "sync-value"}
        )

        async_session = uf.AsyncSession(
            base_url="https://httpbin.org", headers={"X-Async-Session": "async-value"}
        )

        # Both should be independent
        assert "X-Sync-Session" in sync_session.headers
        assert "X-Async-Session" in async_session.session_headers

        # Headers should not interfere
        assert "X-Async-Session" not in sync_session.headers
        assert "X-Sync-Session" not in async_session.session_headers

    def test_session_with_different_configurations(self):
        """Test sessions with different configurations"""
        session1 = uf.Session(
            base_url="https://httpbin.org",
            persist_cookies=True,
            headers={"X-Session": "session1"},
        )

        session2 = uf.Session(
            base_url="https://example.com",
            persist_cookies=False,
            headers={"X-Session": "session2"},
        )

        # Both should maintain their own configuration
        assert session1.base_url != session2.base_url
        assert session1.persist_cookies != session2.persist_cookies
        assert session1.headers["X-Session"] != session2.headers["X-Session"]

    @pytest.mark.asyncio
    async def test_concurrent_async_sessions(self):
        """Test multiple async sessions running concurrently"""
        session1 = uf.AsyncSession(base_url="https://httpbin.org")
        session2 = uf.AsyncSession(base_url="https://httpbin.org")

        # Make concurrent requests with different sessions
        tasks = [
            session1.get("/delay/1"),
            session2.get("/delay/1"),
        ]

        responses = await asyncio.gather(*tasks)

        # Both responses should be successful
        for response in responses:
            assert response.status_code == 200
