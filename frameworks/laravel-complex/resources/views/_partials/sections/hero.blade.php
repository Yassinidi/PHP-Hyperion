@php
    $content = $section->content ?? [];
    $badge = $content['badge'] ?? '2026 FLAGSHIP RELEASE';
    $title = $section->title ?? 'Next-Gen Computing & Enterprise Hardware';
    $subtitle = $section->subtitle ?? 'Precision engineered developer workstations and cloud appliances powered by PHP-Hyperion.';
    $primaryText = $content['cta_primary_text'] ?? 'Explore Catalog';
    $primaryLink = $content['cta_primary_link'] ?? '#products';
    $secondaryText = $content['cta_secondary_text'] ?? 'Customizer Studio';
    $secondaryLink = $content['cta_secondary_link'] ?? '/admin/customizer';
@endphp

<section class="section-hero" style="position: relative; overflow: hidden; padding: 4.5rem 1.5rem; margin-bottom: 2.5rem; background: linear-gradient(180deg, rgba(99, 102, 241, 0.08) 0%, rgba(10, 14, 23, 0) 100%); border-bottom: 1px solid var(--border-color);">
    <div style="max-width: 1280px; margin: 0 auto; display: grid; grid-template-columns: 1fr; gap: 2.5rem; align-items: center;">
        <div style="text-align: center; max-width: 860px; margin: 0 auto;">
            @if($badge)
                <div style="display: inline-flex; align-items: center; gap: 0.5rem; background: rgba(99, 102, 241, 0.15); border: 1px solid var(--accent-glow); padding: 0.35rem 1rem; border-radius: 9999px; margin-bottom: 1.5rem;">
                    <span style="display: inline-block; width: 8px; height: 8px; border-radius: 50%; background: var(--accent-green); box-shadow: 0 0 10px var(--accent-green);"></span>
                    <span style="font-size: 0.8rem; font-weight: 700; letter-spacing: 0.05em; color: var(--accent); text-transform: uppercase;">{{ $badge }}</span>
                </div>
            @endif

            <h1 style="font-size: clamp(2.2rem, 5vw, 3.8rem); font-weight: 800; line-height: 1.1; letter-spacing: -0.03em; margin-bottom: 1.25rem; color: var(--text-main);">
                {{ $title }}
            </h1>

            <p style="font-size: clamp(1.05rem, 2vw, 1.25rem); color: var(--text-muted); line-height: 1.6; margin-bottom: 2.5rem; max-width: 720px; margin-left: auto; margin-right: auto;">
                {{ $subtitle }}
            </p>

            <div style="display: flex; gap: 1rem; justify-content: center; flex-wrap: wrap;">
                @if($primaryText)
                    <a href="{{ $primaryLink }}" class="btn btn-primary" style="padding: 0.85rem 2rem; font-size: 1.05rem; font-weight: 700; border-radius: var(--theme-radius, 12px);">
                        {{ $primaryText }} →
                    </a>
                @endif
                @if($secondaryText)
                    <a href="{{ $secondaryLink }}" class="btn btn-outline" style="padding: 0.85rem 2rem; font-size: 1.05rem; font-weight: 600; border-radius: var(--theme-radius, 12px);">
                        🎨 {{ $secondaryText }}
                    </a>
                @endif
            </div>
        </div>
    </div>
</section>
