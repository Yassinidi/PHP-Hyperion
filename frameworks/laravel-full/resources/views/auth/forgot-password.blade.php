@extends('layouts.master')

@section('title', 'Forgot Password - Tedja')

@section('content')
    <div
        style="min-height: 100vh; justify-content: center; width: 100%; background-color: #FAFAFA; background-image: radial-gradient(#E5E5E5 1px, transparent 1px); background-size: 20px 20px; display: flex; flex-direction: column; align-items: center; padding: 40px 16px;">

        {{-- Logo --}}
        <div style="margin-bottom: 30px;">
            <a href="{{ route('front.index') }}">
                <img src="{{ asset('assets/images/logos/logo-black.svg') }}" alt="Tedja" style="height: 40px; width: auto;">
            </a>
        </div>

        {{-- Card --}}
        <div
            style="width: 100%; max-width: 500px; background: white; border: 1px solid #F2F2F4; border-radius: 20px; padding: 24px; box-shadow: 0px 8px 30px rgba(6,9,34,0.03);">

            {{-- Heading --}}
            <div style="text-align: center; margin-bottom: 24px;">
                <h1 style="font-weight: 700; font-size: 24px; line-height: 36px; margin-bottom: 4px;">Forgot Password?</h1>
                <p style="font-weight: 600; color: #8F91A2; font-size: 14px;">No problem! We'll send you a reset link</p>
            </div>

            {{-- Description --}}
            <div style="text-align: center; margin-bottom: 20px; padding: 16px; background-color: #F2F2F4; border-radius: 12px;">
                <p style="font-size: 13px; color: #8F91A2; line-height: 1.5; margin: 0;">
                    Enter your email address and we'll send you a password reset link that will allow you to choose a new one.
                </p>
            </div>

            {{-- Session Status --}}
            @if (session('status'))
                <div style="margin-bottom: 16px; padding: 12px 16px; background-color: #CEF27F; border-radius: 12px; text-align: center;">
                    <p style="font-size: 13px; font-weight: 600; color: #060922; margin: 0;">
                        {{ session('status') }}
                    </p>
                </div>
            @endif

            {{-- Form --}}
            <form method="POST" action="{{ route('password.email') }}" style="display: flex; flex-direction: column; gap: 14px;">
                @csrf

                {{-- Email --}}
                <div style="display: flex; flex-direction: column; gap: 6px;">
                    <label style="font-weight: 600; font-size: 14px;">Email Address</label>
                    <div style="position: relative;">
                        <img src="{{ asset('assets/images/icons/sms.svg') }}"
                            style="position: absolute; width: 20px; height: 20px; top: 50%; transform: translateY(-50%); left: 16px;"
                            alt="icon">
                        <input type="email" name="email" value="{{ old('email') }}" required autofocus
                            style="width: 100%; appearance: none; outline: none; border-radius: 9999px; border: 1px solid #F2F2F4; padding: 12px 16px 12px 48px; font-weight: 600; font-size: 14px; box-sizing: border-box;">
                    </div>
                    @error('email')
                        <p style="font-size: 12px; color: #FF3E3E;">{{ $message }}</p>
                    @enderror
                </div>

                {{-- Submit Button --}}
                <button type="submit"
                    style="width: 100%; border-radius: 9999px; padding: 12px 20px; background-color: #CEF27F; border: none; font-weight: 600; font-size: 14px; cursor: pointer; margin-top: 4px;">
                    Send Password Reset Link
                </button>
            </form>

            {{-- Divider --}}
            <div style="display: flex; align-items: center; gap: 12px; margin: 20px 0;">
                <div style="flex: 1; height: 1px; background-color: #F2F2F4;"></div>
                <span style="font-size: 12px; font-weight: 600; color: #8F91A2;">or</span>
                <div style="flex: 1; height: 1px; background-color: #F2F2F4;"></div>
            </div>

            {{-- Back to Login --}}
            <a href="{{ route('login') }}"
                style="display: block; width: 100%; border-radius: 9999px; padding: 12px 20px; border: 1px solid #060922; text-align: center; font-weight: 600; font-size: 14px; text-decoration: none; color: #060922; box-sizing: border-box;">
                Back to Sign In
            </a>
        </div>

        {{-- Back Link --}}
        <div style="margin-top: 24px;">
            <a href="{{ route('front.index') }}"
                style="display: flex; align-items: center; gap: 8px; font-size: 14px; font-weight: 600; color: #8F91A2; text-decoration: none;">
                <span style="font-size: 18px;">←</span> Back to homepage
            </a>
        </div>
    </div>
@endsection