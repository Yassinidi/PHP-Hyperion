#!/usr/bin/env python3
"""
⚡ PERFORMANCE BENCHMARK: Standard PHP 8.4 vs PHP-Hyperion
Project: frameworks/normal-php-app (100% Vanilla PHP 8 REST Application)
Engines compared:
  1. Standard PHP 8.4 CLI Server (Multi-worker pool, Zend OPcache)
  2. PHP-Hyperion Persistent Daemon Worker (Hybrid SAPI + RFC 9111 Cache)
  3. PHP-Hyperion Pure Zero-Copy Rust VM (Fibre Runtime)
"""

import os
import sys
import time
import json
import re
import subprocess
import urllib.request

def find_workspace_root():
    d = os.path.dirname(os.path.abspath(__file__))
    while d and d != os.path.dirname(d):
        if os.path.exists(os.path.join(d, "php-hyperion")) and os.path.exists(os.path.join(d, "frameworks")):
            return d
        d = os.path.dirname(d)
    return os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))

WORKSPACE = find_workspace_root()
APP_DIR = os.path.join(WORKSPACE, "frameworks", "normal-php-app")
PUBLIC_DIR = os.path.join(APP_DIR, "public")
INDEX_PHP = os.path.join(PUBLIC_DIR, "index.php")
HYPERION_CLI = os.path.join(WORKSPACE, "php-hyperion", "target", "release", "hyperion-cli")
PHP_CLI = "php"
AB_BIN = "/usr/sbin/ab"

PORT_STD = 8880
PORT_HYP_HYBRID = 8881
PORT_HYP_VM = 8882

def log(msg="", end="\n"):
    print(msg, end=end, flush=True)

def cleanup_servers():
    for port in [PORT_STD, PORT_HYP_HYBRID, PORT_HYP_VM]:
        subprocess.run(["pkill", "-9", "-f", f"127.0.0.1:{port}"], stderr=subprocess.DEVNULL)
    time.sleep(0.6)

def wait_for_server(url, max_retries=35, delay=0.2):
    for _ in range(max_retries):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "HyperionProbe"})
            with urllib.request.urlopen(req, timeout=1.5) as resp:
                if resp.status in (200, 302, 404):
                    return True
        except Exception:
            time.sleep(delay)
    return False

def warm_up(url, count=25):
    for _ in range(count):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "Warmup"})
            with urllib.request.urlopen(req, timeout=1.5) as resp:
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
        
        failed_match = re.search(r"Failed requests:\s+(\d+)", out)
        failed = int(failed_match.group(1)) if failed_match else 0
        
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
            "p50_ms": p50,
            "p95_ms": p95,
            "p99_ms": p99,
            "max_ms": p_max,
            "failed": failed,
            "wall_time_s": round(wall_time, 3)
        }
    except Exception as e:
        return {"success": False, "error": str(e), "rps": 0.0, "failed": total_requests}

