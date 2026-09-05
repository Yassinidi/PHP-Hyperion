<?php

use App\Http\Controllers\CartController;
use App\Http\Controllers\CheckoutController;
use App\Http\Controllers\CustomizerController;
use App\Http\Controllers\StorefrontController;
use Illuminate\Support\Facades\Route;

// Storefront & Catalog
Route::get('/', [StorefrontController::class, 'index'])->name('store.index');
Route::get('/products/{id}', [StorefrontController::class, 'show'])->name('store.show');
Route::get('/orders', [StorefrontController::class, 'orders'])->name('store.orders');
Route::get('/orders/{order_number}/track', [StorefrontController::class, 'track'])->name('store.track');
Route::get('/operations', [StorefrontController::class, 'operations'])->name('store.operations');
Route::get('/complex', [StorefrontController::class, 'complex'])->name('store.complex');

// Dynamic Custom Pages
Route::get('/pages/{slug}', [StorefrontController::class, 'page'])->name('store.page');

// Shopping Cart Routes
Route::get('/cart', [CartController::class, 'index'])->name('cart.index');
Route::post('/cart/add', [CartController::class, 'add'])->name('cart.add');
Route::post('/cart/update', [CartController::class, 'update'])->name('cart.update');
Route::post('/cart/remove/{productId}', [CartController::class, 'remove'])->name('cart.remove');
Route::post('/cart/clear', [CartController::class, 'clear'])->name('cart.clear');
Route::post('/cart/coupon', [CartController::class, 'applyCoupon'])->name('cart.coupon');
Route::post('/cart/coupon/remove', [CartController::class, 'removeCoupon'])->name('cart.coupon.remove');

// Checkout Flow
Route::get('/checkout', [CheckoutController::class, 'index'])->name('checkout.index');
Route::post('/checkout', [CheckoutController::class, 'process'])->name('checkout.process');

// No-Code Visual Customizer Studio
Route::get('/admin/customizer', [CustomizerController::class, 'index'])->name('customizer.index');

// System Health Probe
Route::get('/health', function () {
    return response()->json([
        'status' => 'healthy',
        'timestamp' => now()->toIso8601String(),
        'database' => \Illuminate\Support\Facades\DB::connection()->getPdo() ? 'connected' : 'error',
    ]);
});