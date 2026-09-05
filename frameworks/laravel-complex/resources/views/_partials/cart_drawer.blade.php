<div id="cart-drawer-overlay" class="cart-overlay" onclick="closeCartDrawer()"></div>

<div id="cart-drawer" class="cart-drawer">
    <div class="cart-drawer-header">
        <div class="cart-title-row">
            <span class="cart-icon">🛒</span>
            <h3>Shopping Cart</h3>
            <span id="drawer-badge" class="drawer-badge">{{ $cartCount ?? 0 }}</span>
        </div>
        <button type="button" class="btn-close" onclick="closeCartDrawer()">&times;</button>
    </div>

    <div id="drawer-items-container" class="cart-drawer-body">
        <div class="cart-loading">Loading cart...</div>
    </div>

    <div class="cart-drawer-footer">
        <div class="cart-coupon-row">
            <input type="text" id="drawer-coupon-input" placeholder="Coupon (e.g. HYPERION20)" class="form-input-sm">
            <button type="button" onclick="applyDrawerCoupon()" class="btn-sm btn-outline">Apply</button>
        </div>
        <div id="drawer-coupon-msg" class="drawer-coupon-msg"></div>

        <div class="cart-summary-breakdown">
            <div class="summary-row">
                <span>Subtotal</span>
                <span id="drawer-subtotal">$0.00</span>
            </div>
            <div id="drawer-discount-row" class="summary-row discount-text" style="display: none;">
                <span>Discount</span>
                <span id="drawer-discount">-$0.00</span>
            </div>
            <div class="summary-row">
                <span>Est. Tax (8%)</span>
                <span id="drawer-tax">$0.00</span>
            </div>
            <div class="summary-row">
                <span>Shipping</span>
                <span id="drawer-shipping">Calculated at checkout</span>
            </div>
            <div class="summary-row total-row">
                <span>Estimated Total</span>
                <span id="drawer-total">$0.00</span>
            </div>
        </div>

        <div class="drawer-actions">
            <a href="{{ route('checkout.index') }}" class="btn btn-primary btn-block">Proceed to Checkout →</a>
            <a href="{{ route('cart.index') }}" class="btn btn-subtle btn-block" style="margin-top: 0.5rem;">View Full Cart</a>
        </div>
    </div>
</div>

<style>
.cart-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(4px);
    z-index: 998;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.3s ease;
}
.cart-overlay.open {
    opacity: 1;
    pointer-events: auto;
}
.cart-drawer {
    position: fixed;
    top: 0;
    right: -450px;
    width: 420px;
    max-width: 90vw;
    height: 100vh;
    background: var(--bg-card);
    border-left: 1px solid var(--border-color);
    box-shadow: -10px 0 30px rgba(0, 0, 0, 0.5);
    z-index: 999;
    display: flex;
    flex-direction: column;
    transition: right 0.3s cubic-bezier(0.16, 1, 0.3, 1);
}
.cart-drawer.open {
    right: 0;
}
.cart-drawer-header {
    padding: 1.25rem 1.5rem;
    border-bottom: 1px solid var(--border-color);
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: rgba(0, 0, 0, 0.15);
}
.cart-title-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
}
.cart-title-row h3 {
    font-size: 1.15rem;
    font-weight: 700;
}
.drawer-badge {
    background: var(--accent);
    color: white;
    font-size: 0.75rem;
    font-weight: 700;
    padding: 0.15rem 0.5rem;
    border-radius: 9999px;
}
.btn-close {
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 1.5rem;
    cursor: pointer;
    line-height: 1;
}
.btn-close:hover {
    color: var(--text-main);
}
.cart-drawer-body {
    flex: 1;
    overflow-y: auto;
    padding: 1.25rem 1.5rem;
}
.cart-drawer-footer {
    padding: 1.25rem 1.5rem;
    border-top: 1px solid var(--border-color);
    background: rgba(0, 0, 0, 0.2);
}
.drawer-item-card {
    display: flex;
    gap: 1rem;
    padding: 1rem 0;
    border-bottom: 1px solid var(--border-color);
}
.drawer-item-card:last-child {
    border-bottom: none;
}
.drawer-item-icon {
    width: 48px;
    height: 48px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.05);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1.5rem;
    border: 1px solid var(--border-color);
}
.drawer-item-details {
    flex: 1;
}
.drawer-item-name {
    font-weight: 600;
    font-size: 0.95rem;
    margin-bottom: 0.25rem;
}
.drawer-item-price {
    font-weight: 700;
    color: var(--accent);
    font-size: 0.9rem;
}
.drawer-item-qty {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.5rem;
}
.qty-btn {
    width: 24px;
    height: 24px;
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid var(--border-color);
    color: var(--text-main);
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    font-size: 0.85rem;
}
.qty-btn:hover {
    background: var(--accent);
}
.qty-val {
    font-size: 0.85rem;
    font-weight: 600;
    min-width: 20px;
    text-align: center;
}
.btn-remove-item {
    background: none;
    border: none;
    color: #ef4444;
    font-size: 0.8rem;
    cursor: pointer;
    margin-left: auto;
}
.cart-empty-state {
    text-align: center;
    padding: 3rem 1rem;
    color: var(--text-muted);
}
.cart-coupon-row {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 1rem;
}
.form-input-sm {
    flex: 1;
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    padding: 0.4rem 0.75rem;
    color: var(--text-main);
    font-size: 0.85rem;
}
.btn-sm {
    padding: 0.4rem 0.75rem;
    font-size: 0.85rem;
}
.drawer-coupon-msg {
    font-size: 0.8rem;
    margin-top: -0.5rem;
    margin-bottom: 0.75rem;
}
.summary-row {
    display: flex;
    justify-content: space-between;
    font-size: 0.85rem;
    color: var(--text-muted);
    margin-bottom: 0.35rem;
}
.discount-text {
    color: #10b981;
    font-weight: 600;
}
.total-row {
    font-size: 1.1rem;
    font-weight: 700;
    color: var(--text-main);
    border-top: 1px solid var(--border-color);
    padding-top: 0.5rem;
    margin-top: 0.5rem;
}
.btn-block {
    display: block;
    width: 100%;
    text-align: center;
    box-sizing: border-box;
}
</style>

