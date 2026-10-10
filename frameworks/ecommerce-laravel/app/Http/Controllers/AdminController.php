<?php

namespace App\Http\Controllers;

use Illuminate\Http\Request;
use Illuminate\Http\JsonResponse;
use App\Models\User;
use App\Models\Order;
use App\Models\Product;
use Illuminate\Support\Facades\Hash;

class AdminController
{
    public function admin()
    {
        $dashboard = $this->getDashboardData();
        $users = User::all();

        return view('admin', compact('dashboard', 'users'));
    }

    public function dashboard(): JsonResponse
    {
        return response()->json(['data' => $this->getDashboardData()]);
    }

    public function users(): JsonResponse
    {
        return response()->json(['data' => User::select('id', 'name', 'email', 'role', 'created_at')->get()]);
    }

    public function login(Request $request): JsonResponse
    {
        $email = (string)$request->input('email', '');
        $pass = (string)$request->input('password', '');
        $user = User::where('email', $email)->first();
        if ($user && Hash::check($pass, $user->password)) {
            return response()->json(['success' => true, 'user' => $user->makeHidden('password')]);
        }
        return response()->json(['success' => false, 'error' => 'Invalid email or password'], 401);
    }

    private function getDashboardData(): array
    {
        $revenue = (float)Order::where('status', 'completed')->sum('total');
        $totalOrders = (int)Order::count();
        $totalProducts = (int)Product::count();
        $totalClients = (int)User::where('role', 'client')->count();
        $lowStock = Product::where('stock', '<', 20)->orderBy('stock')->get();
        $recentOrders = Order::with('user')->orderBy('id', 'desc')->take(5)->get();

        return [
            'metrics' => [
                'total_revenue'   => round($revenue, 2),
                'total_orders'    => $totalOrders,
                'total_products'  => $totalProducts,
                'total_clients'   => $totalClients,
                'low_stock_count' => count($lowStock),
            ],
            'low_stock_items' => $lowStock,
            'recent_orders'   => $recentOrders,
        ];
    }
}
