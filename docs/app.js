/**
 * PHP-Hyperion Documentation Portal Engine
 * Interactive Search, Benchmark Calculator, Tab Switching, and Terminal Simulator
 */

document.addEventListener('DOMContentLoaded', () => {
    initTabs();
    initCopyButtons();
    initBenchmarkSlider();
    initTerminalSimulator();
    initScrollSpy();
    initSearch();
    initFaqAccordion();
    initMobileSidebar();
});

// 1. Tab Switching System
function initTabs() {
    document.querySelectorAll('.tabs-container').forEach(container => {
        const buttons = container.querySelectorAll('.tab-btn');
        const contents = container.querySelectorAll('.tab-content');

        buttons.forEach(btn => {
            btn.addEventListener('click', () => {
                const targetId = btn.getAttribute('data-tab');

                buttons.forEach(b => b.classList.remove('active'));
                contents.forEach(c => c.classList.remove('active'));

                btn.classList.add('active');
                const targetContent = container.querySelector(`#${targetId}`);
                if (targetContent) {
                    targetContent.classList.add('active');
                }
            });
        });
    });
}

// 2. Copy Code to Clipboard
function initCopyButtons() {
    document.querySelectorAll('.btn-code-copy, .btn-copy-small').forEach(btn => {
        btn.addEventListener('click', async () => {
            const targetSelector = btn.getAttribute('data-copy-target');
            let textToCopy = '';

            if (targetSelector) {
                const targetEl = document.querySelector(targetSelector);
                textToCopy = targetEl ? targetEl.innerText.trim() : '';
            } else {
                const codeEl = btn.closest('.code-wrapper')?.querySelector('pre code');
                textToCopy = codeEl ? codeEl.innerText.trim() : '';
            }

            if (!textToCopy) return;

            try {
                await navigator.clipboard.writeText(textToCopy);
                const originalText = btn.innerText;
                btn.innerText = '✓ Copied!';
                btn.style.color = '#34d399';
                setTimeout(() => {
                    btn.innerText = originalText;
                    btn.style.color = '';
                }, 2000);
            } catch (err) {
                console.error('Failed to copy', err);
            }
        });
    });
}

// 3. Interactive Benchmark Visualizer
function initBenchmarkSlider() {
    const slider = document.getElementById('bench-concurrency-slider');
    const valDisplay = document.getElementById('bench-concurrency-val');
    if (!slider || !valDisplay) return;

    const barHyperionCache = document.getElementById('bar-hyperion-cache');
    const barHyperionDyn = document.getElementById('bar-hyperion-dyn');
    const barPhpJit = document.getElementById('bar-php-jit');
    const barPhpFpm = document.getElementById('bar-php-fpm');

    const txtHyperionCache = document.getElementById('txt-hyperion-cache');
    const txtHyperionDyn = document.getElementById('txt-hyperion-dyn');
    const txtPhpJit = document.getElementById('txt-php-jit');
    const txtPhpFpm = document.getElementById('txt-php-fpm');

    function updateBenchmark() {
        const c = parseInt(slider.value, 10);
        valDisplay.innerText = `c = ${c}`;

        // Dynamic formula based on real benchmarks
        // Hyperion Fast-Path scales up to 215k req/s
        let hypCache = Math.round(185000 + (c * 230));
        if (hypCache > 218500) hypCache = 218500;

        // Hyperion Dynamic IPC scales with concurrency
        let hypDyn = Math.round(62000 + (c * 335));
        if (hypDyn > 95200) hypDyn = 95200;

        // PHP 8.4 JIT saturates around 1,450 req/s
        let phpJit = Math.min(1450, Math.round(420 + (c * 9)));

        // PHP-FPM saturates around 380 req/s
        let phpFpm = Math.min(380, Math.round(280 + (c * 1.5)));

        // Percentages relative to peak 220,000
        const maxScale = 220000;
        const pctCache = Math.min(100, (hypCache / maxScale) * 100);
        const pctDyn = Math.min(100, (hypDyn / maxScale) * 100);
        const pctJit = Math.max(3, (phpJit / maxScale) * 100);
        const pctFpm = Math.max(2, (phpFpm / maxScale) * 100);

        if (barHyperionCache) barHyperionCache.style.width = `${pctCache}%`;
        if (barHyperionDyn) barHyperionDyn.style.width = `${pctDyn}%`;
        if (barPhpJit) barPhpJit.style.width = `${pctJit}%`;
        if (barPhpFpm) barPhpFpm.style.width = `${pctFpm}%`;

        if (txtHyperionCache) txtHyperionCache.innerText = `${hypCache.toLocaleString()} req/s (0.48 ms)`;
        if (txtHyperionDyn) txtHyperionDyn.innerText = `${hypDyn.toLocaleString()} req/s (1.05 ms)`;
        if (txtPhpJit) txtPhpJit.innerText = `${phpJit.toLocaleString()} req/s (18.2 ms)`;
        if (txtPhpFpm) txtPhpFpm.innerText = `${phpFpm.toLocaleString()} req/s (68.4 ms)`;
    }

    slider.addEventListener('input', updateBenchmark);
    updateBenchmark();
}

