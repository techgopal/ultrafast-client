use ahash::AHashMap;
use bytes::Bytes;
use once_cell::sync::Lazy;
use parking_lot::{Mutex, RwLock};
use pyo3::exceptions::PyRuntimeError;
use pyo3::PyResult;
use smallvec::SmallVec;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

/// High-performance header pool for reducing allocations
#[derive(Clone)]
#[allow(dead_code)]
pub struct HeaderPool {
    pool: Arc<Mutex<Vec<AHashMap<String, String>>>>,
    max_pool_size: usize,
    // Pool for frequently used header values
    value_pool: Arc<Mutex<Vec<String>>>,
    // Pre-allocated capacity for common use cases
    default_capacity: usize,
}

#[allow(dead_code)]
impl HeaderPool {
    const DEFAULT_POOL_SIZE: usize = 128;
    const DEFAULT_CAPACITY: usize = 16; // Most requests have < 16 headers

    pub fn new(max_pool_size: usize) -> Self {
        Self {
            pool: Arc::new(Mutex::new(Vec::with_capacity(max_pool_size))),
            value_pool: Arc::new(Mutex::new(Vec::with_capacity(max_pool_size * 8))),
            max_pool_size,
            default_capacity: Self::DEFAULT_CAPACITY,
        }
    }

    /// Get a reusable header map from the pool
    pub fn get(&self) -> PooledHeaders {
        let mut pool = self.pool.lock();
        let mut headers = pool
            .pop()
            .unwrap_or_else(|| AHashMap::with_capacity(self.default_capacity));
        headers.clear(); // Ensure it's clean

        PooledHeaders {
            headers,
            pool: Arc::clone(&self.pool),
            max_pool_size: self.max_pool_size,
        }
    }

    /// Get a reusable string from the value pool
    pub fn get_string(&self) -> PooledString {
        let mut value_pool = self.value_pool.lock();
        let mut string = value_pool.pop().unwrap_or_else(String::new);
        string.clear();

        PooledString {
            string,
            pool: Arc::clone(&self.value_pool),
            max_pool_size: self.max_pool_size,
        }
    }
}

/// A header map that returns to the pool when dropped
#[allow(dead_code)]
pub struct PooledHeaders {
    pub headers: AHashMap<String, String>,
    pool: Arc<Mutex<Vec<AHashMap<String, String>>>>,
    max_pool_size: usize,
}

#[allow(dead_code)]
impl PooledHeaders {
    /// Insert a header with zero-copy string operations where possible
    pub fn insert_cow(&mut self, key: Cow<'_, str>, value: Cow<'_, str>) {
        self.headers.insert(key.into_owned(), value.into_owned());
    }

    /// Get a header value
    pub fn get(&self, key: &str) -> Option<&String> {
        self.headers.get(key)
    }

    /// Insert a header
    pub fn insert(&mut self, key: String, value: String) {
        self.headers.insert(key, value);
    }
}

impl Drop for PooledHeaders {
    fn drop(&mut self) {
        let mut pool = self.pool.lock();
        if pool.len() < self.max_pool_size {
            // Return to pool only if we're under capacity
            let mut headers = std::mem::take(&mut self.headers);
            headers.clear();
            headers.shrink_to_fit(); // Free excess capacity
            pool.push(headers);
        }
    }
}

/// A string that returns to the pool when dropped
#[allow(dead_code)]
pub struct PooledString {
    pub string: String,
    pool: Arc<Mutex<Vec<String>>>,
    max_pool_size: usize,
}

#[allow(dead_code)]
impl PooledString {
    /// Push a string slice
    pub fn push_str(&mut self, s: &str) {
        self.string.push_str(s);
    }

    /// Get the string as a &str
    pub fn as_str(&self) -> &str {
        &self.string
    }
}

impl Drop for PooledString {
    fn drop(&mut self) {
        let mut pool = self.pool.lock();
        if pool.len() < self.max_pool_size {
            let mut string = std::mem::take(&mut self.string);
            string.clear();
            string.shrink_to_fit();
            pool.push(string);
        }
    }
}

/// Global header pool instance
#[allow(dead_code)]
static HEADER_POOL: Lazy<HeaderPool> = Lazy::new(|| HeaderPool::new(256));

/// Get a header map from the global pool
#[allow(dead_code)]
pub fn get_pooled_headers() -> PooledHeaders {
    HEADER_POOL.get()
}

/// Get a string from the global pool
#[allow(dead_code)]
pub fn get_pooled_string() -> PooledString {
    HEADER_POOL.get_string()
}

/// Fast header operations with string interning
#[allow(dead_code)]
pub struct HeaderCache {
    // Intern common header names for zero-allocation lookups
    header_names: Arc<RwLock<AHashMap<String, Arc<str>>>>,
    // Cache common header values
    header_values: Arc<RwLock<AHashMap<String, Arc<str>>>>,
}

