<?php

namespace App\Http\Controllers;

use App\Models\Page;
use App\Models\StoreSetting;
use App\Services\ThemeService;
use Illuminate\Contracts\View\View;

class CustomizerController extends Controller
{
    public function __construct(
        protected ThemeService $themeService
    ) {}

    /**
     * Visual split-screen Theme & Page Customizer Studio
     */
    public function index(): View
    {
        $themes = $this->themeService->getAllThemes();
        $activeTheme = $this->themeService->getActiveTheme();
        $settings = StoreSetting::getAllGrouped();

        $homePage = Page::with('sections')->where('slug', 'home')->first();
        $sections = $homePage ? $homePage->sections : collect([]);

        return view('admin.customizer', compact('themes', 'activeTheme', 'settings', 'sections'));
    }
}
