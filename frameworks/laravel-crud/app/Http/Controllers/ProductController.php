<?php

namespace App\Http\Controllers;

use App\Models\Category;
use App\Models\Product;
use Illuminate\Http\Request;
use Illuminate\Support\Str;

class ProductController extends Controller
{
    /**
     * Display a listing of the resource.
     */
    public function index(Request $request)
    {
        $query = Product::with('category');

        // Search by name, SKU, or description
        if ($search = $request->input('q')) {
            $query->where(function ($q) use ($search) {
                $q->where('name', 'like', "%{$search}%")
                  ->orWhere('sku', 'like', "%{$search}%")
                  ->orWhere('description', 'like', "%{$search}%");
            });
        }

        // Filter by category
        if ($categorySlug = $request->input('category')) {
            $query->whereHas('category', function ($q) use ($categorySlug) {
                $q->where('slug', $categorySlug);
            });
        }

        // Filter by stock status
        if ($status = $request->input('status')) {
            $query->where('status', $status);
        }

        // Sorting
        $sort = $request->input('sort', 'latest');
        switch ($sort) {
            case 'price_asc':
                $query->orderBy('price', 'asc');
                break;
            case 'price_desc':
                $query->orderBy('price', 'desc');
                break;
            case 'stock_asc':
                $query->orderBy('stock', 'asc');
                break;
            case 'stock_desc':
                $query->orderBy('stock', 'desc');
                break;
            case 'name_asc':
                $query->orderBy('name', 'asc');
                break;
            default:
                $query->orderBy('id', 'desc');
                break;
        }

        $products = $query->get();
        $categories = Category::withCount('products')->get();

        // Calculate Real-Time Inventory Metrics
        $allProducts = Product::all();
        $stats = [
            'total_products' => $allProducts->count(),
            'total_stock' => $allProducts->sum('stock'),
            'total_inventory_value' => $allProducts->sum(function ($p) {
                return $p->price * $p->stock;
            }),
            'low_stock_count' => $allProducts->where('status', 'low_stock')->count(),
            'out_of_stock_count' => $allProducts->where('status', 'out_of_stock')->count(),
            'in_stock_count' => $allProducts->where('status', 'in_stock')->count(),
        ];

        if ($request->wantsJson() || $request->is('api/*')) {
            return response()->json([
                'status' => 'success',
                'stats' => $stats,
                'categories' => $categories,
                'products' => $products,
            ]);
        }

        return view('products.index', compact('products', 'categories', 'stats'));
    }

    /**
     * Store a newly created resource in storage.
     */
    public function store(Request $request)
    {
        $validated = $request->validate([
            'name' => 'required|string|max:255',
            'category_id' => 'nullable|exists:categories,id',
            'sku' => 'nullable|string|max:50|unique:products,sku',
            'price' => 'required|numeric|min:0',
            'cost' => 'nullable|numeric|min:0',
            'stock' => 'required|integer|min:0',
            'description' => 'nullable|string',
            'image_url' => 'nullable|url',
            'featured' => 'nullable|boolean',
        ]);

        $validated['slug'] = Str::slug($validated['name']) . '-' . Str::random(5);
        if (empty($validated['sku'])) {
            $validated['sku'] = 'SKU-' . strtoupper(Str::random(8));
        }

        $stock = (int) $validated['stock'];
        if ($stock == 0) {
            $validated['status'] = 'out_of_stock';
        } elseif ($stock <= 10) {
            $validated['status'] = 'low_stock';
        } else {
            $validated['status'] = 'in_stock';
        }

        $product = Product::create($validated);

        if ($request->wantsJson() || $request->is('api/*')) {
            return response()->json([
                'status' => 'success',
                'message' => 'Product successfully created!',
                'product' => $product->load('category'),
            ], 201);
        }

        return redirect()->route('products.index')->with('success', "Product '{$product->name}' created successfully!");
    }

    /**
     * Display the specified resource.
     */
    public function show(Request $request, $id)
    {
        $product = Product::with('category')->findOrFail($id);

        if ($request->wantsJson() || $request->is('api/*')) {
            return response()->json([
                'status' => 'success',
                'product' => $product,
            ]);
        }

        return view('products.show', compact('product'));
    }

    /**
     * Update the specified resource in storage.
     */
    public function update(Request $request, $id)
    {
        $product = Product::findOrFail($id);

        $validated = $request->validate([
            'name' => 'sometimes|required|string|max:255',
            'category_id' => 'nullable|exists:categories,id',
            'sku' => "sometimes|required|string|max:50|unique:products,sku,{$id}",
            'price' => 'sometimes|required|numeric|min:0',
            'cost' => 'nullable|numeric|min:0',
            'stock' => 'sometimes|required|integer|min:0',
            'description' => 'nullable|string',
            'image_url' => 'nullable|url',
            'featured' => 'nullable|boolean',
        ]);

        if (isset($validated['stock'])) {
            $stock = (int) $validated['stock'];
            if ($stock == 0) {
                $validated['status'] = 'out_of_stock';
            } elseif ($stock <= 10) {
                $validated['status'] = 'low_stock';
            } else {
                $validated['status'] = 'in_stock';
            }
        }

        if (isset($validated['name']) && $validated['name'] !== $product->name) {
            $validated['slug'] = Str::slug($validated['name']);
        }

        $product->update($validated);

        if ($request->wantsJson() || $request->is('api/*')) {
            return response()->json([
                'status' => 'success',
                'message' => 'Product updated successfully!',
                'product' => $product->fresh()->load('category'),
            ]);
        }

        return redirect()->route('products.index')->with('success', "Product '{$product->name}' updated successfully!");
    }

    /**
     * Remove the specified resource from storage.
     */
    public function destroy(Request $request, $id)
    {
        $product = Product::findOrFail($id);
        $name = $product->name;
        $product->delete();

        if ($request->wantsJson() || $request->is('api/*')) {
            return response()->json([
                'status' => 'success',
                'message' => "Product '{$name}' deleted successfully!",
            ]);
        }

        return redirect()->route('products.index')->with('success', "Product '{$name}' deleted successfully!");
    }

    /**
     * Quick stock delta adjustment.
     */
    public function adjustStock(Request $request, $id)
    {
        $product = Product::findOrFail($id);
        $delta = (int) $request->input('delta', 1);

        $newStock = max(0, $product->stock + $delta);
        $product->stock = $newStock;

        if ($newStock == 0) {
            $product->status = 'out_of_stock';
        } elseif ($newStock <= 10) {
            $product->status = 'low_stock';
        } else {
            $product->status = 'in_stock';
        }

        $product->save();

        return response()->json([
            'status' => 'success',
            'product' => $product->load('category'),
        ]);
    }
}
