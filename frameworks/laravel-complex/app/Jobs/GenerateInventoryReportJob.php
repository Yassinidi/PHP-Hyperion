<?php

namespace App\Jobs;

use App\Models\ActivityLog;
use App\Models\Category;
use App\Models\Product;
use Illuminate\Bus\Batchable;
use Illuminate\Bus\Queueable;
use Illuminate\Contracts\Queue\ShouldQueue;
use Illuminate\Foundation\Bus\Dispatchable;
use Illuminate\Queue\InteractsWithQueue;
use Illuminate\Queue\SerializesModels;
use Illuminate\Support\Facades\Cache;

class GenerateInventoryReportJob implements ShouldQueue
{
    use Batchable, Dispatchable, InteractsWithQueue, Queueable, SerializesModels;

    public int $tries = 2;

    public function __construct(
        public string $reportType = 'standard'
    ) {}

    public function handle(): void
    {
        if ($this->batch() && $this->batch()->cancelled()) {
            return;
        }

        $totalProducts = Product::count();
        $totalStock = Product::sum('stock');
        $lowStockCount = Product::where('stock', '<=', 15)->count();
        $outOfStockCount = Product::where('stock', 0)->count();
        $inventoryValue = Product::selectRaw('SUM(price * stock) as total_val')->value('total_val') ?? 0;

        $categories = Category::withCount('products')->get()->map(fn($c) => [
            'id' => $c->id,
            'name' => $c->name,
            'products_count' => $c->products_count,
        ])->toArray();

        $report = [
            'type' => $this->reportType,
            'generated_at' => now()->toIso8601String(),
            'total_products' => $totalProducts,
            'total_stock_units' => (int) $totalStock,
            'total_valuation' => (float) $inventoryValue,
            'low_stock_count' => $lowStockCount,
            'out_of_stock_count' => $outOfStockCount,
            'categories' => $categories,
        ];

        // Cache the report for 1 hour
        Cache::put('inventory_report_latest', $report, 3600);

        ActivityLog::create([
            'subject_type' => 'App\\Models\\Product',
            'subject_id' => 0,
            'action' => 'inventory_report_compiled',
            'description' => "Inventory valuation report ({$this->reportType}) compiled: \${$inventoryValue} across {$totalProducts} SKUs",
            'properties' => $report,
        ]);
    }
}
