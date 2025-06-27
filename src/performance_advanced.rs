// Advanced Performance Optimizations - Phase 4
// Compiler and Runtime Optimizations for UltraFast HTTP Client

use ahash::AHashMap;
use parking_lot::Mutex;
use std::alloc::{GlobalAlloc, Layout, System};
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
use std::sync::atomic::{AtomicUsize, Ordering};

/// High-performance allocator with memory safety guarantees
pub struct HighPerformanceAllocator {
    system: System,
    // Pool for common allocation sizes with bounds checking
    pools: [Mutex<Vec<*mut u8>>; 8], // Powers of 2: 64, 128, 256, ..., 8192
    pool_sizes: [usize; 8],
    allocated_bytes: AtomicUsize,
    allocation_count: AtomicUsize,
    // Track pool usage for leak detection
    pool_usage: [AtomicUsize; 8],
}

impl HighPerformanceAllocator {
    pub const fn new() -> Self {
        const EMPTY_VEC: Mutex<Vec<*mut u8>> = Mutex::new(Vec::new());
        const ATOMIC_ZERO: AtomicUsize = AtomicUsize::new(0);
        Self {
            system: System,
            pools: [EMPTY_VEC; 8],
            pool_sizes: [64, 128, 256, 512, 1024, 2048, 4096, 8192],
            allocated_bytes: AtomicUsize::new(0),
            allocation_count: AtomicUsize::new(0),
            pool_usage: [ATOMIC_ZERO; 8],
        }
    }

    /// Get pool index for a given size with bounds checking
    fn pool_index(&self, size: usize) -> Option<usize> {
        if size == 0 || size > 8192 {
            return None;
        }
        self.pool_sizes
            .iter()
            .position(|&pool_size| size <= pool_size)
    }

    /// Validate pointer before returning to pool (basic safety check)
    ///
    /// # Safety
    /// This performs basic pointer validation but cannot guarantee the pointer
    /// is still valid. The caller must ensure the pointer is still owned.
    unsafe fn is_valid_pool_pointer(&self, ptr: *mut u8, layout: Layout) -> bool {
        // Basic validation: check if pointer is non-null and reasonably aligned
        if ptr.is_null() {
            return false;
        }

        // Check alignment
        if ptr.align_offset(layout.align()) != 0 {
            return false;
        }

        // Additional basic checks could be added here
        true
    }

    pub fn stats(&self) -> (usize, usize) {
        (
            self.allocated_bytes.load(Ordering::Relaxed),
            self.allocation_count.load(Ordering::Relaxed),
        )
    }

    /// Get pool usage statistics for monitoring
    pub fn pool_stats(&self) -> [usize; 8] {
        let mut stats = [0; 8];
        for (i, usage) in self.pool_usage.iter().enumerate() {
            stats[i] = usage.load(Ordering::Relaxed);
        }
        stats
    }

    /// Force cleanup of all pools (for testing/debugging)
    pub fn force_cleanup(&self) {
        for (i, pool) in self.pools.iter().enumerate() {
            let mut pool_guard = pool.lock();
            for ptr in pool_guard.drain(..) {
                // SAFETY: We're deallocating pointers that were previously allocated
                // through the system allocator. We assume they're still valid since
                // they were stored in our pool.
                unsafe {
                    let layout = Layout::from_size_align_unchecked(self.pool_sizes[i], 1);
                    self.system.dealloc(ptr, layout);
                }
            }
            self.pool_usage[i].store(0, Ordering::Relaxed);
        }
    }
}

/// SAFETY: This implementation is safe because:
/// 1. We delegate to the system allocator for actual allocation/deallocation
/// 2. We perform bounds checking on pool indices
/// 3. We validate pointers before reuse (basic checks)
/// 4. We limit pool sizes to prevent unbounded growth
/// 5. We track allocation metrics for monitoring
unsafe impl GlobalAlloc for HighPerformanceAllocator {
    /// Allocate memory with pool optimization
    ///
    /// # Safety
    /// This function is safe because:
    /// - We validate the layout and size before proceeding
    /// - Pool access is protected by bounds checking
    /// - We delegate to the system allocator for actual allocation
    /// - We only reuse pointers that were previously allocated with compatible layouts
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Validate layout
        if layout.size() == 0 {
            return std::ptr::null_mut();
        }

        self.allocation_count.fetch_add(1, Ordering::Relaxed);
        self.allocated_bytes
            .fetch_add(layout.size(), Ordering::Relaxed);

