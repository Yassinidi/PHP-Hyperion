@extends('layouts.store')

@section('title', 'Secure Checkout - ' . ($siteSettings['store_name'] ?? 'Hyperion Pro Store'))

@section('styles')
<style>
    .checkout-wrapper {
        max-width: 1200px;
        margin: 2rem auto 4rem auto;
        padding: 0 1.5rem;
    }
    .checkout-grid {
        display: grid;
        grid-template-columns: 1.6fr 1fr;
        gap: 2.5rem;
        align-items: start;
    }
    @media (max-width: 900px) {
        .checkout-grid {
            grid-template-columns: 1fr;
        }
    }
    .form-card {
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: var(--theme-radius, 14px);
        padding: 2rem;
        margin-bottom: 2rem;
    }
    .form-card h3 {
        font-size: 1.25rem;
        font-weight: 700;
        margin-bottom: 1.5rem;
        display: flex;
        align-items: center;
        gap: 0.5rem;
        color: var(--text-main);
    }
    .form-row {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 1rem;
        margin-bottom: 1rem;
    }
    @media (max-width: 600px) {
        .form-row {
            grid-template-columns: 1fr;
        }
    }
    .form-group {
        display: flex;
        flex-direction: column;
        gap: 0.4rem;
        margin-bottom: 1rem;
    }
    .form-group label {
        font-size: 0.85rem;
        font-weight: 600;
        color: var(--text-muted);
    }
    .form-input {
        background: rgba(0, 0, 0, 0.25);
        border: 1px solid var(--border-color);
        border-radius: var(--theme-radius, 8px);
        padding: 0.75rem 1rem;
        color: var(--text-main);
        font-size: 0.95rem;
        outline: none;
        transition: border-color 0.2s;
    }
    .form-input:focus {
        border-color: var(--accent);
    }
    .payment-option-box {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
    }
    .payment-label {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        background: rgba(255, 255, 255, 0.03);
        border: 1px solid var(--border-color);
        padding: 1rem;
        border-radius: var(--theme-radius, 10px);
        cursor: pointer;
        transition: border-color 0.2s, background 0.2s;
    }
    .payment-label:hover {
        border-color: var(--border-highlight);
    }
    .checkout-summary-card {
        background: var(--bg-card);
        border: 1px solid var(--border-highlight);
        border-radius: var(--theme-radius, 14px);
        padding: 2rem;
        position: sticky;
        top: 6rem;
        box-shadow: 0 10px 40px rgba(0, 0, 0, 0.3);
    }
</style>
@endsection

