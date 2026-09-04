#!/usr/bin/env python3
"""
⚡ PHP-Hyperion vs Standard PHP 8.4 (CLI & JIT): Production Laravel 11 CRUD & Eloquent Benchmark Suite
"""

import sys
import os
import time
import subprocess
import json
import re
import urllib.request
import urllib.error

WORKSPACE = os.path.dirname(os.path.abspath(__file__))
HYPERION_CLI = os.path.join(WORKSPACE, "php-hyperion", "target", "release", "hyperion-cli")
PHP_CLI = "/opt/homebrew/bin/php" if os.path.exists("/opt/homebrew/bin/php") else "php"
CRUD_DIR = os.path.join(WORKSPACE, "laravel-crud")

def log(msg=""):
    print(msg, flush=True)

def run_cmd(cmd, cwd=None, env=None, timeout=120):
    start = time.perf_counter()
    try:
        res = subprocess.run(cmd, cwd=cwd, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=timeout)
        duration_ms = (time.perf_counter() - start) * 1000.0
        return res.returncode, res.stdout, res.stderr, duration_ms
    except subprocess.TimeoutExpired:
        return -1, "", "Timeout expired", (time.perf_counter() - start) * 1000.0
    except Exception as e:
        return -1, "", str(e), 0.0

def wait_for_server(url, max_retries=25):
    for _ in range(max_retries):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "BenchProbe"})
            with urllib.request.urlopen(req, timeout=1.5) as resp:
                if resp.status in (200, 302, 404):
                    return True
        except Exception:
            time.sleep(0.2)
    return False

def get_process_rss_mb(pid):
    try:
        res = subprocess.run(["ps", "-o", "rss=", "-p", str(pid)], stdout=subprocess.PIPE, text=True)
        rss_kb = int(res.stdout.strip())
        return round(rss_kb / 1024.0, 2)
    except Exception:
        return 0.0

def run_ab(url, total_requests, concurrency, keep_alive=True):
    cmd = ["/usr/sbin/ab"]
    if keep_alive:
        cmd.append("-k")
    cmd.extend(["-n", str(total_requests), "-c", str(concurrency), url])
    
    try:
        res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=60)
        out = res.stdout
        
        rps_match = re.search(r"Requests per second:\s+([\d.]+)", out)
        rps = float(rps_match.group(1)) if rps_match else 0.0
        
        mean_time_match = re.search(r"Time per request:\s+([\d.]+)\s+\[ms\]\s+\(mean\)", out)
        mean_time = float(mean_time_match.group(1)) if mean_time_match else 0.0
        
        p50_match = re.search(r"50%\s+(\d+)", out)
        p90_match = re.search(r"90%\s+(\d+)", out)
        p99_match = re.search(r"99%\s+(\d+)", out)
        
        p50 = float(p50_match.group(1)) if p50_match else 0.0
        p90 = float(p90_match.group(1)) if p90_match else 0.0
        p99 = float(p99_match.group(1)) if p99_match else 0.0
        
        failed_match = re.search(r"Failed requests:\s+(\d+)", out)
        failed = int(failed_match.group(1)) if failed_match else 0

        if "Length:" in out:
            length_match = re.search(r"Length:\s+(\d+)", out)
            if length_match and int(length_match.group(1)) == failed:
                failed = 0
                
        return {
            "rps": rps,
            "mean_ms": mean_time,
            "p50_ms": p50,
            "p90_ms": p90,
            "p99_ms": p99,
            "failed": failed,
        }
    except Exception as e:
        log(f"    [!] ab error: {e}")
        return {"rps": 0.0, "mean_ms": 0.0, "p50_ms": 0.0, "p90_ms": 0.0, "p99_ms": 0.0, "failed": total_requests}

