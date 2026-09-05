@extends('layouts.store')

@section('title', ($siteSettings['store_name'] ?? 'Hyperion Pro Store') . ' - Hardware Catalog & Gear')

@section('styles')
<style>
    .store-content-wrapper {
        max-width: 1280px;
        margin: 0 auto;
        padding: 0 1.5rem 3rem 1.5rem;
    }
    .metrics-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
        gap: 1.25rem;
        margin-bottom: 2.5rem;
    }
    .metric-card {
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: var(--theme-radius, 12px);
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
        border-radius: 9999px;
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
        border-radius: var(--theme-radius, 8px);
        color: var(--text-main);
        font-size: 0.9rem;
        outline: none;
        transition: border-color 0.2s;
    }
    .search-input:focus {
        border-color: var(--accent);
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
        border-radius: var(--theme-radius, 14px);
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
        color: var(--accent);
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
        color: var(--accent-green);
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
        color: var(--accent);
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
        gap: 0.5rem;
    }
    .price {
        font-size: 1.35rem;
        font-weight: 800;
        color: var(--text-main);
    }
    .btn-add-cart {
        background: var(--accent);
        color: white;
        border: none;
        padding: 0.5rem 0.9rem;
        border-radius: var(--theme-radius, 8px);
        font-size: 0.85rem;
        font-weight: 700;
        cursor: pointer;
        display: inline-flex;
        align-items: center;
        gap: 0.35rem;
        transition: transform 0.15s, opacity 0.15s;
    }
    .btn-add-cart:hover {
        opacity: 0.9;
        transform: translateY(-1px);
    }
    .pagination-wrapper {
        margin-top: 2rem;
        display: flex;
        justify-content: center;
    }
</style>
@endsection

@section('content')
<!-- Dynamic No-Code Page Builder Sections (Hero, Features, Flash Sale, etc.) -->
@if(isset($sections) && $sections->isNotEmpty())
    @foreach($sections as $section)
        @if($section->section_type === 'featured_products')
            <!-- Inlined Catalog Grid Anchor Point -->
            <div id="products" class="store-content-wrapper">
                <div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 1.5rem; border-bottom: 1px solid var(--border-color); padding-bottom: 1rem;">
                    <div>
                        <h2 style="font-size: 1.75rem; font-weight: 800; letter-spacing: -0.02em; color: var(--text-main);">
                            {{ $section->title ?? 'Curated Hardware & Peripherals' }}
                        </h2>
                        <p style="color: var(--text-muted); font-size: 0.95rem;">
                            {{ $section->subtitle ?? 'Handpicked by system architects for maximum throughput.' }}
                        </p>
                    </div>
                    <span style="font-size: 0.85rem; color: var(--accent-green); font-weight: 700;">
                        ● Instant Dispatch Available
                    </span>
                </div>

                <!-- Controls Row: Category Filtering and Search -->
                <div class="controls-row">
                    <div class="category-pills">
                        <a href="{{ route('store.index') }}#products" class="pill {{ !request('category') ? 'active' : '' }}">
                            All Hardware
                        </a>
                        @foreach($categories as $cat)
                            <a href="{{ route('store.index', ['category' => $cat->slug]) }}#products" class="pill {{ request('category') === $cat->slug ? 'active' : '' }}">
                                {{ $cat->name }}
                                <span style="font-size: 0.75rem; opacity: 0.7;">({{ $cat->products_count }})</span>
                            </a>
                        @endforeach
                    </div>

                    <form action="{{ route('store.index') }}#products" method="GET" class="search-form">
                        @if(request('category'))
                            <input type="hidden" name="category" value="{{ request('category') }}">
                        @endif
                        <input type="text" name="search" class="search-input" placeholder="Search devices, chips..." value="{{ request('search') }}">
                        <button type="submit" class="btn btn-primary">Search</button>
                    </form>
                </div>

                <!-- Products Grid -->
                <div class="products-grid">
                    @forelse($products as $product)
                        <div class="product-card">
                            <div>
                                <div class="card-top">
                                    <span class="cat-badge">{{ $product->category->name ?? 'Hardware' }}</span>
                                    <span class="stock-badge stock-in">In Stock ({{ $product->stock }})</span>
                                </div>
                                <h3 class="product-name">
                                    <a href="{{ route('store.show', $product->id) }}">{{ $product->name }}</a>
                                </h3>
                                <p class="product-desc">{{ $product->description }}</p>
                            </div>
                            <div class="card-footer">
                                <div class="price">${{ number_format($product->price, 2) }}</div>
                                <button type="button" class="btn-add-cart" onclick="quickAddToCart({{ $product->id }})">
                                    🛒 Add to Cart
                                </button>
                            </div>
                        </div>
                    @empty
                        <div style="grid-column: 1 / -1; text-align: center; padding: 3rem 0; color: var(--text-muted);">
                            <h3>No hardware products found.</h3>
                            <p>Try clearing your category or search filter.</p>
                        </div>
                    @endforelse
                </div>

                <!-- Pagination -->
                @if($products->hasPages())
                    <div class="pagination-wrapper">
                        {{ $products->links() }}
                    </div>
                @endif
            </div>
        @elseif(view()->exists('_partials.sections.' . $section->section_type))
            @include('_partials.sections.' . $section->section_type, ['section' => $section])
        @endif
    @endforeach
@else
    <!-- Fallback Standard Catalog View if no sections exist -->
    <div id="products" class="store-content-wrapper" style="padding-top: 2rem;">
        <div class="products-grid">
            @foreach($products as $product)
                <div class="product-card">
                    <div>
                        <h3 class="product-name"><a href="{{ route('store.show', $product->id) }}">{{ $product->name }}</a></h3>
                        <p class="product-desc">{{ $product->description }}</p>
                    </div>
                    <div class="card-footer">
                        <div class="price">${{ number_format($product->price, 2) }}</div>
                        <button type="button" class="btn-add-cart" onclick="quickAddToCart({{ $product->id }})">🛒 Add to Cart</button>
                    </div>
                </div>
            @endforeach
        </div>
    </div>
@endif
@endsection
