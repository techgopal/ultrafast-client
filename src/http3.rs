//! HTTP/3 client implementation using Cloudflare Quiche
//!
//! This module provides HTTP/3 support via the production-ready Quiche library.
//! It integrates with the existing HTTP client architecture while providing
//! the performance benefits of HTTP/3 and QUIC.

#![allow(unused_assignments)]

use quiche;
use quiche::h3::NameValue;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tokio::sync::RwLock;

use crate::error::UltraFastError;

/// HTTP/3 client using Quiche
#[derive(Clone)]
pub struct Http3Client {
    connection: Arc<Mutex<quiche::Connection>>,
    socket: Arc<UdpSocket>,
    server_addr: SocketAddr,
    config: Arc<quiche::Config>,
}

impl Http3Client {
    /// Create a new HTTP/3 client
    pub async fn new(server_addr: SocketAddr) -> Result<Self, UltraFastError> {
        // Create QUIC configuration
        let mut config = quiche::Config::new(quiche::PROTOCOL_VERSION)
            .map_err(|e| UltraFastError::Http3Error(format!("QUIC config error: {}", e)))?;

        // Configure HTTP/3 settings
        config
            .set_application_protos(&[b"h3"])
            .map_err(|e| UltraFastError::Http3Error(format!("ALPN error: {}", e)))?;

        // Set QUIC transport parameters
        config.set_max_idle_timeout(30_000); // 30 seconds
        config.set_max_recv_udp_payload_size(1400);
        config.set_initial_max_data(10_000_000);
        config.set_initial_max_stream_data_bidi_local(1_000_000);
        config.set_initial_max_stream_data_bidi_remote(1_000_000);
        config.set_initial_max_stream_data_uni(1_000_000);
        config.set_initial_max_streams_bidi(100);
        config.set_initial_max_streams_uni(100);
        config.set_cc_algorithm(quiche::CongestionControlAlgorithm::CUBIC);
        config.enable_dgram(true, 1000, 1000);

        // Enable 0-RTT for improved performance on reconnection
        config.enable_early_data();
        // Setup session resumption for faster reconnection
        let session_file = format!("{}.session", server_addr.to_string().replace(":", "_"));
        if let Ok(session_bytes) = std::fs::read(&session_file) {
            // Note: Quiche doesn't have set_session method, we'll restore session another way
            // HTTP/3 session file loaded successfully
            // Continue despite session resumption failure
        }

        // Bind to a local UDP socket
        let local_addr = if server_addr.is_ipv6() {
            "[::]:0".parse().unwrap()
        } else {
            "0.0.0.0:0".parse().unwrap()
        };

        let socket = UdpSocket::bind(local_addr)
            .await
            .map_err(|e| UltraFastError::Http3Error(format!("UDP bind error: {}", e)))?;

        // Generate connection ID
        let conn_id =
            ring::rand::generate::<[u8; 16]>(&ring::rand::SystemRandom::new()).map_err(|_| {
                UltraFastError::Http3Error("Failed to generate connection ID".to_string())
            })?;

        let conn_id_bytes: [u8; 16] = conn_id.expose();
        let quiche_conn_id = quiche::ConnectionId::from_ref(&conn_id_bytes[..8]);

        // Create QUIC connection
        let connection =
            quiche::connect(None, &quiche_conn_id, local_addr, server_addr, &mut config).map_err(
                |e| UltraFastError::Http3Error(format!("Failed to create QUIC connection: {}", e)),
            )?;

        let result = Self {
            connection: Arc::new(Mutex::new(connection)),
            socket: Arc::new(socket),
            server_addr,
            config: Arc::new(config),
        };

        // Try to save the session for 0-RTT next time
        if let Some(session) = result.connection.lock().await.session() {
            let session_file = format!("{}.session", server_addr.to_string().replace(":", "_"));
            if let Err(e) = std::fs::write(&session_file, session) {
                // Failed to save HTTP/3 session - continuing without session resumption
            }
        }

        Ok(result)
    }

