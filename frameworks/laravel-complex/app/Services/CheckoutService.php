<?php

namespace App\Services;

use App\Enums\OrderStatus;
use App\Events\OrderPlaced;
use App\Models\ActivityLog;
use App\Models\Customer;
use App\Models\Order;
use App\Models\OrderItem;
use App\Models\Product;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Str;

class CheckoutService
{
    public function __construct(
        protected CartService $cartService
    ) {}

    /**
     * Process checkout and persist an Order
     */
    public function processCheckout(array $customerData, string $paymentMethod = 'credit_card', string $shippingSpeed = 'standard'): Order
    {
        $summary = $this->cartService->getCartSummary();

        if ($summary['is_empty']) {
            throw new \RuntimeException("Cannot checkout with an empty cart.");
        }

        return DB::transaction(function () use ($customerData, $paymentMethod, $shippingSpeed, $summary) {
            $fullName = trim(($customerData['first_name'] ?? '') . ' ' . ($customerData['last_name'] ?? ''));
            if (empty($fullName)) {
                $fullName = $customerData['name'] ?? 'Guest Shopper';
            }

            // 1. Find or create customer
            $customer = Customer::firstOrCreate(
                ['email' => $customerData['email']],
                [
                    'name' => $fullName,
                    'tier' => \App\Enums\CustomerTier::Standard,
                ]
            );

            // 2. Generate unique order number
            $orderNumber = 'ORD-' . date('Y') . '-' . strtoupper(Str::random(6));

            // 3. Create Order
            $order = Order::create([
                'customer_id' => $customer->id,
                'order_number' => $orderNumber,
                'total_amount' => $summary['total'],
                'status' => OrderStatus::Paid,
                'payment_method' => $paymentMethod,
            ]);

            // 4. Create Order Items & decrement product stock
            foreach ($summary['items'] as $item) {
                OrderItem::create([
                    'order_id' => $order->id,
                    'product_id' => $item['id'],
                    'quantity' => $item['quantity'],
                    'unit_price' => $item['price'],
                    'subtotal' => $item['line_total'],
                ]);

                // Decrement stock safely
                Product::where('id', $item['id'])
                    ->where('stock', '>=', $item['quantity'])
                    ->decrement('stock', $item['quantity']);
            }

            // 5. Create Activity Log
            ActivityLog::create([
                'subject_type' => Order::class,
                'subject_id' => $order->id,
                'action' => 'checkout_completed',
                'description' => "Order #{$order->order_number} placed successfully for \${$order->total_amount}",
                'properties' => [
                    'item_count' => $summary['item_count'],
                    'payment_method' => $paymentMethod,
                    'shipping_speed' => $shippingSpeed,
                    'coupon' => $summary['coupon']['code'] ?? null,
                ],
            ]);

            // 6. Fire OrderPlaced event (if configured)
            event(new OrderPlaced($order));

            // 7. Clear shopping cart
            $this->cartService->clearCart();

            return $order->load(['customer', 'items.product', 'activities']);
        });
    }
}
