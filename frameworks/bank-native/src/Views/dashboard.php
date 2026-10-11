<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Mercury Core - Enterprise Bank Ledger</title>
    <link rel="stylesheet" href="/style.css">
</head>
<body>
    <div class="container">
        <!-- Header -->
        <header>
            <div class="brand">
                <div class="brand-icon">🏛️</div>
                <div class="brand-title">
                    <h1>Mercury Core Banking</h1>
                    <p>ACID Ledger & Instant Interbank Settlement</p>
                </div>
            </div>
            <div>
                <span class="badge"><?= htmlspecialchars($stats['engine']) ?></span>
            </div>
        </header>

        <!-- KPI Metrics -->
        <div class="kpi-grid">
            <div class="kpi-card">
                <div class="kpi-label">TOTAL SYSTEM LIQUIDITY</div>
                <div class="kpi-value">$<?= number_format($stats['total_liquidity'], 2) ?></div>
                <div class="kpi-desc">Across <?= $stats['total_accounts'] ?> Institutional Accounts</div>
            </div>
            <div class="kpi-card">
                <div class="kpi-label">TOTAL SETTLED VOLUME</div>
                <div class="kpi-value">$<?= number_format($stats['total_volume_transferred'], 2) ?></div>
                <div class="kpi-desc"><?= $stats['total_transfers'] ?> Total Completed Transfers</div>
            </div>
            <div class="kpi-card">
                <div class="kpi-label">AVERAGE ACCOUNT RESERVE</div>
                <div class="kpi-value">$<?= number_format($stats['avg_balance'], 2) ?></div>
                <div class="kpi-desc">100% Reserve Ratio Guaranteed</div>
            </div>
            <div class="kpi-card">
                <div class="kpi-label">ENGINE ISOLATION & ACID TIER</div>
                <div class="kpi-value" style="font-size: 1.35rem; color: #10b981;">STRICT SERIALIZABLE</div>
                <div class="kpi-desc">WAL Mode • Shared Memory Arena</div>
            </div>
        </div>

        <!-- Main Content Grid -->
        <div class="main-grid">
            <!-- Left: Account Portfolio -->
            <div class="card">
                <div class="card-title">
                    <span>Institutional Accounts</span>
                    <span style="font-size: 0.8rem; color: #94a3b8;"><?= count($accounts) ?> Active Ledgers</span>
                </div>
                <div class="table-responsive">
                    <table>
                        <thead>
                            <tr>
                                <th>Account</th>
                                <th>Holder</th>
                                <th>Balance</th>
                                <th>Status</th>
                                <th>Transfers</th>
                            </tr>
                        </thead>
                        <tbody>
                            <?php foreach ($accounts as $acc): ?>
                            <tr>
                                <td><span class="account-badge"><?= htmlspecialchars($acc['account_number']) ?></span></td>
                                <td><strong><?= htmlspecialchars($acc['holder_name']) ?></strong><br><small style="color: #64748b;"><?= htmlspecialchars($acc['email']) ?></small></td>
                                <td><span class="balance-amount">$<?= number_format((float)$acc['balance'], 2) ?></span></td>
                                <td><span class="status-active"></span><?= ucfirst($acc['status']) ?></td>
                                <td><span style="color: #94a3b8; font-weight: 600;"><?= $acc['total_transfers'] ?> txns</span></td>
                            </tr>
                            <?php endforeach; ?>
                        </tbody>
                    </table>
                </div>
            </div>

            <!-- Right: Instant Money Transfer -->
            <div class="card">
                <div class="card-title">Instant Bank Wire</div>
                <form id="transferForm" method="POST" action="/api/transfer" onsubmit="executeTransfer(event)">
                    <div class="form-group">
                        <label for="from_account">Debit Account (From)</label>
                        <select id="from_account" name="from_account_id" required>
                            <?php foreach ($accounts as $acc): ?>
                            <option value="<?= $acc['id'] ?>">
                                <?= htmlspecialchars($acc['holder_name']) ?> ($<?= number_format((float)$acc['balance'], 2) ?>)
                            </option>
                            <?php endforeach; ?>
                        </select>
                    </div>

                    <div class="form-group">
                        <label for="to_account">Credit Account (To)</label>
                        <select id="to_account" name="to_account_id" required>
                            <?php foreach (array_reverse($accounts) as $acc): ?>
                            <option value="<?= $acc['id'] ?>">
                                <?= htmlspecialchars($acc['holder_name']) ?> (<?= $acc['account_number'] ?>)
                            </option>
                            <?php endforeach; ?>
                        </select>
                    </div>

                    <div class="form-group">
                        <label for="amount">Transfer Amount ($ USD)</label>
                        <input type="number" id="amount" name="amount" min="1" step="0.01" value="100.00" required>
                    </div>

                    <div class="form-group">
                        <label for="note">Reference / Note</label>
                        <input type="text" id="note" name="note" placeholder="Interbank Settlement Memo" value="Interbank Wire">
                    </div>

                    <button type="submit" class="btn-transfer" id="submitBtn">⚡ Execute Immediate Transfer</button>
                    <div id="transferFeedback" style="margin-top: 1rem; font-size: 0.85rem; display: none;"></div>
                </form>
            </div>
        </div>

        <!-- Bottom: Recent Transactions Ledger -->
        <div class="card">
            <div class="card-title">
                <span>Recent Interbank Settlement Ledger</span>
                <span style="font-size: 0.8rem; color: #94a3b8;">Real-Time ACID Log</span>
            </div>
            <div>
                <?php if (empty($transfers)): ?>
                    <p style="color: #64748b; font-size: 0.9rem;">No transactions recorded yet.</p>
                <?php else: ?>
                    <?php foreach ($transfers as $txn): ?>
                    <div class="ledger-item">
                        <div>
                            <div class="ledger-route">
                                <span><?= htmlspecialchars($txn['from_holder']) ?></span>
                                <span style="color: #64748b;"> ➔ </span>
                                <span><?= htmlspecialchars($txn['to_holder']) ?></span>
                                <span style="font-size: 0.75rem; color: #06b6d4; margin-left: 0.5rem; font-family: monospace;">[<?= htmlspecialchars($txn['reference']) ?>]</span>
                            </div>
                            <div class="ledger-meta">
                                <?= htmlspecialchars($txn['note']) ?> • <?= htmlspecialchars($txn['created_at']) ?>
                            </div>
                        </div>
                        <div class="ledger-amount">
                            +$<?= number_format((float)$txn['amount'], 2) ?>
                        </div>
                    </div>
                    <?php endforeach; ?>
                <?php endif; ?>
            </div>
        </div>
    </div>

    <script>
        async function executeTransfer(e) {
            e.preventDefault();
            const form = document.getElementById('transferForm');
            const feedback = document.getElementById('transferFeedback');
            const submitBtn = document.getElementById('submitBtn');

            submitBtn.disabled = true;
            submitBtn.innerText = 'Processing ACID Transaction...';

            const payload = {
                from_account_id: parseInt(document.getElementById('from_account').value),
                to_account_id: parseInt(document.getElementById('to_account').value),
                amount: parseFloat(document.getElementById('amount').value),
                note: document.getElementById('note').value
            };

            try {
                const res = await fetch('/api/transfer', {
                    method: 'POST',
                    headers: {'Content-Type': 'application/json'},
                    body: JSON.stringify(payload)
                });
                const data = await res.json();
                feedback.style.display = 'block';

                if (res.ok && data.success) {
                    feedback.style.color = '#10b981';
                    feedback.innerHTML = `✓ Transferred $${data.amount} successfully! Ref: ${data.reference}`;
                    setTimeout(() => window.location.reload(), 1200);
                } else {
                    feedback.style.color = '#f43f5e';
                    feedback.innerText = `✗ Failed: ${data.error || 'Unknown error'}`;
                    submitBtn.disabled = false;
                    submitBtn.innerText = '⚡ Execute Immediate Transfer';
                }
            } catch (err) {
                feedback.style.display = 'block';
                feedback.style.color = '#f43f5e';
                feedback.innerText = 'Network error executing transfer.';
                submitBtn.disabled = false;
                submitBtn.innerText = '⚡ Execute Immediate Transfer';
            }
        }
    </script>
</body>
</html>