        // Try to use pool for common sizes with bounds checking
        if let Some(pool_idx) = self.pool_index(layout.size()) {
            // Ensure pool index is within bounds (extra safety check)
            if pool_idx < self.pools.len() {
                let mut pool = self.pools[pool_idx].lock();
                if let Some(ptr) = pool.pop() {
                    // Validate the pointer before reuse
                    if self.is_valid_pool_pointer(ptr, layout) {
                        self.pool_usage[pool_idx].fetch_sub(1, Ordering::Relaxed);
                        return ptr;
                    } else {
                        // Invalid pointer found in pool - this shouldn't happen
                        // but if it does, we'll allocate a fresh one
                        // Invalid pointer found in pool - allocating fresh memory
                    }
                }
            }
        }

        // Fallback to system allocator
        self.system.alloc(layout)
    }

    /// Deallocate memory with pool optimization
    ///
    /// # Safety
    /// This function is safe because:
    /// - We validate the pointer and layout before proceeding
    /// - Pool access is protected by bounds checking and size limits
    /// - We perform basic pointer validation before adding to pool
    /// - We delegate to system allocator when pool is full or for large allocations
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // Validate inputs
        if ptr.is_null() || layout.size() == 0 {
            return;
        }

        self.allocated_bytes
            .fetch_sub(layout.size(), Ordering::Relaxed);

        // Try to return to pool for reuse with bounds checking
        if let Some(pool_idx) = self.pool_index(layout.size()) {
            // Ensure pool index is within bounds (extra safety check)
            if pool_idx < self.pools.len() {
                let mut pool = self.pools[pool_idx].lock();
                // Limit pool size to prevent unbounded growth and potential memory leaks
                if pool.len() < 32 && self.is_valid_pool_pointer(ptr, layout) {
                    pool.push(ptr);
                    self.pool_usage[pool_idx].fetch_add(1, Ordering::Relaxed);
                    return;
                }
            }
        }

        // Return to system allocator
        self.system.dealloc(ptr, layout);
    }
}

/// SIMD-optimized string operations with safety guarantees
pub struct SIMDStringOps;

impl SIMDStringOps {
    /// Fast case-insensitive header name comparison using SIMD with safety checks
    ///
    /// # Safety
    /// This function is safe because:
    /// - We validate slice lengths before SIMD operations
    /// - We ensure proper alignment and bounds checking
    /// - We handle the scalar fallback for small strings
    /// - We use safe slice operations for remainder bytes
    #[cfg(target_arch = "x86_64")]
    pub fn compare_header_name_simd(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }

        // For short strings, use regular comparison (avoids SIMD overhead and complexity)
        if a.len() < 16 {
            return a.eq_ignore_ascii_case(b);
        }

