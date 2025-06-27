"""
Comprehensive benchmark script comparing UltraFast HTTP Client vs popular Python HTTP libraries.
Includes: requests, urllib3, httpx (sync/async), aiohttp, and UltraFast (sync/async).
"""
import asyncio
import os
import statistics
import sys
import time
from typing import List

# Ensure python/ultrafast_client package is discoverable
sys.path.insert(
    0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "python"))
)

import aiohttp
import httpx
import requests
import urllib3
from ultrafast_client import AsyncHttpClient, HttpClient

# Test configuration
URL = "https://postman-echo.com/get"
NUM_REQUESTS = 100
NUM_RUNS = 3  # Multiple runs for statistical accuracy
CONCURRENT_LIMIT = 10  # For async tests to avoid overwhelming the server


class BenchmarkResult:
    """Container for benchmark results with statistical analysis."""

    def __init__(self, name: str, times: List[float]):
        self.name = name
        self.times = times
        self.mean_time = statistics.mean(times)
        self.median_time = statistics.median(times)
        self.std_dev = statistics.stdev(times) if len(times) > 1 else 0
        self.req_per_sec = NUM_REQUESTS / self.mean_time
        self.min_time = min(times)
        self.max_time = max(times)

    def __str__(self):
        return (
            f"{self.name:20} | "
            f"Mean: {self.mean_time:6.2f}s | "
            f"RPS: {self.req_per_sec:7.1f} | "
            f"±{self.std_dev:5.2f}s | "
            f"Range: {self.min_time:.2f}-{self.max_time:.2f}s"
        )


def run_multiple_times(benchmark_func, name: str) -> BenchmarkResult:
    """Run a benchmark function multiple times and collect statistics."""
    times = []
    print(f"Running {name}...")

    for run in range(NUM_RUNS):
        try:
            if asyncio.iscoroutinefunction(benchmark_func):
                loop = asyncio.get_event_loop()
                elapsed = loop.run_until_complete(benchmark_func())
            else:
                elapsed = benchmark_func()
            times.append(elapsed)
            print(f"  Run {run + 1}: {elapsed:.2f}s ({NUM_REQUESTS/elapsed:.1f} req/s)")
        except Exception as e:
            print(f"  Run {run + 1}: FAILED - {str(e)}")
            times.append(float("inf"))  # Mark as failed

    return BenchmarkResult(name, times)


# Synchronous benchmarks
def bench_sync_requests():
    """Benchmark using Python requests (synchronous)."""
    start = time.perf_counter()
    for _ in range(NUM_REQUESTS):
        r = requests.get(URL, timeout=30)
        r.raise_for_status()
        _ = r.text
    return time.perf_counter() - start


def bench_sync_urllib3():
    """Benchmark using urllib3 (synchronous)."""
    # Disable SSL warnings and verification for consistency with other tests
    urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)
    http = urllib3.PoolManager(cert_reqs="CERT_NONE", assert_hostname=False)
    start = time.perf_counter()
    for _ in range(NUM_REQUESTS):
        r = http.request("GET", URL, timeout=30)
        if r.status != 200:
            raise RuntimeError(f"Unexpected status: {r.status}")
        _ = r.data.decode("utf-8")
    return time.perf_counter() - start


def bench_sync_httpx():
    """Benchmark using httpx (synchronous)."""
    with httpx.Client(timeout=30.0) as client:
        start = time.perf_counter()
        for _ in range(NUM_REQUESTS):
            r = client.get(URL)
            r.raise_for_status()
            _ = r.text
    return time.perf_counter() - start


def bench_sync_ultrafast():
    """Benchmark using UltraFast HttpClient (synchronous)."""
    client = HttpClient()
    start = time.perf_counter()
    for _ in range(NUM_REQUESTS):
        r = client.get(URL)
        if r.status_code != 200:
            raise RuntimeError(f"Unexpected status: {r.status_code}")
        _ = r.text()
    return time.perf_counter() - start


# Asynchronous benchmarks
async def bench_async_httpx():
    """Benchmark using httpx (asynchronous)."""
    async with httpx.AsyncClient(timeout=30.0) as client:
        start = time.perf_counter()

        # Create semaphore to limit concurrent requests
        semaphore = asyncio.Semaphore(CONCURRENT_LIMIT)

        async def make_request():
            async with semaphore:
                r = await client.get(URL)
                r.raise_for_status()
                return r.text

        tasks = [make_request() for _ in range(NUM_REQUESTS)]
        await asyncio.gather(*tasks)
    return time.perf_counter() - start


async def bench_async_aiohttp():
    """Benchmark using aiohttp (asynchronous)."""
    connector = aiohttp.TCPConnector(ssl=False, limit=CONCURRENT_LIMIT)
    timeout = aiohttp.ClientTimeout(total=30)

    async with aiohttp.ClientSession(connector=connector, timeout=timeout) as session:
        start = time.perf_counter()

        async def make_request():
            async with session.get(URL) as resp:
                resp.raise_for_status()
                return await resp.text()

        tasks = [make_request() for _ in range(NUM_REQUESTS)]
        await asyncio.gather(*tasks)
    return time.perf_counter() - start


