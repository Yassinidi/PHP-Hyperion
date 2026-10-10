<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Aura Admin Portal — Management & Metrics</title>
    <style>
        :root {
            --bg-base: #090d16;
            --bg-card: #111827;
            --border-color: #1f2937;
            --accent: #6366f1;
            --text-main: #f9fafb;
            --text-muted: #9ca3af;
        }
        * { box-sizing: border-box; margin: 0; padding: 0; font-family: system-ui, -apple-system, sans-serif; }
        body { background: var(--bg-base); color: var(--text-main); line-height: 1.5; padding-bottom: 3rem; }
        .navbar { display: flex; justify-content: space-between; align-items: center; padding: 1rem 2rem; background: rgba(17, 24, 39, 0.8); backdrop-filter: blur(12px); border-bottom: 1px solid var(--border-color); }
        .logo { font-size: 1.25rem; font-weight: 800; color: #a5b4fc; }
        .container { max-width: 1200px; margin: 2rem auto; padding: 0 1.5rem; }
        .metrics-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 1.25rem; margin-bottom: 2rem; }
        .metric-card { background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 10px; padding: 1.25rem; }
        .metric-label { font-size: 0.8rem; text-transform: uppercase; letter-spacing: 0.05em; color: var(--text-muted); margin-bottom: 0.4rem; }
        .metric-value { font-size: 1.75rem; font-weight: 800; color: #fff; }
        .tables-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 1.5rem; }
        @media(max-width: 900px) { .tables-grid { grid-template-columns: 1fr; } }
        .card { background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 10px; padding: 1.25rem; }
        .card-title { font-size: 1.1rem; font-weight: 700; margin-bottom: 1rem; }
        table { width: 100%; border-collapse: collapse; font-size: 0.85rem; }
        th, td { padding: 0.65rem 0.75rem; text-align: left; border-bottom: 1px solid var(--border-color); }
        th { color: var(--text-muted); font-weight: 600; }
        .badge { display: inline-block; padding: 0.2rem 0.5rem; border-radius: 9999px; font-size: 0.72rem; font-weight: 600; text-transform: uppercase; }
        .badge-admin { background: rgba(239, 68, 68, 0.2); color: #fca5a5; }
        .badge-client { background: rgba(59, 130, 246, 0.2); color: #93c5fd; }
        .badge-active { background: rgba(16, 185, 129, 0.2); color: #6ee7b7; }
    </style>
</head>
<body>
    <nav class="navbar">
        <div class="logo">🛡️ Aura Admin Portal</div>
        <div><a href="/" style="color: #9ca3af; text-decoration: none; font-size: 0.9rem;">&larr; Back to Storefront</a></div>
    </nav>

    <div class="container">
        <!-- KPI Metrics -->
        <div class="metrics-grid">
            <div class="metric-card">
                <div class="metric-label">Total Revenue</div>
                <div class="metric-value" style="color: #34d399;">$<?= number_format($dashboard['metrics']['total_revenue'], 2) ?></div>
            </div>
            <div class="metric-card">
                <div class="metric-label">Completed Orders</div>
                <div class="metric-value"><?= $dashboard['metrics']['total_orders'] ?></div>
            </div>
            <div class="metric-card">
                <div class="metric-label">Total Client Users</div>
                <div class="metric-value"><?= $dashboard['metrics']['total_clients'] ?></div>
            </div>
            <div class="metric-card">
                <div class="metric-label">Active Products</div>
                <div class="metric-value"><?= $dashboard['metrics']['total_products'] ?></div>
            </div>
        </div>

        <div class="tables-grid">
            <!-- User Management Table -->
            <div class="card">
                <div class="card-title">👥 User Management (Admin & Client)</div>
                <table>
                    <thead>
                        <tr>
                            <th>ID</th>
                            <th>Name</th>
                            <th>Email</th>
                            <th>Role</th>
                        </tr>
                    </thead>
                    <tbody>
                        <?php foreach ($users as $u): ?>
                        <tr>
                            <td>#<?= $u['id'] ?></td>
                            <td><strong><?= htmlspecialchars($u['name']) ?></strong></td>
                            <td style="color: var(--text-muted);"><?= htmlspecialchars($u['email']) ?></td>
                            <td>
                                <span class="badge <?= $u['role'] === 'admin' ? 'badge-admin' : 'badge-client' ?>">
                                    <?= htmlspecialchars($u['role']) ?>
                                </span>
                            </td>
                        </tr>
                        <?php endforeach; ?>
                    </tbody>
                </table>
            </div>

            <!-- Recent Orders & Inventory -->
            <div class="card">
                <div class="card-title">📦 Recent Orders</div>
                <table>
                    <thead>
                        <tr>
                            <th>Order</th>
                            <th>Customer</th>
                            <th>Total</th>
                            <th>Status</th>
                        </tr>
                    </thead>
                    <tbody>
                        <?php foreach ($dashboard['recent_orders'] as $ord): ?>
                        <tr>
                            <td>#<?= $ord['id'] ?></td>
                            <td><?= htmlspecialchars($ord['customer_name']) ?></td>
                            <td style="color: #34d399; font-weight: 600;">$<?= number_format((float)$ord['total_amount'], 2) ?></td>
                            <td><span class="badge badge-active"><?= htmlspecialchars($ord['status']) ?></span></td>
                        </tr>
                        <?php endforeach; ?>
                    </tbody>
                </table>

                <div class="card-title" style="margin-top: 1.5rem;">⚠️ Low Stock Inventory Alert</div>
                <table>
                    <thead>
                        <tr>
                            <th>Product</th>
                            <th>SKU</th>
                            <th>Remaining Stock</th>
                        </tr>
                    </thead>
                    <tbody>
                        <?php foreach ($dashboard['low_stock_alerts'] as $ls): ?>
                        <tr>
                            <td><?= htmlspecialchars($ls['title']) ?></td>
                            <td style="color: var(--text-muted);"><?= htmlspecialchars($ls['sku']) ?></td>
                            <td style="color: #f87171; font-weight: 700;"><?= $ls['stock'] ?> units</td>
                        </tr>
                        <?php endforeach; ?>
                    </tbody>
                </table>
            </div>
        </div>
    </div>
</body>
</html>
