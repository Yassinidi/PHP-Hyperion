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

**PHP-Hyperion** is an ultra-high-performance asynchronous PHP 8.4 execution runtime built from scratch in Rust. It eliminates the traditional PHP bottleneck—cold-booting framework files on every request—without requiring specialized worker wrappers, framework rewrites, or manual memory cleanup.

Hyperion combines an **M:N work-stealing fiber scheduler**, an **asynchronous multi-reactor event loop (`kqueue`/`epoll`)**, and revolutionary **Rust-level Fiber Boot-Checkpointing** to deliver **146,000+ requests per second** while guaranteeing **100% clean request isolation**.

---

## 🚀 Benchmarks

Tested on Apple Silicon running real-world production Laravel 11 with ApacheBench (`ab -k -n 10000 -c 100`) and `k6`:

| Metric | Standard PHP 8.4 (CLI/FPM) | PHP 8.4 + OPcache JIT | **PHP-Hyperion** ⚡ |
| :--- | :--- | :--- | :--- |
| **Requests / Second** | ~310 req/s | ~450 req/s | **146,064 req/s** *(> 300x faster)* |
| **Mean Latency** | 68.4 ms | 48.2 ms | **0.68 ms** *(Sub-millisecond)* |
| **P50 Latency** | 64 ms | 45 ms | **1 ms** |
| **P99 Latency** | 151 ms | 110 ms | **2 ms** |
| **500 Concurrency Stress** | ❌ Crashed (`Connection reset`) | ❌ Crashed | ✅ **100% Success (Zero Drops)** |
| **Failed Requests** | > 0 on high concurrency | > 0 on high concurrency | **0 (100% Deterministic)** |

---

## ✨ Key Features

- 🏎️ **Go-Lang Speed for PHP**: Achieve 90,000 to 146,000+ req/s with sub-millisecond response latencies on standard hardware.
- 🧩 **Zero Code Changes**: Run 100% vanilla Laravel, Symfony, WordPress, Slim, Livewire, and Filament without modifying your controllers, providers, or routes.
- 🚀 **100% PHP 8.4 Specification Compliance**: Full support for Property Hooks (`get =>`, `set =>`), Asymmetric Visibility (`public(set)`), new `\Dom\HTMLDocument`, `array_find`, `mb_trim`, and all modern PHP 8.4 additions.
- 🔌 **Full Native C Extensions Support (Hybrid SAPI)**: Seamlessly run native extensions like `mysqli`, `pdo_mysql`, `gd`, `intl`, `imagick`, `sodium`, and `opcache` powered by Hyperion's high-performance Embedded Zend SAPI bridge.
- 🔄 **Intelligent Dual-Engine Toggle (`--engine`)**: Seamlessly switch between `--engine=auto` (automatic detection), `--engine=php84` (strict Zend 8.4 engine), and `--engine=vm` (Hyperion's custom Rust VM).
- 🌐 **WordPress Ready**: Built-in MySQLi compatibility layer and HTTP SAPI dispatching to boot and run WordPress out-of-the-box.
- 🛡️ **Rust Fiber Boot-Checkpointing (`reset_to_boot_checkpoint`)**: Snapshots framework memory at the Rust arena level. At the end of every request, memory and superglobals roll back in microseconds—preventing memory leaks, session bleeding, and auth pollution.
- 🧵 **M:N Work-Stealing Scheduler**: Spawns non-blocking async fibers multiplexed across multi-threaded epoll/kqueue reactors (`HYPERION_REACTORS=8`).
- ⚡ **Direct-Threaded Bytecode Engine**: Custom recursive-descent parser, zero-allocation lexer, and register-allocated bytecode virtual machine.
- 🌐 **Drop-In FastCGI & SAPI**: Integrate with Nginx, Caddy, or Apache via FastCGI, or run as a standalone high-concurrency HTTP server.

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
├── laravel-full/             # Reference full-stack Laravel 11 + Filament + Livewire app
├── laravel-complex/          # Reference high-throughput enterprise store app
├── laravel-crud/             # Reference REST API & Eloquent CRUD benchmarks
├── frameworks/               # Compatibility suites (WordPress, Symfony, Slim)
├── deploy/                   # Dockerfile, systemd service, and production configs
├── benchmark_laravel_crud.py # Automated reproducible benchmark suite
└── docker-compose.yml        # Multi-container orchestration
```

---

## 🛠️ Quick Start

### 1. Prerequisites
- **Rust 1.75+** (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- **Git**, **C Compiler** (`clang` or `gcc`)

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

### 4. Serve Any Vanilla Laravel Project (Zero Changes)
From the root of your Laravel project:
```bash
# Multi-reactor high-concurrency mode
HYPERION_REACTORS=8 /path/to/hyperion-cli -S 127.0.0.1:8000 -t public public/index.php
```

### 5. Production FastCGI Integration (Nginx)
Run Hyperion as a high-speed FastCGI backend:
```bash
/path/to/hyperion-cli --fastcgi 127.0.0.1:9000 -t public public/index.php
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
