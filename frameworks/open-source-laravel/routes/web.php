<?php

use Illuminate\Support\Facades\Route;
use App\Livewire\Pages\ListProducts;
use Illuminate\Support\Facades\Log;

Route::view('/', 'welcome');

Route::get('debug', function(){

    Log::channel('db')->info('Your message');
});

Route::view('dashboard', 'dashboard')
    ->middleware(['auth', 'verified'])
    ->name('dashboard');

Route::view('profile', 'profile')
    ->middleware(['auth'])
    ->name('profile');



Route::get('/products', ListProducts::class);

require __DIR__.'/auth.php';