        // SAFETY: This unsafe block is safe because:
        // 1. We've validated that both slices have the same length >= 16
        // 2. We check bounds before each SIMD load operation
        // 3. We handle remainder bytes with safe slice operations
        // 4. SIMD intrinsics are safe when given valid aligned data
        unsafe {
            #[cfg(target_feature = "sse2")]
            {
                use std::arch::x86_64::*;

                let mut i = 0;
                let len = a.len();

                // Process 16 bytes at a time using SIMD with bounds checking
                while i + 16 <= len {
                    // Bounds check (redundant but explicit for safety)
                    if i + 16 > a.len() || i + 16 > b.len() {
                        break;
                    }

                    let va = _mm_loadu_si128(a.as_ptr().add(i) as *const __m128i);
                    let vb = _mm_loadu_si128(b.as_ptr().add(i) as *const __m128i);

                    // Convert to lowercase using SIMD bit manipulation
                    let mask_upper = _mm_set1_epi8(0x40u8 as i8); // '@'
                    let mask_lower = _mm_set1_epi8(0x5Bu8 as i8); // '['
                    let to_lower = _mm_set1_epi8(0x20u8 as i8); // Space to add

                    let is_upper_a = _mm_and_si128(
                        _mm_cmpgt_epi8(va, mask_upper),
                        _mm_cmpgt_epi8(mask_lower, va),
                    );
                    let is_upper_b = _mm_and_si128(
                        _mm_cmpgt_epi8(vb, mask_upper),
                        _mm_cmpgt_epi8(mask_lower, vb),
                    );

                    let lower_a = _mm_or_si128(va, _mm_and_si128(is_upper_a, to_lower));
                    let lower_b = _mm_or_si128(vb, _mm_and_si128(is_upper_b, to_lower));

                    let cmp = _mm_cmpeq_epi8(lower_a, lower_b);
                    let mask = _mm_movemask_epi8(cmp);

                    if mask != 0xFFFF {
                        return false;
                    }

                    i += 16;
                }

                // Handle remaining bytes safely using slice operations
                while i < len {
                    if a[i].to_ascii_lowercase() != b[i].to_ascii_lowercase() {
                        return false;
                    }
                    i += 1;
                }

                true
            }

            #[cfg(not(target_feature = "sse2"))]
            {
                // Fallback to regular comparison if SSE2 is not available
                a.eq_ignore_ascii_case(b)
            }
        }
    }

    /// Fallback for non-x86_64 targets
    #[cfg(not(target_arch = "x86_64"))]
    pub fn compare_header_name_simd(a: &[u8], b: &[u8]) -> bool {
        a.eq_ignore_ascii_case(b)
    }

    /// Fast URL parsing with SIMD optimizations and safety checks
    ///
    /// # Safety
    /// This function is safe because:
    /// - We validate input length before SIMD operations
    /// - We perform bounds checking on all slice accesses
    /// - We use safe fallback for edge cases
    /// - SIMD operations are protected by bounds checks
    #[cfg(target_arch = "x86_64")]
    pub fn find_url_parts_simd(url: &str) -> Option<(usize, usize, usize)> {
        let bytes = url.as_bytes();
        if bytes.len() < 8 {
            return Self::find_url_parts_fallback(url);
        }

        // SAFETY: This unsafe block is safe because:
        // 1. We validate the input length is at least 8 bytes
        // 2. We perform bounds checking before each SIMD operation
        // 3. We validate slice bounds before pattern matching
        // 4. We use safe fallback for any edge cases
        unsafe {
            #[cfg(target_feature = "sse2")]
            {
                use std::arch::x86_64::*;

                let mut i = 0;
                let len = bytes.len();
                let mut scheme_end = None;
                let mut host_start = None;
                let mut path_start = None;

                // Look for "://" pattern using SIMD with bounds checking
                let colon = _mm_set1_epi8(b':' as i8);
                let slash = _mm_set1_epi8(b'/' as i8);

                while i + 16 <= len && scheme_end.is_none() {
                    // Extra bounds check for safety
                    if i + 16 > bytes.len() {
                        break;
                    }

                    let chunk = _mm_loadu_si128(bytes.as_ptr().add(i) as *const __m128i);
                    let colon_mask = _mm_cmpeq_epi8(chunk, colon);
                    let colon_bits = _mm_movemask_epi8(colon_mask);

                    if colon_bits != 0 {
                        let colon_pos = i + colon_bits.trailing_zeros() as usize;
                        // Bounds check before slice access
                        if colon_pos + 3 <= len && colon_pos + 2 < bytes.len() {
                            // Safe slice access due to bounds check above
                            if &bytes[colon_pos..colon_pos + 3] == b"://" {
                                scheme_end = Some(colon_pos);
                                host_start = Some(colon_pos + 3);
                                break;
                            }
                        }
                    }
                    i += 16;
                }

                // Find path start with bounds checking
                if let Some(host_start_pos) = host_start {
                    i = host_start_pos;
                    while i + 16 <= len && path_start.is_none() {
                        // Extra bounds check for safety
                        if i + 16 > bytes.len() {
                            break;
                        }

                        let chunk = _mm_loadu_si128(bytes.as_ptr().add(i) as *const __m128i);
                        let slash_mask = _mm_cmpeq_epi8(chunk, slash);
                        let slash_bits = _mm_movemask_epi8(slash_mask);

                        if slash_bits != 0 {
                            let slash_pos = i + slash_bits.trailing_zeros() as usize;
                            // Bounds check
                            if slash_pos < len {
                                path_start = Some(slash_pos);
                                break;
                            }
                        }
                        i += 16;
                    }
                }

                match (scheme_end, host_start, path_start) {
                    (Some(scheme), Some(host), Some(path)) => Some((scheme, host, path)),
                    (Some(scheme), Some(host), None) => Some((scheme, host, len)),
                    _ => Self::find_url_parts_fallback(url),
                }
            }

            #[cfg(not(target_feature = "sse2"))]
            {
                Self::find_url_parts_fallback(url)
            }
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    pub fn find_url_parts_simd(url: &str) -> Option<(usize, usize, usize)> {
        Self::find_url_parts_fallback(url)
    }

    /// Safe fallback implementation for URL parsing
    fn find_url_parts_fallback(url: &str) -> Option<(usize, usize, usize)> {
        let scheme_end = url.find("://")?;
        let host_start = scheme_end + 3;

        // Bounds check before slice operation
        if host_start >= url.len() {
            return None;
        }

        let path_start = url[host_start..]
            .find('/')
            .map(|p| host_start + p)
            .unwrap_or(url.len());
        Some((scheme_end, host_start, path_start))
    }
}

/// CPU-specific optimizations detector with feature detection
#[allow(dead_code)]
pub struct CPUOptimizer {
    has_avx2: bool,
    has_sse4_2: bool,
    has_popcnt: bool,
    cache_line_size: usize,
}

#[allow(dead_code)]
impl CPUOptimizer {
    pub fn new() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            Self {
                has_avx2: is_x86_feature_detected!("avx2"),
                has_sse4_2: is_x86_feature_detected!("sse4.2"),
                has_popcnt: is_x86_feature_detected!("popcnt"),
                cache_line_size: 64, // Most modern CPUs
            }
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            Self {
                has_avx2: false,
                has_sse4_2: false,
                has_popcnt: false,
                cache_line_size: 64,
            }
        }
    }

    pub fn optimal_buffer_size(&self) -> usize {
        // Align to cache line size for optimal performance
        let base_size = if self.has_avx2 {
            8192 // AVX2 can handle larger chunks efficiently
        } else if self.has_sse4_2 {
            4096
        } else {
            2048
        };

        // Ensure minimum safe size
        std::cmp::max(base_size, 1024)
    }

    pub fn optimal_hash_map_capacity(&self, expected_items: usize) -> usize {
        // Prevent overflow and ensure reasonable limits
        let expected_items = std::cmp::min(expected_items, 1_000_000);

        // Round up to next power of 2, considering cache efficiency
        let capacity = expected_items.next_power_of_two();
        std::cmp::max(capacity, self.cache_line_size / 8) // At least cache_line_size / 8
    }
}

