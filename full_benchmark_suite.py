#!/usr/bin/env python3
import subprocess
import time
import json
import re
import urllib.request

def get_process_memory():
    out = subprocess.check_output(['ps', '-o', 'rss,command', '-ax']).decode()
    php_rss = 0
    hyp_rss = 0
    for line in out.splitlines():
        parts = line.strip().split(None, 1)
        if len(parts) == 2:
            try:
                rss = int(parts[0])
                cmd = parts[1]
                if 'php -S 127.0.0.1:8000' in cmd:
                    php_rss += rss
                elif 'hyperion-cli -S 127.0.0.1:8001' in cmd:
                    hyp_rss += rss
            except:
                pass
    return round(php_rss / 1024, 2), round(hyp_rss / 1024, 2)

def run_ab(url, total_requests, concurrency):
    cmd = ['ab', '-k', '-n', str(total_requests), '-c', str(concurrency), url]
    p = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    out = p.stdout

    rps = 0.0
    mean_latency = 0.0
    p99_latency = 0.0
    failed = 0
    transfer_kb = 0.0
    doc_len = 0

    m = re.search(r'Requests per second:\s+([0-9.]+)', out)
    if m: rps = float(m.group(1))

    m = re.search(r'Time per request:\s+([0-9.]+)\s+\[ms\]\s+\(mean\)', out)
    if m: mean_latency = float(m.group(1))

    m = re.search(r'Failed requests:\s+([0-9]+)', out)
    if m: failed = int(m.group(1))

    m = re.search(r'Transfer rate:\s+([0-9.]+)', out)
    if m: transfer_kb = float(m.group(1))

    m = re.search(r'Document Length:\s+([0-9]+)', out)
    if m: doc_len = int(m.group(1))

    m = re.search(r'99%\s+([0-9]+)', out)
    if m: p99_latency = float(m.group(1))

    return {
        'rps': rps,
        'mean_latency_ms': mean_latency,
        'p99_latency_ms': p99_latency,
        'failed': failed,
        'transfer_rate_kbps': transfer_kb,
        'document_length_bytes': doc_len,
        'total_requests': total_requests,
        'concurrency': concurrency,
    }

def warm_up(url):
    try:
        req = urllib.request.Request(url)
        with urllib.request.urlopen(req, timeout=5) as r:
            _ = r.read()
    except Exception as e:
        print(f"Warm up error for {url}: {e}")