#[allow(dead_code)]
impl HeaderCache {
    pub fn new() -> Self {
        let mut header_names = AHashMap::with_capacity(64);
        let mut header_values = AHashMap::with_capacity(128);

        // Pre-populate with common headers
        let common_headers = [
            "accept",
            "accept-encoding",
            "accept-language",
            "authorization",
            "cache-control",
            "connection",
            "content-encoding",
            "content-length",
            "content-type",
            "cookie",
            "date",
            "etag",
            "host",
            "if-match",
            "if-modified-since",
            "if-none-match",
            "last-modified",
            "location",
            "pragma",
            "server",
            "set-cookie",
            "transfer-encoding",
            "user-agent",
            "vary",
            "x-forwarded-for",
            "x-real-ip",
        ];

        for &header in &common_headers {
            header_names.insert(header.to_string(), Arc::from(header));
        }

        // Common header values
        let common_values = [
            "application/json",
            "text/html",
            "text/plain",
            "gzip",
            "deflate",
            "br",
            "close",
            "keep-alive",
            "no-cache",
            "max-age=0",
            "GET",
            "POST",
            "PUT",
            "DELETE",
            "HEAD",
            "OPTIONS",
            "PATCH",
        ];

        for &value in &common_values {
            header_values.insert(value.to_string(), Arc::from(value));
        }

        Self {
            header_names: Arc::new(RwLock::new(header_names)),
            header_values: Arc::new(RwLock::new(header_values)),
        }
    }

    /// Intern a header name, returning an Arc<str> for zero-copy operations
    pub fn intern_name(&self, name: &str) -> Arc<str> {
        // Fast path: try read lock first
        {
            let cache = self.header_names.read();
            if let Some(interned) = cache.get(name) {
                return Arc::clone(interned);
            }
        }

        // Slow path: acquire write lock and insert
        let mut cache = self.header_names.write();
        // Double-check after acquiring write lock
        if let Some(interned) = cache.get(name) {
            return Arc::clone(interned);
        }

        let interned = Arc::from(name);
        cache.insert(name.to_string(), Arc::clone(&interned));
        interned
    }

    /// Intern a header value
    pub fn intern_value(&self, value: &str) -> Arc<str> {
        // Fast path: try read lock first
        {
            let cache = self.header_values.read();
            if let Some(interned) = cache.get(value) {
                return Arc::clone(interned);
            }
        }

        // Slow path: acquire write lock and insert
        let mut cache = self.header_values.write();
        // Double-check after acquiring write lock
        if let Some(interned) = cache.get(value) {
            return Arc::clone(interned);
        }

        let interned = Arc::from(value);
        cache.insert(value.to_string(), Arc::clone(&interned));
        interned
    }

    /// Clear old entries to prevent unbounded growth
    pub fn trim_cache(&self, max_size: usize) {
        let mut names = self.header_names.write();
        let mut values = self.header_values.write();

        if names.len() > max_size {
            names.clear();
        }
        if values.len() > max_size {
            values.clear();
        }
    }

    /// Get a map of common headers with pre-allocated values
    pub fn get_common_headers(&self) -> std::collections::HashMap<String, String> {
        use std::collections::HashMap;
        let mut headers = HashMap::new();

        // Add common headers that are often used
        headers.insert(
            "User-Agent".to_string(),
            "UltraFast-Client/0.2.0".to_string(),
        );
        headers.insert("Accept".to_string(), "*/*".to_string());
        headers.insert(
            "Accept-Encoding".to_string(),
            "gzip, deflate, br".to_string(),
        );
        headers.insert("Connection".to_string(), "keep-alive".to_string());

        headers
    }
}

/// Global header cache instance
#[allow(dead_code)]
static HEADER_CACHE: Lazy<HeaderCache> = Lazy::new(HeaderCache::new);

/// Intern a header name using the global cache
#[allow(dead_code)]
pub fn intern_header(header: &str) -> Arc<str> {
    HEADER_CACHE.intern_name(header)
}

/// Intern a header value using the global cache
#[allow(dead_code)]
pub fn intern_value(value: &str) -> Arc<str> {
    HEADER_CACHE.intern_value(value)
}

/// Fast header builder using stack-allocated vectors for small header sets
#[allow(dead_code)]
pub struct FastHeaderBuilder {
    // Use SmallVec to avoid heap allocation for small header sets (< 8 headers)
    headers: SmallVec<[(Arc<str>, Arc<str>); 8]>,
    pooled_headers: Option<PooledHeaders>,
}

#[allow(dead_code)]
impl FastHeaderBuilder {
    pub fn new() -> Self {
        Self {
            headers: SmallVec::new(),
            pooled_headers: None,
        }
    }

