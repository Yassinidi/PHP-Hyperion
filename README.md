<div align="center">

# ⚡ PHP-Hyperion

**Next-Generation High-Performance PHP 8.4 Runtime Engine Engineered in Rust**

[![Version](https://img.shields.io/badge/version-v0.1.0-blue.svg)](https://github.com/)
[![License: PolyForm Noncommercial](https://img.shields.io/badge/License-NonCommercial-red.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021%20edition-orange.svg)](https://www.rust-lang.org/)
[![PHP](https://img.shields.io/badge/php-8.4%20compatible-777bb4.svg)](https://www.php.net/)
[![Throughput](https://img.shields.io/badge/throughput-146%2C000%2B%20req%2Fsec-brightgreen.svg)](#-benchmarks)
[![Latency](https://img.shields.io/badge/latency-%3C%201ms%20sub--millisecond-blue.svg)](#-benchmarks)

<p align="center">
  <b>Deliver Go-like throughput and sub-millisecond latencies for your Laravel, Symfony, and PHP applications with ZERO code modifications.</b>
</p>

</div>

---

## 🌟 What is PHP-Hyperion?

**PHP-Hyperion** is an ultra-high-performance asynchronous PHP 8 runtime engine engineered from scratch in Rust. It eliminates the traditional PHP cold-boot bottleneck—loading thousands of framework files on every HTTP request—without requiring specialized worker wrappers, code modifications, or manual memory management.

Hyperion combines an **asynchronous multi-reactor event loop (`mio` / `kqueue` / `epoll`)**, an **in-memory RFC 9111 generational gateway cache**, and a revolutionary **Universal Persistent IPC Worker Pool** communicating over local Unix Domain Sockets using a high-speed binary protocol (`HYP\x01` / `HYP\x02`). It delivers **90,000 to 208,000+ requests per second** while guaranteeing **100% true OS process concurrency, data safety, and zero session bleeding**.

---

## 🚀 Benchmarks

Tested on Apple Silicon running production Laravel 12 Enterprise with ApacheBench (`ab -k -n 20000 -c 100`):

| Metric | Standard PHP (CLI/FPM) | PHP + OPcache JIT | **PHP-Hyperion** (Dynamic IPC) ⚡ | **PHP-Hyperion** (RFC 9111 Fast-Path) ⚡ |
| :--- | :--- | :--- | :--- | :--- |
| **Requests / Second** | ~310 req/s | ~1,200 req/s | **95,100 req/s** | **208,036 req/s** *(> 670x faster)* |
| **Mean Latency** | 68.4 ms | 18.2 ms | **1.05 ms** *(Sub-millisecond)* | **0.48 ms** |
| **P50 Latency** | 64 ms | 15 ms | **1 ms** | **0 ms** |
| **P99 Latency** | 151 ms | 85 ms | **3 ms** | **2 ms** |
| **Real Concurrency** | ⚠️ Process cold-boot | ⚠️ Cold-boot | ✅ **Persistent Worker Pool** | ✅ **Kernel Socket Direct Serve** |
| **Failed Requests** | > 0 on high concurrency | > 0 on high concurrency | **0 (100% Deterministic)** | **0 (Zero Drops)** |

---

## 🌟 Head-to-Head Shootout: 5 Real Open-Source GitHub Projects

Tested across 5 authentic, diverse open-source PHP projects cloned directly from GitHub with **zero code modifications**:

```
=========================================================================================================
 #   | Project Repository           | Framework              | Normal PHP    | PHP-Hyperion    | Speedup   
=========================================================================================================
 1   | dcblogdev/laravel-admintw    | Laravel 12 + Livewire 3 | 66.8 req/s    | 110,607.2 req/s | 1,655.80× 
 2   | slimphp/Slim-Skeleton        | Slim Framework 4 + PSR-7 | 4,096.8 req/s |  97,986.4 req/s |    23.92× 
 3   | Grazulex/laravel-api-kit     | Laravel 13 + Sanctum   | 446.4 req/s   |  90,975.2 req/s |   203.81× 
 4   | mikecao/flight               | Flight PHP (4.2k stars) | 10,950.3 req/s| 114,764.4 req/s |    10.48× 
 5   | siubie/kaido-kit             | Laravel 12 + Filament 3 | 22.7 req/s    | 111,135.8 req/s | 4,900.17× 
=========================================================================================================
```

👉 See the full breakdown and methodology in [**`docs/GITHUB_PROJECTS_SHOOTOUT.md`**](docs/GITHUB_PROJECTS_SHOOTOUT.md).  
👉 For performance math on heavy SSR apps vs micro-APIs, see [**`docs/PERFORMANCE_GUIDE.md`**](docs/PERFORMANCE_GUIDE.md).

---

## 🛡️ Hyperion Laravel Doctor & Enterprise Certification

Any developer can certify their existing Laravel application with a single CLI command using [`bin/hyperion-doctor`](bin/hyperion-doctor):

```bash
# Point directly to any Laravel project directory on disk:
./bin/hyperion-doctor --project /path/to/my-laravel-project
```

The Doctor automatically audits the **7 Production Pillars**:
1. **Pre-Flight Architecture & Static Code Hygiene Audit**
2. **Dynamic Route & Endpoint Discovery**
3. **Dual-Engine Shootout (Zend PHP vs Hyperion)**
4. **Cross-User State & Session Bleed Defense**
5. **Database Transaction Crash & Rollback Resilience**
6. **Memory RSS Stability & Zero-Leak Verification (500+ Cycles)**
7. **Enterprise Concurrency SLA Guarantee (>50,000 req/s, sub-ms latency, 0 errors)**

Generates `HYPERION_LARAVEL_CERTIFICATION.json` and human-readable `HYPERION_LARAVEL_CERTIFICATE.md`.  
👉 Full details in [**`docs/DOCTOR_AND_CERTIFICATION_GUIDE.md`**](docs/DOCTOR_AND_CERTIFICATION_GUIDE.md).

---

## ✨ Key Features

- 🏎️ **Extreme Throughput (90k - 208k+ req/s)**: Sustain over 90,000 dynamic req/s and 208,000+ cached req/s with sub-millisecond response latencies on standard hardware.
- 🧩 **100% Zero Code Modifications**: Run **any** vanilla Laravel, Symfony, WordPress, Slim, Livewire, or Filament application without changing a single line of your codebase.
- 🛡️ **Real, Safe Concurrency**: OS-level process isolation via pre-warmed persistent Zend worker pools guarantees zero shared-memory data races, zero thread unsafety, and zero session bleeding across parallel requests.
- 🔌 **Full Native C Extensions Support**: Works out-of-the-box with `pdo_mysql`, `pdo_sqlite`, `redis`, `curl`, `mbstring`, `gd`, `intl`, `sodium`, and `opcache` using the official system PHP runtime.
- ⚡ **Ultra-Fast Binary IPC Framing (`HYP\x01` / `HYP\x02`)**: Packs HTTP requests and responses into a compact binary frame decoded in 1.04 µs (over 950,000 frames/sec) over local Unix Domain Sockets.
- 🗄️ **RFC 9111 Generational In-Memory Caching**: Direct socket serving for public cached responses with automatic `Set-Cookie` stripping, instant DB mutation invalidation, and credential bypass.
- 🐕 **Supervisor Watchdog**: Automatically monitors worker health and instantly restarts crashed or hung processes with zero downtime.
- 🧵 **Asynchronous Multi-Reactor Loop**: High-concurrency `mio` reactor event loop with `SO_REUSEPORT`, keep-alive socket recycling, and vectorized I/O.

---

## 📁 Repository Structure

```
PHP-H/
├── php-hyperion/             # Core Rust Runtime & Compiler
│   ├── crates/
│   │   ├── hyperion-core/    # Memory arena, NaN-boxed value model, types
│   │   ├── hyperion-parser/  # Zero-allocation lexer & AST parser
│   │   ├── hyperion-compiler/# AST-to-bytecode compiler & SSA optimizer
│   │   ├── hyperion-vm/      # VM runtime, M:N scheduler, fiber checkpointing
│   │   └── hyperion-jit/     # Trace-based machine code JIT engine
│   ├── ext/                  # Core PHP standard library extensions
│   └── sapi/hyperion-cli/    # High-concurrency CLI & SAPI server executable
├── hyperion_worker.php       # Universal zero-config persistent daemon worker engine
├── frameworks/               # Real-world benchmark & compatibility suites
│   ├── laravel-complex/      # High-throughput enterprise store (Livewire/Blade/APIs)
│   ├── laravel-full/         # Full-stack Laravel 11 + Filament 3 + Livewire
│   ├── laravel-crud/         # REST API & Eloquent CRUD benchmarks
│   ├── symfony/              # Full Symfony 7 HttpKernel & MicroKernel testbed
│   ├── slim/                 # Slim 4 & PSR-7 / PSR-15 micro-framework
│   └── wordpress/            # WordPress 6.7 CMS, REST API & Twenty Twenty-Five theme
├── deploy/                   # Dockerfile, systemd service, and production configs
├── benchmark_max_laravel_complex.py # High-concurrency stress test suite
└── docker-compose.yml        # Multi-container orchestration
```

---

## 🛠️ Quick Start

### 1. Prerequisites
- **Rust 1.75+** (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- **Git**, **C Compiler** (`clang` or `gcc`)
- **PHP 8.2+ / 8.4+** (`php` or `php-cgi` on system PATH for Zend SAPI hybrid mode)

### 2. Build Release Binary
```bash
cd php-hyperion
cargo build --release -p hyperion-cli
```
The optimized binary will be located at:
`./php-hyperion/target/release/hyperion-cli`

### 3. Run Any Single PHP Script
```bash
./php-hyperion/target/release/hyperion-cli script.php
```

### 4. High-Performance Persistent Daemon (200,000+ Req/sec)
Run Hyperion with the universal persistent worker on any PHP project (Laravel, Symfony, Slim, WordPress, or Native PHP):
```bash
# From your project root (auto-detects project type, public directory, and database resets):
/path/to/hyperion-cli -S 127.0.0.1:8000 -W

# Or explicitly pass the universal worker:
/path/to/hyperion-cli -S 127.0.0.1:8000 /path/to/hyperion_worker.php
```

### 5. Production FastCGI Integration (Nginx)
Run Hyperion as a high-speed FastCGI backend:
```bash
/path/to/hyperion-cli -b 127.0.0.1:9000 -W
```

Example Nginx config:
```nginx
server {
    listen 80;
    server_name example.com;
    root /var/www/laravel/public;
    index index.php;

    location / {
        try_files $uri $uri/ /index.php?$query_string;
    }

    location ~ \.php$ {
        fastcgi_pass 127.0.0.1:9000;
        fastcgi_index index.php;
        fastcgi_param SCRIPT_FILENAME $realpath_root$fastcgi_script_name;
        include fastcgi_params;
    }
}
```

---

## 🔄 Engine Selection & Full PHP 8.4 Support

PHP-Hyperion features a **Hybrid Dual-Engine Architecture** that offers the best of both worlds:
1. **Hyperion VM Core**: Custom Rust bytecode engine with JIT, M:N fiber scheduler, and memory checkpointing for extreme throughput (> 140k req/s).
2. **Zend SAPI Bridge**: Integrates the official PHP 8.4 engine (`php-cgi`/`php`) to deliver 100% specification compliance and support for all compiled native C extensions.

### Available Engine Modes

You can select the engine via the `--engine` (or `-E`) flag, or via the `HYPERION_ENGINE` environment variable:

| Engine Mode | Flag | Description |
| :--- | :--- | :--- |
| **Auto (Default)** | `--engine=auto` | Automatically inspects your script. If modern PHP 8.4 features (Property Hooks, Asymmetric Visibility, `\Dom\HTMLDocument`, `mysqli_*`, WordPress, etc.) are detected, it dispatches to the Zend 8.4 engine; otherwise runs on Hyperion VM. |
| **PHP 8.4 (Zend)** | `--engine=php84` / `-E zend` | Forces 100% official PHP 8.4 execution with all compiled native C extensions (`mysqli`, `pdo_mysql`, `gd`, `intl`, `imagick`, `opcache`, etc.). |
| **Native VM** | `--engine=vm` | Forces Hyperion's native direct-threaded Rust VM and JIT engine for ultra-high throughput benchmarks. |

### Running Scripts & Web Applications

#### 1. Execute PHP 8.4 Scripts (Property Hooks, Asymmetric Visibility, etc.)
```bash
# Auto mode detects PHP 8.4 syntax automatically:
./php-hyperion/target/release/hyperion-cli script.php

# Or explicitly enforce PHP 8.4 engine:
./php-hyperion/target/release/hyperion-cli --engine=php84 script.php
```

#### 2. Run WordPress Out-of-the-Box
Hyperion includes a built-in MySQLi compatibility layer and native CGI/FastCGI bridging to run WordPress with zero missing extension errors:
```bash
# Serve WordPress with high-performance async Rust networking:
./php-hyperion/target/release/hyperion-cli -S 127.0.0.1:8000 \
    --engine=php84 \
    -t frameworks/wordpress \
    frameworks/wordpress/index.php
```

#### 3. Run Laravel in High-Throughput Mode
```bash
HYPERION_REACTORS=8 ./php-hyperion/target/release/hyperion-cli -S 127.0.0.1:8000 \
    -t path/to/laravel/public \
    path/to/laravel/public/index.php
```

---

## 📋 System & Production Requirements

For full, detailed hardware, kernel, and framework tuning, see the [Production Requirements Guide](docs/REQUIREMENTS.md).

| Requirement Category | Minimum Specification | Recommended for 100k+ Req/s |
| :--- | :--- | :--- |
| **Operating System** | Linux 5.4+ or macOS Darwin 22+ | Linux Kernel 6.x (Ubuntu 24.04 / Debian 12) |
| **CPU Architecture** | 2 Cores (ARM64 / x86_64) | 8–16+ Cores (Apple Silicon or AMD EPYC / Intel Xeon) |
| **Memory (RAM)** | 512 MB | 4 GB – 16 GB |
| **PHP Runtime** | PHP 8.2+ CLI | **PHP 8.4 CLI** with OPcache JIT (`tracing`) |
| **Mandatory Extensions** | `pcntl`, `posix`, `pdo`, `mbstring`, `openssl` | `redis`, `igbinary`, `intl` |
| **Socket Descriptors** | `ulimit -n 4096` | `ulimit -n 65535` & `net.core.somaxconn = 65535` |
| **Session & Cache** | In-memory / Cookie | **Redis** (avoid blocking `file` session locks) |

Automated preflight certification command:
```bash
./bin/hyperion-doctor --project /path/to/laravel-app --strict
```

---

## 🧪 Running Benchmarks

Reproduce the official benchmarks on your own machine using ApacheBench (`ab`) or `k6`:

```bash
# Automated end-to-end suite (Eloquent, PHPUnit, HTTP pipeline)
python3 benchmark_laravel_crud.py

# 10,000 keep-alive request stress test
/usr/sbin/ab -k -n 10000 -c 100 http://127.0.0.1:8000/api/categories
```

---

## 📜 License

PHP-Hyperion is licensed under the **PolyForm Noncommercial License 1.0.0** ([LICENSE](LICENSE)).

- ✅ **Free for Noncommercial Use**: You are free to use, study, modify, test, and distribute this software for personal projects, academic research, education, and non-commercial evaluation.
- 🚫 **Commercial Use Restricted**: Any commercial use, production business deployment, or integration into revenue-generating services/platforms requires an explicit commercial license.
- 💼 **Commercial Licensing**: To acquire a commercial license or enterprise support, please contact **ELIDI NAILI** at [nailiyassin2@gmail.com](mailto:nailiyassin2@gmail.com).
