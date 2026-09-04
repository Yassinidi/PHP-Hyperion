@extends('layouts.master')
@section('title', 'Success Request Mortgage - Experimental Project')
@section('content')

    <x-nav-tedja />
    <div class="flex justify-center gap-[50px] mt-[202px] mb-[70px]">
        <div class="flex flex-col h-fit rounded-[30px] ring-1 ring-tedja-border p-[10px] pb-5 gap-3 bg-white">
            <div class="thumbnail-container relative w-full h-[240px] rounded-[30px] overflow-hidden">
                <img src="{{ Storage::url($interest->house->thumbnail) }}"class="w-full h-full object-cover" alt="thumbnail">
            </div>
            <div class="flex flex-col gap-[18px] px-[10px]">
                <div class="flex flex-col gap-[6px]">
                    <h3 class="font-bold text-lg">{{ $interest->house->name }}</h3>
                    <div class="flex items-center gap-[6px]">
                        <img src="{{ asset('assets/images/icons/location.svg') }}" class="size-5 flex shrink-0"
                            alt="icon">
                        <p class="font-semibold text-sm">{{ $interest->house->category->name }},
                            {{ $interest->house->city->name }}</p>
                    </div>
                </div>
                <hr class="border-tedja-border">
                <div class="grid grid-cols-2 gap-y-[18px] gap-x-3">
                    <div class="flex items-center rounded-[14px] border border-tedja-border p-[10px] gap-[6px]">
                        <img src="{{ asset('assets/images/icons/slider-vertical.svg') }}" class="size-5 flex shrink-0"
                            alt="icon">
                        <p class="font-semibold text-sm">{{ $interest->house->bedroom }} Bedroom</p>
                    </div>
                    <div class="flex items-center rounded-[14px] border border-tedja-border p-[10px] gap-[6px]">
                        <img src="{{ asset('assets/images/icons/slider-horizontal.svg') }}" class="size-5 flex shrink-0"
                            alt="icon">
                        <p class="font-semibold text-sm">{{ $interest->house->bathroom }} Bathroom</p>
                    </div>
                    <div class="flex items-center rounded-[14px] border border-tedja-border p-[10px] gap-[6px]">
                        <img src="{{ asset('assets/images/icons/building-3.svg') }}" class="size-5 flex shrink-0"
                            alt="icon">
                        <p class="font-semibold text-sm">{{ $interest->house->building_area }} M²</p>
                    </div>
                    <div class="flex items-center rounded-[14px] border border-tedja-border p-[10px] gap-[6px]">
                        <img src="{{ asset('assets/images/icons/maximize-3.svg') }}" class="size-5 flex shrink-0"
                            alt="icon">
                        <p class="font-semibold text-sm">{{ $interest->house->land_area }} M²</p>
                    </div>
                    <div class="flex items-center rounded-[14px] border border-tedja-border p-[10px] gap-[6px]">
                        <img src="{{ asset('assets/images/icons/note-favorite.svg') }}" class="size-5 flex shrink-0"
                            alt="icon">
                        <p class="font-semibold text-sm">{{ $interest->house->certificate }}</p>
                    </div>
                    <div class="flex items-center rounded-[14px] border border-tedja-border p-[10px] gap-[6px]">
                        <img src="{{ asset('assets/images/icons/flash.svg') }}" class="size-5 flex shrink-0"
                            alt="icon">
                        <p class="font-semibold text-sm">{{ $interest->house->electric }} Watts</p>
                    </div>
                </div>
            </div>
        </div>
        <div class="flex flex-col w-[499px] shrink-0 gap-[33px] justify-center items-center">
            <img src="{{ asset('assets/images/icons/crown-green-fill.svg') }}" class="size-20 flex shrink-0"
                alt="icon">
            <div class="flex flex-col gap-4">
                <h1 class="font-bold text-2xl leading-9 text-center">It’s Done, Yay, Good Job! <br>Now is Waiting for Bank
                    Approval</h1>
                <p>We will help you submit your application to the relevant bank to ensure the results meet your
                    expectations</p>
            </div>
            <div class="flex items-center justify-center gap-3">
                <a href="{{ route('dashboard') }}"
                    class="rounded-full py-[14px] px-5 w-[194px] text-center font-semibold text-white bg-tedja-black">
                    My Mortgages
                </a>
                <a href="{{ route('front.index') }}"
                    class="rounded-full py-[14px] px-5 w-[194px] text-center font-semibold bg-tedja-green">
                    Explore Houses
                </a>
            </div>
        </div>
    </div>

@endsection
