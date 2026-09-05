@extends('layouts.store')

@section('title', 'Shopping Cart - ' . ($siteSettings['store_name'] ?? 'Hyperion Pro Store'))

@section('styles')
<style>
    .cart-page-wrapper {
        max-width: 1280px;
        margin: 2rem auto 4rem auto;
        padding: 0 1.5rem;
    }
    .cart-grid {
        display: grid;
        grid-template-columns: 2fr 1fr;
        gap: 2rem;
        align-items: start;
    }
    @media (max-width: 900px) {
        .cart-grid {
            grid-template-columns: 1fr;
        }
    }
    .cart-table-card {
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: var(--theme-radius, 14px);
        padding: 1.5rem;
    }
    .cart-item-row {
        display: flex;
        gap: 1.25rem;
        align-items: center;
        padding: 1.25rem 0;
        border-bottom: 1px solid var(--border-color);
    }
    .cart-item-row:last-child {
        border-bottom: none;
    }
    .item-icon {
        width: 60px;
        height: 60px;
        border-radius: var(--theme-radius, 10px);
        background: rgba(255, 255, 255, 0.05);
        border: 1px solid var(--border-color);
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 1.75rem;
    }
    .item-info {
        flex: 1;
    }
    .item-info h4 {
        font-size: 1.05rem;
        font-weight: 700;
        margin-bottom: 0.25rem;
    }
    .item-info .item-cat {
        font-size: 0.8rem;
        color: var(--text-muted);
    }
    .item-unit-price {
        font-weight: 700;
        color: var(--accent);
        font-size: 1.1rem;
        min-width: 90px;
        text-align: right;
    }
    .item-qty-box {
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }
    .summary-card {
        background: var(--bg-card);
        border: 1px solid var(--border-highlight);
        border-radius: var(--theme-radius, 14px);
        padding: 1.75rem;
        box-shadow: 0 10px 30px rgba(0,0,0,0.25);
    }
    .summary-card h3 {
        font-size: 1.25rem;
        font-weight: 800;
        margin-bottom: 1.25rem;
        border-bottom: 1px solid var(--border-color);
        padding-bottom: 0.75rem;
    }
    .fee-row {
        display: flex;
        justify-content: space-between;
        margin-bottom: 0.75rem;
        font-size: 0.95rem;
        color: var(--text-muted);
    }
    .fee-row.total {
        font-size: 1.25rem;
        font-weight: 800;
        color: var(--text-main);
        border-top: 1px solid var(--border-color);
        padding-top: 1rem;
        margin-top: 1rem;
    }
</style>
@endsection

