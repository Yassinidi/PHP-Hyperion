#!/usr/bin/env python3
"""
⚡ PHP-Hyperion Concurrency Benchmark Suite for E-Commerce Frameworks
Runs all 5 E-Commerce projects across multiple concurrency tiers (c=10, 50, 100, 200)
Comparing Standard PHP 8.4 vs. PHP-Hyperion using hyperion_worker.php.
"""

import os
import sys
import time
import json
import re
import argparse
import subprocess
import urllib.request
import urllib.error

WORKSPACE = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
FRAMEWORKS_DIR = os.path.join(WORKSPACE, "frameworks")
HYPERION_CLI = os.path.join(WORKSPACE, "php-hyperion", "target", "release", "hyperion-cli")
WORKER_PHP = os.path.join(WORKSPACE, "hyperion_worker.php")
PHP_CLI = "/opt/homebrew/bin/php" if os.path.exists("/opt/homebrew/bin/php") else "php"
AB_BIN = "/usr/sbin/ab"

PORT_PHP = 8081
PORT_HYP = 8082

CONCURRENCY_LEVELS = [10, 50, 100, 200]
REQUESTS_PER_LEVEL = 2000

PROJECTS = [
    {"name": "ecommerce-native", "dir": "ecommerce-native", "docroot": "public", "title": "Native PHP E-Commerce"},
    {"name": "ecommerce-slim", "dir": "ecommerce-slim", "docroot": "public", "title": "Slim 4 E-Commerce"},
    {"name": "ecommerce-symfony", "dir": "ecommerce-symfony", "docroot": "public", "title": "Symfony 7 E-Commerce"},
    {"name": "ecommerce-wordpress", "dir": "ecommerce-wordpress", "docroot": "public", "title": "WordPress E-Commerce"},
    {"name": "ecommerce-laravel", "dir": "ecommerce-laravel", "docroot": "public", "title": "Laravel 11 E-Commerce"},
]

def kill_port(port):
    try:
        out = subprocess.check_output(f"lsof -ti tcp:{port}", shell=True, text=True).strip()
        if out:
            for pid in out.split():
                try:
                    os.kill(int(pid), 9)
                except OSError:
                    pass
    except subprocess.CalledProcessError:
        pass
    time.sleep(0.3)

def cleanup():
    kill_port(PORT_PHP)
    kill_port(PORT_HYP)
    subprocess.run(["pkill", "-9", "-f", f"-S 127.0.0.1:{PORT_PHP}"], stderr=subprocess.DEVNULL)
    subprocess.run(["pkill", "-9", "-f", f"-S 127.0.0.1:{PORT_HYP}"], stderr=subprocess.DEVNULL)
    time.sleep(0.5)

def wait_for_server(url, timeout=10.0):
    start = time.time()
    while time.time() - start < timeout:
        try:
            req = urllib.request.Request(url, headers={'User-Agent': 'HealthCheck/1.0'})
            with urllib.request.urlopen(req, timeout=1.0) as resp:
                if resp.status in (200, 404):
                    return True
        except Exception:
            time.sleep(0.15)
    return False

def get_process_rss_mb(pattern):
    try:
        out = subprocess.check_output(["pgrep", "-f", pattern], text=True).strip()
        if not out:
            return 0.0
        pids = [int(p) for p in out.splitlines()]
        total_kb = 0
        for pid in pids:
            try:
                ps_out = subprocess.check_output(["ps", "-o", "rss=", "-p", str(pid)], text=True).strip()
                if ps_out:
                    total_kb += int(ps_out.split()[0])
            except Exception:
                pass
        return round(total_kb / 1024.0, 1)
    except Exception:
        return 0.0

