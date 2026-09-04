<?php

use Illuminate\Support\Facades\Route;
use App\Http\Controllers\WebController;
use App\Http\Controllers\PostController;
use App\Http\Controllers\CommentController;
use App\Http\Controllers\AuthController;
use App\Http\Controllers\AuthorController;
use App\Http\Controllers\DashboardController;

/*
|--------------------------------------------------------------------------
| Web Routes (Single Page Application Multi-Views)
|--------------------------------------------------------------------------
*/
Route::get('/', [WebController::class, 'index'])->name('home');
Route::get('/posts/{id}', [WebController::class, 'index'])->name('posts.show');
Route::get('/create', [WebController::class, 'index'])->name('posts.create');
Route::get('/authors/{id}', [WebController::class, 'index'])->name('authors.show');
Route::get('/dashboard', [WebController::class, 'index'])->name('dashboard');
Route::get('/explore', [WebController::class, 'index'])->name('explore');

/*
|--------------------------------------------------------------------------
| Public Storage Static Asset Fallback Route
|--------------------------------------------------------------------------
*/
Route::get('/storage/{folder}/{filename}', function (string $folder, string $filename) {
    $path = storage_path("app/public/{$folder}/{$filename}");
    if (file_exists($path) && !is_dir($path)) {
        $ext = strtolower(pathinfo($path, PATHINFO_EXTENSION));
        $mimes = [
            'png' => 'image/png', 'jpg' => 'image/jpeg', 'jpeg' => 'image/jpeg',
            'gif' => 'image/gif', 'svg' => 'image/svg+xml', 'webp' => 'image/webp',
            'css' => 'text/css', 'js' => 'application/javascript',
        ];
        return response(file_get_contents($path), 200, [
            'Content-Type' => $mimes[$ext] ?? 'application/octet-stream',
            'Cache-Control' => 'public, max-age=86400',
        ]);
    }
    return response()->json(['error' => 'File not found'], 404);
});

/*
|--------------------------------------------------------------------------
| API Routes (Controller-Based REST Endpoints)
|--------------------------------------------------------------------------
*/
Route::prefix('api')->group(function () {
    // Auth endpoints
    Route::get('/me', [AuthController::class, 'me']);
    Route::post('/register', [AuthController::class, 'register']);
    Route::post('/login', [AuthController::class, 'login']);
    Route::post('/logout', [AuthController::class, 'logout']);
    Route::post('/profile/photo', [AuthController::class, 'updatePhoto']);

    // Post CRUD endpoints
    Route::get('/posts', [PostController::class, 'index']);
    Route::get('/posts/{id}', [PostController::class, 'show']);
    Route::post('/posts', [PostController::class, 'store']);
    Route::put('/posts/{id}', [PostController::class, 'update']);
    Route::patch('/posts/{id}', [PostController::class, 'update']);
    Route::delete('/posts/{id}', [PostController::class, 'destroy']);
    Route::post('/posts/{id}/like', [PostController::class, 'like']);

    // Comment endpoints
    Route::post('/posts/{id}/comments', [CommentController::class, 'store']);

    // Author & Dashboard endpoints
    Route::get('/authors/{id}', [AuthorController::class, 'show']);
    Route::get('/dashboard/stats', [DashboardController::class, 'stats']);
});
