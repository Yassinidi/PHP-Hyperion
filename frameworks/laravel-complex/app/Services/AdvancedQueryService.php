<?php

namespace App\Services;

use App\Models\Attachment;
use App\Models\Category;
use App\Models\Customer;
use App\Models\Order;
use App\Models\Product;
use Illuminate\Support\Collection;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Str;

class AdvancedQueryService
{
    /**
     * 1. Window Functions & Partition Ranking
     */
    public function getCategoryProductRankings(): Collection
    {
        return DB::table('products')
            ->join('categories', 'products.category_id', '=', 'categories.id')
            ->select([
                'products.id',
                'products.sku',
                'products.name as product_name',
                'categories.name as category_name',
                'products.price',
                DB::raw('ROW_NUMBER() OVER (PARTITION BY products.category_id ORDER BY products.price DESC) as rank_in_category'),
                DB::raw('ROUND(AVG(products.price) OVER (PARTITION BY products.category_id), 2) as category_avg_price'),
                DB::raw('ROUND(products.price - AVG(products.price) OVER (PARTITION BY products.category_id), 2) as diff_from_category_avg'),
            ])
            ->orderBy('categories.name')
            ->orderBy('rank_in_category')
            ->get();
    }

    /**
     * 2. Subquery Join (joinSub) with Order Statistics
     */
    public function getProductPerformanceViaJoinSub(): Collection
    {
        $orderStatsSub = DB::table('order_items')
            ->select([
                'product_id',
                DB::raw('COUNT(*) as line_count'),
                DB::raw('SUM(quantity) as total_units_sold'),
                DB::raw('ROUND(SUM(subtotal), 2) as gross_revenue'),
            ])
            ->groupBy('product_id');

        return DB::table('products')
            ->joinSub($orderStatsSub, 'order_stats', function ($join) {
                $join->on('products.id', '=', 'order_stats.product_id');
            })
            ->join('categories', 'products.category_id', '=', 'categories.id')
            ->select([
                'products.id',
                'products.sku',
                'products.name as product_name',
                'categories.name as category_name',
                'products.price',
                'order_stats.total_units_sold',
                'order_stats.gross_revenue',
            ])
            ->orderByDesc('order_stats.gross_revenue')
            ->get();
    }

    /**
     * 3. Correlated Subqueries via Eloquent addSelect
     */
    public function getCustomerMetricsCorrelated(): Collection
    {
        return Customer::query()
            ->addSelect([
                'latest_order_total' => Order::select('total_amount')
                    ->whereColumn('customer_id', 'customers.id')
                    ->latest()
                    ->limit(1),
                'order_count' => Order::selectRaw('count(*)')
                    ->whereColumn('customer_id', 'customers.id'),
                'total_spend' => Order::selectRaw('coalesce(sum(total_amount), 0)')
                    ->whereColumn('customer_id', 'customers.id'),
            ])
            ->whereHas('orders')
            ->get();
    }

    /**
     * 4. Conditional Aggregation with HAVING Clause
     */
    public function getCustomerCohortSpending(): Collection
    {
        return DB::table('orders')
            ->join('order_items', 'orders.id', '=', 'order_items.order_id')
            ->select([
                'orders.customer_id',
                DB::raw('COUNT(DISTINCT orders.id) as orders_count'),
                DB::raw("SUM(CASE WHEN orders.status = 'paid' THEN order_items.subtotal ELSE 0 END) as paid_revenue"),
                DB::raw("SUM(CASE WHEN orders.status = 'pending' THEN order_items.subtotal ELSE 0 END) as pending_revenue"),
            ])
            ->groupBy('orders.customer_id')
            ->havingRaw('COUNT(DISTINCT orders.id) >= 1')
            ->get();
    }

