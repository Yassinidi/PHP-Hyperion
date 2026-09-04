<?php

namespace App\Listeners;

use App\Events\OrderShipped;
use App\Models\ActivityLog;
use App\Notifications\OrderInvoiceNotification;
use Illuminate\Contracts\Queue\ShouldQueue;
use Illuminate\Queue\InteractsWithQueue;

class SendOrderShippedNotification implements ShouldQueue
{
    use InteractsWithQueue;

    /**
     * The number of times the queued listener may be attempted.
     */
    public int $tries = 3;

    public function handle(OrderShipped $event): void
    {
        $order = $event->order;

        if ($order->customer) {
            $order->customer->notify(new OrderInvoiceNotification($order, $event->trackingNumber));
        }

        ActivityLog::create([
            'subject_type' => get_class($order),
            'subject_id' => $order->id,
            'action' => 'order_shipped_notification_dispatched',
            'description' => "Shipped notification dispatched for order {$order->order_number} (Tracking: {$event->trackingNumber})",
            'properties' => [
                'tracking_number' => $event->trackingNumber,
                'customer_id' => $order->customer_id,
                'via' => ['database', 'mail'],
            ],
        ]);
    }
}
