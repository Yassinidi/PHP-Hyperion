@extends('layouts.master')
@section('title', 'House Details - Experimental Project')
@section('content')

    <x-nav-tedja />
    <div class="mt-[164px] flex flex-col gap-5 text-center items-center">
        <p class="flex items-center gap-[6px] rounded-full py-[6px] px-3 bg-white border border-tedja-border">
            <img src="{{ asset('assets/images/icons/crown.svg') }}" class="flex shrink-0 size-5" alt="icon">
            <span class="font-semibold text-sm">Top Well-Designed House by Tedja Team</span>
        </p>
        <h1 class="font-bold text-4xl leading-[54px]">{{ $houseDetails->name }}</h1>
        <div class="flex items-center justify-center gap-5">
            <div class="flex items-center gap-[6px]">
                <img src="{{ asset('assets/images/icons/location.svg') }}" class="size-6 flex shrink-0" alt="icon">
                <p class="font-semibold">{{ $houseDetails->category->name }}, {{ $houseDetails->city->name }}</p>
            </div>
            <div class="flex items-center gap-[6px]">
                <img src="{{ asset('assets/images/icons/security-user.svg') }}" class="size-6 flex shrink-0" alt="icon">
                <p class="font-semibold">Certified Developer</p>
            </div>
        </div>
    </div>
    <section id="Gallery" class="flex gap-5 w-full max-w-[1280px] h-[450px] px-[75px] mt-[50px] mx-auto">
        <button class="gallery-btn relative group flex w-full h-[450px] rounded-[30px] overflow-hidden">
            <img src="{{ Storage::url($houseDetails->thumbnail) }}" class="w-full h-full object-cover"
                alt="house thumbnail">
            <div
                class="absolute inset-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-all duration-300 bg-tedja-black/20">
                <img src="{{ asset('assets/images/icons/eye-white-fill.svg') }}" class="size-[50px]" alt="icon">
            </div>
        </button>
        <div class="grid grid-cols-2 gap-5 w-[450px] shrink-0" id="gallery-grid">

            @foreach ($houseDetails->photos->take(4) as $index => $photo)
                @if ($index === 3 && $houseDetails->photos->count() > 4)
                    <button id="see-all-photos-btn" data-photo-index="{{ $index }}"
                        class="relative group flex size-[215px] shrink-0 rounded-[22px] overflow-hidden cursor-pointer">
                        <img src="{{ Storage::url($photo->photo) }}" class="w-full h-full object-cover" alt="house details">
                        <div
                            class="absolute inset-0 bg-tedja-black/60 flex flex-col items-center justify-center group-hover:bg-tedja-black/70 transition-all">
                            <span class="text-white font-bold text-2xl">+{{ $houseDetails->photos->count() - 4 }}</span>
                            <span class="text-white font-semibold text-sm">See All Photos</span>
                        </div>
                    </button>
                @else
                    <button class="gallery-btn relative group flex size-[215px] shrink-0 rounded-[22px] overflow-hidden"
                        data-photo-index="{{ $index }}">
                        <img src="{{ Storage::url($photo->photo) }}" class="w-full h-full object-cover"
                            alt="house details">
                        <div
                            class="absolute inset-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-all duration-300 bg-tedja-black/20">
                            <img src="{{ asset('assets/images/icons/eye-white-fill.svg') }}" class="size-[50px]"
                                alt="icon">
                        </div>
                    </button>
                @endif
            @endforeach

        </div>
    </section>
    <section id="specs" class="w-full max-w-[1280px] px-[75px] mt-[30px] mx-auto">
        <div class="flex items-center justify-between rounded-[20px] border border-tedja-border py-5 px-[30px] bg-white">
            <div class="flex flex-col w-fit gap-3">
                <p class="text-sm text-tedja-secondary">Bedroom</p>
                <div class="flex items-center gap-[6px]">
                    <img src="{{ asset('assets/images/icons/slider-vertical.svg') }}" class="size-6 flex shrink-0"
                        alt="icon">
                    <p class="font-semibold">{{ $houseDetails->bedroom }} Bedroom</p>
                </div>
            </div>
            <div class="h-[60px] border border-tedja-border"></div>
            <div class="flex flex-col w-fit gap-3">
                <p class="text-sm text-tedja-secondary">Bathroom</p>
                <div class="flex items-center gap-[6px]">
                    <img src="{{ asset('assets/images/icons/slider-vertical.svg') }}" class="size-6 flex shrink-0"
                        alt="icon">
                    <p class="font-semibold">{{ $houseDetails->bathroom }} Bathroom</p>
                </div>
            </div>
            <div class="h-[60px] border border-tedja-border"></div>
            <div class="flex flex-col w-fit gap-3">
                <p class="text-sm text-tedja-secondary">Certificate</p>
                <div class="flex items-center gap-[6px]">
                    <img src="{{ asset('assets/images/icons/note-favorite.svg') }}" class="size-6 flex shrink-0"
                        alt="icon">
                    <p class="font-semibold">{{ $houseDetails->certificate }}</p>
                </div>
            </div>
            <div class="h-[60px] border border-tedja-border"></div>
            <div class="flex flex-col w-fit gap-3">
                <p class="text-sm text-tedja-secondary">Land of Area</p>
                <div class="flex items-center gap-[6px]">
                    <img src="{{ asset('assets/images/icons/maximize-3.svg') }}" class="size-6 flex shrink-0"
                        alt="icon">
                    <p class="font-semibold">{{ $houseDetails->land_area }} M²</p>
                </div>
            </div>
            <div class="h-[60px] border border-tedja-border"></div>
            <div class="flex flex-col w-fit gap-3">
                <p class="text-sm text-tedja-secondary">Land of Building</p>
                <div class="flex items-center gap-[6px]">
                    <img src="{{ asset('assets/images/icons/building-3.svg') }}" class="size-6 flex shrink-0"
                        alt="icon">
                    <p class="font-semibold">{{ $houseDetails->building_area }} M²</p>
                </div>
            </div>
            <div class="h-[60px] border border-tedja-border"></div>
            <div class="flex flex-col w-fit gap-3">
                <p class="text-sm text-tedja-secondary">Electric Power</p>
                <div class="flex items-center gap-[6px]">
                    <img src="{{ asset('assets/images/icons/flash.svg') }}" class="size-6 flex shrink-0" alt="icon">
                    <p class="font-semibold">{{ $houseDetails->electric }} Watt</p>
                </div>
            </div>
        </div>
    </section>
    <section id="Details" class="w-full flex gap-[70px] max-w-[1280px] px-[75px] my-[50px] mx-auto">
        <div class="flex flex-col gap-[30px]">
            <div id="About" class="flex flex-col gap-[14px]">
                <h2 class="font-semibold text-[22px] leading-[33px]">About Project</h2>
                <p class="leading-8">{{ $houseDetails->about }}</p>
            </div>
            <div id="Nerby-Facilities" class="flex flex-col gap-[14px]">
                <h2 class="font-semibold text-[22px] leading-[33px]">Nearby Facilities</h2>
                <div class="grid grid-cols-4 gap-5">

                    @foreach ($houseDetails->facilities as $facility)
                        <div
                            class="flex flex-col min-h-[140px] rounded-[20px] border border-tedja-border p-5 gap-5 bg-white">
                            <img src="{{ Storage::url($facility->facility->photo) }}" class="size-8 flex shrink-0"
                                alt="icon">
                            <p class="font-semibold">{{ $facility->facility->name }}</p>
                        </div>
                    @endforeach

                </div>
            </div>
            <div id="Location" class="flex flex-col gap-[14px]">
                <h2 class="font-semibold text-[22px] leading-[33px]">Strategic Location</h2>
                <div class="overflow-hidden w-full h-[320px]">
                    <div id="my-map-display" class="h-full w-full max-w-[none] bg-none">
                        <iframe class="h-full w-full border-0" frameborder="0"
                            src="https://www.google.com/maps/embed/v1/place?q={{ $houseDetails->name }}&key=AIzaSyBFw0Qbyq9zTFTd-tUY6dZWTgaQzuU17R8"></iframe>
                    </div>
                </div>
            </div>
        </div>
        <div class="flex flex-col w-[400px] shrink-0 h-fit rounded-[30px] border border-tedja-border p-5 gap-5 bg-white">
            <p class="font-bold text-[38px] leading-[57px] text-center text-tedja-blue">Rp
                {{ number_format($houseDetails->price, 0, '', '.') }}</p>
            <hr class="border-x-tedja-border">
            <div class="flex flex-col gap-4">
                <div class="flex items-center gap-[6px]">
                    <img src="{{ asset('assets/images/icons/tick-circle.svg') }}" class="size-6 flex shrink-0"
                        alt="icon">
                    <p class="font-semibold">Built by reliable developers</p>
                </div>
                <div class="flex items-center gap-[6px]">
                    <img src="{{ asset('assets/images/icons/tick-circle.svg') }}" class="size-6 flex shrink-0"
                        alt="icon">
                    <p class="font-semibold">100% money back guarantee</p>
                </div>
                <div class="flex items-center gap-[6px]">
                    <img src="{{ asset('assets/images/icons/tick-circle.svg') }}" class="size-6 flex shrink-0"
                        alt="icon">
                    <p class="font-semibold">Free name transfer fee</p>
                </div>
            </div>
            <hr class="border-x-tedja-border">
            <p class="font-semibold">Available Mortgages</p>

            @forelse ($houseDetails->interests as $interest)
                <div
                    class="group flex items-center justify-between rounded-[20px] ring-1 ring-tedja-border p-4 bg-white hover:ring-2 hover:ring-tedja-blue transition-all duration-300">
                    <div class="flex items-center gap-[14px]">
                        <div class="flex items-center w-[71px] h-[50px] shrink-0 overflow-hidden">
                            <img src="{{ Storage::url($interest->bank->photo) }}" class="w-full h-full object-contain"
                                alt="bank logo">
                        </div>
                        <div>
                            <p class="font-semibold">{{ $interest->bank->name }}</p>
                            <p class="text-sm text-tedja-secondary mt-0.5">Interest {{ $interest->interest }}%</p>
                        </div>
                    </div>
                    <a href="{{ route('front.interest', $interest->id) }}"
                        class="rounded-full py-[6px] px-3 bg-tedja-green font-semibold text-sm opacity-0 group-hover:opacity-100 transition-all duration-300">
                        Calculate
                    </a>
                </div>
            @empty
                <p class="text-center text-tedja-secondary font-semibold py-4">No mortgage options available yet</p>
            @endforelse

            <hr class="border-x-tedja-border">
            <div class="flex items-center justify-center gap-[6px]">
                <img src="{{ asset('assets/images/icons/security-safe-blue-fill.svg') }}" class="size-6 flex shrink-0"
                    alt="icon">
                <p class="font-semibold">All your privacy data secured</p>
            </div>
        </div>
    </section>

    <!-- All Photos Data (hidden) -->
    <div id="photos-data"
        data-thumb="{{ Storage::url($houseDetails->thumbnail) }}"
        data-photos='@json($houseDetails->photos->pluck('photo')->map(fn($p) => Storage::url($p)))'
        data-total="{{ $houseDetails->photos->count() + 1 }}"
        style="display: none;">
    </div>

    <!-- Modal -->
    <div id="Gallery-Modal"
        style="display: none; position: fixed; inset: 0; z-index: 9999; align-items: center; justify-content: center;">
        <div id="modal-backdrop" style="position: absolute; inset: 0; background: rgba(0,0,0,0.75);"></div>

        <div id="Modal-Content"
            style="position: relative; background: white; border-radius: 16px; overflow: hidden; max-width: 900px; width: 90%; z-index: 1;">

            {{-- Close Button --}}
            <button id="closeModal"
                style="position: absolute; top: 12px; right: 12px; z-index: 10; background: rgba(255,255,255,0.9); border: none; border-radius: 50%; width: 32px; height: 32px; display: flex; align-items: center; justify-content: center; cursor: pointer;">
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#333" stroke-width="2">
                    <line x1="18" y1="6" x2="6" y2="18"></line>
                    <line x1="6" y1="6" x2="18" y2="18"></line>
                </svg>
            </button>

            {{-- Main Image Area --}}
            <div style="position: relative; background: #f0f0f0; padding: 16px;">
                <div style="display: flex; align-items: center; justify-content: center; height: 500px;">
                    <img id="modal-main-image" src="{{ Storage::url($houseDetails->thumbnail) }}"
                        style="width: 100%; height: 100%; object-fit: cover; border-radius: 8px;" alt="preview">
                </div>

                {{-- Left Arrow --}}
                <button id="prev-photo"
                    style="position: absolute; left: 24px; top: 50%; transform: translateY(-50%); width: 40px; height: 40px; background: rgba(255,255,255,0.95); border: none; border-radius: 50%; display: flex; align-items: center; justify-content: center; cursor: pointer; box-shadow: 0 2px 8px rgba(0,0,0,0.2);">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#333"
                        stroke-width="2.5">
                        <polyline points="15,18 9,12 15,6"></polyline>
                    </svg>
                </button>

                {{-- Right Arrow --}}
                <button id="next-photo"
                    style="position: absolute; right: 24px; top: 50%; transform: translateY(-50%); width: 40px; height: 40px; background: rgba(255,255,255,0.95); border: none; border-radius: 50%; display: flex; align-items: center; justify-content: center; cursor: pointer; box-shadow: 0 2px 8px rgba(0,0,0,0.2);">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#333"
                        stroke-width="2.5">
                        <polyline points="9,18 15,12 9,6"></polyline>
                    </svg>
                </button>

                {{-- Photo Counter --}}
                <div
                    style="position: absolute; bottom: 24px; left: 50%; transform: translateX(-50%); background: rgba(0,0,0,0.6); color: white; font-size: 13px; font-weight: 500; padding: 6px 16px; border-radius: 20px;">
                    <span id="modal-counter">1</span> of <span id="modal-total">6</span>
                </div>
            </div>

            {{-- Thumbnails Area --}}
            <div style="background: white; border-top: 1px solid #eee; padding: 12px;">
                <div style="display: flex; gap: 8px; justify-content: center; flex-wrap: nowrap; overflow-x: auto;">
                    {{-- Main Thumbnail --}}
                    <button class="modal-thumb" data-index="0"
                        style="width: 56px; height: 56px; min-width: 56px; border-radius: 8px; overflow: hidden; border: 2px solid #CEF27F; cursor: pointer; padding: 0; opacity: 1;">
                        <img src="{{ Storage::url($houseDetails->thumbnail) }}"
                            style="width: 100%; height: 100%; object-fit: cover;" alt="thumbnail">
                    </button>

                    {{-- Photo Thumbnails --}}
                    @foreach ($houseDetails->photos as $idx => $photo)
                        <button class="modal-thumb" data-index="{{ $idx + 1 }}"
                            style="width: 56px; height: 56px; min-width: 56px; border-radius: 8px; overflow: hidden; border: 2px solid #e0e0e0; cursor: pointer; padding: 0; opacity: 0.5;">
                            <img src="{{ Storage::url($photo->photo) }}"
                                style="width: 100%; height: 100%; object-fit: cover;" alt="photo {{ $idx + 1 }}">
                        </button>
                    @endforeach
                </div>
            </div>
        </div>
    </div>

    <style>
        .modal-thumb:hover {
            opacity: 1 !important;
            border-color: #CEF27F !important;
        }

        .modal-thumb.active {
            opacity: 1 !important;
            border-color: #CEF27F !important;
        }
    </style>

@endsection

@push('after-styles')
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/swiper@11/swiper-bundle.min.css" />
@endpush

@push('after-scripts')
    <script src="{{ asset('js/gallery.js') }}"></script>
    <script src="https://cdnjs.cloudflare.com/ajax/libs/gsap/3.12.2/gsap.min.js"></script>
@endpush
