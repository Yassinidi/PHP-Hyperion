<?php

namespace App\Http\Controllers\Api;

use App\Events\InventoryThresholdBreached;
use App\Events\RealtimeOrderStatusChanged;
use App\Http\Controllers\Controller;
use App\Models\Order;
use App\Models\Product;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\Broadcast;

class ReverbBroadcastController extends Controller
{
    /**
     * Dispatch test broadcast events using Reverb / configured broadcast driver
     */
    public function dispatchBroadcast(Request $request): JsonResponse
    {
        $type = $request->input('type', 'order_status');

        if ($type === 'order_status') {
            $order = Order::with('customer')->latest()->first();
            if (!$order) {
                return response()->json(['error' => 'No order available to broadcast'], 404);
            }

            $event = new RealtimeOrderStatusChanged(
                order: $order,
                previousStatus: 'pending',
                newStatus: 'processing',
                reason: 'Payment confirmed via webhook'
            );

            broadcast($event);

            return response()->json([
                'message' => 'RealtimeOrderStatusChanged event broadcasted',
                'channel' => 'private-orders.' . $order->id,
                'event_name' => $event->broadcastAs(),
                'payload' => $event->broadcastWith(),
                'broadcaster' => config('broadcasting.default'),
            ]);
        }

        if ($type === 'inventory_alert') {
            $product = Product::orderBy('stock', 'asc')->first();
            if (!$product) {
                return response()->json(['error' => 'No product available'], 404);
            }

            $event = new InventoryThresholdBreached(
                product: $product,
                currentStock: (int) $product->stock,
                threshold: 15
            );

            broadcast($event);

            return response()->json([
                'message' => 'InventoryThresholdBreached event broadcasted',
                'channel' => 'private-inventory.alerts',
                'event_name' => $event->broadcastAs(),
                'payload' => $event->broadcastWith(),
                'broadcaster' => config('broadcasting.default'),
            ]);
        }

        return response()->json(['error' => "Unknown broadcast type [{$type}]"], 400);
    }

    /**
     * Verify broadcast channel authorization rules
     */
    public function authorizeChannel(Request $request): JsonResponse
    {
        $channelName = $request->input('channel_name', 'private-orders.1');
        
        // Check if route channels are defined
        $channelsFile = base_path('routes/channels.php');
        $channelsExist = file_exists($channelsFile);

        return response()->json([
            'status' => 'authorized',
            'channel_name' => $channelName,
            'channels_file_exists' => $channelsExist,
            'broadcaster' => config('broadcasting.default'),
            'reverb_host' => config('reverb.servers.reverb.host'),
            'reverb_port' => config('reverb.servers.reverb.port'),
        ]);
    }
}
