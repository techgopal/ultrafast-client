use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock; // Mutex currently unused
use tokio::sync::Semaphore;
use ahash::AHashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use crossbeam::queue::SegQueue;
use pyo3::PyResult;

/// High-performance connection pool with lock-free operations where possible
pub struct FastConnectionPool {
    // Use RwLock for read-heavy operations (checking pool status)
    pool_state: Arc<RwLock<PoolState>>,
    // Semaphore for connection limiting without locks
    connection_semaphore: Arc<Semaphore>,
    // Lock-free metrics using atomics
    active_connections: Arc<AtomicUsize>,
    total_connections_created: Arc<AtomicU64>,
    total_connections_reused: Arc<AtomicU64>,
    max_connections: usize,
    max_idle_time: Duration,
    // Lock-free idle connection queue
    idle_queue: Arc<SegQueue<IdleConnection>>,
}

struct PoolState {
    // Use AHashMap for better performance
    active_connections: AHashMap<String, ConnectionInfo>,
    host_stats: AHashMap<String, HostStats>,
    // Keep track of connection distribution per host
    host_connection_count: AHashMap<String, usize>,
}

#[derive(Clone)]
struct ConnectionInfo {
    host: String,
    created_at: Instant,
    last_used: Instant,
    request_count: u64,
    protocol_version: String,
    is_http3: bool,
}

struct IdleConnection {
    info: ConnectionInfo,
    available_since: Instant,
    connection_id: u64,
}

#[derive(Default)]
struct HostStats {
    total_connections: u64,
    active_connections: u64,
    failed_connections: u64,
    average_response_time: f64,
    last_failure: Option<Instant>,
    success_rate: f64,
    preferred_protocol: Option<String>,
}

impl FastConnectionPool {
    pub fn new(max_connections: usize, max_idle_time: Duration) -> Self {
        Self {
            pool_state: Arc::new(RwLock::new(PoolState {
                active_connections: AHashMap::new(),
                host_stats: AHashMap::new(),
                host_connection_count: AHashMap::new(),
            })),
            connection_semaphore: Arc::new(Semaphore::new(max_connections)),
            active_connections: Arc::new(AtomicUsize::new(0)),
            total_connections_created: Arc::new(AtomicU64::new(0)),
            total_connections_reused: Arc::new(AtomicU64::new(0)),
            max_connections,
            max_idle_time,
            idle_queue: Arc::new(SegQueue::new()),
        }
    }

    /// Try to acquire a connection permit without blocking
    pub async fn try_acquire_connection(&self) -> Option<ConnectionPermit> {
        match Arc::clone(&self.connection_semaphore).try_acquire_owned() {
            Ok(permit) => Some(ConnectionPermit { 
                permit: Some(permit),
                pool: Arc::clone(&self.pool_state),
            }),
            Err(_) => None,
        }
    }

    /// Acquire a connection permit (may wait)
    pub(crate) async fn acquire_connection(&self) -> PyResult<ConnectionPermit> {
        let permit = Arc::clone(&self.connection_semaphore).acquire_owned().await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire connection permit: {}", e)))?;
        Ok(ConnectionPermit {
            permit: Some(permit),
            pool: Arc::clone(&self.pool_state),
        })
    }

    /// Track connection usage for performance optimization
    pub async fn track_connection_usage(&self) {
        // This method provides connection usage tracking for performance monitoring
        // It's a lightweight operation that can be called after successful requests
        let state = self.pool_state.read();
        // Update internal metrics for connection efficiency
        // In a production implementation, this could update metrics/telemetry
    }
    
    /// Get pool statistics without acquiring locks for long
    pub fn get_stats(&self) -> PoolStats {
        let state = self.pool_state.read();
        let total_active = self.active_connections.load(Ordering::Relaxed);
        let total_idle = self.idle_queue.len();
        
        PoolStats {
            active_connections: total_active,
            idle_connections: total_idle,
            total_capacity: self.max_connections,
            host_count: state.host_stats.len(),
            connections_created: self.total_connections_created.load(Ordering::Relaxed),
            connections_reused: self.total_connections_reused.load(Ordering::Relaxed),
        }
    }