    /**
     * 5. Nested Transactions & Savepoint Isolation Test
     */
    public function testNestedTransactionWithSavepoint(): array
    {
        $testSku = 'TEST-SP-' . strtoupper(substr(md5(uniqid()), 0, 6));

        DB::beginTransaction();

        try {
            // Outer transaction inserts product
            $category = Category::first();
            $product = Product::create([
                'sku' => $testSku,
                'name' => 'Savepoint Test Product',
                'slug' => Str::slug($testSku),
                'price' => 199.99,
                'stock' => 50,
                'category_id' => $category->id,
                'is_active' => true,
            ]);

            $outerCreated = Product::where('sku', $testSku)->exists();

            // Inner transaction with savepoint that rolls back
            $innerRolledBack = false;
            try {
                DB::transaction(function () use ($product) {
                    $product->update(['stock' => 1000]);
                    throw new \RuntimeException('Simulated inner transaction failure to test savepoint rollback');
                });
            } catch (\RuntimeException $e) {
                $innerRolledBack = true;
            }

            // Verify outer transaction state: product exists with original stock 50!
            $reloaded = Product::where('sku', $testSku)->first();
            $stockRemainedOriginal = ($reloaded->stock === 50);

            // Clean up test product
            $product->delete();
            DB::commit();

            return [
                'outer_created' => $outerCreated,
                'inner_rolled_back' => $innerRolledBack,
                'stock_remained_original' => $stockRemainedOriginal,
                'isolation_passed' => $outerCreated && $innerRolledBack && $stockRemainedOriginal,
            ];
        } catch (\Throwable $e) {
            DB::rollBack();
            throw $e;
        }
    }

    /**
     * 6. Soft Deletes Full Lifecycle Test
     */
    public function testSoftDeleteLifecycle(): array
    {
        $uuid = (string) Str::uuid();

        // 1. Create
        $attachment = Attachment::create([
            'uuid' => $uuid,
            'filename' => 'lifecycle_test.pdf',
            'original_filename' => 'lifecycle_test.pdf',
            'disk' => 'local',
            'path' => 'tests/' . $uuid . '.pdf',
            'size_bytes' => 1024,
            'sha256_hash' => hash('sha256', 'lifecycle_test_content'),
        ]);

        $createdId = $attachment->id;

        // 2. Normal query finds it
        $existsActive = Attachment::where('id', $createdId)->exists();

        // 3. Soft delete
        $attachment->delete();
        $existsAfterSoftDelete = Attachment::where('id', $createdId)->exists(); // should be false
        $existsInTrash = Attachment::onlyTrashed()->where('id', $createdId)->exists(); // should be true
        $existsWithTrash = Attachment::withTrashed()->where('id', $createdId)->exists(); // should be true

        // 4. Restore
        $attachment->restore();
        $existsAfterRestore = Attachment::where('id', $createdId)->exists(); // should be true

        // 5. Force Delete
        $attachment->forceDelete();
        $existsAfterForceDelete = Attachment::withTrashed()->where('id', $createdId)->exists(); // should be false

        return [
            'step_create' => $existsActive,
            'step_soft_delete_hidden' => !$existsAfterSoftDelete,
            'step_only_trashed' => $existsInTrash,
            'step_with_trashed' => $existsWithTrash,
            'step_restore' => $existsAfterRestore,
            'step_force_delete' => !$existsAfterForceDelete,
            'lifecycle_passed' => $existsActive && !$existsAfterSoftDelete && $existsInTrash && $existsWithTrash && $existsAfterRestore && !$existsAfterForceDelete,
        ];
    }

    /**
     * 7. Complex Modern PHP 8.4 Syntax Execution
     */
    public function testModernPhpSyntax(float $amount, ?Customer $customer = null): array
    {
        // Match expression with multiple conditions
        $tier = match (true) {
            $amount >= 5000.0 => 'Enterprise VIP',
            $amount >= 1000.0 => 'Business Premium',
            $amount >= 250.0  => 'Preferred Client',
            default           => 'Standard Consumer',
        };

        // First-class callable syntax
        $curFormatter = fn(float $v): string => '$' . number_format($v, 2);
        $formatted = $curFormatter($amount);

        // Nullsafe operator
        $customerName = $customer?->name ?? 'Anonymous';
        $customerEmail = $customer?->email;

        // Keyed array destructuring
        $payload = ['status' => 'active', 'code' => 200, 'flag' => true];
        ['status' => $status, 'code' => $code] = $payload;

        return [
            'tier' => $tier,
            'formatted' => $formatted,
            'customer_name' => $customerName,
            'customer_email' => $customerEmail,
            'destructured_status' => $status,
            'destructured_code' => $code,
        ];
    }
}
