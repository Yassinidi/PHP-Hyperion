<?php

namespace App\Jobs;

use App\Models\Product;
use Illuminate\Bus\Queueable;
use Illuminate\Contracts\Queue\ShouldQueue;
use Illuminate\Foundation\Bus\Dispatchable;
use Illuminate\Queue\InteractsWithQueue;
use Illuminate\Queue\SerializesModels;
use Illuminate\Support\Facades\Cache;

class RecalculateProductMetricsJob implements ShouldQueue
{
    use Dispatchable, InteractsWithQueue, Queueable, SerializesModels;

    public string $cacheKey;

    /**
     * Create a new job instance.
     */
    public function __construct(string $cacheKey = 'catalog_metrics')
    {
        $this->cacheKey = $cacheKey;
    }

    /**
     * Execute the job.
     */
    public function handle(): array
    {
        $metrics = [
            'total_products' => Product::count(),
            'total_stock' => (int) Product::sum('stock'),
            'average_price' => round((float) Product::avg('price'), 2),
            'featured_count' => Product::where('featured', true)->count(),
        ];

        Cache::put($this->cacheKey, $metrics, 3600);

        return $metrics;
    }
}