def main():
    log("=" * 80)
    log("⚡ TRIPLE BENCHMARK: STANDARD PHP 8.4 vs PHP-HYPERION (HYBRID & PURE VM)")
    log("Target: frameworks/normal-php-app (Vanilla PHP 8 REST API)")
    log("=" * 80)

    cleanup_servers()

    # 1. Start Standard PHP Server
    log("\n[1/3] Starting Standard PHP 8.4 Server (Multi-worker pool)...")
    php_env = os.environ.copy()
    php_env["PHP_CLI_SERVER_WORKERS"] = "8"
    subprocess.Popen(
        [
            PHP_CLI,
            "-S", f"127.0.0.1:{PORT_STD}",
            "-t", PUBLIC_DIR,
            INDEX_PHP,
        ],
        cwd=APP_DIR,
        env=php_env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    url_std = f"http://127.0.0.1:{PORT_STD}"
    if not wait_for_server(url_std):
        log("❌ Standard PHP failed to start.")
        cleanup_servers()
        sys.exit(1)
    log(f"✅ Standard PHP listening on {url_std}")
    warm_up(url_std, count=30)

    # 2. Start PHP-Hyperion Hybrid Server
    log("\n[2/3] Starting PHP-Hyperion Hybrid Server (Persistent Daemon Worker)...")
    hyp_env = os.environ.copy()
    hyp_env["PHP_CLI_SERVER_WORKERS"] = "8"
    hyp_env["HYPERION_MAX_FIBRES"] = "64"
    subprocess.Popen(
        [
            HYPERION_CLI,
            "-S", f"127.0.0.1:{PORT_HYP_HYBRID}",
            "-t", PUBLIC_DIR,
            "-W",
        ],
        cwd=WORKSPACE,
        env=hyp_env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    url_hyp_hybrid = f"http://127.0.0.1:{PORT_HYP_HYBRID}"
    if not wait_for_server(url_hyp_hybrid):
        log("❌ PHP-Hyperion Hybrid failed to start.")
        cleanup_servers()
        sys.exit(1)
    log(f"✅ PHP-Hyperion Hybrid listening on {url_hyp_hybrid}")
    warm_up(url_hyp_hybrid, count=30)

    # 3. Start PHP-Hyperion Pure Zero-Copy Rust VM
    log("\n[3/3] Starting PHP-Hyperion Pure Rust VM (Fibre Engine)...")
    vm_env = os.environ.copy()
    vm_env["HYPERION_ENGINE"] = "vm"
    vm_env["PHP_CLI_SERVER_WORKERS"] = "10"
    vm_env["HYPERION_REACTORS"] = "10"
    vm_env["HYPERION_MAX_FIBRES"] = "256"
    subprocess.Popen(
        [
            HYPERION_CLI,
            "-S", f"127.0.0.1:{PORT_HYP_VM}",
            "-t", PUBLIC_DIR,
            INDEX_PHP,
        ],
        cwd=WORKSPACE,
        env=vm_env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    url_hyp_vm = f"http://127.0.0.1:{PORT_HYP_VM}"
    if not wait_for_server(url_hyp_vm):
        log("❌ PHP-Hyperion Pure VM failed to start.")
        cleanup_servers()
        sys.exit(1)
    log(f"✅ PHP-Hyperion Pure VM listening on {url_hyp_vm}")
    warm_up(url_hyp_vm, count=30)

    scenarios = [
        {
            "name": "1. Micro JSON Status (GET /)",
            "path": "/",
            "requests": 5000,
            "concurrency": 50,
        },
        {
            "name": "2. Models & Serialization (GET /api/users)",
            "path": "/api/users",
            "requests": 3000,
            "concurrency": 25,
        },
        {
            "name": "3. Pricing & Tax Calculations (GET /api/products?tax=0.15&discount=0.10)",
            "path": "/api/products?tax=0.15&discount=0.10",
            "requests": 3000,
            "concurrency": 25,
        },
        {
            "name": "4. Extreme Concurrency Stress (GET /)",
            "path": "/",
            "requests": 10000,
            "concurrency": 100,
        },
        {
            "name": "5. Crypto & Array Transformation (GET /api/compute?n=100)",
            "path": "/api/compute?n=100",
            "requests": 2000,
            "concurrency": 20,
        },
    ]

    all_results = []

    log("\n" + "=" * 80)
    log("🚀 EXECUTING COMPARATIVE PERFORMANCE BENCHMARK")
    log("=" * 80)

    for sc in scenarios:
        log(f"\n--- Scenario: {sc['name']} ---")
        log(f"Config: {sc['requests']} requests, {sc['concurrency']} concurrent connections")

        # Standard PHP
        res_std = run_ab(f"{url_std}{sc['path']}", sc["requests"], sc["concurrency"])
        log(f"  [1] Standard PHP 8.4   : {res_std['rps']:>8.1f} req/s | Mean: {res_std['mean_ms']:>5.2f}ms | p95: {res_std['p95_ms']:>4.1f}ms")

        # Hyperion Hybrid
        res_hybrid = run_ab(f"{url_hyp_hybrid}{sc['path']}", sc["requests"], sc["concurrency"])
        log(f"  [2] Hyperion Hybrid    : {res_hybrid['rps']:>8.1f} req/s | Mean: {res_hybrid['mean_ms']:>5.2f}ms | p95: {res_hybrid['p95_ms']:>4.1f}ms")

        # Hyperion Pure VM
        res_vm = run_ab(f"{url_hyp_vm}{sc['path']}", sc["requests"], sc["concurrency"])
        log(f"  [3] Hyperion Pure VM   : {res_vm['rps']:>8.1f} req/s | Mean: {res_vm['mean_ms']:>5.2f}ms | p95: {res_vm['p95_ms']:>4.1f}ms")

        best_rps = max(res_hybrid['rps'], res_vm['rps'])
        speedup = best_rps / max(res_std['rps'], 0.1)
        log(f"  ⚡ Top Hyperion Advantage: {speedup:.2f}x of Standard PHP 8.4")

        all_results.append({
            "scenario": sc["name"],
            "requests": sc["requests"],
            "concurrency": sc["concurrency"],
            "standard_php": res_std,
            "hyperion_hybrid": res_hybrid,
            "hyperion_pure_vm": res_vm,
            "speedup_factor": round(speedup, 2),
        })

    cleanup_servers()

    # Save Results
    output_json = os.path.join(WORKSPACE, "benchmarks", "reports", "benchmark_normal_php_results.json")
    with open(output_json, "w") as f:
        json.dump(all_results, f, indent=2)

    log("\n" + "=" * 80)
    log(f"📊 BENCHMARK COMPLETE! Full results saved to {output_json}")
    log("=" * 80)

if __name__ == "__main__":
    main()
