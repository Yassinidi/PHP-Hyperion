<?php

use Illuminate\Support\Facades\Route;
use App\Http\Controllers\StoreController;
use App\Http\Controllers\AdminController;

Route::get('/', [StoreController::class, 'home']);
Route::get('/admin', [AdminController::class, 'admin']);

Route::get('/api/banners', [StoreController::class, 'banners']);
Route::get('/api/categories', [StoreController::class, 'categories']);
Route::get('/api/products', [StoreController::class, 'products']);
Route::get('/api/products/{id}', [StoreController::class, 'productDetail'])->where('id', '[0-9]+');
Route::post('/api/cart/checkout', [StoreController::class, 'checkout']);

Route::get('/api/admin/dashboard', [AdminController::class, 'dashboard']);
Route::get('/api/admin/users', [AdminController::class, 'users']);
Route::post('/api/auth/login', [AdminController::class, 'login']);

Route::get('/health', function () {
    return response()->json(['status' => 'ok', 'project' => 'ecommerce-laravel', 'timestamp' => microtime(true)]);
});
