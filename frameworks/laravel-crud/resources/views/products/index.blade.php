<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="csrf-token" content="{{ csrf_token() }}">
    <title>⚡ PHP-Hyperion — Inventory & Product CRUD Studio</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800&family=Outfit:wght@600;700;800&family=Fira+Code:wght@400;500&display=swap" rel="stylesheet">
    <style>
        :root {
            --bg: #090d16;
            --surface: #0f172a;
            --card-bg: rgba(18, 24, 43, 0.75);
            --card-border: rgba(255, 255, 255, 0.08);
            --accent: #6366f1;
            --accent-hover: #4f46e5;
            --accent-glow: rgba(99, 102, 241, 0.35);
            --success: #10b981;
            --danger: #f43f5e;
            --warning: #f59e0b;
            --info: #0ea5e9;
            --text-primary: #f8fafc;
            --text-secondary: #94a3b8;
            --text-muted: #64748b;
        }

        * {
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }

        body {
            background-color: var(--bg);
            color: var(--text-primary);
            font-family: 'Inter', sans-serif;
            min-height: 100vh;
            background-image: 
                radial-gradient(circle at 15% 15%, rgba(99, 102, 241, 0.12) 0%, transparent 40%),
                radial-gradient(circle at 85% 85%, rgba(14, 165, 233, 0.10) 0%, transparent 40%);
            background-attachment: fixed;
        }

        /* Top Navigation */
        .navbar {
            background: rgba(15, 23, 42, 0.85);
            backdrop-filter: blur(12px);
            border-bottom: 1px solid var(--card-border);
            padding: 1rem 2rem;
            position: sticky;
            top: 0;
            z-index: 100;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }

        .brand {
            display: flex;
            align-items: center;
            gap: 0.75rem;
            font-family: 'Outfit', sans-serif;
            font-size: 1.25rem;
            font-weight: 700;
            color: #fff;
            text-decoration: none;
        }

        .brand-badge {
            background: linear-gradient(135deg, #6366f1, #0ea5e9);
            color: #fff;
            font-size: 0.7rem;
            padding: 0.2rem 0.5rem;
            border-radius: 6px;
            font-weight: 800;
            letter-spacing: 0.5px;
            text-transform: uppercase;
        }

        .nav-actions {
            display: flex;
            align-items: center;
            gap: 1rem;
        }

        .btn {
            display: inline-flex;
            align-items: center;
            gap: 0.5rem;
            padding: 0.6rem 1.2rem;
            border-radius: 8px;
            font-size: 0.875rem;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.2s ease;
            text-decoration: none;
            border: none;
        }

        .btn-primary {
            background: var(--accent);
            color: #fff;
            box-shadow: 0 4px 14px var(--accent-glow);
        }

        .btn-primary:hover {
            background: var(--accent-hover);
            transform: translateY(-1px);
        }

        .btn-secondary {
            background: rgba(255, 255, 255, 0.06);
            color: var(--text-primary);
            border: 1px solid var(--card-border);
        }

        .btn-secondary:hover {
            background: rgba(255, 255, 255, 0.1);
        }

        .btn-danger {
            background: rgba(244, 63, 94, 0.15);
            color: var(--danger);
            border: 1px solid rgba(244, 63, 94, 0.3);
        }

        .btn-danger:hover {
            background: var(--danger);
            color: #fff;
        }

        .btn-sm {
            padding: 0.35rem 0.65rem;
            font-size: 0.75rem;
            border-radius: 6px;
        }

        /* Container Layout */
        .container {
            max-width: 1350px;
            margin: 0 auto;
            padding: 2rem;
        }

        /* Metrics Banner */
        .metrics-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
            gap: 1.25rem;
            margin-bottom: 2rem;
        }

        .metric-card {
            background: var(--card-bg);
            border: 1px solid var(--card-border);
            backdrop-filter: blur(10px);
            border-radius: 12px;
            padding: 1.25rem;
            display: flex;
            align-items: center;
            gap: 1rem;
            transition: transform 0.2s, border-color 0.2s;
        }

        .metric-card:hover {
            transform: translateY(-2px);
            border-color: rgba(99, 102, 241, 0.4);
        }

        .metric-icon {
            width: 48px;
            height: 48px;
            border-radius: 10px;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 1.5rem;
            background: rgba(99, 102, 241, 0.15);
            color: var(--accent);
        }

        .metric-info h3 {
            font-size: 0.8rem;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            color: var(--text-secondary);
            margin-bottom: 0.25rem;
        }

        .metric-info .metric-val {
            font-family: 'Outfit', sans-serif;
            font-size: 1.5rem;
            font-weight: 700;
            color: #fff;
        }

        /* Search & Filters Bar */
        .filter-panel {
            background: var(--card-bg);
            border: 1px solid var(--card-border);
            border-radius: 12px;
            padding: 1.25rem;
            margin-bottom: 1.5rem;
            display: flex;
            flex-wrap: wrap;
            gap: 1rem;
            align-items: center;
            justify-content: space-between;
        }

        .search-box {
            position: relative;
            flex: 1;
            min-width: 260px;
        }

        .search-box input {
            width: 100%;
            padding: 0.65rem 1rem 0.65rem 2.5rem;
            background: rgba(15, 23, 42, 0.7);
            border: 1px solid var(--card-border);
            border-radius: 8px;
            color: #fff;
            font-size: 0.875rem;
            outline: none;
            transition: border-color 0.2s;
        }

        .search-box input:focus {
            border-color: var(--accent);
            box-shadow: 0 0 0 3px var(--accent-glow);
        }

        .search-icon {
            position: absolute;
            left: 0.85rem;
            top: 50%;
            transform: translateY(-50%);
            color: var(--text-muted);
        }

        .filter-controls {
            display: flex;
            flex-wrap: wrap;
            gap: 0.75rem;
            align-items: center;
        }

        .select-input {
            background: rgba(15, 23, 42, 0.7);
            border: 1px solid var(--card-border);
            color: #fff;
            padding: 0.65rem 1rem;
            border-radius: 8px;
            font-size: 0.875rem;
            outline: none;
            cursor: pointer;
        }

        /* Products Table */
        .table-card {
            background: var(--card-bg);
            border: 1px solid var(--card-border);
            border-radius: 12px;
            overflow: hidden;
            box-shadow: 0 8px 30px rgba(0, 0, 0, 0.3);
        }

        table {
            width: 100%;
            border-collapse: collapse;
            text-align: left;
            font-size: 0.875rem;
        }

        th {
            background: rgba(15, 23, 42, 0.9);
            color: var(--text-secondary);
            font-weight: 600;
            padding: 1rem 1.25rem;
            border-bottom: 1px solid var(--card-border);
            text-transform: uppercase;
            font-size: 0.75rem;
            letter-spacing: 0.5px;
        }

        td {
            padding: 1rem 1.25rem;
            border-bottom: 1px solid var(--card-border);
            vertical-align: middle;
        }

        tr:hover td {
            background: rgba(255, 255, 255, 0.02);
        }

        .product-meta {
            display: flex;
            align-items: center;
            gap: 0.85rem;
        }

        .product-thumb {
            width: 44px;
            height: 44px;
            border-radius: 8px;
            object-fit: cover;
            background: #1e293b;
            border: 1px solid var(--card-border);
        }

        .product-name {
            font-weight: 600;
            color: #fff;
            display: block;
            margin-bottom: 0.15rem;
        }

        .product-sku {
            font-family: 'Fira Code', monospace;
            font-size: 0.75rem;
            color: var(--text-muted);
        }

        .badge {
            display: inline-flex;
            align-items: center;
            gap: 0.35rem;
            padding: 0.25rem 0.6rem;
            border-radius: 6px;
            font-size: 0.75rem;
            font-weight: 600;
        }

        .badge-category {
            background: rgba(99, 102, 241, 0.15);
            color: #818cf8;
            border: 1px solid rgba(99, 102, 241, 0.3);
        }

        .badge-in-stock {
            background: rgba(16, 185, 129, 0.15);
            color: var(--success);
            border: 1px solid rgba(16, 185, 129, 0.3);
        }

        .badge-low-stock {
            background: rgba(245, 158, 11, 0.15);
            color: var(--warning);
            border: 1px solid rgba(245, 158, 11, 0.3);
        }

        .badge-out-of-stock {
            background: rgba(244, 63, 94, 0.15);
            color: var(--danger);
            border: 1px solid rgba(244, 63, 94, 0.3);
        }

        .stock-control {
            display: inline-flex;
            align-items: center;
            gap: 0.4rem;
            background: rgba(15, 23, 42, 0.6);
            border: 1px solid var(--card-border);
            padding: 0.2rem 0.4rem;
            border-radius: 6px;
        }

        .stock-btn {
            background: none;
            border: none;
            color: var(--text-secondary);
            font-weight: bold;
            font-size: 0.9rem;
            cursor: pointer;
            padding: 0.1rem 0.4rem;
            border-radius: 4px;
            transition: background 0.15s;
        }

        .stock-btn:hover {
            background: rgba(255, 255, 255, 0.1);
            color: #fff;
        }

        .actions-cell {
            display: flex;
            gap: 0.5rem;
        }

        /* Modal Structure */
        .modal-overlay {
            position: fixed;
            top: 0;
            left: 0;
            right: 0;
            bottom: 0;
            background: rgba(0, 0, 0, 0.75);
            backdrop-filter: blur(8px);
            display: none;
            align-items: center;
            justify-content: center;
            z-index: 1000;
            padding: 1.5rem;
        }

        .modal-overlay.active {
            display: flex;
        }

        .modal-content {
            background: #0f172a;
            border: 1px solid var(--card-border);
            border-radius: 16px;
            width: 100%;
            max-width: 600px;
            padding: 2rem;
            box-shadow: 0 20px 50px rgba(0, 0, 0, 0.6);
            position: relative;
        }

        .modal-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 1.5rem;
        }

        .modal-title {
            font-family: 'Outfit', sans-serif;
            font-size: 1.35rem;
            color: #fff;
        }

        .close-btn {
            background: none;
            border: none;
            color: var(--text-muted);
            font-size: 1.5rem;
            cursor: pointer;
            line-height: 1;
        }

        .form-grid {
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 1rem;
        }

        .form-group {
            margin-bottom: 1rem;
        }

        .form-group.full {
            grid-column: span 2;
        }

        .form-label {
            display: block;
            font-size: 0.8rem;
            font-weight: 600;
            color: var(--text-secondary);
            margin-bottom: 0.4rem;
        }

        .form-input, .form-textarea, .form-select {
            width: 100%;
            padding: 0.65rem 0.85rem;
            background: rgba(18, 24, 43, 0.8);
            border: 1px solid var(--card-border);
            border-radius: 8px;
            color: #fff;
            font-size: 0.875rem;
            outline: none;
        }

        .form-input:focus, .form-textarea:focus, .form-select:focus {
            border-color: var(--accent);
            box-shadow: 0 0 0 2px var(--accent-glow);
        }

        .form-textarea {
            resize: vertical;
            min-height: 80px;
        }

        .modal-footer {
            display: flex;
            justify-content: flex-end;
            gap: 0.75rem;
            margin-top: 1.5rem;
        }

        /* Toast Alert */
        .toast {
            position: fixed;
            bottom: 2rem;
            right: 2rem;
            background: var(--surface);
            border: 1px solid var(--accent);
            color: #fff;
            padding: 1rem 1.5rem;
            border-radius: 10px;
            box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
            display: none;
            align-items: center;
            gap: 0.75rem;
            z-index: 2000;
            animation: slideIn 0.3s ease;
        }

        @keyframes slideIn {
            from { transform: translateX(100%); opacity: 0; }
            to { transform: translateX(0); opacity: 1; }
        }
    </style>
