#!/usr/bin/env python3
"""
⚡ PHP-HYPERION UNIVERSAL STABILITY & COMPREHENSIVE BENCHMARK BATTERY
Evaluates:
  1. Multiple Distinct PHP Application Workloads:
     - Bank Transfer (ACID Row-Lock Ledger)
     - Analytics & Telemetry (Time-series Ingestion & Quantile Analytics)
     - Auth & RBAC (CPU Cryptography, HMAC-SHA256 & Hierarchical RBAC)
     - CMS & Markdown (Regex Markdown Engine, TOC Generation & Search)
     - Queue & Workflow (Atomic Row-Leased Job Dispatch & FSM Transitions)
  2. Multi-Run Jitter & Stability Index:
     - Measures 5 consecutive benchmark rounds per workload over time
     - Calculates Mean, Min, Max, Standard Deviation, and Jitter / CV%
  3. Direct Performance Comparison:
     - Standard PHP 8.4 vs PHP-Hyperion Core 2.0
     - Throughput multiplier & Sub-millisecond latency analysis
"""

import os
import sys
import time
import json
import math
import re
import subprocess
import urllib.request
import urllib.error

WORKSPACE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
HYPERION_CLI = os.path.join(WORKSPACE, "php-hyperion", "target", "release", "hyperion-cli")
PHP_CLI = "/opt/homebrew/bin/php" if os.path.exists("/opt/homebrew/bin/php") else "php"
AB_BIN = "/usr/sbin/ab"

PORT_PHP = 8083
PORT_HYP = 8084

ROUNDS_PER_WORKLOAD = 4
BENCHMARK_N_READ = 1000
BENCHMARK_C_READ = 50
BENCHMARK_N_WRITE = 300
BENCHMARK_C_WRITE = 15

PROJECTS = [
    {
        "id": "bank-native",
        "name": "Bank Money Transfer & Ledger",
        "dir": os.path.join(WORKSPACE, "frameworks", "bank-native"),
        "docroot": os.path.join(WORKSPACE, "frameworks", "bank-native", "public"),
        "entry": os.path.join(WORKSPACE, "frameworks", "bank-native", "public", "index.php"),
        "read_endpoint": "/api/accounts",
        "read_method": "GET",
        "write_endpoint": "/api/transfer",
        "write_method": "POST",
        "write_payload": {"from_account_id": 1, "to_account_id": 2, "amount": 10.0, "note": "wire"},
        "verify_url": "/api/accounts"
    },
    {
        "id": "analytics-telemetry",
        "name": "Analytics & Time-Series Quantiles",
        "dir": os.path.join(WORKSPACE, "frameworks", "analytics-telemetry"),
        "docroot": os.path.join(WORKSPACE, "frameworks", "analytics-telemetry", "public"),
        "entry": os.path.join(WORKSPACE, "frameworks", "analytics-telemetry", "public", "index.php"),
        "read_endpoint": "/api/metrics/summary",
        "read_method": "GET",
        "write_endpoint": "/api/telemetry/ingest",
        "write_method": "POST",
        "write_payload": {"node_id": "bench-node", "service_name": "api-gw", "metric_name": "request_latency", "val": 18.5, "latency_ms": 18.5, "status_code": 200},
        "verify_url": "/api/metrics/summary"
    },
    {
        "id": "auth-rbac",
        "name": "Auth, HMAC-SHA256 & RBAC Matrix",
        "dir": os.path.join(WORKSPACE, "frameworks", "auth-rbac"),
        "docroot": os.path.join(WORKSPACE, "frameworks", "auth-rbac", "public"),
        "entry": os.path.join(WORKSPACE, "frameworks", "auth-rbac", "public", "index.php"),
        "read_endpoint": "/api/rbac/roles",
        "read_method": "GET",
        "write_endpoint": "/api/auth/token",
        "write_method": "POST",
        "write_payload": {"username": "alice_admin", "password": "HyperionSec2026!"},
        "verify_url": "/api/rbac/roles"
    },
    {
        "id": "cms-markdown",
        "name": "CMS, Regex Markdown & Search",
        "dir": os.path.join(WORKSPACE, "frameworks", "cms-markdown"),
        "docroot": os.path.join(WORKSPACE, "frameworks", "cms-markdown", "public"),
        "entry": os.path.join(WORKSPACE, "frameworks", "cms-markdown", "public", "index.php"),
        "read_endpoint": "/article/hyperion-architectural-deep-dive",
        "read_method": "GET",
        "write_endpoint": "/api/search?q=concurrency",
        "write_method": "GET",
        "write_payload": None,
        "verify_url": "/api/articles"
    },
    {
        "id": "queue-workflow",
        "name": "Queue, FSM & Atomic Leases",
        "dir": os.path.join(WORKSPACE, "frameworks", "queue-workflow"),
        "docroot": os.path.join(WORKSPACE, "frameworks", "queue-workflow", "public"),
        "entry": os.path.join(WORKSPACE, "frameworks", "queue-workflow", "public", "index.php"),
        "read_endpoint": "/api/queue/stats",
        "read_method": "GET",
        "write_endpoint": "/api/queue/push",
        "write_method": "POST",
        "write_payload": {"queue": "notifications", "data": {"type": "email", "recipient": "test@domain.com"}, "priority": 20},
        "verify_url": "/api/queue/stats"
    }
]

