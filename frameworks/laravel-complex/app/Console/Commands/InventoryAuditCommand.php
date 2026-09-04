<?php

namespace App\Console\Commands;

use App\Models\Product;
use Illuminate\Console\Command;

class InventoryAuditCommand extends Command
{
    protected $signature = 'store:audit-inventory {--min-stock=15 : Minimum threshold to flag low stock}';

    protected $description = 'Audit inventory levels and flag products requiring warehouse replenishment';

    public function handle(): int
    {
        $threshold = (int) $this->option('min-stock');
        $this->info("⚡ Running Hyperion Inventory Audit (Threshold <= {$threshold} units)...");

        $lowStockProducts = Product::with('category')
            ->where('stock', '<=', $threshold)
            ->orderBy('stock', 'asc')
            ->get();

        if ($lowStockProducts->isEmpty()) {
            $this->info('✓ All products have healthy inventory levels.');
            return self::SUCCESS;
        }

        $headers = ['SKU', 'Product Name', 'Category', 'Price', 'Stock', 'Status'];
        $rows = $lowStockProducts->map(fn($p) => [
            $p->sku,
            $p->name,
            $p->category->name,
            '$' . number_format($p->price, 2),
            $p->stock,
            $p->stock === 0 ? 'OUT OF STOCK' : 'LOW STOCK',
        ])->toArray();

        $this->table($headers, $rows);
        $this->warn("⚠ Found {$lowStockProducts->count()} products requiring replenishment.");

        return self::SUCCESS;
    }
}
