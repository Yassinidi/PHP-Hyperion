<?php

namespace App\Jobs;

use App\Models\Product;
use App\Models\Category;
use Illuminate\Bus\Queueable;
use Illuminate\Contracts\Queue\ShouldQueue;
use Illuminate\Foundation\Bus\Dispatchable;
use Illuminate\Queue\InteractsWithQueue;
use Illuminate\Queue\SerializesModels;
use Illuminate\Support\Facades\Log;

class ProcessInventoryAuditJob implements ShouldQueue
{
    use Dispatchable, InteractsWithQueue, Queueable, SerializesModels;

    public Product $product;
    public ?Category $category;
    public int $previousStock;
    public int $newStock;

    /**
     * Create a new job instance.
     */
    public function __construct(Product $product, int $previousStock, int $newStock)
    {
        $this->product = $product;
        $this->category = $product->category;
        $this->previousStock = $previousStock;
        $this->newStock = $newStock;
    }

    /**
     * Execute the job.
     */
    public function handle(): array
    {
        $diff = $this->newStock - $this->previousStock;
        $status = $this->newStock === 0 ? 'out_of_stock' : ($this->newStock < 10 ? 'low_stock' : 'in_stock');
        
        $this->product->update([
            'stock' => $this->newStock,
            'status' => $status,
        ]);

        return [
            'product_id' => $this->product->id,
            'sku' => $this->product->sku,
            'stock_diff' => $diff,
            'new_status' => $status,
            'category_name' => $this->category ? $this->category->name : null,
        ];
    }
}
