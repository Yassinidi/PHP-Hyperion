<?php

use App\Http\Controllers\Api\AnalyticsController;
use App\Http\Controllers\Api\OrderController;
use App\Http\Controllers\Api\ProductController;
use App\Http\Controllers\Api\QueueOperationsController;
use Illuminate\Support\Facades\Route;

Route::prefix('v1')->group(function () {
    Route::get('/products', [ProductController::class, 'index']);
    Route::get('/products/{id}', [ProductController::class, 'show']);
    Route::post('/orders', [OrderController::class, 'store']);
    Route::get('/orders/{id}', [OrderController::class, 'show']);
    Route::post('/orders/{id}/ship', [QueueOperationsController::class, 'shipOrder']);
    Route::get('/analytics/dashboard', [AnalyticsController::class, 'dashboard']);

    // Queue, Jobs & Batching Operations
    Route::get('/queue/stats', [QueueOperationsController::class, 'stats']);
    Route::post('/queue/dispatch-job', [QueueOperationsController::class, 'dispatchJob']);
    Route::post('/queue/work-once', [QueueOperationsController::class, 'workOnce']);
    Route::post('/queue/retry-failed', [QueueOperationsController::class, 'retryFailed']);
});
