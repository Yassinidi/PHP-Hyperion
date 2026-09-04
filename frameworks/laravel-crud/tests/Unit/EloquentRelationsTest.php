<?php
declare(strict_types=1);

namespace Tests\Unit;

use Tests\TestCase;
use App\Models\Category;
use App\Models\Product;
use Illuminate\Foundation\Testing\DatabaseTransactions;
use Illuminate\Support\Collection;

class EloquentRelationsTest extends TestCase
{
    use DatabaseTransactions;

    protected function setUp(): void
    {
        parent::setUp();
        \Illuminate\Support\Facades\DB::statement('DELETE FROM products');
        \Illuminate\Support\Facades\DB::statement('DELETE FROM categories');
    }

    public function testCategoryAndProductCreationWithRelations(): void
    {
        $category = Category::create([
            'name' => 'High-Performance Computing',
            'slug' => 'hpc',
            'icon' => 'server',
            'color' => '#3b82f6',
            'description' => 'Enterprise compute hardware and compilers',
        ]);

        $this->assertDatabaseHas('categories', [
            'id' => $category->id,
            'name' => 'High-Performance Computing',
            'slug' => 'hpc',
        ]);

        $product1 = Product::create([
            'category_id' => $category->id,
            'name' => 'Hyperion JIT Engine',
            'slug' => 'hyperion-jit',
            'sku' => 'HYP-JIT-001',
            'price' => 199.99,
            'cost' => 49.50,
            'stock' => 50,
            'status' => 'active',
            'featured' => true,
            'rating' => 4.9,
        ]);

        $product2 = Product::create([
            'category_id' => $category->id,
            'name' => 'Async Reactor Cluster',
            'slug' => 'async-reactor',
            'sku' => 'HYP-REACT-002',
            'price' => 499.00,
            'cost' => 120.00,
            'stock' => 15,
            'status' => 'active',
            'featured' => false,
            'rating' => 4.8,
        ]);

        // Test BelongsTo relation
        $this->assertNotNull($product1->category);
        $this->assertSame($category->id, $product1->category->id);
        $this->assertSame('High-Performance Computing', $product1->category->name);

        // Test HasMany relation
        $products = $category->products;
        $this->assertInstanceOf(Collection::class, $products);
        $this->assertCount(2, $products);
        $this->assertSame('Hyperion JIT Engine', $products->first()->name);
    }

    public function testEagerLoadingAndRelationCount(): void
    {
        $cat1 = Category::create(['name' => 'Compilers', 'slug' => 'compilers']);
        $cat2 = Category::create(['name' => 'Frameworks', 'slug' => 'frameworks']);

        Product::create([
            'category_id' => $cat1->id,
            'name' => 'Rust PHP VM',
            'slug' => 'rust-php-vm',
            'sku' => 'RUST-01',
            'price' => 99.0,
            'stock' => 10,
        ]);

        Product::create([
            'category_id' => $cat1->id,
            'name' => 'Bytecode Optimizer',
            'slug' => 'bytecode-opt',
            'sku' => 'OPT-02',
            'price' => 149.0,
            'stock' => 20,
        ]);

        Product::create([
            'category_id' => $cat2->id,
            'name' => 'Laravel 11 Accelerator',
            'slug' => 'laravel-accel',
            'sku' => 'LAR-01',
            'price' => 299.0,
            'stock' => 5,
        ]);

        // Test withCount
        $categoriesWithCount = Category::withCount('products')->orderBy('id')->get();
        $this->assertCount(2, $categoriesWithCount);
        $this->assertSame(2, (int) $categoriesWithCount[0]->products_count);
        $this->assertSame(1, (int) $categoriesWithCount[1]->products_count);

        // Test Eager Loading with('category')
        $eagerProducts = Product::with('category')->orderBy('id')->get();
        $this->assertCount(3, $eagerProducts);
        $this->assertTrue($eagerProducts[0]->relationLoaded('category'));
        $this->assertSame('Compilers', $eagerProducts[0]->category->name);
        $this->assertSame('Frameworks', $eagerProducts[2]->category->name);
    }

    public function testQueryBuilderAggregatesAndFiltering(): void
    {
        $category = Category::create(['name' => 'Hardware', 'slug' => 'hardware']);

        for ($i = 1; $i <= 5; $i++) {
            Product::create([
                'category_id' => $category->id,
                'name' => "Blade Server Node #{$i}",
                'slug' => "node-{$i}",
                'sku' => "NODE-00{$i}",
                'price' => 100.0 * $i, // 100, 200, 300, 400, 500
                'stock' => 10 * $i,    // 10, 20, 30, 40, 50
                'status' => $i % 2 === 0 ? 'inactive' : 'active',
                'featured' => $i > 3,
            ]);
        }

        $totalCount = Product::count();
        $this->assertSame(5, $totalCount);

        $activeCount = Product::where('status', 'active')->count();
        $this->assertSame(3, $activeCount);

        $totalStock = Product::sum('stock');
        $this->assertSame(150, (int) $totalStock); // 10+20+30+40+50 = 150

        $averagePrice = Product::avg('price');
        $this->assertEquals(300.0, (float) $averagePrice);

        $maxPrice = Product::max('price');
        $this->assertEquals(500.0, (float) $maxPrice);

        $minPrice = Product::min('price');
        $this->assertEquals(100.0, (float) $minPrice);

        $filtered = Product::whereBetween('price', [150.0, 350.0])->get();
        $this->assertCount(2, $filtered); // 200 and 300
    }

    public function testModelCastsAndDirtyTracking(): void
    {
        $category = Category::create(['name' => 'Software', 'slug' => 'software']);

        $product = new Product([
            'category_id' => $category->id,
            'name' => 'Hyperion Pro License',
            'slug' => 'hyperion-pro',
            'sku' => 'PRO-99',
            'price' => '129.50',
            'stock' => '25',
            'featured' => 1,
        ]);

        $this->assertTrue($product->isDirty());
        $this->assertArrayHasKey('name', $product->getDirty());

        $product->save();

        $this->assertFalse($product->isDirty());
        // Verify casts
        $this->assertIsFloat($product->price);
        $this->assertSame(129.50, $product->price);
        $this->assertIsInt($product->stock);
        $this->assertSame(25, $product->stock);
        $this->assertIsBool($product->featured);
        $this->assertTrue($product->featured);

        // Update attribute
        $product->price = 149.99;
        $this->assertTrue($product->isDirty('price'));
        $this->assertFalse($product->isDirty('name'));
        $product->save();

        $reloaded = Product::find($product->id);
        $this->assertSame(149.99, $reloaded->price);

        // Delete
        $id = $product->id;
        $product->delete();
        $this->assertNull(Product::find($id));
        $this->assertDatabaseMissing('products', ['id' => $id]);
    }
}
