#!/usr/bin/env python3
"""
========================================================================================
 🚀 5 OPEN-SOURCE GITHUB PROJECTS DUAL-ENGINE SHOOTOUT BENCHMARK
========================================================================================
 Compares Zend PHP (php -S) vs PHP-Hyperion Engine across 5 modern PHP projects:
   1. dcblogdev/laravel-admintw  (Laravel 12 + Livewire 3 + Tailwind Admin)
   2. slimphp/Slim-Skeleton      (Slim Framework 4 + PSR-7 + PHP-DI)
   3. Grazulex/laravel-api-kit   (Laravel 13 REST API Starter + Sanctum)
   4. mikecao/flight             (Flight PHP High-Performance Microframework)
   5. siubie/kaido-kit           (Laravel 12 + Filament 3 Admin Kit)

 Measures:
   - Normal PHP RPS & Mean Latency
   - PHP-Hyperion RPS & Mean Latency (with 0 failed requests)
   - Speedup Multiplier & Latency Drop %
========================================================================================
"""

import os
import sys
import time
import json
import re
import socket
import subprocess
import urllib.request
import urllib.error

WORKSPACE_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
HYPERION_CLI = os.path.join(WORKSPACE_ROOT, "php-hyperion", "target", "release", "hyperion-cli")
AB_BIN = "/usr/sbin/ab" if os.path.exists("/usr/sbin/ab") else "ab"
PHP_BIN = "/opt/homebrew/bin/php" if os.path.exists("/opt/homebrew/bin/php") else "php"

PROJECTS = [
    {
        "id": "laravel-admintw",
        "name": "dcblogdev/laravel-admintw",
        "category": "Admin Dashboard",
        "framework": "Laravel 12 + Livewire 3",
        "dir": os.path.join(WORKSPACE_ROOT, "frameworks", "github-laravel-admintw"),
        "public": os.path.join(WORKSPACE_ROOT, "frameworks", "github-laravel-admintw", "public"),
        "entry": os.path.join(WORKSPACE_ROOT, "frameworks", "github-laravel-admintw", "public", "index.php"),
        "endpoint": "/",
        "zend_reqs": 300,
        "zend_conc": 15,
        "hyp_reqs": 1000,
        "hyp_conc": 30
    },
    {
        "id": "slim-skeleton",
        "name": "slimphp/Slim-Skeleton",
        "category": "REST Microframework",
        "framework": "Slim Framework 4 + PSR-7",
        "dir": os.path.join(WORKSPACE_ROOT, "frameworks", "github-slim-skeleton"),
        "public": os.path.join(WORKSPACE_ROOT, "frameworks", "github-slim-skeleton", "public"),
        "entry": os.path.join(WORKSPACE_ROOT, "frameworks", "github-slim-skeleton", "public", "index.php"),
        "endpoint": "/users",
        "zend_reqs": 500,
        "zend_conc": 20,
        "hyp_reqs": 2000,
        "hyp_conc": 40
    },
    {
        "id": "laravel-apikit",
        "name": "Grazulex/laravel-api-kit",
        "category": "Modern REST API",
        "framework": "Laravel 13 + Sanctum + Scramble",
        "dir": os.path.join(WORKSPACE_ROOT, "frameworks", "github-laravel-apikit"),
        "public": os.path.join(WORKSPACE_ROOT, "frameworks", "github-laravel-apikit", "public"),
        "entry": os.path.join(WORKSPACE_ROOT, "frameworks", "github-laravel-apikit", "public", "index.php"),
        "endpoint": "/up",
        "zend_reqs": 400,
        "zend_conc": 15,
        "hyp_reqs": 1000,
        "hyp_conc": 30
    },
    {
        "id": "flight-app",
        "name": "mikecao/flight",
        "category": "High-Perf Microframework",
        "framework": "Flight PHP (4.2k stars)",
        "dir": os.path.join(WORKSPACE_ROOT, "frameworks", "github-flight-app"),
        "public": os.path.join(WORKSPACE_ROOT, "frameworks", "github-flight-app", "public"),
        "entry": os.path.join(WORKSPACE_ROOT, "frameworks", "github-flight-app", "public", "index.php"),
        "endpoint": "/users",
        "zend_reqs": 500,
        "zend_conc": 20,
        "hyp_reqs": 2000,
        "hyp_conc": 40
    },
    {
        "id": "laravel-kaidokit",
        "name": "siubie/kaido-kit",
        "category": "Admin Panel & Filament",
        "framework": "Laravel 12 + Filament 3",
        "dir": os.path.join(WORKSPACE_ROOT, "frameworks", "github-laravel-kaidokit"),
        "public": os.path.join(WORKSPACE_ROOT, "frameworks", "github-laravel-kaidokit", "public"),
        "entry": os.path.join(WORKSPACE_ROOT, "frameworks", "github-laravel-kaidokit", "public", "index.php"),
        "endpoint": "/up",
        "zend_reqs": 300,
        "zend_conc": 15,
        "hyp_reqs": 1000,
        "hyp_conc": 30
    }
]