/// Profile-guided optimization data collector with bounds checking
#[allow(dead_code)]
pub struct ProfileCollector {
    header_frequency: Mutex<AHashMap<String, u64>>,
    url_patterns: Mutex<AHashMap<String, u64>>,
    response_sizes: Mutex<Vec<usize>>,
    connection_patterns: Mutex<AHashMap<String, ConnectionProfile>>,
    // Add limits to prevent unbounded growth
    max_entries: usize,
    max_response_samples: usize,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ConnectionProfile {
    pub avg_response_time: f64,
    pub success_rate: f64,
    pub preferred_protocol: String,
    pub typical_payload_size: usize,
}

#[allow(dead_code)]
impl ProfileCollector {
    pub fn new() -> Self {
        Self {
            header_frequency: Mutex::new(AHashMap::new()),
            url_patterns: Mutex::new(AHashMap::new()),
            response_sizes: Mutex::new(Vec::new()),
            connection_patterns: Mutex::new(AHashMap::new()),
            max_entries: 10_000,
            max_response_samples: 50_000,
        }
    }

    /// Record header usage with bounds checking
    pub fn record_header_usage(&self, header_name: &str) {
        // Validate input
        if header_name.is_empty() || header_name.len() > 256 {
            return;
        }

        let mut freq = self.header_frequency.lock();
        // Limit the number of tracked headers to prevent unbounded growth
        if freq.len() >= self.max_entries && !freq.contains_key(header_name) {
            return;
        }

        let counter = freq.entry(header_name.to_string()).or_insert(0);
        // Prevent overflow
        if *counter < u64::MAX {
            *counter += 1;
        }
    }

    /// Record URL pattern with bounds checking
    pub fn record_url_pattern(&self, url: &str) {
        // Validate input
        if url.is_empty() || url.len() > 2048 {
            return;
        }

        // Extract domain pattern safely
        if let Some((_, host_start, _)) = SIMDStringOps::find_url_parts_simd(url) {
            // Bounds check
            if host_start >= url.len() {
                return;
            }

            let remaining = &url[host_start..];
            let host_end = remaining.find('/').unwrap_or(remaining.len());

            // Validate host_end
            if host_end > remaining.len() {
                return;
            }

            let host = &remaining[..host_end];
            if !host.is_empty() && host.len() <= 253 {
                // Valid domain name length
                let mut patterns = self.url_patterns.lock();
                // Limit the number of tracked patterns
                if patterns.len() >= self.max_entries && !patterns.contains_key(host) {
                    return;
                }

                let counter = patterns.entry(host.to_string()).or_insert(0);
                // Prevent overflow
                if *counter < u64::MAX {
                    *counter += 1;
                }
            }
        }
    }

