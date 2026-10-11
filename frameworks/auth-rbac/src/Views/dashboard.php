<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Aegis Auth & RBAC — Identity Verification Engine</title>
    <link rel="stylesheet" href="/style.css">
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;600;700&family=JetBrains+Mono:wght@400;600&display=swap" rel="stylesheet">
</head>
<body>
    <div class="container">
        <header>
            <div class="brand">
                <h1>🛡️ Aegis Identity & RBAC Matrix</h1>
                <div style="font-size: 0.85rem; color: var(--text-secondary); margin-top: 0.25rem;">
                    Cryptographic HMAC-SHA256 Token Minting & Hierarchical Permissions
                </div>
            </div>
            <div style="display: flex; gap: 0.75rem;">
                <span class="badge">Engine: <?= defined('HYPERION_PERSISTENT') ? 'Hyperion Core (Persistent)' : 'Standard PHP 8.4' ?></span>
            </div>
        </header>

        <div class="grid">
            <section class="card">
                <div class="card-title">Role Hierarchy & Inherited Masks</div>
                <table>
                    <thead>
                        <tr>
                            <th>Role ID</th>
                            <th>Role Name</th>
                            <th>Parent</th>
                            <th>Perm Mask</th>
                        </tr>
                    </thead>
                    <tbody>
                        <?php foreach ($roles as $r): ?>
                        <tr>
                            <td>#<?= $r['id'] ?></td>
                            <td style="color: var(--accent-emerald); font-weight: 600;"><?= htmlspecialchars($r['name']) ?></td>
                            <td><?= $r['parent_id'] ? '#' . $r['parent_id'] : 'Root' ?></td>
                            <td><span class="bit-tag"><?= $r['permission_mask'] ?> (0b<?= sprintf('%05b', $r['permission_mask']) ?>)</span></td>
                        </tr>
                        <?php endforeach; ?>
                    </tbody>
                </table>
            </section>

            <section class="card">
                <div class="card-title">Registered Security Principals</div>
                <table>
                    <thead>
                        <tr>
                            <th>User ID</th>
                            <th>Username</th>
                            <th>Role</th>
                            <th>Status</th>
                        </tr>
                    </thead>
                    <tbody>
                        <?php foreach ($users as $u): ?>
                        <tr>
                            <td>#<?= $u['id'] ?></td>
                            <td><?= htmlspecialchars($u['username']) ?></td>
                            <td>#<?= $u['role_id'] ?></td>
                            <td><span style="color: var(--accent-emerald);"><?= htmlspecialchars($u['status']) ?></span></td>
                        </tr>
                        <?php endforeach; ?>
                    </tbody>
                </table>
            </section>
        </div>
    </div>
</body>
</html>
