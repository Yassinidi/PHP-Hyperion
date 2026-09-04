<?php

namespace Tests\Feature;

use Tests\TestCase;
use App\Models\Category;
use App\Models\Product;
use App\Events\ProductStockUpdated;
use App\Events\ProductPriceChanged;
use App\Listeners\RecordAuditLogListener;
use Illuminate\Support\Facades\Event;
use Illuminate\Support\Facades\Cache;
use Illuminate\Support\Facades\DB;

class EventDispatcherTest extends TestCase
{
    protected function setUp(): void
    {
        parent::setUp();
        
        Cache::forget('stock_audit_log');
        Cache::forget('price_audit_log');
        Cache::flush();
        DB::statement('DELETE FROM products');
        DB::statement('DELETE FROM categories');
    }

    public function test_event_dispatching_and_listener_execution(): void
    {
        $category = Category::create([
            'name' => 'Audio Gear',
            'slug' => 'audio-gear',
            'icon' => '🎧',
            'color' => '#ec4899',
        ]);

        $product = Product::create([
            'category_id' => $category->id,
            'name' => 'Studio Headphones Pro',
            'slug' => 'studio-headphones-pro',
            'sku' => 'AUD-STU-001',
            'price' => 350.00,
            'cost' => 150.00,
            'stock' => 100,
            'status' => 'in_stock',
        ]);

        $listener = new RecordAuditLogListener();

        // 1. Direct Listener invocation and handling
        $stockEvent = new ProductStockUpdated($product, 100, 45);
        $listener->handleStockUpdated($stockEvent);

        $stockLogs = Cache::get('stock_audit_log', []);
        $this->assertCount(1, $stockLogs);
        $this->assertEquals('stock_updated', $stockLogs[0]['type']);
        $this->assertEquals(100, $stockLogs[0]['old_stock']);
        $this->assertEquals(45, $stockLogs[0]['new_stock']);

        // 2. Direct Price Changed handling
        $priceEvent = new ProductPriceChanged($product, 350.00, 299.00);
        $listener->handlePriceChanged($priceEvent);

        $priceLogs = Cache::get('price_audit_log', []);
        $this->assertCount(1, $priceLogs);
        $this->assertEquals('price_changed', $priceLogs[0]['type']);
        $this->assertEquals(350.00, $priceLogs[0]['old_price']);
        $this->assertEquals(299.00, $priceLogs[0]['new_price']);
    }

    public function test_event_fake_assertions(): void
    {
        Event::fake([
            ProductStockUpdated::class,
            ProductPriceChanged::class,
        ]);

        $category = Category::create([
            'name' => 'Smart Home',
            'slug' => 'smart-home',
            'icon' => '🏠',
            'color' => '#10b981',
        ]);

        $product = Product::create([
            'category_id' => $category->id,
            'name' => 'Smart Zigbee Hub V3',
            'slug' => 'smart-zigbee-hub-v3',
            'sku' => 'HUB-ZGB-003',
            'price' => 89.00,
            'cost' => 35.00,
            'stock' => 25,
            'status' => 'in_stock',
        ]);

        ProductStockUpdated::dispatch($product, 25, 12);
        ProductPriceChanged::dispatch($product, 89.00, 79.00);

        Event::assertDispatched(ProductStockUpdated::class, function ($e) use ($product) {
            return $e->product->id === $product->id && $e->newStock === 12;
        });

        Event::assertDispatched(ProductPriceChanged::class, function ($e) use ($product) {
            return $e->product->id === $product->id && $e->newPrice === 79.00;
        });

        Event::assertDispatched(ProductStockUpdated::class, 1);
        Event::assertDispatched(ProductPriceChanged::class, 1);
    }
}