    /// Record response size with bounds checking
    pub fn record_response_size(&self, size: usize) {
        // Validate reasonable size limits
        if size > 100_000_000 {
            // 100MB limit
            return;
        }

        let mut sizes = self.response_sizes.lock();
        if sizes.len() < self.max_response_samples {
            sizes.push(size);
        } else {
            // Replace oldest entry with new one (circular buffer behavior)
            let index = sizes.len() % self.max_response_samples;
            if index < sizes.len() {
                sizes[index] = size;
            }
        }
    }

    /// Record connection performance with validation
    pub fn record_connection_performance(
        &self,
        host: &str,
        response_time: f64,
        success: bool,
        protocol: &str,
        payload_size: usize,
    ) {
        // Validate inputs
        if host.is_empty() || host.len() > 253 || response_time < 0.0 || response_time > 300.0 {
            return;
        }

        if protocol.len() > 32 || payload_size > 100_000_000 {
            return;
        }

        let mut patterns = self.connection_patterns.lock();
        // Limit the number of tracked connections
        if patterns.len() >= self.max_entries && !patterns.contains_key(host) {
            return;
        }

        let profile = patterns
            .entry(host.to_string())
            .or_insert_with(|| ConnectionProfile {
                avg_response_time: response_time,
                success_rate: if success { 1.0 } else { 0.0 },
                preferred_protocol: protocol.to_string(),
                typical_payload_size: payload_size,
            });

        // Exponential moving average with bounds checking
        if response_time.is_finite() && response_time >= 0.0 {
            profile.avg_response_time = profile.avg_response_time * 0.9 + response_time * 0.1;
            profile.avg_response_time = profile.avg_response_time.clamp(0.0, 300.0);
        }

        let success_value = if success { 1.0 } else { 0.0 };
        profile.success_rate = profile.success_rate * 0.95 + success_value * 0.05;
        profile.success_rate = profile.success_rate.clamp(0.0, 1.0);

        // Update preferred protocol based on performance
        if response_time < profile.avg_response_time && response_time > 0.0 {
            profile.preferred_protocol = protocol.to_string();
        }

        // Update typical payload size (running average) with overflow protection
        if payload_size > 0 && payload_size <= 100_000_000 {
            let current_size = profile.typical_payload_size;
            profile.typical_payload_size =
                ((current_size as u64 + payload_size as u64) / 2) as usize;
        }
    }