@section('content')
<div class="checkout-wrapper">
    <div style="margin-bottom: 2rem;">
        <h1 style="font-size: 2.25rem; font-weight: 800; letter-spacing: -0.02em;">Secure Checkout</h1>
        <p style="color: var(--text-muted);">Complete your order with encrypted transaction security.</p>
    </div>

    @if(session('error'))
        <div style="background: rgba(239, 68, 68, 0.15); border: 1px solid #ef4444; color: #ef4444; padding: 1rem 1.5rem; border-radius: var(--theme-radius, 8px); margin-bottom: 1.5rem; font-weight: 600;">
            ⚠ {{ session('error') }}
        </div>
    @endif

    <form action="{{ route('checkout.process') }}" method="POST">
        @csrf
        <div class="checkout-grid">
            <!-- Left: Checkout Forms -->
            <div>
                <!-- Contact & Shipping -->
                <div class="form-card">
                    <h3>📍 1. Shipping Information</h3>
                    <div class="form-row">
                        <div class="form-group">
                            <label>First Name</label>
                            <input type="text" name="first_name" required value="{{ old('first_name', 'Alex') }}" class="form-input">
                        </div>
                        <div class="form-group">
                            <label>Last Name</label>
                            <input type="text" name="last_name" required value="{{ old('last_name', 'Rivers') }}" class="form-input">
                        </div>
                    </div>

                    <div class="form-row">
                        <div class="form-group">
                            <label>Email Address</label>
                            <input type="email" name="email" required value="{{ old('email', 'alex.rivers@hyperion.io') }}" class="form-input">
                        </div>
                        <div class="form-group">
                            <label>Phone Number</label>
                            <input type="text" name="phone" required value="{{ old('phone', '+1 (555) 382-9901') }}" class="form-input">
                        </div>
                    </div>

                    <div class="form-group">
                        <label>Delivery Address</label>
                        <input type="text" name="address" required value="{{ old('address', '500 Innovation Blvd, Suite 400') }}" class="form-input">
                    </div>

                    <div class="form-row">
                        <div class="form-group">
                            <label>City</label>
                            <input type="text" name="city" required value="{{ old('city', 'San Francisco') }}" class="form-input">
                        </div>
                        <div class="form-group">
                            <label>State / Region</label>
                            <input type="text" name="state" required value="{{ old('state', 'CA') }}" class="form-input">
                        </div>
                    </div>

                    <div class="form-row">
                        <div class="form-group">
                            <label>Postal Code</label>
                            <input type="text" name="postal_code" required value="{{ old('postal_code', '94105') }}" class="form-input">
                        </div>
                        <div class="form-group">
                            <label>Country</label>
                            <input type="text" name="country" required value="{{ old('country', 'United States') }}" class="form-input">
                        </div>
                    </div>
                </div>

                <!-- Shipping Options -->
                <div class="form-card">
                    <h3>🚀 2. Delivery Speed</h3>
                    <div class="payment-option-box">
                        <label class="payment-label">
                            <input type="radio" name="shipping_speed" value="standard" checked>
                            <div>
                                <div style="font-weight: 700;">Standard Courier (3-5 Business Days)</div>
                                <div style="font-size: 0.8rem; color: var(--text-muted);">{{ $summary['shipping'] === 0.0 ? 'Free Shipping Qualified' : '$15.00 Flat Rate' }}</div>
                            </div>
                        </label>
                        <label class="payment-label">
                            <input type="radio" name="shipping_speed" value="express">
                            <div>
                                <div style="font-weight: 700;">Hyperion Air Express (1-2 Business Days)</div>
                                <div style="font-size: 0.8rem; color: var(--text-muted);">Priority air hub dispatch with insurance</div>
                            </div>
                        </label>
                    </div>
                </div>

                <!-- Payment Methods -->
                <div class="form-card">
                    <h3>💳 3. Payment Method</h3>
                    <div class="payment-option-box">
                        <label class="payment-label">
                            <input type="radio" name="payment_method" value="credit_card" checked>
                            <div>
                                <div style="font-weight: 700;">Credit / Debit Card (Stripe Mock)</div>
                                <div style="font-size: 0.8rem; color: var(--text-muted);">Instant authorization, 256-bit encrypted</div>
                            </div>
                        </label>
                        <label class="payment-label">
                            <input type="radio" name="payment_method" value="paypal">
                            <div>
                                <div style="font-weight: 700;">PayPal Express</div>
                                <div style="font-size: 0.8rem; color: var(--text-muted);">Pay securely using your PayPal balance</div>
                            </div>
                        </label>
                        <label class="payment-label">
                            <input type="radio" name="payment_method" value="cod">
                            <div>
                                <div style="font-weight: 700;">Cash / Invoice on Delivery</div>
                                <div style="font-size: 0.8rem; color: var(--text-muted);">Pay upon physical courier reception</div>
                            </div>
                        </label>
                    </div>
                </div>
            </div>

            <!-- Right: Order Summary -->
            <div class="checkout-summary-card">
                <h3 style="font-size: 1.25rem; font-weight: 800; margin-bottom: 1.25rem; border-bottom: 1px solid var(--border-color); padding-bottom: 0.75rem;">
                    Items ({{ $summary['item_count'] }})
                </h3>

                <div style="max-height: 250px; overflow-y: auto; margin-bottom: 1.5rem; padding-right: 0.5rem;">
                    @foreach($summary['items'] as $it)
                        <div style="display: flex; justify-content: space-between; margin-bottom: 0.75rem; font-size: 0.9rem;">
                            <div>
                                <span style="font-weight: 600;">{{ $it['name'] }}</span>
                                <span style="color: var(--text-muted); font-size: 0.8rem;"> &times; {{ $it['quantity'] }}</span>
                            </div>
                            <span style="font-weight: 700;">${{ number_format($it['line_total'], 2) }}</span>
                        </div>
                    @endforeach
                </div>

                <div style="border-top: 1px solid var(--border-color); padding-top: 1rem; margin-bottom: 1.5rem;">
                    <div style="display: flex; justify-content: space-between; margin-bottom: 0.5rem; font-size: 0.95rem; color: var(--text-muted);">
                        <span>Subtotal</span>
                        <span>${{ number_format($summary['subtotal'], 2) }}</span>
                    </div>
                    @if($summary['discount_amount'] > 0)
                        <div style="display: flex; justify-content: space-between; margin-bottom: 0.5rem; font-size: 0.95rem; color: var(--accent-green); font-weight: 600;">
                            <span>Discount ({{ $summary['coupon']['code'] }})</span>
                            <span>-${{ number_format($summary['discount_amount'], 2) }}</span>
                        </div>
                    @endif
                    <div style="display: flex; justify-content: space-between; margin-bottom: 0.5rem; font-size: 0.95rem; color: var(--text-muted);">
                        <span>Estimated Tax (8%)</span>
                        <span>${{ number_format($summary['tax'], 2) }}</span>
                    </div>
                    <div style="display: flex; justify-content: space-between; margin-bottom: 0.5rem; font-size: 0.95rem; color: var(--text-muted);">
                        <span>Shipping</span>
                        <span>{{ $summary['shipping'] === 0.0 ? 'FREE' : '$' . number_format($summary['shipping'], 2) }}</span>
                    </div>
                    <div style="display: flex; justify-content: space-between; margin-top: 1rem; padding-top: 1rem; border-top: 1px solid var(--border-color); font-size: 1.35rem; font-weight: 800; color: var(--text-main);">
                        <span>Total Due</span>
                        <span style="color: var(--accent);">${{ number_format($summary['total'], 2) }}</span>
                    </div>
                </div>

                <button type="submit" class="btn btn-primary btn-block" style="padding: 1rem; font-size: 1.1rem; font-weight: 800; box-shadow: 0 0 25px var(--accent-glow);">
                    🔒 Authorize & Place Order (${{ number_format($summary['total'], 2) }})
                </button>

                <div style="text-align: center; margin-top: 1rem; font-size: 0.8rem; color: var(--text-muted);">
                    🛡️ 100% Satisfaction Guarantee &bull; 30-Day Free Returns
                </div>
            </div>
        </div>
    </form>
</div>
@endsection
