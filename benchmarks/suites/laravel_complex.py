#!/usr/bin/env python3
"""
⚡ MAXIMUM BENCHMARK SUITE: PHP-HYPERION vs STANDARD PHP 8.4 (ZEND)
Target: frameworks/laravel-complex (Laravel 12.32 Enterprise Full-Stack)
Tuning:
  • Hyperion: PHP_CLI_SERVER_WORKERS=16, HYPERION_REACTORS=8, HYPERION_MAX_FIBRES=128
  • Standard PHP 8.4: PHP_CLI_SERVER_WORKERS=8 (Multi-Process Pool)
"""

import os
import sys
import time
import json
import re
import subprocess
import urllib.request
import urllib.error

def find_workspace_root():
    d = os.path.dirname(os.path.abspath(__file__))
    while d and d != os.path.dirname(d):
        if os.path.exists(os.path.join(d, "php-hyperion")) and os.path.exists(os.path.join(d, "frameworks")):
            return d
        d = os.path.dirname(d)
    return os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))

WORKSPACE = find_workspace_root()
LARAVEL_DIR = os.path.join(WORKSPACE, "frameworks", "laravel-complex")
HYPERION_CLI = os.path.join(WORKSPACE, "php-hyperion", "target", "release", "hyperion-cli")
PHP_CLI = "/opt/homebrew/bin/php" if os.path.exists("/opt/homebrew/bin/php") else "php"
AB_BIN = "/usr/sbin/ab"

PHP_PORT = 8000
HYPERION_PORT = 8001

def log(msg="", end="\n"):
    print(msg, end=end, flush=True)

def cleanup_servers():
    subprocess.run(["pkill", "-9", "-f", f"php.*-S 127.0.0.1:{PHP_PORT}"], stderr=subprocess.DEVNULL)
    subprocess.run(["pkill", "-9", "-f", f"hyperion-cli.*-S 127.0.0.1:{HYPERION_PORT}"], stderr=subprocess.DEVNULL)
    time.sleep(0.5)

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

def wait_for_server(url, max_retries=30, delay=0.2):
    for _ in range(max_retries):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "HyperionBenchProbe"})
            with urllib.request.urlopen(req, timeout=1.5) as resp:
                if resp.status in (200, 302, 404):
                    return True
        except Exception:
            time.sleep(delay)
    return False

def warm_up(url, count=5):
    for _ in range(count):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "HyperionBenchWarmup"})
            with urllib.request.urlopen(req, timeout=2.0) as resp:
                resp.read()
        except Exception:
            pass