</head>
<body>

    <!-- Navigation -->
    <header class="navbar">
        <a href="/" class="brand">
            ⚡ PHP-Hyperion
            <span class="brand-badge">CRUD Studio</span>
        </a>
        <div class="nav-actions">
            <button class="btn btn-secondary" onclick="openCategoryModal()">
                <span>🏷️</span> New Category
            </button>
            <button class="btn btn-primary" onclick="openCreateModal()">
                <span>➕</span> Add Product
            </button>
        </div>
    </header>

    <main class="container">
        <!-- Live Inventory Metrics -->
        <section class="metrics-grid">
            <div class="metric-card">
                <div class="metric-icon">📦</div>
                <div class="metric-info">
                    <h3>Total Products</h3>
                    <div class="metric-val" id="metric-total-products">{{ $stats['total_products'] }}</div>
                </div>
            </div>
            <div class="metric-card">
                <div class="metric-icon" style="background: rgba(16, 185, 129, 0.15); color: var(--success);">💰</div>
                <div class="metric-info">
                    <h3>Total Valuation</h3>
                    <div class="metric-val">${{ number_format($stats['total_inventory_value'], 2) }}</div>
                </div>
            </div>
            <div class="metric-card">
                <div class="metric-icon" style="background: rgba(245, 158, 11, 0.15); color: var(--warning);">⚠️</div>
                <div class="metric-info">
                    <h3>Low Stock Items</h3>
                    <div class="metric-val">{{ $stats['low_stock_count'] }}</div>
                </div>
            </div>
            <div class="metric-card">
                <div class="metric-icon" style="background: rgba(14, 165, 233, 0.15); color: var(--info);">🏷️</div>
                <div class="metric-info">
                    <h3>Categories</h3>
                    <div class="metric-val">{{ $categories->count() }}</div>
                </div>
            </div>
        </section>

        <!-- Search & Filters Toolbar -->
        <section class="filter-panel">
            <div class="search-box">
                <span class="search-icon">🔍</span>
                <input type="text" id="searchInput" placeholder="Search by name, SKU, description..." onkeyup="handleSearch(event)">
            </div>
            <div class="filter-controls">
                <select class="select-input" id="categoryFilter" onchange="applyFilters()">
                    <option value="">All Categories</option>
                    @foreach($categories as $cat)
                        <option value="{{ $cat->slug }}">{{ $cat->icon }} {{ $cat->name }} ({{ $cat->products_count }})</option>
                    @endforeach
                </select>

                <select class="select-input" id="statusFilter" onchange="applyFilters()">
                    <option value="">All Stock Status</option>
                    <option value="in_stock">In Stock</option>
                    <option value="low_stock">Low Stock (≤10)</option>
                    <option value="out_of_stock">Out of Stock</option>
                </select>

                <select class="select-input" id="sortFilter" onchange="applyFilters()">
                    <option value="latest">Newest First</option>
                    <option value="price_asc">Price: Low to High</option>
                    <option value="price_desc">Price: High to Low</option>
                    <option value="stock_desc">Stock: High to Low</option>
                    <option value="name_asc">Name: A-Z</option>
                </select>
            </div>
        </section>

        <!-- Products Table -->
        <section class="table-card">
            <table>
                <thead>
                    <tr>
                        <th>Product & SKU</th>
                        <th>Category</th>
                        <th>Price / Cost</th>
                        <th>Stock Level</th>
                        <th>Status</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody id="productsTableBody">
                    @forelse($products as $product)
                    <tr id="row-{{ $product->id }}">
                        <td>
                            <div class="product-meta">
                                <img src="{{ $product->image_url ?: 'https://images.unsplash.com/photo-1591488320449-011701bb6704?w=100&auto=format&fit=crop&q=80' }}" class="product-thumb" alt="thumb">
                                <div>
                                    <span class="product-name">{{ $product->name }}</span>
                                    <span class="product-sku">{{ $product->sku }}</span>
                                </div>
                            </div>
                        </td>
                        <td>
                            @if($product->category)
                                <span class="badge badge-category">{{ $product->category->icon }} {{ $product->category->name }}</span>
                            @else
                                <span class="badge badge-category">Uncategorized</span>
                            @endif
                        </td>
                        <td>
                            <strong style="color: #fff;">${{ number_format($product->price, 2) }}</strong>
                            <div style="font-size: 0.75rem; color: var(--text-muted);">Cost: ${{ number_format($product->cost, 2) }}</div>
                        </td>
                        <td>
                            <div class="stock-control">
                                <button class="stock-btn" onclick="adjustStock({{ $product->id }}, -1)">−</button>
                                <span id="stock-val-{{ $product->id }}" style="font-weight: 600; min-width: 24px; text-align: center;">{{ $product->stock }}</span>
                                <button class="stock-btn" onclick="adjustStock({{ $product->id }}, 1)">+</button>
                            </div>
                        </td>
                        <td>
                            @if($product->status === 'in_stock')
                                <span class="badge badge-in-stock" id="badge-{{ $product->id }}">● In Stock</span>
                            @elseif($product->status === 'low_stock')
                                <span class="badge badge-low-stock" id="badge-{{ $product->id }}">▲ Low Stock</span>
                            @else
                                <span class="badge badge-out-of-stock" id="badge-{{ $product->id }}">✕ Out of Stock</span>
                            @endif
                        </td>
                        <td>
                            <div class="actions-cell">
                                <button class="btn btn-secondary btn-sm" onclick="openEditModal({{ json_encode($product) }})">✏️ Edit</button>
                                <button class="btn btn-danger btn-sm" onclick="deleteProduct({{ $product->id }}, '{{ addslashes($product->name) }}')">🗑️</button>
                            </div>
                        </td>
                    </tr>
                    @empty
                    <tr>
                        <td colspan="6" style="text-align: center; padding: 3rem; color: var(--text-muted);">
                            No products found matching the criteria. Click "Add Product" to create one.
                        </td>
                    </tr>
                    @endforelse
                </tbody>
            </table>
        </section>
    </main>

    <!-- Create Product Modal -->
    <div class="modal-overlay" id="createModal">
        <div class="modal-content">
            <div class="modal-header">
                <h2 class="modal-title">➕ Add New Product</h2>
                <button class="close-btn" onclick="closeCreateModal()">×</button>
            </div>
            <form id="createForm" onsubmit="handleCreateProduct(event)">
                <div class="form-grid">
                    <div class="form-group full">
                        <label class="form-label">Product Name *</label>
                        <input type="text" name="name" class="form-input" required placeholder="e.g. Titan Liquid Workstation">
                    </div>
                    <div class="form-group">
                        <label class="form-label">Category</label>
                        <select name="category_id" class="form-select">
                            <option value="">Select Category</option>
                            @foreach($categories as $cat)
                                <option value="{{ $cat->id }}">{{ $cat->icon }} {{ $cat->name }}</option>
                            @endforeach
                        </select>
                    </div>
                    <div class="form-group">
                        <label class="form-label">SKU (Optional)</label>
                        <input type="text" name="sku" class="form-input" placeholder="Auto-generated if empty">
                    </div>
                    <div class="form-group">
                        <label class="form-label">Price ($) *</label>
                        <input type="number" step="0.01" name="price" class="form-input" required placeholder="299.99">
                    </div>
                    <div class="form-group">
                        <label class="form-label">Cost ($)</label>
                        <input type="number" step="0.01" name="cost" class="form-input" placeholder="150.00">
                    </div>
                    <div class="form-group">
                        <label class="form-label">Initial Stock *</label>
                        <input type="number" name="stock" class="form-input" required value="20">
                    </div>
                    <div class="form-group">
                        <label class="form-label">Image URL</label>
                        <input type="url" name="image_url" class="form-input" placeholder="https://images.unsplash.com/...">
                    </div>
                    <div class="form-group full">
                        <label class="form-label">Description</label>
                        <textarea name="description" class="form-textarea" placeholder="Product details, specs, notes..."></textarea>
                    </div>
                </div>
                <div class="modal-footer">
                    <button type="button" class="btn btn-secondary" onclick="closeCreateModal()">Cancel</button>
                    <button type="submit" class="btn btn-primary">Create Product</button>
                </div>
            </form>
        </div>
    </div>

    <!-- Edit Product Modal -->
    <div class="modal-overlay" id="editModal">
        <div class="modal-content">
            <div class="modal-header">
                <h2 class="modal-title">✏️ Edit Product</h2>
                <button class="close-btn" onclick="closeEditModal()">×</button>
            </div>
            <form id="editForm" onsubmit="handleUpdateProduct(event)">
                <input type="hidden" id="edit_id" name="id">
                <div class="form-grid">
                    <div class="form-group full">
                        <label class="form-label">Product Name *</label>
                        <input type="text" id="edit_name" name="name" class="form-input" required>
                    </div>
                    <div class="form-group">
                        <label class="form-label">Category</label>
                        <select id="edit_category_id" name="category_id" class="form-select">
                            <option value="">Select Category</option>
                            @foreach($categories as $cat)
                                <option value="{{ $cat->id }}">{{ $cat->icon }} {{ $cat->name }}</option>
                            @endforeach
                        </select>
                    </div>
                    <div class="form-group">
                        <label class="form-label">SKU</label>
                        <input type="text" id="edit_sku" name="sku" class="form-input">
                    </div>
                    <div class="form-group">
                        <label class="form-label">Price ($) *</label>
                        <input type="number" step="0.01" id="edit_price" name="price" class="form-input" required>
                    </div>
                    <div class="form-group">
                        <label class="form-label">Cost ($)</label>
                        <input type="number" step="0.01" id="edit_cost" name="cost" class="form-input">
                    </div>
                    <div class="form-group">
                        <label class="form-label">Stock *</label>
                        <input type="number" id="edit_stock" name="stock" class="form-input" required>
                    </div>
                    <div class="form-group">
                        <label class="form-label">Image URL</label>
                        <input type="url" id="edit_image_url" name="image_url" class="form-input">
                    </div>
                    <div class="form-group full">
                        <label class="form-label">Description</label>
                        <textarea id="edit_description" name="description" class="form-textarea"></textarea>
                    </div>
                </div>
                <div class="modal-footer">
                    <button type="button" class="btn btn-secondary" onclick="closeEditModal()">Cancel</button>
                    <button type="submit" class="btn btn-primary">Save Changes</button>
                </div>
            </form>
        </div>
    </div>

    <!-- Category Modal -->
    <div class="modal-overlay" id="categoryModal">
        <div class="modal-content" style="max-width: 460px;">
            <div class="modal-header">
                <h2 class="modal-title">🏷️ New Category</h2>
                <button class="close-btn" onclick="closeCategoryModal()">×</button>
            </div>
            <form id="categoryForm" onsubmit="handleCreateCategory(event)">
                <div class="form-group">
                    <label class="form-label">Category Name *</label>
                    <input type="text" name="name" class="form-input" required placeholder="e.g. Smart Devices">
                </div>
                <div class="form-group">
                    <label class="form-label">Emoji Icon</label>
                    <input type="text" name="icon" class="form-input" value="📦" placeholder="📦">
                </div>
                <div class="form-group">
                    <label class="form-label">Description</label>
                    <textarea name="description" class="form-textarea" placeholder="Category purpose..."></textarea>
                </div>
                <div class="modal-footer">
                    <button type="button" class="btn btn-secondary" onclick="closeCategoryModal()">Cancel</button>
                    <button type="submit" class="btn btn-primary">Save Category</button>
                </div>
            </form>
        </div>
    </div>

    <!-- Toast Notification -->
    <div class="toast" id="toastBox">
        <span id="toastIcon">✨</span>
        <span id="toastMsg">Operation completed successfully!</span>
    </div>

    <!-- Interactive Client JavaScript -->
    <script>
        function showToast(msg, icon = '✨') {
            const t = document.getElementById('toastBox');
            document.getElementById('toastMsg').textContent = msg;
            document.getElementById('toastIcon').textContent = icon;
            t.style.display = 'flex';
            setTimeout(() => { t.style.display = 'none'; }, 3000);
        }

        function openCreateModal() {
            document.getElementById('createModal').classList.add('active');
        }

        function closeCreateModal() {
            document.getElementById('createModal').classList.remove('active');
        }

        function openCategoryModal() {
            document.getElementById('categoryModal').classList.add('active');
        }

        function closeCategoryModal() {
            document.getElementById('categoryModal').classList.remove('active');
        }

        function openEditModal(prod) {
            document.getElementById('edit_id').value = prod.id;
            document.getElementById('edit_name').value = prod.name;
            document.getElementById('edit_category_id').value = prod.category_id || '';
            document.getElementById('edit_sku').value = prod.sku;
            document.getElementById('edit_price').value = prod.price;
            document.getElementById('edit_cost').value = prod.cost || '';
            document.getElementById('edit_stock').value = prod.stock;
            document.getElementById('edit_image_url').value = prod.image_url || '';
            document.getElementById('edit_description').value = prod.description || '';
            document.getElementById('editModal').classList.add('active');
        }

        function closeEditModal() {
            document.getElementById('editModal').classList.remove('active');
        }

        async function handleCreateProduct(e) {
            e.preventDefault();
            const form = e.target;
            const formData = new FormData(form);
            const data = Object.fromEntries(formData.entries());

            try {
                const res = await fetch('/api/products', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json', 'Accept': 'application/json', 'X-CSRF-TOKEN': document.querySelector('meta[name="csrf-token"]')?.getAttribute('content') },
                    body: JSON.stringify(data)
                });
                const result = await res.json();
                if (res.ok) {
                    showToast(`Product '${data.name}' added successfully!`, '✅');
                    closeCreateModal();
                    form.reset();
                    setTimeout(() => window.location.reload(), 600);
                } else {
                    alert(result.message || 'Validation error');
                }
            } catch (err) {
                console.error(err);
            }
        }

        async function handleUpdateProduct(e) {
            e.preventDefault();
            const form = e.target;
            const id = document.getElementById('edit_id').value;
            const formData = new FormData(form);
            const data = Object.fromEntries(formData.entries());

            try {
                const res = await fetch(`/api/products/${id}`, {
                    method: 'PUT',
                    headers: { 'Content-Type': 'application/json', 'Accept': 'application/json', 'X-CSRF-TOKEN': document.querySelector('meta[name="csrf-token"]')?.getAttribute('content') },
                    body: JSON.stringify(data)
                });
                const result = await res.json();
                if (res.ok) {
                    showToast(`Product updated successfully!`, '✅');
                    closeEditModal();
                    setTimeout(() => window.location.reload(), 600);
                } else {
                    alert(result.message || 'Update failed');
                }
            } catch (err) {
                console.error(err);
            }
        }

        async function deleteProduct(id, name) {
            if (!confirm(`Are you sure you want to delete '${name}'?`)) return;

            try {
                const res = await fetch(`/api/products/${id}`, {
                    method: 'DELETE',
                    headers: { 'Accept': 'application/json', 'X-CSRF-TOKEN': document.querySelector('meta[name="csrf-token"]')?.getAttribute('content') }
                });
                if (res.ok) {
                    const row = document.getElementById(`row-${id}`);
                    if (row) row.remove();
                    showToast(`Product '${name}' deleted!`, '🗑️');
                }
            } catch (err) {
                console.error(err);
            }
        }

        async function adjustStock(id, delta) {
            try {
                const res = await fetch(`/api/products/${id}/adjust-stock`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json', 'Accept': 'application/json', 'X-CSRF-TOKEN': document.querySelector('meta[name="csrf-token"]')?.getAttribute('content') },
                    body: JSON.stringify({ delta })
                });
                const result = await res.json();
                if (res.ok && result.product) {
                    document.getElementById(`stock-val-${id}`).textContent = result.product.stock;
                    const badge = document.getElementById(`badge-${id}`);
                    if (result.product.status === 'in_stock') {
                        badge.className = 'badge badge-in-stock';
                        badge.textContent = '● In Stock';
                    } else if (result.product.status === 'low_stock') {
                        badge.className = 'badge badge-low-stock';
                        badge.textContent = '▲ Low Stock';
                    } else {
                        badge.className = 'badge badge-out-of-stock';
                        badge.textContent = '✕ Out of Stock';
                    }
                }
            } catch (err) {
                console.error(err);
            }
        }

        async function handleCreateCategory(e) {
            e.preventDefault();
            const form = e.target;
            const formData = new FormData(form);
            const data = Object.fromEntries(formData.entries());

            try {
                const res = await fetch('/api/categories', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json', 'Accept': 'application/json', 'X-CSRF-TOKEN': document.querySelector('meta[name="csrf-token"]')?.getAttribute('content') },
                    body: JSON.stringify(data)
                });
                if (res.ok) {
                    showToast(`Category '${data.name}' created!`, '🏷️');
                    closeCategoryModal();
                    setTimeout(() => window.location.reload(), 600);
                }
            } catch (err) {
                console.error(err);
            }
        }

        let searchTimeout;
        function handleSearch(e) {
            clearTimeout(searchTimeout);
            searchTimeout = setTimeout(() => {
                applyFilters();
            }, 300);
        }

        function applyFilters() {
            const q = document.getElementById('searchInput').value;
            const category = document.getElementById('categoryFilter').value;
            const status = document.getElementById('statusFilter').value;
            const sort = document.getElementById('sortFilter').value;

            const params = new URLSearchParams();
            if (q) params.set('q', q);
            if (category) params.set('category', category);
            if (status) params.set('status', status);
            if (sort) params.set('sort', sort);

            window.location.search = params.toString();
        }
    </script>
</body>
</html>
