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
                <form class="relative" action="{{ route('dashboard') }}" method="GET">
                    <button type="submit" class="absolute right-5 top-1/2 shrink-0 translate-y-[-50%]">
                        <img src="{{ asset('assets/images/icons/search.svg') }}" alt="icon" />
                    </button>
                    <input type="text" name="search" value="{{ request('search') }}"
                        class="w-[440px] placeholder:font-normal placeholder:text-base placeholder:leading-[24px] placeholder:text-tedja-secondary rounded-full border border-tedja-black py-[14px] pl-5  focus:outline-none pr-[64px] outline-none focus:ring-[2px] focus:ring-tedja-blue focus:border-transparent transition-all duration-300"
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
            <main class="flex w-full flex-col justify-center gap-[30px]  py-[28px]">
                <header class="flex items-center justify-between">
                    <div class="flex flex-col gap-[6px]">
                        <h1 class="text-[26px] font-bold leading-[39px]">My Mortgages</h1>
                        <p class="text-sm leading-[21px] text-tedja-secondary">We will help you achieve greatest</p>
                    </div>
                    <div class="buttons flex gap-[12px]">
                        <button type="button"
                            class="font-semibold text-white rounded-full py-[14px] px-5 bg-tedja-black transition-all duration-300">
                            Export to
                        </button>
                        <a href="">
                            <div class="font-semibold rounded-full py-[14px] px-5 bg-tedja-green text-tedja-black ">
                                Add New
                            </div>
                        </a>
                    </div>
                </header>
                <section id="Cards" class="flex flex-col gap-[30px]">

                    @forelse ($mortgages as $mortgage)
                        <div
                            class="card flex items-center gap-[12px] pt-[10px] pb-5 bg-white border border-[#F2F2F4] px-[10px] rounded-[30px]">
                            <div
                                class="w-[150px] h-[168px] shrink-0 rounded-[30px] overflow-hidden flex justify-center items-center">
                                <img src="{{ asset(Storage::url($mortgage->house->thumbnail)) }}" alt="image"
                                    class="w-full h-full object-cover">
                            </div>
                            <div class="px-[10px] flex flex-col gap-[18px] w-full">
                                <div class="title flex flex-col gap-[6px]">
                                    <h2 class="font-bold text-lg leading-[27px]">{{ $mortgage->house->name }}</h2>
                                    <div class="flex items-center gap-[6px]">
                                        <img src="{{ asset('assets/images/icons/location.svg') }}" alt="icon"
                                            class="shrink-0 size-5">
                                        <p class="font-semibold text-sm leading-[21px]">
                                            {{ $mortgage->house->category->name }},
                                            {{ $mortgage->house->city->name }}</p>
                                    </div>
                                </div>
                                <div class="points grid grid-cols-3 gap-x-[18px] gap-y-[12px]">
                                    <div
                                        class="flex items-center gap-[6px] p-[10px] border border-[#F2F2F4] rounded-[14px]">
                                        <img src="{{ asset('assets/images/icons/slider-vertical.svg') }}" alt="icon"
                                            class="shrink-0 size-5">
                                        <p class="font-semibold text-sm leading-[21px]">{{ $mortgage->house->bedroom }}
                                            Bedroom</p>
                                    </div>
                                    <div
                                        class="flex items-center gap-[6px] p-[10px] border border-[#F2F2F4] rounded-[14px]">
                                        <img src="{{ asset('assets/images/icons/building.svg') }}" alt="icon"
                                            class="shrink-0 size-5">
                                        <p class="font-semibold text-sm leading-[21px]">
                                            {{ $mortgage->house->building_area }} M²</p>
                                    </div>
                                    <div
                                        class="flex items-center gap-[6px] p-[10px] border border-[#F2F2F4] rounded-[14px]">
                                        <img src="{{ asset('assets/images/icons/note-favorite.svg') }}" alt="icon"
                                            class="shrink-0 size-5">
                                        <p class="font-semibold text-sm leading-[21px]">
                                            {{ $mortgage->house->certificate }}</p>
                                    </div>
                                    <div
                                        class="flex items-center gap-[6px] p-[10px] border border-[#F2F2F4] rounded-[14px]">
                                        <img src="{{ asset('assets/images/icons/slider-horizontal.svg') }}"
                                            alt="icon" class="shrink-0 size-5">
                                        <p class="font-semibold text-sm leading-[21px]">{{ $mortgage->house->bathroom }}
                                            Bathroom</p>
                                    </div>
                                    <div
                                        class="flex items-center gap-[6px] p-[10px] border border-[#F2F2F4] rounded-[14px]">
                                        <img src="{{ asset('assets/images/icons/maximize.svg') }}" alt="icon"
                                            class="shrink-0 size-5">
                                        <p class="font-semibold text-sm leading-[21px]">{{ $mortgage->house->land_area }}
                                            M²</p>
                                    </div>
                                    <div
                                        class="flex items-center gap-[6px] p-[10px] border border-[#F2F2F4] rounded-[14px]">
                                        <img src="{{ asset('assets/images/icons/flash.svg') }}" alt="icon"
                                            class="shrink-0 size-5">
                                        <p class="font-semibold text-sm leading-[21px]">{{ $mortgage->house->electric }}
                                            Watts</p>
                                    </div>
                                </div>
                            </div>
                            <div class="buttons flex flex-col gap-[12px] shrink-0">
                                @if ($mortgage->status == 'Waiting for Bank')
                                    <span
                                        class="text-center bg-tedja-orange rounded-full py-[12px] w-[140px] font-semibold">Waiting</span>
                                @elseif ($mortgage->status == 'approved')
                                    <span
                                        class="text-center bg-tedja-blue text-white rounded-full py-[12px] w-[140px] font-semibold">Approved</span>
                                @elseif ($mortgage->status == 'rejected')
                                    <span
                                        class="text-center bg-tedja-red text-white rounded-full py-[12px] w-[140px] font-semibold">Rejected</span>
                                @endif
                                <a href="{{ route('dashboard.mortgage.details', $mortgage) }}">
                                    <div
                                        class="font-semibold hover:text-[#FAFAFA] rounded-full py-[12px] w-[140px] bg-white border text-center border-tedja-black text-tedja-black hover:bg-tedja-black transition-all duration-300">
                                        Manage</div>
                                </a>
                                <a href="#">
                                    <div
                                        class="font-semibold hover:text-[#FAFAFA] rounded-full py-[12px] w-[140px] bg-white border text-center border-tedja-black text-tedja-black hover:bg-tedja-black transition-all duration-300">
                                        Download</div>
                                </a>
                            </div>
                        </div>
                    @empty
                        <div class="flex flex-col items-center justify-center py-16 px-8">
                            <p class="text-center text-tedja-secondary font-semibold text-lg mb-2">No mortgage data available</p>
                            <p class="text-center text-tedja-secondary text-sm">Apply for a mortgage to see it here</p>
                        </div>
                    @endforelse

                </section>

            </main>
        </div>
    </div>

@endsection
