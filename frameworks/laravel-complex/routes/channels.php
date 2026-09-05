<?php

use App\Models\Customer;
use App\Models\Order;
use App\Models\User;
use Illuminate\Support\Facades\Broadcast;

/*
|--------------------------------------------------------------------------
| Broadcast Channels
|--------------------------------------------------------------------------
|
| Here you may register all of the event broadcasting channels that your
| application supports. The given channel authorization callbacks are
| used to check if an authenticated user can listen to the channel.
|
*/

// Private channel for a specific order
Broadcast::channel('orders.{orderId}', function ($user, int $orderId) {
    // Both User and Customer models or admin users can listen
    $order = Order::find($orderId);
    if (!$order) {
        return false;
    }

    if ($user instanceof User) {
        return true;
    }

    if ($user instanceof Customer) {
        return (int) $user->id === (int) $order->customer_id;
    }

    return true;
});

// Private channel for warehouse inventory alerts
Broadcast::channel('inventory.alerts', function ($user) {
    return true;
});

// Private channel for attachment file notifications
Broadcast::channel('files.{uuid}', function ($user, string $uuid) {
    return !empty($uuid);
});
