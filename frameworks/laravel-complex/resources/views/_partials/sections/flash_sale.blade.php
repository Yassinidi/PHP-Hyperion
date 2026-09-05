@php
    $content = $section->content ?? [];
    $title = $section->title ?? 'Limited-Time Velocity Sale';
    $subtitle = $section->subtitle ?? 'Save up to 40% on neural processors, OLED displays, and developer peripherals.';
    $badge = $content['discount_badge'] ?? 'SAVE 20% WITH HYPERION20';
    $btnText = $content['button_text'] ?? 'Claim Discount Now';
    $btnLink = $content['button_link'] ?? '#products';
@endphp

<section class="section-flash-sale" style="max-width: 1280px; margin: 0 auto 3.5rem auto; padding: 0 1.5rem;">
    <div style="background: linear-gradient(135deg, rgba(99, 102, 241, 0.25) 0%, rgba(16, 185, 129, 0.15) 100%), var(--bg-card); border: 1px solid var(--border-highlight); border-radius: var(--theme-radius, 16px); padding: 2.5rem 2rem; display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 1.5rem; position: relative; overflow: hidden;">
        <div style="max-width: 650px;">
            <span style="background: var(--accent); color: white; font-size: 0.75rem; font-weight: 800; padding: 0.25rem 0.75rem; border-radius: 9999px; text-transform: uppercase; letter-spacing: 0.05em; display: inline-block; margin-bottom: 0.75rem;">
                🔥 {{ $badge }}
            </span>
            <h2 style="font-size: 1.75rem; font-weight: 800; margin-bottom: 0.5rem; color: var(--text-main);">{{ $title }}</h2>
            <p style="color: var(--text-muted); font-size: 0.95rem; line-height: 1.5;">{{ $subtitle }}</p>
        </div>
        <div>
            <a href="{{ $btnLink }}" class="btn btn-primary" style="padding: 0.85rem 1.75rem; font-weight: 700; font-size: 1rem; border-radius: var(--theme-radius, 12px); box-shadow: 0 0 20px var(--accent-glow);">
                {{ $btnText }} →
            </a>
        </div>
    </div>
</section>
