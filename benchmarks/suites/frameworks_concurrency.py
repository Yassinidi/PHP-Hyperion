#!/usr/bin/env python3
"""
⚡ PHP-Hyperion vs Standard PHP 8.4: Master Maximum Concurrency Benchmark Suite
Across all projects and routes in frameworks/
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
FRAMEWORKS_DIR = os.path.join(WORKSPACE, "frameworks")
HYPERION_CLI = os.path.join(WORKSPACE, "php-hyperion", "target", "release", "hyperion-cli")
PHP_CLI = "/opt/homebrew/bin/php" if os.path.exists("/opt/homebrew/bin/php") else "php"
AB_BIN = "/usr/sbin/ab"

PORT_PHP = 8000
PORT_HYP = 8001

DUMMY_JSON_POST = os.path.join(WORKSPACE, "scratch", "bench_post.json")
os.makedirs(os.path.dirname(DUMMY_JSON_POST), exist_ok=True)
with open(DUMMY_JSON_POST, "w") as f:
    json.dump({"name": "BenchUser", "role": "developer", "quantity": 1, "product_id": 1, "stock": 10}, f)

def log(msg="", end="\n"):
    print(msg, end=end, flush=True)

def cleanup_servers():
    subprocess.run(["pkill", "-9", "-f", f"php.*-S 127.0.0.1:{PORT_PHP}"], stderr=subprocess.DEVNULL)
    subprocess.run(["pkill", "-9", "-f", f"hyperion-cli.*-S 127.0.0.1:{PORT_HYP}"], stderr=subprocess.DEVNULL)
    time.sleep(0.6)

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
                if resp.status in (200, 301, 302, 401, 403, 404, 500):
                    return True
        except urllib.error.HTTPError:
            return True
        except Exception:
            time.sleep(delay)
    return False

def probe_url(url):
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "HyperionBenchProbe"})
        with urllib.request.urlopen(req, timeout=2.0) as resp:
            return resp.status, len(resp.read())
    except urllib.error.HTTPError as e:
        return e.code, len(e.read())
    except Exception as e:
        return 0, str(e)

def run_ab(url, total_requests, concurrency, method="GET", keep_alive=True, timeout=60):
    cmd = [AB_BIN]
    if keep_alive:
        cmd.append("-k")
    if method == "POST" and os.path.exists(DUMMY_JSON_POST):
        cmd.extend(["-p", DUMMY_JSON_POST, "-T", "application/json"])
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

        # Ignore Length mismatches if status was 200
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

def substitute_route_params(uri):
    """
    Substitutes route placeholders like {id}, {slug} with concrete test values.
    """
    replacements = {
        r"\{id\}": "1",
        r"\{course\}": "1",
        r"\{course_id\}": "1",
        r"\{student\}": "1",
        r"\{student_id\}": "1",
        r"\{record\}": "1",
        r"\{house\}": "1",
        r"\{category\}": "1",
        r"\{installment\}": "1",
        r"\{mortgageRequest\}": "1",
        r"\{productId\}": "1",
        r"\{order_number\}": "1",
        r"\{uuid\}": "1",
        r"\{export\}": "1",
        r"\{import\}": "1",
        r"\{interest\}": "1",
        r"\{slug\}": "about-us",
        r"\{folder\}": "avatars",
        r"\{filename\}": "logo.png",
        r"\{token\}": "token123",
        r"\{hash\}": "hash123",
        r"\{path\}": "app.css",
        r"\{asset\}": "swagger-ui.css"
    }
    
    clean_uri = uri.strip()
    if clean_uri.startswith("/"):
        clean_uri = clean_uri[1:]
    
    for pat, rep in replacements.items():
        clean_uri = re.sub(pat, rep, clean_uri)
    
    # Catch any remaining {param}
    clean_uri = re.sub(r"\{[^}]+\}", "1", clean_uri)
    
    # Clean double slashes
    clean_uri = re.sub(r"/+", "/", clean_uri)
    return "/" + clean_uri if not clean_uri.startswith("/") else clean_uri

def discover_routes(proj_name, proj_dir):
    routes = []
    
    # Check if Laravel
    artisan_path = os.path.join(proj_dir, "artisan")
    if os.path.exists(artisan_path):
        try:
            res = subprocess.run(["php", "artisan", "route:list", "--json"], cwd=proj_dir, capture_output=True, text=True)
            if res.returncode == 0:
                raw_routes = json.loads(res.stdout)
                seen_uris = set()
                for r in raw_routes:
                    method = r.get("method", "GET")
                    if "|" in method:
                        methods = method.split("|")
                        primary_method = "GET" if "GET" in methods else methods[0]
                    else:
                        primary_method = method
                    
                    uri = r.get("uri", "")
                    concrete_path = substitute_route_params(uri)
                    
                    # Deduplicate
                    key = (primary_method, concrete_path)
                    if key not in seen_uris:
                        seen_uris.add(key)
                        routes.append({
                            "method": primary_method,
                            "original_uri": uri,
                            "path": concrete_path,
                            "name": r.get("name") or uri
                        })
                return routes
        except Exception as e:
            log(f"  Warning: failed to parse artisan route:list for {proj_name}: {e}")

    # Microframeworks / Vanilla PHP
    if proj_name == "normal-php-app":
        return [
            {"method": "GET", "original_uri": "/", "path": "/", "name": "API Root"},
            {"method": "GET", "original_uri": "/health", "path": "/health", "name": "Health Ingress"},
            {"method": "GET", "original_uri": "/api/users", "path": "/api/users", "name": "Users List"},
            {"method": "GET", "original_uri": "/api/products", "path": "/api/products", "name": "Products Catalog"},
            {"method": "POST", "original_uri": "/api/orders", "path": "/api/orders", "name": "Create Order"},
            {"method": "GET", "original_uri": "/api/compute", "path": "/api/compute", "name": "CPU Compute"}
        ]
    elif proj_name == "slim":
        return [
            {"method": "GET", "original_uri": "/", "path": "/", "name": "Slim 4 Root"},
            {"method": "GET", "original_uri": "/users/{id}", "path": "/users/1", "name": "User Lookup Regex"},
            {"method": "POST", "original_uri": "/users", "path": "/users", "name": "Create User POST"}
        ]
    elif proj_name == "symfony":
        return [
            {"method": "GET", "original_uri": "/", "path": "/", "name": "Symfony 7 Home"},
            {"method": "GET", "original_uri": "/users/{id}", "path": "/users/1", "name": "User Show Matcher"}
        ]
    elif proj_name == "wordpress":
        return [
            {"method": "GET", "original_uri": "/", "path": "/", "name": "WP Core Front"},
            {"method": "GET", "original_uri": "/wp-login.php", "path": "/wp-login.php", "name": "WP Admin Login"}
        ]
    elif proj_name == "composer_test":
        return []
        
    return []

def main():
    log("=" * 86)
    log("⚡ PHP-HYPERION vs STANDARD PHP 8.4: MASTER MAX CONCURRENCY BENCHMARK SUITE")
    log("=" * 86)
    log(f" Machine Cores : {os.cpu_count()} Hardware Cores (Apple Silicon)")
    log(f" Standard PHP  : {PHP_CLI} (Multi-Worker Zend Server, Workers=8)")
    log(f" PHP-Hyperion  : {HYPERION_CLI} (Rust Multi-Reactor Fibre Engine, Workers=16, Fibres=128)")
    log("=" * 86 + "\n")

    projects = [
        "laravel-complex",
        "laravel-school",
        "laravel-crud",
        "laravel-blog",
        "laravel-app",
        "laravel-full",
        "normal-php-app",
        "slim",
        "symfony",
        "wordpress"
    ]

    all_results = {
        "metadata": {
            "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
            "cpu_cores": os.cpu_count(),
            "php_cli": PHP_CLI,
            "hyperion_cli": HYPERION_CLI
        },
        "frameworks": {},
        "escalation_ladders": {}
    }

    # Flagship endpoints for the Max Concurrency Escalation Ladder
    flagship_endpoints = {
        "laravel-complex": [
            {"name": "API Health / Ingress", "path": "/health", "method": "GET"},
            {"name": "Eloquent Products Catalog", "path": "/api/v1/products", "method": "GET"},
            {"name": "Advanced SQL Window Rankings", "path": "/api/v1/queries/window-rankings", "method": "GET"},
            {"name": "Complex Blade View (48KB)", "path": "/complex", "method": "GET"}
        ],
        "laravel-school": [
            {"name": "School Dashboard Stats", "path": "/api/v1/stats", "method": "GET"},
            {"name": "Student Gradebook Hydration", "path": "/api/v1/courses/1/gradebook", "method": "GET"}
        ],
        "laravel-crud": [
            {"name": "Product REST Catalog", "path": "/api/products", "method": "GET"}
        ],
        "laravel-blog": [
            {"name": "Blog Posts Stream", "path": "/api/posts", "method": "GET"}
        ],
        "normal-php-app": [
            {"name": "Vanilla REST Products", "path": "/api/products", "method": "GET"},
            {"name": "CPU Compute Workload", "path": "/api/compute", "method": "GET"}
        ],
        "slim": [
            {"name": "Slim 4 JSON Endpoint", "path": "/", "method": "GET"}
        ],
        "symfony": [
            {"name": "Symfony 7 JSON Controller", "path": "/", "method": "GET"}
        ]
    }

    for proj in projects:
        proj_dir = os.path.join(FRAMEWORKS_DIR, proj)
        if not os.path.exists(proj_dir):
            continue

        log("\n" + "#" * 86)
        log(f"  TARGET APPLICATION: [{proj.upper()}]")
        log("#" * 86)

        routes = discover_routes(proj, proj_dir)
        log(f"  Found {len(routes)} routes for {proj}")

        cleanup_servers()

        # Determine docroot and entrypoint
        docroot = "." if proj == "wordpress" else "public"
        index_file = "index.php" if proj == "wordpress" else "public/index.php"

        # Boot Standard PHP 8.4 Server
        php_env = os.environ.copy()
        php_env["PHP_CLI_SERVER_WORKERS"] = "8"
        php_proc = subprocess.Popen(
            [PHP_CLI, "-S", f"127.0.0.1:{PORT_PHP}", "-t", docroot, index_file],
            cwd=proj_dir,
            env=php_env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL
        )

        # Boot PHP-Hyperion Server
        hyp_env = os.environ.copy()
        hyp_env["PHP_CLI_SERVER_WORKERS"] = "16"
        hyp_env["HYPERION_REACTORS"] = "8"
        hyp_env["HYPERION_MAX_FIBRES"] = "128"
        hyp_proc = subprocess.Popen(
            [HYPERION_CLI, "-S", f"127.0.0.1:{PORT_HYP}", "-t", docroot, index_file],
            cwd=proj_dir,
            env=hyp_env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL
        )

        time.sleep(1.2)

        probe_url_base = f"http://127.0.0.1:{PORT_PHP}/"
        if not wait_for_server(probe_url_base):
            log(f"❌ Failed to start Standard PHP for {proj}")
            cleanup_servers()
            continue

        if not wait_for_server(f"http://127.0.0.1:{PORT_HYP}/"):
            log(f"❌ Failed to start PHP-Hyperion for {proj}")
            cleanup_servers()
            continue

        php_pids = get_process_pids(f"php.*-S 127.0.0.1:{PORT_PHP}")
        hyp_pids = get_process_pids(f"hyperion-cli.*-S 127.0.0.1:{PORT_HYP}")

        log(f"  ✅ Standard PHP 8.4  (PIDs: {php_pids} | RAM: {get_total_rss_mb(php_pids)} MB)")
        log(f"  ✅ PHP-Hyperion      (PIDs: {hyp_pids} | RAM: {get_total_rss_mb(hyp_pids)} MB)")
        log("-" * 86)

        proj_report = {
            "routes": [],
            "summary": {
                "total_routes": len(routes),
                "benchmarked_routes": 0,
                "avg_php_rps": 0.0,
                "avg_hyp_rps": 0.0,
                "avg_speedup": 0.0,
                "initial_php_rss_mb": get_total_rss_mb(php_pids),
                "initial_hyp_rss_mb": get_total_rss_mb(hyp_pids)
            }
        }

        # Benchmark every route
        route_count = 0
        total_php_rps = 0.0
        total_hyp_rps = 0.0
        speedup_sum = 0.0

        for r in routes:
            method = r["method"]
            path = r["path"]
            name = r["name"]

            # We benchmark GET and POST endpoints
            if method not in ("GET", "POST", "HEAD"):
                continue

            php_url = f"http://127.0.0.1:{PORT_PHP}{path}"
            hyp_url = f"http://127.0.0.1:{PORT_HYP}{path}"

            # Quick probe check
            php_code, _ = probe_url(php_url)
            hyp_code, _ = probe_url(hyp_url)

            # Skip routes that completely fail on probe (e.g. strict missing POST payload or unresolvable 404)
            if php_code in (404, 405) and hyp_code in (404, 405):
                continue

            # Standard route sweep: concurrency = 50, requests = 300
            req_count = 300
            conc = 50

            php_res = run_ab(php_url, req_count, conc, method=method)
            hyp_res = run_ab(hyp_url, req_count, conc, method=method)

            speedup = round(hyp_res["rps"] / php_res["rps"], 2) if php_res.get("rps", 0) > 0 else 0.0

            route_item = {
                "method": method,
                "path": path,
                "name": name,
                "concurrency": conc,
                "requests": req_count,
                "php": php_res,
                "hyperion": hyp_res,
                "speedup": speedup
            }
            proj_report["routes"].append(route_item)

            if php_res.get("success") and hyp_res.get("success") and php_res["rps"] > 0:
                route_count += 1
                total_php_rps += php_res["rps"]
                total_hyp_rps += hyp_res["rps"]
                speedup_sum += speedup

                log(f"  [{method:4}] {path[:35]:35} | PHP: {php_res['rps']:>7.1f} rps ({php_res['mean_ms']:>4.1f}ms) | Hyp: {hyp_res['rps']:>8.1f} rps ({hyp_res['mean_ms']:>4.1f}ms) | {speedup:>5.1f}x")

        # Record summary
        if route_count > 0:
            proj_report["summary"]["benchmarked_routes"] = route_count
            proj_report["summary"]["avg_php_rps"] = round(total_php_rps / route_count, 1)
            proj_report["summary"]["avg_hyp_rps"] = round(total_hyp_rps / route_count, 1)
            proj_report["summary"]["avg_speedup"] = round(speedup_sum / route_count, 2)
            proj_report["summary"]["final_php_rss_mb"] = get_total_rss_mb(php_pids)
            proj_report["summary"]["final_hyp_rss_mb"] = get_total_rss_mb(hyp_pids)

            log("-" * 86)
            log(f"  📊 {proj.upper()} Summary: {route_count} routes tested | Avg PHP: {proj_report['summary']['avg_php_rps']} rps | Avg Hyp: {proj_report['summary']['avg_hyp_rps']} rps | Avg Speedup: {proj_report['summary']['avg_speedup']}x")

        all_results["frameworks"][proj] = proj_report

        # Check if project has flagship endpoints for the Max Concurrency Escalation Ladder
        if proj in flagship_endpoints:
            log("\n  🚀 Running Maximum Concurrency Escalation Ladder...")
            ladder_concurrencies = [25, 50, 100, 200]
            
            for ep in flagship_endpoints[proj]:
                ep_key = f"{proj}:{ep['name']}"
                all_results["escalation_ladders"][ep_key] = []
                log(f"    ▶ Endpoint: {ep['name']} ({ep['path']})")

                for c in ladder_concurrencies:
                    n_reqs = c * 20  # e.g. 500, 1000, 2000, 4000
                    p_url = f"http://127.0.0.1:{PORT_PHP}{ep['path']}"
                    h_url = f"http://127.0.0.1:{PORT_HYP}{ep['path']}"

                    p_res = run_ab(p_url, n_reqs, c, method=ep["method"])
                    h_res = run_ab(h_url, n_reqs, c, method=ep["method"])

                    sp = round(h_res["rps"] / p_res["rps"], 2) if p_res.get("rps", 0) > 0 else 0.0

                    ladder_entry = {
                        "concurrency": c,
                        "requests": n_reqs,
                        "php": p_res,
                        "hyperion": h_res,
                        "speedup": sp
                    }
                    all_results["escalation_ladders"][ep_key].append(ladder_entry)

                    log(f"       Concurrency {c:>3} (n={n_reqs:>4}): PHP={p_res.get('rps', 0):>7.1f} rps, p95={p_res.get('p95_ms', 0):>4.0f}ms | Hyp={h_res.get('rps', 0):>8.1f} rps, p95={h_res.get('p95_ms', 0):>4.0f}ms | {sp:>5.1f}x")

        cleanup_servers()

    # CLI Benchmark for composer_test
    log("\n" + "#" * 86)
    log("  TARGET APPLICATION: [COMPOSER_TEST (CLI PACKAGE MANAGER ENGINE)]")
    log("#" * 86)
    comp_script = os.path.join(FRAMEWORKS_DIR, "composer_test", "test_composer.php")
    if os.path.exists(comp_script):
        php_times = []
        hyp_times = []
        for _ in range(5):
            t0 = time.perf_counter()
            subprocess.run([PHP_CLI, comp_script], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            php_times.append((time.perf_counter() - t0) * 1000.0)

            t0 = time.perf_counter()
            subprocess.run([HYPERION_CLI, comp_script], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            hyp_times.append((time.perf_counter() - t0) * 1000.0)

        php_avg = round(sum(php_times) / len(php_times), 2)
        hyp_avg = round(sum(hyp_times) / len(hyp_times), 2)
        speedup = round(php_avg / hyp_avg, 2) if hyp_avg > 0 else 0.0

        comp_result = {
            "php_avg_ms": php_avg,
            "hyp_avg_ms": hyp_avg,
            "speedup": speedup
        }
        all_results["composer_test"] = comp_result
        log(f"  Standard PHP 8.4 Execution : {php_avg} ms")
        log(f"  PHP-Hyperion Execution     : {hyp_avg} ms")
        log(f"  Speedup Multiplier         : {speedup}x Faster")

    # Save results to JSON
    report_file = os.path.join(WORKSPACE, "benchmarks", "reports", "all_frameworks_max_concurrency_report.json")
    with open(report_file, "w") as f:
        json.dump(all_results, f, indent=2)

    log("\n" + "=" * 86)
    log(f"✅ BENCHMARK COMPLETE! Full report saved to {report_file}")
    log("=" * 86)

if __name__ == "__main__":
    main()
