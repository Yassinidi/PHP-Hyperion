<?php

namespace App\Http\Controllers;

use App\Services\CartService;
use Illuminate\Contracts\View\View;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;

class CartController extends Controller
{
    public function __construct(
        protected CartService $cartService
    ) {}

    /**
     * Dedicated cart view page
     */
    public function index(): View
    {
        $summary = $this->cartService->getCartSummary();
        return view('store.cart', compact('summary'));
    }

    /**
     * Add an item to cart (supports AJAX or standard web redirect)
     */
    public function add(Request $request): JsonResponse|RedirectResponse
    {
        $validated = $request->validate([
            'product_id' => 'required|integer|exists:products,id',
            'quantity' => 'nullable|integer|min:1|max:50',
        ]);

        $quantity = (int) ($validated['quantity'] ?? 1);
        $summary = $this->cartService->addItem($validated['product_id'], $quantity);

        if ($request->wantsJson() || $request->ajax()) {
            return response()->json([
                'success' => true,
                'message' => 'Item added to cart!',
                'summary' => $summary,
            ]);
        }

        return redirect()->back()->with('success', 'Item successfully added to cart!');
    }

    /**
     * Update quantity of an item
     */
    public function update(Request $request): JsonResponse|RedirectResponse
    {
        $validated = $request->validate([
            'product_id' => 'required|integer',
            'quantity' => 'required|integer|min:0|max:99',
        ]);

        $summary = $this->cartService->updateQuantity($validated['product_id'], $validated['quantity']);

        if ($request->wantsJson() || $request->ajax()) {
            return response()->json([
                'success' => true,
                'summary' => $summary,
            ]);
        }

        return redirect()->route('cart.index')->with('success', 'Cart updated.');
    }

    /**
     * Remove item from cart
     */
    public function remove(Request $request, int $productId): JsonResponse|RedirectResponse
    {
        $summary = $this->cartService->removeItem($productId);

        if ($request->wantsJson() || $request->ajax()) {
            return response()->json([
                'success' => true,
                'summary' => $summary,
            ]);
        }

        return redirect()->route('cart.index')->with('success', 'Item removed from cart.');
    }

    /**
     * Clear all items in cart
     */
    public function clear(): RedirectResponse
    {
        $this->cartService->clearCart();
        return redirect()->route('cart.index')->with('success', 'Cart cleared.');
    }

    /**
     * Apply discount coupon code
     */
    public function applyCoupon(Request $request): JsonResponse|RedirectResponse
    {
        $validated = $request->validate([
            'coupon_code' => 'required|string|max:50',
        ]);

        $result = $this->cartService->applyCoupon($validated['coupon_code']);

        if ($request->wantsJson() || $request->ajax()) {
            return response()->json($result);
        }

        if ($result['success']) {
            return redirect()->back()->with('success', $result['message']);
        }

        return redirect()->back()->with('error', $result['message']);
    }

    /**
     * Remove coupon code
     */
    public function removeCoupon(): RedirectResponse
    {
        $this->cartService->removeCoupon();
        return redirect()->back()->with('success', 'Coupon removed.');
    }

    /**
     * API endpoint to get live cart state for drawers & badges
     */
    public function apiSummary(): JsonResponse
    {
        return response()->json($this->cartService->getCartSummary());
    }
}
