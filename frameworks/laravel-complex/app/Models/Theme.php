<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Builder;
use Illuminate\Database\Eloquent\Factories\HasFactory;
use Illuminate\Database\Eloquent\Model;

class Theme extends Model
{
    use HasFactory;

    protected $fillable = [
        'name',
        'slug',
        'description',
        'is_active',
        'primary_color',
        'accent_color',
        'bg_base',
        'bg_card',
        'bg_card_hover',
        'text_main',
        'text_muted',
        'border_color',
        'border_highlight',
        'border_radius',
        'font_family',
        'custom_css',
    ];

    protected $casts = [
        'is_active' => 'boolean',
    ];

    public function scopeActive(Builder $query): Builder
    {
        return $query->where('is_active', true);
    }

    /**
     * Generate root CSS variables for dynamic frontend styling
     */
    public function toCssVariables(): string
    {
        $primary = $this->primary_color ?: '#6366f1';
        $accent = $this->accent_color ?: '#10b981';
        $bgBase = $this->bg_base ?: '#0a0e17';
        $bgCard = $this->bg_card ?: '#131b2e';
        $bgCardHover = $this->bg_card_hover ?: '#1b2640';
        $textMain = $this->text_main ?: '#f8fafc';
        $textMuted = $this->text_muted ?: '#94a3b8';
        $borderColor = $this->border_color ?: 'rgba(255, 255, 255, 0.08)';
        $borderHighlight = $this->border_highlight ?: 'rgba(99, 102, 241, 0.4)';
        $borderRadius = $this->border_radius ?: '12px';
        $fontFamily = $this->font_family ?: "'Plus Jakarta Sans', sans-serif";

        return <<<CSS
            --bg-base: {$bgBase};
            --bg-card: {$bgCard};
            --bg-card-hover: {$bgCardHover};
            --accent: {$primary};
            --accent-glow: {$borderHighlight};
            --accent-green: {$accent};
            --text-main: {$textMain};
            --text-muted: {$textMuted};
            --border-color: {$borderColor};
            --border-highlight: {$borderHighlight};
            --theme-radius: {$borderRadius};
            --theme-font: {$fontFamily};
        CSS;
    }
}
