@php
    $content = $section->content ?? [];
    $title = $section->title ?? 'Join the Hyperion Hardware Insider';
    $subtitle = $section->subtitle ?? 'Get early access to benchmark releases, new hardware drops, and secret flash coupons.';
    $incentive = $content['incentive'] ?? 'Get 10% off your first checkout when you subscribe today.';
    $btnText = $content['button_text'] ?? 'Subscribe to Insiders';
@endphp

<section class="section-newsletter" style="max-width: 1280px; margin: 0 auto 4rem auto; padding: 0 1.5rem;">
    <div style="background: var(--bg-card); border: 1px solid var(--border-color); border-radius: var(--theme-radius, 16px); padding: 3rem 2rem; text-align: center; max-width: 800px; margin: 0 auto; box-shadow: 0 10px 40px rgba(0,0,0,0.3);">
        <div style="font-size: 2.5rem; margin-bottom: 1rem;">📬</div>
        <h2 style="font-size: 1.85rem; font-weight: 800; margin-bottom: 0.5rem; color: var(--text-main);">{{ $title }}</h2>
        <p style="color: var(--text-muted); font-size: 0.95rem; margin-bottom: 1.25rem; max-width: 600px; margin-left: auto; margin-right: auto;">{{ $subtitle }}</p>
        
        <div style="background: rgba(16, 185, 129, 0.1); border: 1px solid rgba(16, 185, 129, 0.3); color: var(--accent-green); padding: 0.4rem 1rem; border-radius: 9999px; display: inline-block; font-size: 0.85rem; font-weight: 600; margin-bottom: 1.5rem;">
            ✨ {{ $incentive }}
        </div>

        <form onsubmit="event.preventDefault(); alert('Subscribed successfully! Your 10% code is PROSTORE10');" style="display: flex; gap: 0.75rem; max-width: 500px; margin: 0 auto; flex-wrap: wrap;">
            <input type="email" required placeholder="Enter your work email address..." style="flex: 1; min-width: 240px; background: rgba(0,0,0,0.3); border: 1px solid var(--border-color); border-radius: var(--theme-radius, 8px); padding: 0.85rem 1rem; color: var(--text-main); font-size: 0.95rem; outline: none;">
            <button type="submit" class="btn btn-primary" style="padding: 0.85rem 1.5rem; font-weight: 700; border-radius: var(--theme-radius, 8px);">
                {{ $btnText }}
            </button>
        </form>
    </div>
</section>
