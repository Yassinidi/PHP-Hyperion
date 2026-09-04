<?php

namespace App\Listeners;

use App\Events\ProductStockUpdated;
use App\Events\ProductPriceChanged;
use Illuminate\Support\Facades\Cache;

class RecordAuditLogListener
{
    /**
     * Handle ProductStockUpdated event.
     */
    public function handleStockUpdated(ProductStockUpdated $event): void
    {
        $logs = Cache::get('stock_audit_log', []);
        $logs[] = [
            'type' => 'stock_updated',
            'product_id' => $event->product->id,
            'sku' => $event->product->sku,
            'old_stock' => $event->oldStock,
            'new_stock' => $event->newStock,
            'timestamp' => time(),
        ];
        Cache::put('stock_audit_log', $logs, 3600);
    }

    /**
     * Handle ProductPriceChanged event.
     */
    public function handlePriceChanged(ProductPriceChanged $event): void
    {
        $logs = Cache::get('price_audit_log', []);
        $logs[] = [
            'type' => 'price_changed',
            'product_id' => $event->product->id,
            'sku' => $event->product->sku,
            'old_price' => $event->oldPrice,
            'new_price' => $event->newPrice,
            'timestamp' => time(),
        ];
        Cache::put('price_audit_log', $logs, 3600);
    }
}