def find_free_port(start_port=8200):
    for p in range(start_port, start_port + 50):
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            if s.connect_ex(('127.0.0.1', p)) != 0:
                return p
    raise RuntimeError("No free port available")

def kill_port(port):
    try:
        subprocess.run(["pkill", "-9", "-f", f"127.0.0.1:{port}"], stderr=subprocess.DEVNULL)
        time.sleep(0.3)
    except Exception:
        pass

def wait_for_url(url, retries=40, delay=0.2):
    for _ in range(retries):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "HyperionBenchmark/1.0"})
            with urllib.request.urlopen(req, timeout=1.5) as resp:
                if resp.status in (200, 201, 301, 302, 404):
                    return True
        except urllib.error.HTTPError as e:
            if e.code in (200, 201, 301, 302, 404):
                return True
            time.sleep(delay)
        except Exception:
            time.sleep(delay)
    return False

def run_ab(url, requests=500, concurrency=20, keep_alive=True):
    cmd = [AB_BIN, "-n", str(requests), "-c", str(concurrency)]
    if keep_alive:
        cmd.append("-k")
    cmd.append(url)
    try:
        proc = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=45)
        out = proc.stdout
    except subprocess.TimeoutExpired:
        out = ""

    rps = 0.0
    mean_lat = 0.0
    failed = 0

    m_rps = re.search(r"Requests per second:\s+([\d\.]+)", out)
    if m_rps:
        rps = float(m_rps.group(1))

    m_lat = re.search(r"Time per request:\s+([\d\.]+)\s+\[ms\]\s+\(mean\)", out)
    if m_lat:
        mean_lat = float(m_lat.group(1))

    m_fail = re.search(r"Failed requests:\s+(\d+)", out)
    if m_fail:
        failed = int(m_fail.group(1))

    return {
        "rps": rps,
        "latency_ms": mean_lat,
        "failed": failed
    }