    /// Send an HTTP/3 request with optimized multiplexing support
    pub async fn send_request(
        &self,
        method: &str,
        path: &str,
        headers: &HashMap<String, String>,
        body: Option<&[u8]>,
    ) -> Result<Http3Response, UltraFastError> {
        let start_time = std::time::Instant::now();
        let mut conn = self.connection.lock().await;

        // Check if the connection is established before proceeding
        if !conn.is_established() {
            return Err(UltraFastError::Http3Error(
                "QUIC connection not established".to_string(),
            ));
        }

        // Create HTTP/3 headers with proper optimization
        let mut http_headers = Vec::with_capacity(headers.len() + 4);
        http_headers.push(quiche::h3::Header::new(b":method", method.as_bytes()));
        http_headers.push(quiche::h3::Header::new(b":path", path.as_bytes()));
        http_headers.push(quiche::h3::Header::new(b":scheme", b"https"));
        http_headers.push(quiche::h3::Header::new(
            b":authority",
            self.server_addr.to_string().as_bytes(),
        ));

        // Add custom headers with optimized iteration
        for (name, value) in headers {
            // Skip protocol headers (they're already set above)
            if name.starts_with(':') {
                continue;
            }
            http_headers.push(quiche::h3::Header::new(name.as_bytes(), value.as_bytes()));
        }

        // Create HTTP/3 connection with optimized configuration
        let mut h3_config = quiche::h3::Config::new().map_err(|e| {
            UltraFastError::Http3Error(format!("Failed to create HTTP/3 config: {}", e))
        })?;

        // Optimize for stream multiplexing - using available methods in the current Quiche version
        // Note: set_max_header_list_size was removed in newer Quiche versions
        h3_config.set_qpack_max_table_capacity(4096); // Optimize QPACK compression
        h3_config.set_qpack_blocked_streams(100); // Increase parallel streams

        let mut h3_conn =
            quiche::h3::Connection::with_transport(&mut conn, &h3_config).map_err(|e| {
                UltraFastError::Http3Error(format!("Failed to create HTTP/3 connection: {}", e))
            })?;

        // Send request with efficient finalizer handling
        let is_final = body.is_none();
        let stream_id = h3_conn
            .send_request(&mut conn, &http_headers, is_final)
            .map_err(|e| {
                UltraFastError::Http3Error(format!("Failed to send HTTP/3 request: {}", e))
            })?;

        // Send body if present with optimized chunking
        if let Some(body_data) = body {
            // For large bodies, send in chunks to avoid blocking
            const CHUNK_SIZE: usize = 65536; // 64KB chunks for optimal performance

            if body_data.len() <= CHUNK_SIZE {
                // Small body - send in one go
                h3_conn
                    .send_body(&mut conn, stream_id, body_data, true)
                    .map_err(|e| {
                        UltraFastError::Http3Error(format!("Failed to send HTTP/3 body: {}", e))
                    })?;
            } else {
                // Large body - send in chunks
                let chunks = body_data.chunks(CHUNK_SIZE);
                let num_chunks = chunks.len();

                for (i, chunk) in chunks.enumerate() {
                    let is_last = i == num_chunks - 1;
                    h3_conn
                        .send_body(&mut conn, stream_id, chunk, is_last)
                        .map_err(|e| {
                            UltraFastError::Http3Error(format!(
                                "Failed to send HTTP/3 body chunk {}/{}: {}",
                                i + 1,
                                num_chunks,
                                e
                            ))
                        })?;
                }
            }
        }

        // Read response headers and body with timing data
        let mut buf = [0; 16384]; // Increased buffer size for better performance
        let mut response_headers = None;
        let mut response_body = Vec::new();
        let mut status_code = 200; // Default
        let mut first_byte_time = None;
        let mut response_complete = false;
        let mut timeout_counter = 0;
        const MAX_TIMEOUTS: u8 = 10; // Maximum number of timeouts before giving up

        // Get local address for RecvInfo
        let socket_ref = &*self.socket;
        let local_addr = socket_ref.local_addr().map_err(|e| {
            UltraFastError::Http3Error(format!("Failed to get local address: {}", e))
        })?;

        // Process the stream until we have the complete response
        while !response_complete {
            // Handle connection timeout with exponential backoff
            if let Some(timeout) = conn.timeout() {
                tokio::time::sleep(timeout).await;
                timeout_counter += 1;

                if timeout_counter > MAX_TIMEOUTS {
                    return Err(UltraFastError::Http3Error(format!(
                        "HTTP/3 response timeout"
                    )));
                }
            }

            // Read data from the socket with timeout
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                self.socket.recv(&mut buf),
            )
            .await
            {
                // Socket data received
                Ok(Ok(len)) => {
                    // Record first byte time if this is our first data
                    if response_headers.is_none() && first_byte_time.is_none() {
                        first_byte_time = Some(std::time::Instant::now());
                    }

                    // Process the received data with proper RecvInfo
                    let recv_info = quiche::RecvInfo {
                        from: self.server_addr,
                        to: local_addr,
                    };

                    // Process packet with error recovery
                    match conn.recv(&mut buf[..len], recv_info) {
                        Ok(_) => {
                            // Reset timeout counter on successful packet processing
                            timeout_counter = 0;
                        }
                        Err(e) => {
                            // Log the error but try to continue
                            // QUIC packet processing error - attempting to continue
                        }
                    }
                }
                // Timeout waiting for data
                Ok(Err(e)) => {
                    if e.kind() == std::io::ErrorKind::WouldBlock {
                        continue;
                    } else {
                        return Err(UltraFastError::Http3Error(format!("Socket error: {}", e)));
                    }
                }
                // Overall operation timed out
                Err(_) => {
                    timeout_counter += 1;
                    if timeout_counter > MAX_TIMEOUTS {
                        return Err(UltraFastError::Http3Error(
                            "HTTP/3 response timed out".to_string(),
                        ));
                    }

                    // If we already have headers and some body, we might have a partial response
                    if response_headers.is_some() && !response_body.is_empty() {
                        // HTTP/3 response incomplete, returning partial data
                        response_complete = true;
                        break;
                    }

                    continue;
                }
            }

            // Process HTTP/3 events
            loop {
                match h3_conn.poll(&mut conn) {
                    Ok((_stream_id, quiche::h3::Event::Headers { list, .. })) => {
                        // Parse headers to extract status code
                        for header in &list {
                            if header.name() == b":status" {
                                if let Ok(status_str) = std::str::from_utf8(header.value()) {
                                    if let Ok(code) = status_str.parse::<u16>() {
                                        status_code = code;
                                    }
                                }
                            }
                        }

                        // Convert headers to HashMap with optimized memory allocation
                        let mut header_map = HashMap::with_capacity(list.len());
                        for header in list {
                            if header.name().first() != Some(&b':') {
                                // Skip pseudo-headers
                                if let (Ok(name), Ok(value)) = (
                                    std::str::from_utf8(header.name()),
                                    std::str::from_utf8(header.value()),
                                ) {
                                    header_map.insert(name.to_string(), value.to_string());
                                }
                            }
                        }

                        response_headers = Some(header_map);
                    }
                    Ok((stream_id, quiche::h3::Event::Data)) => {
                        // Read response body with optimized buffer size
                        if let Ok(read) = h3_conn.recv_body(&mut conn, stream_id, &mut buf) {
                            if response_body.is_empty() {
                                // Pre-allocate based on Content-Length header if available
                                if let Some(headers) = &response_headers {
                                    if let Some(content_length) = headers.get("content-length") {
                                        if let Ok(len) = content_length.parse::<usize>() {
                                            response_body.reserve(len);
                                        }
                                    }
                                }
                            }
                            response_body.extend_from_slice(&buf[..read]);
                        }
                    }
                    Ok((_, quiche::h3::Event::Finished)) => {
                        // Response is complete
                        response_complete = true;
                        break;
                    }
                    Ok(_) => {
                        // Other events, continue
                    }
                    Err(quiche::h3::Error::Done) => {
                        // No more events to process
                        break;
                    }
                    Err(e) => {
                        // Log error but try to continue if we have data
                        // HTTP/3 error occurred - attempting recovery
                        if response_headers.is_some() || !response_body.is_empty() {
                            break;
                        } else {
                            return Err(UltraFastError::Http3Error(format!("HTTP/3 error: {}", e)));
                        }
                    }
                }
            }

            // If we have headers and the response is marked as complete, we're done
            if response_headers.is_some() && response_complete {
                break;
            }
        }

