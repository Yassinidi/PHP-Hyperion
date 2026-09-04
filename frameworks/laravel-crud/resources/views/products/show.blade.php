<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>⚡ {{ $product->name }} — PHP-Hyperion CRUD</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&family=Outfit:wght@600;700;800&family=Fira+Code:wght@400;500&display=swap" rel="stylesheet">
    <style>
        :root {
            --bg: #090d16;
            --surface: #0f172a;
            --card-bg: rgba(18, 24, 43, 0.75);
            --card-border: rgba(255, 255, 255, 0.08);
            --accent: #6366f1;
            --accent-hover: #4f46e5;
            --text-primary: #f8fafc;
            --text-secondary: #94a3b8;
            --text-muted: #64748b;
        }

        body {
            background-color: var(--bg);
            color: var(--text-primary);
            font-family: 'Inter', sans-serif;
            min-height: 100vh;
            padding: 3rem 1.5rem;
        }

        .container {
            max-width: 800px;
            margin: 0 auto;
        }

        .card {
            background: var(--card-bg);
            border: 1px solid var(--card-border);
            border-radius: 16px;
            padding: 2.5rem;
            box-shadow: 0 20px 40px rgba(0, 0, 0, 0.4);
        }

        .back-link {
            color: var(--accent);
            text-decoration: none;
            font-weight: 600;
            display: inline-flex;
            align-items: center;
            gap: 0.5rem;
            margin-bottom: 1.5rem;
        }

        .title {
            font-family: 'Outfit', sans-serif;
            font-size: 2rem;
            font-weight: 700;
            margin-bottom: 0.5rem;
        }

        .sku {
            font-family: 'Fira Code', monospace;
            color: var(--text-muted);
            margin-bottom: 1.5rem;
        }

        .price-tag {
            font-size: 1.75rem;
            font-weight: 800;
            color: #fff;
            margin-bottom: 1.5rem;
        }

        .desc {
            color: var(--text-secondary);
            line-height: 1.6;
            margin-bottom: 2rem;
        }
    </style>
</head>
<body>
    <div class="container">
        <a href="/" class="back-link">← Back to Products List</a>
        <div class="card">
            <h1 class="title">{{ $product->name }}</h1>
            <div class="sku">SKU: {{ $product->sku }} | Category: {{ $product->category ? $product->category->name : 'Uncategorized' }}</div>
            <div class="price-tag">${{ number_format($product->price, 2) }}</div>
            <div class="desc">{{ $product->description }}</div>
            <div>Stock Remaining: <strong>{{ $product->stock }} units</strong> ({{ $product->status }})</div>
        </div>
    </div>
</body>
</html>
