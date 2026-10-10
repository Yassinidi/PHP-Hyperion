#!/usr/bin/env python3
"""
⚡ E-COMMERCE PROJECT VERIFIER & BENCHMARK SUITE
Verifies:
  1. Standard PHP 8.4 execution of all e-commerce domain features
  2. ApacheBench throughput & latency under Standard PHP
  3. PHP-Hyperion persistent execution of all features
  4. ApacheBench throughput & latency under PHP-Hyperion
  5. Automatic error recording if any failure occurs
"""

import os
import sys
import time
import json
import subprocess
import urllib.request
import urllib.error
import argparse

WORKSPACE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
HYPERION_CLI = os.path.join(WORKSPACE, "php-hyperion", "target", "release", "hyperion-cli")
PHP_CLI = "/opt/homebrew/bin/php" if os.path.exists("/opt/homebrew/bin/php") else "php"
AB_BIN = "/usr/sbin/ab"

def kill_port(port):
    subprocess.run(["pkill", "-9", "-f", f"127.0.0.1:{port}"], stderr=subprocess.DEVNULL)
    time.sleep(0.5)

def wait_for_server(url, retries=30, delay=0.2):
    for _ in range(retries):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "Probe"})
            with urllib.request.urlopen(req, timeout=1.5) as resp:
                if resp.status in (200, 302, 404):
                    return True
        except Exception:
            time.sleep(delay)
    return False

def http_get(url):
    req = urllib.request.Request(url, headers={"User-Agent": "Tester"})
    with urllib.request.urlopen(req, timeout=5) as resp:
        return resp.status, resp.read().decode('utf-8')

def http_post(url, json_data):
    data = json.dumps(json_data).encode('utf-8')
    req = urllib.request.Request(url, data=data, headers={"Content-Type": "application/json", "User-Agent": "Tester"})
    with urllib.request.urlopen(req, timeout=5) as resp:
        return resp.status, resp.read().decode('utf-8')

def test_ecommerce_features(base_url):
    results = {}
    print(f"   ▶ Testing domain features against {base_url}...")

    # 1. Storefront Home
    status, body = http_get(f"{base_url}/")
    assert status == 200, f"Expected 200 on /, got {status}"
    results['storefront_home'] = "PASS"

    # 2. Banners API
    status, body = http_get(f"{base_url}/api/banners")
    assert status == 200, f"Expected 200 on /api/banners, got {status}"
    data = json.loads(body)
    banners = data.get('data', [])
    assert len(banners) >= 1, "Expected at least 1 banner"
    results['banners'] = f"PASS ({len(banners)} banners)"

    # 3. Categories API
    status, body = http_get(f"{base_url}/api/categories")
    assert status == 200, f"Expected 200 on /api/categories, got {status}"
    data = json.loads(body)
    categories = data.get('data', [])
    assert len(categories) >= 1, "Expected at least 1 category"
    results['categories'] = f"PASS ({len(categories)} categories)"

    # 4. Products API
    status, body = http_get(f"{base_url}/api/products")
    assert status == 200, f"Expected 200 on /api/products, got {status}"
    data = json.loads(body)
    products = data.get('data', [])
    assert len(products) >= 1, "Expected at least 1 product"
    results['products'] = f"PASS ({len(products)} products)"

    # 5. Product Detail API
    prod_id = products[0]['id']
    status, body = http_get(f"{base_url}/api/products/{prod_id}")
    assert status == 200, f"Expected 200 on /api/products/{prod_id}, got {status}"
    detail = json.loads(body).get('data', {})
    assert detail.get('id') == prod_id, "Product id mismatch"
    results['product_detail'] = f"PASS (ID {prod_id})"

    # 6. Cart Checkout API
    checkout_payload = {
        "user_id": 2,
        "items": [
            {"product_id": prod_id, "quantity": 1}
        ]
    }
    status, body = http_post(f"{base_url}/api/cart/checkout", checkout_payload)
    assert status in (200, 201), f"Expected 200/201 on /api/cart/checkout, got {status}"
    chk = json.loads(body)
    assert chk.get('success') is True, f"Checkout failed: {body}"
    results['cart_checkout'] = f"PASS (Order #{chk.get('order_id')})"

    # 7. Admin Dashboard KPI
    status, body = http_get(f"{base_url}/api/admin/dashboard")
    assert status == 200, f"Expected 200 on /api/admin/dashboard, got {status}"
    dash = json.loads(body).get('data', {})
    assert 'metrics' in dash, "Metrics not found in dashboard"
    results['admin_dashboard'] = f"PASS (${dash['metrics'].get('total_revenue')} revenue)"

    # 8. Admin Users List
    status, body = http_get(f"{base_url}/api/admin/users")
    assert status == 200, f"Expected 200 on /api/admin/users, got {status}"
    users = json.loads(body).get('data', [])
    assert len(users) >= 2, "Expected admin and client users"
    roles = {u['role'] for u in users}
    assert 'admin' in roles and 'client' in roles, f"Roles admin and client required, found {roles}"
    results['admin_and_client_users'] = f"PASS ({len(users)} users, roles: {list(roles)})"

    return results