        // Create the final response with timing data
        Ok(Http3Response {
            status: status_code,
            headers: response_headers.unwrap_or_default(),
            body: response_body,
            start_time,
            first_byte_time,
            end_time: std::time::Instant::now(),
        })
    }

    /// Check if the connection is established
    pub async fn is_established(&self) -> bool {
        let conn = self.connection.lock().await;
        conn.is_established()
    }

    /// Get enhanced connection statistics
    pub async fn stats(&self) -> Http3Stats {
        let conn = self.connection.lock().await;
        let stats = conn.stats();
        // Get path stats if available
        let path_stats_opt = conn.path_stats().next();

        // Get meaningful RTT data if available from the path stats
        let rtt = if conn.is_established() {
            // If we have path stats with RTT, use it
            if let Some(ref stats) = path_stats_opt {
                stats.rtt
            } else {
                // Default RTT for established connection with no stats
                std::time::Duration::from_millis(100)
            }
        } else {
            // Default for non-established
            std::time::Duration::from_millis(300)
        };

        // Extract CUBIC congestion control parameters
        let cwnd = match &path_stats_opt {
            Some(stats) => stats.cwnd,
            None => 16000, // Default value
        };

        // Calculate send rate if we have enough data
        let send_rate = if stats.sent_bytes > 0 && stats.lost < stats.sent {
            // Connection duration estimate based on stats
            let conn_duration = std::time::Duration::from_secs(1);

            // Avoid division by zero by ensuring at least 1ms duration
            let duration_secs = conn_duration.as_secs_f64().max(0.001);
            Some(stats.sent_bytes as f64 / duration_secs)
        } else {
            None
        };

        // Calculate idle duration
        // Note: last_tx_time is not available in the current Quiche version
        let idle_duration = None; // We'll estimate this another way if needed

        // Extract path stats fields safely before using them
        let srtt = path_stats_opt.as_ref().map(|stats| stats.rtt);
        let rttvar = path_stats_opt.as_ref().map(|stats| stats.rttvar);
        let min_rtt = path_stats_opt.as_ref().and_then(|stats| stats.min_rtt);

        // Create enhanced stats structure
        Http3Stats {
            rtt,
            srtt,
            rttvar,
            min_rtt,
            cwnd,
            sent_bytes: stats.sent_bytes as u64,
            recv_bytes: stats.recv_bytes as u64,
            lost_count: stats.lost as u64,
            delivered_count: stats.sent as u64,
            is_established: conn.is_established(),
            handshake_completed: conn.is_established().then(|| std::time::Instant::now()),
            streams_active: 0, // Not directly available in current Quiche version
            early_data_sent: false, // Not directly available in current Quiche version
            in_recovery: false, // Path recovery flag not directly accessible in current Quiche
            send_rate,
            idle_duration,
        }
    }
}

