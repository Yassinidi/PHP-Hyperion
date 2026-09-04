<?php

namespace App\Http\Controllers;

use Illuminate\Contracts\View\View;
use Illuminate\Http\Response;

class WebController extends Controller
{
    /**
     * Render the main SPA Application view.
     */
    public function index(): Response|View
    {
        $viewPath = resource_path('views/app.blade.php');
        if (file_exists($viewPath)) {
            return response(file_get_contents($viewPath), 200, [
                'Content-Type' => 'text/html; charset=UTF-8',
                'Cache-Control' => 'no-cache',
            ]);
        }
        return view('app');
    }
}
