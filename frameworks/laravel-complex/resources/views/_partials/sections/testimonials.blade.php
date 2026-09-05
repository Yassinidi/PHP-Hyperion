@php
    $content = $section->content ?? [];
    $title = $section->title ?? 'Trusted by 10,000+ Engineers Worldwide';
    $subtitle = $section->subtitle ?? 'Hear from engineering leads running mission-critical workloads on Hyperion.';
    $testimonials = $content['testimonials'] ?? [];
@endphp

<section class="section-testimonials" style="max-width: 1280px; margin: 0 auto 3.5rem auto; padding: 0 1.5rem;">
    <div style="text-align: center; margin-bottom: 2.5rem;">
        <h2 style="font-size: 2rem; font-weight: 800; letter-spacing: -0.02em; margin-bottom: 0.5rem; color: var(--text-main);">{{ $title }}</h2>
        <p style="color: var(--text-muted); font-size: 1rem;">{{ $subtitle }}</p>
    </div>

    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 1.5rem;">
        @foreach($testimonials as $t)
            <div style="background: var(--bg-card); border: 1px solid var(--border-color); border-radius: var(--theme-radius, 14px); padding: 1.75rem; display: flex; flex-direction: column; justify-content: space-between; box-shadow: 0 8px 30px rgba(0,0,0,0.2);">
                <div>
                    <div style="color: #f59e0b; margin-bottom: 1rem; font-size: 1.1rem;">
                        @for($i = 0; $i < ($t['rating'] ?? 5); $i++) ★ @endfor
                    </div>
                    <p style="font-size: 0.95rem; line-height: 1.6; color: var(--text-main); font-style: italic; margin-bottom: 1.5rem;">
                        “{{ $t['quote'] ?? '' }}”
                    </p>
                </div>
                <div style="display: flex; align-items: center; gap: 0.75rem; border-top: 1px solid var(--border-color); padding-top: 1rem;">
                    <div style="width: 40px; height: 40px; border-radius: 50%; background: var(--accent); color: white; display: flex; align-items: center; justify-content: center; font-weight: 700;">
                        {{ substr($t['author'] ?? 'U', 0, 1) }}
                    </div>
                    <div>
                        <div style="font-weight: 700; font-size: 0.9rem; color: var(--text-main);">{{ $t['author'] ?? '' }}</div>
                        <div style="font-size: 0.8rem; color: var(--text-muted);">{{ $t['role'] ?? '' }}</div>
                    </div>
                </div>
            </div>
        @endforeach
    </div>
</section>
