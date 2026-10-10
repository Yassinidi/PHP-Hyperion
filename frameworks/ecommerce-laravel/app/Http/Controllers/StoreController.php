<?php

namespace App\Http\Controllers;

use Illuminate\Http\Request;
use Illuminate\Http\JsonResponse;
use App\Models\Banner;
use App\Models\Category;
use App\Models\Product;
use App\Models\Order;
use App\Models\OrderItem;
use Illuminate\Support\Facades\DB;

class StoreController
{
    public function home(Request $request)
    {
        $catId = $request->query('category_id');
        $banners = Banner::where('is_active', 1)->orderBy('id')->get();
        $categories = Category::orderBy('display_order')->get();
        $query = Product::with('category')->orderBy('id');
        if ($catId) {
            $query->where('category_id', $catId);
        }
        $products = $query->get();

        return view('home', compact('banners', 'categories', 'products'));
    }

    public function banners(): JsonResponse
    {
        return response()->json(['data' => Banner::where('is_active', 1)->orderBy('id')->get()]);
    }

    public function categories(): JsonResponse
    {
        return response()->json(['data' => Category::orderBy('display_order')->get()]);
    }

    public function products(Request $request): JsonResponse
    {
        $catId = $request->query('category_id');
        $query = Product::orderBy('id');
        if ($catId) {
            $query->where('category_id', $catId);
        }
        return response()->json(['data' => $query->get()]);
    }

    public function productDetail(int $id): JsonResponse
    {
        $prod = Product::find($id);
        if ($prod) {
            return response()->json(['data' => $prod]);
        }
        return response()->json(['error' => 'Product not found'], 404);
    }

    public function checkout(Request $request): JsonResponse
    {
        $userId = (int)$request->input('user_id', 2);
        $items = $request->input('items', []);
        if (empty($items)) {
            return response()->json(['success' => false, 'error' => 'Cart is empty'], 400);
        }

        return DB::transaction(function () use ($userId, $items) {
            $total = 0.0;
            $validatedItems = [];
            foreach ($items as $item) {
                $pid = (int)($item['product_id'] ?? 0);
                $qty = max(1, (int)($item['quantity'] ?? 1));
                $p = Product::find($pid);
                if (!$p) {
                    throw new \Exception("Product ID {$pid} not found");
                }
                $price = (float)$p->price;
                $total += $price * $qty;
                $validatedItems[] = ['product_id' => $pid, 'quantity' => $qty, 'unit_price' => $price];

                $p->decrement('stock', $qty);
            }

            $order = Order::create([
                'user_id' => $userId,
                'total'   => $total,
                'status'  => 'completed',
            ]);

            foreach ($validatedItems as $vi) {
                OrderItem::create([
                    'order_id'   => $order->id,
                    'product_id' => $vi['product_id'],
                    'quantity'   => $vi['quantity'],
                    'unit_price' => $vi['unit_price'],
                ]);
            }

            return response()->json([
                'success'  => true,
                'order_id' => $order->id,
                'total'    => round($total, 2),
                'items'    => count($validatedItems),
                'status'   => 'completed',
            ], 201);
        });
    }
}
