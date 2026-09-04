<?php

namespace App\Observers;

use App\Events\OrderPlaced;
use App\Models\Order;
use Illuminate\Support\Str;

class OrderObserver
{
    public function creating(Order $order): void
    {
        if (empty($order->order_number)) {
            $order->order_number = 'ORD-' . strtoupper(Str::random(8));
        }
    }

    public function created(Order $order): void
    {
        OrderPlaced::dispatch($order);
    }

    public function updating(Order $order): void
    {
        if ($order->isDirty('status')) {
            $old = $order->getOriginal('status');
            $new = $order->status;

            $order->activities()->create([
                'action' => 'status_transition',
                'description' => "Status changed from {$old->value} to {$new->value}",
                'properties' => [
                    'from' => $old->value,
                    'to' => $new->value,
                ],
            ]);
        }
    }
}
