<?php

namespace App\Services;

use Illuminate\Support\Facades\Log;
use Midtrans\Config;
use Midtrans\Notification;
use Midtrans\Snap;

class MidtransService
{
    public function __construct()
    {
        // Set midtrans configuration
        Config::$serverKey = config('midtrans.server_key');
        Config::$isProduction = config('midtrans.is_production');
        Config::$isSanitized = config('midtrans.is_sanitized');
        Config::$is3ds = config('midtrans.is_3ds');
    }

    public function createSnapToken(array $params): string
    {
        try {
            return Snap::getSnapToken($params);
        } catch (\Exception $e) {
            Log::error('Failed to create snap token: '.$e->getMessage());
            throw $e;
        }
    }

    public function handleNotification(): array
    {
        try {
            // Automatically loads data from the request
            $notification = new Notification;

            return [
                'order_id' => $notification->order_id,
                'transaction_status' => $notification->transaction_status,
                'gross_amount' => $notification->gross_amount,
                'custom_field1' => $notification->custom_field1, // User ID
                'custom_field2' => $notification->custom_field2, // Mortgage Request ID
            ];
        } catch (\Exception $e) {
            Log::error('Midtrans Notification Error: '.$e->getMessage());
            throw $e;
        }
    }
}
