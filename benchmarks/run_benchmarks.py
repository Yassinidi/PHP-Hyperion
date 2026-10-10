#!/usr/bin/env python3
"""
⚡ PHP-HYPERION MASTER BENCHMARK RUNNER
Unified launcher for all performance benchmark suites, latency probes, and stress tests.
"""

import os
import sys
import time
import json
import argparse
import subprocess

BENCH_DIR = os.path.dirname(os.path.abspath(__file__))
WORKSPACE = os.path.dirname(BENCH_DIR)

# Terminal Colors
GREEN = "\033[32m"
RED = "\033[31m"
BLUE = "\033[34m"
CYAN = "\033[36m"
YELLOW = "\033[33m"
BOLD = "\033[1m"
RESET = "\033[0m"

SUITES = {
    "1": ("Laravel 12 Complex Enterprise", os.path.join(BENCH_DIR, "suites", "laravel_complex.py")),
    "2": ("Multi-Framework Concurrency (Laravel, Symfony, Slim)", os.path.join(BENCH_DIR, "suites", "frameworks_concurrency.py")),
    "3": ("Eloquent ORM & Database CRUD", os.path.join(BENCH_DIR, "suites", "laravel_crud.py")),
    "4": ("Vanilla PHP REST High-Speed", os.path.join(BENCH_DIR, "suites", "vanilla_php.py")),
    "5": ("Dual-Engine Latency & RAM Profile", os.path.join(BENCH_DIR, "suites", "dual_engine.py")),
    "6": ("RAM Stability & 250k Request Soak Test", os.path.join(BENCH_DIR, "stress", "stress_concurrency_and_ram.py")),
    "7": ("Engine Optimization Phases Verifier", os.path.join(BENCH_DIR, "stress", "verify_optimizations.py")),
}

def print_banner(title):
    print(f"\n{BLUE}{BOLD}{'=' * 74}{RESET}")
    print(f" {CYAN}{BOLD}{title}{RESET}")
    print(f"{BLUE}{BOLD}{'=' * 74}{RESET}\n")

def list_reports():
    reports_dir = os.path.join(BENCH_DIR, "reports")
    print_banner("📊 SAVED BENCHMARK JSON REPORTS")
    if not os.path.exists(reports_dir):
        print("No reports directory found.")
        return

    files = sorted(os.listdir(reports_dir))
    print(f"{BOLD}{'Report File':<45} | {'Size':<10} | {'Status'}{RESET}")
    print("-" * 72)
    for f in files:
        if f.endswith(".json"):
            fp = os.path.join(reports_dir, f)
            sz = os.path.getsize(fp)
            sz_str = f"{sz / 1024:.1f} KB" if sz > 1024 else f"{sz} B"
            print(f"{f:<45} | {sz_str:<10} | {GREEN}Available ✓{RESET}")
    print("=" * 72 + "\n")

def run_suite(name, path):
    print_banner(f"LAUNCHING BENCHMARK: {name}")
    print(f"Target Script: {path}\n")
    res = subprocess.run([sys.executable, path])
    if res.returncode == 0:
        print(f"\n{GREEN}{BOLD}✓ Benchmark completed successfully.{RESET}")
    else:
        print(f"\n{RED}{BOLD}✗ Benchmark exited with return code {res.returncode}.{RESET}")
    return res.returncode

def interactive_menu():
    print_banner("⚡ PHP-HYPERION BENCHMARK LAUNCHER")
    print("Select a benchmark suite to run:\n")
    for k, (name, _) in SUITES.items():
        print(f"  {BOLD}[{k}]{RESET} {name}")
    print(f"  {BOLD}[R]{RESET} View Saved JSON Reports")
    print(f"  {BOLD}[Q]{RESET} Quit\n")

    choice = input("Enter choice [1-7, R, Q]: ").strip()
    if choice.upper() == "Q":
        sys.exit(0)
    elif choice.upper() == "R":
        list_reports()
    elif choice in SUITES:
        name, path = SUITES[choice]
        run_suite(name, path)
    else:
        print(f"{RED}Invalid option selected.{RESET}")

def main():
    parser = argparse.ArgumentParser(description="PHP-Hyperion Benchmark Launcher")
    parser.add_argument("--laravel", action="store_true", help="Run Laravel 12 Complex Enterprise benchmark")
    parser.add_argument("--frameworks", action="store_true", help="Run Multi-Framework Concurrency benchmark")
    parser.add_argument("--crud", action="store_true", help="Run Eloquent ORM & CRUD benchmark")
    parser.add_argument("--vanilla", action="store_true", help="Run Vanilla PHP benchmark")
    parser.add_argument("--dual", action="store_true", help="Run Dual-Engine profile benchmark")
    parser.add_argument("--stress", action="store_true", help="Run Concurrency & RAM Stress benchmark")
    parser.add_argument("--optimizations", action="store_true", help="Run Optimization phases verifier")
    parser.add_argument("--reports", action="store_true", help="List all generated benchmark reports")

    args = parser.parse_args()

    if args.reports:
        list_reports()
        return

    if args.laravel:
        run_suite(SUITES["1"][0], SUITES["1"][1])
    elif args.frameworks:
        run_suite(SUITES["2"][0], SUITES["2"][1])
    elif args.crud:
        run_suite(SUITES["3"][0], SUITES["3"][1])
    elif args.vanilla:
        run_suite(SUITES["4"][0], SUITES["4"][1])
    elif args.dual:
        run_suite(SUITES["5"][0], SUITES["5"][1])
    elif args.stress:
        run_suite(SUITES["6"][0], SUITES["6"][1])
    elif args.optimizations:
        run_suite(SUITES["7"][0], SUITES["7"][1])
    else:
        interactive_menu()

if __name__ == "__main__":
    main()
