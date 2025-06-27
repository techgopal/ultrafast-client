use pyo3::prelude::*;
use std::collections::HashMap;
use std::time::Instant;
use tokio::runtime::Runtime;

/// Benchmarking utility for performance tests
#[pyclass]
pub struct Benchmark {
    results: HashMap<String, Vec<f64>>,
    runtime: Runtime,
}

#[pymethods]
impl Benchmark {
    #[new]
    pub fn new() -> PyResult<Self> {
        let runtime = Runtime::new().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create runtime: {}", e))
        })?;
        
        Ok(Benchmark {
            results: HashMap::new(),
            runtime,
        })
    }
    
    /// Add a new benchmark result
    pub fn add_result(&mut self, name: String, value: f64) {
        self.results.entry(name).or_insert_with(Vec::new).push(value);
    }
    
    /// Run a benchmark test
    pub fn run_test(&mut self, name: &str, iterations: u32, test_fn: &PyAny) -> PyResult<HashMap<String, f64>> {
        let mut times = Vec::new();
        
        for _ in 0..iterations {
            let start = Instant::now();
            test_fn.call0()?;
            let elapsed = start.elapsed().as_secs_f64();
            times.push(elapsed);
        }
        
        self.results.insert(name.to_string(), times.clone());
        
        // Calculate statistics
        let mut stats = HashMap::new();
        let sum: f64 = times.iter().sum();
        let mean = sum / times.len() as f64;
        
        stats.insert("mean".to_string(), mean);
        stats.insert("min".to_string(), *times.iter().min_by(|a, b| a.partial_cmp(b)
            .unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(&0.0));
        stats.insert("max".to_string(), *times.iter().max_by(|a, b| a.partial_cmp(b)
            .unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(&0.0));
        stats.insert("total".to_string(), sum);
        
        // Calculate median
        let mut sorted_times = times.clone();
        sorted_times.sort_by(|a, b| a.partial_cmp(b)
            .unwrap_or(std::cmp::Ordering::Equal));
        let median = if sorted_times.len() % 2 == 0 {
            (sorted_times[sorted_times.len() / 2 - 1] + sorted_times[sorted_times.len() / 2]) / 2.0
        } else {
            sorted_times[sorted_times.len() / 2]
        };
        stats.insert("median".to_string(), median);
        
        // Calculate percentiles
        let p95_idx = ((0.95 * sorted_times.len() as f64) as usize).min(sorted_times.len() - 1);
        let p99_idx = ((0.99 * sorted_times.len() as f64) as usize).min(sorted_times.len() - 1);
        stats.insert("p95".to_string(), sorted_times[p95_idx]);
        stats.insert("p99".to_string(), sorted_times[p99_idx]);
        
        Ok(stats)
    }
    
    /// Compare two benchmark results
    pub fn compare(&self, name1: &str, name2: &str) -> PyResult<HashMap<String, f64>> {
        let times1 = self.results.get(name1)
            .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err(format!("No results for '{}'", name1)))?;
        let times2 = self.results.get(name2)
            .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err(format!("No results for '{}'", name2)))?;
        
        let mean1: f64 = times1.iter().sum::<f64>() / times1.len() as f64;
        let mean2: f64 = times2.iter().sum::<f64>() / times2.len() as f64;
        
        let mut comparison = HashMap::new();
        comparison.insert("mean1".to_string(), mean1);
        comparison.insert("mean2".to_string(), mean2);
        comparison.insert("difference".to_string(), mean2 - mean1);
        comparison.insert("speedup".to_string(), mean1 / mean2);
        comparison.insert("percent_change".to_string(), ((mean2 - mean1) / mean1) * 100.0);
        
        Ok(comparison)
    }
    
    /// Get all results
    pub fn get_results(&self) -> HashMap<String, Vec<f64>> {
        self.results.clone()
    }
    
    /// Clear all results
    pub fn clear(&mut self) {
        self.results.clear();
    }
    
    /// Benchmark HTTP requests per second
    pub fn benchmark_rps(&mut self, url: &str, duration: f64) -> PyResult<HashMap<String, f64>> {
        let start = Instant::now();
        let mut request_count = 0;
        let mut errors = 0;
        
        let client = reqwest::blocking::Client::new();
        
        while start.elapsed().as_secs_f64() < duration {
            match client.get(url).send() {
                Ok(_) => request_count += 1,
                Err(_) => errors += 1,
            }
        }
        
        let total_time = start.elapsed().as_secs_f64();
        let rps = request_count as f64 / total_time;
        
        let mut stats = HashMap::new();
        stats.insert("requests_per_second".to_string(), rps);
        stats.insert("total_requests".to_string(), request_count as f64);
        stats.insert("errors".to_string(), errors as f64);
        stats.insert("error_rate".to_string(), errors as f64 / (request_count + errors) as f64);
        stats.insert("duration".to_string(), total_time);
        
        Ok(stats)
    }

    pub fn calculate_statistics(&self) -> PyResult<HashMap<String, f64>> {
        if self.results.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err("No benchmark data available"));
        }

        let times = &self.results.values().flatten().cloned().collect::<Vec<f64>>();
        let mut stats = HashMap::new();
        
        let sum: f64 = times.iter().sum();
        let mean = sum / times.len() as f64;
        
        stats.insert("count".to_string(), times.len() as f64);
        stats.insert("mean".to_string(), mean);
        stats.insert("min".to_string(), *times.iter().min_by(|a, b| a.partial_cmp(b)
            .unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(&0.0));
        stats.insert("max".to_string(), *times.iter().max_by(|a, b| a.partial_cmp(b)
            .unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(&0.0));
        stats.insert("total".to_string(), sum);
        
        // Calculate median
        let mut sorted_times = times.clone();
        sorted_times.sort_by(|a, b| a.partial_cmp(b)
            .unwrap_or(std::cmp::Ordering::Equal));
        let median = if sorted_times.len() % 2 == 0 {
            (sorted_times[sorted_times.len() / 2 - 1] + sorted_times[sorted_times.len() / 2]) / 2.0
        } else {
            sorted_times[sorted_times.len() / 2]
        };
        stats.insert("median".to_string(), median);
        
        // Calculate standard deviation
        let variance = times.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / times.len() as f64;
        let std_dev = variance.sqrt();
        stats.insert("std_dev".to_string(), std_dev);
        
        Ok(stats)
    }
}