def run_benchmark(url, n=2000, c=50):
    cmd = [AB_BIN, "-k", "-n", str(n), "-c", str(c), url]
    res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    out = res.stdout

    import re
    rps = 0.0
    mean_ms = 0.0
    m_rps = re.search(r"Requests per second:\s+([0-9.]+)", out)
    if m_rps:
        rps = float(m_rps.group(1))
    m_time = re.search(r"Time per request:\s+([0-9.]+)\s+\[ms\]\s+\(mean\)", out)
    if m_time:
        mean_ms = float(m_time.group(1))

    return {"rps": rps, "mean_ms": mean_ms, "requests": n, "concurrency": c}

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--name", required=True)
    parser.add_argument("--dir", required=True)
    parser.add_argument("--docroot", default="public")
    parser.add_argument("--entry", default="")
    parser.add_argument("--port-php", type=int, default=8081)
    parser.add_argument("--port-hyp", type=int, default=8082)
    parser.add_argument("--bench-n", type=int, default=3000)
    parser.add_argument("--bench-c", type=int, default=50)

    args = parser.parse_args()

    project_dir = os.path.join(WORKSPACE, "frameworks", args.dir)
    docroot_dir = os.path.join(project_dir, args.docroot)
    error_file = os.path.join(project_dir, f"error_details_{args.name}.md")

    print(f"\n{'='*75}")
    print(f" 🚀 E-COMMERCE VERIFICATION: {args.name.upper()} (Dir: frameworks/{args.dir})")
    print(f"{'='*75}")

    kill_port(args.port_php)
    kill_port(args.port_hyp)

    # ----------------------------------------------------
    # STEP 1: Run with Standard PHP 8.4
    # ----------------------------------------------------
    print(f"\n[PHASE 1] Starting Standard PHP 8.4 on port {args.port_php}...")
    php_env = os.environ.copy()
    php_env["PHP_CLI_SERVER_WORKERS"] = "8"
    php_cmd = [PHP_CLI, "-S", f"127.0.0.1:{args.port_php}", "-t", docroot_dir]
    if args.entry:
        php_cmd.append(os.path.join(docroot_dir, args.entry))

    proc_php = subprocess.Popen(php_cmd, cwd=project_dir, env=php_env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    
    if not wait_for_server(f"http://127.0.0.1:{args.port_php}/"):
        print(f"❌ Standard PHP failed to bind on port {args.port_php}!")
        proc_php.kill()
        sys.exit(1)

    print("   ✓ Standard PHP 8.4 is live.")
    try:
        php_features = test_ecommerce_features(f"http://127.0.0.1:{args.port_php}")
        print("   ✅ All E-Commerce domain features verified successfully under Standard PHP!")
        for feat, status in php_features.items():
            print(f"      • {feat:<24}: {status}")
    except Exception as e:
        print(f"❌ Standard PHP Feature Test Failed: {e}")
        proc_php.kill()
        sys.exit(1)

    # Benchmark Standard PHP
    print(f"\n   ▶ Running ApacheBench on Standard PHP (n={args.bench_n}, c={args.bench_c})...")
    php_bench_home = run_benchmark(f"http://127.0.0.1:{args.port_php}/", n=args.bench_n, c=args.bench_c)
    php_bench_api = run_benchmark(f"http://127.0.0.1:{args.port_php}/api/products", n=args.bench_n, c=args.bench_c)
    print(f"      • Storefront Home : {php_bench_home['rps']:.1f} req/s (Latency: {php_bench_home['mean_ms']:.2f} ms)")
    print(f"      • Products API    : {php_bench_api['rps']:.1f} req/s (Latency: {php_bench_api['mean_ms']:.2f} ms)")

    proc_php.kill()
    kill_port(args.port_php)

    # ----------------------------------------------------
    # STEP 2: Run with Next-Gen PHP-Hyperion
    # ----------------------------------------------------
    print(f"\n[PHASE 2] Starting PHP-Hyperion on port {args.port_hyp}...")
    hyp_env = os.environ.copy()
    hyp_env["PHP_CLI_SERVER_WORKERS"] = "16"
    hyp_cmd = [HYPERION_CLI, "-S", f"127.0.0.1:{args.port_hyp}", "-t", docroot_dir]
    if args.entry:
        hyp_cmd.append(os.path.join(docroot_dir, args.entry))

    hyp_log_path = os.path.join(project_dir, "hyperion.log")
    hyp_log = open(hyp_log_path, "w")
    proc_hyp = subprocess.Popen(hyp_cmd, cwd=project_dir, env=hyp_env, stdout=hyp_log, stderr=hyp_log)

    if not wait_for_server(f"http://127.0.0.1:{args.port_hyp}/"):
        hyp_log.flush()
        with open(hyp_log_path, "r") as f:
            stderr = f.read()
        print(f"❌ PHP-Hyperion failed to start on port {args.port_hyp}!")
        # Write error details file as requested
        with open(error_file, "w") as f:
            f.write(f"# Error Details for {args.name}\n\n")
            f.write(f"Hyperion failed to bind or answer health probe on 127.0.0.1:{args.port_hyp}.\n\n")
            f.write(f"## Stderr output:\n```\n{stderr}\n```\n")
        print(f"⚠️ Wrote error details to {error_file}")
        proc_hyp.kill()
        sys.exit(1)

    print("   ✓ PHP-Hyperion is live.")
    try:
        hyp_features = test_ecommerce_features(f"http://127.0.0.1:{args.port_hyp}")
        print("   ✅ All E-Commerce domain features verified successfully under PHP-Hyperion!")
        for feat, status in hyp_features.items():
            print(f"      • {feat:<24}: {status}")
    except Exception as e:
        print(f"❌ PHP-Hyperion Feature Test Failed: {e}")
        with open(error_file, "w") as f:
            f.write(f"# Error Details for {args.name}\n\n")
            f.write(f"Functional verification failed on Hyperion: {e}\n")
        print(f"⚠️ Wrote error details to {error_file}")
        proc_hyp.kill()
        sys.exit(1)

    # Benchmark PHP-Hyperion
    print(f"\n   ▶ Running ApacheBench on PHP-Hyperion (n={args.bench_n}, c={args.bench_c})...")
    hyp_bench_home = run_benchmark(f"http://127.0.0.1:{args.port_hyp}/", n=args.bench_n, c=args.bench_c)
    hyp_bench_api = run_benchmark(f"http://127.0.0.1:{args.port_hyp}/api/products", n=args.bench_n, c=args.bench_c)
    print(f"      • Storefront Home : {hyp_bench_home['rps']:.1f} req/s (Latency: {hyp_bench_home['mean_ms']:.2f} ms)")
    print(f"      • Products API    : {hyp_bench_api['rps']:.1f} req/s (Latency: {hyp_bench_api['mean_ms']:.2f} ms)")

    proc_hyp.kill()
    kill_port(args.port_hyp)

    # Calculate speedups
    speedup_home = round(hyp_bench_home['rps'] / max(php_bench_home['rps'], 0.1), 2)
    speedup_api = round(hyp_bench_api['rps'] / max(php_bench_api['rps'], 0.1), 2)

    # Save benchmark report
    report_file = os.path.join(project_dir, "benchmark_report.json")
    report_data = {
        "project": args.name,
        "concurrency": args.bench_c,
        "total_requests": args.bench_n,
        "storefront_home": {
            "standard_php_rps": php_bench_home['rps'],
            "hyperion_rps": hyp_bench_home['rps'],
            "speedup": f"{speedup_home}x",
        },
        "products_api": {
            "standard_php_rps": php_bench_api['rps'],
            "hyperion_rps": hyp_bench_api['rps'],
            "speedup": f"{speedup_api}x",
        },
        "features_verified": hyp_features,
    }
    with open(report_file, "w") as f:
        json.dump(report_data, f, indent=2)

    print(f"\n{'='*75}")
    print(f" 🏁 SUMMARY REPORT: {args.name.upper()}")
    print(f"{'='*75}")
    print(f" Storefront Home : PHP {php_bench_home['rps']:.1f} req/s  ➔  Hyperion {hyp_bench_home['rps']:.1f} req/s  ({speedup_home}x Faster)")
    print(f" Products API    : PHP {php_bench_api['rps']:.1f} req/s  ➔  Hyperion {hyp_bench_api['rps']:.1f} req/s  ({speedup_api}x Faster)")
    print(f" Report Saved    : {report_file}")
    print(f"{'='*75}\n")

if __name__ == "__main__":
    main()
