@extends('layouts.master')

@section('title', '403 Forbidden - Tedja')

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
                    style="width: 80px; height: 80px; background-color: #FFF5F5; border-radius: 50%; display: flex; align-items: center; justify-content: center; margin: 0 auto;">
                    <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="#FF3E3E" stroke-width="2"
                        stroke-linecap="round" stroke-linejoin="round">
                        <circle cx="12" cy="12" r="10"></circle>
                        <line x1="12" y1="8" x2="12" y2="12"></line>
                        <line x1="12" y1="16" x2="12.01" y2="16"></line>
                    </svg>
                </div>
            </div>

            {{-- Error Code --}}
            <h1 style="font-weight: 800; font-size: 72px; line-height: 1; margin-bottom: 8px; color: #060922;">403</h1>

            {{-- Error Title --}}
            <h2 style="font-weight: 700; font-size: 24px; line-height: 36px; margin-bottom: 12px; color: #060922;">
                Access Denied
            </h2>

            {{-- Error Message --}}
            <p style="font-weight: 500; color: #8F91A2; font-size: 14px; margin-bottom: 32px; line-height: 1.6;">
                Sorry, you don't have permission to access this page. Please contact your administrator if you believe
                this is a mistake.
            </p>

            {{-- Action Buttons --}}
            <div style="display: flex; flex-direction: column; gap: 12px;">
                @auth
                    {{-- Logged in users --}}
                    @if (auth()->user()->hasRole('admin'))
                        <a href="{{ route('filament.admin.pages.dashboard') }}"
                            style="display: block; width: 100%; border-radius: 9999px; padding: 14px 24px; background-color: #CEF27F; border: none; font-weight: 600; font-size: 14px; color: #060922; text-decoration: none; box-sizing: border-box;">
                            Go to Admin Dashboard
                        </a>
                    @else
                        <a href="{{ route('dashboard') }}"
                            style="display: block; width: 100%; border-radius: 9999px; padding: 14px 24px; background-color: #CEF27F; border: none; font-weight: 600; font-size: 14px; color: #060922; text-decoration: none; box-sizing: border-box;">
                            Go to My Dashboard
                        </a>
                    @endif
                @else
                    {{-- Guest users --}}
                    <a href="{{ route('login') }}"
                        style="display: block; width: 100%; border-radius: 9999px; padding: 14px 24px; background-color: #CEF27F; border: none; font-weight: 600; font-size: 14px; color: #060922; text-decoration: none; box-sizing: border-box;">
                        Sign In
                    </a>
                @endauth

                <a href="{{ route('front.index') }}"
                    style="display: block; width: 100%; border-radius: 9999px; padding: 14px 24px; border: 1px solid #060922; font-weight: 600; font-size: 14px; color: #060922; text-decoration: none; box-sizing: border-box;">
                    Back to Homepage
                </a>
            </div>

            {{-- Additional Help --}}
            <div style="margin-top: 32px; padding-top: 24px; border-top: 1px solid #F2F2F4;">
                <p style="font-size: 12px; color: #8F91A2; margin-bottom: 8px;">Need help?</p>
                <a href="mailto:support@tedja.com"
                    style="font-size: 14px; font-weight: 600; color: #3F52FF; text-decoration: none;">
                    Contact Support
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
