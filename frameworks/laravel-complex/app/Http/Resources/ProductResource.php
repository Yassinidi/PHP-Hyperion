<?php

namespace App\Http\Resources;

use Illuminate\Http\Request;
use Illuminate\Http\Resources\Json\JsonResource;

class ProductResource extends JsonResource
{
    public function toArray(Request $request): array
    {
        return [
            'id' => $this->id,
            'sku' => $this->sku,
            'name' => $this->name,
            'slug' => $this->slug,
            'description' => $this->description,
            'price' => (float) $this->price,
            'formatted_price' => '$' . number_format($this->price, 2),
            'stock' => (int) $this->stock,
            'is_in_stock' => $this->stock > 0,
            $this->mergeWhen($this->stock <= 15, [
                'stock_status' => 'LOW_STOCK',
                'alert' => "Only {$this->stock} units left in warehouse!",
            ]),
            'average_rating' => $this->average_rating,
            'category' => new CategoryResource($this->whenLoaded('category')),
            'tags' => TagResource::collection($this->whenLoaded('tags')),
            'reviews_count' => $this->whenCounted('reviews'),
            'created_at' => $this->created_at?->toIso8601String(),
        ];
    }
}
