<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>@yield('title', ($siteSettings['store_name'] ?? 'Hyperion Enterprise Store'))</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Fira+Code:wght@400;600&family=Inter:wght@300;400;500;600;700;800&family=Outfit:wght@300;400;500;600;700;800&family=Plus+Jakarta+Sans:wght@300;400;500;600;700;800&display=swap" rel="stylesheet">
    
    <style id="hyperion-theme-variables">
        :root {
            /* Fallback defaults */
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
            --theme-radius: 12px;
            --theme-font: 'Plus Jakarta Sans', sans-serif;

            /* Dynamic Active Theme Variables */
            {!! $currentTheme?->toCssVariables() ?? '' !!}
        }
        * {
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }
        body {
            font-family: var(--theme-font, 'Plus Jakarta Sans', sans-serif);
            background-color: var(--bg-base);
            color: var(--text-main);
            line-height: 1.6;
            min-height: 100vh;
            display: flex;
            flex-direction: column;
            transition: background-color 0.25s ease, color 0.25s ease;
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
            padding: 0.85rem 1.5rem;
            display: flex;
            align-items: center;
            justify-content: space-between;
            flex-wrap: wrap;
            gap: 1rem;
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
            background: linear-gradient(135deg, var(--accent), var(--accent-green));
            border-radius: var(--theme-radius, 10px);
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
            color: var(--accent);
            padding: 0.2rem 0.5rem;
            border-radius: 6px;
            border: 1px solid var(--border-highlight);
            margin-left: 0.5rem;
        }
        nav {
            display: flex;
            align-items: center;
            gap: 1.25rem;
            flex-wrap: wrap;
        }
        nav a {
            color: var(--text-muted);
            text-decoration: none;
            font-size: 0.9rem;
            font-weight: 500;
            transition: color 0.2s;
        }
        nav a:hover, nav a.active {
            color: var(--text-main);
        }
        .btn-customizer-nav {
            display: inline-flex;
            align-items: center;
            gap: 0.4rem;
            background: linear-gradient(135deg, var(--accent), var(--accent-green));
            color: white !important;
            padding: 0.4rem 0.85rem;
            border-radius: var(--theme-radius, 8px);
            font-weight: 700 !important;
            font-size: 0.85rem !important;
            box-shadow: 0 0 12px var(--accent-glow);
            transition: transform 0.2s, box-shadow 0.2s;
        }
        .btn-customizer-nav:hover {
            transform: translateY(-1px);
            box-shadow: 0 0 18px var(--accent-glow);
        }
        .cart-nav-btn {
            background: rgba(255, 255, 255, 0.05);
            border: 1px solid var(--border-color);
            color: var(--text-main);
            padding: 0.4rem 0.85rem;
            border-radius: var(--theme-radius, 8px);
            cursor: pointer;
            display: inline-flex;
            align-items: center;
            gap: 0.5rem;
            font-weight: 600;
            font-size: 0.85rem;
            transition: all 0.2s;
        }
        .cart-nav-btn:hover {
            border-color: var(--accent);
            background: rgba(99, 102, 241, 0.1);
        }
        .btn {
            background: var(--accent);
            color: white;
            border: none;
            padding: 0.6rem 1.25rem;
            border-radius: var(--theme-radius, 8px);
            cursor: pointer;
            font-weight: 600;
            font-size: 0.9rem;
            text-decoration: none;
            display: inline-flex;
            align-items: center;
            justify-content: center;
            gap: 0.4rem;
            transition: all 0.2s;
        }
        .btn:hover {
            opacity: 0.9;
            transform: translateY(-1px);
        }
        .btn-primary {
            background: var(--accent);
            color: white;
        }
        .btn-outline {
            background: transparent;
            border: 1px solid var(--border-highlight);
            color: var(--accent);
        }
        .btn-outline:hover {
            background: rgba(99, 102, 241, 0.1);
        }
        .btn-subtle {
            background: rgba(255, 255, 255, 0.06);
            color: var(--text-muted);
        }
        .btn-subtle:hover {
            color: var(--text-main);
            background: rgba(255, 255, 255, 0.1);
        }
        main {
            flex: 1;
            width: 100%;
        }
        footer {
            border-top: 1px solid var(--border-color);
            padding: 2.5rem 1.5rem;
            text-align: center;
            font-size: 0.85rem;
            color: var(--text-muted);
            background: rgba(0, 0, 0, 0.2);
        }
    </style>
    @yield('styles')