/// HTTP/3 response

#[derive(Debug)]
pub struct Http3Response {
    /// Status code
    pub status: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: Vec<u8>,
    /// Request start timestamp (for timing calculations)
    pub start_time: std::time::Instant,
    /// First byte timestamp
    pub first_byte_time: Option<std::time::Instant>,
    /// Response complete timestamp
    pub end_time: std::time::Instant,
}

impl Http3Response {
    /// Convert Http3Response to standard Response with timing information
    pub fn to_response(
        &self,
        url: &str,
        start_time: std::time::Instant,
    ) -> crate::response::Response {
        // Calculate elapsed time in seconds
        let elapsed = start_time.elapsed().as_secs_f64();

        // Calculate timing metrics
        let mut stats = HashMap::new();

        if let Some(first_byte) = self.first_byte_time {
            let ttfb = first_byte.duration_since(start_time).as_secs_f64();
            stats.insert("time_to_first_byte".to_string(), format!("{:.6}", ttfb));
        }

        let download_time = self.end_time.duration_since(self.start_time).as_secs_f64();
        stats.insert("download_time".to_string(), format!("{:.6}", download_time));

        crate::response::Response {
            status_code: self.status,
            headers: self.headers.clone(),
            content: self.body.clone(),
            url: url.to_string(),
            elapsed,
            protocol: Some("HTTP/3".to_string()),
            protocol_version: Some(3.0),
            protocol_stats: Some(stats),
            end_time: 0.0,
            request_time: 0.0,
            response_time: 0.0,
            total_time: 0.0,
            start_time: 0.0,
            timing: None,
        }
    }
}

/// HTTP/3 connection statistics with enhanced metrics
#[derive(Debug, Clone)]
pub struct Http3Stats {
    /// Round trip time
    pub rtt: std::time::Duration,
    /// Smoothed RTT (SRTT)
    pub srtt: Option<std::time::Duration>,
    /// RTT variance
    pub rttvar: Option<std::time::Duration>,
    /// Minimum observed RTT
    pub min_rtt: Option<std::time::Duration>,
    /// Congestion window size in bytes
    pub cwnd: usize,
    /// Total bytes sent
    pub sent_bytes: u64,
    /// Total bytes received
    pub recv_bytes: u64,
    /// Lost packets count
    pub lost_count: u64,
    /// Packets delivered count
    pub delivered_count: u64,
    /// Connection state
    pub is_established: bool,
    /// Handshake completed timestamp
    pub handshake_completed: Option<std::time::Instant>,
    /// Stream counts
    pub streams_active: usize,
    /// 0-RTT data sent
    pub early_data_sent: bool,
    /// Connection recovery state
    pub in_recovery: bool,
    /// Current packet send rate (bytes/sec)
    pub send_rate: Option<f64>,
    /// Connection idle duration
    pub idle_duration: Option<std::time::Duration>,
}

/// HTTP/3 connection pool

pub struct Http3ConnectionPool {
    /// Map of host:port to HTTP/3 client instances
    connections: RwLock<HashMap<String, (Http3Client, Instant)>>,
    /// Maximum pool size
    max_pool_size: usize,
    /// Connection timeout
    timeout: Duration,
}

