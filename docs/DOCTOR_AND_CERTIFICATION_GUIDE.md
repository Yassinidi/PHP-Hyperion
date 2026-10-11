# 🛡️ Hyperion Laravel Pro Doctor & Production Certification Guide

## 1. Overview

**Hyperion Laravel Doctor** ([`bin/hyperion-doctor`](file:///Users/elidinaili/Desktop/PHP-H/bin/hyperion-doctor) / [`tools/hyperion_laravel_doctor.py`](file:///Users/elidinaili/Desktop/PHP-H/tools/hyperion_laravel_doctor.py)) is an enterprise-grade automated diagnostic and verification engine designed to guarantee that **PHP-Hyperion** can replace **Zend PHP / PHP-FPM** in **ANY** Laravel application with **zero regressions, zero state leaks, and verified speedups**.

The tool executes an automated 7-Pillar verification battery against any target project directory on disk and outputs:
1. Real-time terminal diagnostic scorecard.
2. `HYPERION_LARAVEL_CERTIFICATION.json`: Full telemetry, timestamps, route discovery, and raw benchmark measurements.
3. `HYPERION_LARAVEL_CERTIFICATE.md`: Official markdown certificate with performance badges and readiness verdict.

---

## 2. The 7 Enterprise Production Pillars

Every Laravel project is audited against seven strict invariants:

### Pillar 1: Pre-Flight Architecture & Static Code Hygiene Audit
- **Goal**: Verifies directory integrity, `.env` configuration, database drivers, and Laravel framework version compatibility (Laravel 10, 11, 12, 13).
- **Checks**: Presence of `artisan`, `public/index.php`, `bootstrap/app.php`, valid PDO driver (`sqlite`, `mysql`, `pgsql`).

### Pillar 2: Dynamic Route & Endpoint Discovery
- **Goal**: Auto-discovers web views, JSON read endpoints, mutation routes, and probe endpoints without requiring manual configuration.
- **Checks**: Scans routing table via Artisan to locate SSR views (`/`, `/login`), API endpoints (`/users`, `/api/v1/...`), and health probes (`/up`, `/health`).

### Pillar 3: Dual-Engine Shootout (Zend PHP vs PHP-Hyperion)
- **Goal**: Head-to-head performance comparison under identical ApacheBench load.
- **Checks**:
  - Boots Zend PHP (`php -S`) on an isolated port $\rightarrow$ measures throughput (req/s) and latency (ms).
  - Boots PHP-Hyperion (`hyperion-cli -S`) with persistent worker pool $\rightarrow$ measures throughput, latency, and speedup multiplier.
  - Requires **$\ge 2.0\times$ speedup** and **0 failed requests**.

### Pillar 4: State & Session Bleed Defense
- **Goal**: Prevents cross-user authentication leakage and session poisoning across persistent worker cycles.
- **Checks**:
  - Dispatches 25 consecutive authenticated requests with custom session cookies.
  - Immediately issues a subsequent request **without cookies**.
  - Verifies that the unauthenticated response receives zero cached session data, zero leaked user credentials, and fresh request state.

### Pillar 5: Database Transaction Crash & Deadlock Immunity
- **Goal**: Ensures database connections do not hang or deadlock if an exception occurs mid-transaction.
- **Checks**:
  - Intentionally injects an aborting database mutation payload (`invalid_key_trigger: 999999`).
  - Verifies that uncommitted transactions are immediately rolled back.
  - Immediately executes subsequent queries to confirm **zero lock contention** and **zero `SQLITE_BUSY` errors**.

### Pillar 6: Memory RSS Stability & Zero-Leak Probe
- **Goal**: Validates that persistent worker processes do not leak memory across hundreds of request cycles.
- **Checks**:
  - Samples physical process Resident Set Size (RSS) directly from OS kernel before and after 500 consecutive full requests.
  - Computes linear memory slope: requires slope $< 1.0\text{ MB} / 100\text{ reqs}$.

### Pillar 7: Enterprise Concurrency SLA Guarantee
- **Goal**: Validates high-concurrency throughput and latency guarantees under production load.
- **Checks**:
  - Executes ApacheBench stress test at concurrency $c=20$ to $40$.
  - Requires throughput $> 500\text{ req/s}$ on full SSR pages ($> 50,000\text{ req/s}$ on APIs), sub-millisecond P99 latency, and **0 failed requests**.

---

## 3. How to Run `hyperion-doctor`

### Basic Usage
Run against any Laravel project on disk:

```bash
# Point directly to your Laravel project directory:
./bin/hyperion-doctor --project /path/to/my-laravel-project
```

### Advanced Options
```bash
./bin/hyperion-doctor \
    --project /path/to/my-laravel-project \
    --endpoint /up \
    --workers 16 \
    --cycles 500 \
    --concurrency 40 \
    --php-binary /opt/homebrew/bin/php
```

| Flag | Default | Description |
| :--- | :--- | :--- |
| `--project` | `frameworks/ecommerce-laravel` | Absolute or relative path to target Laravel project |
| `--endpoint` | Auto-detected | Specific route URI to benchmark (e.g. `/`, `/up`, `/api/users`) |
| `--workers` | `16` | Number of persistent worker processes in pool |
| `--cycles` | `500` | Number of soak cycles for memory leak detection |
| `--concurrency` | `40` | Concurrency level for ApacheBench SLA battery |
| `--php-binary` | System `php` | Path to custom PHP executable |

---

## 4. Sample Output & Scorecard

```
================================================================================
 🏁 FINAL LARAVEL PRODUCTION CERTIFICATION AUDIT
================================================================================
  Project Name    : github-laravel-admintw
  Laravel Version : Laravel Framework 12.14.1
  PHP Runtime     : PHP 8.2.33 (cli) (built: Jul 28 2026 10:09:14) (NTS)
  Audit Verdict   : CERTIFIED PRODUCTION READY
--------------------------------------------------------------------------------
  • Preflight                           : PASSED ✓
  • Route Discovery                     : PASSED ✓
  • Session Bleed                       : PASSED ✓
  • Transaction Resilience              : PASSED ✓
  • Memory Stability                    : PASSED ✓
  • Concurrency Sla                     : PASSED ✓
  • Hyperion Speedup vs Zend            : 1655.80× FASTER
================================================================================
  📁 Machine-Readable JSON Export : /path/to/project/HYPERION_LARAVEL_CERTIFICATION.json
  📄 Human-Readable Markdown Cert : /path/to/project/HYPERION_LARAVEL_CERTIFICATE.md
```
