<?php

namespace App\Events;

use App\Models\Product;
use Illuminate\Broadcasting\Channel;
use Illuminate\Broadcasting\InteractsWithSockets;
use Illuminate\Broadcasting\PrivateChannel;
use Illuminate\Contracts\Broadcasting\ShouldBroadcastNow;
use Illuminate\Foundation\Events\Dispatchable;
use Illuminate\Queue\SerializesModels;

class InventoryThresholdBreached implements ShouldBroadcastNow
{
    use Dispatchable, InteractsWithSockets, SerializesModels;

    public function __construct(
        public Product $product,
        public int $currentStock,
        public int $threshold
    ) {}

    public function broadcastOn(): array
    {
        return [
            new PrivateChannel('inventory.alerts'),
        ];
    }

    public function broadcastAs(): string
    {
        return 'inventory.threshold.breached';
    }

    public function broadcastWith(): array
    {
        return [
            'product_id' => $this->product->id,
            'sku' => $this->product->sku,
            'name' => $this->product->name,
            'current_stock' => $this->currentStock,
            'threshold' => $this->threshold,
            'requires_reorder' => $this->currentStock <= ($this->threshold / 2),
            'timestamp' => now()->toIso8601String(),
        ];
    }
}
