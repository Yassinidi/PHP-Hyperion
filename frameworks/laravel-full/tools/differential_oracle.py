#!/usr/bin/env python3
import json
import sys
import time
import urllib.request
import urllib.error
import urllib.parse
from typing import Dict, Any, Tuple, Optional

ZEND_URL = "http://127.0.0.1:8001"
HYPERION_URL = "http://127.0.0.1:8000"

def send_request(base_url: str, method: str, path: str, headers: Optional[Dict[str, str]] = None, data: Optional[bytes] = None) -> Tuple[int, Dict[str, str], bytes]:
    url = f"{base_url}{path}"
    req_headers = {"User-Agent": "DifferentialOracle/1.0"}
    if headers:
        req_headers.update(headers)
    req = urllib.request.Request(url, data=data, headers=req_headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            status = resp.status
            resp_headers = {k.lower(): v for k, v in resp.headers.items()}
            body = resp.read()
            return status, resp_headers, body
    except urllib.error.HTTPError as e:
        resp_headers = {k.lower(): v for k, v in e.headers.items()}
        body = e.read()
        return e.code, resp_headers, body
    except Exception as e:
        return -1, {}, str(e).encode()

def compare_responses(test_name: str, zend_res: Tuple[int, Dict[str, str], bytes], hyp_res: Tuple[int, Dict[str, str], bytes]) -> Tuple[bool, str]:
    z_status, z_headers, z_body = zend_res
    h_status, h_headers, h_body = hyp_res

    if h_status == -1:
        return False, f"Hyperion connection failed: {h_body.decode(errors='replace')}"
    if z_status == -1:
        return False, f"Zend connection failed: {z_body.decode(errors='replace')}"

    if z_status != h_status:
        return False, f"Status code mismatch: Zend={z_status}, Hyperion={h_status}"

    z_ct = z_headers.get("content-type", "").split(";")[0].strip()
    h_ct = h_headers.get("content-type", "").split(";")[0].strip()
    if z_ct != h_ct:
        return False, f"Content-Type mismatch: Zend='{z_ct}', Hyperion='{h_ct}'"

    # Compare content
    if "json" in z_ct:
        try:
            z_json = json.loads(z_body.decode("utf-8"))
            h_json = json.loads(h_body.decode("utf-8"))
            # Compare json structure / contents
            if isinstance(z_json, dict) and isinstance(h_json, dict):
                # Ignore volatile keys like timestamp/trace if any
                z_keys = set(z_json.keys())
                h_keys = set(h_json.keys())
                if z_keys != h_keys:
                    return False, f"JSON key mismatch: Zend keys={sorted(z_keys)}, Hyperion keys={sorted(h_keys)}"
            elif isinstance(z_json, list) and isinstance(h_json, list):
                if len(z_json) != len(h_json):
                    return False, f"JSON array length mismatch: Zend={len(z_json)}, Hyperion={len(h_json)}"
        except Exception as e:
            return False, f"JSON parsing failed: {e}"
    else:
        # Non-JSON comparison (HTML/Text)
        if len(z_body) > 0 and len(h_body) == 0:
            return False, "Hyperion returned empty body while Zend returned content"

    return True, "OK"

def main():
    print("================================================================")
    print("Dual-Engine Differential Oracle: Zend PHP 8.4 vs PHP-Hyperion")
    print("================================================================")

    # 1. Connectivity health checks
    z_stat, _, _ = send_request(ZEND_URL, "GET", "/")
    h_stat, _, _ = send_request(HYPERION_URL, "GET", "/")

    if z_stat == -1:
        print(f"❌ Error: Zend PHP 8.4 engine is not reachable at {ZEND_URL}")
        print("Please start Zend: php artisan serve --port=8001")
        sys.exit(1)
    if h_stat == -1:
        print(f"❌ Error: PHP-Hyperion engine is not reachable at {HYPERION_URL}")
        print("Please start Hyperion: hyperion-cli --server 127.0.0.1:8000 public/index.php")
        sys.exit(1)

    print(f"✔ Zend PHP 8.4 reachable at {ZEND_URL} (Status {z_stat})")
    print(f"✔ PHP-Hyperion reachable at {HYPERION_URL} (Status {h_stat})")
    print()

    # Test suite endpoints
    endpoints = [
        # (Name, Method, Path, Headers, Body)
        ("Frontend Homepage (Blade)", "GET", "/", {}, None),
        ("Login Page (Blade)", "GET", "/login", {}, None),
        ("Register Page (Blade)", "GET", "/register", {}, None),
        ("Swagger UI Endpoint", "GET", "/api/documentation", {}, None),
        ("OpenAPI JSON Spec", "GET", "/docs", {"Accept": "application/json"}, None),
        
        # Public JSON APIs
        ("API Houses Index", "GET", "/api/houses", {"Accept": "application/json"}, None),
        ("API Categories Index", "GET", "/api/categories", {"Accept": "application/json"}, None),
        ("API Cities Index", "GET", "/api/cities", {"Accept": "application/json"}, None),
        ("API Banks Index", "GET", "/api/banks", {"Accept": "application/json"}, None),
        ("API Facilities Index", "GET", "/api/facilities", {"Accept": "application/json"}, None),
        
        # Parameterized APIs
        ("API Bank Details (id=1)", "GET", "/api/banks/1", {"Accept": "application/json"}, None),
        ("API Bank Details Nonexistent (id=9999)", "GET", "/api/banks/9999", {"Accept": "application/json"}, None),
        
        # Validation rules & Form Requests
        ("Mortgage Calculate (Empty Payload -> 422)", "POST", "/api/mortgages/calculate", 
         {"Accept": "application/json", "Content-Type": "application/json"}, b"{}"),
        ("Mortgage Calculate (Valid Payload -> 200)", "POST", "/api/mortgages/calculate",
         {"Accept": "application/json", "Content-Type": "application/json"}, 
         b'{"house_price": 500000000, "down_payment": 100000000, "interest_rate": 5.5, "duration_years": 15}'),
        ("Mortgage Calculate (Negative Price -> 422)", "POST", "/api/mortgages/calculate",
         {"Accept": "application/json", "Content-Type": "application/json"}, 
         b'{"house_price": -500, "down_payment": 100, "interest_rate": 5, "duration_years": 10}'),
        
        # Auth protected endpoints (assert 401 on unauthenticated)
        ("API Current User (Unauthorized -> 401)", "GET", "/api/user", {"Accept": "application/json"}, None),
        ("API Mortgages Index (Unauthorized -> 401)", "GET", "/api/mortgages", {"Accept": "application/json"}, None),
        ("API Mortgages Store (Unauthorized -> 401)", "POST", "/api/mortgages", 
         {"Accept": "application/json", "Content-Type": "application/json"}, b'{"house_id": 1}'),
        ("API Payment Breakdown (Unauthorized -> 401)", "GET", "/api/mortgages/1/payment-breakdown", {"Accept": "application/json"}, None),
        ("API Mortgage Pay (Unauthorized -> 401)", "POST", "/api/mortgages/1/pay", 
         {"Accept": "application/json", "Content-Type": "application/json"}, b'{"amount": 5000000}'),
        ("API Installments Index (Unauthorized -> 401)", "GET", "/api/mortgages/1/installments", {"Accept": "application/json"}, None),
        ("API Installment Show (Unauthorized -> 401)", "GET", "/api/installments/1", {"Accept": "application/json"}, None),

        # Routing edge cases
        ("Nonexistent Route 404 (Web)", "GET", "/nonexistent-page-xyz-404", {}, None),
        ("Nonexistent Route 404 (JSON API)", "GET", "/api/nonexistent-xyz-404", {"Accept": "application/json"}, None),
        ("Method Not Allowed 405", "POST", "/api/houses", {"Accept": "application/json"}, None),
    ]

    passed = 0
    failed = 0

    print(f"Executing {len(endpoints)} differential oracle tests side-by-side...")
    print("-" * 64)

    for name, method, path, headers, body in endpoints:
        z_res = send_request(ZEND_URL, method, path, headers, body)
        h_res = send_request(HYPERION_URL, method, path, headers, body)

        ok, msg = compare_responses(name, z_res, h_res)
        if ok:
            print(f"  ✔ [PASS] {name} ({method} {path}) -> Status {h_res[0]}")
            passed += 1
        else:
            print(f"  ❌ [FAIL] {name} ({method} {path}) -> {msg}")
            failed += 1

    print("-" * 64)
    print(f"Differential Oracle Results: {passed} passed, {failed} failed (Total: {len(endpoints)})")
    if failed == 0:
        print("🎉 100% DIFFERENTIAL PARITY ACHIEVED BETWEEN ZEND PHP 8.4 AND PHP-HYPERION!")
        sys.exit(0)
    else:
        print(f"⚠️ {failed} regressions identified between Zend and Hyperion.")
        sys.exit(1)

if __name__ == "__main__":
    main()