@section('content')
<div class="cart-page-wrapper">
    <div style="margin-bottom: 2rem;">
        <h1 style="font-size: 2.25rem; font-weight: 800; letter-spacing: -0.02em;">Shopping Cart</h1>
        <p style="color: var(--text-muted);">Review your hardware devices and enterprise licenses before checkout.</p>
    </div>

    @if(session('success'))
        <div style="background: rgba(16, 185, 129, 0.15); border: 1px solid var(--accent-green); color: var(--accent-green); padding: 1rem 1.5rem; border-radius: var(--theme-radius, 8px); margin-bottom: 1.5rem; font-weight: 600;">
            ✓ {{ session('success') }}
        </div>
    @endif
    @if(session('error'))
        <div style="background: rgba(239, 68, 68, 0.15); border: 1px solid #ef4444; color: #ef4444; padding: 1rem 1.5rem; border-radius: var(--theme-radius, 8px); margin-bottom: 1.5rem; font-weight: 600;">
            ⚠ {{ session('error') }}
        </div>
    @endif

    @if($summary['is_empty'])
        <div class="cart-table-card" style="text-align: center; padding: 5rem 2rem;">
            <div style="font-size: 4rem; margin-bottom: 1rem;">🛒</div>
            <h2 style="font-size: 1.5rem; font-weight: 700; margin-bottom: 0.5rem;">Your Cart is Completely Empty</h2>
            <p style="color: var(--text-muted); margin-bottom: 2rem;">You haven't added any products to your cart yet.</p>
            <a href="{{ route('store.index') }}" class="btn btn-primary" style="padding: 0.75rem 2rem;">Explore Hardware Store →</a>
        </div>
    @else
        <div class="cart-grid">
            <!-- Left Items List -->
            <div class="cart-table-card">
                <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; border-bottom: 1px solid var(--border-color); padding-bottom: 0.75rem;">
                    <span style="font-weight: 700; font-size: 1rem;">Products ({{ $summary['item_count'] }})</span>
                    <form action="{{ route('cart.clear') }}" method="POST" onsubmit="return confirm('Clear entire cart?');">
                        @csrf
                        <button type="submit" style="background: none; border: none; color: #ef4444; cursor: pointer; font-size: 0.85rem; font-weight: 600;">Clear All Items</button>
                    </form>
                </div>

                @foreach($summary['items'] as $item)
                    <div class="cart-item-row">
                        <div class="item-icon">⚡</div>
                        <div class="item-info">
                            <h4><a href="{{ route('store.show', $item['id']) }}" style="color: inherit; text-decoration: none;">{{ $item['name'] }}</a></h4>
                            <span class="item-cat">{{ $item['category_name'] }} &bull; SKU: {{ $item['sku'] }}</span>
                        </div>
                        <div class="item-qty-box">
                            <form action="{{ route('cart.update') }}" method="POST" style="display: inline;">
                                @csrf
                                <input type="hidden" name="product_id" value="{{ $item['id'] }}">
                                <input type="hidden" name="quantity" value="{{ $item['quantity'] - 1 }}">
                                <button type="submit" class="qty-btn">-</button>
                            </form>
                            <span class="qty-val">{{ $item['quantity'] }}</span>
                            <form action="{{ route('cart.update') }}" method="POST" style="display: inline;">
                                @csrf
                                <input type="hidden" name="product_id" value="{{ $item['id'] }}">
                                <input type="hidden" name="quantity" value="{{ $item['quantity'] + 1 }}">
                                <button type="submit" class="qty-btn">+</button>
                            </form>
                        </div>
                        <div class="item-unit-price">${{ number_format($item['line_total'], 2) }}</div>
                        <form action="{{ route('cart.remove', $item['id']) }}" method="POST">
                            @csrf
                            <button type="submit" class="btn-remove-item" title="Remove Item">&times;</button>
                        </form>
                    </div>
                @endforeach
            </div>

            <!-- Right Financial Summary -->
            <div class="summary-card">
                <h3>Order Summary</h3>
                
                <form action="{{ route('cart.coupon') }}" method="POST" style="margin-bottom: 1.5rem;">
                    @csrf
                    <div style="display: flex; gap: 0.5rem;">
                        <input type="text" name="coupon_code" placeholder="Coupon (e.g. HYPERION20)" required style="flex: 1; background: rgba(0,0,0,0.3); border: 1px solid var(--border-color); border-radius: var(--theme-radius, 6px); padding: 0.5rem 0.75rem; color: var(--text-main); font-size: 0.9rem;">
                        <button type="submit" class="btn btn-outline" style="padding: 0.5rem 1rem;">Apply</button>
                    </div>
                </form>

                <div class="fee-row">
                    <span>Subtotal</span>
                    <span>${{ number_format($summary['subtotal'], 2) }}</span>
                </div>

                @if($summary['discount_amount'] > 0)
                    <div class="fee-row" style="color: var(--accent-green); font-weight: 600;">
                        <span>Discount ({{ $summary['coupon']['code'] }})</span>
                        <span>-${{ number_format($summary['discount_amount'], 2) }}</span>
                    </div>
                @endif

                <div class="fee-row">
                    <span>Estimated Sales Tax (8%)</span>
                    <span>${{ number_format($summary['tax'], 2) }}</span>
                </div>

                <div class="fee-row">
                    <span>Standard Shipping</span>
                    <span>{{ $summary['shipping'] === 0.0 ? 'FREE' : '$' . number_format($summary['shipping'], 2) }}</span>
                </div>

                <div class="fee-row total">
                    <span>Total</span>
                    <span>${{ number_format($summary['total'], 2) }}</span>
                </div>

                <div style="margin-top: 1.5rem;">
                    <a href="{{ route('checkout.index') }}" class="btn btn-primary btn-block" style="padding: 0.9rem 1.5rem; font-size: 1.05rem; font-weight: 700;">
                        Proceed to Checkout →
                    </a>
                    <a href="{{ route('store.index') }}" class="btn btn-subtle btn-block" style="margin-top: 0.75rem;">
                        ← Continue Shopping
                    </a>
                </div>
            </div>
        </div>
    @endif
</div>
@endsection
