@extends('layouts.store')

@section('title', 'Order #' . $order->order_number . ' - Tracking & Receipt')

@section('styles')
<style>
    .track-wrapper {
        max-width: 960px;
        margin: 2.5rem auto 4rem auto;
        padding: 0 1.5rem;
    }
    .track-card {
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: var(--theme-radius, 16px);
        padding: 2.5rem;
        box-shadow: 0 15px 40px rgba(0,0,0,0.3);
        margin-bottom: 2rem;
    }
    .status-timeline {
        display: flex;
        justify-content: space-between;
        position: relative;
        margin: 3rem 0 2rem 0;
    }
    .status-timeline::before {
        content: '';
        position: absolute;
        top: 20px;
        left: 30px;
        right: 30px;
        height: 4px;
        background: var(--border-color);
        z-index: 1;
    }
    .status-step {
        position: relative;
        z-index: 2;
        text-align: center;
        flex: 1;
    }
    .step-circle {
        width: 44px;
        height: 44px;
        border-radius: 50%;
        background: var(--bg-card);
        border: 2px solid var(--border-color);
        margin: 0 auto 0.75rem auto;
        display: flex;
        align-items: center;
        justify-content: center;
        font-weight: 800;
        font-size: 1rem;
        color: var(--text-muted);
        transition: all 0.3s;
    }
    .status-step.completed .step-circle {
        background: var(--accent-green);
        border-color: var(--accent-green);
        color: white;
        box-shadow: 0 0 15px rgba(16, 185, 129, 0.4);
    }
    .status-step.active .step-circle {
        background: var(--accent);
        border-color: var(--accent);
        color: white;
        box-shadow: 0 0 15px var(--accent-glow);
        animation: pulseGlow 2s infinite;
    }
    @keyframes pulseGlow {
        0%, 100% { transform: scale(1); }
        50% { transform: scale(1.08); }
    }
    .step-label {
        font-size: 0.85rem;
        font-weight: 700;
        color: var(--text-main);
    }
    .step-sub {
        font-size: 0.75rem;
        color: var(--text-muted);
    }
    .receipt-item {
        display: flex;
        justify-content: space-between;
        padding: 0.85rem 0;
        border-bottom: 1px solid var(--border-color);
        font-size: 0.95rem;
    }
    .receipt-item:last-child {
        border-bottom: none;
    }
</style>
@endsection

@section('content')
<div class="track-wrapper">
    <!-- Celebratory Success Banner -->
    <div style="text-align: center; margin-bottom: 2.5rem;">
        <div style="width: 72px; height: 72px; border-radius: 50%; background: linear-gradient(135deg, var(--accent-green), #059669); color: white; display: flex; align-items: center; justify-content: center; font-size: 2.25rem; margin: 0 auto 1.25rem auto; box-shadow: 0 0 30px rgba(16, 185, 129, 0.4);">
            ✓
        </div>
        <h1 style="font-size: 2.25rem; font-weight: 800; letter-spacing: -0.02em; margin-bottom: 0.5rem;">Order Confirmed & Placed!</h1>
        <p style="color: var(--text-muted); font-size: 1.05rem;">
            Order Reference: <strong style="color: var(--accent);">#{{ $order->order_number }}</strong>
        </p>
    </div>

    <!-- Live Interactive Tracking Timeline -->
    <div class="track-card">
        <h3 style="font-size: 1.25rem; font-weight: 700; margin-bottom: 1rem;">📦 Real-Time Delivery Tracking</h3>

        <div class="status-timeline">
            <div class="status-step completed">
                <div class="step-circle">✓</div>
                <div class="step-label">Order Placed</div>
                <div class="step-sub">{{ $order->created_at->format('M j, g:i A') }}</div>
            </div>
            <div class="status-step completed">
                <div class="step-circle">✓</div>
                <div class="step-label">Payment Confirmed</div>
                <div class="step-sub">Encrypted & Verified</div>
            </div>
            <div class="status-step active">
                <div class="step-circle">⚙️</div>
                <div class="step-label">Cleanroom Prep</div>
                <div class="step-sub">Hardware QA & Flashing</div>
            </div>
            <div class="status-step">
                <div class="step-circle">✈️</div>
                <div class="step-label">In Transit</div>
                <div class="step-sub">Courier Handover</div>
            </div>
            <div class="status-step">
                <div class="step-circle">🏠</div>
                <div class="step-label">Delivered</div>
                <div class="step-sub">Signature Required</div>
            </div>
        </div>
    </div>

    <!-- Receipt & Customer Details -->
    <div style="display: grid; grid-template-columns: 1.5fr 1fr; gap: 2rem;">
        <div class="track-card" style="margin-bottom: 0;">
            <h3 style="font-size: 1.15rem; font-weight: 700; margin-bottom: 1rem; border-bottom: 1px solid var(--border-color); padding-bottom: 0.75rem;">
                Items Purchased
            </h3>

            <div>
                @foreach($order->items as $item)
                    <div class="receipt-item">
                        <div>
                            <div style="font-weight: 600;">{{ $item->product->name ?? 'Hardware Device' }}</div>
                            <div style="font-size: 0.8rem; color: var(--text-muted);">${{ number_format($item->unit_price, 2) }} &times; {{ $item->quantity }} units</div>
                        </div>
                        <div style="font-weight: 700;">
                            ${{ number_format($item->subtotal, 2) }}
                        </div>
                    </div>
                @endforeach
            </div>

            <div style="margin-top: 1.5rem; border-top: 1px solid var(--border-color); padding-top: 1rem;">
                <div style="display: flex; justify-content: space-between; font-size: 1.25rem; font-weight: 800; color: var(--text-main);">
                    <span>Total Amount Paid</span>
                    <span style="color: var(--accent);">${{ number_format($order->total_amount, 2) }}</span>
                </div>
            </div>
        </div>

        <div class="track-card" style="margin-bottom: 0;">
            <h3 style="font-size: 1.15rem; font-weight: 700; margin-bottom: 1rem; border-bottom: 1px solid var(--border-color); padding-bottom: 0.75rem;">
                Delivery Details
            </h3>

            <div style="font-size: 0.9rem; line-height: 1.6; color: var(--text-muted);">
                <div style="font-weight: 700; color: var(--text-main); font-size: 1rem; margin-bottom: 0.5rem;">
                    {{ $order->customer->name }}
                </div>
                <div>📧 {{ $order->customer->email }}</div>
                <div>📞 {{ $order->customer->phone }}</div>
                <div style="margin-top: 0.5rem;">📍 {{ $order->customer->address }}</div>
                <div>{{ $order->customer->city }}, {{ $order->customer->state }} {{ $order->customer->postal_code }}</div>
                <div>{{ $order->customer->country }}</div>
            </div>

            <div style="margin-top: 2rem; display: flex; flex-direction: column; gap: 0.75rem;">
                <button type="button" onclick="window.print()" class="btn btn-outline btn-block">
                    🖨️ Print Receipt
                </button>
                <a href="{{ route('store.index') }}" class="btn btn-primary btn-block">
                    Explore More Gear →
                </a>
            </div>
        </div>
    </div>
</div>
@endsection