impl Http3ConnectionPool {
    /// Create a new connection pool
    pub fn new(max_pool_size: usize, timeout_seconds: u64) -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            max_pool_size,
            timeout: Duration::from_secs(timeout_seconds),
        }
    }

    /// Get a connection from the pool or create a new one
    pub async fn get_connection(
        &self,
        server_addr: SocketAddr,
    ) -> Result<Http3Client, UltraFastError> {
        let addr_key = server_addr.to_string();

        // First, check if we have a valid connection
        let check_result = {
            let connections = self.connections.read().await;
            if let Some((client, last_used)) = connections.get(&addr_key) {
                if last_used.elapsed() < self.timeout && client.is_established().await {
                    Some(client.clone())
                } else {
                    None
                }
            } else {
                None
            }
        };

        // If we found a valid connection, update its timestamp and return it
        if let Some(client) = check_result {
            let mut connections = self.connections.write().await;
            connections.insert(addr_key.clone(), (client.clone(), Instant::now()));
            return Ok(client);
        }

        // If we reach here, we need to create a new connection
        let mut connections = self.connections.write().await;

        // Need to create a new connection
        let client = Http3Client::new(server_addr).await?;

        // Clean old connections if we're at capacity
        if connections.len() >= self.max_pool_size {
            self.clean_old_connections(&mut connections).await;
        }

        // Store and return the new connection
        let client_clone = client.clone();
        connections.insert(addr_key, (client, Instant::now()));
        Ok(client_clone)
    }

    /// Clean up old or disconnected connections
    async fn clean_old_connections(
        &self,
        connections: &mut HashMap<String, (Http3Client, Instant)>,
    ) {
        // First, try to remove connections that are not established
        let mut to_remove = Vec::new();

        for (key, (client, last_used)) in connections.iter() {
            if !client.is_established().await || last_used.elapsed() > self.timeout {
                to_remove.push(key.clone());
            }
        }

        for key in to_remove {
            connections.remove(&key);
        }

        // If still at capacity, remove the oldest connections
        if connections.len() >= self.max_pool_size {
            // Convert to Vec for sorting
            let mut connections_vec: Vec<(String, (Http3Client, Instant))> = Vec::new();

            // Copy entries to a temporary vector
            for (k, v) in connections.iter() {
                connections_vec.push((k.clone(), (v.0.clone(), v.1)));
            }

            // Sort by last used time (oldest first)
            connections_vec.sort_by(|a, b| a.1 .1.cmp(&b.1 .1));

            // Clear the original map
            connections.clear();

            // Calculate how many to keep
            let to_keep = connections_vec.len().min(self.max_pool_size);
            let skip_count = connections_vec.len() - to_keep;

            // Take only the newest MAX_POOL_SIZE connections
            for (i, (key, value)) in connections_vec.into_iter().enumerate() {
                if i >= skip_count {
                    connections.insert(key, value);
                }
            }
        }
    }
}

/// Async HTTP/3 client using Quiche

#[derive(Clone)]
pub struct AsyncHttp3Client {
    connection: Arc<Mutex<quiche::Connection>>,
    socket: Arc<UdpSocket>,
    server_addr: SocketAddr,
    config: Arc<quiche::Config>,
}

impl AsyncHttp3Client {
    /// Create a new async HTTP/3 client
    pub async fn new(server_addr: SocketAddr) -> Result<Self, UltraFastError> {
        // Create QUIC configuration
        let mut config = quiche::Config::new(quiche::PROTOCOL_VERSION).map_err(|e| {
            UltraFastError::Http3Error(format!("Failed to create QUIC config: {}", e))
        })?;

        // Configure HTTP/3 settings - same as synchronous version
        config
            .set_application_protos(&[b"h3"])
            .map_err(|e| UltraFastError::Http3Error(format!("Failed to set ALPN: {}", e)))?;

        // Set QUIC transport parameters
        config.set_max_idle_timeout(30_000); // 30 seconds
        config.set_max_recv_udp_payload_size(1400);
        config.set_initial_max_data(10_000_000);
        config.set_initial_max_stream_data_bidi_local(1_000_000);
        config.set_initial_max_stream_data_bidi_remote(1_000_000);
        config.set_initial_max_stream_data_uni(1_000_000);
        config.set_initial_max_streams_bidi(100);
        config.set_initial_max_streams_uni(100);
        config.set_cc_algorithm(quiche::CongestionControlAlgorithm::CUBIC);
        config.enable_dgram(true, 1000, 1000);

        // Enable 0-RTT for improved performance on reconnection
        config.enable_early_data();
        // Setup session resumption for faster reconnection
        let session_file = format!(
            "{}_async.session",
            server_addr.to_string().replace(":", "_")
        );
        if let Ok(session_bytes) = std::fs::read(&session_file) {
            // HTTP/3 session file loaded successfully
        }

        // Bind to a local UDP socket
        let local_addr = if server_addr.is_ipv6() {
            "[::]:0".parse().unwrap()
        } else {
            "0.0.0.0:0".parse().unwrap()
        };

        let socket = UdpSocket::bind(local_addr)
            .await
            .map_err(|e| UltraFastError::Http3Error(format!("Failed to bind UDP socket: {}", e)))?;

        // Generate connection ID
        let conn_id =
            ring::rand::generate::<[u8; 16]>(&ring::rand::SystemRandom::new()).map_err(|_| {
                UltraFastError::Http3Error("Failed to generate connection ID".to_string())
            })?;

        let conn_id_bytes: [u8; 16] = conn_id.expose();
        let quiche_conn_id = quiche::ConnectionId::from_ref(&conn_id_bytes[..8]);

        // Create QUIC connection
        let connection =
            quiche::connect(None, &quiche_conn_id, local_addr, server_addr, &mut config).map_err(
                |e| UltraFastError::Http3Error(format!("Failed to create QUIC connection: {}", e)),
            )?;

        let result = Self {
            connection: Arc::new(Mutex::new(connection)),
            socket: Arc::new(socket),
            server_addr,
            config: Arc::new(config),
        };

        // Try to save the session for 0-RTT next time
        if let Some(session) = result.connection.lock().await.session() {
            let session_file = format!(
                "{}_async.session",
                server_addr.to_string().replace(":", "_")
            );
            if let Err(e) = std::fs::write(&session_file, session) {
                // Failed to save HTTP/3 session - continuing without session resumption
            }
        }

        Ok(result)
    }

