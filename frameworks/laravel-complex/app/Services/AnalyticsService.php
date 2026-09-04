<?php

namespace App\Services;

use App\Models\Category;
use App\Models\Customer;
use App\Models\Order;
use App\Models\Product;
use Illuminate\Support\Facades\DB;

class AnalyticsService
{
    public function getDashboardMetrics(): array
    {
        $totalRevenue = (float) (Order::where('status', 'paid')->sum('total_amount') ?? 0.0);
        $totalOrders = Order::count();
        $totalProducts = Product::count();
        $totalCustomers = Customer::count();

        // Top 5 best selling products
        $topProducts = Product::query()
            ->select('products.id', 'products.name', 'products.price', DB::raw('COALESCE(SUM(order_items.quantity), 0) as total_sold'))
            ->leftJoin('order_items', 'products.id', '=', 'order_items.product_id')
            ->groupBy('products.id', 'products.name', 'products.price')
            ->orderByDesc('total_sold')
            ->limit(5)
            ->get();

        // Category product breakdown
        $categoryBreakdown = Category::query()
            ->withCount('products')
            ->orderByDesc('products_count')
            ->get(['id', 'name', 'slug', 'products_count']);

        // Recent orders
        $recentOrders = Order::with('customer')
            ->latest()
            ->limit(5)
            ->get();

        return [
            'metrics' => [
                'total_revenue' => round($totalRevenue, 2),
                'total_orders' => $totalOrders,
                'total_products' => $totalProducts,
                'total_customers' => $totalCustomers,
                'average_order_value' => $totalOrders > 0 ? round($totalRevenue / $totalOrders, 2) : 0.0,
            ],
            'top_products' => $topProducts,
            'category_breakdown' => $categoryBreakdown,
            'recent_orders' => $recentOrders,
        ];
    }
}
