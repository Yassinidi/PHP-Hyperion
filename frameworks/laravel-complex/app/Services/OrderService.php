<?php

namespace App\Services;

use App\Contracts\PaymentGatewayInterface;
use App\Enums\OrderStatus;
use App\Models\Customer;
use App\Models\Order;
use App\Models\Product;
use Illuminate\Support\Facades\DB;

class OrderService
{
    public function __construct(
        protected PaymentGatewayInterface $paymentGateway
    ) {}

    /**
     * Create an order transaction-safely
     *
     * @param int $customerId
     * @param array $items Array of ['product_id' => int, 'quantity' => int]
     * @param string $paymentMethod
     * @return Order
     * @throws \RuntimeException
     */
    public function createOrder(int $customerId, array $items, string $paymentMethod = 'credit_card'): Order
    {
        return DB::transaction(function () use ($customerId, $items, $paymentMethod) {
            $customer = Customer::findOrFail($customerId);
            $subtotalSum = 0.0;
            $preparedItems = [];

            foreach ($items as $itemData) {
                // Lock row for update to prevent race conditions
                $product = Product::lockForUpdate()->findOrFail($itemData['product_id']);

                if ($product->stock < $itemData['quantity']) {
                    throw new \RuntimeException("Insufficient stock for product: {$product->name} (available: {$product->stock})");
                }

                $qty = (int) $itemData['quantity'];
                $unitPrice = (float) $product->price;
                $lineTotal = $qty * $unitPrice;
                $subtotalSum += $lineTotal;

                // Decrement stock
                $product->decrement('stock', $qty);

                // Log activity on the product
                $product->activities()->create([
                    'action' => 'stock_deducted',
                    'description' => "Stock reduced by {$qty} for order placement",
                    'properties' => [
                        'quantity_deducted' => $qty,
                        'remaining_stock' => $product->fresh()->stock,
                    ],
                ]);

                $preparedItems[] = [
                    'product_id' => $product->id,
                    'quantity' => $qty,
                    'unit_price' => $unitPrice,
                    'subtotal' => $lineTotal,
                ];
            }

            // Apply customer tier discount rate
            $finalTotal = round($subtotalSum * $customer->discount_multiplier, 2);

            // Process payment via injected PaymentGatewayInterface
            $charge = $this->paymentGateway->charge($finalTotal, 'USD', [
                'customer_id' => $customer->id,
                'customer_email' => $customer->email,
            ]);

            if (!$charge['success']) {
                throw new \RuntimeException("Payment capture failed on gateway {$charge['gateway']}");
            }

            // Create order (OrderObserver auto-generates order_number and fires OrderPlaced event)
            $order = Order::create([
                'customer_id' => $customer->id,
                'total_amount' => $finalTotal,
                'status' => OrderStatus::Paid,
                'payment_method' => $paymentMethod,
            ]);

            foreach ($preparedItems as $item) {
                $order->items()->create($item);
            }

            return $order->load(['items.product', 'customer', 'activities']);
        });
    }
}
