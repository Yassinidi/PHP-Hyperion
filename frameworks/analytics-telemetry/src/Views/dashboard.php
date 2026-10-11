<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Nexus Telemetry — Real-Time Systems Intelligence</title>
    <link rel="stylesheet" href="/style.css">
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;600;700&family=JetBrains+Mono:wght@400;600&display=swap" rel="stylesheet">
</head>
<body>
    <div class="dashboard-container">
        <header>
            <div class="brand">
                <span style="font-size: 1.75rem;">📡</span>
                <div>
                    <h1>Nexus Telemetry Gateway</h1>
                    <div style="font-size: 0.8rem; color: var(--text-secondary);">Distributed Metrics Ingestion & Quantile Engine</div>
                </div>
            </div>
            <div style="display: flex; gap: 0.75rem; align-items: center;">
                <span class="badge">Engine: <?= defined('HYPERION_PERSISTENT') ? 'Hyperion Core (Persistent)' : 'Standard PHP 8.4' ?></span>
                <span class="badge" style="background: rgba(16, 185, 129, 0.15); color: var(--accent-green); border-color: rgba(16, 185, 129, 0.3);">Live Cluster</span>
            </div>
        </header>

        <section class="stats-grid">
            <div class="stat-card">
                <div class="stat-title">P99 Latency</div>
                <div class="stat-value stat-rose"><?= number_format($summary['p99_ms'] ?? 0, 2) ?><span style="font-size: 1rem; color: var(--text-secondary);"> ms</span></div>
            </div>
            <div class="stat-card">
                <div class="stat-title">P50 Median</div>
                <div class="stat-value stat-cyan"><?= number_format($summary['p50_ms'] ?? 0, 2) ?><span style="font-size: 1rem; color: var(--text-secondary);"> ms</span></div>
            </div>
            <div class="stat-card">
                <div class="stat-title">Sample Volume (1h)</div>
                <div class="stat-value stat-indigo"><?= number_format($summary['sample_size'] ?? 0) ?></div>
            </div>
            <div class="stat-card">
                <div class="stat-title">Error Rate</div>
                <div class="stat-value stat-green"><?= number_format($summary['error_rate_pct'] ?? 0, 2) ?>%</div>
            </div>
        </section>

        <div class="content-grid">
            <section class="panel">
                <div class="panel-title">
                    <span>Recent Telemetry Ingest Stream</span>
                    <span style="font-size: 0.8rem; color: var(--text-secondary);">Last 15 samples</span>
                </div>
                <table>
                    <thead>
                        <tr>
                            <th>Node</th>
                            <th>Service</th>
                            <th>Latency</th>
                            <th>Status</th>
                            <th>Timestamp</th>
                        </tr>
                    </thead>
                    <tbody>
                        <?php foreach (array_slice($recentMetrics, 0, 15) as $m): ?>
                        <tr>
                            <td style="color: var(--accent-cyan);"><?= htmlspecialchars($m['node_id']) ?></td>
                            <td><?= htmlspecialchars($m['service_name']) ?></td>
                            <td style="<?= $m['latency_ms'] > 50 ? 'color: var(--accent-rose); font-weight: 600;' : '' ?>"><?= number_format($m['latency_ms'], 2) ?> ms</td>
                            <td><span style="color: <?= $m['status_code'] < 400 ? 'var(--accent-green)' : 'var(--accent-rose)' ?>"><?= $m['status_code'] ?></span></td>
                            <td style="color: var(--text-secondary);"><?= date('H:i:s', (int)$m['created_at']) ?></td>
                        </tr>
                        <?php endforeach; ?>
                    </tbody>
                </table>
            </section>

            <section class="panel">
                <div class="panel-title">
                    <span>Active Alert Rules</span>
                    <span style="font-size: 0.8rem; color: var(--text-secondary);"><?= count($alerts['alerts'] ?? []) ?> rules</span>
                </div>
                <div>
                    <?php foreach ($alerts['alerts'] ?? [] as $a): ?>
                    <div class="alert-item">
                        <div>
                            <div style="font-weight: 600; font-size: 0.9rem;"><?= htmlspecialchars($a['name']) ?></div>
                            <div style="font-size: 0.75rem; color: var(--text-secondary);">Threshold: <?= $a['threshold'] ?> | Current: <?= $a['current_val'] ?></div>
                        </div>
                        <div class="<?= $a['status'] === 'OK' ? 'status-ok' : 'status-firing' ?>">
                            <?= $a['status'] ?>
                        </div>
                    </div>
                    <?php endforeach; ?>
                </div>
            </section>
        </div>
    </div>
</body>
</html>
