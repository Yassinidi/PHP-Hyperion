<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Enterprise CRM Portal</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; background: #0f172a; color: #f8fafc; padding: 2rem; margin: 0; }
        .card { background: #1e293b; border-radius: 8px; padding: 1.5rem; margin-bottom: 1.5rem; border: 1px solid #334155; }
        .grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 1rem; }
        .stat { font-size: 2rem; font-weight: bold; color: #38bdf8; }
        h1 { margin-top: 0; font-size: 1.75rem; color: #ffffff; }
        table { width: 100%; border-collapse: collapse; margin-top: 1rem; }
        th, td { text-align: left; padding: 0.75rem; border-bottom: 1px solid #334155; }
        th { color: #94a3b8; font-size: 0.85rem; text-transform: uppercase; }
    </style>
</head>
<body>
    <h1>🏢 Hyperion CRM Enterprise Console</h1>
    
    <div class="grid">
        <div class="card">
            <div>Total Leads</div>
            <div class="stat">{{ $totalLeads }}</div>
        </div>
        <div class="card">
            <div>Active Pipeline</div>
            <div class="stat">${{ number_format($totalPipelineValue, 2) }}</div>
        </div>
        <div class="card">
            <div>Deals Closed Won</div>
            <div class="stat">{{ $dealsWon }}</div>
        </div>
    </div>

    <div class="card">
        <h2>Recent Leads Ingested</h2>
        <table>
            <thead>
                <tr>
                    <th>ID</th>
                    <th>Name</th>
                    <th>Company</th>
                    <th>Email</th>
                    <th>Status</th>
                </tr>
            </thead>
            <tbody>
                @foreach ($recentLeads as $lead)
                <tr>
                    <td>#{{ $lead->id }}</td>
                    <td>{{ $lead->first_name }} {{ $lead->last_name }}</td>
                    <td>{{ $lead->company }}</td>
                    <td>{{ $lead->email }}</td>
                    <td>{{ $lead->status }}</td>
                </tr>
                @endforeach
            </tbody>
        </table>
    </div>
</body>
</html>
