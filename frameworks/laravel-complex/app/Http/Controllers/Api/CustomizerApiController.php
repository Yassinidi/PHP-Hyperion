<?php

namespace App\Http\Controllers\Api;

use App\Http\Controllers\Controller;
use App\Models\Page;
use App\Models\PageSection;
use App\Models\StoreSetting;
use App\Models\Theme;
use App\Services\ThemeService;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;

class CustomizerApiController extends Controller
{
    public function __construct(
        protected ThemeService $themeService
    ) {}

    /**
     * Switch active theme preset with 1-click
     */
    public function activateTheme(Request $request): JsonResponse
    {
        $validated = $request->validate([
            'slug' => 'required|string|exists:themes,slug',
        ]);

        $theme = $this->themeService->activateTheme($validated['slug']);

        return response()->json([
            'success' => true,
            'message' => "Theme '{$theme->name}' is now active!",
            'theme' => $theme,
            'css_variables' => $theme->toCssVariables(),
        ]);
    }

    /**
     * Update visual theme tokens (colors, fonts, radius) without code
     */
    public function updateStyles(Request $request): JsonResponse
    {
        $validated = $request->validate([
            'primary_color' => 'nullable|string|max:30',
            'accent_color' => 'nullable|string|max:30',
            'bg_base' => 'nullable|string|max:30',
            'bg_card' => 'nullable|string|max:30',
            'bg_card_hover' => 'nullable|string|max:30',
            'text_main' => 'nullable|string|max:30',
            'text_muted' => 'nullable|string|max:30',
            'border_color' => 'nullable|string|max:60',
            'border_highlight' => 'nullable|string|max:60',
            'border_radius' => 'nullable|string|max:20',
            'font_family' => 'nullable|string|max:100',
        ]);

        $theme = $this->themeService->updateActiveThemeCustomizations($validated);

        return response()->json([
            'success' => true,
            'message' => 'Theme styles saved and published!',
            'theme' => $theme,
            'css_variables' => $theme->toCssVariables(),
        ]);
    }

    /**
     * Toggle or edit a page section
     */
    public function updateSection(Request $request, int $id): JsonResponse
    {
        $section = PageSection::findOrFail($id);

        $validated = $request->validate([
            'title' => 'nullable|string|max:255',
            'subtitle' => 'nullable|string|max:500',
            'is_active' => 'nullable|boolean',
            'content' => 'nullable|array',
        ]);

        $section->update(array_filter($validated, fn($v) => !is_null($v)));

        return response()->json([
            'success' => true,
            'message' => "Section '{$section->section_type}' updated successfully!",
            'section' => $section,
        ]);
    }

    /**
     * Reorder sections
     */
    public function reorderSections(Request $request): JsonResponse
    {
        $validated = $request->validate([
            'order' => 'required|array',
            'order.*' => 'integer|exists:page_sections,id',
        ]);

        foreach ($validated['order'] as $index => $sectionId) {
            PageSection::where('id', $sectionId)->update(['sort_order' => $index + 1]);
        }

        return response()->json([
            'success' => true,
            'message' => 'Sections reordered successfully!',
        ]);
    }

    /**
     * Update store general settings (announcement banner, name, etc.)
     */
    public function updateSettings(Request $request): JsonResponse
    {
        $validated = $request->validate([
            'store_name' => 'nullable|string|max:100',
            'store_tagline' => 'nullable|string|max:200',
            'announcement_enabled' => 'nullable|string|in:true,false',
            'announcement_text' => 'nullable|string|max:255',
            'announcement_link' => 'nullable|string|max:255',
        ]);

        foreach ($validated as $key => $value) {
            StoreSetting::set($key, (string) $value);
        }

        return response()->json([
            'success' => true,
            'message' => 'Store settings saved successfully!',
            'settings' => StoreSetting::getAllGrouped(),
        ]);
    }

    /**
     * Reset active theme to factory defaults
     */
    public function resetTheme(Request $request): JsonResponse
    {
        $slug = $request->input('slug', 'hyperion-slate');
        $theme = $this->themeService->resetTheme($slug);

        return response()->json([
            'success' => true,
            'message' => 'Theme reset to factory defaults.',
            'theme' => $theme,
            'css_variables' => $theme->toCssVariables(),
        ]);
    }
}
