<?php
declare(strict_types=1);

namespace Tests\Feature;

use Tests\TestCase;
use App\Models\Category;
use App\Models\Product;
use Illuminate\Foundation\Testing\DatabaseTransactions;

class ProductCrudApiTest extends TestCase
{
    use DatabaseTransactions;

    public function test_can_list_categories_and_products_via_api(): void
    {
        $category = Category::create([
            'name' => 'Edge Computing',
            'slug' => 'edge-computing',
            'icon' => '⚡',
            'color' => '#10b981',
        ]);

        Product::create([
            'category_id' => $category->id,
            'name' => 'Edge Router Node',
            'slug' => 'edge-router-node',
            'sku' => 'EDGE-001',
            'price' => 199.99,
            'stock' => 25,
            'status' => 'in_stock',
        ]);

        // GET /api/categories
        $catResponse = $this->getJson('/api/categories');
        $catResponse->assertStatus(200)
                    ->assertJsonPath('status', 'success');

        // GET /api/products
        $prodResponse = $this->getJson('/api/products');
        $prodResponse->assertStatus(200)
                     ->assertJsonPath('status', 'success')
                     ->assertJsonStructure([
                         'status',
                         'stats' => ['total_products', 'total_stock', 'in_stock_count'],
                         'categories',
                         'products',
                     ]);
    }

    public function test_can_create_category_via_api(): void
    {
        $payload = [
            'name' => 'Quantum Accelerators',
            'icon' => '⚛️',
            'color' => '#8b5cf6',
            'description' => 'Superconducting quantum logic modules',
        ];

        $response = $this->postJson('/api/categories', $payload);

        $response->assertStatus(201)
                 ->assertJsonPath('status', 'success')
                 ->assertJsonPath('category.name', 'Quantum Accelerators')
                 ->assertJsonPath('category.slug', 'quantum-accelerators');

        $this->assertDatabaseHas('categories', [
            'name' => 'Quantum Accelerators',
            'slug' => 'quantum-accelerators',
        ]);
    }

    public function test_can_create_product_with_validation(): void
    {
        $category = Category::create([
            'name' => 'FPGA Modules',
            'slug' => 'fpga-modules',
        ]);

        // Test invalid payload (missing required fields: name, price, stock)
        $invalidResponse = $this->postJson('/api/products', []);
        $invalidResponse->assertStatus(422)
                        ->assertJsonValidationErrors(['name', 'price', 'stock']);

        // Test valid payload
        $validPayload = [
            'category_id' => $category->id,
            'name' => 'UltraScale+ Processing Card',
            'sku' => 'FPGA-9900',
            'price' => 1299.50,
            'cost' => 450.00,
            'stock' => 5, // low_stock threshold (<= 10)
            'description' => 'PCIe Gen5 high throughput processing card',
            'featured' => true,
        ];

        $response = $this->postJson('/api/products', $validPayload);

        $response->assertStatus(201)
                 ->assertJsonPath('status', 'success')
                 ->assertJsonPath('product.name', 'UltraScale+ Processing Card')
                 ->assertJsonPath('product.sku', 'FPGA-9900')
                 ->assertJsonPath('product.status', 'low_stock');

        $this->assertDatabaseHas('products', [
            'sku' => 'FPGA-9900',
            'status' => 'low_stock',
        ]);
    }

    public function test_can_show_product_detail_via_api(): void
    {
        $category = Category::create(['name' => 'Memory', 'slug' => 'memory']);
        $product = Product::create([
            'category_id' => $category->id,
            'name' => '128GB HBM3 Memory Stack',
            'slug' => '128gb-hbm3-memory',
            'sku' => 'MEM-HBM3-128',
            'price' => 899.00,
            'stock' => 50,
        ]);

        $response = $this->getJson("/api/products/{$product->id}");

        $response->assertStatus(200)
                 ->assertJsonPath('status', 'success')
                 ->assertJsonPath('product.id', $product->id)
                 ->assertJsonPath('product.sku', 'MEM-HBM3-128')
                 ->assertJsonPath('product.category.name', 'Memory');
    }

    public function test_can_update_product_via_api(): void
    {
        $product = Product::create([
            'name' => 'Base Compute Unit',
            'slug' => 'base-compute-unit',
            'sku' => 'BCU-001',
            'price' => 49.99,
            'stock' => 20,
        ]);

        $updatePayload = [
            'price' => 59.99,
            'stock' => 0, // Should update status to out_of_stock
        ];

        $response = $this->putJson("/api/products/{$product->id}", $updatePayload);

        $response->assertStatus(200)
                 ->assertJsonPath('status', 'success')
                 ->assertJsonPath('product.status', 'out_of_stock');

        $this->assertDatabaseHas('products', [
            'id' => $product->id,
            'status' => 'out_of_stock',
        ]);
    }

    public function test_can_adjust_stock_via_api(): void
    {
        $product = Product::create([
            'name' => 'Server Chassis Unit',
            'slug' => 'server-chassis-unit',
            'sku' => 'CHA-001',
            'price' => 350.00,
            'stock' => 10,
        ]);

        // Add 15 items -> stock becomes 25 (status in_stock)
        $response = $this->postJson("/api/products/{$product->id}/adjust-stock", [
            'delta' => 15,
        ]);

        $response->assertStatus(200)
                 ->assertJsonPath('status', 'success')
                 ->assertJsonPath('product.stock', 25)
                 ->assertJsonPath('product.status', 'in_stock');
    }

    public function test_can_delete_product_via_api(): void
    {
        $product = Product::create([
            'name' => 'Legacy Controller Card',
            'slug' => 'legacy-controller-card',
            'sku' => 'LEG-001',
            'price' => 19.99,
            'stock' => 2,
        ]);

        $response = $this->deleteJson("/api/products/{$product->id}");

        $response->assertStatus(200)
                 ->assertJsonPath('status', 'success');

        $this->assertDatabaseMissing('products', [
            'id' => $product->id,
        ]);
    }
}