    /// Add a header with string interning for performance
    pub fn add(&mut self, name: &str, value: &str) -> &mut Self {
        let interned_name = intern_header(name);
        let interned_value = intern_value(value);
        self.headers.push((interned_name, interned_value));
        self
    }

    /// Add a header without interning (for unique values)
    pub fn add_raw(&mut self, name: String, value: String) -> &mut Self {
        let name_arc = Arc::from(name.as_str());
        let value_arc = Arc::from(value.as_str());
        self.headers.push((name_arc, value_arc));
        self
    }

    /// Build into a HashMap for reqwest compatibility
    pub fn build(self) -> HashMap<String, String> {
        let mut map = HashMap::with_capacity(self.headers.len());
        for (name, value) in self.headers {
            map.insert(name.to_string(), value.to_string());
        }
        map
    }

    /// Build into pooled headers for memory efficiency
    pub fn build_pooled(self) -> PooledHeaders {
        let mut pooled = get_pooled_headers();
        for (name, value) in self.headers {
            pooled.insert(name.to_string(), value.to_string());
        }
        pooled
    }
}

impl Default for FastHeaderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Zero-copy response body wrapper for improved performance
#[derive(Clone)]
#[allow(dead_code)]
pub struct ZeroCopyBody {
    data: Bytes,
    content_type: Option<Arc<str>>,
}

#[allow(dead_code)]
impl ZeroCopyBody {
    pub fn new(data: Bytes, content_type: Option<&str>) -> Self {
        Self {
            data,
            content_type: content_type.map(|ct| intern_value(ct)),
        }
    }

    /// Get the raw bytes without copying
    pub fn bytes(&self) -> &Bytes {
        &self.data
    }

    /// Get the content type
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_ref().map(|ct| ct.as_ref())
    }

    /// Convert to text with zero-copy when possible
    pub fn text(&self) -> Result<Cow<str>, std::str::Utf8Error> {
        match std::str::from_utf8(&self.data) {
            Ok(s) => Ok(Cow::Borrowed(s)),
            Err(e) => Err(e),
        }
    }

    /// Try to parse as JSON with zero-copy string operations
    pub fn json<T>(&self) -> Result<T, serde_json::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        serde_json::from_slice(&self.data)
    }
}

/// Generic memory pool for any type
#[allow(dead_code)]
pub struct MemoryPool<T> {
    pool: Arc<Mutex<Vec<T>>>,
    factory: fn() -> T,
    max_size: usize,
}

#[allow(dead_code)]
impl<T> MemoryPool<T> {
    pub fn new(factory: fn() -> T, max_size: usize) -> Self {
        Self {
            pool: Arc::new(Mutex::new(Vec::with_capacity(max_size))),
            factory,
            max_size,
        }
    }

    pub fn get(&self) -> PooledItem<T> {
        let mut pool = self.pool.lock();
        let item = pool.pop().unwrap_or_else(self.factory);

        PooledItem {
            item: Some(item),
            pool: Arc::clone(&self.pool),
            max_size: self.max_size,
        }
    }
}

/// An item that returns to the memory pool when dropped
#[allow(dead_code)]
pub struct PooledItem<T> {
    item: Option<T>,
    pool: Arc<Mutex<Vec<T>>>,
    max_size: usize,
}

#[allow(dead_code)]
impl<T> PooledItem<T> {
    pub fn get(&self) -> PyResult<&T> {
        self.item
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("No item available"))
    }

    pub fn get_mut(&mut self) -> PyResult<&mut T> {
        self.item
            .as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("No item available"))
    }
}

impl<T> Drop for PooledItem<T> {
    fn drop(&mut self) {
        if let Some(item) = self.item.take() {
            let mut pool = self.pool.lock();
            if pool.len() < self.max_size {
                pool.push(item);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_pool() {
        let pool = HeaderPool::new(2);

        // Get a header map
        let mut headers1 = pool.get();
        headers1.insert("content-type".to_string(), "application/json".to_string());

        // Drop it back to pool
        drop(headers1);

        // Get another one (should reuse)
        let headers2 = pool.get();
        assert!(headers2.headers.is_empty()); // Should be clean
    }

    #[test]
    fn test_header_cache() {
        let cache = HeaderCache::new();

        let name1 = cache.intern_name("content-type");
        let name2 = cache.intern_name("content-type");

        // Should be the same Arc
        assert!(Arc::ptr_eq(&name1, &name2));
    }

    #[test]
    fn test_zero_copy_body() -> Result<(), Box<dyn std::error::Error>> {
        let data = Bytes::from("hello world");
        let body = ZeroCopyBody::new(data.clone(), Some("text/plain"));

        assert_eq!(body.bytes(), &data);
        assert_eq!(body.content_type(), Some("text/plain"));

        let text = body
            .text()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to get response text: {}", e)))?;
        assert_eq!(text, "hello world");

        Ok(())
    }
}
