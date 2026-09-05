<?php

namespace App\Services;

use App\Models\Product;
use Illuminate\Support\Facades\Session;

class CartService
{
    protected const CART_SESSION_KEY = 'hyperion_ecommerce_cart';
    protected const COUPON_SESSION_KEY = 'hyperion_ecommerce_coupon';

    /**
     * Get raw cart contents
     */
    public function getCart(): array
    {
        return Session::get(self::CART_SESSION_KEY, []);
    }

    /**
     * Add item to cart
     */
    public function addItem(int $productId, int $quantity = 1): array
    {
        $cart = $this->getCart();
        $product = Product::with('category')->findOrFail($productId);

        if (isset($cart[$productId])) {
            $cart[$productId]['quantity'] += max(1, $quantity);
        } else {
            $cart[$productId] = [
                'id' => $product->id,
                'name' => $product->name,
                'slug' => $product->slug,
                'sku' => $product->sku,
                'price' => (float) $product->price,
                'quantity' => max(1, $quantity),
                'category_name' => $product->category?->name ?? 'General',
                'max_stock' => $product->stock,
            ];
        }

        // Cap at available stock if stock > 0
        if ($product->stock > 0 && $cart[$productId]['quantity'] > $product->stock) {
            $cart[$productId]['quantity'] = $product->stock;
        }

        Session::put(self::CART_SESSION_KEY, $cart);
        return $this->getCartSummary();
    }

    /**
     * Update item quantity
     */
    public function updateQuantity(int $productId, int $quantity): array
    {
        $cart = $this->getCart();

        if ($quantity <= 0) {
            return $this->removeItem($productId);
        }

        if (isset($cart[$productId])) {
            $product = Product::find($productId);
            $maxStock = $product ? $product->stock : 999;
            $cart[$productId]['quantity'] = min($quantity, $maxStock);
            Session::put(self::CART_SESSION_KEY, $cart);
        }

        return $this->getCartSummary();
    }

    /**
     * Remove item from cart
     */
    public function removeItem(int $productId): array
    {
        $cart = $this->getCart();
        unset($cart[$productId]);
        Session::put(self::CART_SESSION_KEY, $cart);
        return $this->getCartSummary();
    }

    /**
     * Clear all items in cart
     */
    public function clearCart(): void
    {
        Session::forget(self::CART_SESSION_KEY);
        Session::forget(self::COUPON_SESSION_KEY);
    }

    /**
     * Get total item count in cart
     */
    public function getItemCount(): int
    {
        $cart = $this->getCart();
        return (int) array_sum(array_column($cart, 'quantity'));
    }

    /**
     * Apply coupon code
     */
    public function applyCoupon(string $code): array
    {
        $code = strtoupper(trim($code));
        $validCoupons = [
            'HYPERION20' => ['discount_pct' => 20, 'label' => 'Hyperion 20% Off Launch Special'],
            'PROSTORE10' => ['discount_pct' => 10, 'label' => 'Pro Store 10% Welcome Discount'],
            'FREESHIP'   => ['discount_pct' => 0, 'free_shipping' => true, 'label' => 'Free Express Shipping'],
        ];

        if (isset($validCoupons[$code])) {
            Session::put(self::COUPON_SESSION_KEY, array_merge(['code' => $code], $validCoupons[$code]));
            return [
                'success' => true,
                'message' => "Coupon '{$code}' applied successfully!",
                'summary' => $this->getCartSummary(),
            ];
        }

        return [
            'success' => false,
            'message' => "Invalid or expired coupon code '{$code}'. Try HYPERION20 for 20% off.",
            'summary' => $this->getCartSummary(),
        ];
    }

    /**
     * Remove active coupon
     */
    public function removeCoupon(): array
    {
        Session::forget(self::COUPON_SESSION_KEY);
        return $this->getCartSummary();
    }

    /**
     * Get full cart financial summary
     */
    public function getCartSummary(): array
    {
        $cart = $this->getCart();
        $subtotal = 0.0;
        $items = [];

        foreach ($cart as $id => $item) {
            $lineTotal = round($item['price'] * $item['quantity'], 2);
            $subtotal += $lineTotal;
            $items[] = array_merge($item, ['line_total' => $lineTotal]);
        }

        $coupon = Session::get(self::COUPON_SESSION_KEY);
        $discountAmount = 0.0;
        $isFreeShipping = false;

        if ($coupon) {
            if (!empty($coupon['discount_pct'])) {
                $discountAmount = round($subtotal * ($coupon['discount_pct'] / 100), 2);
            }
            if (!empty($coupon['free_shipping'])) {
                $isFreeShipping = true;
            }
        }

        $taxableAmount = max(0.0, $subtotal - $discountAmount);
        $tax = round($taxableAmount * 0.08, 2); // 8% sales tax

        // Standard shipping $15, free if subtotal >= 100 or free shipping coupon
        $shipping = 0.0;
        if ($subtotal > 0) {
            $shipping = ($subtotal >= 100.0 || $isFreeShipping) ? 0.0 : 15.0;
        }

        $total = max(0.0, round($subtotal - $discountAmount + $tax + $shipping, 2));

        return [
            'items' => $items,
            'item_count' => $this->getItemCount(),
            'subtotal' => $subtotal,
            'discount_amount' => $discountAmount,
            'coupon' => $coupon,
            'tax' => $tax,
            'shipping' => $shipping,
            'total' => $total,
            'is_empty' => empty($items),
        ];
    }
}
