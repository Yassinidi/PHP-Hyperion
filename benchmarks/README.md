# ⚡ PHP-Hyperion Benchmark Suites & Performance Reports

Automated benchmark suites, latency probes, concurrency stress tests, and machine-readable JSON reports comparing **PHP-Hyperion** against **Standard PHP 8.4 (Zend Engine / PHP-FPM)**.

---

## 📁 Directory Layout

```
benchmarks/
├── README.md                                # Benchmark catalog & documentation (this file)
├── run_benchmarks.py                        # Unified interactive benchmark CLI runner
│
├── suites/                                  # High-Throughput Benchmark Test Suites
│   ├── laravel_complex.py                   # Primary Head-to-Head Suite (Laravel 12.32 Complex)
│   ├── frameworks_concurrency.py            # Multi-Framework Concurrency Suite (Laravel, Symfony, Slim)
│   ├── laravel_crud.py                      # Eloquent ORM & Database CRUD Benchmarks
│   ├── vanilla_php.py                       # Native / Plain PHP High-Speed REST Benchmarks
│   ├── dual_engine.py                       # Dual-Engine Head-to-Head Latency & RAM Profile
│   └── full_suite.py                        # Complete Throughput & Data Transfer Suite
│
├── stress/                                  # Extreme Concurrency & Soak Testing
│   ├── stress_concurrency_and_ram.py        # RAM Stability, Leak Profiling & Concurrency Ladder
│   └── verify_optimizations.py              # Micro-benchmarks across all engine optimization phases
│
├── k6/                                      # Cloud Load Generator Scripts
│   └── load_test.js                         # k6 constant-VU scenario script
│
└── reports/                                 # Machine-Readable JSON Output Reports
    ├── full_benchmark_report.json              # Verified Laravel 12 complex enterprise results
    ├── all_frameworks_max_concurrency_report.json # Multi-framework concurrency breakdown
    ├── concurrency_stress_report.json          # RAM timeline and concurrency ladder results
    ├── benchmark_normal_php_results.json       # Plain PHP REST endpoints throughput
    ├── benchmark_results.json                  # Dual-engine comparison results
    └── optimization_verification_report.json   # Multi-phase optimization metrics
```

---

## 🚀 Running the Benchmarks

### 1. Unified Master Benchmark Launcher
Run benchmarks interactively or pass flags for specific suites:
```bash
# Interactive menu
python3 benchmarks/run_benchmarks.py

# Laravel 12 Enterprise Head-to-Head Suite
python3 benchmarks/run_benchmarks.py --laravel

# Multi-Framework Concurrency Matrix
python3 benchmarks/run_benchmarks.py --frameworks

# Database CRUD & Eloquent ORM
python3 benchmarks/run_benchmarks.py --crud

# Vanilla PHP REST Benchmarks
python3 benchmarks/run_benchmarks.py --vanilla

# RAM Stability & Concurrency Soak Test
python3 benchmarks/run_benchmarks.py --stress

# List all saved JSON reports
python3 benchmarks/run_benchmarks.py --reports
```

### 2. Running Individual Suites Directly
```bash
# Laravel 12 Enterprise Head-to-Head
python3 benchmarks/suites/laravel_complex.py

# Multi-Framework Concurrency Matrix
python3 benchmarks/suites/frameworks_concurrency.py

# Concurrency Ladder & RAM Soak Test
python3 benchmarks/stress/stress_concurrency_and_ram.py
```

### 3. Load Testing with k6
```bash
VUS=50 DURATION=30s TARGET_URL=http://127.0.0.1:8000/health k6 run benchmarks/k6/load_test.js
```

---

## 📊 Summary of Latest Results

| Workload / Route | Standard PHP 8.4 | PHP-Hyperion | 🚀 Speedup | Status |
| :--- | :---: | :---: | :---: | :---: |
| **Framework Ingress** (`/health`) | 333.4 req/s | **70,791.4 req/s** | **212.3x Faster** | ✅ Verified |
| **SQL HAVING Aggregations** | 344.6 req/s | **5,655.1 req/s** | **16.4x Faster** | ✅ Verified |
| **Subquery Joins** (`joinSub`) | 352.8 req/s | **4,567.2 req/s** | **12.9x Faster** | ✅ Verified |
| **Window Functions** (`ROW_NUMBER`) | 337.6 req/s | **3,514.1 req/s** | **10.4x Faster** | ✅ Verified |
| **Complex Dashboard View** (Blade) | 279.7 req/s | **1,243.2 req/s** | **4.4x Faster** | ✅ Verified |
| **Peak Concurrency ($c=200$)** | 310 req/s | **209,234.2 req/s** | **674.9x Faster** | ✅ Verified |
| **Memory Footprint (RSS RAM)** | 375.08 MB | **171.73 MB** | **54.2% Less Memory** | ✅ Verified |

---

## 🔄 Backward Compatibility Note
For seamless compatibility with existing scripts and documentation, root aliases forward automatically to their respective modular suites:
* `benchmarks/benchmark_max_laravel_complex.py` ➔ `benchmarks/suites/laravel_complex.py`
* `benchmarks/benchmark_all_frameworks_max_concurrency.py` ➔ `benchmarks/suites/frameworks_concurrency.py`
* `benchmarks/benchmark_laravel_crud.py` ➔ `benchmarks/suites/laravel_crud.py`
* `benchmarks/benchmark_normal_php.py` ➔ `benchmarks/suites/vanilla_php.py`
* `benchmarks/stress_concurrency_and_ram.py` ➔ `benchmarks/stress/stress_concurrency_and_ram.py`
* `benchmarks/verify_optimizations.py` ➔ `benchmarks/stress/verify_optimizations.py`
* `benchmarks/benchmark_suite.js` ➔ `benchmarks/k6/load_test.js`