/// Memory profiler for tracking allocations
#[pyclass]
pub struct MemoryProfiler {
    baseline: Option<usize>,
    measurements: Vec<(String, usize)>,
}

#[pymethods]
impl MemoryProfiler {
    #[new]
    pub fn new() -> Self {
        MemoryProfiler {
            baseline: None,
            measurements: Vec::new(),
        }
    }
    
    /// Set memory baseline
    pub fn set_baseline(&mut self) {
        self.baseline = Some(Self::get_current_memory());
    }
    
    /// Take a memory measurement
    pub fn measure(&mut self, label: &str) {
        let current = Self::get_current_memory();
        self.measurements.push((label.to_string(), current));
    }
    
    /// Get memory report
    pub fn report(&self) -> HashMap<String, f64> {
        let mut report = HashMap::new();
        
        if let Some(baseline) = self.baseline {
            report.insert("baseline_mb".to_string(), baseline as f64 / 1_048_576.0);
            
            for (label, measurement) in &self.measurements {
                let diff = *measurement as i64 - baseline as i64;
                report.insert(format!("{}_mb", label), *measurement as f64 / 1_048_576.0);
                report.insert(format!("{}_diff_mb", label), diff as f64 / 1_048_576.0);
            }
        }
        
        report
    }
    
    /// Clear measurements
    pub fn clear(&mut self) {
        self.baseline = None;
        self.measurements.clear();
    }
    
    /// Get current memory usage (simplified)
    #[staticmethod]
    fn get_current_memory() -> usize {
        // This is a simplified placeholder
        // In a real implementation, you'd use system APIs
        0
    }
}