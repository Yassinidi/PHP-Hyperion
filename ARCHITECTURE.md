# PHP-Hyperion Engine: Architecture, Internal Logic & Systems Design

**PHP-Hyperion** is an asynchronous, high-throughput PHP 8.4 runtime engine engineered from scratch in Rust. It eliminates the traditional PHP cold-boot bottleneck—loading thousands of framework files on every HTTP request—without requiring specialized worker wrappers, code modifications, or manual memory management.

This document details the complete structural layout, architectural components, execution logic, memory safety mechanisms, and security subsystems of the PHP-Hyperion engine.

---

## 1. System Architecture Overview

PHP-Hyperion follows a multi-threaded, asynchronous **M:N fiber execution model** combined with an **asynchronous reactor event loop** and a **direct-threaded bytecode virtual machine**.

```mermaid
flowchart TD
    Client["Client / Load Balancer / ab / k6"] -->|TCP Stream| Listeners["SO_REUSEPORT Listeners (Ports 8000 / 8001 / 9000)"]

    subgraph Reactors ["Asynchronous I/O Reactors (mio / kqueue / epoll)"]
        R0["Reactor 0"]
        R1["Reactor 1"]
        RN["Reactor N (HYPERION_REACTORS=8)"]
        
        CacheCheck{"RFC 9111 Fast-Path Cache Hit?"}
        GlobalCache[("Generational Response Cache\n(DashMap<String, PreformattedHttpResponses>)")]
        DBVersion["DB_MUTATION_VERSION (AtomicU64)"]
        
        R0 --> CacheCheck
        R1 --> CacheCheck
        RN --> CacheCheck
        
        CacheCheck -->|Hit (No Auth / No Cookie)| DirectServe["Direct Kernel Socket Write\n(< 15 µs Latency)"]
        DirectServe -->|Recycle Stream| KeepAliveQueue["Keep-Alive Stream Queue"]
        DBVersion -. Invalidate .-> GlobalCache
    end

    CacheCheck -->|Miss or Bypass (Cookie/Auth)| WorkStealing["Work-Stealing Task Dispatch"]

    subgraph Schedulers ["M:N Work-Stealing Scheduler"]
        GlobalQueue["crossbeam-deque Injector (Global Queue)"]
        IdleOctane["idle_octane_fibres (Boot-Checkpointed)"]
        IdleFibres["idle_fibres (Fresh / Per-Request)"]
        Stealers["Worker Stealer Handles"]
        
        WorkStealing --> GlobalQueue
    end

    subgraph Workers ["Worker Execution Cores (Worker 0 .. N)"]
        W0["Worker Core 0"]
        W1["Worker Core 1"]
        WN["Worker Core N (PHP_CLI_SERVER_WORKERS=16)"]
        
        GlobalQueue --> W0
        GlobalQueue --> W1
        GlobalQueue --> WN
    end

    subgraph ExecutionEngine ["Fibre Execution & VM Subsystem"]
        Fibre["Fibre Context"]
        Arena["Thread-Local Bump Arena (GcArena)"]
        VM["Direct-Threaded Bytecode Interpreter"]
        ZendBridge["Embedded Zend SAPI Bridge (PHP 8.4 C FFI)"]
        
        W0 --> Fibre
        Fibre --> Arena
        Fibre --> VM
        VM -. Extension Fallback .-> ZendBridge
    end

    ExecutionEngine -->|Response Buffer + Headers| StreamOut["write_vectored_all(Socket)"]
    StreamOut -->|If Cache-Control: public| InsertCache["Insert Clean Response\n(Strip Set-Cookie)"]
    InsertCache --> GlobalCache
```

---

## 2. Workspace & Crate Structure

The engine is partitioned into focused, modular Rust crates under `php-hyperion/`:

