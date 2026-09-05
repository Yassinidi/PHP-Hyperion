<?php

use App\Http\Controllers\StorefrontController;
use Illuminate\Support\Facades\Route;

Route::get('/', [StorefrontController::class, 'index'])->name('store.index');
Route::get('/orders', [StorefrontController::class, 'orders'])->name('store.orders');
Route::get('/operations', [StorefrontController::class, 'operations'])->name('store.operations');
Route::get('/complex', [StorefrontController::class, 'complex'])->name('store.complex');
Route::get('/products/{id}', [StorefrontController::class, 'show'])->name('store.show');


Route::get('/health', function () {
    return response()->json([
        'status' => 'healthy',
        'timestamp' => now()->toIso8601String(),
        'database' => \Illuminate\Support\Facades\DB::connection()->getPdo() ? 'connected' : 'error',
    ]);
});