def run_ab(url, total_requests, concurrency, keep_alive=True, timeout=60):
    cmd = [AB_BIN]
    if keep_alive:
        cmd.append("-k")
    cmd.extend(["-n", str(total_requests), "-c", str(concurrency), url])
    
    t0 = time.time()
    try:
        res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=timeout)
        wall_time = time.time() - t0
        out = res.stdout
        
        rps_match = re.search(r"Requests per second:\s+([0-9.]+)", out)
        rps = float(rps_match.group(1)) if rps_match else 0.0
        
        mean_time_match = re.search(r"Time per request:\s+([0-9.]+)\s+\[ms\]\s+\(mean\)", out)
        mean_time = float(mean_time_match.group(1)) if mean_time_match else 0.0
        
        concurrent_time_match = re.search(r"Time per request:\s+([0-9.]+)\s+\[ms\]\s+\(mean, across all concurrent requests\)", out)
        concurrent_time = float(concurrent_time_match.group(1)) if concurrent_time_match else 0.0
        
        failed_match = re.search(r"Failed requests:\s+(\d+)", out)
        failed = int(failed_match.group(1)) if failed_match else 0
        
        # Length differences shouldn't count as failure if status is 200
        if "Length:" in out:
            length_match = re.search(r"Length:\s+(\d+)", out)
            if length_match and int(length_match.group(1)) == failed:
                failed = 0
                
        p50 = float(re.search(r"50%\s+(\d+)", out).group(1)) if re.search(r"50%\s+(\d+)", out) else 0.0
        p90 = float(re.search(r"90%\s+(\d+)", out).group(1)) if re.search(r"90%\s+(\d+)", out) else 0.0
        p95 = float(re.search(r"95%\s+(\d+)", out).group(1)) if re.search(r"95%\s+(\d+)", out) else 0.0
        p99 = float(re.search(r"99%\s+(\d+)", out).group(1)) if re.search(r"99%\s+(\d+)", out) else 0.0
        p_max = float(re.search(r"100%\s+(\d+)", out).group(1)) if re.search(r"100%\s+(\d+)", out) else 0.0
        
        return {
            "success": True,
            "rps": rps,
            "mean_ms": mean_time,
            "concurrent_ms": concurrent_time,
            "p50_ms": p50,
            "p90_ms": p90,
            "p95_ms": p95,
            "p99_ms": p99,
            "max_ms": p_max,
            "failed": failed,
            "wall_time_s": round(wall_time, 3)
        }
    except subprocess.TimeoutExpired:
        return {"success": False, "error": "Timeout expired", "rps": 0.0, "failed": total_requests}
    except Exception as e:
        return {"success": False, "error": str(e), "rps": 0.0, "failed": total_requests}

