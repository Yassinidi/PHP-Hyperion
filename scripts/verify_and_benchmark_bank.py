#!/usr/bin/env python3
"""
⚡ BANK-NATIVE VERIFICATION & BENCHMARK SUITE
Verifies:
  1. Standard PHP 8.4 execution of all banking features (Accounts, ACID Transfers, Stats, Ledger)
  2. ApacheBench throughput & latency under Standard PHP (Read & Dynamic Write)
  3. PHP-Hyperion Core 2.0 persistent execution of all banking features
  4. ApacheBench throughput & latency under PHP-Hyperion
  5. Compares performance gains & saves comprehensive report
"""

import os
import sys
import time
import json
import re
import subprocess
import urllib.request
import urllib.error

WORKSPACE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PROJECT_DIR = os.path.join(WORKSPACE, "frameworks", "bank-native")
DOCROOT_DIR = os.path.join(PROJECT_DIR, "public")
ENTRY_FILE = os.path.join(DOCROOT_DIR, "index.php")
WORKER_PHP = os.path.join(WORKSPACE, "hyperion_worker.php")
HYPERION_CLI = os.path.join(WORKSPACE, "php-hyperion", "target", "release", "hyperion-cli")
PHP_CLI = "/opt/homebrew/bin/php" if os.path.exists("/opt/homebrew/bin/php") else "php"
AB_BIN = "/usr/sbin/ab"

PORT_PHP = 8081
PORT_HYP = 8082

def kill_ports():
    for p in (PORT_PHP, PORT_HYP):
        subprocess.run(["pkill", "-9", "-f", f"127.0.0.1:{p}"], stderr=subprocess.DEVNULL)
    time.sleep(0.5)

def wait_for_server(url, retries=35, delay=0.2):
    for _ in range(retries):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "HealthProbe/1.0"})
            with urllib.request.urlopen(req, timeout=1.5) as resp:
                if resp.status in (200, 302, 404):
                    return True
        except Exception:
            time.sleep(delay)
    return False

def http_get(url):
    req = urllib.request.Request(url, headers={"User-Agent": "BankTester/1.0"})
    with urllib.request.urlopen(req, timeout=5) as resp:
        return resp.status, resp.read().decode('utf-8')

def http_post(url, data_dict):
    data = json.dumps(data_dict).encode('utf-8')
    req = urllib.request.Request(url, data=data, headers={"Content-Type": "application/json", "User-Agent": "BankTester/1.0"})
    with urllib.request.urlopen(req, timeout=5) as resp:
        return resp.status, resp.read().decode('utf-8')