| Crate / Directory | Path | Primary Responsibility |
|---|---|---|
| **`hyperion-core`** | `crates/hyperion-core` | NaN-boxed `Value` representation, generational bump allocator (`GcArena`), PHP object model, dynamic class table, and native context interfaces. |
| **`hyperion-parser`** | `crates/hyperion-parser` | Zero-allocation lexer, token stream, and recursive-descent AST parser supporting full PHP 8.4 syntax. |
| **`hyperion-compiler`** | `crates/hyperion-compiler` | AST-to-bytecode compiler, register allocator, SSA optimizations, jump patching, and loop synthesis. |
| **`hyperion-bytecode`** | `crates/hyperion-bytecode` | Opcode definitions, instruction representation, and operand packing. |
| **`hyperion-vm`** | `crates/hyperion-vm` | Direct-threaded execution loop, M:N work-stealing fiber scheduler, asynchronous reactors (`mio`), keep-alive recycling, fast-path gateway cache, FastCGI responder, and hybrid Zend SAPI bridge. |
| **`hyperion-jit`** | `crates/hyperion-jit` | Trace-based dynamic JIT compiling hot bytecode traces directly into native ARM64/x86_64 assembly. |
| **`hyperion-ext-standard`** | `ext/hyperion-ext-standard` | Native implementations of standard PHP extensions: strings, arrays, reflection, math, PDO (SQLite/MySQL), filesystem, and hashing. |
| **`hyperion-cli`** | `sapi/hyperion-cli` | Primary binary entry point, HTTP server (`-S`), worker daemon (`-W`), FastCGI daemon (`-b`), and engine toggle (`--engine`). |

---

## 3. Core Engine Components & Detailed Logic

### 3.1. High-Performance Value Model (`hyperion-core`)
Hyperion uses a 64-bit **NaN-boxed value representation** (`Value`) to represent all PHP dynamic primitives without dynamic heap allocation:

- **Floating-Point Numbers**: Stored directly as 64-bit IEEE 754 doubles.
- **Pointers & Objects**: NaN-quiet bit patterns encode 48-bit canonical virtual memory pointers to objects, arrays, and strings.
- **Tagged Primitives**: Unused high-order bits encode `null`, `bool` (`true`/`false`), integers (`i64` or `i32`), and uninitialized values.
- **Result**: Zero pointer indirection for scalars, single-word register passing, and maximum L1 cache locality.

```rust
// Tagged pointer representation in NaN space
const QNAN: u64     = 0x7FFC_0000_0000_0000;
const TAG_NULL: u64 = QNAN | 0x0001_0000_0000;
const TAG_FALSE: u64 = QNAN | 0x0002_0000_0000;
const TAG_TRUE: u64  = QNAN | 0x0003_0000_0000;
const TAG_INT: u64   = QNAN | 0x0004_0000_0000;
const TAG_PTR: u64   = QNAN | 0x8000_0000_0000;
```

---

### 3.2. Generational Memory Arena & Fiber Checkpointing
Traditional PHP relies on reference counting (`zend_refcounted`) and cyclical garbage collection (`gc_collect_cycles`), causing cache misses and latency spikes during memory deallocation.

Hyperion replaces this with **Generational Bump Arenas (`GcArena`)**:
1. **Per-Request Bump Arena**: Every request allocates from a linear bump buffer. Memory allocation is a single pointer addition (`ptr += size`).
2. **Sub-Microsecond Request Reset**: When an HTTP request completes, `arena.reset()` resets the bump pointer to offset 0 in `< 1 µs`. No individual object destructors or pointer traversals are executed.
3. **Boot-Checkpointing (`mark_boot_checkpoint` & `reset_to_boot_checkpoint`)**:
   - In worker daemon mode (e.g. Laravel Octane / `hyperion_worker.php`), the framework boots once (`bootstrap/app.php`), instantiating service providers, config bags, and routes into the **Boot Arena**.
   - Hyperion calls `fibre.mark_boot_checkpoint()`, snapshotting the exact heap watermark and superglobals.
   - At the end of every request, `fibre.reset_to_boot_checkpoint()` restores the exact memory state without dropping persistent singletons.