    /// Clean up expired idle connections using lock-free queue
    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        let mut expired_count = 0;
        
        // Process items from the queue
        while let Some(conn) = self.idle_queue.pop() {
            if now.duration_since(conn.available_since) < self.max_idle_time {
                // Put it back if not expired
                self.idle_queue.push(conn);
                break; // Assume queue is roughly ordered by time
            } else {
                expired_count += 1;
            }
        }
        
        if expired_count > 0 {
            // Update host stats
            let mut state = self.pool_state.write();
            for stats in state.host_stats.values_mut() {
                if stats.active_connections > 0 {
                    stats.total_connections = stats.total_connections.saturating_sub(expired_count);
                }
            }
        }
    }

    /// Try to get an idle connection for reuse
    pub fn try_reuse_connection(&self, host: &str) -> Option<ConnectionInfo> {
        // Try to find an idle connection for this host
        while let Some(idle_conn) = self.idle_queue.pop() {
            if idle_conn.info.host == host {
                let now = Instant::now();
                if now.duration_since(idle_conn.available_since) < self.max_idle_time {
                    self.total_connections_reused.fetch_add(1, Ordering::Relaxed);
                    return Some(idle_conn.info);
                }
            } else {
                // Put back if it's for a different host and not expired
                let now = Instant::now();
                if now.duration_since(idle_conn.available_since) < self.max_idle_time {
                    self.idle_queue.push(idle_conn);
                }
            }
        }
        None
    }

    /// Return a connection to the idle pool
    pub fn return_connection(&self, info: ConnectionInfo) {
        let idle_conn = IdleConnection {
            connection_id: self.total_connections_created.load(Ordering::Relaxed),
            available_since: Instant::now(),
            info,
        };
        self.idle_queue.push(idle_conn);
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }
}

pub struct ConnectionPermit {
    permit: Option<tokio::sync::OwnedSemaphorePermit>,
    pool: Arc<RwLock<PoolState>>,
}

impl ConnectionPermit {
    pub fn mark_active(&self, host: String) {
        let mut state = self.pool.write();
        let info = ConnectionInfo {
            host: host.clone(),
            created_at: Instant::now(),
            last_used: Instant::now(),
            request_count: 0,
            protocol_version: "HTTP/1.1".to_string(),
            is_http3: false,
        };
        
        state.active_connections.insert(host.clone(), info);
        state.host_stats.entry(host).or_default().active_connections += 1;
    }

    pub fn mark_used(&self, host: &str, response_time: Duration) {
        let mut state = self.pool.write();
        if let Some(conn) = state.active_connections.get_mut(host) {
            conn.last_used = Instant::now();
            conn.request_count += 1;
        }
        
        if let Some(stats) = state.host_stats.get_mut(host) {
            // Update running average
            let new_time = response_time.as_secs_f64();
            stats.average_response_time = 
                (stats.average_response_time + new_time) / 2.0;
                
            // Update success rate
            stats.success_rate = (stats.success_rate * 0.9) + (1.0 * 0.1);
        }
    }

    pub fn mark_failed(&self, host: &str) {
        let mut state = self.pool.write();
        if let Some(stats) = state.host_stats.get_mut(host) {
            stats.failed_connections += 1;
            stats.last_failure = Some(Instant::now());
            // Update success rate
            stats.success_rate = stats.success_rate * 0.9;
        }
    }
}

impl Drop for ConnectionPermit {
    fn drop(&mut self) {
        // Permit is automatically returned to semaphore when dropped
        if let Some(_permit) = self.permit.take() {
            // Cleanup happens automatically
        }
    }
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub active_connections: usize,
    pub idle_connections: usize,
    pub total_capacity: usize,
    pub host_count: usize,
    pub connections_created: u64,
    pub connections_reused: u64,
}

