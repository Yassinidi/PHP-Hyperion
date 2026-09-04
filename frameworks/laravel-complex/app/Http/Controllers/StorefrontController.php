<?php

namespace App\Http\Controllers;

use App\Models\Category;
use App\Models\Product;
use App\Services\AnalyticsService;
use Illuminate\Contracts\View\View;
use Illuminate\Http\Request;

class StorefrontController extends Controller
{
    public function __construct(
        protected AnalyticsService $analyticsService
    ) {}

    public function index(Request $request): View
    {
        $categories = Category::withCount('products')->whereNull('parent_id')->get();

        $query = Product::with(['category', 'tags'])
            ->withCount('reviews')
            ->active();

        if ($request->filled('category')) {
            $query->whereHas('category', fn($q) => $q->where('slug', $request->query('category')));
        }

        if ($request->filled('search')) {
            $search = '%' . $request->query('search') . '%';
            $query->where('name', 'like', $search);
        }

        $products = $query->latest()->paginate(8);
        $analytics = $this->analyticsService->getDashboardMetrics();

        return view('store.index', compact('categories', 'products', 'analytics'));
    }

    public function show(int $id): View
    {
        $product = Product::with(['category', 'tags', 'reviews.customer'])
            ->findOrFail($id);

        $relatedProducts = Product::where('category_id', $product->category_id)
            ->where('id', '!=', $product->id)
            ->limit(4)
            ->get();

        return view('store.show', compact('product', 'relatedProducts'));
    }

    public function orders(): View
    {
        $orders = \App\Models\Order::with(['customer', 'items.product', 'activities'])
            ->latest()
            ->paginate(8);

        return view('store.orders', compact('orders'));
    }

    public function operations(): View
    {
        $pendingJobs = \Illuminate\Support\Facades\DB::table('jobs')->count();
        $failedJobs = \Illuminate\Support\Facades\DB::table('failed_jobs')->count();
        $recentFailed = \Illuminate\Support\Facades\DB::table('failed_jobs')->latest()->take(5)->get();
        $batches = \Illuminate\Support\Facades\DB::table('job_batches')->latest()->take(5)->get();
        $notifications = \Illuminate\Support\Facades\DB::table('notifications')->latest()->take(5)->get();
        $cachedReport = \Illuminate\Support\Facades\Cache::get('inventory_report_latest');
        $recentActivities = \App\Models\ActivityLog::latest()->take(10)->get();

        return view('store.operations', compact(
            'pendingJobs', 'failedJobs', 'recentFailed', 'batches', 'notifications', 'cachedReport', 'recentActivities'
        ));
    }
}