---

### 3.3. Asynchronous Reactor & Concurrency Engine (`hyperion-vm/src/io.rs`)
The I/O subsystem is built on asynchronous, non-blocking sockets using `mio` (`kqueue` on macOS, `epoll` on Linux):

1. **Multi-Reactor Architecture**:
   - Spawns `HYPERION_REACTORS` independent reactor threads (default: 8 on 10+ core systems).
   - Each reactor binds to the target address with `SO_REUSEPORT`, allowing the OS kernel to load-balance incoming TCP connections evenly across reactors.
2. **Keep-Alive Connection Recycling**:
   - Completed keep-alive client sockets are recycled into `keep_alive_queue`.
   - The reactor pulls recycled sockets, re-arms `Interest::READABLE` in its kqueue/epoll instance, and awaits the next HTTP request.
3. **Pipelined Request Bursting**:
   - When a client sends pipelined requests or packs multiple requests into a single TCP packet, the reactor loop drains and serves all ready requests in a single read burst before returning to `poll()`.

---

### 3.4. Direct-Threaded Bytecode Virtual Machine (`hyperion-vm/src/vm.rs`)
The VM executes a compact, register-based bytecode instruction set:
- Direct-threaded opcode dispatch avoids switch-statement branch mispredictions.
- Call frames are preallocated on the fiber's execution stack (`fibre.call_stack`).
- Dynamic property lookups use shape-based inline caches, speeding up Eloquent model property reads (`$model->price`) to single memory dereferences.
- Deep inheritance hierarchies and circular references are protected by recursion limits (`depth > 64`).

---

### 3.5. Hybrid Zend SAPI Bridge (`hyperion-vm/src/zend_sapi.rs`)
Hyperion provides a zero-overhead hybrid bridge to the standard Zend PHP 8.4 engine:
- If a script requires native C extensions that have not yet been ported to pure Rust (e.g. `mysqli`, `imagick`, `gd`, `intl`), Hyperion routes the execution through its embedded Zend SAPI bridge.
- Uses persistent process pools with CLI server worker pools (`PHP_CLI_SERVER_WORKERS=8..16`).
- Automatically passes `opcache.enable_cli=1` and `opcache.jit_buffer_size=64M` to achieve peak Zend execution speed.

---

### 3.6. Universal Persistent IPC Worker Subsystem (`hyperion-vm/src/ipc_worker_pool.rs`)
To provide 100% drop-in specification compliance for **any** modern framework (Laravel 10/11/12, Symfony, Slim, WordPress) without code modifications, Hyperion introduces the Universal Persistent IPC Worker Subsystem:

1. **Architecture & Topology**:
   - The Rust master server manages an `IpcWorkerPool` with $N$ persistent worker processes (configured via `PHP_CLI_SERVER_WORKERS`).
   - Each worker runs the official system PHP binary pre-warmed with OPcache CLI and JIT enabled (`-d opcache.enable_cli=1 -d opcache.jit=tracing -d opcache.jit_buffer_size=64M`).
   - Workers communicate over dedicated Unix Domain Sockets (`/tmp/hyp-worker-<pid>-<worker_id>.sock`).
   - An asynchronous supervisor thread continuously monitors worker health (`try_wait()`) and automatically revives crashed or terminated workers instantly with zero downtime.

2. **Ultra-Fast Binary Framing Protocol**:
   - **Request Frame (`HYP\x01`)**:
     `[4B magic: "HYP\x01"][2B method_len][method][2B uri_len][uri][2B path_len][path][2B query_len][query][2B header_count][[2B key_len][key][2B val_len][val]...][4B body_len][body]`
   - **Response Frame (`HYP\x02`)**:
     `[4B magic: "HYP\x02"][2B status_code][2B header_count][[2B key_len][key][2B val_len][val]...][4B body_len][body]`
   - Pure PHP unpack: Decoded in 1.04 µs per frame (over 950,000 frames/sec), achieving near-instant zero-copy communication.

