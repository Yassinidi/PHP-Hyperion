<?php

namespace App\Enums;

enum CustomerTier: string
{
    case Standard = 'standard';
    case Gold = 'gold';
    case VIP = 'vip';

    public function discountRate(): float
    {
        return match($this) {
            self::Standard => 0.0,
            self::Gold => 0.05,
            self::VIP => 0.15,
        };
    }

    public function maxOrderItems(): int
    {
        return match($this) {
            self::Standard => 10,
            self::Gold => 25,
            self::VIP => 100,
        };
    }
}
