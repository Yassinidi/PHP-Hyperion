@extends('layouts.store')

@section('title', 'Hyperion Enterprise Store - Orders & Audit Trails')

@section('styles')
<style>
    .orders-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 2rem;
    }
    .orders-title {
        font-size: 1.8rem;
        font-weight: 800;
        letter-spacing: -0.02em;
    }
    .orders-subtitle {
        color: var(--text-muted);
        font-size: 0.95rem;
        margin-top: 0.25rem;
    }
    .orders-list {
        display: flex;
        flex-direction: column;
        gap: 1.5rem;
    }
    .order-card {
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: 14px;
        padding: 1.5rem;
        transition: border-color 0.2s, transform 0.2s;
    }
    .order-card:hover {
        border-color: var(--border-highlight);
        transform: translateY(-2px);
    }
    .order-top {
        display: flex;
        flex-wrap: wrap;
        justify-content: space-between;
        align-items: center;
        gap: 1rem;
        padding-bottom: 1.25rem;
        border-bottom: 1px solid var(--border-color);
    }
    .order-meta-left {
        display: flex;
        align-items: center;
        gap: 1rem;
    }
    .order-num {
        font-family: monospace;
        font-weight: 700;
        font-size: 1.15rem;
        color: var(--text-main);
        letter-spacing: 0.05em;
    }
    .badge {
        font-size: 0.75rem;
        font-weight: 700;
        padding: 0.25rem 0.65rem;
        border-radius: 9999px;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        display: inline-flex;
        align-items: center;
        gap: 0.35rem;
    }
    .badge-paid, .badge-completed {
        background: rgba(16, 185, 129, 0.15);
        color: #34d399;
        border: 1px solid rgba(16, 185, 129, 0.3);
    }
    .badge-pending {
        background: rgba(245, 158, 11, 0.15);
        color: #fbbf24;
        border: 1px solid rgba(245, 158, 11, 0.3);
    }
    .badge-cancelled {
        background: rgba(239, 68, 68, 0.15);
        color: #f87171;
        border: 1px solid rgba(239, 68, 68, 0.3);
    }
    .badge-tier-vip {
        background: linear-gradient(135deg, rgba(236, 72, 153, 0.2), rgba(168, 85, 247, 0.2));
        color: #f472b6;
        border: 1px solid rgba(236, 72, 153, 0.4);
    }
    .badge-tier-gold {
        background: rgba(234, 179, 8, 0.15);
        color: #fde047;
        border: 1px solid rgba(234, 179, 8, 0.4);
    }
    .badge-tier-standard {
        background: rgba(148, 163, 184, 0.15);
        color: #cbd5e1;
        border: 1px solid rgba(148, 163, 184, 0.3);
    }
    .order-total-block {
        text-align: right;
    }
    .order-total-label {
        font-size: 0.75rem;
        color: var(--text-muted);
        text-transform: uppercase;
    }
    .order-total-val {
        font-size: 1.4rem;
        font-weight: 800;
        color: var(--accent-green);
    }
    .order-body {
        padding: 1.25rem 0;
        display: grid;
        grid-template-columns: 2fr 1fr;
        gap: 1.5rem;
    }
    @media (max-width: 768px) {
        .order-body {
            grid-template-columns: 1fr;
        }
    }
    .items-table {
        width: 100%;
        border-collapse: collapse;
        font-size: 0.9rem;
    }
    .items-table th {
        text-align: left;
        color: var(--text-muted);
        font-weight: 600;
        padding-bottom: 0.5rem;
        font-size: 0.8rem;
        text-transform: uppercase;
    }
    .items-table td {
        padding: 0.4rem 0;
        border-top: 1px solid rgba(255, 255, 255, 0.04);
    }
    .customer-info {
        background: rgba(255, 255, 255, 0.02);
        border-radius: 8px;
        padding: 1rem;
        border: 1px solid var(--border-color);
        font-size: 0.85rem;
    }
    .customer-info-title {
        font-weight: 700;
        font-size: 0.8rem;
        text-transform: uppercase;
        color: var(--text-muted);
        margin-bottom: 0.5rem;
    }
    .activities-section {
        margin-top: 1rem;
        padding-top: 1rem;
        border-top: 1px solid var(--border-color);
    }
    .activities-title {
        font-size: 0.8rem;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: var(--accent);
        margin-bottom: 0.75rem;
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }
    .activity-item {
        font-size: 0.825rem;
        color: var(--text-muted);
        display: flex;
        align-items: center;
        gap: 0.5rem;
        margin-bottom: 0.35rem;
    }
    .activity-dot {
        width: 6px;
        height: 6px;
        border-radius: 50%;
        background: var(--accent);
    }
