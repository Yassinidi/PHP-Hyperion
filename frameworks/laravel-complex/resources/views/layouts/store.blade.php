<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>@yield('title', 'Hyperion Enterprise Store')</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:wght@300;400;500;600;700;800&display=swap" rel="stylesheet">
    <style>
        :root {
            --bg-base: #0a0e17;
            --bg-card: #131b2e;
            --bg-card-hover: #1b2640;
            --accent: #6366f1;
            --accent-glow: rgba(99, 102, 241, 0.35);
            --accent-green: #10b981;
            --text-main: #f8fafc;
            --text-muted: #94a3b8;
            --border-color: rgba(255, 255, 255, 0.08);
            --border-highlight: rgba(99, 102, 241, 0.4);
        }
        * {
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }
        body {
            font-family: 'Plus Jakarta Sans', sans-serif;
            background-color: var(--bg-base);
            color: var(--text-main);
            line-height: 1.6;
            min-height: 100vh;
            display: flex;
            flex-direction: column;
        }
        header {
            border-bottom: 1px solid var(--border-color);
            background: rgba(10, 14, 23, 0.85);
            backdrop-filter: blur(12px);
            position: sticky;
            top: 0;
            z-index: 50;
        }
        .header-inner {
            max-width: 1280px;
            margin: 0 auto;
            padding: 1rem 1.5rem;
            display: flex;
            align-items: center;
            justify-content: space-between;
        }
        .brand {
            display: flex;
            align-items: center;
            gap: 0.75rem;
            text-decoration: none;
            color: inherit;
        }
        .brand-logo {
            width: 36px;
            height: 36px;
            background: linear-gradient(135deg, #6366f1, #a855f7);
            border-radius: 10px;
            display: flex;
            align-items: center;
            justify-content: center;
            font-weight: 800;
            font-size: 1.25rem;
            color: white;
            box-shadow: 0 0 15px var(--accent-glow);
        }
        .brand-text {
            font-size: 1.25rem;
            font-weight: 700;
            letter-spacing: -0.02em;
        }
        .brand-tag {
            font-size: 0.75rem;
            font-weight: 600;
            background: rgba(99, 102, 241, 0.15);
            color: #818cf8;
            padding: 0.2rem 0.5rem;
            border-radius: 6px;
            border: 1px solid rgba(99, 102, 241, 0.3);
            margin-left: 0.5rem;
        }
        nav a {
            color: var(--text-muted);
            text-decoration: none;
            font-size: 0.9rem;
            font-weight: 500;
            margin-left: 1.5rem;
            transition: color 0.2s;
        }
        nav a:hover, nav a.active {
            color: var(--text-main);
        }
        main {
            flex: 1;
            max-width: 1280px;
            margin: 0 auto;
            padding: 2.5rem 1.5rem;
            width: 100%;
        }
        footer {
            border-top: 1px solid var(--border-color);
            padding: 2rem 1.5rem;
            text-align: center;
            font-size: 0.85rem;
            color: var(--text-muted);
        }
        .container {
            max-width: 1280px;
            margin: 0 auto;
        }
    </style>
    @yield('styles')
</head>
<body>
    <header>
        <div class="header-inner">
            <a href="{{ route('store.index') }}" class="brand">
                <div class="brand-logo">⚡</div>
                <div class="brand-text">Hyperion Store</div>
                <span class="brand-tag">PHP 8.4 Enterprise</span>
            </a>
            <nav>
                <a href="{{ route('store.index') }}" class="{{ request()->routeIs('store.index') ? 'active' : '' }}">Catalog</a>
                <a href="{{ route('store.orders') }}" class="{{ request()->routeIs('store.orders') ? 'active' : '' }}">Orders & Audits</a>
                <a href="{{ route('store.operations') }}" class="{{ request()->routeIs('store.operations') ? 'active' : '' }}">Queues & Ops</a>
                <a href="{{ route('store.complex') }}" class="{{ request()->routeIs('store.complex') ? 'active' : '' }}" id="nav-complex-lab">⚡ Complex Lab</a>
                <a href="/api/v1/products" target="_blank">Products API</a>
                <a href="/api/v1/analytics/dashboard" target="_blank">Analytics JSON</a>

                <a href="/health" target="_blank">System Health</a>
            </nav>
        </div>
    </header>

    <main>
        @yield('content')
    </main>

    <footer>
        <p>⚡ Powered by <strong>PHP-Hyperion v1.0 Enterprise Engine</strong> | Zero Latency &bull; Zero State Bleed &bull; Multi-Reactor Architecture</p>
    </footer>
</body>
</html>
