# 🛡️ PHP-Hyperion Security Policy & Enterprise Hardening Specifications

PHP-Hyperion is architected from the ground up with a **Zero-Trust, Multi-Layer Defense-in-Depth** model designed to run high-concurrency Laravel, Symfony, WordPress, and native PHP applications safely in mission-critical enterprise environments.

---

## 1. Supported Versions

| Version | Engine Core | Persistent IPC Subsystem | Security Status |
| :--- | :--- | :--- | :--- |
| **1.0.x (Current)** | Rust Epoll / Kqueue Fibre Engine | Zend PHP 8.2 / 8.3 / 8.4 Daemon Pool | 🛡️ Active Enterprise Support |
| **< 1.0** | Legacy Prototype | Embedded / CGI fallback | ❌ Deprecated (Upgrade recommended) |

---

## 2. Security Architecture & Threat Defense Matrix

PHP-Hyperion enforces strict defensive boundaries at both the network I/O layer (Rust) and the PHP application execution layer (`hyperion_worker.php`):

| Invariant ID | Security Threat Vector | Defensive Mechanism Enforced | Status |
| :--- | :--- | :--- | :--- |
| **HYP-SEC-01** | **Directory Traversal & Sensitive File Leakage** | Paths with `..`, `\0`, backslashes, or hidden dotfiles (`.env`, `.git`, `.lock`, `.sqlite`, `.yml`, `.sh`) are strictly blocked before execution. Static files are restricted to a strict whitelist with `X-Content-Type-Options: nosniff`. | ✅ 100% Enforced |
| **HYP-SEC-02** | **Header Buffer Overflow & Slowloris DoS** | Request headers bounded to **64 KB**; oversized headers rejected immediately with `HTTP 431 Request Header Fields Too Large` without worker allocation or memory growth. | ✅ 100% Enforced |
| **HYP-SEC-03** | **RFC 9111 Shared Cache & Session Poisoning** | Response cache strictly verifies RFC 9111 cacheability. Requests with `Cookie` or `Authorization` headers completely bypass cache. `Set-Cookie` headers are stripped from cached entries to guarantee zero session leakage. | ✅ 100% Enforced |
| **HYP-SEC-04** | **Payload Body Denial-of-Service (OOM)** | Request bodies capped at **16 MB** by default; payloads exceeding limit trigger instant `HTTP 413 Payload Too Large` with zero memory allocation. | ✅ 100% Enforced |
| **HYP-SEC-05** | **Zero-Trust Local IPC Socket Sandboxing** | IPC Unix domain sockets reside in `/tmp/hyperion-<uid>` with strict POSIX permissions (`0o700`). Only the executing user ID has access to connect or communicate with worker processes. | ✅ 100% Enforced |
| **HYP-SEC-06** | **Multipart Form-Data Traversal & Null Bytes** | Upload filenames are sanitized with `basename(str_replace(['\0', '\\'], ['', '/'], $filename))` to prevent traversal. File contents preserve binary integrity without stripping trailing newline bytes. | ✅ 100% Enforced |
| **HYP-SEC-07** | **HTTP Pipelining & Request Smuggling** | Request framing strictly validates `Content-Length` and delimiter boundaries, preventing desynchronization attacks across keep-alive streams. | ✅ 100% Enforced |
| **HYP-SEC-08** | **Response Splitting & CRLF Injection** | IPC header serialization explicitly frames header key-value lengths via binary uint16 length prefixes, eliminating header injection vulnerabilities. | ✅ 100% Enforced |
| **HYP-SEC-09** | **Information Disclosure on Exceptions** | IPC worker catch blocks respect `APP_DEBUG` settings. In production (`APP_DEBUG=false`), internal stack traces and server errors are concealed behind standard `500 Internal Server Error` and `502 Bad Gateway`. | ✅ 100% Enforced |

---

## 3. Persistent Worker State Isolation

In persistent execution mode (e.g. Laravel 10/11/12 Octane-style, Symfony 7, Slim 4), worker memory persists across requests. Hyperion eliminates cross-request state bleeding via:

1. **Superglobals Sanitation:** `$_GET`, `$_POST`, `$_FILES`, `$_COOKIE`, `$_REQUEST`, and `$_SERVER` are reset from scratch on every incoming request.
2. **Session Cleanup:** `$_SESSION` and active session handlers are closed and wiped before new request processing.
3. **Database Transaction Safeguard:** Uncommitted database transactions (`transactionLevel > 0`) are rolled back automatically, and query logs are flushed to avoid memory bloat.
4. **Authentication & Session Stores:** Resolved user guards (`Auth::guard()`), queued cookies, and scoped container singletons are purged between requests.
5. **Temporary Files Garbage Collection:** Any files uploaded during multipart requests are recorded and cleanly removed via `@unlink()` after request termination.
6. **Max Request & Memory Limits:** Workers automatically cycle after reaching `HYPERION_MAX_REQUESTS` (default: 10,000) or `HYPERION_MAX_MEMORY_MB` (default: 512 MB) to prevent fragmentation leaks.

---

## 4. Reporting a Vulnerability

We take the security of PHP-Hyperion seriously. If you discover a security vulnerability, please report it responsibly:

- **Email:** `security@hyperion-engine.io` (or create a private GitHub Security Advisory).
- **Response SLA:** Within 24 hours of report receipt.
- **Triage & Patching:** High-severity issues are patched and released within 48 to 72 hours.
- **Public Disclosure:** Vulnerabilities will only be publicly disclosed after an advisory and patched release are published.