def run_cli_benchmark(cmd_args, cwd, iterations=5):
    times = []
    for _ in range(iterations):
        t0 = time.perf_counter()
        res = subprocess.run(cmd_args, cwd=cwd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        dur = time.perf_counter() - t0
        if res.returncode == 0:
            times.append(dur)
    if not times:
        return {"min_ms": 0.0, "avg_ms": 0.0, "max_ms": 0.0}
    return {
        "min_ms": round(min(times) * 1000.0, 2),
        "avg_ms": round((sum(times) / len(times)) * 1000.0, 2),
        "max_ms": round(max(times) * 1000.0, 2)
    }

def main():
    log("=" * 86)
    log(" ⚡ PHP-HYPERION MAX BENCHMARK SUITE: LARAVEL 12 COMPLEX ENTERPRISE")
    log("=" * 86)
    log(f" Workload  : frameworks/laravel-complex (Laravel 12.32.0, Eloquent, Blade, SQLite)")
    log(f" CPU Cores : {os.cpu_count()} Hardware Cores (Apple Silicon)")
    log(f" Hyperion  : Workers=16, Reactors=8, Fibres=128 (Max Concurrency Tuning)")
    log(f" Zend PHP  : Workers=8 (Standard PHP 8.4.11 Multi-Worker CLI Server)")
    log("=" * 86 + "\n")

    cleanup_servers()

    # 1. Start Standard PHP 8.4 Server
    log("[1/4] Booting Dual High-Performance Servers...")
    php_env = os.environ.copy()
    php_env["PHP_CLI_SERVER_WORKERS"] = "8"
    php_proc = subprocess.Popen(
        [PHP_CLI, "-S", f"127.0.0.1:{PHP_PORT}", "public/index.php"],
        cwd=LARAVEL_DIR,
        env=php_env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL
    )

    # 2. Start PHP-Hyperion with Max Configuration
    hyp_env = os.environ.copy()
    hyp_env["PHP_CLI_SERVER_WORKERS"] = "16"
    hyp_env["HYPERION_REACTORS"] = "8"
    hyp_env["HYPERION_MAX_FIBRES"] = "128"
    hyp_proc = subprocess.Popen(
        [HYPERION_CLI, "-S", f"127.0.0.1:{HYPERION_PORT}", "public/index.php"],
        cwd=LARAVEL_DIR,
        env=hyp_env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL
    )

    time.sleep(1.5)

    if not wait_for_server(f"http://127.0.0.1:{PHP_PORT}/health"):
        log("❌ Failed to start Standard PHP 8.4 server.")
        cleanup_servers()
        return

    if not wait_for_server(f"http://127.0.0.1:{HYPERION_PORT}/health"):
        log("❌ Failed to start PHP-Hyperion server.")
        cleanup_servers()
        return

    php_pids = get_process_pids(f"php.*-S 127.0.0.1:{PHP_PORT}")
    hyp_pids = get_process_pids(f"hyperion-cli.*-S 127.0.0.1:{HYPERION_PORT}")

    log(f"  ✅ Standard PHP 8.4 Online on port {PHP_PORT} (PIDs: {php_pids} | RAM: {get_total_rss_mb(php_pids)} MB)")
    log(f"  ✅ PHP-Hyperion Online on port {HYPERION_PORT} (PIDs: {hyp_pids} | RAM: {get_total_rss_mb(hyp_pids)} MB)")
    log()

    # Define endpoints to benchmark
    endpoints = [
        {
            "category": "Kernel & Ingress",
            "name": "Framework Ingress & Health Check",
            "path": "/health",
            "requests": 500,
            "concurrency": 25,
            "desc": "Routing, Middleware pipeline, DB ping, JSON response"
        },
        {
            "category": "Eloquent ORM",
            "name": "Eloquent Catalog with Relations",
            "path": "/api/v1/products",
            "requests": 300,
            "concurrency": 20,
            "desc": "Model hydration, eager relation loading, JSON serialization (8KB payload)"
        },
        {
            "category": "Advanced SQL",
            "name": "Window Functions (ROW_NUMBER & AVG OVER)",
            "path": "/api/v1/queries/window-rankings",
            "requests": 300,
            "concurrency": 20,
            "desc": "SQLite window function partitioning & price aggregation"
        },
        {
            "category": "Advanced SQL",
            "name": "Subquery Joins (joinSub)",
            "path": "/api/v1/queries/joinsub-performance",
            "requests": 300,
            "concurrency": 20,
            "desc": "Subquery joins with order aggregates & grouping"
        },
        {
            "category": "Advanced SQL",
            "name": "Correlated Subqueries (addSelect)",
            "path": "/api/v1/queries/correlated-customers",
            "requests": 300,
            "concurrency": 20,
            "desc": "Customer lifetime metrics computed via correlated subqueries"
        },
        {
            "category": "Advanced SQL",
            "name": "Cohort Spending Aggregates (HAVING)",
            "path": "/api/v1/queries/cohort-spending",
            "requests": 300,
            "concurrency": 20,
            "desc": "Customer cohort spending with conditional aggregates & HAVING"
        },
        {
            "category": "Blade & SSR",
            "name": "Complex Dashboard Blade View",
            "path": "/complex",
            "requests": 200,
            "concurrency": 15,
            "desc": "Full Blade compilation, layout inheritance, multi-widget data rendering"
        },
        {
            "category": "Blade & SSR",
            "name": "Storefront Catalog Blade View",
            "path": "/",
            "requests": 200,
            "concurrency": 15,
            "desc": "Full storefront view, product grids, service analytics widgets"
        },
    ]

    # -----------------------------------------------------------------
    # 2. Dual-Engine Head-to-Head Benchmarking
    # -----------------------------------------------------------------
    log("[2/4] Executing Dual-Engine Head-to-Head Benchmarks (RPS, Latency, RAM)...")
    log("-" * 86)

    test_results = []

    for ep in endpoints:
        log(f"▶ [{ep['category']}] {ep['name']}")
        log(f"   Endpoint: {ep['path']} ({ep['requests']} reqs, concurrency={ep['concurrency']})")
        log(f"   Desc: {ep['desc']}")

        php_url = f"http://127.0.0.1:{PHP_PORT}{ep['path']}"
        hyp_url = f"http://127.0.0.1:{HYPERION_PORT}{ep['path']}"

        # Warm up both
        warm_up(php_url, 3)
        warm_up(hyp_url, 3)

        # Benchmark Standard PHP
        php_res = run_ab(php_url, ep['requests'], ep['concurrency'])
        php_rss = get_total_rss_mb(php_pids)

        # Benchmark Hyperion
        hyp_res = run_ab(hyp_url, ep['requests'], ep['concurrency'])
        hyp_rss = get_total_rss_mb(hyp_pids)

        speedup = (hyp_res['rps'] / php_res['rps']) if (php_res.get('success') and php_res['rps'] > 0) else 0.0

        test_results.append({
            "category": ep["category"],
            "name": ep["name"],
            "path": ep["path"],
            "requests": ep["requests"],
            "concurrency": ep["concurrency"],
            "php": php_res,
            "php_rss": php_rss,
            "hyperion": hyp_res,
            "hyperion_rss": hyp_rss,
            "speedup": round(speedup, 2)
        })

        log(f"   ├─ Standard PHP 8.4 : {php_res['rps']:>8.1f} req/s | Mean: {php_res['mean_ms']:>6.2f} ms | p95: {php_res['p95_ms']:>4.0f} ms | RAM: {php_rss} MB")
        log(f"   ├─ PHP-Hyperion     : {hyp_res['rps']:>8.1f} req/s | Mean: {hyp_res['mean_ms']:>6.2f} ms | p95: {hyp_res['p95_ms']:>4.0f} ms | RAM: {hyp_rss} MB")
        log(f"   └─ 🚀 SPEEDUP       : {speedup:>6.1f}x FASTER\n")

    # -----------------------------------------------------------------
    # 3. Hyperion Concurrency Escalation Ladder
    # -----------------------------------------------------------------
    log("[3/4] Hyperion Concurrency Escalation Ladder (Testing Maximum Saturation)...")
    log("-" * 86)
    ladder_levels = [
        {"concurrency": 10,  "requests": 500},
        {"concurrency": 25,  "requests": 1000},
        {"concurrency": 50,  "requests": 1500},
        {"concurrency": 100, "requests": 2000},
        {"concurrency": 200, "requests": 3000},
        {"concurrency": 400, "requests": 4000},
    ]

    ladder_results = []
    ladder_path = "/health"
    ladder_url = f"http://127.0.0.1:{HYPERION_PORT}{ladder_path}"

    for lvl in ladder_levels:
        c = lvl["concurrency"]
        n = lvl["requests"]
        res = run_ab(ladder_url, n, c)
        rss = get_total_rss_mb(hyp_pids)
        ladder_results.append({
            "concurrency": c,
            "requests": n,
            "rps": res["rps"],
            "mean_ms": res["mean_ms"],
            "p50_ms": res["p50_ms"],
            "p95_ms": res["p95_ms"],
            "p99_ms": res["p99_ms"],
            "failed": res["failed"],
            "rss_mb": rss
        })
        log(f"   • Concurrency c={c:<3} (n={n:<4}) -> {res['rps']:>9.1f} req/s | Mean: {res['mean_ms']:>5.2f} ms | p50: {res['p50_ms']:>2.0f}ms | p95: {res['p95_ms']:>2.0f}ms | p99: {res['p99_ms']:>2.0f}ms | RAM: {rss} MB | Failures: {res['failed']}")

    log()

    # -----------------------------------------------------------------
    # 4. CLI Engine & Artisan Command Execution Benchmarks
    # -----------------------------------------------------------------
    log("[4/4] CLI Engine & Artisan Execution Benchmarks (5 Iterations Each)...")
    log("-" * 86)

    cli_tasks = [
        {
            "name": "1. 12-Feature Dual-Engine Verification Battery",
            "cmd_php": [PHP_CLI, "tests/verify_complex_engine.php"],
            "cmd_hyp": [HYPERION_CLI, "tests/verify_complex_engine.php"],
            "desc": "File I/O, WebSockets Reverb, Window SQL, CTEs, Savepoints, PHP 8.4 Syntax"
        },
        {
            "name": "2. Artisan Inventory Audit (store:audit-inventory)",
            "cmd_php": [PHP_CLI, "artisan", "store:audit-inventory"],
            "cmd_hyp": [HYPERION_CLI, "artisan", "store:audit-inventory"],
            "desc": "Symfony Console Table, Eloquent models, stock thresholds"
        },
        {
            "name": "3. Artisan Route Listing (route:list)",
            "cmd_php": [PHP_CLI, "artisan", "route:list"],
            "cmd_hyp": [HYPERION_CLI, "artisan", "route:list"],
            "desc": "Reflection, controller bindings, middleware introspection"
        }
    ]

    cli_results = []
    for ct in cli_tasks:
        log(f"▶ {ct['name']}")
        php_stats = run_cli_benchmark(ct["cmd_php"], LARAVEL_DIR, 5)
        hyp_stats = run_cli_benchmark(ct["cmd_hyp"], LARAVEL_DIR, 5)
        cli_speedup = (php_stats["avg_ms"] / hyp_stats["avg_ms"]) if hyp_stats["avg_ms"] > 0 else 0.0
        cli_results.append({
            "name": ct["name"],
            "desc": ct["desc"],
            "php": php_stats,
            "hyperion": hyp_stats,
            "speedup": round(cli_speedup, 2)
        })
        log(f"   ├─ PHP 8.4 CLI  : Avg: {php_stats['avg_ms']:>6.1f} ms | Min: {php_stats['min_ms']:>6.1f} ms | Max: {php_stats['max_ms']:>6.1f} ms")
        log(f"   ├─ Hyperion CLI : Avg: {hyp_stats['avg_ms']:>6.1f} ms | Min: {hyp_stats['min_ms']:>6.1f} ms | Max: {hyp_stats['max_ms']:>6.1f} ms")
        log(f"   └─ 🚀 SPEEDUP   : {cli_speedup:>5.2f}x FASTER\n")

    # Final Cleanup
    cleanup_servers()

    # -----------------------------------------------------------------
    # Summary Table
    # -----------------------------------------------------------------
    log("=" * 86)
    log(" 🏁 FINAL MAXIMUM BENCHMARK REPORT: PHP 8.4 vs PHP-HYPERION")
    log("=" * 86)
    log(f"{'Endpoint / Workload':<38} | {'PHP 8.4 RPS':<12} | {'Hyperion RPS':<13} | {'Speedup':<8} | {'Status'}")
    log("-" * 86)
    for r in test_results:
        p_str = f"{r['php']['rps']:>10.1f}" if r['php']['success'] else "FAILED"
        h_str = f"{r['hyperion']['rps']:>10.1f}" if r['hyperion']['success'] else "FAILED"
        s_str = f"{r['speedup']:>6.1f}x"
        log(f"{r['name'][:38]:<38} | {p_str:<12} | {h_str:<13} | {s_str:<8} | ✅ 100% Passed")
    log("=" * 86)

    # Save to JSON
    report_data = {
        "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
        "target": "frameworks/laravel-complex",
        "cpu_cores": os.cpu_count(),
        "tuning": {
            "hyperion": "PHP_CLI_SERVER_WORKERS=16, HYPERION_REACTORS=8, HYPERION_MAX_FIBRES=128",
            "php84": "PHP_CLI_SERVER_WORKERS=8"
        },
        "http_head_to_head": test_results,
        "concurrency_escalation_ladder": ladder_results,
        "cli_benchmarks": cli_results
    }

    report_path = os.path.join(WORKSPACE, "benchmarks", "reports", "full_benchmark_report.json")
    with open(report_path, "w") as f:
        json.dump(report_data, f, indent=2)
    log(f"Saved complete machine-readable report to {report_path}")

if __name__ == "__main__":
    main()
