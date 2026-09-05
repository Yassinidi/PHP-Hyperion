#!/usr/bin/env python3
"""
⚡ Real Dual-Engine Benchmark Suite: PHP 8.4 vs PHP-Hyperion
Measures:
  1. Requests & Throughput (RPS, Latency p50/p90/p99)
  2. Complex Database Queries & Window Functions
  3. Blade Views & HTML Rendering
  4. Real-time RAM / RSS Memory Profile
  5. Standalone CLI Compute & Code Execution
"""

import subprocess
import time
import json
import re
import os
import sys

PHP_PORT = 8000
HYPERION_PORT = 8001
AB_BIN = "/usr/sbin/ab"

def get_process_pids(pattern):
    try:
        out = subprocess.check_output(f"pgrep -f '{pattern}'", shell=True, text=True).strip()
        if out:
            return [int(p) for p in out.splitlines()]
    except subprocess.CalledProcessError:
        pass
    return []

def get_total_rss_mb(pids):
    if not pids:
        return 0.0
    total_kb = 0
    for pid in pids:
        try:
            out = subprocess.check_output(f"ps -o rss= -p {pid}", shell=True, text=True).strip()
            if out:
                total_kb += int(out)
        except Exception:
            pass
    return round(total_kb / 1024.0, 2)

def run_ab(url, total_requests=1000, concurrency=25):
    cmd = [AB_BIN, "-k", "-n", str(total_requests), "-c", str(concurrency), url]
    try:
        t0 = time.time()
        res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=60)
        wall_time = time.time() - t0
        output = res.stdout
        
        # Parse output
        rps = 0.0
        m = re.search(r"Requests per second:\s+([0-9.]+)", output)
        if m:
            rps = float(m.group(1))
            
        time_per_req = 0.0
        m = re.search(r"Time per request:\s+([0-9.]+)\s+\[ms\]\s+\(mean\)", output)
        if m:
            time_per_req = float(m.group(1))
            
        failed = 0
        m = re.search(r"Failed requests:\s+([0-9]+)", output)
        if m:
            failed = int(m.group(1))
            
        # Percentiles
        p50 = 0.0
        p90 = 0.0
        p99 = 0.0
        m50 = re.search(r"\s+50%\s+([0-9]+)", output)
        m90 = re.search(r"\s+90%\s+([0-9]+)", output)
        m99 = re.search(r"\s+99%\s+([0-9]+)", output)
        if m50: p50 = float(m50.group(1))
        if m90: p90 = float(m90.group(1))
        if m99: p99 = float(m99.group(1))
        
        return {
            "success": True,
            "rps": rps,
            "mean_ms": time_per_req,
            "p50_ms": p50,
            "p90_ms": p90,
            "p99_ms": p99,
            "failed": failed,
            "wall_time": round(wall_time, 2)
        }
    except Exception as e:
        return {
            "success": False,
            "error": str(e)
        }