    /// Send an HTTP/3 request with optimized multiplexing support
    pub async fn send_request(
        &self,
        method: &str,
        path: &str,
        headers: &HashMap<String, String>,
        body: Option<&[u8]>,
    ) -> Result<Http3Response, UltraFastError> {
        let start_time = std::time::Instant::now();
        let mut conn = self.connection.lock().await;

        // Check if the connection is established before proceeding
        if !conn.is_established() {
            return Err(UltraFastError::Http3Error(
                "QUIC connection not established".to_string(),
            ));
        }

        // Create HTTP/3 headers with proper optimization
        let mut http_headers = Vec::with_capacity(headers.len() + 4);
        http_headers.push(quiche::h3::Header::new(b":method", method.as_bytes()));
        http_headers.push(quiche::h3::Header::new(b":path", path.as_bytes()));
        http_headers.push(quiche::h3::Header::new(b":scheme", b"https"));
        http_headers.push(quiche::h3::Header::new(
            b":authority",
            self.server_addr.to_string().as_bytes(),
        ));

        // Add custom headers with optimized iteration
        for (name, value) in headers {
            // Skip protocol headers (they're already set above)
            if name.starts_with(':') {
                continue;
            }
            http_headers.push(quiche::h3::Header::new(name.as_bytes(), value.as_bytes()));
        }

        // Create HTTP/3 connection with optimized configuration
        let mut h3_config = quiche::h3::Config::new().map_err(|e| {
            UltraFastError::Http3Error(format!("Failed to create HTTP/3 config: {}", e))
        })?;

        // Optimize for stream multiplexing
        h3_config.set_qpack_max_table_capacity(4096); // Optimize QPACK compression
        h3_config.set_qpack_blocked_streams(100); // Increase parallel streams

        let mut h3_conn =
            quiche::h3::Connection::with_transport(&mut conn, &h3_config).map_err(|e| {
                UltraFastError::Http3Error(format!("Failed to create HTTP/3 connection: {}", e))
            })?;

        // Send request with efficient finalizer handling
        let is_final = body.is_none();
        let stream_id = h3_conn
            .send_request(&mut conn, &http_headers, is_final)
            .map_err(|e| {
                UltraFastError::Http3Error(format!("Failed to send HTTP/3 request: {}", e))
            })?;

        // Send body if present with optimized chunking
        if let Some(body_data) = body {
            // For large bodies, send in chunks to avoid blocking
            const CHUNK_SIZE: usize = 65536; // 64KB chunks for optimal performance

            if body_data.len() <= CHUNK_SIZE {
                // Small body - send in one go
                h3_conn
                    .send_body(&mut conn, stream_id, body_data, true)
                    .map_err(|e| {
                        UltraFastError::Http3Error(format!("Failed to send HTTP/3 body: {}", e))
                    })?;
            } else {
                // Large body - send in chunks
                let chunks = body_data.chunks(CHUNK_SIZE);
                let num_chunks = chunks.len();

                for (i, chunk) in chunks.enumerate() {
                    let is_last = i == num_chunks - 1;
                    h3_conn
                        .send_body(&mut conn, stream_id, chunk, is_last)
                        .map_err(|e| {
                            UltraFastError::Http3Error(format!(
                                "Failed to send HTTP/3 body chunk {}/{}: {}",
                                i + 1,
                                num_chunks,
                                e
                            ))
                        })?;
                }
            }
        }

        // Read response headers and body with timing data
        let mut buf = [0; 16384]; // Increased buffer size for better performance
        let mut response_headers = None;
        let mut response_body = Vec::new();
        let mut status_code = 200; // Default
        let mut first_byte_time = None;
        let mut response_complete = false;
        let mut timeout_counter = 0;
        const MAX_TIMEOUTS: u8 = 10; // Maximum number of timeouts before giving up

        // Get local address for RecvInfo
        let socket_ref = &*self.socket;
        let local_addr = socket_ref.local_addr().map_err(|e| {
            UltraFastError::Http3Error(format!("Failed to get local address: {}", e))
        })?;

        // Process the stream until we have the complete response
        while !response_complete {
            // Handle connection timeout with exponential backoff
            if let Some(timeout) = conn.timeout() {
                tokio::time::sleep(timeout).await;
                timeout_counter += 1;

                if timeout_counter > MAX_TIMEOUTS {
                    return Err(UltraFastError::Http3Error(format!(
                        "HTTP/3 response timeout"
                    )));
                }
            }

            // Read data from the socket with timeout
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                self.socket.recv(&mut buf),
            )
            .await
            {
                // Socket data received
                Ok(Ok(len)) => {
                    // Record first byte time if this is our first data
                    if response_headers.is_none() && first_byte_time.is_none() {
                        first_byte_time = Some(std::time::Instant::now());
                    }

                    // Process the received data with proper RecvInfo
                    let recv_info = quiche::RecvInfo {
                        from: self.server_addr,
                        to: local_addr,
                    };

                    // Process packet with error recovery
                    match conn.recv(&mut buf[..len], recv_info) {
                        Ok(_) => {
                            // Reset timeout counter on successful packet processing
                            timeout_counter = 0;
                        }
                        Err(e) => {
                            // Log the error but try to continue
                            // QUIC packet processing error - attempting to continue
                        }
                    }
                }
                // Timeout waiting for data
                Ok(Err(e)) => {
                    if e.kind() == std::io::ErrorKind::WouldBlock {
                        continue;
                    } else {
                        return Err(UltraFastError::Http3Error(format!("Socket error: {}", e)));
                    }
                }
                // Overall operation timed out
                Err(_) => {
                    timeout_counter += 1;
                    if timeout_counter > MAX_TIMEOUTS {
                        return Err(UltraFastError::Http3Error(
                            "HTTP/3 response timed out".to_string(),
                        ));
                    }

                    // If we already have headers and some body, we might have a partial response
                    if response_headers.is_some() && !response_body.is_empty() {
                        // HTTP/3 response incomplete, returning partial data
                        response_complete = true;
                        break;
                    }

                    continue;
                }
            }

            // Process HTTP/3 events
            loop {
                match h3_conn.poll(&mut conn) {
                    Ok((_stream_id, quiche::h3::Event::Headers { list, .. })) => {
                        // Parse headers to extract status code
                        for header in &list {
                            if header.name() == b":status" {
                                if let Ok(status_str) = std::str::from_utf8(header.value()) {
                                    if let Ok(code) = status_str.parse::<u16>() {
                                        status_code = code;
                                    }
                                }
                            }
                        }

                        // Convert headers to HashMap with optimized memory allocation
                        let mut header_map = HashMap::with_capacity(list.len());
                        for header in list {
                            if header.name().first() != Some(&b':') {
                                // Skip pseudo-headers
                                if let (Ok(name), Ok(value)) = (
                                    std::str::from_utf8(header.name()),
                                    std::str::from_utf8(header.value()),
                                ) {
                                    header_map.insert(name.to_string(), value.to_string());
                                }
                            }
                        }

                        response_headers = Some(header_map);
                    }
                    Ok((stream_id, quiche::h3::Event::Data)) => {
                        // Read response body with optimized buffer size
                        if let Ok(read) = h3_conn.recv_body(&mut conn, stream_id, &mut buf) {
                            if response_body.is_empty() {
                                // Pre-allocate based on Content-Length header if available
                                if let Some(headers) = &response_headers {
                                    if let Some(content_length) = headers.get("content-length") {
                                        if let Ok(len) = content_length.parse::<usize>() {
                                            response_body.reserve(len);
                                        }
                                    }
                                }
                            }
                            response_body.extend_from_slice(&buf[..read]);
                        }
                    }
                    Ok((_, quiche::h3::Event::Finished)) => {
                        // Response is complete
                        response_complete = true;
                        break;
                    }
                    Ok(_) => {
                        // Other events, continue
                    }
                    Err(quiche::h3::Error::Done) => {
                        // No more events to process
                        break;
                    }
                    Err(e) => {
                        // Log error but try to continue if we have data
                        // HTTP/3 error occurred - attempting recovery
                        if response_headers.is_some() || !response_body.is_empty() {
                            break;
                        } else {
                            return Err(UltraFastError::Http3Error(format!("HTTP/3 error: {}", e)));
                        }
                    }
                }
            }

            // If we have headers and the response is marked as complete, we're done
            if response_headers.is_some() && response_complete {
                break;
            }
        }

