@extends('layouts.store')

@section('title', $page->title . ' - ' . ($siteSettings['store_name'] ?? 'Hyperion Pro Store'))

@section('content')
<div style="max-width: 1280px; margin: 0 auto; padding: 2rem 1.5rem 4rem 1.5rem;">
    <div style="text-align: center; margin-bottom: 3rem;">
        <h1 style="font-size: 2.5rem; font-weight: 800; letter-spacing: -0.02em; margin-bottom: 0.75rem;">{{ $page->title }}</h1>
        @if($page->meta_description)
            <p style="color: var(--text-muted); font-size: 1.1rem; max-width: 700px; margin: 0 auto;">{{ $page->meta_description }}</p>
        @endif
    </div>

    @if($page->activeSections->isNotEmpty())
        @foreach($page->activeSections as $section)
            @if(view()->exists('_partials.sections.' . $section->section_type))
                @include('_partials.sections.' . $section->section_type, ['section' => $section])
            @endif
        @endforeach
    @else
        <div style="background: var(--bg-card); border: 1px solid var(--border-color); border-radius: var(--theme-radius, 14px); padding: 3rem; text-align: center; color: var(--text-muted);">
            <h3>Page Under Construction</h3>
            <p>Add dynamic sections via the Theme & Page Customizer.</p>
        </div>
    @endif
</div>
@endsection
