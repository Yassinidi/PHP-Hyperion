# ⚡ System & Runtime Requirements for Peak Performance

This document outlines the **essential hardware, kernel, runtime, and framework requirements** for PHP-Hyperion to operate at maximum efficiency, sub-millisecond latency, and 100,000+ requests per second with zero process drops or memory leaks.

---

## 📋 Quick Preflight Checklist

Run this command to automatically verify your project and runtime against all 7 production pillars:
```bash
./bin/hyperion-doctor --project /path/to/your-laravel-app --strict
```

| Component | Minimum Requirement | Recommended for 100k+ Req/s |
| :--- | :--- | :--- |
| **Operating System** | Linux Kernel 5.4+ or macOS Darwin 22+ | Linux Kernel 6.x+ (Ubuntu 24.04 / Debian 12 / RHEL 9) |
| **CPU Architecture** | 2 Cores (ARM64 or x86_64) | 8 to 16+ Cores (Apple Silicon M-Series or AMD EPYC / Intel Xeon) |
| **RAM (Memory)** | 512 MB | 4 GB to 16 GB |
| **PHP Runtime** | PHP 8.2 CLI | **PHP 8.4 CLI** with OPcache JIT (`tracing`) |
| **File Descriptors** | `ulimit -n 4096` | `ulimit -n 65535` (or `1048576`) |
| **Kernel `somaxconn`**| 128 (Default) | `65535` |
| **Session Driver** | `cookie` / `array` | `redis` (avoid blocking `file` locks under high load) |
| **Database Pool** | Direct PDO connections | **PgBouncer** / MySQL Proxy (for 500+ concurrency) |

---

## 1. 🖥️ Operating System & Architecture

### Supported Platforms
- **Linux**: Ubuntu 22.04/24.04 LTS, Debian 11/12, RHEL/Rocky Linux 8/9, Alpine Linux 3.19+ (glibc or musl).
- **macOS**: macOS 13 (Ventura), 14 (Sonoma), 15 (Sequoia) on Apple Silicon (ARM64) and Intel (x86_64).
- **Containers**: Docker 24+, containerd, Kubernetes 1.28+.

### CPU Architecture
- **ARM64**: Apple Silicon (M1/M2/M3/M4 Pro/Max/Ultra), AWS Graviton 2/3/4, Ampere Altra.
- **x86_64**: Modern Intel/AMD with AVX2 or AVX-512 instruction sets.
- **Core Allocation**: Hyperion scales linearly with CPU cores ($Workers = Cores \times 1\text{ to }2$).

### Memory (RAM) Sizing Formula
To prevent out-of-memory restarts under peak load:
$$\text{Total RAM Required} = (\text{Workers} \times 32\text{ MB}) + \text{Gateway Cache} (256\text{ MB}) + \text{OS Buffer} (512\text{ MB})$$
- **16 Workers**: $\approx 1.25\text{ GB RAM}$
- **32 Workers**: $\approx 1.80\text{ GB RAM}$
- **64 Workers**: $\approx 2.80\text{ GB RAM}$

---

## 2. 🐘 PHP Runtime & Required Extensions

Hyperion requires a standard PHP CLI binary accessible in your system `PATH` (or specified via `HYPERION_PHP_CLI=/path/to/php`).

### Required PHP Version
- **PHP 8.2**, **PHP 8.3**, or **PHP 8.4** (Recommended for Property Hooks & Asymmetric Visibility support).

### Mandatory PHP Extensions
These extensions must be enabled in `php.ini` (`php -m`):
```ini
; 1. Process and Signal Supervision (Required for worker lifecycle)
extension=pcntl
extension=posix

; 2. Database Connectivity
extension=pdo
extension=pdo_mysql   ; Or pdo_pgsql / pdo_sqlite depending on your database

; 3. Core Cryptography & String Primitives
extension=mbstring
extension=openssl
extension=curl
extension=tokenizer
extension=filter
extension=json
```