// 4. Interactive Terminal Simulator
function initTerminalSimulator() {
    const input = document.getElementById('term-cmd-input');
    const historyContainer = document.getElementById('term-history');
    if (!input || !historyContainer) return;

    input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
            const cmd = input.value.trim();
            if (!cmd) return;

            appendTermLine(`$ ${cmd}`, 'accent');
            processCommand(cmd);
            input.value = '';
            historyContainer.scrollTop = historyContainer.scrollHeight;
        }
    });

    function appendTermLine(text, type = '') {
        const div = document.createElement('div');
        div.className = `term-output ${type}`;
        div.innerText = text;
        historyContainer.appendChild(div);
    }

    function processCommand(raw) {
        const cmd = raw.toLowerCase().trim();

        if (cmd === 'clear') {
            historyContainer.innerHTML = '';
            return;
        }

        if (cmd === 'help') {
            appendTermLine(
                `Available simulated commands:\n` +
                `  hyperion -v                Show version & compiler specs\n` +
                `  hyperion -S 127.0.0.1:8000 Start high-concurrency SAPI server\n` +
                `  hyperion --bench           Run quick simulated ApacheBench battery\n` +
                `  hyperion --security        Verify HYP-SEC-01..09 security invariants\n` +
                `  clear                      Clear terminal history`,
                'warning'
            );
            return;
        }

        if (cmd.includes('-v') || cmd.includes('--version')) {
            appendTermLine(
                `⚡ PHP-Hyperion v0.1.0 (Darwin ARM64 Apple Silicon)\n` +
                `   Compiler: rustc 1.82 (Direct-Threaded Bytecode & M:N Fibre Runtime)\n` +
                `   Protocol: HYP/1.0 Binary IPC framing (1.04 µs unpack)\n` +
                `   Target: PHP 8.2 / 8.4 Specification Compliant`,
                'success'
            );
            return;
        }

        if (cmd.includes('-s') || cmd.includes('serve')) {
            appendTermLine(
                `[HYPERION_BOOT] Detected framework: Laravel 12 Enterprise\n` +
                `[HYPERION_BOOT] Spawning 16 persistent IPC workers (OPcache JIT enabled)...\n` +
                `⚡ Hyperion Persistent IPC Worker Pool initialized (16 workers, UDS /tmp/hyp-worker-*.sock)\n` +
                `[208,000 req/s reactor online] Listening on http://127.0.0.1:8000 (SO_REUSEPORT)`,
                'success'
            );
            return;
        }

        if (cmd.includes('--bench')) {
            appendTermLine(
                `⚡ Maximum Head-to-Head Benchmark (Laravel 12.32 Complex Enterprise):\n` +
                `Workload / Endpoint               | Standard PHP | PHP-Hyperion | Speedup\n` +
                `----------------------------------+--------------+--------------+--------\n` +
                `Framework Ingress (/health)       |    333 req/s | 70,791 req/s | 212.3x 🚀\n` +
                `SQL Aggregates (HAVING / Group)   |    344 req/s |  5,655 req/s |  16.4x 🚀\n` +
                `Subquery Joins (joinSub)          |    352 req/s |  4,567 req/s |  12.9x 🚀\n` +
                `Window Functions (ROW_NUMBER)     |    337 req/s |  3,514 req/s |  10.4x 🚀\n` +
                `Blade Dashboard (/complex)        |    279 req/s |  1,243 req/s |   4.4x 🚀\n` +
                `Storefront Catalog (/)            |    243 req/s |    908 req/s |   3.7x 🚀\n` +
                `----------------------------------+--------------+--------------+--------\n` +
                `Peak Concurrency (c=200): 209,234 req/s (0.96 ms latency, 0 failures)\n` +
                `Memory RSS Footprint: Hyperion 171.7 MB vs PHP 375.1 MB (54.2% Less RAM)`,
                'success'
            );
            return;
        }

        if (cmd.includes('--security')) {
            appendTermLine(
                `🛡️ PHP-Hyperion Zero-Trust Defense Verification Audit:\n` +
                `  [HYP-SEC-01] Path Traversal & Dotfile Access      : PASSED (Jailed in docroot)\n` +
                `  [HYP-SEC-02] 64 KB Header Overflow / Slowloris     : PASSED (HTTP 431 rejected)\n` +
                `  [HYP-SEC-03] RFC 9111 Session / Cookie Poisoning   : PASSED (Set-Cookie stripped)\n` +
                `  [HYP-SEC-04] 16 MB Payload Overflow Limit          : PASSED (HTTP 413 bounded)\n` +
                `  [HYP-SEC-05] Zero-Trust IPC Socket Directory       : PASSED (POSIX 0o700 UID Jail)\n` +
                `  [HYP-SEC-06] Multipart Upload Traversal & Integrity: PASSED (Sanitized & Preserved)\n` +
                `  [HYP-SEC-07] HTTP Pipelining & Request Smuggling   : PASSED (Zero Stream Desync)\n` +
                `  [HYP-SEC-08] CRLF Injection & Header Splitting     : PASSED (Binary Length Framed)\n` +
                `  [HYP-SEC-09] Uncaught Exception Masking            : PASSED (APP_DEBUG protected)\n` +
                `Audit Result: 9/9 Invariants 100% Passed. Zero State Bleeding. Production Certified.`,
                'success'
            );
            return;
        }

        if (cmd.includes('--routes')) {
            appendTermLine(
                `⚡ Hyperion Kernel Fast-Routing Table (Pre-Compiled Route Cache):\n` +
                `  Method | Path                    | Target Handler                   | Cache Policy\n` +
                `  -------+-------------------------+----------------------------------+-------------------\n` +
                `  GET    | /health                 | FrameworkHealthProbe             | public, max-age=5\n` +
                `  GET    | /api/v1/users           | App\\Http\\Controllers\\UserController | private, no-store\n` +
                `  GET    | /public-stats           | StatsAggregateController         | public, max-age=3600\n` +
                `  POST   | /api/v1/auth/login      | AuthController::login            | private (Sets cookie)\n` +
                `  POST   | /api/v1/files/upload    | UploadController::store          | multipart/form-data\n` +
                `Total routes registered: 48 (Dispatched via Direct Memory Lookups in 0.04 µs)`,
                'accent'
            );
            return;
        }

        if (cmd.includes('--status') || cmd.includes('status')) {
            appendTermLine(
                `⚡ PHP-Hyperion Daemon Status:\n` +
                `  State             : RUNNING (PID: 49201)\n` +
                `  Reactors          : 8 async mio loops (SO_REUSEPORT enabled)\n` +
                `  Worker Pool       : 16 persistent IPC workers (/tmp/hyp-worker-*.sock)\n` +
                `  Supervisor        : Watchdog active (interval: 500 ms, crash revive: ~15 ms)\n` +
                `  Active Sockets    : 1,024 TCP connections open (0 drops)\n` +
                `  Total Requests    : 1,482,904\n` +
                `  Fast-Path Hits    : 1,120,440 (75.5% hit ratio)\n` +
                `  Mean RSS RAM      : 171.7 MB (steady state, zero fragmentation)`,
                'success'
            );
            return;
        }

        if (cmd.includes('--cache-stats') || cmd.includes('cache')) {
            appendTermLine(
                `🗄️ RFC 9111 Generational In-Memory Cache Stats:\n` +
                `  Cache Capacity   : 256 MB (18.4 MB in use)\n` +
                `  Cached Objects   : 342 representations\n` +
                `  Zero-Copy Splice : 208,036 req/s peak delivery rate\n` +
                `  Cookie Stripping : Active (HYP-SEC-03: Set-Cookie eliminated)\n` +
                `  Purge Triggers   : POST/PUT/PATCH/DELETE auto-eviction enabled`,
                'success'
            );
            return;
        }

        if (cmd.includes('cat hyperion.json')) {
            appendTermLine(
                `{\n` +
                `  "server": { "listen": "0.0.0.0:8000", "workers": 16, "reactors": 8 },\n` +
                `  "cache": { "enabled": true, "ttl": 3600, "max_memory_mb": 256 },\n` +
                `  "limits": { "max_request_body_mb": 16, "max_header_kb": 64 },\n` +
                `  "worker": { "max_requests": 10000, "max_memory_mb": 512 },\n` +
                `  "security": { "mask_errors": true, "docroot_jail": true }\n` +
                `}`,
                'accent'
            );
            return;
        }

        appendTermLine(`Command not recognized: '${raw}'. Type 'help' for available commands.`, 'warning');
    }
}