<script>
function openCartDrawer() {
    document.getElementById('cart-drawer').classList.add('open');
    document.getElementById('cart-drawer-overlay').classList.add('open');
    refreshCartDrawer();
}

function closeCartDrawer() {
    document.getElementById('cart-drawer').classList.remove('open');
    document.getElementById('cart-drawer-overlay').classList.remove('open');
}

async function refreshCartDrawer() {
    try {
        const res = await fetch('/api/v1/cart');
        const data = await res.json();
        renderDrawerItems(data);
    } catch (e) {
        console.error('Failed to fetch cart:', e);
    }
}

function renderDrawerItems(summary) {
    const container = document.getElementById('drawer-items-container');
    const badge = document.getElementById('drawer-badge');
    const navBadge = document.getElementById('nav-cart-badge');
    
    if (badge) badge.innerText = summary.item_count;
    if (navBadge) {
        navBadge.innerText = summary.item_count;
        navBadge.style.display = summary.item_count > 0 ? 'inline-block' : 'none';
    }

    document.getElementById('drawer-subtotal').innerText = '$' + summary.subtotal.toFixed(2);
    document.getElementById('drawer-tax').innerText = '$' + summary.tax.toFixed(2);
    document.getElementById('drawer-shipping').innerText = summary.shipping === 0 ? 'FREE' : '$' + summary.shipping.toFixed(2);
    document.getElementById('drawer-total').innerText = '$' + summary.total.toFixed(2);

    const discountRow = document.getElementById('drawer-discount-row');
    if (summary.discount_amount > 0) {
        discountRow.style.display = 'flex';
        document.getElementById('drawer-discount').innerText = '-$' + summary.discount_amount.toFixed(2);
    } else {
        discountRow.style.display = 'none';
    }

    if (summary.is_empty) {
        container.innerHTML = `
            <div class="cart-empty-state">
                <div style="font-size: 3rem; margin-bottom: 1rem;">🛍️</div>
                <h4 style="margin-bottom: 0.5rem;">Your Cart is Empty</h4>
                <p style="font-size: 0.85rem;">Discover our high-performance hardware gear and add items to your cart.</p>
            </div>
        `;
        return;
    }

    let html = '';
    summary.items.forEach(item => {
        html += `
            <div class="drawer-item-card">
                <div class="drawer-item-icon">💻</div>
                <div class="drawer-item-details">
                    <div class="drawer-item-name">${item.name}</div>
                    <div class="drawer-item-price">$${item.price.toFixed(2)}</div>
                    <div class="drawer-item-qty">
                        <button type="button" class="qty-btn" onclick="updateCartQty(${item.id}, ${item.quantity - 1})">-</button>
                        <span class="qty-val">${item.quantity}</span>
                        <button type="button" class="qty-btn" onclick="updateCartQty(${item.id}, ${item.quantity + 1})">+</button>
                        <button type="button" class="btn-remove-item" onclick="removeCartItem(${item.id})">Remove</button>
                    </div>
                </div>
            </div>
        `;
    });
    container.innerHTML = html;
}

async function quickAddToCart(productId, quantity = 1) {
    try {
        const res = await fetch('/api/v1/cart/add', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'X-CSRF-TOKEN': '{{ csrf_token() }}',
                'Accept': 'application/json'
            },
            body: JSON.stringify({ product_id: productId, quantity: quantity })
        });
        const data = await res.json();
        if (data.success) {
            renderDrawerItems(data.summary);
            openCartDrawer();
        }
    } catch (e) {
        console.error('Add to cart error:', e);
    }
}

async function updateCartQty(productId, quantity) {
    try {
        const res = await fetch('/api/v1/cart/update', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'X-CSRF-TOKEN': '{{ csrf_token() }}',
                'Accept': 'application/json'
            },
            body: JSON.stringify({ product_id: productId, quantity: quantity })
        });
        const data = await res.json();
        if (data.success) {
            renderDrawerItems(data.summary);
        }
    } catch (e) {
        console.error('Update qty error:', e);
    }
}

async function removeCartItem(productId) {
    try {
        const res = await fetch(`/api/v1/cart/remove/${productId}`, {
            method: 'POST',
            headers: {
                'X-CSRF-TOKEN': '{{ csrf_token() }}',
                'Accept': 'application/json'
            }
        });
        const data = await res.json();
        if (data.success) {
            renderDrawerItems(data.summary);
        }
    } catch (e) {
        console.error('Remove error:', e);
    }
}

async function applyDrawerCoupon() {
    const input = document.getElementById('drawer-coupon-input');
    const msg = document.getElementById('drawer-coupon-msg');
    const code = input.value.trim();
    if (!code) return;

    try {
        const res = await fetch('/api/v1/cart/coupon', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'X-CSRF-TOKEN': '{{ csrf_token() }}',
                'Accept': 'application/json'
            },
            body: JSON.stringify({ coupon_code: code })
        });
        const data = await res.json();
        msg.innerText = data.message;
        msg.style.color = data.success ? '#10b981' : '#ef4444';
        if (data.success) {
            renderDrawerItems(data.summary);
        }
    } catch (e) {
        console.error('Coupon error:', e);
    }
}
</script>