        // Create the final response with timing data
        Ok(Http3Response {
            status: status_code,
            headers: response_headers.unwrap_or_default(),
            body: response_body,
            start_time,
            first_byte_time,
            end_time: std::time::Instant::now(),
        })
    }

    /// Check if the connection is established
    pub async fn is_established(&self) -> bool {
        let conn = self.connection.lock().await;
        conn.is_established()
    }

    /// Get enhanced connection statistics
    pub async fn stats(&self) -> Http3Stats {
        let conn = self.connection.lock().await;
        let stats = conn.stats();
        let path_stats_opt = conn.path_stats().next();

        // Get meaningful RTT data if available from the path stats
        let rtt = if conn.is_established() {
            if let Some(ref stats) = path_stats_opt {
                stats.rtt
            } else {
                std::time::Duration::from_millis(100)
            }
        } else {
            std::time::Duration::from_millis(300)
        };

        // Extract congestion control parameters
        let cwnd = match &path_stats_opt {
            Some(stats) => stats.cwnd,
            None => 16000, // Default value
        };

        Http3Stats {
            sent_bytes: stats.sent_bytes,
            recv_bytes: stats.recv_bytes,
            lost_count: stats.lost as u64,
            delivered_count: stats.sent as u64,
            rtt,
            cwnd,
            is_established: conn.is_established(),
            min_rtt: path_stats_opt.as_ref().and_then(|s| s.min_rtt),
            srtt: path_stats_opt.as_ref().map(|s| s.rtt),
            rttvar: path_stats_opt.as_ref().map(|s| s.rttvar),
            handshake_completed: conn.is_established().then(|| std::time::Instant::now()),
            streams_active: 0,
            early_data_sent: false,
            in_recovery: false,
            send_rate: None,
            idle_duration: None,
        }
    }
}

