@extends('layouts.master')

@section('title', '404 Not Found - Tedja')

@section('content')
    <div
        style="min-height: 100vh; width: 100%; background-color: #FAFAFA; background-image: radial-gradient(#E5E5E5 1px, transparent 1px); background-size: 20px 20px; display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 40px 16px;">

        {{-- Logo --}}
        <div style="margin-bottom: 30px;">
            <a href="{{ route('front.index') }}">
                <img src="{{ asset('assets/images/logos/logo-black.svg') }}" alt="Tedja" style="height: 40px; width: auto;">
            </a>
        </div>

        {{-- Error Card --}}
        <div
            style="width: 100%; max-width: 500px; background: white; border: 1px solid #F2F2F4; border-radius: 20px; padding: 48px 32px; box-shadow: 0px 8px 30px rgba(6,9,34,0.03); text-align: center;">

            {{-- Error Icon --}}
            <div style="margin-bottom: 24px;">
                <div
                    style="width: 80px; height: 80px; background-color: #FFF9E6; border-radius: 50%; display: flex; align-items: center; justify-content: center; margin: 0 auto;">
                    <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="#FFB800" stroke-width="2"
                        stroke-linecap="round" stroke-linejoin="round">
                        <circle cx="12" cy="12" r="10"></circle>
                        <path d="M8 15s1.5-2 4-2 4 2 4 2"></path>
                        <line x1="9" y1="9" x2="9.01" y2="9"></line>
                        <line x1="15" y1="9" x2="15.01" y2="9"></line>
                    </svg>
                </div>
            </div>

            {{-- Error Code --}}
            <h1 style="font-weight: 800; font-size: 72px; line-height: 1; margin-bottom: 8px; color: #060922;">404</h1>

            {{-- Error Title --}}
            <h2 style="font-weight: 700; font-size: 24px; line-height: 36px; margin-bottom: 12px; color: #060922;">
                Page Not Found
            </h2>

            {{-- Error Message --}}
            <p style="font-weight: 500; color: #8F91A2; font-size: 14px; margin-bottom: 32px; line-height: 1.6;">
                Oops! The page you're looking for doesn't exist or has been moved. Let's get you back on track.
            </p>

            {{-- Action Buttons --}}
            <div style="display: flex; flex-direction: column; gap: 12px;">
                <a href="{{ route('front.index') }}"
                    style="display: block; width: 100%; border-radius: 9999px; padding: 14px 24px; background-color: #CEF27F; border: none; font-weight: 600; font-size: 14px; color: #060922; text-decoration: none; box-sizing: border-box;">
                    Back to Homepage
                </a>

                <a href="{{ route('front.search') }}"
                    style="display: block; width: 100%; border-radius: 9999px; padding: 14px 24px; border: 1px solid #060922; font-weight: 600; font-size: 14px; color: #060922; text-decoration: none; box-sizing: border-box;">
                    Browse Properties
                </a>
            </div>

            {{-- Search Suggestion --}}
            <div style="margin-top: 32px; padding-top: 24px; border-top: 1px solid #F2F2F4;">
                <p style="font-size: 12px; color: #8F91A2; margin-bottom: 8px;">Looking for something specific?</p>
                <a href="{{ route('front.search') }}"
                    style="font-size: 14px; font-weight: 600; color: #3F52FF; text-decoration: none;">
                    Search Properties
                </a>
            </div>
        </div>

        {{-- Footer Text --}}
        <div style="margin-top: 24px;">
            <p style="font-size: 12px; color: #8F91A2;">
                &copy; {{ date('Y') }} Tedja. All rights reserved.
            </p>
        </div>
    </div>
@endsection
