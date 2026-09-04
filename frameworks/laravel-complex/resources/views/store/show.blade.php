@extends('layouts.store')

@section('title', $product->name . ' - Hyperion Enterprise Store')

@section('styles')
<style>
    .breadcrumb {
        margin-bottom: 2rem;
        font-size: 0.85rem;
        color: var(--text-muted);
    }
    .breadcrumb a {
        color: var(--text-muted);
        text-decoration: none;
        transition: color 0.2s;
    }
    .breadcrumb a:hover {
        color: var(--text-main);
    }
    .product-detail-layout {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 3rem;
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: 16px;
        padding: 2.5rem;
        margin-bottom: 3rem;
    }
    @media (max-width: 768px) {
        .product-detail-layout {
            grid-template-columns: 1fr;
        }
    }
    .preview-box {
        background: rgba(255, 255, 255, 0.02);
        border: 1px dashed var(--border-color);
        border-radius: 12px;
        height: 360px;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        gap: 1rem;
    }
    .preview-icon {
        font-size: 5rem;
        background: linear-gradient(135deg, #6366f1, #a855f7);
        -webkit-background-clip: text;
        -webkit-text-fill-color: transparent;
    }
    .detail-info {
        display: flex;
        flex-direction: column;
        justify-content: space-between;
    }
    .detail-sku {
        font-size: 0.8rem;
        color: var(--text-muted);
        text-transform: uppercase;
        letter-spacing: 0.05em;
        margin-bottom: 0.5rem;
    }
    .detail-title {
        font-size: 2rem;
        font-weight: 800;
        margin-bottom: 1rem;
        line-height: 1.2;
    }
    .tags-container {
        display: flex;
        gap: 0.5rem;
        margin-bottom: 1.5rem;
    }
    .tag-badge {
        font-size: 0.75rem;
        font-weight: 600;
        background: rgba(255, 255, 255, 0.06);
        color: var(--text-muted);
        padding: 0.2rem 0.6rem;
        border-radius: 6px;
    }
    .detail-desc {
        color: var(--text-muted);
        font-size: 0.95rem;
        line-height: 1.7;
        margin-bottom: 2rem;
    }
    .price-box {
        display: flex;
        align-items: baseline;
        gap: 1rem;
        margin-bottom: 1.5rem;
    }
    .big-price {
        font-size: 2.25rem;
        font-weight: 800;
        color: var(--accent-green);
    }
    .stock-indicator {
        font-size: 0.9rem;
        font-weight: 600;
        color: #34d399;
    }
    .reviews-section {
        margin-top: 3rem;
    }
    .section-heading {
        font-size: 1.5rem;
        font-weight: 800;
        margin-bottom: 1.5rem;
    }
    .review-card {
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: 12px;
        padding: 1.25rem;
        margin-bottom: 1rem;
    }
    .review-header {
        display: flex;
        justify-content: space-between;
        margin-bottom: 0.5rem;
    }
    .reviewer-name {
        font-weight: 700;
    }
    .review-stars {
        color: #fbbf24;
        font-size: 0.9rem;
    }
    .review-comment {
        font-size: 0.9rem;
        color: var(--text-muted);
    }
</style>
@endsection

@section('content')
<div class="breadcrumb">
    <a href="{{ route('store.index') }}">Catalog</a> &rsaquo;
    <a href="{{ route('store.index', ['category' => $product->category->slug]) }}">{{ $product->category->name }}</a> &rsaquo;
    <span>{{ $product->name }}</span>
</div>

<div class="product-detail-layout">
    <div class="preview-box">
        <div class="preview-icon">⚡</div>
        <div style="font-weight: 600; color: var(--text-muted);">High-Performance Hardware</div>
    </div>

    <div class="detail-info">
        <div>
            <div class="detail-sku">SKU: {{ $product->sku }} &bull; {{ $product->category->name }}</div>
            <h1 class="detail-title">{{ $product->name }}</h1>

            <div class="tags-container">
                @foreach($product->tags as $tag)
                    <span class="tag-badge">#{{ $tag->name }}</span>
                @endforeach
            </div>

            <p class="detail-desc">{{ $product->description }}</p>
        </div>

        <div>
            <div class="price-box">
                <div class="big-price">${{ number_format($product->price, 2) }}</div>
                <div class="stock-indicator">In Stock ({{ $product->stock }} units ready to ship)</div>
            </div>

            <div style="display: flex; gap: 1rem;">
                <a href="{{ route('store.index') }}" class="btn" style="background: rgba(255, 255, 255, 0.1);">Back to Catalog</a>
            </div>
        </div>
    </div>
</div>

<section class="reviews-section">
    <h2 class="section-heading">Verified Customer Reviews ({{ $product->reviews->count() }})</h2>
    @forelse($product->reviews as $rev)
        <div class="review-card">
            <div class="review-header">
                <span class="reviewer-name">{{ $rev->customer->name ?? 'Verified Buyer' }} <span style="font-size: 0.75rem; color: #818cf8; margin-left: 0.5rem; text-transform: uppercase;">[{{ $rev->customer->tier ?? 'Member' }}]</span></span>
                <span class="review-stars">★ {{ $rev->rating }} / 5</span>
            </div>
            <p class="review-comment">"{{ $rev->comment }}"</p>
        </div>
    @empty
        <p style="color: var(--text-muted);">No reviews yet for this product.</p>
    @endforelse
</section>
@endsection