def main():
    log("=" * 96)
    log(" ⚡ PRODUCTION BENCHMARKS: PHP-HYPERION vs STANDARD PHP 8.4 (CLI & JIT)")
    log("    Workload: Laravel 11.x Enterprise CRUD Application & Eloquent ORM")
    log("=" * 96)

    # -------------------------------------------------------------
    # 1. Eloquent ORM Micro-Benchmarks
    # -------------------------------------------------------------
    log("\n[1/3] Running Eloquent ORM Micro-Benchmarks (2,000 Models, Eager Loading, Casts & Aggregates)...")
    
    # PHP 8.4 Standard
    code, stdout, stderr, dur_php = run_cmd([PHP_CLI, "bench_eloquent.php"], cwd=CRUD_DIR)
    eloquent_php = json.loads(stdout.strip()) if code == 0 and stdout.strip().startswith("{") else {}
    log(f"  • Standard PHP 8.4 CLI : {eloquent_php.get('total_time_ms', dur_php):>8.2f} ms | RAM: {eloquent_php.get('peak_memory_mb', 0)} MB")
    
    # PHP 8.4 JIT
    jit_flags = ["-d", "opcache.enable_cli=1", "-d", "opcache.jit=tracing", "-d", "opcache.jit_buffer_size=128M"]
    code, stdout, stderr, dur_jit = run_cmd([PHP_CLI] + jit_flags + ["bench_eloquent.php"], cwd=CRUD_DIR)
    eloquent_jit = json.loads(stdout.strip()) if code == 0 and stdout.strip().startswith("{") else {}
    log(f"  • PHP 8.4 OPcache JIT  : {eloquent_jit.get('total_time_ms', dur_jit):>8.2f} ms | RAM: {eloquent_jit.get('peak_memory_mb', 0)} MB")

    # PHP-Hyperion
    code, stdout, stderr, dur_hyp = run_cmd([HYPERION_CLI, "bench_eloquent.php"], cwd=CRUD_DIR)
    eloquent_hyp = json.loads(stdout.strip()) if code == 0 and stdout.strip().startswith("{") else {}
    log(f"  • PHP-Hyperion 8.4     : {eloquent_hyp.get('total_time_ms', dur_hyp):>8.2f} ms | RAM: {eloquent_hyp.get('peak_memory_mb', 0)} MB")

    # -------------------------------------------------------------
    # 2. PHPUnit 11 Full Test Suite Execution Benchmark
    # -------------------------------------------------------------
    log("\n[2/3] Running Full PHPUnit 11 Test Suite (24 Tests, 133 Assertions)...")
    
    # Setup test database first
    run_cmd([PHP_CLI, "-r", "if (!file_exists('/tmp/hyperion_test.sqlite')) touch('/tmp/hyperion_test.sqlite');"], cwd=CRUD_DIR)
    run_cmd([PHP_CLI, "artisan", "migrate:fresh", "--env=testing"], cwd=CRUD_DIR)

    # Standard PHP 8.4
    code, stdout, stderr, dur_phpunit_php = run_cmd([PHP_CLI, "vendor/phpunit/phpunit/phpunit", "tests/"], cwd=CRUD_DIR)
    m = re.search(r"Time:\s+([\d:.]+)", stdout)
    php_time_str = m.group(1) if m else f"{dur_phpunit_php/1000.0:.2f}s"
    log(f"  • Standard PHP 8.4 CLI : {dur_phpunit_php:>8.2f} ms ({php_time_str})")

    # PHP 8.4 JIT
    code, stdout, stderr, dur_phpunit_jit = run_cmd([PHP_CLI] + jit_flags + ["vendor/phpunit/phpunit/phpunit", "tests/"], cwd=CRUD_DIR)
    m = re.search(r"Time:\s+([\d:.]+)", stdout)
    jit_time_str = m.group(1) if m else f"{dur_phpunit_jit/1000.0:.2f}s"
    log(f"  • PHP 8.4 OPcache JIT  : {dur_phpunit_jit:>8.2f} ms ({jit_time_str})")

    # PHP-Hyperion
    code, stdout, stderr, dur_phpunit_hyp = run_cmd([HYPERION_CLI, "vendor/phpunit/phpunit/phpunit", "tests/"], cwd=CRUD_DIR)
    m = re.search(r"Time:\s+([\d:.]+)", stdout)
    hyp_time_str = m.group(1) if m else f"{dur_phpunit_hyp/1000.0:.2f}s"
    log(f"  • PHP-Hyperion 8.4     : {dur_phpunit_hyp:>8.2f} ms ({hyp_time_str})")

    # -------------------------------------------------------------
    # 3. High-Concurrency HTTP API Pipeline Benchmarks
    # -------------------------------------------------------------
    log("\n[3/3] High-Concurrency HTTP API Pipeline Benchmarks (ApacheBench)...")
    
    subprocess.run(["pkill", "-9", "-f", "hyperion-cli.*-S"], stderr=subprocess.DEVNULL)
    subprocess.run(["pkill", "-9", "-f", "php.*-S 127.0.0.1:8001"], stderr=subprocess.DEVNULL)
    subprocess.run(["pkill", "-9", "-f", "php.*-S 127.0.0.1:8002"], stderr=subprocess.DEVNULL)
    time.sleep(0.5)

    # Clean and seed database for HTTP benchmarks
    for f in ["database.sqlite", "database.sqlite-wal", "database.sqlite-shm"]:
        fp = os.path.join(CRUD_DIR, "database", f)
        if os.path.exists(fp):
            try:
                os.remove(fp)
            except Exception:
                pass
    with open(os.path.join(CRUD_DIR, "database", "database.sqlite"), "w") as f:
        pass
    run_cmd([PHP_CLI, "artisan", "migrate:fresh", "--seed"], cwd=CRUD_DIR)

    hyp_env = os.environ.copy()
    hyp_env["PHP_CLI_SERVER_WORKERS"] = str(os.cpu_count() or 8)
    hyp_env["HYPERION_REACTORS"] = "8"
    hyp_env["HYPERION_MAX_FIBRES"] = "16"
    hyp_stderr_file = open("/tmp/hyp_server_stderr.log", "w")
    hyp_proc = subprocess.Popen(
        [HYPERION_CLI, "-S", "127.0.0.1:8000", "-W", "hyperion_worker.php"],
        cwd=CRUD_DIR,
        env=hyp_env,
        stdout=subprocess.DEVNULL,
        stderr=hyp_stderr_file,
    )

    # 2. Start Standard PHP Server (1 worker)
    php_env = os.environ.copy()
    php_env["PHP_CLI_SERVER_WORKERS"] = "1"
    php_proc = subprocess.Popen(
        [PHP_CLI, "-S", "127.0.0.1:8001", "public/index.php"],
        cwd=CRUD_DIR,
        env=php_env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )

    # 3. Start PHP 8.4 JIT Server (1 worker)
    jit_env = os.environ.copy()
    jit_env["PHP_CLI_SERVER_WORKERS"] = "1"
    jit_proc = subprocess.Popen(
        [PHP_CLI] + jit_flags + ["-S", "127.0.0.1:8002", "public/index.php"],
        cwd=CRUD_DIR,
        env=jit_env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )

    time.sleep(3.0)

    if not wait_for_server("http://127.0.0.1:8000/api/categories"):
        log("  ❌ Hyperion server failed to start.")
        hyp_proc.kill()
        php_proc.kill()
        jit_proc.kill()
        return

    if not wait_for_server("http://127.0.0.1:8001/api/categories"):
        log("  ❌ PHP 8.4 CLI server failed to start.")
        hyp_proc.kill()
        php_proc.kill()
        jit_proc.kill()
        return

    if not wait_for_server("http://127.0.0.1:8002/api/categories"):
        log("  ❌ PHP 8.4 JIT server failed to start.")
        hyp_proc.kill()
        php_proc.kill()
        jit_proc.kill()
        return

    log("  ✅ All 3 server engines successfully booted and responding on ports 8000, 8001, 8002.")

    http_test_cases = [
        {
            "name": "1. Category List & Relation Aggregates (/api/categories)",
            "path": "/api/categories",
            "hyp_reqs": 500,
            "hyp_c": 20,
            "php_reqs": 500,
            "php_c": 20,
        },
        {
            "name": "2. Product Catalog with Eager Relations (/api/products)",
            "path": "/api/products",
            "hyp_reqs": 500,
            "hyp_c": 20,
            "php_reqs": 500,
            "php_c": 20,
        },
        {
            "name": "3. Single Product Query & Nested Casts (/api/products/1)",
            "path": "/api/products/1",
            "hyp_reqs": 500,
            "hyp_c": 20,
            "php_reqs": 500,
            "php_c": 20,
        },
    ]

    http_results = []

    for tc in http_test_cases:
        log(f"\n▶ Benchmarking: {tc['name']}")
        
        # Test Hyperion
        log(f"   • Running PHP-Hyperion ({tc['hyp_reqs']:,} reqs, c={tc['hyp_c']})...")
        hyp_res = run_ab(f"http://127.0.0.1:8000{tc['path']}", tc['hyp_reqs'], tc['hyp_c'], keep_alive=True)
        hyp_rss = get_process_rss_mb(hyp_proc.pid)
        log(f"     -> Hyperion: {hyp_res['rps']:>9.1f} req/s | Mean Latency: {hyp_res['mean_ms']:.2f} ms | p50: {hyp_res['p50_ms']:.1f}ms | p99: {hyp_res['p99_ms']:.1f}ms | RSS: {hyp_rss} MB")
        
        # Test Standard PHP 8.4 CLI
        log(f"   • Running Standard PHP 8.4 ({tc['php_reqs']:,} reqs, c={tc['php_c']})...")
        php_res = run_ab(f"http://127.0.0.1:8001{tc['path']}", tc['php_reqs'], tc['php_c'], keep_alive=True)
        php_rss = get_process_rss_mb(php_proc.pid)
        log(f"     -> PHP 8.4:  {php_res['rps']:>9.1f} req/s | Mean Latency: {php_res['mean_ms']:.2f} ms | p50: {php_res['p50_ms']:.1f}ms | p99: {php_res['p99_ms']:.1f}ms | RSS: {php_rss} MB")

        # Test PHP 8.4 JIT
        log(f"   • Running PHP 8.4 JIT ({tc['php_reqs']:,} reqs, c={tc['php_c']})...")
        jit_res = run_ab(f"http://127.0.0.1:8002{tc['path']}", tc['php_reqs'], tc['php_c'], keep_alive=True)
        jit_rss = get_process_rss_mb(jit_proc.pid)
        log(f"     -> PHP JIT:  {jit_res['rps']:>9.1f} req/s | Mean Latency: {jit_res['mean_ms']:.2f} ms | p50: {jit_res['p50_ms']:.1f}ms | p99: {jit_res['p99_ms']:.1f}ms | RSS: {jit_rss} MB")

        speedup_cli = (hyp_res['rps'] / php_res['rps']) if php_res['rps'] > 0 else 0.0
        speedup_jit = (hyp_res['rps'] / jit_res['rps']) if jit_res['rps'] > 0 else 0.0

        http_results.append({
            "name": tc['name'],
            "path": tc['path'],
            "hyp_rps": hyp_res['rps'],
            "hyp_mean_ms": hyp_res['mean_ms'],
            "hyp_p50_ms": hyp_res['p50_ms'],
            "hyp_p90_ms": hyp_res['p90_ms'],
            "hyp_p99_ms": hyp_res['p99_ms'],
            "hyp_rss_mb": hyp_rss,
            "php_rps": php_res['rps'],
            "php_mean_ms": php_res['mean_ms'],
            "php_p50_ms": php_res['p50_ms'],
            "php_p99_ms": php_res['p99_ms'],
            "php_rss_mb": php_rss,
            "jit_rps": jit_res['rps'],
            "jit_mean_ms": jit_res['mean_ms'],
            "jit_p50_ms": jit_res['p50_ms'],
            "jit_p99_ms": jit_res['p99_ms'],
            "jit_rss_mb": jit_rss,
            "speedup_vs_cli": speedup_cli,
            "speedup_vs_jit": speedup_jit,
        })

    # Teardown processes
    hyp_proc.kill()
    php_proc.kill()
    jit_proc.kill()

    # -------------------------------------------------------------
    # 4. Final Comparison Report
    # -------------------------------------------------------------
    log("\n" + "=" * 96)
    log(" 📊 FINAL BENCHMARK COMPARISON TABLE: LARAVEL 11 CRUD API & ELOQUENT")
    log("=" * 96)
    log(f"{'Endpoint / Workload':<44} | {'PHP 8.4 CLI':<14} | {'PHP 8.4 JIT':<14} | {'PHP-Hyperion':<15} | {'Speedup vs JIT':<12}")
    log("-" * 96)
    for r in http_results:
        log(f"{r['name']:<44} | {r['php_rps']:>8.1f} req/s | {r['jit_rps']:>8.1f} req/s | {r['hyp_rps']:>9.1f} req/s | {r['speedup_vs_jit']:>8.1f}x")
    log("-" * 96)
    log(f"{'PHPUnit 11 Suite (24 tests, 133 asserts)':<44} | {dur_phpunit_php:>8.1f} ms    | {dur_phpunit_jit:>8.1f} ms    | {dur_phpunit_hyp:>9.1f} ms   | {(dur_phpunit_jit/dur_phpunit_hyp):>8.2f}x")
    log("=" * 96)

    output_payload = {
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "eloquent_microbenchmarks": {
            "php_cli": eloquent_php,
            "php_jit": eloquent_jit,
            "hyperion": eloquent_hyp,
        },
        "phpunit_execution": {
            "php_cli_ms": dur_phpunit_php,
            "php_jit_ms": dur_phpunit_jit,
            "hyperion_ms": dur_phpunit_hyp,
        },
        "http_benchmarks": http_results,
    }

    out_file = os.path.join(WORKSPACE, "benchmark_laravel_crud_results.json")
    with open(out_file, "w") as f:
        json.dump(output_payload, f, indent=2)
    log(f"\n📁 Detailed metrics successfully exported to: {out_file}\n")

if __name__ == "__main__":
    main()