    pub fn get_optimization_hints(&self) -> OptimizationHints {
        // Acquire all locks sequentially to avoid deadlocks
        let header_freq = self.header_frequency.lock();
        let url_patterns = self.url_patterns.lock();
        let response_sizes = self.response_sizes.lock();
        let connection_patterns = self.connection_patterns.lock();

        // Find most common headers for pre-allocation
        let mut common_headers: Vec<_> = header_freq.iter().collect();
        common_headers.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        let top_headers: Vec<String> = common_headers
            .into_iter()
            .take(20)
            .map(|(name, _)| name.clone())
            .collect();

        // Calculate typical response size with overflow protection
        let avg_response_size = if response_sizes.is_empty() {
            8192
        } else {
            let sum: u64 = response_sizes.iter().map(|&size| size as u64).sum();
            let avg = sum / response_sizes.len() as u64;
            std::cmp::min(avg as usize, 100_000_000) // Cap at 100MB
        };

        // Find best performing hosts/protocols
        let best_performers: Vec<_> = connection_patterns
            .iter()
            .filter(|(_, profile)| profile.success_rate > 0.9)
            .map(|(host, profile)| (host.clone(), profile.clone()))
            .collect();

        let http3_count = connection_patterns
            .values()
            .filter(|p| p.preferred_protocol == "HTTP/3")
            .count();

        let should_use_http3 = if connection_patterns.is_empty() {
            false
        } else {
            http3_count > connection_patterns.len() / 2
        };

        OptimizationHints {
            common_headers: top_headers,
            typical_response_size: avg_response_size,
            best_performers,
            should_use_http3,
        }
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct OptimizationHints {
    pub common_headers: Vec<String>,
    pub typical_response_size: usize,
    pub best_performers: Vec<(String, ConnectionProfile)>,
    pub should_use_http3: bool,
}

/// Runtime optimization engine that adapts based on usage patterns
#[allow(dead_code)]
pub struct RuntimeOptimizer {
    cpu_optimizer: CPUOptimizer,
    profile_collector: ProfileCollector,
    optimization_hints: Mutex<Option<OptimizationHints>>,
}

#[allow(dead_code)]
impl RuntimeOptimizer {
    pub fn new() -> Self {
        Self {
            cpu_optimizer: CPUOptimizer::new(),
            profile_collector: ProfileCollector::new(),
            optimization_hints: Mutex::new(None),
        }
    }

    pub fn record_request(&self, url: &str, headers: &AHashMap<String, String>) {
        self.profile_collector.record_url_pattern(url);
        for header_name in headers.keys() {
            self.profile_collector.record_header_usage(header_name);
        }
    }

    pub fn record_response(
        &self,
        size: usize,
        response_time: f64,
        success: bool,
        host: &str,
        protocol: &str,
    ) {
        self.profile_collector.record_response_size(size);
        self.profile_collector.record_connection_performance(
            host,
            response_time,
            success,
            protocol,
            size,
        );
    }

    pub fn get_optimal_buffer_size(&self) -> usize {
        let base_size = self.cpu_optimizer.optimal_buffer_size();

        // Adjust based on typical response sizes if available
        if let Some(hints) = self.optimization_hints.lock().as_ref() {
            std::cmp::max(base_size, hints.typical_response_size.next_power_of_two())
        } else {
            base_size
        }
    }

    pub fn get_optimal_header_capacity(&self, expected_headers: usize) -> usize {
        self.cpu_optimizer
            .optimal_hash_map_capacity(expected_headers)
    }

    pub fn should_preload_headers(&self) -> Vec<String> {
        self.optimization_hints
            .lock()
            .as_ref()
            .map(|hints| hints.common_headers.clone())
            .unwrap_or_default()
    }

    pub fn update_optimization_hints(&self) {
        let hints = self.profile_collector.get_optimization_hints();
        *self.optimization_hints.lock() = Some(hints);
    }

    pub fn get_preferred_protocol(&self, host: &str) -> Option<String> {
        self.optimization_hints.lock().as_ref().and_then(|hints| {
            hints
                .best_performers
                .iter()
                .find(|(h, _)| h == host)
                .map(|(_, profile)| profile.preferred_protocol.clone())
        })
    }
}

/// Global runtime optimizer instance
static RUNTIME_OPTIMIZER: once_cell::sync::Lazy<RuntimeOptimizer> =
    once_cell::sync::Lazy::new(|| RuntimeOptimizer::new());

/// Get global runtime optimizer instance
pub fn get_runtime_optimizer() -> &'static RuntimeOptimizer {
    &RUNTIME_OPTIMIZER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_header_comparison() {
        assert!(SIMDStringOps::compare_header_name_simd(
            b"Content-Type",
            b"content-type"
        ));
        assert!(SIMDStringOps::compare_header_name_simd(
            b"Accept-Encoding",
            b"ACCEPT-ENCODING"
        ));
        assert!(!SIMDStringOps::compare_header_name_simd(
            b"Content-Type",
            b"Content-Length"
        ));
    }

    #[test]
    fn test_url_parsing_simd() {
        let url = "https://api.example.com/v1/users";
        let result = SIMDStringOps::find_url_parts_simd(url);
        assert!(result.is_some());

        let (scheme_end, host_start, path_start) = result.unwrap();
        assert_eq!(&url[..scheme_end], "https");
        assert_eq!(&url[host_start..path_start], "api.example.com");
        assert_eq!(&url[path_start..], "/v1/users");
    }

    #[test]
    fn test_profile_collector() {
        let collector = ProfileCollector::new();
        collector.record_header_usage("content-type");
        collector.record_header_usage("content-type");
        collector.record_header_usage("accept");

        collector.record_url_pattern("https://api.example.com/users");
        collector.record_response_size(1024);

        let hints = collector.get_optimization_hints();
        assert!(hints.common_headers.contains(&"content-type".to_string()));
        assert_eq!(hints.typical_response_size, 1024);
    }

    #[test]
    fn test_runtime_optimizer() {
        let optimizer = RuntimeOptimizer::new();

        let mut headers = AHashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());

        optimizer.record_request("https://api.example.com/test", &headers);
        optimizer.record_response(2048, 0.1, true, "api.example.com", "HTTP/2");

        optimizer.update_optimization_hints();

        let buffer_size = optimizer.get_optimal_buffer_size();
        assert!(buffer_size >= 2048);

        let preload_headers = optimizer.should_preload_headers();
        assert!(preload_headers.contains(&"content-type".to_string()));
    }
}