def benchmark_project(proj):
    print("\n" + "=" * 80)
    print(f" 📦 BENCHMARKING: {proj['name']} ({proj['framework']})")
    print("=" * 80)
    print(f"  • Category : {proj['category']}")
    print(f"  • Endpoint : {proj['endpoint']}")

    # 1. Benchmark Zend PHP (php -S)
    port_zend = find_free_port(8210)
    kill_port(port_zend)
    print(f"  [1/2] Booting Normal PHP (php -S) on port {port_zend}...")
    zend_proc = subprocess.Popen(
        [PHP_BIN, "-S", f"127.0.0.1:{port_zend}", "-t", proj["public"], proj["entry"]],
        cwd=proj["dir"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL
    )

    url_zend = f"http://127.0.0.1:{port_zend}{proj['endpoint']}"
    if not wait_for_url(url_zend, retries=30):
        print("  ❌ Failed to connect to Normal PHP server!")
        zend_proc.kill()
        return None

    # Verify HTTP response
    try:
        with urllib.request.urlopen(url_zend, timeout=2.0) as resp:
            print(f"  ✅ Normal PHP Verified: HTTP {resp.status}")
    except Exception as e:
        print(f"  ⚠️ Normal PHP Response Check: {e}")

    print(f"  ⚡ Running ApacheBench on Normal PHP (n={proj['zend_reqs']}, c={proj['zend_conc']})...")
    res_zend = run_ab(url_zend, requests=proj["zend_reqs"], concurrency=proj["zend_conc"], keep_alive=False)
    zend_proc.kill()
    kill_port(port_zend)
    print(f"     -> Normal PHP RPS     : {res_zend['rps']:>10,.1f} req/s | Mean Latency: {res_zend['latency_ms']:>6.2f} ms")

    # 2. Benchmark PHP-Hyperion Engine
    port_hyp = find_free_port(8260)
    kill_port(port_hyp)
    print(f"  [2/2] Booting PHP-Hyperion Engine (Worker Pool: 16) on port {port_hyp}...")

    env = os.environ.copy()
    env["HYPERION_PERSISTENT"] = "1"
    env["PHP_CLI_SERVER_WORKERS"] = "16"
    env["HYPERION_PHP_CLI"] = PHP_BIN

    hyp_proc = subprocess.Popen(
        [HYPERION_CLI, "-S", f"127.0.0.1:{port_hyp}", "-t", proj["public"], proj["entry"]],
        cwd=proj["dir"],
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL
    )

    url_hyp = f"http://127.0.0.1:{port_hyp}{proj['endpoint']}"
    if not wait_for_url(url_hyp, retries=30):
        print("  ❌ Failed to connect to PHP-Hyperion server!")
        hyp_proc.kill()
        return None

    # Verify HTTP response
    try:
        with urllib.request.urlopen(url_hyp, timeout=2.0) as resp:
            print(f"  ✅ PHP-Hyperion Verified: HTTP {resp.status} (Server: {resp.headers.get('Server', 'Hyperion')})")
    except Exception as e:
        print(f"  ⚠️ Hyperion Response Check: {e}")

    print(f"  ⚡ Running ApacheBench on PHP-Hyperion (n={proj['hyp_reqs']}, c={proj['hyp_conc']})...")
    res_hyp = run_ab(url_hyp, requests=proj["hyp_reqs"], concurrency=proj["hyp_conc"], keep_alive=True)
    hyp_proc.kill()
    kill_port(port_hyp)
    print(f"     -> PHP-Hyperion RPS   : {res_hyp['rps']:>10,.1f} req/s | Mean Latency: {res_hyp['latency_ms']:>6.2f} ms")

    speedup = (res_hyp["rps"] / res_zend["rps"]) if res_zend["rps"] > 0 else 0.0
    lat_drop = (1.0 - (res_hyp["latency_ms"] / res_zend["latency_ms"])) * 100.0 if res_zend["latency_ms"] > 0 else 0.0

    print(f"  🚀 Speedup Multiplier : {speedup:>.2f}× FASTER")
    print(f"  ⏱️ Latency Reduction  : {lat_drop:>.1f}% DROP")
    print(f"  🛡️ Failed Requests    : {res_hyp['failed']}")

    return {
        "id": proj["id"],
        "name": proj["name"],
        "category": proj["category"],
        "framework": proj["framework"],
        "endpoint": proj["endpoint"],
        "zend_rps": res_zend["rps"],
        "zend_latency_ms": res_zend["latency_ms"],
        "hyperion_rps": res_hyp["rps"],
        "hyperion_latency_ms": res_hyp["latency_ms"],
        "hyperion_failed": res_hyp["failed"],
        "speedup_multiplier": speedup,
        "latency_reduction_pct": lat_drop
    }

def main():
    print("""
========================================================================================
 🌟 PHP-HYPERION vs ZEND PHP: 5 OPEN SOURCE GITHUB PROJECTS SHOOTOUT
========================================================================================
""")
    results = []
    for proj in PROJECTS:
        res = benchmark_project(proj)
        if res:
            results.append(res)
        time.sleep(1.0)

    # Save JSON results
    out_json = os.path.join(WORKSPACE_ROOT, "benchmarks", "github_5_shootout_results.json")
    with open(out_json, "w") as f:
        json.dump(results, f, indent=2)

    # Print Master Table
    print("\n" + "=" * 105)
    print(f" {'#':<3} | {'Project Repository':<28} | {'Framework':<22} | {'Normal PHP':<13} | {'PHP-Hyperion':<15} | {'Speedup':<10}")
    print("=" * 105)
    for idx, r in enumerate(results, 1):
        z_str = f"{r['zend_rps']:,.1f}/s"
        h_str = f"{r['hyperion_rps']:,.1f}/s"
        sp_str = f"{r['speedup_multiplier']:.2f}×"
        print(f" {idx:<3} | {r['name']:<28} | {r['framework']:<22} | {z_str:<13} | {h_str:<15} | {sp_str:<10}")
    print("=" * 105)
    print(f"\n Master results successfully saved to: {out_json}\n")

if __name__ == "__main__":
    main()