async def bench_async_ultrafast():
    """Benchmark using UltraFast AsyncHttpClient."""
    client = AsyncHttpClient()
    start = time.perf_counter()

    # Create semaphore to limit concurrent requests
    semaphore = asyncio.Semaphore(CONCURRENT_LIMIT)

    async def make_request():
        async with semaphore:
            r = await client.get(URL)
            if r.status_code != 200:
                raise RuntimeError(f"Unexpected status: {r.status_code}")
            return r.text()

    tasks = [make_request() for _ in range(NUM_REQUESTS)]
    await asyncio.gather(*tasks)
    return time.perf_counter() - start


def print_comparison_table(results: List[BenchmarkResult]):
    """Print a formatted comparison table of benchmark results."""
    print(f"\n{'='*80}")
    print(f"BENCHMARK RESULTS - {NUM_REQUESTS} requests x {NUM_RUNS} runs")
    print(f"{'='*80}")
    print(
        f"{'Library':20} | {'Mean Time':>10} | {'Req/Sec':>7} | {'Std Dev':>7} | {'Range':>15}"
    )
    print(f"{'-'*80}")

    # Sort by requests per second (descending)
    sorted_results = sorted(results, key=lambda r: r.req_per_sec, reverse=True)

    # Find the best performance for relative comparison
    best_rps = sorted_results[0].req_per_sec if sorted_results else 0

    for result in sorted_results:
        if result.req_per_sec == float("inf") or result.req_per_sec == 0:
            speedup = "FAILED"
        else:
            speedup = f"{result.req_per_sec/best_rps:.1f}x" if best_rps > 0 else "N/A"
        print(
            f"{result.name:20} | {result.mean_time:8.2f}s | {result.req_per_sec:7.1f} | ±{result.std_dev:5.2f}s | {result.min_time:.2f}-{result.max_time:.2f}s | {speedup}"
        )


def print_summary(results: List[BenchmarkResult]):
    """Print summary insights from benchmark results."""
    print(f"\n{'='*80}")
    print("PERFORMANCE INSIGHTS")
    print(f"{'='*80}")

    # Filter out failed results
    valid_results = [
        r for r in results if r.req_per_sec != float("inf") and r.req_per_sec > 0
    ]

    if not valid_results:
        print("No valid results to analyze.")
        return

    # Find fastest and slowest
    fastest = max(valid_results, key=lambda r: r.req_per_sec)
    slowest = min(valid_results, key=lambda r: r.req_per_sec)

    print(f"🏆 Fastest: {fastest.name} ({fastest.req_per_sec:.1f} req/s)")
    print(f"🐌 Slowest: {slowest.name} ({slowest.req_per_sec:.1f} req/s)")
    print(f"📊 Speed difference: {fastest.req_per_sec/slowest.req_per_sec:.1f}x")

    # Categorize by sync/async
    sync_results = [
        r
        for r in valid_results
        if not any(keyword in r.name.lower() for keyword in ["async", "aiohttp"])
    ]
    async_results = [
        r
        for r in valid_results
        if any(keyword in r.name.lower() for keyword in ["async", "aiohttp"])
    ]

    if sync_results:
        best_sync = max(sync_results, key=lambda r: r.req_per_sec)
        print(f"🔄 Best Sync: {best_sync.name} ({best_sync.req_per_sec:.1f} req/s)")

    if async_results:
        best_async = max(async_results, key=lambda r: r.req_per_sec)
        print(f"⚡ Best Async: {best_async.name} ({best_async.req_per_sec:.1f} req/s)")

    # UltraFast specific insights
    ultrafast_results = [r for r in valid_results if "ultrafast" in r.name.lower()]
    if ultrafast_results:
        print("\n🚀 UltraFast Performance:")
        for result in ultrafast_results:
            requests_result = next(
                (r for r in valid_results if "requests" in r.name.lower()), None
            )
            if requests_result:
                speedup = result.req_per_sec / requests_result.req_per_sec
                print(f"   {result.name}: {speedup:.1f}x faster than requests")


