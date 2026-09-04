<?php

namespace App\Jobs;

use App\Models\ActivityLog;
use App\Models\Order;
use Illuminate\Bus\Queueable;
use Illuminate\Contracts\Queue\ShouldQueue;
use Illuminate\Foundation\Bus\Dispatchable;
use Illuminate\Queue\InteractsWithQueue;
use Illuminate\Queue\SerializesModels;

class ProcessOrderInvoiceJob implements ShouldQueue
{
    use Dispatchable, InteractsWithQueue, Queueable, SerializesModels;

    public int $tries = 3;
    public int $timeout = 60;

    public function __construct(
        public int $orderId
    ) {}

    public function handle(): void
    {
        $order = Order::with(['customer', 'items.product'])->findOrFail($this->orderId);

        // Simulate invoice generation and cryptographic checksum
        $invoiceNumber = 'INV-' . strtoupper(substr(hash('sha256', (string)$order->id . $order->order_number), 0, 12));
        
        ActivityLog::create([
            'subject_type' => get_class($order),
            'subject_id' => $order->id,
            'action' => 'invoice_generated',
            'description' => "Invoice {$invoiceNumber} compiled asynchronously for order {$order->order_number}",
            'properties' => [
                'invoice_number' => $invoiceNumber,
                'total_amount' => $order->total_amount,
                'items_count' => $order->items->count(),
                'processed_at' => now()->toIso8601String(),
            ],
        ]);
    }
}
