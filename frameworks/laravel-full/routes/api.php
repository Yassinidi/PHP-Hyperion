<?php

use App\Http\Controllers\Api\BankController;
use App\Http\Controllers\Api\CategoryController;
use App\Http\Controllers\Api\CityController;
use App\Http\Controllers\Api\CurrentUserController;
use App\Http\Controllers\Api\FacilityController;
use App\Http\Controllers\Api\HouseController;
use App\Http\Controllers\Api\InstallmentController;
use App\Http\Controllers\Api\MortgageController;
use App\Http\Controllers\Api\PaymentController;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\Route;

Route::middleware('auth:sanctum')->get('/user', [CurrentUserController::class, 'show']);

Route::get('/houses', [HouseController::class, 'index']);
Route::get('/categories', [CategoryController::class, 'index']);
Route::get('/cities', [CityController::class, 'index']);
Route::get('/banks', [BankController::class, 'index']);
Route::get('/facilities', [FacilityController::class, 'index']);

Route::post('/mortgages/calculate', [MortgageController::class, 'calculate']);

Route::middleware('auth:sanctum')->group(function () {
    Route::get('/mortgages', [MortgageController::class, 'index']);
    Route::post('/mortgages', [MortgageController::class, 'store']);
    Route::get('/mortgages/{id}', [MortgageController::class, 'show']);

    Route::get('/mortgages/{mortgageRequest}/payment-breakdown', [PaymentController::class, 'breakdown']);
    Route::post('/mortgages/{mortgageRequest}/pay', [PaymentController::class, 'pay']);

    Route::get('/mortgages/{mortgageRequest}/installments', [InstallmentController::class, 'index']);
    Route::get('/installments/{installment}', [InstallmentController::class, 'show']);
});
