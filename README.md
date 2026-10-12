<div align="center">

# ⚡ PHP-Hyperion

**Next-Generation High-Performance PHP 8.4 Runtime Engine**

[![Version](https://img.shields.io/badge/version-v0.1.0-blue.svg)](https://github.com/Yassinidi/PHP-Hyperion)
[![License: PolyForm Noncommercial](https://img.shields.io/badge/License-NonCommercial-red.svg)](LICENSE)
[![Platforms](https://img.shields.io/badge/platforms-macOS%20%7C%20Linux%20%7C%20Windows-brightgreen.svg)](#-universal-installation-for-all-operating-systems)
[![PHP](https://img.shields.io/badge/php-8.4%20compatible-777bb4.svg)](https://www.php.net/)
[![Throughput](https://img.shields.io/badge/throughput-146%2C000%2B%20req%2Fsec-brightgreen.svg)](#-benchmarks)
[![Latency](https://img.shields.io/badge/latency-%3C%201ms%20sub--millisecond-blue.svg)](#-benchmarks)

<p align="center">
  <b>Deliver Go-like throughput and sub-millisecond latencies for your Laravel, Symfony, WordPress, and PHP applications with ZERO code modifications.</b>
</p>

</div>

---

## 🌟 What is PHP-Hyperion?

**PHP-Hyperion** is an ultra-high-performance asynchronous PHP 8 runtime engine engineered to eliminate the traditional PHP cold-boot bottleneck—loading thousands of framework files on every HTTP request—without requiring specialized worker wrappers, code modifications, or manual memory management.

Hyperion combines an **asynchronous multi-reactor event loop (`mio` / `kqueue` / `epoll`)**, an **in-memory RFC 9111 generational gateway cache**, and a revolutionary **Universal Persistent IPC Worker Pool** communicating over local Unix Domain Sockets using a high-speed binary protocol (`HYP\x01` / `HYP\x02`). It delivers **90,000 to 208,000+ requests per second** while guaranteeing **100% true OS process concurrency, data safety, and zero session bleeding**.

---

## 📦 Universal Installation for All Operating Systems

PHP-Hyperion is distributed as pre-compiled, optimized deployment binaries ready for immediate use on all major operating systems.

### 🍎 macOS (Apple Silicon M1–M4 & Intel x86_64)

#### Option 1: 1-Line Universal Installer (Recommended)
Open Terminal and run:
```bash
curl -fsSL https://raw.githubusercontent.com/Yassinidi/PHP-Hyperion/main/install.sh | bash
```

#### Option 2: Clone & Install Local Binary
```bash
git clone https://github.com/Yassinidi/PHP-Hyperion.git
cd PHP-Hyperion
./install.sh
```
Or directly copy the pre-compiled Universal binary to your PATH:
```bash
# If using Homebrew (Apple Silicon):
cp deploy/hyperion /opt/homebrew/bin/hyperion

# Or system-wide:
sudo cp deploy/hyperion /usr/local/bin/hyperion
```

---

### 🐧 Linux (Ubuntu, Debian, Alpine, RHEL / x86_64 & ARM64)

#### Option 1: 1-Line Automated Installer
```bash
curl -fsSL https://raw.githubusercontent.com/Yassinidi/PHP-Hyperion/main/install.sh | bash
```

#### Option 2: Docker & Docker Compose (Zero Configuration)
Run Hyperion instantly with built-in Redis caching and Nginx reverse proxy:
```bash
git clone https://github.com/Yassinidi/PHP-Hyperion.git
cd PHP-Hyperion
docker compose up -d
```

---

### 🪟 Windows (WSL2, Native & Docker)

#### Option 1: WSL2 (Ubuntu / Debian - Recommended for Max Performance)
Inside your WSL2 terminal:
```bash
curl -fsSL https://raw.githubusercontent.com/Yassinidi/PHP-Hyperion/main/install.sh | bash
```

#### Option 2: PowerShell
Open PowerShell as Administrator:
```powershell
.\install.ps1
```

---

## 🛠️ Quick Start

Once installed, verify that `hyperion` is available in your shell:
```bash
hyperion --version
# PHP-Hyperion 0.1.0 (cli)
```

### 1. Run Any Single PHP Script
```bash
hyperion script.php
```

### 2. High-Performance Web Server
Serve the current directory or framework with auto-detected router:
```bash
hyperion -S 127.0.0.1:8000
```

### 3. Persistent Worker Mode (200,000+ Req/sec)
Run Hyperion in persistent daemon worker mode for any framework (Laravel, Symfony, Slim, WordPress, Native PHP):
```bash
# Auto-detects public directory, project type, and database resets:
hyperion -S 127.0.0.1:8000 -W

# Or explicitly pass document root and the universal worker:
hyperion -S 127.0.0.1:8000 -W -t public hyperion_worker.php
```

### 4. High-Concurrency FastCGI Backend (Nginx)
Run Hyperion as a high-speed FastCGI daemon:
```bash
hyperion -b 127.0.0.1:9000 -W
```

Nginx configuration example:
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

## 📁 Deployed Repository Structure

```
PHP-Hyperion/
├── deploy/                   # Production deployment assets
│   ├── hyperion              # Universal native binary (Apple Silicon + Intel macOS)
│   ├── hyperion.service      # Systemd service unit for Linux servers
│   └── sysctl.conf           # Kernel high-concurrency socket tuning parameters
├── bin/
│   ├── hyperion              # Deployed executable
│   └── hyperion-doctor       # Enterprise Laravel certification and audit tool
├── install.sh                # Cross-platform 1-line installer (macOS & Linux)
├── install.ps1               # Windows PowerShell installer
├── Dockerfile                # High-performance PHP 8.4 container deployment
├── docker-compose.yml        # Multi-container orchestration (Hyperion + Nginx + Redis)
├── hyperion_worker.php       # Universal zero-config persistent worker engine
├── benchmarks/               # Concurrency benchmark suites & load test scripts
├── scripts/                  # Automated verification and stress tools
├── tests/                    # Unit, integration, and memory stability tests
└── README.md                 # Complete system guide
```

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

## 🌟 Head-to-Head Shootout: Real Open-Source Projects

Tested across authentic open-source PHP projects with **zero code modifications**:

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

---

## 🛡️ Hyperion Laravel Doctor & Enterprise Certification

Any developer can certify their existing Laravel application with a single CLI command:

```bash
# Audit any Laravel project directory on disk:
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

---

## 🔄 Engine Modes & Full PHP 8.4 Support

| Engine Mode | Flag | Description |
| :--- | :--- | :--- |
| **Auto (Default)** | `--engine=auto` | Automatically inspects your script. If modern PHP 8.4 features (Property Hooks, Asymmetric Visibility, `mysqli_*`, WordPress, etc.) are detected, it dispatches to the Zend 8.4 engine; otherwise runs on Hyperion VM. |
| **PHP 8.4 (Zend)** | `--engine=php84` / `-E zend` | Forces 100% official PHP 8.4 execution with all compiled native C extensions (`mysqli`, `pdo_mysql`, `gd`, `intl`, `imagick`, `opcache`, etc.). |
| **Native VM** | `--engine=vm` | Forces Hyperion's native direct-threaded VM and JIT engine for ultra-high throughput. |

---

## 📋 System & Production Requirements

| Requirement Category | Minimum Specification | Recommended for 100k+ Req/s |
| :--- | :--- | :--- |
| **Operating System** | Linux 5.4+ or macOS Darwin 22+ or Windows WSL2 | Linux Kernel 6.x (Ubuntu 24.04 / Debian 12) |
| **CPU Architecture** | 2 Cores (ARM64 / x86_64) | 8–16+ Cores (Apple Silicon or AMD EPYC / Intel Xeon) |
| **Memory (RAM)** | 512 MB | 4 GB – 16 GB |
| **PHP Runtime** | PHP 8.2+ CLI | **PHP 8.4 CLI** with OPcache JIT (`tracing`) |
| **Mandatory Extensions** | `pcntl`, `posix`, `pdo`, `mbstring`, `openssl` | `redis`, `igbinary`, `intl` |
| **Socket Descriptors** | `ulimit -n 4096` | `ulimit -n 65535` & `net.core.somaxconn = 65535` |
| **Session & Cache** | In-memory / Cookie | **Redis** (avoid blocking `file` session locks) |

---

## 📜 License

PHP-Hyperion is licensed under the **PolyForm Noncommercial License 1.0.0** ([LICENSE](LICENSE)).

- ✅ **Free for Noncommercial Use**: You are free to use, study, modify, test, and distribute this software for personal projects, academic research, education, and non-commercial evaluation.
- 🚫 **Commercial Use Restricted**: Any commercial use, production business deployment, or integration into revenue-generating services/platforms requires an explicit commercial license.
- 💼 **Commercial Licensing**: To acquire a commercial license or enterprise support, please contact **ELIDI NAILI** at [nailiyassin2@gmail.com](mailto:nailiyassin2@gmail.com).