</style>
@endsection

@section('content')
<div class="orders-header">
    <div>
        <h1 class="orders-title">Orders & Polymorphic Audit Trail</h1>
        <p class="orders-subtitle">Live orders with PHP 8.4 Backed Enums, Domain Events, and Polymorphic Activity Logs</p>
    </div>
    <div>
        <a href="{{ route('store.index') }}" style="color: var(--accent); text-decoration: none; font-weight: 600; font-size: 0.9rem;">&larr; Back to Catalog</a>
    </div>
</div>

<div class="orders-list">
    @forelse($orders as $order)
        <div class="order-card">
            <div class="order-top">
                <div class="order-meta-left">
                    <span class="order-num">{{ $order->order_number }}</span>
                    <span class="badge badge-{{ $order->status->value }}">
                        ● {{ $order->status->label() }}
                    </span>
                    @if($order->customer)
                        <span class="badge badge-tier-{{ $order->customer->tier->value }}">
                            Tier: {{ ucfirst($order->customer->tier->value) }}
                        </span>
                    @endif
                </div>
                <div class="order-total-block">
                    <div class="order-total-label">Total Amount (Inc. Tier Discount)</div>
                    <div class="order-total-val">${{ number_format($order->total_amount, 2) }}</div>
                </div>
            </div>

            <div class="order-body">
                <div>
                    <table class="items-table">
                        <thead>
                            <tr>
                                <th>Item</th>
                                <th>Qty</th>
                                <th>Unit Price</th>
                                <th>Subtotal</th>
                            </tr>
                        </thead>
                        <tbody>
                            @foreach($order->items as $item)
                                <tr>
                                    <td>{{ $item->product ? $item->product->name : 'Product #' . $item->product_id }}</td>
                                    <td>{{ $item->quantity }}</td>
                                    <td>${{ number_format($item->unit_price, 2) }}</td>
                                    <td>${{ number_format($item->subtotal, 2) }}</td>
                                </tr>
                            @endforeach
                        </tbody>
                    </table>
                </div>

                <div class="customer-info">
                    <div class="customer-info-title">Customer & Payment</div>
                    @if($order->customer)
                        <div><strong>{{ $order->customer->name }}</strong></div>
                        <div style="color: var(--text-muted); margin-bottom: 0.5rem;">{{ $order->customer->email }}</div>
                    @endif
                    <div>Payment: <span style="text-transform: capitalize; color: var(--accent);">{{ $order->payment_method }}</span></div>
                    <div style="color: var(--text-muted); margin-top: 0.25rem;">Date: {{ $order->created_at->format('M d, Y H:i') }}</div>
                </div>
            </div>

            @if($order->activities->count() > 0)
                <div class="activities-section">
                    <div class="activities-title">
                        <span>⚡</span> Domain Event & Polymorphic Audit Trail
                    </div>
                    @foreach($order->activities as $activity)
                        <div class="activity-item">
                            <span class="activity-dot"></span>
                            <span>{{ $activity->description }}</span>
                            <span style="opacity: 0.6; font-size: 0.75rem;">({{ $activity->created_at->diffForHumans() }})</span>
                        </div>
                    @endforeach
                </div>
            @endif
        </div>
    @empty
        <div style="text-align: center; padding: 4rem; color: var(--text-muted);">
            No orders found.
        </div>
    @endforelse
</div>

<div style="margin-top: 2rem;">
    {{ $orders->links() }}
</div>
@endsection
