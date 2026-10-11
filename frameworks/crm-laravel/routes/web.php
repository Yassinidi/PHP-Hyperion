<?php

use Illuminate\Support\Facades\Route;
use App\Http\Controllers\CrmDashboardController;
use App\Http\Controllers\LeadController;
use App\Http\Controllers\DealController;

Route::get('/', [CrmDashboardController::class, 'index']);
Route::get('/api/stats', [CrmDashboardController::class, 'stats']);

Route::get('/api/leads', [LeadController::class, 'index']);
Route::get('/api/leads/{id}', [LeadController::class, 'show'])->where('id', '[0-9]+');
Route::post('/api/leads', [LeadController::class, 'store']);

Route::get('/api/deals', [DealController::class, 'index']);
Route::post('/api/deals', [DealController::class, 'createDeal']);

Route::get('/health', function () {
    return response()->json([
        'status' => 'healthy',
        'application' => 'crm-laravel',
        'framework' => 'Laravel 11.38',
        'timestamp' => microtime(true)
    ]);
});