### OPcache & JIT Configuration (Critical for Max Performance)
Add or update these directives in your `php.ini` or `/etc/php/8.4/cli/conf.d/10-opcache.ini`:
```ini
[opcache]
opcache.enable=1
opcache.enable_cli=1
opcache.memory_consumption=256
opcache.interned_strings_buffer=32
opcache.max_accelerated_files=20000
opcache.validate_timestamps=0
opcache.revalidate_freq=0
opcache.save_comments=1

; Enable JIT Compiler:
opcache.jit=tracing
opcache.jit_buffer_size=128M
```

---

## 3. 🐧 Linux Kernel TCP & Socket Tuning (`sysctl.conf`)

Default Linux kernel socket limits (128 listen backlog) choke under heavy concurrent load (> 5,000 req/s). Apply these settings to handle 100,000+ req/s:

Create `/etc/sysctl.d/99-hyperion.conf`:
```ini
# Maximum number of system file descriptors
fs.file-max = 2097152

# Socket listen backlog (allows thousands of simultaneous connection attempts)
net.core.somaxconn = 65535
net.ipv4.tcp_max_syn_backlog = 65535

# Fast TCP TIME_WAIT socket recycling
net.ipv4.tcp_tw_reuse = 1
net.ipv4.tcp_fin_timeout = 15

# Expand local port range for outbound connections
net.ipv4.ip_local_port_range = 1024 65535

# Increase network queue lengths
net.core.netdev_max_backlog = 65535

# TCP memory buffer autotuning (Min, Default, Max in bytes)
net.ipv4.tcp_rmem = 4096 87380 16777216
net.ipv4.tcp_wmem = 4096 65536 16777216
net.core.rmem_max = 16777216
net.core.wmem_max = 16777216

# Enable TCP BBR Congestion Control (Kernel 4.9+)
net.core.default_qdisc = fq
net.ipv4.tcp_congestion_control = bbr
```

Apply immediately:
```bash
sudo sysctl -p /etc/sysctl.d/99-hyperion.conf
```

### User File Descriptor Limits (`ulimit`)
In `/etc/security/limits.conf`:
```ini
* soft nofile 65535
* hard nofile 65535
root soft nofile 65535
root hard nofile 65535
```

---

## 4. ⚡ Framework Pre-Optimization Requirements

To eliminate disk I/O during request dispatch, warm up and cache your framework configuration before launching Hyperion:

### For Laravel 10 / 11 / 12:
```bash
# Pre-compile configuration, route tree, and Blade views:
php artisan config:cache
php artisan route:cache
php artisan view:cache

# In .env: avoid file session locks under concurrent load:
SESSION_DRIVER=redis   # or cookie / array
CACHE_DRIVER=redis     # or array
```

### For Symfony 6 / 7:
```bash
# Dump production environment and compile DI container:
APP_ENV=prod composer dump-env prod
php bin/console cache:warmup --env=prod
```

### Database Connection Pooling:
When handling 1,000+ concurrent requests, direct MySQL/PostgreSQL connection exhaustion will occur if not pooled:
- **PostgreSQL**: Place **PgBouncer** in front (`pool_mode = transaction`, `max_client_conn = 10000`).
- **MySQL**: Enable ProxySQL or configure PDO persistent connections.

---

## 5. 🛠️ Development & Benchmark Toolchain

To compile the engine from source or run test batteries:

- **Rust Compiler**: Rust 1.80+ and Cargo (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`).
- **Python**: Python 3.9+ (for `tests/run_all_tests.py` and diagnostic tools).
- **Composer**: Composer 2.x (pre-bundled in repository at [`bin/composer`](file:///Users/elidinaili/Desktop/PHP-H/bin/composer)).
- **Benchmarking Tools**:
  - ApacheBench (`ab`): `/usr/sbin/ab`
  - Grafana k6: `brew install k6` or `apt install k6`

---

## 🚀 Ready to Launch in Production

Once requirements are verified:
```bash
HYPERION_PERSISTENT=1 PHP_CLI_SERVER_WORKERS=16 HYPERION_FAST_ROUTE=true \
./php-hyperion/target/release/hyperion-cli -S 0.0.0.0:8000 -t public -W
```
