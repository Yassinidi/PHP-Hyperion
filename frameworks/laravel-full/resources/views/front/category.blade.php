@extends('layouts.master')
@section('title', 'Search Houses - Experimental Project')
@section('content')

    <x-nav-tedja />

    <div class="mt-[164px] flex flex-col gap-[6px] text-center items-center">
        <h1 class="font-bold text-4xl leading-[54px]">Browse {{ $category->name }}</h1>
        <div class="flex items-center gap-[6px]">
            <img src="{{ asset('assets/images/icons/building-3.svg') }}" class="size-6 flex shrink-0" alt="icon">
            <p class="font-semibold">Available {{ $category->houses->count() }} Houses</p>
        </div>
    </div>
    <main class="grid grid-cols-3 w-full max-w-[1280px] px-[75px] gap-[30px] mx-auto my-[50px]">

        @forelse($category->houses as $house)
            <a href="{{ route('front.details', $house->slug) }}" class="card">
                <div
                    class="flex flex-col rounded-[30px] ring-1 ring-tedja-border p-[10px] pb-5 gap-3 bg-white hover:ring-2 hover:ring-tedja-blue transition-all duration-300">
                    <div class="thumbnail-container relative w-full h-[240px] rounded-[30px] overflow-hidden">
                        <img src="{{ Storage::url($house->thumbnail) }}" class="w-full h-full object-cover"
                            alt="{{ $house->name }}">
                        <button class="absolute top-5 right-5">
                            <img src="{{ asset('assets/images/icons/heart-white-fill.svg') }}"
                                class="size-[50px] flex shrink-0" alt="icon whishlist">
                        </button>
                    </div>
                    <div class="flex flex-col gap-[18px] px-[10px]">
                        <div class="flex flex-col gap-[6px]">
                            <h3 class="font-bold text-lg">
                                {{ $house->name }}
                            </h3>
                            <div class="flex items-center gap-[6px]">
                                <img src="{{ asset('assets/images/icons/location.svg') }}" class="size-5 flex shrink-0"
                                    alt="icon">
                                <p class="font-semibold text-sm">
                                    {{ $house->city->name }}
                                </p>
                            </div>
                        </div>
                        <hr class="border-tedja-border">
                        <div class="grid grid-cols-2 gap-y-[18px] gap-x-3">
                            <div class="flex items-center rounded-[14px] border border-tedja-border p-[10px] gap-[6px]">
                                <img src="{{ asset('assets/images/icons/slider-vertical.svg') }}"
                                    class="size-5 flex shrink-0" alt="icon">
                                <p class="font-semibold text-sm">
                                    {{ $house->bedroom }} Bedroom
                                </p>
                            </div>
                            <div class="flex items-center rounded-[14px] border border-tedja-border p-[10px] gap-[6px]">
                                <img src="{{ asset('assets/images/icons/slider-horizontal.svg') }}"
                                    class="size-5 flex shrink-0" alt="icon">
                                <p class="font-semibold text-sm">
                                    {{ $house->bathroom }} Bathroom
                                </p>
                            </div>
                            <div class="flex items-center rounded-[14px] border border-tedja-border p-[10px] gap-[6px]">
                                <img src="{{ asset('assets/images/icons/note-favorite.svg') }}"
                                    class="size-5 flex shrink-0" alt="icon">
                                <p class="font-semibold text-sm">
                                    {{ $house->certificate }}
                                </p>
                            </div>
                            <div class="flex items-center rounded-[14px] border border-tedja-border p-[10px] gap-[6px]">
                                <img src="{{ asset('assets/images/icons/maximize-3.svg') }}" class="size-5 flex shrink-0"
                                    alt="icon">
                                <p class="font-semibold text-sm">
                                    {{ $house->land_area }} M²
                                </p>
                            </div>
                        </div>
                    </div>
                </div>
            </a>
        @empty
            <div class="flex flex-col items-center justify-center py-20">
                <p class="text-center text-tedja-secondary font-semibold text-lg">No houses found in this category</p>
                <p class="text-center text-tedja-secondary text-sm mt-2">Check out other categories</p>
            </div>
        @endforelse

    </main>

@endsection
