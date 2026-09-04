<?php

namespace Tests\Feature;

use Tests\TestCase;
use App\Models\Category;
use App\Models\Product;
use Illuminate\Support\Facades\Cache;
use Illuminate\Support\Facades\DB;

class CacheDriverTest extends TestCase
{
    protected function setUp(): void
    {
        parent::setUp();
        
        Cache::flush();
        DB::statement('DELETE FROM products');
        DB::statement('DELETE FROM categories');
    }

    public function test_array_cache_driver_basic_crud(): void
    {
        Cache::put('server_status', 'online', 60);
        $this->assertEquals('online', Cache::get('server_status'));
        $this->assertTrue(Cache::has('server_status'));

        Cache::put('complex_payload', [
            'engine' => 'PHP-Hyperion',
            'version' => '8.4.1',
            'features' => ['async_reactor', 'work_stealing', 'nan_box'],
        ], 120);

        $payload = Cache::get('complex_payload');
        $this->assertIsArray($payload);
        $this->assertEquals('PHP-Hyperion', $payload['engine']);
        $this->assertCount(3, $payload['features']);

        Cache::forget('server_status');
        $this->assertFalse(Cache::has('server_status'));
        $this->assertNull(Cache::get('server_status'));
    }

    public function test_cache_remember_and_remember_forever(): void
    {
        $category = Category::create([
            'name' => 'Monitors & Displays',
            'slug' => 'monitors-displays',
            'icon' => '🖥️',
            'color' => '#f59e0b',
        ]);

        $callCount = 0;
        $fetcher = function () use (&$callCount, $category) {
            $callCount++;
            return Product::create([
                'category_id' => $category->id,
                'name' => '4K UltraSharp OLED 32"',
                'slug' => '4k-ultrasharp-oled-32',
                'sku' => 'MON-4K-032',
                'price' => 1199.00,
                'cost' => 600.00,
                'stock' => 18,
                'status' => 'in_stock',
            ]);
        };

        // First call evaluates closure
        $product1 = Cache::remember('featured_monitor', 300, $fetcher);
        $this->assertEquals(1, $callCount);
        $this->assertEquals('MON-4K-032', $product1->sku);

        // Second call retrieves from cache without evaluating closure
        $product2 = Cache::remember('featured_monitor', 300, $fetcher);
        $this->assertEquals(1, $callCount);
        $this->assertEquals('MON-4K-032', $product2->sku);
    }

    public function test_cache_increment_and_decrement_counters(): void
    {
        Cache::put('api_hits', 10, 60);

        $val1 = Cache::increment('api_hits');
        $this->assertEquals(11, $val1);

        $val2 = Cache::increment('api_hits', 5);
        $this->assertEquals(16, $val2);

        $val3 = Cache::decrement('api_hits', 6);
        $this->assertEquals(10, $val3);

        $this->assertEquals(10, Cache::get('api_hits'));
    }

    public function test_cache_atomic_locks(): void
    {
        $lock = Cache::lock('inventory_reorder_lock', 10);

        $this->assertTrue($lock->get());

        // Second lock attempt on same key should fail because it is held
        $lock2 = Cache::lock('inventory_reorder_lock', 10);
        $this->assertFalse($lock2->get());

        // Release first lock
        $lock->release();

        // Now second lock can acquire
        $this->assertTrue($lock2->get());
        $lock2->release();
    }

    public function test_cache_pull_and_add(): void
    {
        Cache::put('temporary_token', 'SECRET_AUTH_XYZ', 60);
        
        $token = Cache::pull('temporary_token');
        $this->assertEquals('SECRET_AUTH_XYZ', $token);
        $this->assertNull(Cache::get('temporary_token'));

        $added = Cache::add('unique_session', 'sess_123', 60);
        $this->assertTrue($added);

        $addedAgain = Cache::add('unique_session', 'sess_456', 60);
        $this->assertFalse($addedAgain);
        $this->assertEquals('sess_123', Cache::get('unique_session'));
    }
}