def verify_banking_features(base_url):
    print(f"   ▶ Running Bank Feature Verification against {base_url}...")
    results = {}

    # 1. Dashboard HTML
    status, body = http_get(f"{base_url}/")
    assert status == 200, f"Expected 200 on /, got {status}"
    assert "Mercury Core Banking" in body, "Dashboard title missing"
    results['web_dashboard'] = "PASS ✓"

    # 2. Health Endpoint
    status, body = http_get(f"{base_url}/health")
    assert status == 200, f"Expected 200 on /health, got {status}"
    h_data = json.loads(body)
    assert h_data.get('status') == 'ok', "Health status not ok"
    results['health_probe'] = f"PASS ✓ (Engine: {h_data.get('engine')})"

    # 3. Accounts API
    status, body = http_get(f"{base_url}/api/accounts")
    assert status == 200, f"Expected 200 on /api/accounts, got {status}"
    acc_data = json.loads(body).get('data', [])
    assert len(acc_data) >= 5, f"Expected >= 5 accounts, got {len(acc_data)}"
    results['accounts_list'] = f"PASS ✓ ({len(acc_data)} accounts)"

    # 4. Single Account API
    status, body = http_get(f"{base_url}/api/accounts/1")
    assert status == 200, f"Expected 200 on /api/accounts/1, got {status}"
    a1 = json.loads(body).get('data', {})
    assert a1.get('holder_name') == 'Alice Walker', "Account #1 holder mismatch"
    results['account_detail'] = f"PASS ✓ (Alice Walker: ${a1.get('balance'):,.2f})"

    # 5. ACID Money Transfer Execution
    alice_bal_before = float(a1['balance'])
    transfer_amount = 120.00
    transfer_payload = {
        "from_account_id": 1,
        "to_account_id": 2,
        "amount": transfer_amount,
        "note": "Verification wire"
    }
    status, body = http_post(f"{base_url}/api/transfer", transfer_payload)
    assert status in (200, 201), f"Expected 200/201 on /api/transfer, got {status}: {body}"
    tx = json.loads(body)
    assert tx.get('success') is True, f"Transfer failed: {body}"
    assert abs(tx.get('sender_remaining_balance') - (alice_bal_before - transfer_amount)) < 0.01, "Balance debit mismatch"
    results['acid_transfer'] = f"PASS ✓ (Ref: {tx.get('reference')}, Amount: ${transfer_amount:.2f})"

    # 6. Transfer Ledger API
    status, body = http_get(f"{base_url}/api/transfers")
    assert status == 200, f"Expected 200 on /api/transfers, got {status}"
    txns = json.loads(body).get('data', [])
    assert len(txns) >= 1, "Expected at least 1 transfer in ledger"
    results['ledger_history'] = f"PASS ✓ ({len(txns)} ledger entries)"

    # 7. System Stats API
    status, body = http_get(f"{base_url}/api/stats")
    assert status == 200, f"Expected 200 on /api/stats, got {status}"
    stats = json.loads(body).get('data', {})
    assert stats.get('total_accounts') >= 5, "Account count mismatch in stats"
    results['system_stats'] = f"PASS ✓ (${stats.get('total_liquidity'):,.2f} liquidity)"

    return results

