<?php

namespace App\Services;

use App\Models\StoreSetting;
use App\Models\Theme;
use Illuminate\Support\Collection;

class ThemeService
{
    /**
     * Get the currently active theme or a sensible fallback
     */
    public function getActiveTheme(): Theme
    {
        $theme = Theme::active()->first();

        if (!$theme) {
            $theme = Theme::first();
        }

        if (!$theme) {
            // Memory fallback if table empty
            $theme = new Theme([
                'name' => 'Hyperion Slate',
                'slug' => 'hyperion-slate',
                'primary_color' => '#6366f1',
                'accent_color' => '#10b981',
                'bg_base' => '#0a0e17',
                'bg_card' => '#131b2e',
                'bg_card_hover' => '#1b2640',
                'text_main' => '#f8fafc',
                'text_muted' => '#94a3b8',
                'border_color' => 'rgba(255, 255, 255, 0.08)',
                'border_highlight' => 'rgba(99, 102, 241, 0.4)',
                'border_radius' => '12px',
                'font_family' => "'Plus Jakarta Sans', sans-serif",
            ]);
        }

        return $theme;
    }

    /**
     * Retrieve all themes
     */
    public function getAllThemes(): Collection
    {
        return Theme::all();
    }

    /**
     * Switch the active theme preset
     */
    public function activateTheme(string $slug): Theme
    {
        Theme::query()->update(['is_active' => false]);
        $theme = Theme::where('slug', $slug)->firstOrFail();
        $theme->update(['is_active' => true]);
        return $theme;
    }

    /**
     * Update custom styling attributes on the active theme
     */
    public function updateActiveThemeCustomizations(array $data): Theme
    {
        $theme = $this->getActiveTheme();
        
        $allowed = [
            'primary_color', 'accent_color', 'bg_base', 'bg_card', 
            'bg_card_hover', 'text_main', 'text_muted', 'border_color', 
            'border_highlight', 'border_radius', 'font_family', 'custom_css'
        ];

        $payload = array_intersect_key($data, array_flip($allowed));
        $theme->update($payload);

        return $theme;
    }

    /**
     * Reset active theme to factory settings
     */
    public function resetTheme(string $slug = 'hyperion-slate'): Theme
    {
        $presets = $this->getDefaultThemePresets();
        if (isset($presets[$slug])) {
            $theme = Theme::where('slug', $slug)->first();
            if ($theme) {
                $theme->update($presets[$slug]);
            } else {
                $theme = Theme::create(array_merge($presets[$slug], ['slug' => $slug, 'is_active' => true]));
            }
            return $this->activateTheme($slug);
        }

        return $this->getActiveTheme();
    }

    /**
     * Built-in designer theme presets
     */
    public function getDefaultThemePresets(): array
    {
        return [
            'hyperion-slate' => [
                'name' => 'Hyperion Slate',
                'description' => 'Sleek luxury enterprise dark theme with electric indigo radiance and deep slate surfaces.',
                'primary_color' => '#6366f1',
                'accent_color' => '#10b981',
                'bg_base' => '#0a0e17',
                'bg_card' => '#131b2e',
                'bg_card_hover' => '#1b2640',
                'text_main' => '#f8fafc',
                'text_muted' => '#94a3b8',
                'border_color' => 'rgba(255, 255, 255, 0.08)',
                'border_highlight' => 'rgba(99, 102, 241, 0.4)',
                'border_radius' => '12px',
                'font_family' => "'Plus Jakarta Sans', sans-serif",
            ],
            'cyberpunk-neon' => [
                'name' => 'Cyberpunk Neon',
                'description' => 'High-octane OLED black aesthetic with vivid cyan, neon magenta, and futuristic tech glow.',
                'primary_color' => '#06b6d4',
                'accent_color' => '#f43f5e',
                'bg_base' => '#050508',
                'bg_card' => '#0f111a',
                'bg_card_hover' => '#181b28',
                'text_main' => '#f0fdf4',
                'text_muted' => '#64748b',
                'border_color' => 'rgba(6, 182, 212, 0.25)',
                'border_highlight' => 'rgba(244, 63, 94, 0.5)',
                'border_radius' => '6px',
                'font_family' => "'Outfit', sans-serif",
            ],
            'minimalist-luxe' => [
                'name' => 'Minimalist Luxe',
                'description' => 'Crisp Apple-inspired clean modern light theme with subtle shadows and refined typography.',
                'primary_color' => '#2563eb',
                'accent_color' => '#059669',
                'bg_base' => '#f8fafc',
                'bg_card' => '#ffffff',
                'bg_card_hover' => '#f1f5f9',
                'text_main' => '#0f172a',
                'text_muted' => '#64748b',
                'border_color' => 'rgba(0, 0, 0, 0.08)',
                'border_highlight' => 'rgba(37, 99, 235, 0.3)',
                'border_radius' => '16px',
                'font_family' => "'Inter', sans-serif",
            ],
            'emerald-horizon' => [
                'name' => 'Emerald Horizon',
                'description' => 'Deep organic obsidian dark theme accented with luminous emerald green and fresh mint.',
                'primary_color' => '#10b981',
                'accent_color' => '#38bdf8',
                'bg_base' => '#06130e',
                'bg_card' => '#0b2019',
                'bg_card_hover' => '#102e24',
                'text_main' => '#ecfdf5',
                'text_muted' => '#6ee7b7',
                'border_color' => 'rgba(16, 185, 129, 0.2)',
                'border_highlight' => 'rgba(16, 185, 129, 0.5)',
                'border_radius' => '14px',
                'font_family' => "'Plus Jakarta Sans', sans-serif",
            ],
            'sunset-amber' => [
                'name' => 'Sunset Amber',
                'description' => 'Warm volcanic dark mode infused with golden amber gradients and rose gold accents.',
                'primary_color' => '#f59e0b',
                'accent_color' => '#f43f5e',
                'bg_base' => '#120b08',
                'bg_card' => '#1e140f',
                'bg_card_hover' => '#2b1c15',
                'text_main' => '#fffbeb',
                'text_muted' => '#d97706',
                'border_color' => 'rgba(245, 158, 11, 0.2)',
                'border_highlight' => 'rgba(244, 63, 94, 0.4)',
                'border_radius' => '12px',
                'font_family' => "'Outfit', sans-serif",
            ],
        ];
    }
}
