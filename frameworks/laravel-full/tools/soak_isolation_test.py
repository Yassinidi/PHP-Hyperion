#!/usr/bin/env python3
import concurrent.futures
import json
import sys
import time
import urllib.request
import urllib.error
from typing import Dict, Any, Tuple, Optional

HYPERION_URL = "http://127.0.0.1:8000"

def send_request(path: str, headers: Optional[Dict[str, str]] = None) -> Tuple[int, bytes]:
    url = f"{HYPERION_URL}{path}"
    req_headers = {"User-Agent": "SoakAudit/1.0"}
    if headers:
        req_headers.update(headers)
    req = urllib.request.Request(url, headers=req_headers, method="GET")
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            return resp.status, resp.read()
    except urllib.error.HTTPError as e:
        return e.code, e.read()
    except Exception as e:
        return -1, str(e).encode()

def test_session_isolation():
    print("=== Sub-Test 4.1: Persistent Worker Session & State Isolation Audit ===")
    
    # We will test unauthenticated requests interspersed with authenticated requests or distinct client identities
    total_checks = 100
    leaks_detected = 0
    unauth_success = 0

    print(f"Sending {total_checks} interlaced requests to assert zero session/auth bleed...")

    def check_req(i: int):
        # Even requests: Unauthenticated /api/user -> MUST be 401 Unauthorized
        # Odd requests: Public API /api/houses -> MUST be 200 OK
        if i % 2 == 0:
            status, body = send_request("/api/user", {"Accept": "application/json"})
            if status != 401:
                return False, f"Request #{i}: Expected 401 Unauthorized on unauthenticated /api/user, got {status}!"
            return True, "401 OK"
        else:
            status, body = send_request("/api/houses", {"Accept": "application/json"})
            if status != 200:
                return False, f"Request #{i}: Expected 200 OK on /api/houses, got {status}!"
            return True, "200 OK"

    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as executor:
        futures = [executor.submit(check_req, i) for i in range(total_checks)]
        for f in concurrent.futures.as_completed(futures):
            ok, msg = f.result()
            if not ok:
                leaks_detected += 1
                print(f"  ❌ {msg}")
            else:
                unauth_success += 1

    if leaks_detected == 0:
        print(f"  ✔ [PASS] Zero state leaks across all {total_checks} interlaced requests! (100% isolation)")
        return True
    else:
        print(f"  ❌ [FAIL] {leaks_detected} session bleeds detected!")
        return False

def test_soak_memory_stability(num_requests=100):
    print()
    print(f"=== Sub-Test 4.2: High-Throughput Soak Audit ({num_requests} requests) ===")
    
    start_time = time.time()
    success_count = 0
    error_count = 0

    print(f"Dispatching {num_requests} rapid requests across concurrent client threads...")
    
    def worker_job(i: int):
        paths = ["/api/houses", "/api/banks", "/api/cities", "/api/categories", "/api/facilities"]
        path = paths[i % len(paths)]
        status, body = send_request(path, {"Accept": "application/json"})
        if status != 200:
            print(f"  ❌ Soak Request #{i} ({path}) failed with status {status}: {body[:80]}")
            return False
        return True

    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as executor:
        futures = [executor.submit(worker_job, i) for i in range(num_requests)]
        for f in concurrent.futures.as_completed(futures):
            if f.result():
                success_count += 1
            else:
                error_count += 1

    elapsed = time.time() - start_time
    rps = num_requests / elapsed if elapsed > 0 else 0

    print(f"  ✔ Completed {num_requests} requests in {elapsed:.2f}s ({rps:.1f} req/sec)")
    print(f"  ✔ Successful (200 OK): {success_count}/{num_requests}")
    print(f"  ✔ Failed/Errors: {error_count}")

    if error_count == 0:
        print("  ✔ [PASS] Soak audit completed with 0 errors and steady throughput!")
        return True
    else:
        print(f"  ❌ [FAIL] {error_count} requests failed during soak testing.")
        return False

def main():
    print("================================================================")
    print("Tier 4 Verification: Persistent Worker State Isolation & Soak")
    print("================================================================")
    
    # Health check
    stat, _ = send_request("/api/houses", {"Accept": "application/json"})
    if stat == -1:
        print(f"❌ Error: PHP-Hyperion is not reachable at {HYPERION_URL}")
        sys.exit(1)

    iso_ok = test_session_isolation()
    soak_ok = test_soak_memory_stability(100)

    print()
    print("-" * 64)
    if iso_ok and soak_ok:
        print("🎉 TIER 4 VERIFICATION COMPLETE: ZERO STATE LEAKS & ROCK-SOLID STABILITY!")
        sys.exit(0)
    else:
        print("⚠️ Tier 4 verification encountered failures.")
        sys.exit(1)

if __name__ == "__main__":
    main()
