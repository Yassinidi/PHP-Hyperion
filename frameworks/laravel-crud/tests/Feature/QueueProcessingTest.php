<?php

namespace Tests\Feature;

use Tests\TestCase;
use App\Models\Category;
use App\Models\Product;
use App\Jobs\ProcessInventoryAuditJob;
use App\Jobs\RecalculateProductMetricsJob;
use Illuminate\Support\Facades\Queue;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Cache;

class QueueProcessingTest extends TestCase
{
    protected function setUp(): void
    {
        parent::setUp();
        
        DB::statement('DELETE FROM products');
        DB::statement('DELETE FROM categories');
        if (DB::getSchemaBuilder()->hasTable('jobs')) {
            DB::statement('DELETE FROM jobs');
        }
    }

    public function test_sync_queue_dispatches_and_executes_immediately(): void
    {
        $category = Category::create([
            'name' => 'High-Performance Servers',
            'slug' => 'high-performance-servers',
            'icon' => '🖥️',
            'color' => '#3b82f6',
        ]);

        $product = Product::create([
            'category_id' => $category->id,
            'name' => 'Hyperion Blade Server 1U',
            'slug' => 'hyperion-blade-server-1u',
            'sku' => 'SRV-HYP-01U',
            'price' => 4500.00,
            'cost' => 2200.00,
            'stock' => 50,
            'status' => 'in_stock',
        ]);

        $job = new ProcessInventoryAuditJob($product, 50, 4);
        $result = $job->handle();

        $this->assertEquals(4, $result['new_stock'] ?? $product->fresh()->stock);
        $this->assertEquals('low_stock', $result['new_status']);
        $this->assertEquals(-46, $result['stock_diff']);
        $this->assertEquals('High-Performance Servers', $result['category_name']);

        $refreshed = $product->fresh();
        $this->assertEquals(4, $refreshed->stock);
        $this->assertEquals('low_stock', $refreshed->status);
    }

    public function test_queue_fake_asserts_jobs_pushed(): void
    {
        Queue::fake();

        $category = Category::create([
            'name' => 'Test Category',
            'slug' => 'test-category',
            'icon' => '📦',
            'color' => '#10b981',
        ]);

        $product = Product::create([
            'category_id' => $category->id,
            'name' => 'Test Item',
            'slug' => 'test-item',
            'sku' => 'TST-001',
            'price' => 99.00,
            'cost' => 40.00,
            'stock' => 15,
            'status' => 'in_stock',
        ]);

        ProcessInventoryAuditJob::dispatch($product, 15, 0);
        RecalculateProductMetricsJob::dispatch('test_metrics');

        Queue::assertPushed(ProcessInventoryAuditJob::class, function ($job) use ($product) {
            return $job->product->id === $product->id && $job->newStock === 0;
        });

        Queue::assertPushed(RecalculateProductMetricsJob::class, function ($job) {
            return $job->cacheKey === 'test_metrics';
        });
    }

    public function test_database_queue_driver_stores_job_payload(): void
    {
        if (!DB::getSchemaBuilder()->hasTable('jobs')) {
            $this->markTestSkipped('Jobs table does not exist.');
        }

        $category = Category::create([
            'name' => 'Storage Arrays',
            'slug' => 'storage-arrays',
            'icon' => '💾',
            'color' => '#8b5cf6',
        ]);

        $product = Product::create([
            'category_id' => $category->id,
            'name' => 'NVMe Enterprise Storage 8TB',
            'slug' => 'nvme-enterprise-storage-8tb',
            'sku' => 'STR-NVM-008',
            'price' => 899.00,
            'cost' => 450.00,
            'stock' => 20,
            'status' => 'in_stock',
        ]);

        // Push directly to database queue
        config(['queue.default' => 'database']);
        
        $job = new ProcessInventoryAuditJob($product, 20, 0);
        app('queue')->connection('database')->push($job);

        $jobRecord = DB::table('jobs')->first();
        $this->assertNotNull($jobRecord);
        $this->assertEquals('default', $jobRecord->queue);
        $this->assertEquals(0, $jobRecord->attempts);

        $payload = json_decode($jobRecord->payload, true);
        $this->assertIsArray($payload);
        $this->assertStringContainsString('ProcessInventoryAuditJob', $payload['displayName'] ?? $payload['job']);

        // Restore default queue
        config(['queue.default' => 'sync']);
    }

    public function test_recalculate_metrics_job_computes_and_caches(): void
    {
        $category = Category::create([
            'name' => 'Computing Devices',
            'slug' => 'computing-devices',
            'icon' => '💻',
            'color' => '#ec4899',
        ]);

        Product::create([
            'category_id' => $category->id,
            'name' => 'Product Alpha',
            'slug' => 'product-alpha',
            'sku' => 'ALP-001',
            'price' => 100.00,
            'cost' => 50.00,
            'stock' => 10,
            'featured' => true,
            'status' => 'in_stock',
        ]);

        Product::create([
            'category_id' => $category->id,
            'name' => 'Product Beta',
            'slug' => 'product-beta',
            'sku' => 'BET-002',
            'price' => 300.00,
            'cost' => 150.00,
            'stock' => 20,
            'featured' => false,
            'status' => 'in_stock',
        ]);

        $job = new RecalculateProductMetricsJob('global_catalog_metrics');
        $metrics = $job->handle();

        $this->assertEquals(2, $metrics['total_products']);
        $this->assertEquals(30, $metrics['total_stock']);
        $this->assertEquals(200.00, $metrics['average_price']);
        $this->assertEquals(1, $metrics['featured_count']);

        $cached = Cache::get('global_catalog_metrics');
        $this->assertEquals($metrics, $cached);
    }
}
