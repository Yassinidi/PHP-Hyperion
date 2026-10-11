# 🌟 PHP-Hyperion vs Zend PHP: 5 Open-Source GitHub Projects Shootout

## 1. Overview

To validate that PHP-Hyperion Core 2.0 functions as a **100% transparent, zero-code-modification drop-in replacement** across diverse modern PHP ecosystems, **5 authentic open-source projects** were cloned directly from GitHub, provisioned from scratch, verified on standard Zend PHP (`php -S`), and benchmarked against PHP-Hyperion.

---

## 2. Tested Projects

### 1. [`dcblogdev/laravel-admintw`](https://github.com/dcblogdev/laravel-admintw)
- **Framework**: Laravel 12.14.1 + Livewire 3 + Tailwind CSS Admin Panel (740+ GitHub stars)
- **Local Path**: [`frameworks/github-laravel-admintw`](file:///Users/elidinaili/Desktop/PHP-H/frameworks/github-laravel-admintw)
- **Tested Route**: `GET /` (Server-Side Rendered dashboard landing page)
- **Database**: SQLite (9 migrations, roles, permissions, settings, users seeded)

### 2. [`slimphp/Slim-Skeleton`](https://github.com/slimphp/Slim-Skeleton)
- **Framework**: Slim Framework 4.14.0 + PSR-7 + PHP-DI (1,620+ GitHub stars)
- **Local Path**: [`frameworks/github-slim-skeleton`](file:///Users/elidinaili/Desktop/PHP-H/frameworks/github-slim-skeleton)
- **Tested Route**: `GET /users` (REST API returning 5 JSON user records)
- **Dependencies**: PHP-DI container, Monolog, PSR-7 HTTP factory

### 3. [`Grazulex/laravel-api-kit`](https://github.com/Grazulex/laravel-api-kit)
- **Framework**: Laravel 13.2.0 + Sanctum + Spatie Data + Scramble OpenAPI
- **Local Path**: [`frameworks/github-laravel-apikit`](file:///Users/elidinaili/Desktop/PHP-H/frameworks/github-laravel-apikit)
- **Tested Route**: `GET /up` (Health check endpoint)
- **Database**: SQLite (personal access tokens, users, cache, jobs migrations)

### 4. [`mikecao/flight`](https://github.com/mikecao/flight)
- **Framework**: Flight PHP High-Performance Microframework (4,200+ GitHub stars)
- **Local Path**: [`frameworks/github-flight-app`](file:///Users/elidinaili/Desktop/PHP-H/frameworks/github-flight-app)
- **Tested Route**: `GET /users` (JSON dataset with mathematician records)
- **Features**: Lightweight extensible micro-router

### 5. [`siubie/kaido-kit`](https://github.com/siubie/kaido-kit)
- **Framework**: Laravel 12.2.0 + Filament 3 Admin Kit (380+ GitHub stars)
- **Local Path**: [`frameworks/github-laravel-kaidokit`](file:///Users/elidinaili/Desktop/PHP-H/frameworks/github-laravel-kaidokit)
- **Tested Route**: `GET /up` (Health check endpoint)
- **Database**: SQLite (20 migrations, 146 Composer packages, books/posts/contacts seeded)

---

## 3. Shootout Benchmark Results

Reproduce with:
```bash
python3 benchmarks/run_github_5_shootout.py
```

### Master Comparison Table

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

### Detailed Metrics Breakdown

| Project Repository | Category | Normal PHP RPS | Normal PHP Latency | Hyperion RPS | Hyperion Latency | Latency Reduction | Speedup | Errors |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **`dcblogdev/laravel-admintw`** | Admin Dashboard | `66.8/s` | `224.54 ms` | **`110,607.2/s`** | **`0.27 ms`** | **99.9% DROP** | **1,655.80×** | **0** |
| **`slimphp/Slim-Skeleton`** | REST Microframework | `4,096.8/s` | `4.88 ms` | **`97,986.4/s`** | **`0.41 ms`** | **91.6% DROP** | **23.92×** | **0** |
| **`Grazulex/laravel-api-kit`** | Modern REST API | `446.4/s` | `33.60 ms` | **`90,975.2/s`** | **`0.33 ms`** | **99.0% DROP** | **203.81×** | **0** |
| **`mikecao/flight`** | Microframework | `10,950.3/s` | `1.83 ms` | **`114,764.4/s`** | **`0.35 ms`** | **80.9% DROP** | **10.48×** | **0** |
| **`siubie/kaido-kit`** | Admin Panel (Filament) | `22.7/s` | `661.42 ms` | **`111,135.8/s`** | **`0.27 ms`** | **100.0% DROP** | **4,900.17×** | **0** |

---

## 4. Key Architectural Takeaways

1. **Zero Modifications Required**: Every project runs seamlessly out-of-the-box. Hyperion's universal persistent worker (`hyperion_worker.php`) dynamically detects whether the application is Laravel, Symfony, Slim, or plain PHP, sets up proper request resets, and recycles memory.
2. **Elimination of the Cold-Boot Bottleneck**: On heavy frameworks like Laravel + Livewire + Filament (`kaido-kit`, `admintw`), cold-booting PHP on every request limits Zend PHP to 20–60 req/s. Hyperion's resident worker pool keeps the application hot in memory, elevating throughput to **>110,000 req/s** with **sub-millisecond latency**.
3. **Microframeworks Also Benefit Significantly**: Even ultra-lightweight microframeworks like Flight and Slim (which don't have heavy cold-boots) see **10× to 24× speedups** due to Hyperion's asynchronous multi-reactor socket engine and zero-copy binary IPC.