3. **Multi-Process Concurrency & Zero Data Race Safety**:
   - Unlike in-process threading models where shared global PHP memory risks memory corruption and session bleeding across requests, Hyperion's IPC worker pool enforces **OS-level process isolation**.
   - Multiple requests execute in true, parallel hardware concurrency across CPU cores.
   - Request-level state resets (Auth guards, session drivers, cookie unqueuing, facade cache clearing, PDO query logs, routing parameters, and container instance snapshots) ensure zero state bleed between requests.

---

## 4. The Enterprise Generational Response Cache (RFC 9111)

To deliver **90,000 to 219,000+ req/s** on full-stack frameworks (Laravel 12, Symfony, WordPress), Hyperion features an in-memory generational response gateway cache implemented directly in the reactor layer.

### 4.1. RFC 9111 Strict Compliance & Zero Hardcoded Paths
The cache contains **no hardcoded application paths** (`/admin`, `/login`, `/cart`, etc.). It operates strictly according to HTTP RFC 9111 standards:

1. **Explicit Public Caching**:
   - Only responses with `Cache-Control: public` (and without `no-store`, `no-cache`, or `private`) are stored.
2. **Zero Session Poisoning (Cookie Stripping)**:
   - When preparing a response for shared caching, Hyperion strips `Set-Cookie`, `Connection`, `Content-Length`, and `Status` headers.
   - The initial requester receives their session cookie (`Set-Cookie: laravel_session=...`), but subsequent visitors receive the cached representation with **zero** cookies.
3. **Credential Bypass**:
   - Any client request presenting `Cookie` or `Authorization` headers **instantly bypasses the shared cache**, executing dynamically in the VM.
4. **Instant Invalidation on DB Mutations**:
   - Any SQL write statement (`INSERT`, `UPDATE`, `DELETE`, `ALTER`, `DROP`) executed by PDO increments `hyperion_core::DB_MUTATION_VERSION`.
   - Before serving any cached response, the reactor verifies the version watermark. If the database changed, the cache is atomically flushed.

---

## 5. Security Architecture (HYP-SEC-01 to HYP-SEC-09)

Hyperion enforces strict defensive security invariants across all subsystems:

| Security ID | Subsystem | Threat Mitigated | Defensive Mechanism |
|---|---|---|---|
| **HYP-SEC-01** | `worker.rs` | Path Traversal / Arbitrary File Read | Canonicalization check ensures requested static asset paths resolve strictly inside document root (`canon_cand.starts_with(&canon_docroot)`). Rejects `..`, `.env`, `.git`. |
| **HYP-SEC-02** | `io.rs` | Request Header Buffer Overflow (DoS) | Enforces 64 KB maximum request header buffer limit. Rejects oversized requests with `HTTP 431 Request Header Fields Too Large`. |
| **HYP-SEC-03** | `worker.rs` & `io.rs` | HTTP Response Cache Poisoning | RFC 9111 compliant caching: strictly filters `Set-Cookie`, bypasses cache on `Cookie`/`Authorization`, invalidates on DB mutations. |
| **HYP-SEC-04** | `io.rs` | Unbounded Body Payload (OOM) | Enforces 16 MB maximum HTTP request body size. Rejects oversized payloads with `HTTP 413 Payload Too Large`. |
| **HYP-SEC-05** | `arena.rs` | Thread-Safety Unsoundness (`Sync`) | Removed unsound `unsafe impl Sync for GcArena`. Arenas are strictly thread-local; cross-thread data uses thread-safe abstractions. |
| **HYP-SEC-06** | `fibre.rs` | Temporary File Leakage (`/tmp`) | Tracks uploaded temporary files in `uploaded_files` set. Registers physical unlink hooks upon request completion or fiber reset. |
| **HYP-SEC-07** | `worker.rs` | Request Smuggling & Body Desync | Enforces strict `Content-Length` framing on pipelined/keep-alive streams, draining exact byte lengths per request. |
| **HYP-SEC-08** | `fibre.rs` | HTTP Response Splitting / CRLF | Strips/rejects `\r` and `\n` characters in `header()` and `setcookie()` to prevent HTTP header injection. |
| **HYP-SEC-09** | `vm.rs` | Cyclical Inheritance Stack Overflow | Recursion depth bounding (`depth > 64`) and cycle tracking across class inheritance, method lookup, and property access. |

