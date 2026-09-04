<?php

namespace App\Enums;

enum OrderStatus: string
{
    case Pending = 'pending';
    case Paid = 'paid';
    case Processing = 'processing';
    case Completed = 'completed';
    case Cancelled = 'cancelled';

    public function label(): string
    {
        return match($this) {
            self::Pending => 'Pending Payment',
            self::Paid => 'Payment Confirmed',
            self::Processing => 'Processing Order',
            self::Completed => 'Order Delivered',
            self::Cancelled => 'Order Cancelled',
        };
    }

    public function badgeClass(): string
    {
        return match($this) {
            self::Pending => 'badge-warning',
            self::Paid, self::Completed => 'badge-success',
            self::Processing => 'badge-info',
            self::Cancelled => 'badge-danger',
        };
    }
}