def kill_ports():
    for p in (PORT_PHP, PORT_HYP):
        subprocess.run(["pkill", "-9", "-f", f"127.0.0.1:{p}"], stderr=subprocess.DEVNULL)
    time.sleep(0.5)

def wait_for_server(url, retries=30, delay=0.2):
    for _ in range(retries):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "Probe/1.0"})
            with urllib.request.urlopen(req, timeout=1.5) as resp:
                if resp.status in (200, 302, 404):
                    return True
        except Exception:
            time.sleep(delay)
    return False

def run_ab(url, n, c, post_file=None):
    cmd = [AB_BIN, "-n", str(n), "-c", str(c), "-k"]
    if post_file:
        cmd.extend(["-p", post_file, "-T", "application/json"])
    cmd.append(url)

    proc = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    out = proc.stdout

    rps = 0.0
    mean_lat = 0.0
    p99_lat = 0.0

    rps_match = re.search(r"Requests per second:\s+([\d\.]+)", out)
    if rps_match:
        rps = float(rps_match.group(1))

    lat_match = re.search(r"Time per request:\s+([\d\.]+)\s+\[ms\]\s+\(mean\)", out)
    if lat_match:
        mean_lat = float(lat_match.group(1))

    p99_match = re.search(r"99%\s+(\d+)", out)
    if p99_match:
        p99_lat = float(p99_match.group(1))

    return rps, mean_lat, p99_lat

def calculate_stats(samples):
    if not samples:
        return {"mean": 0.0, "min": 0.0, "max": 0.0, "stddev": 0.0, "cv_pct": 0.0}
    mean = sum(samples) / len(samples)
    min_val = min(samples)
    max_val = max(samples)
    variance = sum((x - mean) ** 2 for x in samples) / len(samples)
    stddev = math.sqrt(variance)
    cv_pct = (stddev / mean * 100.0) if mean > 0 else 0.0
    return {
        "mean": round(mean, 1),
        "min": round(min_val, 1),
        "max": round(max_val, 1),
        "stddev": round(stddev, 2),
        "cv_pct": round(cv_pct, 2)
    }

