@extends('layouts.store')

@section('title', 'Hyperion Enterprise Store - Product Catalog')

@section('styles')
<style>
    .metrics-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
        gap: 1.25rem;
        margin-bottom: 2.5rem;
    }
    .metric-card {
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: 12px;
        padding: 1.25rem;
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        transition: transform 0.2s, border-color 0.2s;
    }
    .metric-card:hover {
        transform: translateY(-2px);
        border-color: var(--border-highlight);
    }
    .metric-title {
        font-size: 0.8rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: var(--text-muted);
    }
    .metric-val {
        font-size: 1.75rem;
        font-weight: 800;
        color: var(--text-main);
    }
    .metric-val.green {
        color: var(--accent-green);
    }
    .controls-row {
        display: flex;
        flex-wrap: wrap;
        gap: 1rem;
        align-items: center;
        justify-content: space-between;
        margin-bottom: 2rem;
    }
    .category-pills {
        display: flex;
        flex-wrap: wrap;
        gap: 0.5rem;
    }
    .pill {
        display: inline-flex;
        align-items: center;
        gap: 0.4rem;
        padding: 0.45rem 0.9rem;
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: 20px;
        color: var(--text-muted);
        text-decoration: none;
        font-size: 0.85rem;
        font-weight: 500;
        transition: all 0.2s;
    }
    .pill:hover, .pill.active {
        background: var(--accent);
        border-color: var(--accent);
        color: white;
    }
    .search-form {
        display: flex;
        gap: 0.5rem;
    }
    .search-input {
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        padding: 0.5rem 1rem;
        border-radius: 8px;
        color: var(--text-main);
        font-size: 0.9rem;
        outline: none;
        transition: border-color 0.2s;
    }
    .search-input:focus {
        border-color: var(--accent);
    }
    .btn {
        background: var(--accent);
        color: white;
        border: none;
        padding: 0.5rem 1rem;
        border-radius: 8px;
        cursor: pointer;
        font-weight: 600;
        font-size: 0.9rem;
        transition: opacity 0.2s;
        text-decoration: none;
        display: inline-flex;
        align-items: center;
        justify-content: center;
    }
    .btn:hover {
        opacity: 0.9;
    }
    .products-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
        gap: 1.5rem;
        margin-bottom: 2.5rem;
    }
    .product-card {
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: 14px;
        padding: 1.5rem;
        display: flex;
        flex-direction: column;
        justify-content: space-between;
        transition: transform 0.2s, border-color 0.2s, box-shadow 0.2s;
    }
    .product-card:hover {
        transform: translateY(-4px);
        border-color: var(--border-highlight);
        box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.5);
    }
    .card-top {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        margin-bottom: 0.75rem;
    }
    .cat-badge {
        font-size: 0.75rem;
        font-weight: 600;
        color: #818cf8;
        background: rgba(99, 102, 241, 0.12);
        padding: 0.2rem 0.6rem;
        border-radius: 6px;
    }
    .stock-badge {
        font-size: 0.75rem;
        font-weight: 600;
        padding: 0.2rem 0.6rem;
        border-radius: 6px;
    }
    .stock-in {
        color: #34d399;
        background: rgba(16, 185, 129, 0.12);
    }
    .product-name {
        font-size: 1.15rem;
        font-weight: 700;
        margin-bottom: 0.5rem;
        line-height: 1.4;
    }
    .product-name a {
        color: inherit;
        text-decoration: none;
        transition: color 0.2s;
    }
    .product-name a:hover {
        color: #818cf8;
    }
    .product-desc {
        font-size: 0.85rem;
        color: var(--text-muted);
        margin-bottom: 1.25rem;
        display: -webkit-box;
        -webkit-line-clamp: 2;
        -webkit-box-orient: vertical;
        overflow: hidden;
    }
    .card-footer {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding-top: 1rem;
        border-top: 1px solid var(--border-color);
    }
    .price {
        font-size: 1.35rem;
        font-weight: 800;
        color: var(--text-main);
    }
    .rating {
        font-size: 0.85rem;
        font-weight: 600;
        color: #fbbf24;
    }
</style>
@endsection

@section('content')
<!-- Analytics Dashboard Banner -->
<section class="metrics-grid">
    <div class="metric-card">
        <span class="metric-title">Gross Revenue</span>
        <span class="metric-val green">${{ number_format($analytics['metrics']['total_revenue'], 2) }}</span>
    </div>
    <div class="metric-card">
        <span class="metric-title">Completed Orders</span>
        <span class="metric-val">{{ $analytics['metrics']['total_orders'] }}</span>
    </div>
    <div class="metric-card">
        <span class="metric-title">Active Products</span>
        <span class="metric-val">{{ $analytics['metrics']['total_products'] }}</span>
    </div>
    <div class="metric-card">
        <span class="metric-title">Average Order Value</span>
        <span class="metric-val">${{ number_format($analytics['metrics']['average_order_value'], 2) }}</span>
    </div>
</section>

<!-- Filter Controls -->
<div class="controls-row">
    <div class="category-pills">
        <a href="{{ route('store.index') }}" class="pill {{ !request('category') ? 'active' : '' }}">All Products</a>
        @foreach($categories as $cat)
            <a href="{{ route('store.index', ['category' => $cat->slug]) }}" class="pill {{ request('category') === $cat->slug ? 'active' : '' }}">
                {{ $cat->name }} ({{ $cat->products_count }})
            </a>
        @endforeach
    </div>

    <form method="GET" action="{{ route('store.index') }}" class="search-form">
        <input type="text" name="search" class="search-input" placeholder="Search catalog..." value="{{ request('search') }}">
        <button type="submit" class="btn">Filter</button>
    </form>
</div>

<!-- Products Grid -->
<div class="products-grid">
    @forelse($products as $prod)
        <div class="product-card">
            <div>
                <div class="card-top">
                    <span class="cat-badge">{{ $prod->category->name }}</span>
                    <span class="stock-badge stock-in">{{ $prod->stock }} in stock</span>
                </div>
                <h3 class="product-name">
                    <a href="{{ route('store.show', $prod->id) }}">{{ $prod->name }}</a>
                </h3>
                <p class="product-desc">{{ $prod->description }}</p>
            </div>
            <div class="card-footer">
                <div>
                    <div class="price">${{ number_format($prod->price, 2) }}</div>
                    <div class="rating">★ {{ $prod->average_rating }} ({{ $prod->reviews_count }})</div>
                </div>
                <a href="{{ route('store.show', $prod->id) }}" class="btn">View</a>
            </div>
        </div>
    @empty
        <div style="grid-column: 1 / -1; text-align: center; padding: 4rem 1rem; color: var(--text-muted);">
            <h3>No products found matching the criteria.</h3>
        </div>
    @endforelse
</div>
@endsection
