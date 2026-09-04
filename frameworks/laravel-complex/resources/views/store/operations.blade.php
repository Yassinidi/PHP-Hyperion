@extends('layouts.store')

@section('title', 'Hyperion Enterprise Store - Queues, Jobs & Ops Dashboard')

@section('styles')
<style>
    .ops-header {
        display: flex;
        flex-wrap: wrap;
        justify-content: space-between;
        align-items: center;
        gap: 1rem;
        margin-bottom: 2rem;
    }
    .ops-title {
        font-size: 1.8rem;
        font-weight: 800;
        letter-spacing: -0.02em;
    }
    .ops-subtitle {
        color: var(--text-muted);
        font-size: 0.95rem;
        margin-top: 0.25rem;
    }
    .metrics-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
        gap: 1.25rem;
        margin-bottom: 2rem;
    }
    .metric-card {
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: 12px;
        padding: 1.25rem;
        display: flex;
        flex-direction: column;
        gap: 0.4rem;
        transition: transform 0.2s, border-color 0.2s;
    }
    .metric-card:hover {
        transform: translateY(-2px);
        border-color: var(--border-highlight);
    }
    .metric-title {
        font-size: 0.75rem;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: var(--text-muted);
    }
    .metric-val {
        font-size: 1.8rem;
        font-weight: 800;
        color: var(--text-main);
    }
    .metric-val.green { color: var(--accent-green); }
    .metric-val.amber { color: #f59e0b; }
    .metric-val.red { color: #ef4444; }
    .metric-val.purple { color: #a855f7; }

    .actions-panel {
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: 14px;
        padding: 1.5rem;
        margin-bottom: 2rem;
    }
    .actions-title {
        font-size: 0.9rem;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: var(--accent);
        margin-bottom: 1rem;
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }
    .btn-row {
        display: flex;
        flex-wrap: wrap;
        gap: 0.75rem;
    }
    .btn {
        padding: 0.6rem 1.2rem;
        border-radius: 8px;
        font-size: 0.85rem;
        font-weight: 600;
        cursor: pointer;
        border: 1px solid transparent;
        transition: all 0.2s;
        text-decoration: none;
        display: inline-flex;
        align-items: center;
        gap: 0.4rem;
    }
    .btn-primary {
        background: var(--accent);
        color: white;
    }
    .btn-primary:hover {
        background: #4f46e5;
        box-shadow: 0 0 15px var(--accent-glow);
    }
    .btn-success {
        background: rgba(16, 185, 129, 0.2);
        color: #34d399;
        border-color: rgba(16, 185, 129, 0.4);
    }
    .btn-success:hover {
        background: rgba(16, 185, 129, 0.3);
    }
    .btn-danger {
        background: rgba(239, 68, 68, 0.2);
        color: #f87171;
        border-color: rgba(239, 68, 68, 0.4);
    }
    .btn-danger:hover {
        background: rgba(239, 68, 68, 0.3);
    }
    .btn-purple {
        background: rgba(168, 85, 247, 0.2);
        color: #c084fc;
        border-color: rgba(168, 85, 247, 0.4);
    }
    .btn-purple:hover {
        background: rgba(168, 85, 247, 0.3);
    }

    .dashboard-sections {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 1.5rem;
    }
    @media (max-width: 900px) {
        .dashboard-sections {
            grid-template-columns: 1fr;
        }
    }
    .card-box {
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: 14px;
        padding: 1.5rem;
    }
    .card-box-title {
        font-size: 0.95rem;
        font-weight: 700;
        margin-bottom: 1rem;
        display: flex;
        justify-content: space-between;
        align-items: center;
    }
    .table-custom {
        width: 100%;
        border-collapse: collapse;
        font-size: 0.85rem;
    }
    .table-custom th {
        text-align: left;
        color: var(--text-muted);
        font-weight: 600;
        padding-bottom: 0.5rem;
        font-size: 0.75rem;
        text-transform: uppercase;
        border-bottom: 1px solid var(--border-color);
    }
    .table-custom td {
        padding: 0.6rem 0;
        border-bottom: 1px solid rgba(255, 255, 255, 0.04);
    }
    .feed-item {
        padding: 0.6rem 0;
        border-bottom: 1px solid rgba(255, 255, 255, 0.04);
        font-size: 0.825rem;
    }
    .feed-meta {
        font-size: 0.75rem;
        color: var(--text-muted);
        margin-top: 0.2rem;
    }
    #action-feedback {
        margin-top: 1rem;
        padding: 0.75rem 1rem;
        border-radius: 8px;
        font-size: 0.85rem;
        font-family: monospace;
        display: none;
    }
</style>
@endsection

@section('content')
<div class="ops-header">
    <div>
        <h1 class="ops-title">Queues, Jobs & Asynchronous Operations</h1>
        <p class="ops-subtitle">Real-time inspection of database queue workers, job batches, failed job handlers, and cache state</p>
    </div>
    <div>
        <a href="{{ route('store.index') }}" style="color: var(--accent); text-decoration: none; font-weight: 600; font-size: 0.9rem;">&larr; Back to Catalog</a>
    </div>
</div>

<div class="metrics-grid">
    <div class="metric-card">
        <div class="metric-title">Pending Queue Jobs</div>
        <div class="metric-val {{ $pendingJobs > 0 ? 'amber' : 'green' }}">{{ $pendingJobs }}</div>
        <div style="font-size: 0.75rem; color: var(--text-muted);">Database queue driver</div>
    </div>
    <div class="metric-card">
        <div class="metric-title">Failed Jobs Log</div>
        <div class="metric-val {{ $failedJobs > 0 ? 'red' : 'green' }}">{{ $failedJobs }}</div>
        <div style="font-size: 0.75rem; color: var(--text-muted);">Available for retry</div>
    </div>
    <div class="metric-card">
        <div class="metric-title">Job Batches</div>
        <div class="metric-val purple">{{ $batches->count() }}</div>
        <div style="font-size: 0.75rem; color: var(--text-muted);">Bus::batch() pipelines</div>
    </div>
    <div class="metric-card">
        <div class="metric-title">Notifications Sent</div>
        <div class="metric-val green">{{ $notifications->count() }}</div>
        <div style="font-size: 0.75rem; color: var(--text-muted);">Database & Mail channels</div>
    </div>
    <div class="metric-card">
        <div class="metric-title">Cached Valuation</div>
        <div class="metric-val green">
            @if($cachedReport)
                ${{ number_format($cachedReport['total_valuation'], 0) }}
            @else
                <span style="font-size: 1rem; color: var(--text-muted);">Cold Cache</span>
            @endif
        </div>
        <div style="font-size: 0.75rem; color: var(--text-muted);">Cache::remember store</div>
    </div>
</div>

<div class="actions-panel">
    <div class="actions-title">
        <span>⚡</span> Interactive Pipeline Controls (Direct API Triggers)
    </div>
    <div class="btn-row">
        <button class="btn btn-primary" onclick="triggerJob('report')">
            <span>📊</span> Dispatch Valuation Job
        </button>
        <button class="btn btn-purple" onclick="triggerJob('batch')">
            <span>📦</span> Dispatch Bus::batch()
        </button>
        <button class="btn btn-danger" onclick="triggerJob('fail')">
            <span>⚠</span> Dispatch Failing Job
        </button>
        <button class="btn btn-success" onclick="triggerWorkOnce()">
            <span>⚙</span> Execute Queue Worker Once
        </button>
        <button class="btn btn-success" onclick="triggerRetry()">
            <span>🔄</span> Retry All Failed Jobs
        </button>
    </div>
    <div id="action-feedback"></div>
</div>

<div class="dashboard-sections">
    <div class="card-box">
        <div class="card-box-title">
            <span>Job Batches (Bus::batch)</span>
            <span style="font-size: 0.75rem; color: var(--text-muted);">Recent 5</span>
        </div>
        @if($batches->isEmpty())
            <p style="color: var(--text-muted); font-size: 0.85rem; padding: 1.5rem 0;">No batches dispatched yet. Click "Dispatch Bus::batch()" above to trigger one.</p>
        @else
            <table class="table-custom">
                <thead>
                    <tr>
                        <th>Batch Name</th>
                        <th>Total</th>
                        <th>Pending</th>
                        <th>Failed</th>
                    </tr>
                </thead>
                <tbody>
                    @foreach($batches as $b)
                        <tr>
                            <td><strong>{{ $b->name }}</strong></td>
                            <td>{{ $b->total_jobs }}</td>
                            <td>{{ $b->pending_jobs }}</td>
                            <td>{{ $b->failed_jobs }}</td>
                        </tr>
                    @endforeach
                </tbody>
            </table>
        @endif
    </div>

    <div class="card-box">
        <div class="card-box-title">
            <span>Failed Jobs Registry</span>
            <span style="font-size: 0.75rem; color: var(--text-muted);">Recent 5</span>
        </div>
        @if($recentFailed->isEmpty())
            <p style="color: var(--text-muted); font-size: 0.85rem; padding: 1.5rem 0;">Zero failures in registry. All jobs executing cleanly.</p>
        @else
            <table class="table-custom">
                <thead>
                    <tr>
                        <th>Queue</th>
                        <th>Exception</th>
                        <th>Failed At</th>
                    </tr>
                </thead>
                <tbody>
                    @foreach($recentFailed as $f)
                        <tr>
                            <td><code>{{ $f->queue }}</code></td>
                            <td style="color: #f87171; max-width: 250px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                                {{ substr($f->exception, 0, 50) }}...
                            </td>
                            <td>{{ $f->failed_at }}</td>
                        </tr>
                    @endforeach
                </tbody>
            </table>
        @endif
    </div>
</div>

<div style="margin-top: 1.5rem;" class="card-box">
    <div class="card-box-title">
        <span>⚡ Real-Time Polymorphic Audit & Notification Stream</span>
        <span style="font-size: 0.75rem; color: var(--text-muted);">Recent 10 Events</span>
    </div>
    <div>
        @forelse($recentActivities as $act)
            <div class="feed-item">
                <span style="color: var(--accent); font-weight: 700; margin-right: 0.5rem;">[{{ $act->action }}]</span>
                <span>{{ $act->description }}</span>
                <div class="feed-meta">{{ $act->created_at->diffForHumans() }} &bull; Subject: {{ class_basename($act->subject_type) }} #{{ $act->subject_id }}</div>
            </div>
        @empty
            <p style="color: var(--text-muted); font-size: 0.85rem;">No activity records yet.</p>
        @endforelse
    </div>
</div>

<script>
    async function triggerJob(type) {
        showFeedback('Dispatching job: ' + type + '...', 'var(--bg-card)', 'var(--text-muted)');
        try {
            const res = await fetch('/api/v1/queue/dispatch-job', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json', 'Accept': 'application/json' },
                body: JSON.stringify({ type })
            });
            const data = await res.json();
            showFeedback('SUCCESS: ' + JSON.stringify(data), 'rgba(16, 185, 129, 0.15)', '#34d399');
            setTimeout(() => window.location.reload(), 1200);
        } catch(e) {
            showFeedback('ERROR: ' + e.message, 'rgba(239, 68, 68, 0.15)', '#f87171');
        }
    }

    async function triggerWorkOnce() {
        showFeedback('Executing queue worker once...', 'var(--bg-card)', 'var(--text-muted)');
        try {
            const res = await fetch('/api/v1/queue/work-once', {
                method: 'POST',
                headers: { 'Accept': 'application/json' }
            });
            const data = await res.json();
            showFeedback('WORKER: ' + (data.output || data.message) + ' (Remaining: ' + data.remaining_jobs + ')', 'rgba(99, 102, 241, 0.15)', '#818cf8');
            setTimeout(() => window.location.reload(), 1200);
        } catch(e) {
            showFeedback('ERROR: ' + e.message, 'rgba(239, 68, 68, 0.15)', '#f87171');
        }
    }

    async function triggerRetry() {
        showFeedback('Retrying all failed jobs...', 'var(--bg-card)', 'var(--text-muted)');
        try {
            const res = await fetch('/api/v1/queue/retry-failed', {
                method: 'POST',
                headers: { 'Accept': 'application/json' }
            });
            const data = await res.json();
            showFeedback('RETRY: ' + (data.output || data.message), 'rgba(16, 185, 129, 0.15)', '#34d399');
            setTimeout(() => window.location.reload(), 1200);
        } catch(e) {
            showFeedback('ERROR: ' + e.message, 'rgba(239, 68, 68, 0.15)', '#f87171');
        }
    }

    function showFeedback(msg, bg, color) {
        const el = document.getElementById('action-feedback');
        el.style.display = 'block';
        el.style.background = bg;
        el.style.color = color;
        el.style.border = '1px solid ' + color;
        el.innerText = msg;
    }
</script>
@endsection