def main():
    print("=" * 80)
    print(" 🚀 PHP-HYPERION UNIVERSAL STABILITY & MULTI-PROJECT BENCHMARK BATTERY")
    print("=" * 80)
    print(f"PHP CLI      : {PHP_CLI}")
    print(f"Hyperion CLI : {HYPERION_CLI}")
    print(f"Iterations   : {ROUNDS_PER_WORKLOAD} consecutive rounds per workload")
    print("=" * 80)

    # Prepare temp post files
    temp_dir = os.path.join(WORKSPACE, "benchmarks", ".temp")
    os.makedirs(temp_dir, exist_ok=True)

    master_report = {
        "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
        "projects_tested": len(PROJECTS),
        "rounds": ROUNDS_PER_WORKLOAD,
        "results": []
    }

    overall_php_rps = []
    overall_hyp_rps = []

    for proj in PROJECTS:
        proj_id = proj["id"]
        proj_name = proj["name"]
        print(f"\n📂 [{proj_id.upper()}] {proj_name}")
        print("-" * 80)

        # 1. Start Standard PHP Server
        kill_ports()
        php_cmd = [
            PHP_CLI,
            "-S", f"127.0.0.1:{PORT_PHP}",
            "-t", proj["docroot"],
            proj["entry"]
        ]
        php_proc = subprocess.Popen(php_cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        php_url = f"http://127.0.0.1:{PORT_PHP}"
        if not wait_for_server(f"{php_url}{proj['verify_url']}"):
            print(f"❌ Failed to start Standard PHP server on port {PORT_PHP}")
            php_proc.kill()
            continue

        # 2. Start Hyperion Server
        hyp_env = os.environ.copy()
        hyp_env["HYPERION_PERSISTENT"] = "1"
        hyp_cmd = [
            HYPERION_CLI,
            "-S", f"127.0.0.1:{PORT_HYP}",
            "-t", proj["docroot"],
            proj["entry"]
        ]
        hyp_proc = subprocess.Popen(hyp_cmd, env=hyp_env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        hyp_url = f"http://127.0.0.1:{PORT_HYP}"
        if not wait_for_server(f"{hyp_url}{proj['verify_url']}"):
            print(f"❌ Failed to start Hyperion server on port {PORT_HYP}")
            php_proc.kill()
            hyp_proc.kill()
            continue

        # Write post file if applicable
        post_path = None
        if proj["write_payload"] is not None:
            post_path = os.path.join(temp_dir, f"{proj_id}_payload.json")
            with open(post_path, "w") as f:
                json.dump(proj["write_payload"], f)

        # Benchmark: Read Workload Stability (N rounds)
        print(f"   📊 Workload 1: High-Frequency Read ({proj['read_endpoint']})")
        php_read_runs = []
        hyp_read_runs = []
        hyp_read_lat = []

        for r in range(ROUNDS_PER_WORKLOAD):
            rps_php, lat_php, _ = run_ab(f"{php_url}{proj['read_endpoint']}", BENCHMARK_N_READ, BENCHMARK_C_READ)
            rps_hyp, lat_hyp, p99_hyp = run_ab(f"{hyp_url}{proj['read_endpoint']}", BENCHMARK_N_READ, BENCHMARK_C_READ)
            php_read_runs.append(rps_php)
            hyp_read_runs.append(rps_hyp)
            hyp_read_lat.append(lat_hyp)
            time.sleep(0.1)

        read_php_stats = calculate_stats(php_read_runs)
        read_hyp_stats = calculate_stats(hyp_read_runs)
        read_gain = round(read_hyp_stats["mean"] / max(1.0, read_php_stats["mean"]), 2)
        avg_hyp_lat = round(sum(hyp_read_lat) / len(hyp_read_lat), 2)

        print(f"      • Standard PHP 8.4 : {read_php_stats['mean']:>9,.1f} req/s  (Jitter CV: {read_php_stats['cv_pct']}%)")
        print(f"      • Hyperion Core    : {read_hyp_stats['mean']:>9,.1f} req/s  (Jitter CV: {read_hyp_stats['cv_pct']}%)  --> Gain: {read_gain:>5.2f}×  (Lat: {avg_hyp_lat}ms)")

        # Benchmark: Write / Complex Workload Stability (N rounds)
        print(f"   📊 Workload 2: Write / Complex Operation ({proj['write_endpoint']})")
        php_write_runs = []
        hyp_write_runs = []
        hyp_write_lat = []

        for r in range(ROUNDS_PER_WORKLOAD):
            rps_php, lat_php, _ = run_ab(f"{php_url}{proj['write_endpoint']}", BENCHMARK_N_WRITE, BENCHMARK_C_WRITE, post_file=post_path)
            rps_hyp, lat_hyp, p99_hyp = run_ab(f"{hyp_url}{proj['write_endpoint']}", BENCHMARK_N_WRITE, BENCHMARK_C_WRITE, post_file=post_path)
            php_write_runs.append(rps_php)
            hyp_write_runs.append(rps_hyp)
            hyp_write_lat.append(lat_hyp)
            time.sleep(0.1)

        write_php_stats = calculate_stats(php_write_runs)
        write_hyp_stats = calculate_stats(hyp_write_runs)
        write_gain = round(write_hyp_stats["mean"] / max(1.0, write_php_stats["mean"]), 2)
        avg_write_lat = round(sum(hyp_write_lat) / len(hyp_write_lat), 2)

        print(f"      • Standard PHP 8.4 : {write_php_stats['mean']:>9,.1f} req/s  (Jitter CV: {write_php_stats['cv_pct']}%)")
        print(f"      • Hyperion Core    : {write_hyp_stats['mean']:>9,.1f} req/s  (Jitter CV: {write_hyp_stats['cv_pct']}%)  --> Gain: {write_gain:>5.2f}×  (Lat: {avg_write_lat}ms)")

        # Record project metrics
        master_report["results"].append({
            "project_id": proj_id,
            "project_name": proj_name,
            "read_workload": {
                "endpoint": proj["read_endpoint"],
                "php": read_php_stats,
                "hyperion": read_hyp_stats,
                "gain_multiplier": read_gain,
                "hyperion_mean_lat_ms": avg_hyp_lat
            },
            "write_workload": {
                "endpoint": proj["write_endpoint"],
                "php": write_php_stats,
                "hyperion": write_hyp_stats,
                "gain_multiplier": write_gain,
                "hyperion_mean_lat_ms": avg_write_lat
            }
        })

        overall_php_rps.extend([read_php_stats["mean"], write_php_stats["mean"]])
        overall_hyp_rps.extend([read_hyp_stats["mean"], write_hyp_stats["mean"]])

        # Clean shutdown for next project
        php_proc.terminate()
        hyp_proc.terminate()
        kill_ports()

    # Save summary report
    report_dir = os.path.join(WORKSPACE, "benchmarks", "reports")
    os.makedirs(report_dir, exist_ok=True)
    report_file = os.path.join(report_dir, "stability_all_projects_report.json")
    with open(report_file, "w") as f:
        json.dump(master_report, f, indent=2)

    # Print Master Stability Summary Table
    print("\n" + "=" * 90)
    print(" 🏁 PHP-HYPERION UNIVERSAL MULTI-PROJECT STABILITY & PERFORMANCE AUDIT")
    print("=" * 90)
    print(f"{'Project Domain':<26} | {'Workload':<14} | {'Std PHP 8.4':<12} | {'Hyperion Core':<14} | {'Gain':<6} | {'Jitter CV':<9}")
    print("-" * 90)

    for item in master_report["results"]:
        p_name = item["project_id"]
        rw = item["read_workload"]
        ww = item["write_workload"]

        print(f"{p_name:<26} | {'Read':<14} | {rw['php']['mean']:>9,.1f}/s | {rw['hyperion']['mean']:>11,.1f}/s | {rw['gain_multiplier']:>5.2f}× | {rw['hyperion']['cv_pct']:>6.2f}%")
        print(f"{'':<26} | {'Write / Compute':<14} | {ww['php']['mean']:>9,.1f}/s | {ww['hyperion']['mean']:>11,.1f}/s | {ww['gain_multiplier']:>5.2f}× | {ww['hyperion']['cv_pct']:>6.2f}%")
        print("-" * 90)

    overall_gain = round(sum(overall_hyp_rps) / max(1.0, sum(overall_php_rps)), 2)
    print(f"🎉 AUDIT RESULT: Tested across {len(PROJECTS)} architectures with {ROUNDS_PER_WORKLOAD} passes each.")
    print(f"🔥 OVERALL AGGREGATE SPEEDUP ACROSS ALL WORKLOADS: {overall_gain}× FASTER")
    print(f"📄 Detailed JSON telemetry saved to: {report_file}")
    print("=" * 90)

if __name__ == "__main__":
    main()