def create_detailed_report(results: List[BenchmarkResult]):
    """Create a detailed performance report."""
    print(f"\n{'='*80}")
    print("DETAILED PERFORMANCE ANALYSIS")
    print(f"{'='*80}")

    # Performance categories
    print("\n📊 PERFORMANCE CATEGORIES:")
    print(f"{'Category':20} | {'Libraries':50} | {'Best RPS':>10}")
    print(f"{'-'*80}")

    valid_results = [
        r for r in results if r.req_per_sec != float("inf") and r.req_per_sec > 0
    ]

    # Sync category
    sync_results = [
        r
        for r in valid_results
        if not any(keyword in r.name.lower() for keyword in ["async", "aiohttp"])
    ]
    if sync_results:
        best_sync = max(sync_results, key=lambda r: r.req_per_sec)
        sync_libs = ", ".join(
            [
                r.name
                for r in sorted(sync_results, key=lambda r: r.req_per_sec, reverse=True)
            ]
        )
        print(
            f"{'Synchronous':20} | {sync_libs[:48]:50} | {best_sync.req_per_sec:>8.1f}"
        )

    # Async category
    async_results = [
        r
        for r in valid_results
        if any(keyword in r.name.lower() for keyword in ["async", "aiohttp"])
    ]
    if async_results:
        best_async = max(async_results, key=lambda r: r.req_per_sec)
        async_libs = ", ".join(
            [
                r.name
                for r in sorted(
                    async_results, key=lambda r: r.req_per_sec, reverse=True
                )
            ]
        )
        print(
            f"{'Asynchronous':20} | {async_libs[:48]:50} | {best_async.req_per_sec:>8.1f}"
        )

    print("\n🎯 PERFORMANCE RECOMMENDATIONS:")

    if valid_results:
        fastest = max(valid_results, key=lambda r: r.req_per_sec)
        print(f"✅ For maximum performance: Use {fastest.name}")

        # Find most consistent performer
        consistent = min(
            valid_results,
            key=lambda r: r.std_dev / r.mean_time if r.mean_time > 0 else float("inf"),
        )
        print(f"🎯 For consistency: Use {consistent.name} (lowest variance)")

        # UltraFast specific recommendations
        ultrafast_sync = next(
            (
                r
                for r in valid_results
                if "ultrafast" in r.name.lower() and "sync" in r.name.lower()
            ),
            None,
        )
        ultrafast_async = next(
            (
                r
                for r in valid_results
                if "ultrafast" in r.name.lower() and "async" in r.name.lower()
            ),
            None,
        )

        if ultrafast_sync and ultrafast_async:
            async_advantage = ultrafast_async.req_per_sec / ultrafast_sync.req_per_sec
            print(f"⚡ UltraFast async is {async_advantage:.1f}x faster than sync mode")

            if async_advantage > 5:
                print(
                    "💡 Recommendation: Use UltraFast async for high-throughput applications"
                )
            else:
                print("💡 Both UltraFast modes are viable depending on your use case")


def print_system_info():
    """Print system information for benchmark context."""
    import platform
    import sys

    print(f"\n{'='*80}")
    print("SYSTEM INFORMATION")
    print(f"{'='*80}")
    print(f"🖥️  Platform: {platform.system()} {platform.release()}")
    print(f"🐍 Python: {sys.version.split()[0]}")
    print(f"💻 Architecture: {platform.machine()}")
    print(f"🔗 Test URL: {URL}")
    print(f"📊 Requests per test: {NUM_REQUESTS}")
    print(f"🔄 Number of runs: {NUM_RUNS}")
    print(f"⚡ Async concurrency: {CONCURRENT_LIMIT}")


def main():
    """Run comprehensive HTTP client benchmarks."""
    print("🚀 UltraFast HTTP Client Benchmark Suite")
    print(f"📊 Testing {NUM_REQUESTS} requests per run, {NUM_RUNS} runs each")
    print(f"🎯 Target: {URL}")
    print(f"⚡ Async concurrency limit: {CONCURRENT_LIMIT}")

    results = []

    # Synchronous benchmarks
    print(f"\n{'='*50}")
    print("SYNCHRONOUS BENCHMARKS")
    print(f"{'='*50}")

    benchmarks_sync = [
        (bench_sync_requests, "requests (sync)"),
        (bench_sync_urllib3, "urllib3 (sync)"),
        (bench_sync_httpx, "httpx (sync)"),
        (bench_sync_ultrafast, "ultrafast (sync)"),
    ]

    for benchmark_func, name in benchmarks_sync:
        try:
            result = run_multiple_times(benchmark_func, name)
            results.append(result)
        except Exception as e:
            print(f"❌ {name} failed: {str(e)}")
            results.append(BenchmarkResult(name, [float("inf")]))

    # Asynchronous benchmarks
    print(f"\n{'='*50}")
    print("ASYNCHRONOUS BENCHMARKS")
    print(f"{'='*50}")

    benchmarks_async = [
        (bench_async_httpx, "httpx (async)"),
        (bench_async_aiohttp, "aiohttp (async)"),
        (bench_async_ultrafast, "ultrafast (async)"),
    ]

    for benchmark_func, name in benchmarks_async:
        try:
            result = run_multiple_times(benchmark_func, name)
            results.append(result)
        except Exception as e:
            print(f"❌ {name} failed: {str(e)}")
            results.append(BenchmarkResult(name, [float("inf")]))

    # Print results
    print_comparison_table(results)
    print_summary(results)
    create_detailed_report(results)
    print_system_info()

    print(f"\n{'='*80}")
    print("✅ Benchmark completed!")
    print(f"{'='*80}")


if __name__ == "__main__":
    main()
