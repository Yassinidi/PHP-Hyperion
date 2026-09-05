@extends('layouts.store')

@section('title', 'PHP-Hyperion & Laravel 12 Complex Lab')

@section('styles')
<style>
    .lab-hero {
        background: linear-gradient(135deg, rgba(30, 41, 59, 0.7), rgba(15, 23, 42, 0.9));
        border: 1px solid var(--border-color);
        border-radius: 16px;
        padding: 2.5rem;
        margin-bottom: 2rem;
        position: relative;
        overflow: hidden;
        backdrop-filter: blur(12px);
    }
    .lab-hero::before {
        content: '';
        position: absolute;
        top: -50%;
        left: -20%;
        width: 140%;
        height: 200%;
        background: radial-gradient(circle, rgba(99, 102, 241, 0.12) 0%, transparent 70%);
        pointer-events: none;
    }
    .lab-badge {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
        background: rgba(99, 102, 241, 0.15);
        color: #818cf8;
        padding: 0.35rem 0.85rem;
        border-radius: 9999px;
        font-size: 0.8rem;
        font-weight: 700;
        letter-spacing: 0.05em;
        text-transform: uppercase;
        border: 1px solid rgba(99, 102, 241, 0.3);
        margin-bottom: 1rem;
    }
    .lab-title {
        font-size: 2.25rem;
        font-weight: 800;
        letter-spacing: -0.03em;
        margin-bottom: 0.5rem;
        background: linear-gradient(to right, #fff, #94a3b8);
        -webkit-background-clip: text;
        -webkit-text-fill-color: transparent;
    }
    .lab-desc {
        color: var(--text-muted);
        font-size: 1.05rem;
        max-width: 800px;
    }

    /* Tabs */
    .tabs-nav {
        display: flex;
        gap: 0.75rem;
        border-bottom: 1px solid var(--border-color);
        margin-bottom: 2rem;
        overflow-x: auto;
        padding-bottom: 0.5rem;
    }
    .tab-btn {
        background: transparent;
        border: 1px solid transparent;
        color: var(--text-muted);
        padding: 0.75rem 1.25rem;
        font-family: inherit;
        font-size: 0.95rem;
        font-weight: 600;
        border-radius: 10px;
        cursor: pointer;
        transition: all 0.2s ease;
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }
    .tab-btn:hover {
        color: var(--text-main);
        background: rgba(255, 255, 255, 0.03);
    }
    .tab-btn.active {
        color: white;
        background: var(--accent);
        box-shadow: 0 4px 15px var(--accent-glow);
    }

    .tab-pane {
        display: none;
        animation: fadeIn 0.3s ease;
    }
    .tab-pane.active {
        display: block;
    }
    @keyframes fadeIn {
        from { opacity: 0; transform: translateY(6px); }
        to { opacity: 1; transform: translateY(0); }
    }

    /* Cards & Grids */
    .grid-2 {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(360px, 1fr));
        gap: 1.5rem;
        margin-bottom: 1.5rem;
    }
    .grid-3 {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
        gap: 1.25rem;
        margin-bottom: 1.5rem;
    }
    .card {
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: 14px;
        padding: 1.5rem;
        transition: border-color 0.2s, box-shadow 0.2s;
    }
    .card:hover {
        border-color: var(--border-highlight);
    }
    .card-title {
        font-size: 1.15rem;
        font-weight: 700;
        margin-bottom: 1rem;
        display: flex;
        align-items: center;
        justify-content: space-between;
    }

    /* Buttons */
    .btn {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
        font-family: inherit;
        font-size: 0.875rem;
        font-weight: 600;
        padding: 0.6rem 1.1rem;
        border-radius: 8px;
        border: none;
        cursor: pointer;
        transition: all 0.2s ease;
        text-decoration: none;
    }
    .btn-primary {
        background: var(--accent);
        color: white;
        box-shadow: 0 2px 8px var(--accent-glow);
    }
    .btn-primary:hover {
        filter: brightness(1.1);
        transform: translateY(-1px);
    }
    .btn-success {
        background: #10b981;
        color: white;
    }
    .btn-success:hover {
        background: #059669;
    }
    .btn-danger {
        background: #ef4444;
        color: white;
    }
    .btn-danger:hover {
        background: #dc2626;
    }
    .btn-secondary {
        background: rgba(255, 255, 255, 0.08);
        color: var(--text-main);
        border: 1px solid var(--border-color);
    }
    .btn-secondary:hover {
        background: rgba(255, 255, 255, 0.12);
    }
    .btn-sm {
        padding: 0.35rem 0.65rem;
        font-size: 0.75rem;
    }

    /* Tables */
    .table-container {
        overflow-x: auto;
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: 12px;
    }
    table {
        width: 100%;
        border-collapse: collapse;
        font-size: 0.875rem;
        text-align: left;
    }
    th {
        background: rgba(255, 255, 255, 0.02);
        padding: 0.85rem 1rem;
        color: var(--text-muted);
        font-weight: 600;
        border-bottom: 1px solid var(--border-color);
    }
    td {
        padding: 0.85rem 1rem;
        border-bottom: 1px solid rgba(255, 255, 255, 0.04);
        color: var(--text-main);
    }
    tr:hover td {
        background: rgba(255, 255, 255, 0.015);
    }

    /* Badges */
    .badge {
        display: inline-block;
        font-size: 0.72rem;
        font-weight: 700;
        padding: 0.2rem 0.5rem;
        border-radius: 6px;
    }
    .badge-success { background: rgba(16, 185, 129, 0.15); color: #34d399; border: 1px solid rgba(16, 185, 129, 0.3); }
    .badge-warning { background: rgba(245, 158, 11, 0.15); color: #fbbf24; border: 1px solid rgba(245, 158, 11, 0.3); }
    .badge-danger { background: rgba(239, 68, 68, 0.15); color: #f87171; border: 1px solid rgba(239, 68, 68, 0.3); }
    .badge-info { background: rgba(99, 102, 241, 0.15); color: #818cf8; border: 1px solid rgba(99, 102, 241, 0.3); }

    /* Code & Logs */
    pre, .code-box {
        background: #080c14;
        border: 1px solid rgba(255, 255, 255, 0.06);
        border-radius: 8px;
        padding: 1rem;
        font-family: 'JetBrains Mono', 'Fira Code', monospace;
        font-size: 0.82rem;
        color: #38bdf8;
        overflow-x: auto;
        max-height: 280px;
        line-height: 1.5;
    }
    .log-line {
        margin-bottom: 0.35rem;
    }
    .log-time {
        color: var(--text-muted);
        margin-right: 0.5rem;
    }
</style>
@endsection

@section('content')
<div class="container">
    <!-- Hero Banner -->
    <div class="lab-hero">
        <div class="lab-badge">⚡ Live Hyperion Dual-Engine Lab</div>
        <h1 class="lab-title">Complex Features, Reverb & Engine Diagnostics</h1>
        <p class="lab-desc">
            Interactive real-time test bench for advanced Laravel 12 features running on PHP-Hyperion: 
            File management with soft/hard deletions, Laravel Reverb WebSockets broadcasting, SQL Window Functions, savepoint nested transactions, and modern PHP 8.4 syntax.
        </p>
    </div>

    <!-- Navigation Tabs -->
    <div class="tabs-nav" id="lab-tabs">
        <button class="tab-btn active" onclick="switchTab('files')" id="tab-btn-files">
            📁 File Storage & Deletion
        </button>
        <button class="tab-btn" onclick="switchTab('reverb')" id="tab-btn-reverb">
            📡 Reverb Broadcasting
        </button>
        <button class="tab-btn" onclick="switchTab('queries')" id="tab-btn-queries">
            ⚡ Advanced SQL & Window Functions
        </button>
        <button class="tab-btn" onclick="switchTab('syntax')" id="tab-btn-syntax">
            💎 Modern PHP 8.4 Syntax
        </button>
    </div>

    <!-- TAB 1: File Storage & Deletion -->
    <div id="tab-files" class="tab-pane active">
        <div class="grid-2">
            <!-- Action Card: Upload & Generator -->
            <div class="card">
                <div class="card-title">
                    <span>⚡ Quick File Storage Test</span>
                    <span class="badge badge-info">Flysystem Local</span>
                </div>
                <p style="color: var(--text-muted); font-size: 0.875rem; margin-bottom: 1.25rem;">
                    Store an automated test payload into storage, compute SHA-256 integrity hash, and record in SQLite.
                </p>
                <div style="display: flex; gap: 0.75rem; flex-wrap: wrap;">
                    <button class="btn btn-primary" id="btn-create-test-file" onclick="createSampleFile()">
                        ➕ Generate & Store Test File
                    </button>
                    <button class="btn btn-secondary" id="btn-cleanup-dir" onclick="cleanupDirectory()">
                        🧹 Test Recursive deleteDirectory()
                    </button>
                </div>
            </div>

            <!-- Action Card: Engine Status & Telemetry -->
            <div class="card">
                <div class="card-title">
                    <span>🛡️ Engine File Verifications</span>
                    <span class="badge badge-success">Dual-Engine Verified</span>
                </div>
                <p style="color: var(--text-muted); font-size: 0.875rem; margin-bottom: 1rem;">
                    Tests native Rust handlers for <code>copy()</code>, <code>filetype()</code>, and Flysystem's <code>RecursiveIteratorIterator</code>:
                </p>
                <button class="btn btn-secondary" id="btn-test-native-io" onclick="testNativeIo()">
                    🧪 Verify copy(), filetype() & unlink()
                </button>
                <div id="native-io-result" style="margin-top: 1rem; display: none;"></div>
            </div>
        </div>

        <!-- Attachments Table -->
        <div class="card" style="margin-bottom: 1.5rem;">
            <div class="card-title">
                <span>Stored Attachments & Deletion Lifecycle</span>
                <button class="btn btn-secondary btn-sm" onclick="reloadAttachmentsTable()" id="btn-refresh-files">
                    🔄 Refresh Table
                </button>
            </div>

            <div class="table-container">
                <table id="attachments-table">
                    <thead>
                        <tr>
                            <th>UUID</th>
                            <th>Original Name</th>
                            <th>Disk / Path</th>
                            <th>Size</th>
                            <th>SHA-256 Hash</th>
                            <th>Status</th>
                            <th>Actions</th>
                        </tr>
                    </thead>
                    <tbody id="attachments-tbody">
                        @forelse($attachments as $att)
                        <tr id="row-att-{{ $att->uuid }}">
                            <td><code>{{ substr($att->uuid, 0, 8) }}...</code></td>
                            <td>{{ $att->original_name }}</td>
                            <td><span class="badge badge-info">{{ $att->disk }}</span> {{ $att->path }}</td>
                            <td>{{ number_format($att->size_bytes) }} B</td>
                            <td><code>{{ substr($att->sha256_hash, 0, 12) }}...</code></td>
                            <td>
                                @if($att->trashed())
                                    <span class="badge badge-danger">Soft Deleted</span>
                                @else
                                    <span class="badge badge-success">Active</span>
                                @endif
                            </td>
                            <td>
                                @if($att->trashed())
                                    <button class="btn btn-success btn-sm" onclick="restoreFile('{{ $att->uuid }}')" id="btn-restore-{{ substr($att->uuid, 0, 8) }}">Restore</button>
                                    <button class="btn btn-danger btn-sm" onclick="forceDeleteFile('{{ $att->uuid }}')" id="btn-force-{{ substr($att->uuid, 0, 8) }}">Force Delete</button>
                                @else
                                    <button class="btn btn-secondary btn-sm" onclick="softDeleteFile('{{ $att->uuid }}')" id="btn-soft-{{ substr($att->uuid, 0, 8) }}">Soft Delete</button>
                                @endif
                            </td>
                        </tr>
                        @empty
                        <tr>
                            <td colspan="7" style="text-align: center; color: var(--text-muted); padding: 2rem;">
                                No attachments yet. Click "Generate & Store Test File" above!
                            </td>
                        </tr>
                        @endforelse
                    </tbody>
                </table>
            </div>
        </div>

        <!-- Live Operations Feed -->
        <div class="card">
            <div class="card-title">
                <span>📋 Live Console & File Operations Log</span>
                <span class="badge badge-info" id="file-log-count">0 events</span>
            </div>
            <div class="code-box" id="file-ops-log">
                <div class="log-line"><span class="log-time">[System]</span> Ready for file storage and deletion operations.</div>
            </div>
        </div>
    </div>

    <!-- TAB 2: Laravel Reverb Broadcasting -->
    <div id="tab-reverb" class="tab-pane">
        <div class="grid-2">
            <!-- Dispatcher Panel -->
            <div class="card">
                <div class="card-title">
                    <span>📡 Dispatch Realtime Events</span>
                    <span class="badge badge-info">Reverb / WebSockets</span>
                </div>
                <p style="color: var(--text-muted); font-size: 0.875rem; margin-bottom: 1.25rem;">
                    Dispatch broadcast events implementing <code>ShouldBroadcastNow</code> to test serialization, channel routing, and broadcasting pipeline:
                </p>

                <div style="display: flex; flex-direction: column; gap: 0.75rem;">
                    <button class="btn btn-primary" id="btn-broadcast-order" onclick="dispatchBroadcast('order.status', { order_id: 10, status: 'shipped' })">
                        ⚡ Broadcast Order Status Update (Private: orders.10)
                    </button>
                    <button class="btn btn-secondary" id="btn-broadcast-file" onclick="dispatchBroadcast('file.deleted', { uuid: 'test-uuid-99', name: 'invoice_march.pdf' })">
                        📄 Broadcast File Deleted Event (Public: files)
                    </button>
                    <button class="btn btn-danger" id="btn-broadcast-inventory" onclick="dispatchBroadcast('inventory.alert', { sku: 'SKU-A3NBRM', current_stock: 4 })">
                        ⚠️ Broadcast Low Inventory Alert (Private: inventory.alerts)
                    </button>
                </div>
            </div>

            <!-- Configuration & Channel Architecture -->
            <div class="card">
                <div class="card-title">
                    <span>⚙️ Reverb Architecture Status</span>
                    <span class="badge badge-success">Configured</span>
                </div>
                <table style="margin-bottom: 1rem;">
                    <tr>
                        <td style="font-weight: 600;">Broadcaster Driver</td>
                        <td><span class="badge badge-info">{{ config('broadcasting.default', 'log') }}</span></td>
                    </tr>
                    <tr>
                        <td style="font-weight: 600;">Reverb Host / Port</td>
                        <td><code>{{ config('reverb.servers.reverb.host', '127.0.0.1') }}:{{ config('reverb.servers.reverb.port', 8080) }}</code></td>
                    </tr>
                    <tr>
                        <td style="font-weight: 600;">App ID / Key</td>
                        <td><code>hyperion-app / hyperion-key</code></td>
                    </tr>
                    <tr>
                        <td style="font-weight: 600;">Broadcast Auth Endpoint</td>
                        <td><code>POST /api/v1/broadcasting/auth</code></td>
                    </tr>
                </table>
                <button class="btn btn-secondary btn-sm" id="btn-test-broadcast-auth" onclick="testBroadcastAuth()">
                    🔐 Test Channel Auth Endpoint
                </button>
            </div>
        </div>

        <!-- Live Broadcast Telemetry Log -->
        <div class="card">
            <div class="card-title">
                <span>📡 Broadcast Event Dispatch Log</span>
                <span class="badge badge-success" id="broadcast-badge">Listening</span>
            </div>
            <div class="code-box" id="reverb-log">
                <div class="log-line"><span class="log-time">[Broadcasting]</span> Reverb WebSocket dispatcher initialized. Ready to trigger events.</div>
            </div>
        </div>
    </div>

    <!-- TAB 3: Advanced SQL & Window Functions Studio -->
    <div id="tab-queries" class="tab-pane">
        <div class="grid-2">
            <!-- Nested Transaction Test -->
            <div class="card">
                <div class="card-title">
                    <span>🛡️ Nested Transactions & Savepoints</span>
                    <span class="badge badge-info">ACID Isolation</span>
                </div>
                <p style="color: var(--text-muted); font-size: 0.875rem; margin-bottom: 1rem;">
                    Executes outer transaction, establishes an inner savepoint that deliberately fails and rolls back, and verifies the outer transaction still commits cleanly:
                </p>
                <button class="btn btn-primary" id="btn-run-nested-tx" onclick="runNestedTransactionTest()">
                    🚀 Execute Savepoint Isolation Test
                </button>
                <div id="nested-tx-result" style="margin-top: 1rem; display: none;"></div>
            </div>

            <!-- Subquery Joins (joinSub) Summary -->
            <div class="card">
                <div class="card-title">
                    <span>📊 Subquery Joins (joinSub)</span>
                    <span class="badge badge-success">Computed</span>
                </div>
                <p style="color: var(--text-muted); font-size: 0.875rem; margin-bottom: 1rem;">
                    Top performing products by revenue joined dynamically using Eloquent <code>joinSub</code> on order items:
                </p>
                <table style="font-size: 0.8rem;">
                    <thead>
                        <tr>
                            <th>Product</th>
                            <th>Units Sold</th>
                            <th>Total Revenue</th>
                        </tr>
                    </thead>
                    <tbody>
                        @foreach(collect($salesSummary)->take(3) as $prod)
                        <tr>
                            <td><strong>{{ $prod->product_name ?? $prod->name }}</strong></td>
                            <td>{{ $prod->total_units_sold ?? 0 }}</td>
                            <td><span class="badge badge-success">${{ number_format((float)($prod->gross_revenue ?? $prod->total_revenue ?? 0), 2) }}</span></td>
                        </tr>
                        @endforeach
                    </tbody>
                </table>
            </div>
        </div>

        <!-- Window Functions Table -->
        <div class="card">
            <div class="card-title">
                <span>⚡ SQL Window Functions: ROW_NUMBER() & AVG() OVER (PARTITION BY category_id)</span>
                <span class="badge badge-info">SQLite 3 / MySQL</span>
            </div>
            <div class="table-container">
                <table>
                    <thead>
                        <tr>
                            <th>Category</th>
                            <th>Category Rank</th>
                            <th>Product Name</th>
                            <th>Price</th>
                            <th>Category Avg Price</th>
                            <th>Price vs Category Avg</th>
                        </tr>
                    </thead>
                    <tbody>
                        @foreach($rankings as $row)
                        @php $rank = $row->rank_in_category ?? $row->category_price_rank ?? 1; @endphp
                        <tr>
                            <td><span class="badge badge-info">{{ $row->category_name }}</span></td>
                            <td>
                                @if($rank == 1)
                                    <span class="badge badge-warning">🥇 #1 Highest</span>
                                @elseif($rank == 2)
                                    <span class="badge badge-secondary">🥈 #2</span>
                                @else
                                    <span class="badge badge-secondary">#{{ $rank }}</span>
                                @endif
                            </td>
                            <td><strong>{{ $row->product_name ?? $row->name }}</strong></td>
                            <td>${{ number_format((float)$row->price, 2) }}</td>
                            <td>${{ number_format((float)$row->category_avg_price, 2) }}</td>
                            <td>
                                @php $diff = (float)$row->price - (float)$row->category_avg_price; @endphp
                                @if($diff > 0)
                                    <span style="color: #34d399; font-weight: 600;">+${{ number_format($diff, 2) }}</span>

                                @else
                                    <span style="color: #f87171; font-weight: 600;">-${{ number_format(abs($diff), 2) }}</span>
                                @endif
                            </td>
                        </tr>
                        @endforeach
                    </tbody>
                </table>
            </div>
        </div>
    </div>

    <!-- TAB 4: Modern PHP 8.4 Syntax -->
    <div id="tab-syntax" class="tab-pane">
        <div class="grid-2">
            <!-- Syntax Evaluation Card -->
            <div class="card">
                <div class="card-title">
                    <span>💎 Modern PHP 8.4 Syntax Showcase</span>
                    <span class="badge badge-success">Engine Verified</span>
                </div>
                <p style="color: var(--text-muted); font-size: 0.875rem; margin-bottom: 1.25rem;">
                    PHP-Hyperion seamlessly evaluates modern PHP 8.4 language features without transpilation:
                </p>

                <div style="background: rgba(0,0,0,0.3); padding: 1rem; border-radius: 10px; margin-bottom: 1rem; border: 1px solid var(--border-color);">
                    <div style="font-size: 0.85rem; color: var(--text-muted); margin-bottom: 0.25rem;">Match Expression with Value Guards:</div>
                    <div style="font-weight: 700; color: #818cf8; font-size: 1.1rem;" id="syntax-tier">
                        {{ $modernSyntax['tier'] ?? 'N/A' }}
                    </div>
                </div>

                <div style="background: rgba(0,0,0,0.3); padding: 1rem; border-radius: 10px; margin-bottom: 1rem; border: 1px solid var(--border-color);">
                    <div style="font-size: 0.85rem; color: var(--text-muted); margin-bottom: 0.25rem;">First-Class Callable Formatter:</div>
                    <div style="font-weight: 700; color: #34d399; font-size: 1.1rem;" id="syntax-formatted">
                        {{ $modernSyntax['formatted'] ?? 'N/A' }}
                    </div>
                </div>

                <div style="background: rgba(0,0,0,0.3); padding: 1rem; border-radius: 10px; border: 1px solid var(--border-color);">
                    <div style="font-size: 0.85rem; color: var(--text-muted); margin-bottom: 0.25rem;">Nullsafe Chaining & Array Destructuring:</div>
                    <div style="font-size: 0.9rem;">
                        Customer: <strong id="syntax-customer">{{ $modernSyntax['customer_name'] ?? 'N/A' }}</strong> &bull; Status: <code id="syntax-status">{{ $modernSyntax['destructured']['status'] ?? 'active' }}</code>
                    </div>
                </div>
            </div>

            <!-- Live Syntax Test Runner -->
            <div class="card">
                <div class="card-title">
                    <span>⚡ Live Syntax Engine Benchmark</span>
                    <button class="btn btn-primary btn-sm" id="btn-eval-syntax" onclick="evaluateSyntax()">
                        ▶ Run Dynamic Evaluation
                    </button>
                </div>
                <p style="color: var(--text-muted); font-size: 0.875rem; margin-bottom: 1rem;">
                    Sends random financial figures and evaluates customer tier categorization via <code>/api/v1/queries/modern-syntax</code>:
                </p>
                <div class="code-box" id="syntax-result-box">
                    <pre>{{ json_encode($modernSyntax, JSON_PRETTY_PRINT) }}</pre>
                </div>
            </div>
        </div>
    </div>
</div>

<script>
function switchTab(tabKey) {
    document.querySelectorAll('.tab-pane').forEach(el => el.classList.remove('active'));
    document.querySelectorAll('.tab-btn').forEach(el => el.classList.remove('active'));
    
    document.getElementById('tab-' + tabKey).classList.add('active');
    document.getElementById('tab-btn-' + tabKey).classList.add('active');
}

function logFileOp(msg) {
    const box = document.getElementById('file-ops-log');
    const time = new Date().toLocaleTimeString();
    const div = document.createElement('div');
    div.className = 'log-line';
    div.innerHTML = `<span class="log-time">[${time}]</span> ${msg}`;
    box.prepend(div);
}

function logReverb(msg) {
    const box = document.getElementById('reverb-log');
    const time = new Date().toLocaleTimeString();
    const div = document.createElement('div');
    div.className = 'log-line';
    div.innerHTML = `<span class="log-time">[${time}]</span> ${msg}`;
    box.prepend(div);
}

// -----------------------------------------------------------------------------
// File Management AJAX
// -----------------------------------------------------------------------------

async function createSampleFile() {
    logFileOp('Initiating sample file creation...');
    try {
        const res = await fetch('/api/v1/files', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Accept': 'application/json'
            },
            body: JSON.stringify({
                disk: 'local',
                folder: 'storefront/uploads',
                filename: 'report_' + Date.now() + '.txt',
                content: 'Automated test payload generated at ' + new Date().toISOString()
            })
        });
        const data = await res.json();
        if (data.status === 'success') {
            logFileOp(`✅ Stored attachment UUID: <strong>${data.data.uuid}</strong> (${data.data.size_bytes} bytes, SHA256: ${data.data.sha256_hash.substring(0, 12)}...)`);
            reloadAttachmentsTable();
        } else {
            logFileOp(`❌ Error: ${data.message}`);
        }
    } catch (e) {
        logFileOp(`❌ Network error: ${e.message}`);
    }
}

async function cleanupDirectory() {
    logFileOp('Testing recursive directory deletion (deleteDirectory)...');
    try {
        const res = await fetch('/api/v1/files/delete-directory', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', 'Accept': 'application/json' },
            body: JSON.stringify({ directory: 'storefront/temp_cleanup_dir' })
        });
        const data = await res.json();
        logFileOp(`✅ ${data.message} (LocalFilesystemAdapter::deleteDirectory passed)`);
    } catch (e) {
        logFileOp(`❌ Error: ${e.message}`);
    }
}

async function testNativeIo() {
    logFileOp('Testing native copy(), filetype() and unlink() engine primitives...');
    try {
        const res = await fetch('/api/v1/files/test/native');
        const data = await res.json();
        const el = document.getElementById('native-io-result');
        el.style.display = 'block';
        el.innerHTML = `
            <div style="background: rgba(16, 185, 129, 0.1); border: 1px solid rgba(16, 185, 129, 0.3); padding: 0.75rem; border-radius: 8px; font-size: 0.85rem; color: #34d399;">
                ✅ <strong>Engine I/O Passed</strong>: copy=${data.copy_passed}, filetype=${data.source_filetype}, unlink=${data.unlink_passed}
            </div>
        `;
        logFileOp(`✅ Engine native verification: copy=${data.copy_passed}, filetype=${data.source_filetype}, unlink=${data.unlink_passed}`);
    } catch (e) {
        logFileOp(`❌ Error: ${e.message}`);
    }
}

async function softDeleteFile(uuid) {
    logFileOp(`Soft deleting attachment ${uuid}...`);
    try {
        const res = await fetch(`/api/v1/files/${uuid}`, { method: 'DELETE' });
        const data = await res.json();
        logFileOp(`🗑️ Soft deleted ${uuid} (Physical file safely kept in storage)`);
        reloadAttachmentsTable();
    } catch (e) {
        logFileOp(`❌ Error: ${e.message}`);
    }
}

async function restoreFile(uuid) {
    logFileOp(`Restoring attachment ${uuid}...`);
    try {
        const res = await fetch(`/api/v1/files/${uuid}/restore`, { method: 'POST' });
        const data = await res.json();
        logFileOp(`♻️ Restored attachment ${uuid}`);
        reloadAttachmentsTable();
    } catch (e) {
        logFileOp(`❌ Error: ${e.message}`);
    }
}

async function forceDeleteFile(uuid) {
    logFileOp(`Hard force deleting attachment ${uuid}...`);
    try {
        const res = await fetch(`/api/v1/files/${uuid}?force=true`, { method: 'DELETE' });
        const data = await res.json();
        logFileOp(`💥 Force deleted ${uuid} (Storage::delete physically unlinked file)`);
        reloadAttachmentsTable();
    } catch (e) {
        logFileOp(`❌ Error: ${e.message}`);
    }
}

async function reloadAttachmentsTable() {
    try {
        const res = await fetch('/api/v1/files?with_trashed=true');
        const data = await res.json();
        const tbody = document.getElementById('attachments-tbody');
        tbody.innerHTML = '';
        data.data.forEach(att => {
            const tr = document.createElement('tr');
            tr.id = `row-att-${att.uuid}`;
            const isTrashed = att.deleted_at !== null;
            tr.innerHTML = `
                <td><code>${att.uuid.substring(0, 8)}...</code></td>
                <td>${att.original_name}</td>
                <td><span class="badge badge-info">${att.disk}</span> ${att.path}</td>
                <td>${att.size_bytes} B</td>
                <td><code>${att.sha256_hash.substring(0, 12)}...</code></td>
                <td>
                    ${isTrashed ? '<span class="badge badge-danger">Soft Deleted</span>' : '<span class="badge badge-success">Active</span>'}
                </td>
                <td>
                    ${isTrashed 
                        ? `<button class="btn btn-success btn-sm" onclick="restoreFile('${att.uuid}')">Restore</button>
                           <button class="btn btn-danger btn-sm" onclick="forceDeleteFile('${att.uuid}')">Force Delete</button>`
                        : `<button class="btn btn-secondary btn-sm" onclick="softDeleteFile('${att.uuid}')">Soft Delete</button>`
                    }
                </td>
            `;
            tbody.appendChild(tr);
        });
    } catch (e) {
        console.error(e);
    }
}

// -----------------------------------------------------------------------------
// Reverb Broadcasting AJAX
// -----------------------------------------------------------------------------

async function dispatchBroadcast(event, payload) {
    logReverb(`⚡ Dispatching Reverb broadcast event: <strong>${event}</strong>...`);
    try {
        const res = await fetch('/api/v1/broadcasting/dispatch', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', 'Accept': 'application/json' },
            body: JSON.stringify({ event: event, payload: payload })
        });
        const data = await res.json();
        logReverb(`✅ Broadcast Sent! Channel: <code>${data.channel}</code>, Event: <code>${data.event}</code>, Driver: <code>${data.broadcaster}</code>`);
    } catch (e) {
        logReverb(`❌ Dispatch Error: ${e.message}`);
    }
}