</head>
<body>
    <!-- No-Code Dynamic Announcement Bar -->
    @if(!empty($siteSettings['announcement_enabled']) && $siteSettings['announcement_enabled'] === 'true')
        <div class="announcement-bar" style="background: linear-gradient(90deg, var(--accent), var(--accent-green)); color: white; padding: 0.45rem 1.5rem; text-align: center; font-size: 0.85rem; font-weight: 700; display: flex; align-items: center; justify-content: center; gap: 0.5rem; letter-spacing: 0.02em;">
            <span>{{ $siteSettings['announcement_text'] ?? '⚡ Welcome to Hyperion Pro Store!' }}</span>
            @if(!empty($siteSettings['announcement_link']))
                <a href="{{ $siteSettings['announcement_link'] }}" style="color: white; text-decoration: underline; font-weight: 800; margin-left: 0.5rem;">Check Out →</a>
            @endif
        </div>
    @endif

    <header>
        <div class="header-inner">
            <a href="{{ route('store.index') }}" class="brand">
                <div class="brand-logo">⚡</div>
                <div class="brand-text">{{ $siteSettings['store_name'] ?? 'Hyperion Pro Store' }}</div>
                <span class="brand-tag">v1.0 Pro</span>
            </a>
            <nav>
                <a href="{{ route('store.index') }}" class="{{ request()->routeIs('store.index') ? 'active' : '' }}">Catalog</a>
                <a href="{{ route('store.orders') }}" class="{{ request()->routeIs('store.orders') ? 'active' : '' }}">Orders & Audits</a>
                <a href="{{ route('store.operations') }}" class="{{ request()->routeIs('store.operations') ? 'active' : '' }}">Queues & Ops</a>
                <a href="{{ route('store.complex') }}" class="{{ request()->routeIs('store.complex') ? 'active' : '' }}" id="nav-complex-lab">⚡ Complex Lab</a>
                
                <!-- Theme Customizer Trigger -->
                <a href="{{ route('customizer.index') }}" class="btn-customizer-nav">
                    🎨 Visual Customizer
                </a>

                <!-- Interactive Cart Trigger Button -->
                <button type="button" onclick="openCartDrawer()" class="cart-nav-btn">
                    <span>🛒 Cart</span>
                    <span id="nav-cart-badge" class="drawer-badge" style="{{ ($cartCount ?? 0) > 0 ? '' : 'display: none;' }}">{{ $cartCount ?? 0 }}</span>
                </button>
            </nav>
        </div>
    </header>

    <main>
        @yield('content')
    </main>

    <footer>
        <div style="max-width: 1280px; margin: 0 auto; display: flex; flex-direction: column; gap: 1rem; align-items: center;">
            <p>⚡ Powered by <strong>PHP-Hyperion v1.0 Enterprise Engine</strong> | 198,000 req/s &bull; Zero State Bleed &bull; Multi-Reactor Architecture</p>
            <div style="display: flex; gap: 1.5rem; font-size: 0.8rem;">
                <a href="{{ route('store.page', 'about') }}" style="color: var(--text-muted); text-decoration: none;">About Us</a>
                <a href="{{ route('customizer.index') }}" style="color: var(--text-muted); text-decoration: none;">Theme Studio</a>
                <a href="{{ route('cart.index') }}" style="color: var(--text-muted); text-decoration: none;">Cart & Checkout</a>
                <a href="/api/v1/products" target="_blank" style="color: var(--text-muted); text-decoration: none;">Products API</a>
                <a href="/health" target="_blank" style="color: var(--text-muted); text-decoration: none;">System Health</a>
            </div>
        </div>
    </footer>

    <!-- Slide-over Animated Cart Drawer -->
    @include('_partials.cart_drawer')

    @yield('scripts')
</body>
</html>
