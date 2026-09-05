<?php

use App\Http\Controllers\Api\AdvancedQueryController;
use App\Http\Controllers\Api\AnalyticsController;
use App\Http\Controllers\Api\FileManagementController;
use App\Http\Controllers\Api\OrderController;
use App\Http\Controllers\Api\ProductController;
use App\Http\Controllers\Api\QueueOperationsController;
use App\Http\Controllers\Api\ReverbBroadcastController;
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

    // File Management & Deletion Operations
    Route::get('/files', [FileManagementController::class, 'index']);
    Route::post('/files', [FileManagementController::class, 'store']);
    Route::get('/files/test/native', [FileManagementController::class, 'testNativeOperations']);
    Route::get('/files/{uuid}', [FileManagementController::class, 'show']);
    Route::post('/files/{uuid}/restore', [FileManagementController::class, 'restore']);
    Route::delete('/files/{uuid}', [FileManagementController::class, 'destroy']);
    Route::post('/files/batch-delete', [FileManagementController::class, 'batchDelete']);
    Route::post('/files/delete-directory', [FileManagementController::class, 'deleteDirectory']);

    // Laravel Reverb & Real-time WebSockets / Broadcasting
    Route::post('/broadcasting/dispatch', [ReverbBroadcastController::class, 'dispatchBroadcast']);
    Route::post('/broadcasting/auth', [ReverbBroadcastController::class, 'authorizeChannel']);

    // Complex Queries, CTEs, Window Functions, Transactions & Soft Deletes
    Route::get('/queries/window-rankings', [AdvancedQueryController::class, 'windowRankings']);
    Route::get('/queries/joinsub-performance', [AdvancedQueryController::class, 'joinSubPerformance']);
    Route::get('/queries/correlated-customers', [AdvancedQueryController::class, 'correlatedCustomerMetrics']);
    Route::get('/queries/cohort-spending', [AdvancedQueryController::class, 'cohortSpending']);
    Route::post('/queries/test-savepoints', [AdvancedQueryController::class, 'testSavepoints']);
    Route::post('/queries/nested-transaction', [AdvancedQueryController::class, 'testSavepoints']);
    Route::post('/queries/test-soft-deletes', [AdvancedQueryController::class, 'testSoftDeletes']);
    Route::get('/queries/test-modern-syntax', [AdvancedQueryController::class, 'testModernSyntax']);
    Route::get('/queries/modern-syntax', [AdvancedQueryController::class, 'testModernSyntax']);

    // Shopping Cart APIs
    Route::get('/cart', [\App\Http\Controllers\CartController::class, 'apiSummary']);
    Route::post('/cart/add', [\App\Http\Controllers\CartController::class, 'add']);
    Route::post('/cart/update', [\App\Http\Controllers\CartController::class, 'update']);
    Route::post('/cart/remove/{productId}', [\App\Http\Controllers\CartController::class, 'remove']);
    Route::post('/cart/coupon', [\App\Http\Controllers\CartController::class, 'applyCoupon']);

    // No-Code Customizer APIs
    Route::post('/customizer/theme/activate', [\App\Http\Controllers\Api\CustomizerApiController::class, 'activateTheme']);
    Route::post('/customizer/theme/styles', [\App\Http\Controllers\Api\CustomizerApiController::class, 'updateStyles']);
    Route::post('/customizer/sections/{id}', [\App\Http\Controllers\Api\CustomizerApiController::class, 'updateSection']);
    Route::post('/customizer/sections/reorder', [\App\Http\Controllers\Api\CustomizerApiController::class, 'reorderSections']);
    Route::post('/customizer/settings', [\App\Http\Controllers\Api\CustomizerApiController::class, 'updateSettings']);
    Route::post('/customizer/reset', [\App\Http\Controllers\Api\CustomizerApiController::class, 'resetTheme']);
});