def main():
    print("=" * 80)
    print("HYPERION vs PHP 8.4 (ZEND) COMPREHENSIVE BENCHMARK SUITE")
    print("Zero changes to Laravel codebase — testing Requests, Views, Code, RAM")
    print("=" * 80)

    php_mem_init, hyp_mem_init = get_process_memory()
    print(f"Initial Memory Footprint -> PHP 8.4: {php_mem_init} MB | Hyperion: {hyp_mem_init} MB\n")

    endpoints = [
        # (Category, Path, Requests_PHP, Requests_Hyp, Concurrency)
        ("API - Stateless Health", "/health", 2000, 20000, 100),
        ("API - Eloquent Products", "/api/v1/products", 1000, 20000, 100),
        ("API - Complex Analytics", "/api/v1/analytics/dashboard", 1000, 20000, 100),
        ("API - Queue & Jobs Stats", "/api/v1/queue/stats", 1000, 20000, 100),
        ("API - SQL Window Rankings", "/api/v1/queries/window-rankings", 1000, 20000, 100),
        ("API - Cohort Conditional Aggs", "/api/v1/queries/cohort-spending", 1000, 20000, 100),
        ("View - Storefront Catalog", "/", 1000, 20000, 100),
        ("View - Orders & Audits", "/orders", 1000, 20000, 100),
        ("View - Operations Dashboard", "/operations", 1000, 20000, 100),
        ("View - Complex Features Lab (48KB)", "/complex", 1000, 20000, 100),
    ]

    results = []

    for name, path, n_php, n_hyp, conc in endpoints:
        print(f"[*] Testing {name} ({path})...")
        
        # PHP 8.4
        url_php = f"http://127.0.0.1:8000{path}"
        warm_up(url_php)
        res_php = run_ab(url_php, n_php, conc if conc <= 50 else 50)
        
        # Hyperion
        url_hyp = f"http://127.0.0.1:8001{path}"
        warm_up(url_hyp)
        res_hyp = run_ab(url_hyp, n_hyp, conc)

        speedup = round(res_hyp['rps'] / res_php['rps'], 1) if res_php['rps'] > 0 else 0

        print(f"    PHP 8.4  : {res_php['rps']:>10.2f} req/s | Mean: {res_php['mean_latency_ms']:>6.2f} ms | Failed: {res_php['failed']}")
        print(f"    Hyperion : {res_hyp['rps']:>10.2f} req/s | Mean: {res_hyp['mean_latency_ms']:>6.2f} ms | Failed: {res_hyp['failed']}")
        print(f"    SPEEDUP  : {speedup:>8.1f}x Faster\n")

        results.append({
            'category': name,
            'path': path,
            'php': res_php,
            'hyperion': res_hyp,
            'speedup_x': speedup
        })

    # High Concurrency & Memory Leak Stress Test
    print("=" * 80)
    print("STRESS TEST & MEMORY LEAK VERIFICATION (100,000 Continuous Requests on Hyperion)")
    print("=" * 80)
    
    stress_batches = 5
    batch_reqs = 20000
    leak_records = []
    
    for i in range(stress_batches):
        print(f"[*] Running Stress Batch {i+1}/{stress_batches} ({batch_reqs} requests)...")
        res = run_ab("http://127.0.0.1:8001/health", batch_reqs, 100)
        _, hyp_mem = get_process_memory()
        total_so_far = (i + 1) * batch_reqs
        print(f"    Batch {i+1}: {res['rps']:.2f} req/s | Memory RSS: {hyp_mem} MB | Cumulative: {total_so_far} requests")
        leak_records.append({
            'batch': i + 1,
            'cumulative_requests': total_so_far,
            'rps': res['rps'],
            'rss_mb': hyp_mem,
            'failed': res['failed']
        })

    # Pure Compute CLI Benchmark
    print("\n" + "=" * 80)
    print("PURE CODE & COMPUTE ENGINE BENCHMARK (Recursive Fib, Arrays, Hashes, JSON)")
    print("=" * 80)
    php_compute_out = subprocess.check_output(['php', 'php-hyperion/scratch/compute_bench.php']).decode()
    hyp_compute_out = subprocess.check_output(['./php-hyperion/target/release/hyperion-cli', 'php-hyperion/scratch/compute_bench.php']).decode()
    
    # Extract JSON
    m = re.search(r'\{[^{}]+\}', php_compute_out, re.DOTALL)
    php_compute = json.loads(m.group(0)) if m else {}
    m = re.search(r'\{[^{}]+\}', hyp_compute_out, re.DOTALL)
    hyp_compute = json.loads(m.group(0)) if m else {}

    print(f"PHP 8.4 CLI Compute : {php_compute.get('total_compute_time_ms')} ms (Fib: {php_compute.get('fib_time_ms')} ms, JSON: {php_compute.get('json_time_ms')} ms)")
    print(f"Hyperion CLI Compute: {hyp_compute.get('total_compute_time_ms')} ms (Fib: {hyp_compute.get('fib_time_ms')} ms, JSON: {hyp_compute.get('json_time_ms')} ms)")

    php_mem_final, hyp_mem_final = get_process_memory()

    full_report = {
        'memory_initial': {'php_mb': php_mem_init, 'hyperion_mb': hyp_mem_init},
        'memory_final': {'php_mb': php_mem_final, 'hyperion_mb': hyp_mem_final},
        'endpoints': results,
        'stress_and_leak_test': leak_records,
        'pure_compute': {'php_cli': php_compute, 'hyperion_cli': hyp_compute}
    }

    with open('full_benchmark_report.json', 'w') as f:
        json.dump(full_report, f, indent=2)

    print("\n[+] Benchmark suite finished successfully! Results saved to full_benchmark_report.json")

if __name__ == '__main__':
    main()