def run_ab(url, n=1000, c=50, post_data=None):
    cmd = [AB_BIN, "-k"]
    tmp_post_file = None
    if post_data is not None:
        tmp_post_file = os.path.join(WORKSPACE, "scratch", f"post_{int(time.time()*1000)}.json")
        os.makedirs(os.path.dirname(tmp_post_file), exist_ok=True)
        with open(tmp_post_file, "w") as f:
            json.dump(post_data, f)
        cmd.extend(["-p", tmp_post_file, "-T", "application/json"])

    cmd.extend(["-n", str(n), "-c", str(c), url])
    res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    out = res.stdout

    if tmp_post_file and os.path.exists(tmp_post_file):
        try: os.remove(tmp_post_file)
        except OSError: pass

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
    print(f"\n{'='*78}")
    print(f" 🏦 NATIVE BANK TRANSFER: DUAL-ENGINE VERIFICATION & BENCHMARK")
    print(f"{'='*78}")
    print(f" Project Dir    : {PROJECT_DIR}")
    print(f" Standard PHP   : {PHP_CLI}")
    print(f" PHP-Hyperion   : {HYPERION_CLI}")
    print(f"{'='*78}\n")

    kill_ports()

    # -------------------------------------------------------------------------
    # PHASE 1: Standard PHP 8.4 Execution & Benchmark
    # -------------------------------------------------------------------------
    print(f"[PHASE 1] Starting Standard PHP 8.4 on port {PORT_PHP}...")
    php_env = os.environ.copy()
    php_env["PHP_CLI_SERVER_WORKERS"] = "8"
    proc_php = subprocess.Popen(
        [PHP_CLI, "-S", f"127.0.0.1:{PORT_PHP}", "-t", DOCROOT_DIR, ENTRY_FILE],
        cwd=PROJECT_DIR,
        env=php_env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL
    )

    if not wait_for_server(f"http://127.0.0.1:{PORT_PHP}/"):
        print(f"❌ Standard PHP failed to start on port {PORT_PHP}!")
        proc_php.kill()
        sys.exit(1)

    print("   ✓ Standard PHP 8.4 is live.")
    php_features = verify_banking_features(f"http://127.0.0.1:{PORT_PHP}")
    print("   ✅ All Banking features passed under Standard PHP 8.4!")
    for feat, status in php_features.items():
        print(f"      • {feat:<20}: {status}")

    # Benchmark Standard PHP
    print(f"\n   ▶ Benchmarking Standard PHP 8.4...")
    print(f"      1. High-Frequency Read : GET /api/accounts (n=1000, c=50)")
    php_bench_read = run_ab(f"http://127.0.0.1:{PORT_PHP}/api/accounts", n=1000, c=50)
    print(f"         ➔ {php_bench_read['rps']:,.1f} req/s ({php_bench_read['mean_ms']:.2f} ms)")

    print(f"      2. ACID Bank Transfer  : POST /api/transfer (n=300, c=10)")
    transfer_payload = {"from_account_id": 4, "to_account_id": 5, "amount": 10.00, "note": "Bench wire"}
    php_bench_write = run_ab(f"http://127.0.0.1:{PORT_PHP}/api/transfer", n=300, c=10, post_data=transfer_payload)
    print(f"         ➔ {php_bench_write['rps']:,.1f} req/s ({php_bench_write['mean_ms']:.2f} ms)")

    print(f"      3. Micro-Batch Group   : POST /api/transfer/batch (10 txns/batch, n=300, c=10)")
    batch_payload = {"transfers": [
        {"from_account_id": 1, "to_account_id": 2, "amount": 5.00, "note": f"Batch #{i}"}
        for i in range(10)
    ]}
    php_bench_batch = run_ab(f"http://127.0.0.1:{PORT_PHP}/api/transfer/batch", n=300, c=10, post_data=batch_payload)
    php_batch_txns_per_sec = php_bench_batch['rps'] * 10
    print(f"         ➔ {php_bench_batch['rps']:,.1f} batches/s ({php_batch_txns_per_sec:,.1f} transfers/s)")

    proc_php.kill()
    kill_ports()

    # -------------------------------------------------------------------------
    # PHASE 2: PHP-Hyperion Core 2.0 Execution & Benchmark
    # -------------------------------------------------------------------------
    print(f"\n[PHASE 2] Starting PHP-Hyperion Core 2.0 on port {PORT_HYP}...")
    hyp_env = os.environ.copy()
    hyp_env["PHP_CLI_SERVER_WORKERS"] = "16"
    hyp_env["HYPERION_WORKER_PATH"] = WORKER_PHP
    hyp_log_path = os.path.join(PROJECT_DIR, "hyperion.log")
    hyp_log = open(hyp_log_path, "w")

    proc_hyp = subprocess.Popen(
        [HYPERION_CLI, "-S", f"127.0.0.1:{PORT_HYP}", "-t", DOCROOT_DIR, WORKER_PHP],
        cwd=PROJECT_DIR,
        env=hyp_env,
        stdout=hyp_log,
        stderr=hyp_log
    )

    if not wait_for_server(f"http://127.0.0.1:{PORT_HYP}/"):
        print(f"❌ PHP-Hyperion failed to start on port {PORT_HYP}!")
        hyp_log.flush()
        with open(hyp_log_path, "r") as f:
            print(f.read())
        proc_hyp.kill()
        sys.exit(1)

    print("   ✓ PHP-Hyperion is live.")
    hyp_features = verify_banking_features(f"http://127.0.0.1:{PORT_HYP}")
    print("   ✅ All Banking features passed under PHP-Hyperion!")
    for feat, status in hyp_features.items():
        print(f"      • {feat:<20}: {status}")

    # Benchmark PHP-Hyperion
    print(f"\n   ▶ Benchmarking PHP-Hyperion Core 2.0...")
    print(f"      1. High-Frequency Read : GET /api/accounts (n=1000, c=50)")
    hyp_bench_read = run_ab(f"http://127.0.0.1:{PORT_HYP}/api/accounts", n=1000, c=50)
    print(f"         ➔ {hyp_bench_read['rps']:,.1f} req/s ({hyp_bench_read['mean_ms']:.2f} ms)")

    print(f"      2. ACID Bank Transfer  : POST /api/transfer (n=300, c=10)")
    hyp_bench_write = run_ab(f"http://127.0.0.1:{PORT_HYP}/api/transfer", n=300, c=10, post_data=transfer_payload)
    print(f"         ➔ {hyp_bench_write['rps']:,.1f} req/s ({hyp_bench_write['mean_ms']:.2f} ms)")

    print(f"      3. Micro-Batch Group   : POST /api/transfer/batch (10 txns/batch, n=300, c=10)")
    hyp_bench_batch = run_ab(f"http://127.0.0.1:{PORT_HYP}/api/transfer/batch", n=300, c=10, post_data=batch_payload)
    hyp_batch_txns_per_sec = hyp_bench_batch['rps'] * 10
    print(f"         ➔ {hyp_bench_batch['rps']:,.1f} batches/s ({hyp_batch_txns_per_sec:,.1f} transfers/s)")

    proc_hyp.kill()
    kill_ports()

    # -------------------------------------------------------------------------
    # PHASE 3: Comprehensive Summary & Gain Analysis
    # -------------------------------------------------------------------------
    gain_read = round(hyp_bench_read['rps'] / max(php_bench_read['rps'], 0.1), 2)
    gain_write = round(hyp_bench_write['rps'] / max(php_bench_write['rps'], 0.1), 2)
    gain_batch = round(hyp_batch_txns_per_sec / max(php_batch_txns_per_sec, 0.1), 2)

    report_data = {
        "project": "bank-native",
        "description": "High-Integrity Bank Money Transfer with SQLite WAL & ACID Transactions",
        "read_accounts": {
            "standard_php_rps": php_bench_read['rps'],
            "standard_php_ms": php_bench_read['mean_ms'],
            "hyperion_rps": hyp_bench_read['rps'],
            "hyperion_ms": hyp_bench_read['mean_ms'],
            "gain": f"{gain_read}x"
        },
        "acid_transfer_writes": {
            "standard_php_rps": php_bench_write['rps'],
            "standard_php_ms": php_bench_write['mean_ms'],
            "hyperion_rps": hyp_bench_write['rps'],
            "hyperion_ms": hyp_bench_write['mean_ms'],
            "gain": f"{gain_write}x"
        },
        "microbatch_transfers": {
            "standard_php_batches_per_sec": php_bench_batch['rps'],
            "standard_php_transfers_per_sec": php_batch_txns_per_sec,
            "hyperion_batches_per_sec": hyp_bench_batch['rps'],
            "hyperion_transfers_per_sec": hyp_batch_txns_per_sec,
            "gain": f"{gain_batch}x"
        },
        "features_verified": hyp_features
    }

    report_file = os.path.join(PROJECT_DIR, "benchmark_report.json")
    with open(report_file, "w") as f:
        json.dump(report_data, f, indent=2)

    print(f"\n{'='*78}")
    print(f" 🏁 FINAL BENCHMARK MATRIX: BANK-NATIVE MONEY TRANSFER")
    print(f"{'='*78}")
    print(f" Workload / Route       | Standard PHP 8.4     | Hyperion Core 2.0     | Gain")
    print(f" -----------------------+----------------------+-----------------------+--------")
    print(f" Read: /api/accounts    | {php_bench_read['rps']:>8,.1f} req/s ({php_bench_read['mean_ms']:>4.2f}ms) | {hyp_bench_read['rps']:>8,.1f} req/s ({hyp_bench_read['mean_ms']:>4.2f}ms) | {gain_read:>5.2f}x")
    print(f" Single ACID Transfer   | {php_bench_write['rps']:>8,.1f} req/s ({php_bench_write['mean_ms']:>4.2f}ms) | {hyp_bench_write['rps']:>8,.1f} req/s ({hyp_bench_write['mean_ms']:>4.2f}ms) | {gain_write:>5.2f}x")
    print(f" Micro-Batch Group ACID | {php_batch_txns_per_sec:>8,.1f} txns/s ({php_bench_batch['mean_ms']:>4.2f}ms) | {hyp_batch_txns_per_sec:>8,.1f} txns/s ({hyp_bench_batch['mean_ms']:>4.2f}ms) | {gain_batch:>5.2f}x")
    print(f"{'='*78}")
    print(f" Report Saved : {report_file}")
    print(f"{'='*78}\n")
    print(f"{'='*78}")
    print(f" Report Saved : {report_file}")
    print(f"{'='*78}\n")

if __name__ == "__main__":
    main()