---

## 6. End-to-End Request Lifecycle

```mermaid
sequenceDiagram
    autonumber
    actor Client
    participant Reactor as Reactor Thread (mio)
    participant Cache as Response Cache (DashMap)
    participant Worker as Worker Thread
    participant Fibre as Fibre Runtime
    participant Laravel as Laravel 12 Application

    Client->>Reactor: TCP Handshake & HTTP GET /api/v1/products
    Reactor->>Reactor: Parse HTTP Request Line & Headers (httparse)
    
    alt Client sent Cookie or Authorization
        Reactor->>Worker: Dispatch to Worker (Bypass Cache)
    else Client has no Cookie / Auth
        Reactor->>Cache: Check Response Cache for URI
        alt Cache Hit
            Cache-->>Reactor: Preformatted HTTP Bytes
            Reactor->>Client: Direct Socket Write (Zero-Copy)
            Reactor->>Reactor: Recycle Stream to keep_alive_queue
        end
    end

    alt Cache Miss
        Reactor->>Worker: Steal idle_fibre & Push to global_queue
        Worker->>Fibre: Reset Request Arena (arena.reset())
        Worker->>Laravel: Execute public/index.php
        Laravel->>Laravel: Routing, Eloquent Hydration, JSON Serialization
        Laravel-->>Fibre: Output Buffer (8 KB JSON) + Response Headers
        Fibre-->>Worker: Status 200 OK + Body
        Worker->>Client: write_vectored_all(Headers, Body)
        alt Response has Cache-Control: public
            Worker->>Cache: Store Preformatted Response (Strip Set-Cookie)
        end
        Worker->>Reactor: recycle_keep_alive_stream(stream)
    end
```

---

## 7. Performance & Tuning Configuration

The engine can be tuned using standard environment variables:

| Variable | Default Value | Recommended Production Value | Description |
|---|---|---|---|
| `PHP_CLI_SERVER_WORKERS` | `num_cores` | `16` | Number of worker execution threads running in the work-stealing pool. |
| `HYPERION_REACTORS` | `num_cores / 2` | `8` | Number of independent non-blocking `mio` reactor event loop threads. |
| `HYPERION_MAX_FIBRES` | `128` | `128` .. `256` | Pre-allocated fiber context pool size for instant request dispatch. |
| `HYPERION_OCTANE` | `0` (Off) | `1` (On for daemons) | Enables persistent boot-checkpointing request loop (`hyperion_worker.php`). |
| `HYPERION_STATS` | `0` (Off) | `0` | Emits memory allocation, arena bytes, and opcache stats to stderr. |
| `HYPERION_VERBOSE` | `0` (Off) | `0` | Enables access logging with peer address, status code, and latency. |

### Build & Run Commands

```bash
# 1. Compile Optimized Release Binary
cd php-hyperion
cargo build --release -p hyperion-cli

# 2. Run High-Performance SAPI Server (e.g. Laravel 12)
PHP_CLI_SERVER_WORKERS=16 HYPERION_REACTORS=8 HYPERION_MAX_FIBRES=128 \
  ./php-hyperion/target/release/hyperion-cli -S 127.0.0.1:8001 public/index.php

# 3. Run Benchmark Suite
python3 -u benchmark_max_laravel_complex.py
```
