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

        $homePage = \App\Models\Page::with('activeSections')->where('slug', 'home')->first();
        $sections = $homePage ? $homePage->activeSections : collect([]);

        return view('store.index', compact('categories', 'products', 'analytics', 'sections'));
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

    public function track(string $order_number): View
    {
        $order = \App\Models\Order::with(['customer', 'items.product', 'activities'])
            ->where('order_number', $order_number)
            ->firstOrFail();

        return view('store.track', compact('order'));
    }

    public function page(string $slug): View
    {
        $page = \App\Models\Page::with('activeSections')
            ->where('slug', $slug)
            ->where('is_published', true)
            ->firstOrFail();

        return view('store.page', compact('page'));
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

    public function complex(
        \App\Services\FileManagerService $fileService,
        \App\Services\AdvancedQueryService $queryService
    ): View {
        $attachments = \App\Models\Attachment::withTrashed()->latest()->take(10)->get();
        $orders = \App\Models\Order::latest()->take(5)->get();
        $rankings = $queryService->getCategoryProductRankings();
        $salesSummary = $queryService->getProductPerformanceViaJoinSub();
        $customer = \App\Models\Customer::first();
        $modernSyntax = $customer ? $queryService->testModernPhpSyntax(2500.00, $customer) : null;

        return view('store.complex', compact(
            'attachments', 'orders', 'rankings', 'salesSummary', 'modernSyntax'
        ));
    }
}

