<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Aura Storefront — High-Performance E-Commerce</title>
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
        .navbar { display: flex; justify-content: space-between; align-items: center; padding: 1rem 2rem; background: rgba(17, 24, 39, 0.8); backdrop-filter: blur(12px); border-bottom: 1px solid var(--border-color); position: sticky; top: 0; z-index: 50; }
        .logo { font-size: 1.25rem; font-weight: 800; background: linear-gradient(135deg, #818cf8, #c084fc); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }
        .nav-links { display: flex; gap: 1.5rem; align-items: center; }
        .nav-links a { color: var(--text-muted); text-decoration: none; font-size: 0.9rem; font-weight: 500; transition: color 0.2s; }
        .nav-links a:hover { color: var(--text-main); }
        .container { max-width: 1200px; margin: 2rem auto; padding: 0 1.5rem; }
        .banners-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 1.5rem; margin-bottom: 2.5rem; }
        .banner-card { background: linear-gradient(145deg, #1e1b4b, #311042); border: 1px solid rgba(99, 102, 241, 0.3); border-radius: 12px; padding: 1.75rem; display: flex; flex-direction: column; justify-content: space-between; min-height: 160px; box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.5); }
        .banner-badge { font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; padding: 0.25rem 0.6rem; border-radius: 9999px; background: rgba(255, 255, 255, 0.1); width: fit-content; margin-bottom: 0.5rem; }
        .banner-title { font-size: 1.25rem; font-weight: 700; margin-bottom: 0.35rem; }
        .banner-subtitle { font-size: 0.85rem; color: #cbd5e1; margin-bottom: 1rem; }
        .section-title { font-size: 1.35rem; font-weight: 700; margin-bottom: 1.25rem; display: flex; align-items: center; gap: 0.5rem; }
        .categories-chips { display: flex; gap: 0.75rem; overflow-x: auto; margin-bottom: 2rem; padding-bottom: 0.5rem; }
        .chip { background: var(--bg-card); border: 1px solid var(--border-color); padding: 0.5rem 1rem; border-radius: 9999px; font-size: 0.85rem; color: var(--text-muted); text-decoration: none; display: flex; align-items: center; gap: 0.4rem; transition: all 0.2s; }
        .chip:hover, .chip.active { background: var(--accent); color: #fff; border-color: var(--accent); }
        .products-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 1.5rem; }
        .product-card { background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 12px; padding: 1.25rem; display: flex; flex-direction: column; justify-content: space-between; transition: transform 0.2s, border-color 0.2s; }
        .product-card:hover { transform: translateY(-4px); border-color: rgba(99, 102, 241, 0.5); }
        .product-header { display: flex; justify-content: space-between; font-size: 0.75rem; color: var(--text-muted); margin-bottom: 0.5rem; }
        .product-title { font-size: 1.05rem; font-weight: 600; margin-bottom: 0.4rem; }
        .product-desc { font-size: 0.8rem; color: var(--text-muted); line-height: 1.4; margin-bottom: 1rem; }
        .product-footer { display: flex; justify-content: space-between; align-items: center; margin-top: auto; padding-top: 0.75rem; border-top: 1px solid rgba(255, 255, 255, 0.05); }
        .price { font-size: 1.2rem; font-weight: 800; color: #34d399; }
        .btn-cart { background: var(--accent); color: white; border: none; padding: 0.45rem 0.9rem; border-radius: 6px; font-size: 0.8rem; font-weight: 600; cursor: pointer; transition: opacity 0.2s; }
        .btn-cart:hover { opacity: 0.9; }
    </style>
</head>
<body>
    <nav class="navbar">
        <div class="logo">⚡ Aura Storefront (Native PHP)</div>
        <div class="nav-links">
            <a href="/">Storefront</a>
            <a href="/api/products">Products API</a>
            <a href="/api/categories">Categories API</a>
            <a href="/admin">Admin Dashboard</a>
        </div>
    </nav>

    <div class="container">
        <!-- Hero Banners -->
        <div class="banners-grid">
            <?php foreach ($banners as $b): ?>
            <div class="banner-card">
                <div>
                    <div class="banner-badge"><?= htmlspecialchars($b['badge'] ?? 'FEATURED') ?></div>
                    <div class="banner-title"><?= htmlspecialchars($b['title']) ?></div>
                    <div class="banner-subtitle"><?= htmlspecialchars($b['subtitle']) ?></div>
                </div>
                <a href="#" style="color: #a5b4fc; font-size: 0.85rem; font-weight: 600; text-decoration: none;">Shop Now &rarr;</a>
            </div>
            <?php endforeach; ?>
        </div>

        <!-- Categories -->
        <div class="section-title">📂 Explore Categories</div>
        <div class="categories-chips">
            <a href="/" class="chip <?= !isset($_GET['category_id']) ? 'active' : '' ?>">All Products</a>
            <?php foreach ($categories as $cat): ?>
            <a href="/?category_id=<?= $cat['id'] ?>" class="chip <?= (isset($_GET['category_id']) && (int)$_GET['category_id'] === (int)$cat['id']) ? 'active' : '' ?>">
                <span><?= $cat['icon'] ?></span>
                <span><?= htmlspecialchars($cat['name']) ?></span>
            </a>
            <?php endforeach; ?>
        </div>

        <!-- Product Grid -->
        <div class="section-title">🛍️ Featured Products</div>
        <div class="products-grid">
            <?php foreach ($products as $p): ?>
            <div class="product-card">
                <div>
                    <div class="product-header">
                        <span>SKU: <?= htmlspecialchars($p['slug'] ?? ('SKU-' . $p['id'])) ?></span>
                        <span style="color: <?= $p['stock'] < 30 ? '#f87171' : '#34d399' ?>;">Stock: <?= $p['stock'] ?></span>
                    </div>
                    <div class="product-title"><?= htmlspecialchars($p['name'] ?? 'Product') ?></div>
                    <div class="product-desc"><?= htmlspecialchars($p['category_name'] ?? 'Premium Hardware') ?></div>
                </div>
                <div class="product-footer">
                    <div class="price">$<?= number_format((float)$p['price'], 2) ?></div>
                    <button class="btn-cart" onclick="alert('Added <?= htmlspecialchars($p['name'] ?? 'Product') ?> to cart!')">Add to Cart</button>
                </div>
            </div>
            <?php endforeach; ?>
        </div>
    </div>
</body>
</html>
