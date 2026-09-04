# PHP-Hyperion ⚡

**PHP-Hyperion** is an ultra-high-performance, asynchronous PHP 8.4 runtime engine engineered from scratch in Rust.

It delivers **146,000+ Requests/Second** on **100% unmodified vanilla Laravel** applications with **zero code changes**, sub-millisecond latencies, deterministic memory safety, and an advanced M:N fiber concurrency model.

---

## 🚀 Benchmark Highlights

Tested on a local Apple Silicon workstation running real-world Laravel with ApacheBench (`ab -k -n 10000 -c 100`):

| Metric | PHP-FPM (Standard) | PHP-Hyperion | Speedup |
| :--- | :--- | :--- | :--- |
| **Requests / Second** | ~800 - 1,200 req/s | **146,064.30 req/s** | **> 120x Faster** |
| **Mean Latency** | ~80 - 120 ms | **0.007 ms** (0.685 ms / batch) | **Instant (< 1 ms)** |
| **50% Percentile** | ~85 ms | **1 ms** | **Ultra Low** |
| **Failed Requests** | 0 | **0** | **100% Deterministic** |

---

## 📖 Documentation & Usage

For full installation, CLI commands, server setup, and Laravel deployment instructions, read our comprehensive documentation:

👉 **[Complete Usage & Deployment Guide (USAGE.md)](./USAGE.md)**

---

## 🧠 Architectural Overview

- **`crates/hyperion-core`**: NaN-boxed 48-bit pointer value model, Generational Arena (< 1µs reset), object/array internals.
- **`crates/hyperion-parser`**: Handcrafted recursive descent AST parser and zero-allocation lexer for PHP 8.4 syntax.
- **`crates/hyperion-compiler`**: High-performance AST-to-Bytecode compiler with register allocation and SSA optimizations.
- **`crates/hyperion-vm`**: Direct-threaded execution engine, work-stealing M:N fiber scheduler, and asynchronous I/O reactors (`mio`/`kqueue`/`epoll`).
- **`crates/hyperion-jit`**: Trace-based dynamic JIT compiling hot loops into native assembly.
- **`ext/`**: Standard extension libraries (Strings, Math, PDO, SQLite, Hash, OpenSSL, Date, JSON, FastCGI).
- **`sapi/hyperion-cli`**: Command-line interface, development HTTP server (`-S`), worker daemon (`-W`), and FastCGI daemon.

---

## ⚡ Quick Start

### 1. Build
```bash
cargo build --release -p hyperion-cli
```

### 2. Run a Script
```bash
./target/release/hyperion-cli script.php
```

### 3. Run a Laravel Web Server
```bash
PHP_CLI_SERVER_WORKERS=8 ./target/release/hyperion-cli -S 127.0.0.1:8000 path/to/laravel/public/index.php
```

---

## 📄 License
PHP-Hyperion is open-source software licensed under the MIT License.