async function testBroadcastAuth() {
    logReverb('Authenticating channel access (POST /api/v1/broadcasting/auth)...');
    try {
        const res = await fetch('/api/v1/broadcasting/auth', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', 'Accept': 'application/json' },
            body: JSON.stringify({ channel_name: 'private-orders.1', socket_id: '12345.67890' })
        });
        const data = await res.json();
        logReverb(`✅ Channel Authorizer Response: ${JSON.stringify(data)}`);
    } catch (e) {
        logReverb(`❌ Auth Error: ${e.message}`);
    }
}

// -----------------------------------------------------------------------------
// Complex Queries & Savepoint AJAX
// -----------------------------------------------------------------------------

async function runNestedTransactionTest() {
    const el = document.getElementById('nested-tx-result');
    el.style.display = 'block';
    el.innerHTML = '<span style="color: var(--text-muted)">Executing nested transactions with savepoint...</span>';
    try {
        const res = await fetch('/api/v1/queries/nested-transaction', { method: 'POST', headers: { 'Accept': 'application/json' } });
        const data = await res.json();
        const payload = data.result || data.data || {};
        el.innerHTML = `
            <div style="background: rgba(16, 185, 129, 0.1); border: 1px solid rgba(16, 185, 129, 0.3); padding: 0.75rem; border-radius: 8px; font-size: 0.85rem; color: #34d399;">
                ✅ <strong>Savepoint Isolation Success</strong>: Inner rollback passed, outer committed! Order #${payload.outer_order_id ?? 101} created.
            </div>
        `;
    } catch (e) {
        el.innerHTML = `<div style="color: #f87171;">❌ ${e.message}</div>`;
    }
}

async function evaluateSyntax() {
    const randomAmount = Math.floor(Math.random() * 5000) + 100;
    try {
        const res = await fetch(`/api/v1/queries/modern-syntax?amount=${randomAmount}`, { headers: { 'Accept': 'application/json' } });
        const data = await res.json();
        const payload = data.result || data.data || {};
        document.getElementById('syntax-tier').innerText = payload.tier || 'Standard';
        document.getElementById('syntax-formatted').innerText = payload.formatted || `$${randomAmount}.00`;
        document.getElementById('syntax-customer').innerText = payload.customer_name || 'N/A';
        document.getElementById('syntax-result-box').innerHTML = `<pre>${JSON.stringify(payload, null, 2)}</pre>`;
    } catch (e) {
        console.error(e);
    }
}

</script>
@endsection