def run_ab(url, n, c):
    cmd = [AB_BIN, "-k", "-n", str(n), "-c", str(c), url]
    res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    out = res.stdout

    rps = 0.0
    mean_ms = 0.0
    m_rps = re.search(r"Requests per second:\s+([0-9.]+)", out)
    if m_rps:
        rps = float(m_rps.group(1))
    m_time = re.search(r"Time per request:\s+([0-9.]+)\s+\[ms\]\s+\(mean\)", out)
    if m_time:
        mean_ms = float(m_time.group(1))

    return {
        "concurrency": c,
        "requests": n,
        "rps": rps,
        "mean_ms": mean_ms
    }

def benchmark_endpoint(url_php, url_hyp, n, c_levels):
    results = []
    for c in c_levels:
        php_res = run_ab(url_php, n, c)
        hyp_res = run_ab(url_hyp, n, c)
        speedup = (hyp_res["rps"] / php_res["rps"]) if php_res["rps"] > 0 else 0.0
        results.append({
            "concurrency": c,
            "requests": n,
            "php_rps": php_res["rps"],
            "php_ms": php_res["mean_ms"],
            "hyperion_rps": hyp_res["rps"],
            "hyperion_ms": hyp_res["mean_ms"],
            "speedup": round(speedup, 2)
        })
    return results

