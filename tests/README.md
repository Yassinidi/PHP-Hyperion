# 🧪 PHP-Hyperion Test Suites & Verification Infrastructure

Comprehensive, multi-tier automated testing infrastructure for **PHP-Hyperion**. Validates language correctness, enterprise architecture patterns, framework compatibility, and defensive security invariants against high-traffic workloads.

---

## 📁 Test Architecture & Directory Layout

```
tests/
├── README.md                                # Test suite catalog & documentation (this file)
├── run_all_tests.py                         # Master test runner with color-coded CLI report
│
├── unit/                                    # Tier 1: PHP Language Primitives & Micro-Benchmarks
│   ├── run_unit_tests.php                   # Primitive test runner (PHP CLI vs Hyperion)
│   ├── test_arrays.php                      # Array manipulation & key/value allocations
│   ├── test_concat.php                      # String concatenation & buffer allocations
│   ├── test_date.php                        # DateTime & epoch calculations
│   ├── test_exception.php                   # Exception handling & stack unwinding
│   ├── test_explode.php                     # String delimiter splitting & tokenizing
│   ├── test_json.php                        # JSON encode/decode & UTF-8 serialization
│   ├── test_regex.php                       # PCRE2 regular expression matching
│   ├── test_str_repeat.php                  # String memory repetition
│   ├── test_str_replace.php                 # String replace & pattern replacement
│   ├── test_strings.php                     # Multibyte & string functions
│   ├── test_strtoupper.php                  # Case transformations
│   ├── test_substr.php                      # Substring slicing & bounds checking
│   └── test_minimal.php                     # Minimal baseline execution check
│
├── phpunit/                                 # Tier 2: PHPUnit 11 Enterprise Architecture
│   ├── phpunit.xml                          # Test suite configuration
│   ├── AssertionSuiteTest.php               # Data assertions & boundary conditions
│   ├── AdvancedFeaturesTest.php             # Reflection, attributes, and match guards
│   ├── ContainerTest.php                    # PSR-11 container, singletons & resolution
│   ├── DatabaseRepositoryTest.php           # Transactions, savepoints & query mapping
│   ├── DomainModelTest.php                  # Business models, immutability & validation
│   ├── MockingSuiteTest.php                 # Dynamic mocks & test doubles
│   ├── ServiceIntegrationTest.php           # Multi-service composition & pipeline
│   └── fixtures.php                         # Reusable mock models & test fixtures
│
├── integration/                             # Tier 3: Framework Core & Application Integration
│   ├── test_laravel_core.php                # 10 major Laravel subsystems verification battery
│   └── test_school_dual_engine.py           # Laravel School system head-to-head verification
│
├── security/                                # Tier 4: Defensive Security Invariants (HYP-SEC-01..09)
│   └── security_and_compatibility_audit.py  # Path traversal, buffer overflow, cache poisoning & multi-framework
│
└── performance/                             # Tier 5: Memory Leak & Stability Profiling
    ├── verify_memory_rss.py                 # Multi-iteration RSS memory leak verifier
    ├── memory_stress.php                    # Long-running memory stress script
    └── memory_stress_worker.php             # Persistent worker memory stress script
```

---

## 🚀 Running Tests

### 1. Master Test Runner (All Tiers)
Execute all test suites with a single command and generate a unified execution summary:
```bash
python3 tests/run_all_tests.py
```

### 2. Running Individual Tiers via Master Runner
```bash
# Tier 1: Language Primitives Only
python3 tests/run_all_tests.py --unit

# Tier 2: PHPUnit Suite Only
python3 tests/run_all_tests.py --phpunit

# Tier 3: Laravel Core Integration Only
python3 tests/run_all_tests.py --integration

# Tier 4: Security & Framework Audit Only
python3 tests/run_all_tests.py --security
```

### 3. Running Tiers Directly
```bash
# Tier 1: Unit Test Battery
php tests/unit/run_unit_tests.php

# Tier 2: PHPUnit Suite
php frameworks/laravel-full/vendor/phpunit/phpunit/phpunit -c tests/phpunit/phpunit.xml tests/phpunit/

# Tier 3: Laravel Subsystems Battery
php tests/integration/test_laravel_core.php

# Tier 4: Defensive Security Invariants Audit
python3 tests/security/security_and_compatibility_audit.py

# Tier 5: RSS Memory Stability Test
python3 tests/performance/verify_memory_rss.py
```

---

## 🛡️ What Each Tier Verifies

| Tier | Component | What is Verified | Tests | Status |
| :--- | :--- | :--- | :---: | :---: |
| **Tier 1** | `tests/unit/` | Arrays, Strings, Regex, JSON, DateTime, Exceptions, Substr | 13 files | ✅ 100% Passed |
| **Tier 2** | `tests/phpunit/` | Container DI, Database Repositories, Domain Models, Mocking | 38 tests | ✅ 156 Assertions |
| **Tier 3** | `tests/integration/` | 10 Laravel Core Subsystems (Container, Routing, Eloquent, Blade, Events, Cache, Session, Validation, Logging, Artisan) | 10 subsystems | ✅ 100% Passed |
| **Tier 4** | `tests/security/` | HYP-SEC-01 to HYP-SEC-09 Defensive Invariants + Multi-Framework | 9 invariants | ✅ 100% Secure |
| **Tier 5** | `tests/performance/`| Multi-iteration steady-state RAM verification, memory leak detection | 1,000+ reqs | ✅ Zero Leaks |

---

## 🔄 Backward Compatibility Note
For backward compatibility with existing CI scripts and documentation, root aliases exist for:
* `tests/security_and_compatibility_audit.py` ➔ `tests/security/security_and_compatibility_audit.py`
* `tests/verify_laravel_core.php` ➔ `tests/integration/test_laravel_core.php`
* `tests/verify_memory_rss.py` ➔ `tests/performance/verify_memory_rss.py`
* `tests/test_school_dual_engine.py` ➔ `tests/integration/test_school_dual_engine.py`
