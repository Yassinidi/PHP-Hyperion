<?php

namespace App\Http\Controllers\Api;

use App\Http\Controllers\Controller;
use App\Http\Requests\CreateOrderRequest;
use App\Http\Resources\OrderResource;
use App\Services\OrderService;
use Illuminate\Http\JsonResponse;

class OrderController extends Controller
{
    public function __construct(
        protected OrderService $orderService
    ) {}

    public function store(CreateOrderRequest $request): JsonResponse
    {
        $validated = $request->validated();

        try {
            $order = $this->orderService->createOrder(
                $validated['customer_id'],
                $validated['items'],
                $validated['payment_method'] ?? 'credit_card'
            );

            return (new OrderResource($order))
                ->additional(['message' => 'Order created and processed successfully'])
                ->response()
                ->setStatusCode(201);
        } catch (\RuntimeException $e) {
            return response()->json([
                'error' => $e->getMessage(),
            ], 422);
        }
    }

    public function show(int $id): OrderResource
    {
        $order = \App\Models\Order::with(['customer', 'items.product', 'activities'])->findOrFail($id);

        return new OrderResource($order);
    }
}
