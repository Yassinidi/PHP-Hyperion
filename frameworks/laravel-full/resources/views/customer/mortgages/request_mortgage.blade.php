@extends('layouts.master')
@section('title', 'Request Mortgage - Experimental Project')
@section('content')

    <x-nav-tedja />
    <div
        class="relative w-full max-w-[1280px] px-[75px] mt-[164px] flex flex-col gap-[6px] text-center items-center mx-auto">
        <h1 class="font-bold text-4xl leading-[54px]">Request for Mortgage</h1>
        <p>We will give you best of life forever young yay</p>
        <a href="{{ route('front.details', $interest->house->slug) }}"
            class="absolute transform -translate-y-1/2 top-1/2 left-[75px] flex items-center gap-[6px] hover:text-tedja-blue transition-all duration-300">
            <img src="{{ asset('assets/images/icons/arrow-left.svg') }}" class="size-6 flex shrink-0" alt="icon">
            <span class="font-semibold">Back to Details</span>
        </a>
    </div>
    <div class="flex justify-center gap-[30px] mt-[50px] mb-[70px]">
        <div class="relative">
            <div
                class="sticky top-[144px] flex flex-col w-[357px] shrink-0 h-fit rounded-[30px] ring-1 ring-tedja-border p-[10px] pb-5 gap-3 bg-white">
                <div class="thumbnail-container relative w-full h-[240px] rounded-[30px] overflow-hidden">
                    <img src="{{ Storage::url($interest->house->thumbnail) }}" class="w-full h-full object-cover"
                        alt="thumbnail">
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
        </div>
        <div class="flex flex-col gap-5 w-[500px] shrink-0">
            <section id="Price-Details"
                class="border border-[#F2F2F4] bg-white !h-fit w-full p-5 rounded-[20px] flex flex-col gap-4">
                <div class="flex items-center justify-between">
                    <h4 class="font-semibold text-lg leading-[27px]">Price Details</h4>
                    <button type="button" class="shrink-0" data-expand="PriceDetails">
                        <img src="{{ asset('assets/images/icons/arrow-circle-down.svg') }}" alt="icon"
                            class="size-6 shrink-0 transition-all duration-300">
                    </button>
                </div>
                <div class="flex flex-col gap-4" id="PriceDetails">
                    <hr class="border-[#F2F2F4]">
                    <div class="flex flex-col gap-4">
                        <div class="flex items-center justify-between">
                            <div class="flex items-center gap-2">
                                <img src="{{ asset('assets/images/icons/note.svg') }}" alt="icon"
                                    class="size-6 shrink-0">
                                <p>House Price</p>
                            </div>
                            <strong class="font-semibold">Rp
                                {{ number_format($interest->house->price, 0, '', '.') }}</strong>
                        </div>
                        <div class="flex items-center justify-between">
                            <div class="flex items-center gap-2">
                                <img src="{{ asset('assets/images/icons/note.svg') }}" alt="icon"
                                    class="size-6 shrink-0">
                                <p>Security Fee</p>
                            </div>
                            <strong class="font-semibold">Free Included</strong>
                        </div>
                        <div class="flex items-center justify-between">
                            <div class="flex items-center gap-2">
                                <img src="{{ asset('assets/images/icons/note.svg') }}" alt="icon"
                                    class="size-6 shrink-0">
                                <p>New Registration</p>
                            </div>
                            <strong class="font-semibold">Free Included</strong>
                        </div>
                    </div>
                </div>
            </section>
            <section id="Bank-Details"
                class="border border-[#F2F2F4] bg-white !h-fit w-full p-5 rounded-[20px] flex flex-col gap-4">
                <div class="flex items-center justify-between">
                    <h4 class="font-semibold text-lg leading-[27px]">Bank KPR Details</h4>
                    <button type="button" class="shrink-0" data-expand="BankDetails">
                        <img src="{{ asset('assets/images/icons/arrow-circle-down.svg') }}" alt="icon"
                            class="size-6 shrink-0 transition-all duration-300">
                    </button>
                </div>
                <div class="flex flex-col gap-4" id="BankDetails">
                    <hr class="border-[#F2F2F4]">
                    <div class="flex items-center gap-5">
                        <div class="flex h-[70px] shrink-0 overflow-hidden">
                            <img src="{{ Storage::url($interest->bank->photo) }}" class="w-full h-full object-contain"
                                alt="bank logo">
                        </div>
                        <div class="flex flex-col gap-0.5">
                            <div class="flex items-center gap-0.5">
                                <p class="font-semibold">{{ $interest->bank->name }}</p>
                                <img src="{{ asset('assets/images/icons/verify.svg') }}" class="size-5 flex shrink-0"
                                    alt="icon">
                            </div>
                            <p class="text-sm text-tedja-secondary">Dipercaya jutaan nasabah</p>
                        </div>
                    </div>
                    <div class="flex flex-col gap-4">
                        <div class="flex items-center justify-between">
                            <div class="flex items-center gap-2">
                                <img src="{{ asset('assets/images/icons/note.svg') }}" alt="icon"
                                    class="size-6 shrink-0">
                                <p>Bank Interest Guarantee</p>
                            </div>
                            <strong class="font-semibold">{{ $interest->interest }}%</strong>
                        </div>
                        <div class="flex items-center justify-between">
                            <div class="flex items-center gap-2">
                                <img src="{{ asset('assets/images/icons/note.svg') }}" alt="icon"
                                    class="size-6 shrink-0">
                                <p>Loan Duration</p>
                            </div>
                            <strong class="font-semibold">{{ $interest->duration }} Years</strong>
                        </div>
                    </div>
                </div>
            </section>
            <section id="Choose-Amount"
                class="border border-[#F2F2F4] bg-white !h-fit w-full p-5 rounded-[20px] flex flex-col gap-4">
                <h4 class="font-semibold text-lg leading-[27px]">Choose DP Amount</h4>
                <div class="grid grid-cols-3 gap-y-4 gap-x-5" id="dp-options">
                    <a href="#" data-percentage="10"
                        class="dp-option group relative rounded-[14px] border border-tedja-border py-[14px] px-[10px] flex items-center justify-center hover:bg-tedja-black transition-all duration-300">
                        <span class="font-semibold group-hover:text-white transition-all duration-300">10%</span>
                    </a>
                    <a href="#" data-percentage="20"
                        class="dp-option group relative rounded-[14px] border border-tedja-border py-[14px] px-[10px] flex items-center justify-center hover:bg-tedja-black transition-all duration-300">
                        <span class="font-semibold group-hover:text-white transition-all duration-300">20%</span>
                    </a>
                    <a href="#" data-percentage="40"
                        class="dp-option group relative rounded-[14px] border border-tedja-border py-[14px] px-[10px] flex items-center justify-center hover:bg-tedja-black transition-all duration-300">
                        <span class="font-semibold group-hover:text-white transition-all duration-300">40%</span>
                    </a>
                    <a href="#" data-percentage="50"
                        class="dp-option group relative rounded-[14px] border border-tedja-border py-[14px] px-[10px] flex items-center justify-center hover:bg-tedja-black transition-all duration-300">
                        <span class="font-semibold group-hover:text-white transition-all duration-300">50%</span>
                    </a>
                    <a href="#" data-percentage="60"
                        class="dp-option group relative rounded-[14px] border border-tedja-border py-[14px] px-[10px] flex items-center justify-center hover:bg-tedja-black transition-all duration-300">
                        <span class="font-semibold group-hover:text-white transition-all duration-300">60%</span>
                    </a>
                    <a href="#" data-percentage="80"
                        class="dp-option group relative rounded-[14px] border border-tedja-border py-[14px] px-[10px] flex items-center justify-center hover:bg-tedja-black transition-all duration-300">
                        <span class="font-semibold group-hover:text-white transition-all duration-300">80%</span>
                    </a>
                </div>
            </section>
            <section id="KPR-Payment-Details"
                class="border border-[#F2F2F4] bg-white !h-fit w-full p-5 rounded-[20px] flex flex-col gap-4">
                <div class="flex items-center justify-between">
                    <h4 class="font-semibold text-lg leading-[27px]">KPR Payment Details</h4>
                    <button type="button" class="shrink-0" data-expand="KPRPaymentDetails">
                        <img src="{{ asset('assets/images/icons/arrow-circle-down.svg') }}" alt="icon"
                            class="size-6 shrink-0 transition-all duration-300">
                    </button>
                </div>
                <div class="flex flex-col gap-4" id="KPRPaymentDetails">
                    <hr class="border-[#F2F2F4]">
                    <div class="flex flex-col gap-4">
                        <div class="flex items-center justify-between">
                            <div class="flex items-center gap-2">
                                <img src="{{ asset('assets/images/icons/note.svg') }}" alt="icon"
                                    class="size-6 shrink-0">
                                <p>Down Payment</p>
                            </div>
                            <strong class="font-semibold"><span id="down-payment-amount">Rp 0</span>
                                (<span id="down-payment-percent">10%</span>)</strong>
                        </div>
                        <div class="flex items-center justify-between">
                            <div class="flex items-center gap-2">
                                <img src="{{ asset('assets/images/icons/note.svg') }}" alt="icon"
                                    class="size-6 shrink-0">
                                <p>Total Loan Amount</p>
                            </div>
                            <strong class="font-semibold"><span id="total-loan">Rp 0</span></strong>
                        </div>
                        <div class="flex items-center justify-between">
                            <div class="flex items-center gap-2">
                                <img src="{{ asset('assets/images/icons/note.svg') }}" alt="icon"
                                    class="size-6 shrink-0">
                                <p>Monthly Payment</p>
                            </div>
                            <strong class="font-semibold"><span id="monthly-payment">Rp 0</span></strong>
                        </div>
                        <div class="flex items-center justify-between">
                            <div class="flex items-center gap-2">
                                <img src="{{ asset('assets/images/icons/note.svg') }}" alt="icon"
                                    class="size-6 shrink-0">
                                <p>Monthly PPN 11%</p>
                            </div>
                            <strong class="font-semibold"><span id="monthly-ppn">Rp 0</span></strong>
                        </div>
                        <div class="flex items-center justify-between">
                            <div class="flex items-center gap-2">
                                <img src="{{ asset('assets/images/icons/note.svg') }}" alt="icon"
                                    class="size-6 shrink-0">
                                <p>Monthly Insurance</p>
                            </div>
                            <strong class="font-semibold"><span id="monthly-insurance">Rp 900.000</span></strong>
                        </div>
                        <div class="flex items-center justify-between">
                            <div class="flex items-center gap-2">
                                <img src="{{ asset('assets/images/icons/note.svg') }}" alt="icon"
                                    class="size-6 shrink-0">
                                <p>Loan + Interests Amount</p>
                            </div>
                            <strong class="font-bold text-xl text-tedja-blue"><span id="loan-with-interest">Rp
                                    0</span></strong>
                        </div>
                    </div>
                </div>
            </section>
            <form action="{{ route('front.interest.submitted') }}" method="POST" enctype="multipart/form-data"
                id="Request-Mortage"
                class="border border-[#F2F2F4] bg-white !h-fit w-full p-5 rounded-[20px] flex flex-col gap-5">
                @csrf
                <h4 class="font-semibold text-lg leading-[27px]">Request for Mortgage</h4>
                <input type="hidden" name="dp_percentage" id="dp-percentage" value="">
                <input type="hidden" id="interest_id" name="interest_id" value="{{ $interest->id }}">
                <input type="hidden" id="interestRate" value="{{ $interest->interest }}">
                <input type="hidden" id="durationYears" value="{{ $interest->duration }}">
                <input type="hidden" id="housePrice" value="{{ $interest->house->price }}">
                <div class="flex flex-col gap-4">
                    <div class="flex items-center gap-[6px]">
                        <img src="{{ asset('assets/images/icons/tick-circle.svg') }}" class="size-6 flex shrink-0"
                            alt="icon">
                        <p class="font-semibold">FC KTP, NPWP, KK, Akte</p>
                    </div>
                    <div class="flex items-center gap-[6px]">
                        <img src="{{ asset('assets/images/icons/tick-circle.svg') }}" class="size-6 flex shrink-0"
                            alt="icon">
                        <p class="font-semibold">Rekening Koran 6 Bulan</p>
                    </div>
                    <div class="flex items-center gap-[6px]">
                        <img src="{{ asset('assets/images/icons/tick-circle.svg') }}" class="size-6 flex shrink-0"
                            alt="icon">
                        <p class="font-semibold">Slip Gaji 3 Bulan</p>
                    </div>
                </div>
                <div class="flex flex-col gap-2">
                    <p class="font-semibold">Add Your Document in .PDF</p>
                    <label id="Input-File-Trigger" type="button"
                        class="relative flex items-center rounded-full border border-tedja-black py-[14px] px-5 gap-[10px]">
                        <img src="{{ asset('assets/images/icons/task-square.svg') }}" class="flex size-6 shrink-0"
                            alt="icon">
                        <p id="File-Label" class="text-tedja-secondary">Add an attachment to request</p>
                        <input type="file" name="documents" required id="File-Input" class="absolute opacity-0">
                    </label>
                </div>
                <hr class="border-[#F2F2F4]">
                <div class="flex items-center justify-center gap-[6px]">
                    <img src="{{ asset('assets/images/icons/security-safe-blue-fill.svg') }}"
                        class="size-6 flex shrink-0" alt="icon">
                    <p class="font-semibold">All your privacy data secured with our system</p>
                </div>
                <hr class="border-[#F2F2F4]">
                <button type="submit"
                    class="rounded-full py-[14px] px-5 bg-tedja-green w-full text-center font-semibold">Request for
                    Mortgage Now</button>
            </form>
        </div>
    </div>

@endsection

@push('after-scripts')
    <script src="https://code.jquery.com/jquery-3.7.1.min.js"
        integrity="sha256-/JqT3SQfawRcv/BIHPThkBvs0OEvtFFmqPF/lYI/Cxo=" crossorigin="anonymous"></script>
    <script src={{ asset('js/accordion.js') }}></script>
    <script src={{ asset('js/file-input.js') }}></script>
    <script src={{ asset('js/request-mortage.js') }}></script>
@endpush