// 5. Scrollspy for Active Sidebar Link
function initScrollSpy() {
    const sections = document.querySelectorAll('.doc-section');
    const navLinks = document.querySelectorAll('.sidebar-link');

    window.addEventListener('scroll', () => {
        let currentSectionId = '';
        const scrollPos = window.scrollY + 140;

        sections.forEach(section => {
            const top = section.offsetTop;
            const height = section.offsetHeight;
            if (scrollPos >= top && scrollPos < top + height) {
                currentSectionId = section.getAttribute('id');
            }
        });

        navLinks.forEach(link => {
            link.classList.remove('active');
            if (link.getAttribute('href') === `#${currentSectionId}`) {
                link.classList.add('active');
            }
        });
    });
}

// 6. Documentation Search Filtering & Slash Shortcut
function initSearch() {
    const searchInput = document.getElementById('doc-search-input');
    if (!searchInput) return;

    // Press '/' key anywhere to focus search
    window.addEventListener('keydown', (e) => {
        if (e.key === '/' && document.activeElement !== searchInput && document.activeElement.tagName !== 'INPUT') {
            e.preventDefault();
            searchInput.focus();
            searchInput.select();
        }
    });

    searchInput.addEventListener('input', (e) => {
        const query = e.target.value.toLowerCase().trim();
        const sections = document.querySelectorAll('.doc-section');

        if (!query) {
            sections.forEach(s => s.style.display = 'block');
            return;
        }

        sections.forEach(section => {
            const text = section.innerText.toLowerCase();
            if (text.includes(query)) {
                section.style.display = 'block';
            } else {
                section.style.display = 'none';
            }
        });
    });
}

// 7. FAQ Accordion Handler
function initFaqAccordion() {
    document.querySelectorAll('.faq-item .faq-question').forEach(header => {
        header.addEventListener('click', () => {
            const item = header.closest('.faq-item');
            if (item) {
                item.classList.toggle('open');
            }
        });
    });
}

// 8. Mobile Sidebar Toggle & Auto-Close on Navigation
function initMobileSidebar() {
    const toggleBtn = document.getElementById('btn-sidebar-toggle');
    const sidebar = document.getElementById('docs-sidebar');
    if (!toggleBtn || !sidebar) return;

    toggleBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        sidebar.classList.toggle('mobile-open');
    });

    // Close sidebar on link click
    sidebar.querySelectorAll('.sidebar-link').forEach(link => {
        link.addEventListener('click', () => {
            sidebar.classList.remove('mobile-open');
        });
    });

    // Close when clicking outside on mobile
    document.addEventListener('click', (e) => {
        if (!sidebar.contains(e.target) && !toggleBtn.contains(e.target)) {
            sidebar.classList.remove('mobile-open');
        }
    });
}