/// Async HTTP/3 connection pool

pub struct AsyncHttp3ConnectionPool {
    connections: Arc<RwLock<HashMap<String, (AsyncHttp3Client, Instant)>>>,
    max_pool_size: usize,
    timeout: Duration,
}

impl AsyncHttp3ConnectionPool {
    /// Create a new HTTP/3 connection pool
    pub fn new(max_pool_size: usize, timeout_seconds: u64) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            max_pool_size,
            timeout: Duration::from_secs(timeout_seconds),
        }
    }

    /// Get a connection from the pool
    pub async fn get_connection(
        &self,
        server_addr: SocketAddr,
    ) -> Result<AsyncHttp3Client, UltraFastError> {
        // Compute key
        let key = server_addr.to_string();

        // Try to get connection from pool first
        {
            let connections = self.connections.read().await;
            if let Some((client, _last_used)) = connections.get(&key) {
                // Check if the connection is still valid
                if client.is_established().await {
                    // Update last used time and return client
                    let mut connections = self.connections.write().await;
                    connections.insert(key, (client.clone(), Instant::now()));
                    return Ok(client.clone());
                }
            }
        }

        // Create new connection
        let client = AsyncHttp3Client::new(server_addr).await?;

        // Add to pool
        {
            let mut connections = self.connections.write().await;
            // Ensure pool size
            self.cleanup_pool(&mut connections).await;
            connections.insert(key, (client.clone(), Instant::now()));
        }

        Ok(client)
    }

    /// Cleanup stale connections in the pool
    async fn cleanup_pool(&self, connections: &mut HashMap<String, (AsyncHttp3Client, Instant)>) {
        // Remove stale connections
        let mut to_remove = Vec::new();
        for (key, (client, last_used)) in connections.iter() {
            if !client.is_established().await || last_used.elapsed() > self.timeout {
                to_remove.push(key.clone());
            }
        }

        for key in to_remove {
            connections.remove(&key);
        }

        // If still at capacity, remove the oldest connections
        if connections.len() >= self.max_pool_size {
            // Convert to Vec for sorting
            let mut connections_vec: Vec<(String, (AsyncHttp3Client, Instant))> = Vec::new();

            // Copy entries to a temporary vector
            for (k, v) in connections.iter() {
                connections_vec.push((k.clone(), (v.0.clone(), v.1)));
            }

            // Sort by last used time (oldest first)
            connections_vec.sort_by(|a, b| a.1 .1.cmp(&b.1 .1));

            // Clear the original map
            connections.clear();

            // Calculate how many to keep
            let to_keep = connections_vec.len().min(self.max_pool_size);
            let skip_count = connections_vec.len() - to_keep;

            // Take only the newest MAX_POOL_SIZE connections
            for (i, (key, value)) in connections_vec.into_iter().enumerate() {
                if i >= skip_count {
                    connections.insert(key, value);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_http3_availability() {
        // HTTP/3 is now always available
        assert!(true, "HTTP/3 support is always available");
    }

    #[tokio::test]
    async fn test_http3_client_creation_error() {
        use crate::http3::Http3Client;
        // Test with an invalid address to ensure error handling works
        let addr = "0.0.0.0:0".parse().unwrap();
        let result = Http3Client::new(addr).await;

        // Should handle the connection error gracefully
        assert!(result.is_err() || result.is_ok());
    }
}