def run_cli_benchmark(engine_cmd, script_path, iterations=5):
    times = []
    for _ in range(iterations):
        t0 = time.time()
        res = subprocess.run(f"{engine_cmd} {script_path}", shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        times.append(time.time() - t0)
    return {
        "min_time": round(min(times), 4),
        "avg_time": round(sum(times) / len(times), 4),
        "max_time": round(max(times), 4)
    }

def main():
    print("=" * 75)
    print("⚡ REAL DUAL-ENGINE BENCHMARK: PHP 8.4 (ZEND) vs PHP-HYPERION (RUST)")
    print("=" * 75)
    print(f"Target Application: Laravel 12 Complex Full-Stack Application")
    print(f"Machine Cores: {os.cpu_count()} Cores | Concurrency & Worker Parallelism")
    print("=" * 75 + "\n")

    php_pids = get_process_pids("php -S 127.0.0.1:8000")
    hyp_pids = get_process_pids("hyperion-cli -S 127.0.0.1:8001")

    print(f"Process Tracking:")
    print(f"  └─ PHP 8.4 (Zend Workers) PIDs: {php_pids} | Initial RSS: {get_total_rss_mb(php_pids)} MB")
    print(f"  └─ Hyperion (Multi-Threaded) PIDs: {hyp_pids} | Initial RSS: {get_total_rss_mb(hyp_pids)} MB\n")

    endpoints = [
        {
            "name": "1. API Health / Ingress Pipeline",
            "path": "/health",
            "requests": 500,
            "concurrency": 25,
            "desc": "Tests HTTP routing, kernel pipeline, JSON response"
        },
        {
            "name": "2. Eloquent Catalog API",
            "path": "/api/v1/products",
            "requests": 300,
            "concurrency": 15,
            "desc": "Tests Eloquent model hydration, relationships, serialization"
        },
        {
            "name": "3. Window Functions & Complex SQL",
            "path": "/api/v1/queries/window-rankings",
            "requests": 300,
            "concurrency": 15,
            "desc": "ROW_NUMBER() & AVG() OVER (PARTITION BY) SQLite queries"
        },
        {
            "name": "4. Subquery Joins (joinSub)",
            "path": "/api/v1/queries/joinsub-performance",
            "requests": 300,
            "concurrency": 15,
            "desc": "Subquery joins with order aggregates & grouping"
        },
        {
            "name": "5. Full Blade View Rendering (/complex)",
            "path": "/complex",
            "requests": 200,
            "concurrency": 10,
            "desc": "Full Blade compilation, layout inheritance, multi-query data"
        },
        {
            "name": "6. Storefront Catalog View (/)",
            "path": "/",
            "requests": 200,
            "concurrency": 10,
            "desc": "Catalog Blade view, pagination, service analytics"
        }
    ]

    benchmark_results = []

    for ep in endpoints:
        print(f"▶ Benchmarking: {ep['name']}")
        print(f"   URL: {ep['path']} ({ep['requests']} reqs, concurrency {ep['concurrency']}) - {ep['desc']}")
        
        # Test PHP 8.4
        php_url = f"http://127.0.0.1:{PHP_PORT}{ep['path']}"
        php_res = run_ab(php_url, ep['requests'], ep['concurrency'])
        php_rss = get_total_rss_mb(php_pids)
        
        # Test Hyperion
        hyp_url = f"http://127.0.0.1:{HYPERION_PORT}{ep['path']}"
        hyp_res = run_ab(hyp_url, ep['requests'], ep['concurrency'])
        hyp_rss = get_total_rss_mb(hyp_pids)

        benchmark_results.append({
            "name": ep["name"],
            "path": ep["path"],
            "requests": ep["requests"],
            "concurrency": ep["concurrency"],
            "php": php_res,
            "php_rss": php_rss,
            "hyperion": hyp_res,
            "hyperion_rss": hyp_rss
        })

        if php_res.get("success") and hyp_res.get("success"):
            print(f"   ├─ PHP 8.4   : {php_res['rps']:>8.1f} req/s | Mean: {php_res['mean_ms']:>6.1f} ms | p95: {php_res['p90_ms']:>4.0f} ms | RSS: {php_rss} MB")
            print(f"   └─ Hyperion  : {hyp_res['rps']:>8.1f} req/s | Mean: {hyp_res['mean_ms']:>6.1f} ms | p95: {hyp_res['p90_ms']:>4.0f} ms | RSS: {hyp_rss} MB\n")
        else:
            print(f"   ├─ PHP 8.4: {php_res}")
            print(f"   └─ Hyperion: {hyp_res}\n")

    # CLI Engine Compute Benchmark
    print("▶ Benchmarking Standalone CLI Engine Execution (5 iterations):")
    verify_script = "frameworks/laravel-complex/tests/verify_complex_engine.php"
    php_cli = run_cli_benchmark("php", verify_script, 5)
    hyp_cli = run_cli_benchmark("./php-hyperion/target/release/hyperion-cli", verify_script, 5)
    print(f"   ├─ PHP 8.4 CLI   : Avg: {php_cli['avg_time']}s | Min: {php_cli['min_time']}s | Max: {php_cli['max_time']}s")
    print(f"   └─ Hyperion CLI  : Avg: {hyp_cli['avg_time']}s | Min: {hyp_cli['min_time']}s | Max: {hyp_cli['max_time']}s\n")

    # Final Summary Table
    print("=" * 80)
    print("🏁 FINAL BENCHMARK SUMMARY")
    print("=" * 80)
    print(f"{'Endpoint':<35} | {'PHP 8.4 RPS':<12} | {'Hyperion RPS':<12} | {'PHP RAM':<9} | {'Hyperion RAM':<12}")
    print("-" * 80)
    for r in benchmark_results:
        p_rps = f"{r['php']['rps']:.1f}" if r['php'].get('success') else "ERR"
        h_rps = f"{r['hyperion']['rps']:.1f}" if r['hyperion'].get('success') else "ERR"
        p_ram = f"{r['php_rss']} MB"
        h_ram = f"{r['hyperion_rss']} MB"
        print(f"{r['name'][:35]:<35} | {p_rps:<12} | {h_rps:<12} | {p_ram:<9} | {h_ram:<12}")
    print("=" * 80)

    # Save results as JSON
    with open("benchmark_results.json", "w") as f:
        json.dump({
            "http_benchmarks": benchmark_results,
            "cli_benchmarks": {
                "php": php_cli,
                "hyperion": hyp_cli
            }
        }, f, indent=2)
    print("Saved results to benchmark_results.json")

if __name__ == "__main__":
    main()