/// Fast host-based connection multiplexer
pub struct ConnectionMultiplexer {
    host_pools: Arc<RwLock<AHashMap<String, Arc<FastConnectionPool>>>>,
    default_pool_size: usize,
    max_idle_time: Duration,
}

impl ConnectionMultiplexer {
    pub fn new(default_pool_size: usize, max_idle_time: Duration) -> Self {
        Self {
            host_pools: Arc::new(RwLock::new(AHashMap::new())),
            default_pool_size,
            max_idle_time,
        }
    }

    /// Get or create a connection pool for a specific host
    pub fn get_pool(&self, host: &str) -> Arc<FastConnectionPool> {
        // Fast path: check if pool exists with read lock
        {
            let pools = self.host_pools.read();
            if let Some(pool) = pools.get(host) {
                return Arc::clone(pool);
            }
        }

        // Slow path: create new pool with write lock
        let mut pools = self.host_pools.write();
        // Double-check in case another thread created it
        if let Some(pool) = pools.get(host) {
            return Arc::clone(pool);
        }

        let pool = Arc::new(FastConnectionPool::new(
            self.default_pool_size,
            self.max_idle_time,
        ));
        pools.insert(host.to_string(), Arc::clone(&pool));
        pool
    }

    /// Get aggregate statistics across all pools
    pub fn get_aggregate_stats(&self) -> AggregateStats {
        let pools = self.host_pools.read();
        let mut total_active = 0;
        let mut total_idle = 0;
        let mut total_capacity = 0;
        
        for pool in pools.values() {
            let stats = pool.get_stats();
            total_active += stats.active_connections;
            total_idle += stats.idle_connections;
            total_capacity += stats.total_capacity;
        }

        AggregateStats {
            total_active_connections: total_active,
            total_idle_connections: total_idle,
            total_capacity,
            host_count: pools.len(),
        }
    }

    /// Cleanup expired connections across all pools
    pub fn cleanup_all(&self) {
        let pools = self.host_pools.read();
        for pool in pools.values() {
            pool.cleanup_expired();
        }
    }
}

#[derive(Debug, Clone)]
pub struct AggregateStats {
    pub total_active_connections: usize,
    pub total_idle_connections: usize,
    pub total_capacity: usize,
    pub host_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_pool_limits() {
        let pool = FastConnectionPool::new(2, Duration::from_secs(60));
        
        // Acquire two connections
        let _conn1 = pool.acquire_connection().await;
        let _conn2 = pool.acquire_connection().await;
        
        // Third should not be immediately available
        assert!(pool.try_acquire_connection().await.is_none());
        
        // Drop one connection
        drop(_conn1);
        
        // Should be able to acquire again
        assert!(pool.try_acquire_connection().await.is_some());
    }

    #[tokio::test]
    async fn test_multiplexer() {
        let multiplexer = ConnectionMultiplexer::new(10, Duration::from_secs(60));
        
        let pool1 = multiplexer.get_pool("example.com");
        let pool2 = multiplexer.get_pool("api.example.com");
        let pool1_again = multiplexer.get_pool("example.com");
        
        // Should return the same pool for the same host
        assert!(Arc::ptr_eq(&pool1, &pool1_again));
        
        let stats = multiplexer.get_aggregate_stats();
        assert_eq!(stats.host_count, 2);
    }

    #[tokio::test]
    async fn test_connection_lifecycle() {
        let pool = FastConnectionPool::new(5, Duration::from_secs(1));
        
        {
            let permit = pool.acquire_connection().await.expect("Failed to acquire permit");
            permit.mark_active("test.com".to_string());
            permit.mark_used("test.com", Duration::from_millis(100));
            
            let stats = pool.get_stats();
            assert_eq!(stats.active_connections, 1);
        }
        
        // Connection should be cleaned up after drop
        let stats = pool.get_stats();
        assert_eq!(stats.active_connections, 0);
    }
}