def main():
    parser = argparse.ArgumentParser(description="Master E-Commerce Concurrency Benchmark")
    parser.add_argument("--project", default="all", help="Specific project name or 'all'")
    parser.add_argument("--requests", type=int, default=REQUESTS_PER_LEVEL, help="Requests per concurrency tier")
    args = parser.parse_args()

    print("=" * 88)
    print(" 🚀 PHP-HYPERION vs STANDARD PHP 8.4: MASTER E-COMMERCE CONCURRENCY BENCHMARK")
    print(f" Concurrency Tiers : {CONCURRENCY_LEVELS}")
    print(f" Requests Per Tier : {args.requests}")
    print(f" Hyperion Worker   : {WORKER_PHP}")
    print("=" * 88)

    targets = PROJECTS
    if args.project != "all":
        targets = [p for p in PROJECTS if p["name"] == args.project]
        if not targets:
            print(f"Error: Unknown project {args.project}")
            sys.exit(1)

    reports_dir = os.path.join(WORKSPACE, "benchmarks", "reports")
    os.makedirs(reports_dir, exist_ok=True)

    full_suite_results = {
        "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
        "hardware_cores": os.cpu_count(),
        "php_binary": PHP_CLI,
        "hyperion_binary": HYPERION_CLI,
        "worker_php": WORKER_PHP,
        "concurrency_tiers": CONCURRENCY_LEVELS,
        "requests_per_tier": args.requests,
        "projects": {}
    }

    for proj in targets:
        proj_name = proj["name"]
        proj_title = proj["title"]
        proj_dir = os.path.join(FRAMEWORKS_DIR, proj["dir"])
        docroot_dir = os.path.join(proj_dir, proj["docroot"])

        print(f"\n{'#'*88}")
        print(f"  BENCHMARKING: {proj_title.upper()} ({proj_name})")
        print(f"{'#'*88}")

        cleanup()

        # 1. Start Standard PHP
        php_env = os.environ.copy()
        php_env["PHP_CLI_SERVER_WORKERS"] = "8"
        proc_php = subprocess.Popen(
            [PHP_CLI, "-S", f"127.0.0.1:{PORT_PHP}", "-t", docroot_dir],
            cwd=proj_dir,
            env=php_env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL
        )

        if not wait_for_server(f"http://127.0.0.1:{PORT_PHP}/"):
            print(f"❌ Failed to start Standard PHP for {proj_name}")
            cleanup()
            continue

        # 2. Start PHP-Hyperion with hyperion_worker.php
        hyp_env = os.environ.copy()
        hyp_env["PHP_CLI_SERVER_WORKERS"] = "16"
        hyp_env["HYPERION_WORKER_PATH"] = WORKER_PHP
        hyp_log_path = os.path.join(proj_dir, "hyperion_concurrency.log")
        hyp_log = open(hyp_log_path, "w")

        hyp_cmd = [
            HYPERION_CLI,
            "-S", f"127.0.0.1:{PORT_HYP}",
            "-t", docroot_dir,
            WORKER_PHP
        ]
        proc_hyp = subprocess.Popen(hyp_cmd, cwd=proj_dir, env=hyp_env, stdout=hyp_log, stderr=hyp_log)

        if not wait_for_server(f"http://127.0.0.1:{PORT_HYP}/"):
            print(f"❌ Failed to start PHP-Hyperion for {proj_name}")
            cleanup()
            continue

        time.sleep(1.0)
        php_rss = get_process_rss_mb(f"-S 127.0.0.1:{PORT_PHP}")
        hyp_rss = get_process_rss_mb(f"-S 127.0.0.1:{PORT_HYP}")

        print(f"   [INIT] Standard PHP (RSS: {php_rss} MB) | PHP-Hyperion (RSS: {hyp_rss} MB)")

        # Benchmark 1: Products API (High Frequency REST)
        print(f"\n   --- Benchmarking Endpoint: GET /api/products ---")
        api_results = benchmark_endpoint(
            f"http://127.0.0.1:{PORT_PHP}/api/products",
            f"http://127.0.0.1:{PORT_HYP}/api/products",
            args.requests,
            CONCURRENCY_LEVELS
        )

        print(f"   {'Concurrency':<12} | {'Standard PHP':<18} | {'PHP-Hyperion':<18} | {'Speedup':<10}")
        print(f"   {'-'*12}-+-{'-'*18}-+-{'-'*18}-+-{'-'*10}")
        for r in api_results:
            php_info = f"{r['php_rps']:,.1f} rps ({r['php_ms']:.2f}ms)"
            hyp_info = f"{r['hyperion_rps']:,.1f} rps ({r['hyperion_ms']:.2f}ms)"
            print(f"   c = {r['concurrency']:<8} | {php_info:<18} | {hyp_info:<18} | {r['speedup']:.2f}x")

        # Benchmark 2: Storefront Home (Full HTML Template / Blade Rendering)
        print(f"\n   --- Benchmarking Endpoint: GET / (Storefront HTML) ---")
        home_results = benchmark_endpoint(
            f"http://127.0.0.1:{PORT_PHP}/",
            f"http://127.0.0.1:{PORT_HYP}/",
            args.requests,
            CONCURRENCY_LEVELS
        )

        print(f"   {'Concurrency':<12} | {'Standard PHP':<18} | {'PHP-Hyperion':<18} | {'Speedup':<10}")
        print(f"   {'-'*12}-+-{'-'*18}-+-{'-'*18}-+-{'-'*10}")
        for r in home_results:
            php_info = f"{r['php_rps']:,.1f} rps ({r['php_ms']:.2f}ms)"
            hyp_info = f"{r['hyperion_rps']:,.1f} rps ({r['hyperion_ms']:.2f}ms)"
            print(f"   c = {r['concurrency']:<8} | {php_info:<18} | {hyp_info:<18} | {r['speedup']:.2f}x")

        proj_summary = {
            "title": proj_title,
            "php_rss_mb": php_rss,
            "hyperion_rss_mb": hyp_rss,
            "products_api": api_results,
            "storefront_home": home_results
        }
        full_suite_results["projects"][proj_name] = proj_summary

        # Save individual project report
        with open(os.path.join(proj_dir, "benchmark_concurrency_report.json"), "w") as f:
            json.dump(proj_summary, f, indent=2)

        cleanup()

    # Save comprehensive master report
    master_report_path = os.path.join(reports_dir, "ecommerce_concurrency_report.json")
    with open(master_report_path, "w") as f:
        json.dump(full_suite_results, f, indent=2)

    print("\n" + "=" * 88)
    print(" 🏁 MASTER CONCURRENCY BENCHMARK COMPLETE!")
    print(f" Report saved to: {master_report_path}")
    print("=" * 88)

if __name__ == "__main__":
    main()
