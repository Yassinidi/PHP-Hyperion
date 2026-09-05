@php
    $content = $section->content ?? [];
    $features = $content['features'] ?? [
        ['icon' => '⚡', 'title' => 'Sub-Millisecond Speed', 'description' => 'Powered by Hyperion Rust VM delivering 198,000 req/s.'],
        ['icon' => '🛡️', 'title' => 'Enterprise Security', 'description' => 'Encrypted transactions with real-time audit trails.'],
        ['icon' => '🚀', 'title' => 'Worldwide Express', 'description' => 'Free 48-hour global courier dispatch over $100.'],
        ['icon' => '🎨', 'title' => 'No-Code Themes', 'description' => '1-click theme switcher and live visual page builder.'],
    ];
@endphp

<section class="section-features" style="max-width: 1280px; margin: 0 auto 3rem auto; padding: 0 1.5rem;">
    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 1.25rem;">
        @foreach($features as $feat)
            <div style="background: var(--bg-card); border: 1px solid var(--border-color); border-radius: var(--theme-radius, 12px); padding: 1.5rem; transition: transform 0.2s, border-color 0.2s; box-shadow: 0 4px 20px rgba(0,0,0,0.15);" onmouseover="this.style.borderColor='var(--accent)'" onmouseout="this.style.borderColor='var(--border-color)'">
                <div style="font-size: 2rem; margin-bottom: 0.75rem;">{{ $feat['icon'] ?? '⚡' }}</div>
                <h3 style="font-size: 1.05rem; font-weight: 700; margin-bottom: 0.4rem; color: var(--text-main);">{{ $feat['title'] ?? '' }}</h3>
                <p style="font-size: 0.85rem; color: var(--text-muted); line-height: 1.5;">{{ $feat['description'] ?? '' }}</p>
            </div>
        @endforeach
    </div>
</section>
