@extends('layouts.master')
@section('title', 'My Mortgage - Experimental Project')
@section('content')

    <div class="flex min-h-screen">
        <aside class="h-screen min-w-[270px] overflow-y-auto bg-tedja-black text-white [&::-webkit-scrollbar]:hidden">
            <div class="flex h-full w-full flex-col gap-[40px] pt-[40px]">
                <div class="pl-[30px]">
                    <a href="{{ route('front.index') }}" class="shrink-0">
                        <img src="{{ asset('assets/images/logos/logo-white.svg') }}" alt="icon" />
                    </a>
                </div>
                <nav class="flex flex-col gap-[40px] pb-[40px] pl-[30px]">
                    <section id="General" class="flex flex-col gap-[24px]">
                        <h3 class="text-sm font-semibold leading-[21px]">GENERAL</h3>
                        <ul class="flex flex-col gap-[24px]">
                            <li class="group">
                                <a href="">
                                    <div class="relative flex items-center gap-[6px]">
                                        <img src="{{ asset('assets/images/icons/overview-n.svg') }}" alt="icon"
                                            class="shrink-0 group-[&.active]:hidden" />
                                        <p class="group-[&.active]:font-semibold group-[&.active]:text-tedja-green">
                                            Overview</p>
                                    </div>
                                </a>
                            </li>
                            <li class="active group">
                                <a href="{{ route('dashboard') }}">
                                    <div class="relative flex items-center gap-[6px]">
                                        <img src="{{ asset('assets/images/icons/mortgages-y.svg') }}" alt="icon"
                                            class="hidden shrink-0 group-[&.active]:block" />
                                        <p class="group-[&.active]:font-semibold group-[&.active]:text-tedja-green">
                                            Mortgages</p>
                                    </div>
                                </a>
                            </li>
                            <li class="group">
                                <a href="">
                                    <div class="relative flex items-center gap-[6px]">
                                        <img src="{{ asset('assets/images/icons/bank-interests-n.svg') }}" alt="icon"
                                            class="shrink-0 group-[&.active]:hidden" />
                                        <p class="group-[&.active]:font-semibold group-[&.active]:text-tedja-green">Bank
                                            Interests</p>
                                    </div>
                                </a>
                            </li>
                            <li class="group">
                                <a href="">
                                    <div class="relative flex items-center gap-[6px]">
                                        <img src="{{ asset('assets/images/icons/big-rewards-n.svg') }}" alt="icon"
                                            class="shrink-0 group-[&.active]:hidden" />
                                        <p class="group-[&.active]:font-semibold group-[&.active]:text-tedja-green">Big
                                            Rewards</p>
                                    </div>
                                </a>
                            </li>
                        </ul>
                    </section>
                    <section id="Others" class="flex flex-col gap-[24px]">
                        <h3 class="text-sm font-semibold leading-[21px]">OTHERS</h3>
                        <ul class="flex flex-col gap-[24px]">
                            <li class="group">
                                <a href="">
                                    <div class="relative flex items-center gap-[6px]">
                                        <img src="{{ asset('assets/images/icons/help-center-n.svg') }}" alt="icon"
                                            class="shrink-0 group-[&.active]:hidden" />
                                        <p class="group-[&.active]:font-semibold group-[&.active]:text-tedja-green">Help
                                            Center</p>
                                    </div>
                                </a>
                            </li>
                            <li class="group">
                                <a href="">
                                    <div class="relative flex items-center gap-[6px]">
                                        <img src="{{ asset('assets/images/icons/supports-n.svg') }}" alt="icon"
                                            class="shrink-0 group-[&.active]:hidden" />
                                        <p class="group-[&.active]:font-semibold group-[&.active]:text-tedja-green">
                                            Supports</p>
                                    </div>
                                </a>
                            </li>
                            <li class="group">
                                <a href="">
                                    <div class="relative flex items-center gap-[6px]">
                                        <img src="{{ asset('assets/images/icons/settings-n.svg') }}" alt="icon"
                                            class="shrink-0 group-[&.active]:hidden" />
                                        <p class="group-[&.active]:font-semibold group-[&.active]:text-tedja-green">
                                            Settings</p>
                                    </div>
                                </a>
                            </li>
                        </ul>
                    </section>
                </nav>
            </div>
        </aside>
        <div class="h-screen w-full overflow-y-auto pt-[30px] px-[30px]">
            <section id="NavTop" class="flex w-full items-center justify-between bg-white p-4 rounded-3xl">
                <form class="relative opacity-50 pointer-events-none">
                    <button type="submit" disabled class="absolute right-5 top-1/2 shrink-0 translate-y-[-50%] pointer-events-none">
                        <img src="{{ asset('assets/images/icons/search.svg') }}" alt="icon" />
                    </button>
                    <input type="text" disabled
                        class="w-[440px] placeholder:font-normal placeholder:text-base placeholder:leading-[24px] placeholder:text-tedja-secondary rounded-full border border-tedja-black py-[14px] pl-5 focus:outline-none pr-[64px] outline-none focus:ring-[2px] focus:ring-tedja-blue focus:border-transparent transition-all duration-300"
                        placeholder="Search your mortgage" />
                </form>
                <div class="flex items-center gap-5">
                    <div class="flex items-center gap-[12px]">
                        <a href="" class="shrink-0">
                            <div
                                class="p-[13px] rounded-full border border-[#F2F2F4] hover:ring-[2px] hover:ring-tedja-blue transition-all duration-300">
                                <img src="{{ asset('assets/images/icons/device-message.svg') }}" alt="icon" />
                            </div>
                        </a>
                        <a href="" class="shrink-0">
                            <div
                                class="p-[13px] rounded-full border border-[#F2F2F4] hover:ring-[2px] hover:ring-tedja-blue transition-all duration-300">
                                <img src="{{ asset('assets/images/icons/cup.svg') }}" alt="icon" />
                            </div>
                        </a>
                        <a href="" class="shrink-0">
                            <div
                                class="p-[13px] rounded-full border border-[#F2F2F4] hover:ring-[2px] hover:ring-tedja-blue transition-all duration-300">
                                <img src="{{ asset('assets/images/icons/folder-favorite.svg') }}" alt="icon" />
                            </div>
                        </a>

                    </div>
                    <div class="w-px bg-[#F2F2F4] h-[50px]"></div>
                    <button id="Profile" class="relative">
                        <div class="flex items-center gap-[14px]">
                            <div class="flex text-right flex-col gap-0.5">
                                <p class="text-sm text-tedja-secondary">Howdy,</p>
                                <p class="font-semibold">{{ Auth::user()->name }}</p>
                            </div>
                            <div class="flex rounded-full size-[50px] overflow-hidden">
                                <img src="{{ Storage::url(Auth::user()->photo) }}" class="w-full h-full object-cover"
                                    alt="photo">
                            </div>
                        </div>
                        <ul
                            class="hidden absolute top-full mt-[10px] right-0 flex flex-col w-[170px] shrink-0 h-fit text-left rounded-xl border border-tedja-border py-5 px-5 bg-white shadow-[0px_10px_30px_0px_#B8B8B840] gap-[14px]">
                            <li>
                                <a href="#" class="hover:text-tedja-blue transition-all duration-300">Rewards</a>
                            </li>
                            <li>
                                <a href="{{ route('dashboard') }}"
                                    class="hover:text-tedja-blue transition-all duration-300">My
                                    Mortgages</a>
                            </li>
                            <li>
                                <a href="#" class="hover:text-tedja-blue transition-all duration-300">Learn
                                    Property</a>
                            </li>
                            <li>
                                <a href="#" class="hover:text-tedja-blue transition-all duration-300">Settings</a>
                            </li>
                            <li>
                                <!-- Authentication -->
                                <form method="POST" action="{{ route('logout') }}">
                                    @csrf

                                    <a href="route('logout')" class="hover:text-tedja-blue transition-all duration-300"
                                        onclick="event.preventDefault();
                                                this.closest('form').submit();">
                                        {{ __('Log Out') }}
                                    </a>
                                </form>
                            </li>
                        </ul>
                    </button>
                </div>
            </section>
            <main class="flex w-full flex-col justify-center gap-[30px] pt-[28px] pb-[70px]">
                <header class="flex flex-col gap-[6px]">
                    <h1 class="text-[26px] font-bold leading-[39px]">Mortgage Details</h1>
                    <p class="text-sm leading-[21px] text-tedja-secondary">Tedja always make your family comfortable</p>
                </header>
                <div class="flex flex-col gap-5 py-5 pl-5 rounded-[30px] border border-[#F2F2F4] bg-white">
                    <div class="flex items-center gap-4 pr-5">
                        <section id="Hero"
                            class="flex items-center justify-center w-[290px] h-[200px] shrink-0 overflow-hidden rounded-[30px]">
                            <img src="{{ Storage::url($mortgageRequest->house->thumbnail) }}" alt="image"
                                class="w-full h-full object-cover">
                        </section>
                        <div class="flex flex-col gap-5 w-full">
                            <section id="Title" class="flex items-center justify-between">
                                <div class="flex flex-col gap-[6px]">
                                    <h2 class="font-bold text-[20px] leading-[30px]">
                                        {{ $mortgageRequest->house->name }}</h2>
                                    <div class="flex items-center gap-[6px]">
                                        <img src="{{ asset('assets/images/icons/location.svg') }}" alt="icon"
                                            class="shrink-0 size-6">
                                        <p class="font-semibold">
                                            {{ $mortgageRequest->house->category->name }},
                                            {{ $mortgageRequest->house->city->name }}</p>
                                    </div>
                                </div>

                                @if ($mortgageRequest->status == 'Waiting for Bank')
                                    <span class="bg-tedja-orange rounded-full py-[6px] px-[10px] font-semibold">Waiting for
                                        Bank
                                    </span>
                                @elseif ($mortgageRequest->status == 'approved')
                                    <span class="bg-tedja-blue rounded-full text-white py-[6px] px-[10px] font-semibold">
                                        Approved
                                    </span>
                                @elseif ($mortgageRequest->status == 'rejected')
                                    <span style="background-color: #ef4444; color: #ffffff;"
                                        class="rounded-full py-[6px] px-[10px] font-semibold">
                                        Rejected
                                    </span>
                                @endif

                            </section>
                            <section id="Points" class="grid grid-cols-3 gap-x-[18px] gap-y-[12px]">
                                <div class="flex items-center gap-[6px] p-[10px] border border-[#F2F2F4] rounded-[14px]">
                                    <img src="{{ asset('assets/images/icons/slider-vertical.svg') }}" alt="icon"
                                        class="shrink-0 size-5">
                                    <p class="font-semibold text-sm leading-[21px]">
                                        {{ $mortgageRequest->house->bedroom }}
                                        Bedroom</p>
                                </div>
                                <div class="flex items-center gap-[6px] p-[10px] border border-[#F2F2F4] rounded-[14px]">
                                    <img src="{{ asset('assets/images/icons/building.svg') }}" alt="icon"
                                        class="shrink-0 size-5">
                                    <p class="font-semibold text-sm leading-[21px]">
                                        {{ $mortgageRequest->house->building_area }} M²</p>
                                </div>
                                <div class="flex items-center gap-[6px] p-[10px] border border-[#F2F2F4] rounded-[14px]">
                                    <img src="{{ asset('assets/images/icons/note-favorite.svg') }}" alt="icon"
                                        class="shrink-0 size-5">
                                    <p class="font-semibold text-sm leading-[21px]">
                                        {{ $mortgageRequest->house->certificate }}</p>
                                </div>
                                <div class="flex items-center gap-[6px] p-[10px] border border-[#F2F2F4] rounded-[14px]">
                                    <img src="{{ asset('assets/images/icons/slider-horizontal.svg') }}" alt="icon"
                                        class="shrink-0 size-5">
                                    <p class="font-semibold text-sm leading-[21px]">
                                        {{ $mortgageRequest->house->bathroom }} Bathroom</p>
                                </div>
                                <div class="flex items-center gap-[6px] p-[10px] border border-[#F2F2F4] rounded-[14px]">
                                    <img src="{{ asset('assets/images/icons/maximize.svg') }}" alt="icon"
                                        class="shrink-0 size-5">
                                    <p class="font-semibold text-sm leading-[21px]">
                                        {{ $mortgageRequest->house->land_area }} M²</p>
                                </div>
                                <div class="flex items-center gap-[6px] p-[10px] border border-[#F2F2F4] rounded-[14px]">
                                    <img src="{{ asset('assets/images/icons/flash.svg') }}" alt="icon"
                                        class="shrink-0 size-5">
                                    <p class="font-semibold text-sm leading-[21px]">
                                        {{ $mortgageRequest->house->electric }} Watts</p>
                                </div>
                            </section>
                        </div>
                    </div>
                    <div class="ContainerTabs">
                        <div id="TabButtons" class="flex items-center gap-[12px] pr-5">
                            <button type="button"
                                class="rounded-full group bg-tedja-black py-[12px] px-[24px] hover:bg-tedja-black transition-all duration-300">
                                <h3
                                    class="font-semibold text-[#FAFAFA] group-hover:text-[#FAFAFA] transition-all duration-300">
                                    Details</h3>
                            </button>
                            <button type="button"
                                class="rounded-full group bg-white border border-tedja-black py-[12px] px-[24px]  hover:bg-tedja-black transition-all duration-300">
                                <h3
                                    class="font-semibold text-tedja-black group-hover:text-[#FAFAFA] transition-all duration-300">
                                    Installments</h3>
                            </button>
                            <button type="button"
                                class="rounded-full group bg-white border border-tedja-black py-[12px] px-[24px]  hover:bg-tedja-black transition-all duration-300">
                                <h3
                                    class="font-semibold text-tedja-black group-hover:text-[#FAFAFA] transition-all duration-300">
                                    Guidance</h3>
                            </button>
                            <button type="button"
                                class="rounded-full group bg-white border border-tedja-black py-[12px] px-[24px]  hover:bg-tedja-black transition-all duration-300">
                                <h3
                                    class="font-semibold text-tedja-black group-hover:text-[#FAFAFA] transition-all duration-300">
                                    Agents</h3>
                            </button>
                        </div>
                        <div class="TabValues mt-5">
                            <section id="ContainerDetails">
                                <div class="grid grid-cols-2 gap-4">
                                    <div class="flex-col flex gap-4">
                                        <section id="PriceDetails"
                                            class="border border-[#F2F2F4] p-5 rounded-[20px] flex flex-col gap-4">
                                            <h4 class="font-semibold text-lg leading-[27px]">Price Details</h4>
                                            <hr class="border-[#F2F2F4]">
                                            <div class="flex flex-col gap-4">
                                                <div class="flex items-center justify-between">
                                                    <div class="flex items-center gap-2">
                                                        <img src="{{ asset('assets/images/icons/note.svg') }}"
                                                            alt="icon" class="size-6 shrink-0">
                                                        <p>House Price</p>
                                                    </div>
                                                    <strong class="font-semibold">Rp
                                                        {{ number_format($mortgageRequest->house->price, 0, '', '.') }}</strong>
                                                </div>
                                                <div class="flex items-center justify-between">
                                                    <div class="flex items-center gap-2">
                                                        <img src="{{ asset('assets/images/icons/note.svg') }}"
                                                            alt="icon" class="size-6 shrink-0">
                                                        <p>Security Fee</p>
                                                    </div>
                                                    <strong class="font-semibold">Free Included</strong>
                                                </div>
                                                <div class="flex items-center justify-between">
                                                    <div class="flex items-center gap-2">
                                                        <img src="{{ asset('assets/images/icons/note.svg') }}"
                                                            alt="icon" class="size-6 shrink-0">
                                                        <p>New Registration</p>
                                                    </div>
                                                    <strong class="font-semibold">Free Included</strong>
                                                </div>
                                            </div>
                                        </section>
                                        <section id="KPRPaymentDetails"
                                            class="border border-[#F2F2F4] p-5 rounded-[20px] flex flex-col gap-4">
                                            <h4 class="font-semibold text-lg leading-[27px]">KPR Payment Details</h4>
                                            <hr class="border-[#F2F2F4]">
                                            <div class="flex flex-col gap-4">
                                                <div class="flex items-center justify-between">
                                                    <div class="flex items-center gap-2">
                                                        <img src="{{ asset('assets/images/icons/note.svg') }}"
                                                            alt="icon" class="size-6 shrink-0">
                                                        <p>Down Payment</p>
                                                    </div>
                                                    <strong class="font-semibold">Rp
                                                        {{ number_format($mortgageRequest->dp_total_amount, 0, '', '.') }}
                                                        ({{ $mortgageRequest->dp_percentage }}%)</strong>
                                                </div>
                                                <div class="flex items-center justify-between">
                                                    <div class="flex items-center gap-2">
                                                        <img src="{{ asset('assets/images/icons/note.svg') }}"
                                                            alt="icon" class="size-6 shrink-0">
                                                        <p>Total Loan Amount</p>
                                                    </div>
                                                    <strong class="font-semibold">Rp
                                                        {{ number_format($mortgageRequest->loan_total_amount, 0, '', '.') }}</strong>
                                                </div>
                                                <div class="flex items-center justify-between">
                                                    <div class="flex items-center gap-2">
                                                        <img src="{{ asset('assets/images/icons/note.svg') }}"
                                                            alt="icon" class="size-6 shrink-0">
                                                        <p>Monthly Payment</p>
                                                    </div>
                                                    <strong class="font-semibold">Rp
                                                        {{ number_format($mortgageRequest->monthly_amount, 0, '', '.') }}</strong>
                                                </div>
                                                <div class="flex items-center justify-between">
                                                    <div class="flex items-center gap-2">
                                                        <img src="{{ asset('assets/images/icons/note.svg') }}"
                                                            alt="icon" class="size-6 shrink-0">
                                                        <p>Monthly PPN 11%</p>
                                                    </div>
                                                    <strong class="font-semibold">Rp
                                                        {{ number_format($totalTaxAmount, 0, '', '.') }}</strong>
                                                </div>
                                                <div class="flex items-center justify-between">
                                                    <div class="flex items-center gap-2">
                                                        <img src="{{ asset('assets/images/icons/note.svg') }}"
                                                            alt="icon" class="size-6 shrink-0">
                                                        <p>Monthly Insurance</p>
                                                    </div>
                                                    <strong class="font-semibold">Rp
                                                        {{ number_format($insurance, 0, '', '.') }}</strong>
                                                </div>
                                                <div class="flex items-center justify-between">
                                                    <div class="flex items-center gap-2">
                                                        <img src="{{ asset('assets/images/icons/note.svg') }}"
                                                            alt="icon" class="size-6 shrink-0">
                                                        <p>Loan + Interests Amount</p>
                                                    </div>
                                                    <strong class="font-bold text-[20px] leading-[30px] text-tedja-blue">Rp
                                                        {{ number_format($mortgageRequest->loan_interest_total_amount, 0, '', '.') }}</strong>
                                                </div>
                                            </div>
                                        </section>
                                    </div>
                                    <div class="flex flex-col gap-[10px]">
                                        <section id="BankKPRDetails"
                                            class="border border-[#F2F2F4] p-5 rounded-[20px] flex flex-col gap-4">
                                            <h4 class="font-semibold text-lg leading-[27px]">Bank KPR Details</h4>
                                            <hr class="border-[#F2F2F4]">
                                            <div class="bank flex items-center gap-5">
                                                <div class="w-[99.4px] h-[70px] shrink-0 flex justify-center items-center">
                                                    <img src="{{ Storage::url($mortgageRequest->interestModel->bank->photo) }}"
                                                        alt="image" class="w-full h-full object-contain">
                                                </div>
                                                <div class="flex flex-col gap-[2px]">
                                                    <div class="flex items-center gap-[2px]">
                                                        <h5 class="font-semibold">{{ $mortgageRequest->bank_name }}</h5>
                                                        <img src="{{ asset('assets/images/icons/verify-blue-fill.svg') }}"
                                                            alt="icon" class="size-5 shrink-0">
                                                    </div>
                                                    <p class="text-sm leading-[21px] text-tedja-secondary">Dipercaya jutaan
                                                        nasabah</p>
                                                </div>
                                            </div>
                                            <div class="flex flex-col gap-4">
                                                <div class="flex items-center justify-between">
                                                    <div class="flex items-center gap-2">
                                                        <img src="{{ asset('assets/images/icons/note.svg') }}"
                                                            alt="icon" class="size-6 shrink-0">
                                                        <p>Bank Interest Guarantee</p>
                                                    </div>
                                                    <strong class="font-semibold">14%</strong>
                                                </div>
                                                <div class="flex items-center justify-between">
                                                    <div class="flex items-center gap-2">
                                                        <img src="{{ asset('assets/images/icons/note.svg') }}"
                                                            alt="icon" class="size-6 shrink-0">
                                                        <p>Loan Duration</p>
                                                    </div>
                                                    <strong class="font-semibold">{{ $mortgageRequest->duration }}
                                                        Years</strong>
                                                </div>
                                                <div class="flex items-center justify-between">
                                                    <div class="flex items-center gap-2">
                                                        <img src="{{ asset('assets/images/icons/note.svg') }}"
                                                            alt="icon" class="size-6 shrink-0">
                                                        <p>Remaining Loan</p>
                                                    </div>
                                                    <strong class="font-semibold">Rp
                                                        {{ number_format($mortgageRequest->remaining_loan_amount, 0, '', '.') }}</strong>
                                                </div>

                                            </div>
                                        </section>
                                        <section id="YouveSubmittedDocuments"
                                            class="border border-[#F2F2F4] p-5 rounded-[20px] flex flex-col gap-4">
                                            <h4 class="font-semibold text-lg leading-[27px]">You’ve submitted documents:
                                            </h4>
                                            <div class="flex flex-col gap-4">
                                                <div class="flex items-center gap-[6px]">
                                                    <img src="{{ asset('assets/images/icons/tick-circle-green-fill.svg') }}"
                                                        alt="icon" class="size-6 shrink-0">
                                                    <p class="font-semibold">FC KTP, NPWP, KK, Akte</p>
                                                </div>
                                                <div class="flex items-center gap-[6px]">
                                                    <img src="{{ asset('assets/images/icons/tick-circle-green-fill.svg') }}"
                                                        alt="icon" class="size-6 shrink-0">
                                                    <p class="font-semibold">Rekening Koran 6 Bulan</p>
                                                </div>
                                                <div class="flex items-center gap-[6px]">
                                                    <img src="{{ asset('assets/images/icons/tick-circle-green-fill.svg') }}"
                                                        alt="icon" class="size-6 shrink-0">
                                                    <p class="font-semibold">Slip Gaji 3 Bulan</p>
                                                </div>
                                            </div>
                                            <hr class="border-[#F2F2F4]">
                                            <div class="flex items-center gap-[6px] pl-[7px]">
                                                <img src="{{ asset('assets/images/icons/security-safe-blue-fill.svg') }}"
                                                    alt="icon" class="size-6 shrink-0">
                                                <p class="font-semibold">All your privacy data secured with our system</p>
                                            </div>
                                        </section>
                                    </div>
                                </div>

                            </section>
                            <section id="ContainerInstallments" class="pr-5">

                                @if ($mortgageRequest->status !== 'approved')
                                    <section id="NotAvailable" class="py-[40px] flex justify-center">
                                        <div class="flex flex-col gap-5 w-[460px] mx-auto">
                                            <div class="flex flex-col gap-[12px]">
                                                <h4 class="font-bold text-xl leading-[30px] text-center">Not Available</h4>
                                                <p class="leading-7 text-center">Your mortgage status is still in process.
                                                    You cannot make installment payments for this mortgage yet. Please be
                                                    patient.</p>
                                            </div>
                                            <a href="">
                                                <div
                                                    class="bg-tedja-green rounded-full font-semibold w-fit mx-auto py-[14px] px-5">
                                                    Call Tedja Sales</div>
                                            </a>
                                        </div>
                                    </section>
                                @else
                                    <section id="Available" class="flex flex-col gap-5">
                                        <div class="flex items-center justify-between">
                                            <div class="flex flex-col gap-[2px]">
                                                <h4 class="font-bold text-xl leading-[30px]">Installments Histories</h4>
                                                <p class="text-sm leading-[21px] text-tedja-secondary">Tedja always make
                                                    your family comfortable</p>
                                            </div>
                                            <a href="{{ route('dashboard.installment.payment', $mortgageRequest) }}">
                                                <div class="rounded-full py-[14px] px-5 bg-tedja-green font-semibold">Make
                                                    a New Payment</div>
                                            </a>
                                        </div>
                                        <div class="flex flex-col gap-5">

                                            @forelse ($mortgageRequest->installments as $installment)
                                                <div
                                                    class="flex items-center justify-between p-[10px] rounded-[30px] border border-[#F2F2F4]">
                                                    <div class="flex items-center gap-[54px]">
                                                        <img src="{{ asset('assets/images/icons/crown-blue-fill.svg') }}"
                                                            alt="icon" class="size-[60px] shrink-0">
                                                        <div class="flex flex-col gap-[2px]">
                                                            <h5 class="text-sm leading-[21px] text-tedja-secondary">Grand
                                                                Total
                                                            </h5>
                                                            <strong class="font-bold">Rp
                                                                {{ number_format($installment->grand_total_amount, 0, '', '.') }}</strong>
                                                        </div>
                                                    </div>
                                                    <span
                                                        class="font-semibold text-[#FAFAFA] rounded-full bg-tedja-blue px-[10px] py-[6px]">Settlement</span>
                                                    <div class="flex flex-col gap-[2px]">
                                                        <h5 class="text-sm leading-[21px] text-tedja-secondary">Payment
                                                        </h5>
                                                        <strong class="font-bold">Payment of
                                                            {{ $installment->no_of_payment }}</strong>
                                                    </div>
                                                    <div class="flex flex-col gap-[2px]">
                                                        <h5 class="text-sm leading-[21px] text-tedja-secondary">Date</h5>
                                                        <strong
                                                            class="font-bold">{{ $installment->created_at->format('d M, Y') }}</strong>
                                                    </div>
                                                    <a href="{{ route('dashboard.installment.details', $installment) }}"
                                                        class="shrink-0">
                                                        <div
                                                            class="rounded-full py-[12px] font-semibold flex justify-center w-[140px] bg-tedja-black text-[#FAFAFA]">
                                                            Details</div>
                                                    </a>
                                                </div>
                                            @empty
                                                <div class="flex flex-col items-center justify-center py-10">
                                                    <p class="text-center text-tedja-secondary font-semibold">No installment payments yet</p>
                                                    <p class="text-center text-tedja-secondary text-sm mt-2">Payments will appear here once made</p>
                                                </div>
                                            @endforelse

                                        </div>
                                    </section>
                                @endif

                            </section>
                            <section id="ContainerGuidance" class="px-5">
                                <section class="py-[40px] flex justify-center">
                                    <div class="flex flex-col gap-5 w-[460px] mx-auto">
                                        <div class="flex flex-col gap-[12px]">
                                            <h4 class="font-bold text-xl leading-[30px] text-center">Coming Soon</h4>
                                            <p class="leading-7 text-center">Mortgage guidance and property tips feature is
                                                under development. Stay tuned for the latest information from Tedja to help
                                                with your property decisions.</p>
                                        </div>
                                        <a href="">
                                            <div
                                                class="bg-tedja-green rounded-full font-semibold w-fit mx-auto py-[14px] px-5">
                                                Explore Tedja</div>
                                        </a>
                                    </div>
                                </section>
                            </section>
                            <section id="ContainerAgents" class="px-5">
                                <section class="py-[40px] flex justify-center">
                                    <div class="flex flex-col gap-5 w-[460px] mx-auto">
                                        <div class="flex flex-col gap-[12px]">
                                            <h4 class="font-bold text-xl leading-[30px] text-center">Coming Soon</h4>
                                            <p class="leading-7 text-center">Property agent feature is under development.
                                                You will soon be able to connect with trusted agents through Tedja.</p>
                                        </div>
                                        <a href="">
                                            <div
                                                class="bg-tedja-green rounded-full font-semibold w-fit mx-auto py-[14px] px-5">
                                                Call Tedja Sales</div>
                                        </a>
                                    </div>
                                </section>
                            </section>
                        </div>
                    </div>
                </div>
            </main>
        </div>
    </div>

@endsection

@push('after-scripts')
    <script src="{{ asset('js/my-mortgages-details.js') }}"></script>
@endpush
