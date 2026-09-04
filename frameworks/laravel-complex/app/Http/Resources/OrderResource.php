<?php

namespace App\Http\Resources;

use Illuminate\Http\Request;
use Illuminate\Http\Resources\Json\JsonResource;

class OrderResource extends JsonResource
{
    public function toArray(Request $request): array
    {
        return [
            'id' => $this->id,
            'order_number' => $this->order_number,
            'total_amount' => (float) $this->total_amount,
            'formatted_total' => '$' . number_format($this->total_amount, 2),
            'status' => [
                'code' => $this->status->value,
                'label' => $this->status->label(),
                'badge' => $this->status->badgeClass(),
            ],
            'payment_method' => $this->payment_method,
            'customer' => [
                'id' => $this->customer?->id,
                'name' => $this->customer?->name,
                'email' => $this->customer?->email,
                'tier' => $this->customer?->tier?->value,
            ],
            'items' => OrderItemResource::collection($this->whenLoaded('items')),
            'activities' => $this->whenLoaded('activities'),
            'created_at' => $this->created_at?->toIso8601String(),
        ];
    }
}
