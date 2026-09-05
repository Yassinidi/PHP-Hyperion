<?php

namespace App\Http\Controllers;

use App\Services\CartService;
use App\Services\CheckoutService;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;

class CheckoutController extends Controller
{
    public function __construct(
        protected CartService $cartService,
        protected CheckoutService $checkoutService
    ) {}

    /**
     * Display the pro checkout screen
     */
    public function index(): View|RedirectResponse
    {
        $summary = $this->cartService->getCartSummary();

        if ($summary['is_empty']) {
            return redirect()->route('cart.index')->with('error', 'Your shopping cart is empty. Add products to proceed.');
        }

        return view('store.checkout', compact('summary'));
    }

    /**
     * Process order submission
     */
    public function process(Request $request): RedirectResponse
    {
        $validated = $request->validate([
            'first_name' => 'required|string|max:100',
            'last_name' => 'required|string|max:100',
            'email' => 'required|email|max:150',
            'phone' => 'required|string|max:50',
            'address' => 'required|string|max:255',
            'city' => 'required|string|max:100',
            'state' => 'required|string|max:100',
            'postal_code' => 'required|string|max:30',
            'country' => 'required|string|max:100',
            'shipping_speed' => 'required|string|in:standard,express,overnight',
            'payment_method' => 'required|string|in:credit_card,paypal,apple_pay,cod',
        ]);

        try {
            $order = $this->checkoutService->processCheckout(
                customerData: $validated,
                paymentMethod: $validated['payment_method'],
                shippingSpeed: $validated['shipping_speed']
            );

            return redirect()->route('store.track', ['order_number' => $order->order_number])
                ->with('success', "Order #{$order->order_number} successfully placed!");
        } catch (\Throwable $e) {
            return redirect()->back()
                ->withInput()
                ->with('error', 'Checkout failed: ' . $e->getMessage());
        }
    }
}
