<?php

namespace App\Notifications;

use App\Models\Order;
use Illuminate\Bus\Queueable;
use Illuminate\Contracts\Queue\ShouldQueue;
use Illuminate\Notifications\Messages\MailMessage;
use Illuminate\Notifications\Notification;

class OrderInvoiceNotification extends Notification implements ShouldQueue
{
    use Queueable;

    public function __construct(
        public Order $order,
        public string $trackingNumber = ''
    ) {}

    /**
     * Get the notification's delivery channels.
     *
     * @return array<int, string>
     */
    public function via(object $notifiable): array
    {
        return ['database', 'mail'];
    }

    /**
     * Get the mail representation of the notification.
     */
    public function toMail(object $notifiable): MailMessage
    {
        return (new MailMessage)
            ->subject("Order #{$this->order->order_number} Has Shipped!")
            ->greeting("Hello {$notifiable->name},")
            ->line("Your order {$this->order->order_number} has been shipped.")
            ->line("Tracking Number: {$this->trackingNumber}")
            ->line("Total Paid: \${$this->order->total_amount}")
            ->action('View Order', url("/orders/{$this->order->id}"))
            ->line('Thank you for shopping with Hyperion Enterprise!');
    }

    /**
     * Get the array representation of the notification.
     *
     * @return array<string, mixed>
     */
    public function toArray(object $notifiable): array
    {
        return [
            'order_id' => $this->order->id,
            'order_number' => $this->order->order_number,
            'total_amount' => $this->order->total_amount,
            'tracking_number' => $this->trackingNumber,
            'status' => $this->order->status->value,
            'shipped_at' => now()->toIso8601String(),
        ];
    }
}
