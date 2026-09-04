<?php

namespace App\Events;

use App\Models\Order;
use Illuminate\Broadcasting\InteractsWithSockets;
use Illuminate\Foundation\Events\Dispatchable;
use Illuminate\Queue\SerializesModels;

class OrderShipped
{
    use Dispatchable, InteractsWithSockets, SerializesModels;

    public function __construct(
        public Order $order,
        public string $trackingNumber = ''
    ) {
        if (empty($this->trackingNumber)) {
            $this->trackingNumber = 'TRK-' . strtoupper(substr(md5((string) $order->id . microtime()), 0, 10));
        }
    }
}
