<?php

use App\Http\Controllers\DashboardController;
use App\Http\Controllers\FrontController;
use App\Http\Controllers\ProfileController;
use Illuminate\Support\Facades\Route;

Route::get('/', [FrontController::class, 'index'])->name('front.index');
Route::get('/search', [FrontController::class, 'search'])->name('front.search');
Route::get('/category/{category:slug}', [FrontController::class, 'category'])->name('front.category');
Route::get('/details/{house:slug}', [FrontController::class, 'details'])->name('front.details');
Route::get('/interest/{interest}', [FrontController::class, 'interest'])->name('front.interest');
Route::post('/interest/request', [FrontController::class, 'request_interest'])->name('front.request_interest');
Route::get('/interest/success', [FrontController::class, 'request_success'])->name('front.request_success');

Route::middleware(['auth', 'role:customer'])->prefix('dashboard')->group(function () {
    Route::get('/mortgages', [DashboardController::class, 'index'])->name('dashboard.mortgages');
    Route::get('/mortgages/{mortgageRequest}', [DashboardController::class, 'details'])->name('dashboard.mortgages.details');
    Route::get('/installments/{installment}', [DashboardController::class, 'installment_details'])->name('dashboard.installments.details');
    Route::get('/mortgages/{mortgageRequest}/pay', [DashboardController::class, 'installment_payment'])->name('dashboard.installments.pay');
    Route::post('/payment/midtrans', [DashboardController::class, 'payment_store_midtrans'])->name('dashboard.payment.midtrans');
});

Route::get('/dashboard', function () {
    return redirect()->route('dashboard.mortgages');
})->middleware(['auth', 'verified'])->name('dashboard');

Route::middleware('auth')->group(function () {
    Route::get('/profile', [ProfileController::class, 'edit'])->name('profile.edit');
    Route::patch('/profile', [ProfileController::class, 'update'])->name('profile.update');
    Route::delete('/profile', [ProfileController::class, 'destroy'])->name('profile.destroy');
});

require __DIR__.'/auth.php';
