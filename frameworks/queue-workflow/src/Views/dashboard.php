<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Vortex Queue — Transactional Task Orchestration</title>
    <link rel="stylesheet" href="/style.css">
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;600;700&family=JetBrains+Mono:wght@400;600&display=swap" rel="stylesheet">
</head>
<body>
    <div class="queue-container">
        <header>
            <div class="brand">
                <h1>⚡ Vortex Job Dispatcher & FSM</h1>
                <div style="font-size: 0.85rem; color: var(--text-secondary); margin-top: 0.25rem;">
                    Atomic Row-Leased Background Task Queue & Finite State Machine
                </div>
            </div>
            <div>
                <span class="badge">Engine: <?= defined('HYPERION_PERSISTENT') ? 'Hyperion Core (Persistent)' : 'Standard PHP 8.4' ?></span>
            </div>
        </header>

        <section class="grid">
            <div class="card">
                <div class="card-title">Pending Jobs</div>
                <div class="card-val card-amber"><?= number_format($stats['status_breakdown']['pending'] ?? 0) ?></div>
            </div>
            <div class="card">
                <div class="card-title">Claimed (In-Flight)</div>
                <div class="card-val card-sky"><?= number_format($stats['status_breakdown']['claimed'] ?? 0) ?></div>
            </div>
            <div class="card">
                <div class="card-title">Completed Jobs</div>
                <div class="card-val card-teal"><?= number_format($stats['status_breakdown']['completed'] ?? 0) ?></div>
            </div>
            <div class="card">
                <div class="card-title">Total Enqueued</div>
                <div class="card-val card-purple"><?= number_format($stats['total_jobs'] ?? 0) ?></div>
            </div>
        </section>

        <section class="panel">
            <div style="font-size: 1.1rem; font-weight: 600; margin-bottom: 1rem;">Recent Tasks & Worker State Transitions</div>
            <table>
                <thead>
                    <tr>
                        <th>Job ID</th>
                        <th>Queue</th>
                        <th>Priority</th>
                        <th>Status</th>
                        <th>Worker ID</th>
                        <th>Attempts</th>
                        <th>Created</th>
                    </tr>
                </thead>
                <tbody>
                    <?php foreach ($recentJobs as $j): ?>
                    <tr>
                        <td>#<?= $j['id'] ?></td>
                        <td style="color: var(--accent-teal);"><?= htmlspecialchars($j['queue_name']) ?></td>
                        <td><?= $j['priority'] ?></td>
                        <td>
                            <span class="status-badge status-<?= htmlspecialchars($j['status']) ?>">
                                <?= strtoupper(htmlspecialchars($j['status'])) ?>
                            </span>
                        </td>
                        <td><?= $j['worker_id'] ? htmlspecialchars($j['worker_id']) : '—' ?></td>
                        <td><?= $j['attempts'] ?></td>
                        <td style="color: var(--text-secondary);"><?= date('H:i:s', (int)$j['created_at']) ?></td>
                    </tr>
                    <?php endforeach; ?>
                </tbody>
            </table>
        </section>
    </div>
</body>
</html>
