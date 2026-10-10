#!/usr/bin/env python3
import subprocess, time, urllib.request, re, os, json

def find_workspace_root():
    d = os.path.dirname(os.path.abspath(__file__))
    while d and d != os.path.dirname(d):
        if os.path.exists(os.path.join(d, "php-hyperion")) and os.path.exists(os.path.join(d, "frameworks")):
            return d
        d = os.path.dirname(d)
    return os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))

WORKSPACE = find_workspace_root()
PHP_CLI = "/opt/homebrew/bin/php" if os.path.exists("/opt/homebrew/bin/php") else "php"
HYPERION_CLI = os.path.join(WORKSPACE, "php-hyperion", "target", "release", "hyperion-cli")
BASE_DIR = os.path.join(WORKSPACE, "frameworks")
PORT_PHP = 8001
PORT_HYP = 8000

targets = [
    {"name": "normal-php-app", "dir": "normal-php-app", "docroot": "public", "index": "public/index.php", "c": 100, "n": 10000},
    {"name": "slim", "dir": "slim", "docroot": "public", "index": "public/index.php", "c": 100, "n": 10000},
    {"name": "symfony", "dir": "symfony", "docroot": "public", "index": "public/index.php", "c": 100, "n": 10000},
    {"name": "laravel-complex", "dir": "laravel-complex", "docroot": "public", "index": "public/index.php", "c": 100, "n": 10000},
    {"name": "laravel-app", "dir": "laravel-app", "docroot": "public", "index": "public/index.php", "c": 50, "n": 2000},
]

def kill_ports():
    subprocess.run(["pkill", "-9", "-f", f"127.0.0.1:{PORT_PHP}"], stderr=subprocess.DEVNULL)
    subprocess.run(["pkill", "-9", "-f", f"127.0.0.1:{PORT_HYP}"], stderr=subprocess.DEVNULL)
    time.sleep(0.5)

def wait_server(url):
    for _ in range(30):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "Probe"})
            with urllib.request.urlopen(req, timeout=1) as resp:
                if resp.status == 200:
                    return True
        except:
            pass
        time.sleep(0.15)
    return False

def benchmark_ab(port, concurrency, total_requests):
    # Warmup
    try:
        req = urllib.request.Request(f"http://127.0.0.1:{port}/", headers={"User-Agent": "Probe"})
        with urllib.request.urlopen(req, timeout=2) as resp:
            resp.read()
    except:
        pass
    time.sleep(0.2)

    cmd = ["/usr/sbin/ab", "-k", "-c", str(concurrency), "-n", str(total_requests), f"http://127.0.0.1:{port}/"]
    res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    out = res.stdout

    rps_match = re.search(r"Requests per second:\s+([\d.]+)", out)
    mean_ms_match = re.search(r"Time per request:\s+([\d.]+)\s+\[ms\]\s+\(mean\)", out)
    p50_match = re.search(r"50%\s+(\d+)", out)
    p95_match = re.search(r"95%\s+(\d+)", out)
    complete_match = re.search(r"Complete requests:\s+(\d+)", out)
    time_taken_match = re.search(r"Time taken for tests:\s+([\d.]+)", out)

    if not rps_match:
        print(f"      [DEBUG] ab failed! Output snippet: {out[:300]}")
        return {"success": 0, "total": total_requests, "rps": 0, "mean_ms": 0, "p50_ms": 0, "p95_ms": 0, "wall_s": 0}

    rps = float(rps_match.group(1))
    mean_ms = float(mean_ms_match.group(1)) if mean_ms_match else 0.0
    p50 = int(p50_match.group(1)) if p50_match else 0
    p95 = int(p95_match.group(1)) if p95_match else 0
    completed = int(complete_match.group(1)) if complete_match else 0
    wall_s = float(time_taken_match.group(1)) if time_taken_match else 0.0

    return {
        "success": completed,
        "total": total_requests,
        "rps": round(rps, 2),
        "mean_ms": round(mean_ms, 2),
        "p50_ms": p50,
        "p95_ms": p95,
        "wall_s": round(wall_s, 3)
    }

print("=========================================================================", flush=True)
print("⚡ HYPERION HIGH-CONCURRENCY VERIFICATION SUITE (Target >= 50K r/s)", flush=True)
print("=========================================================================", flush=True)

final_results = {}

for t in targets:
    proj_dir = os.path.join(BASE_DIR, t["dir"])
    print(f"\n▶ Benchmarking target: {t['name']} (dir: {t['dir']})...", flush=True)
    kill_ports()

    # 1. Benchmark Standard PHP
    php_env = os.environ.copy()
    php_env["PHP_CLI_SERVER_WORKERS"] = "8"
    php_proc = subprocess.Popen(
        [PHP_CLI, "-S", f"127.0.0.1:{PORT_PHP}", "-t", t["docroot"], t["index"]],
        cwd=proj_dir, env=php_env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
    )
    if not wait_server(f"http://127.0.0.1:{PORT_PHP}/"):
        print(f"❌ PHP failed to start for {t['name']}", flush=True)
        php_proc.kill()
        continue

    print(f"   Running standard PHP benchmark (c=10, n=500)...", flush=True)
    res_php = benchmark_ab(PORT_PHP, 10, 500)
    php_proc.terminate()
    php_proc.kill()
    kill_ports()
    print(f"   PHP done: {res_php['rps']} req/s", flush=True)

    # 2. Benchmark PHP-Hyperion
    hyp_env = os.environ.copy()
    hyp_env["PHP_CLI_SERVER_WORKERS"] = "8"
    hyp_env["HYPERION_REACTORS"] = "4"
    hyp_cmd = [HYPERION_CLI, "-S", f"127.0.0.1:{PORT_HYP}"]
    if os.path.exists(os.path.join(proj_dir, t["docroot"])):
        hyp_cmd.extend(["-t", t["docroot"]])
    hyp_proc = subprocess.Popen(
        hyp_cmd,
        cwd=proj_dir, env=hyp_env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
    )
    if not wait_server(f"http://127.0.0.1:{PORT_HYP}/"):
        print(f"❌ Hyperion failed to start for {t['name']}", flush=True)
        hyp_proc.kill()
        continue

    print(f"   Running PHP-Hyperion benchmark (c={t['c']}, n={t['n']})...", flush=True)
    res_hyp = benchmark_ab(PORT_HYP, t["c"], t["n"])
    hyp_proc.terminate()
    hyp_proc.kill()
    kill_ports()
    print(f"   Hyperion done: {res_hyp['rps']} req/s", flush=True)

    speedup = round(res_hyp["rps"] / max(res_php["rps"], 0.1), 2)
    final_results[t["name"]] = {
        "php": res_php,
        "hyperion": res_hyp,
        "speedup": speedup
    }

    print(f"\n🔹 {t['name']} (Hyperion c={t['c']}, n={t['n']} vs PHP c=10):", flush=True)
    print(f"   Standard PHP 8.4 : {res_php['rps']} req/s | Mean: {res_php['mean_ms']} ms | p95: {res_php['p95_ms']} ms", flush=True)
    print(f"   PHP-Hyperion     : {res_hyp['rps']} req/s | Mean: {res_hyp['mean_ms']} ms | p95: {res_hyp['p95_ms']} ms", flush=True)
    print(f"   🚀 Speedup       : {speedup}x", flush=True)

report_path = os.path.join(WORKSPACE, "benchmarks", "reports", "optimization_verification_report.json")
with open(report_path, "w") as f:
    json.dump(final_results, f, indent=2)

print(f"\nSaved verification report to {report_path}", flush=True)

