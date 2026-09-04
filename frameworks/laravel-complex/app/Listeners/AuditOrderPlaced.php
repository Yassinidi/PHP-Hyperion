<?php

namespace App\Listeners;

use App\Events\OrderPlaced;

class AuditOrderPlaced
{
    public function handle(OrderPlaced $event): void
    {
        $order = $event->order;

        // Log polymorphic activity on the Order
        $order->activities()->create([
            'action' => 'order_placed',
            'description' => "Order {$order->order_number} confirmed with total \${$order->total_amount}",
            'properties' => [
                'order_number' => $order->order_number,
                'total_amount' => $order->total_amount,
                'status' => $order->status->value,
                'items_count' => $order->items()->count(),
            ],
        ]);

        // Log polymorphic activity on Customer
        if ($customer = $order->customer) {
            $customer->activities()->create([
                'action' => 'purchase',
                'description' => "Purchased order {$order->order_number}",
                'properties' => [
                    'order_id' => $order->id,
                    'total_amount' => $order->total_amount,
                ],
            ]);
        }
    }
}
