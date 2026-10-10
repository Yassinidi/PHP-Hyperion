#!/usr/bin/env python3
"""
⚡ PHP-HYPERION MASTER TEST RUNNER
Executes the complete testing suite across all tiers:
  1. Unit Tests (PHP Language Primitives & Micro-Benchmarks)
  2. Architecture Tests (PHPUnit 11 Test Suite)
  3. Integration Tests (Laravel Core Subsystems Battery)
  4. Defensive Security Invariants (HYP-SEC-01 to HYP-SEC-09 + Multi-Framework)
"""

import os
import sys
import time
import argparse
import subprocess

WORKSPACE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TESTS_DIR = os.path.join(WORKSPACE, "tests")

# Colors
GREEN = "\033[32m"
RED = "\033[31m"
BLUE = "\033[34m"
CYAN = "\033[36m"
YELLOW = "\033[33m"
BOLD = "\033[1m"
RESET = "\033[0m"

def print_banner(title):
    print(f"\n{BLUE}{BOLD}{'=' * 72}{RESET}")
    print(f" {CYAN}{BOLD}{title}{RESET}")
    print(f"{BLUE}{BOLD}{'=' * 72}{RESET}\n")

def run_step(name, cmd, cwd=WORKSPACE):
    print(f"{BOLD}[RUNNING]{RESET} {name}...")
    t0 = time.time()
    res = subprocess.run(cmd, cwd=cwd)
    dur = time.time() - t0
    success = (res.returncode == 0)
    status_str = f"{GREEN}PASSED ✓{RESET}" if success else f"{RED}FAILED ✗ (code {res.returncode}){RESET}"
    print(f"{BOLD}[RESULT]{RESET}  {name} -> {status_str} ({dur:.2f}s)\n")
    return success, dur

def main():
    parser = argparse.ArgumentParser(description="PHP-Hyperion Unified Test Runner")
    parser.add_argument("--unit", action="store_true", help="Run primitive micro-unit tests only")
    parser.add_argument("--phpunit", action="store_true", help="Run PHPUnit architecture suite only")
    parser.add_argument("--integration", action="store_true", help="Run Laravel integration battery only")
    parser.add_argument("--security", action="store_true", help="Run security & compatibility audit only")
    parser.add_argument("--all", action="store_true", help="Run all test suites (default)")

    args = parser.parse_args()

    # Default to all if no specific suite selected
    run_all = args.all or (not args.unit and not args.phpunit and not args.integration and not args.security)

    print_banner("🛡️ PHP-HYPERION UNIFIED ENTERPRISE TEST SUITE")
    print(f"Workspace Root: {WORKSPACE}")
    print(f"Python Runtime: {sys.version.split()[0]}")
    print(f"PHP Binary    : {subprocess.getoutput('which php')}")

    results = []

    # 1. Unit Tests
    if run_all or args.unit:
        print_banner("TIER 1: PHP PRIMITIVES & MICRO-BENCHMARKS")
        ok, dur = run_step(
            "Language Primitives & Functions",
            ["php", os.path.join(TESTS_DIR, "unit", "run_unit_tests.php")]
        )
        results.append(("Unit Tests (Primitives)", ok, dur))

    # 2. PHPUnit Suite
    if run_all or args.phpunit:
        print_banner("TIER 2: PHPUNIT 11 ENTERPRISE ARCHITECTURE")
        phpunit_bin = os.path.join(WORKSPACE, "frameworks", "laravel-full", "vendor", "phpunit", "phpunit", "phpunit")
        if not os.path.exists(phpunit_bin):
            phpunit_bin = "phpunit"
        phpunit_xml = os.path.join(TESTS_DIR, "phpunit", "phpunit.xml")
        ok, dur = run_step(
            "PHPUnit Architecture Battery",
            ["php", phpunit_bin, "-c", phpunit_xml, os.path.join(TESTS_DIR, "phpunit")]
        )
        results.append(("PHPUnit 11 Suite", ok, dur))

    # 3. Integration Tests
    if run_all or args.integration:
        print_banner("TIER 3: LARAVEL CORE SUBSYSTEMS INTEGRATION")
        ok, dur = run_step(
            "Laravel 10 Subsystems Battery",
            ["php", os.path.join(TESTS_DIR, "integration", "test_laravel_core.php")]
        )
        results.append(("Laravel Subsystems Integration", ok, dur))

    # 4. Security & Audit Suite
    if run_all or args.security:
        print_banner("TIER 4: DEFENSIVE SECURITY & MULTI-FRAMEWORK AUDIT")
        ok, dur = run_step(
            "Defensive Invariants & Multi-Framework Audit",
            ["python3", os.path.join(TESTS_DIR, "security", "security_and_compatibility_audit.py")]
        )
        results.append(("Security & Multi-Framework Audit", ok, dur))

    # Summary Report
    print_banner("🏁 MASTER TEST SUITE EXECUTION SUMMARY")
    print(f"{BOLD}{'Suite Name':<38} | {'Duration':<10} | {'Status'}{RESET}")
    print("-" * 62)

    all_passed = True
    total_time = 0.0

    for name, ok, dur in results:
        total_time += dur
        if not ok:
            all_passed = False
        status_text = f"{GREEN}PASSED ✓{RESET}" if ok else f"{RED}FAILED ✗{RESET}"
        print(f"{name:<38} | {dur:6.2f}s    | {status_text}")

    print("=" * 62)
    print(f"Total Execution Time: {total_time:.2f}s")
    if all_passed:
        print(f"{GREEN}{BOLD}🎉 ALL TEST SUITES PASSED CLEANLY (100% OPERATIONAL){RESET}\n")
        sys.exit(0)
    else:
        print(f"{RED}{BOLD}❌ ONE OR MORE TEST SUITES FAILED{RESET}\n")
        sys.exit(1)

if __name__ == "__main__":
    main()
