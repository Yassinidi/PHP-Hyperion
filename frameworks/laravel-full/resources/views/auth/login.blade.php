@extends('layouts.master')

@section('title', 'Sign In - Tedja')

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
                <h1 style="font-weight: 700; font-size: 24px; line-height: 36px; margin-bottom: 4px;">Welcome Back</h1>
                <p style="font-weight: 600; color: #8F91A2; font-size: 14px;">Sign in to your account</p>
            </div>

            {{-- Form --}}
            <form action="{{ route('login') }}" method="POST" style="display: flex; flex-direction: column; gap: 14px;">
                @csrf

                {{-- Email --}}
                <div style="display: flex; flex-direction: column; gap: 6px;">
                    <label style="font-weight: 600; font-size: 14px;">Email</label>
                    <div style="position: relative;">
                        <img src="{{ asset('assets/images/icons/sms.svg') }}"
                            style="position: absolute; width: 20px; height: 20px; top: 50%; transform: translateY(-50%); left: 16px;"
                            alt="icon">
                        <input type="email" name="email" value="{{ old('email') }}" required
                            style="width: 100%; appearance: none; outline: none; border-radius: 9999px; border: 1px solid #F2F2F4; padding: 12px 16px 12px 48px; font-weight: 600; font-size: 14px; box-sizing: border-box;">
                    </div>
                    @error('email')
                        <p style="font-size: 12px; color: #FF3E3E;">{{ $message }}</p>
                    @enderror
                </div>

                {{-- Password --}}
                <div style="display: flex; flex-direction: column; gap: 6px;">
                    <div style="display: flex; align-items: center; justify-content: space-between;">
                        <label style="font-weight: 600; font-size: 14px;">Password</label>
                        <a href="{{ route('password.request') }}"
                            style="font-size: 12px; color: #3F52FF; text-decoration: underline; font-weight: 600;">Forgot?</a>
                    </div>
                    <div style="position: relative;">
                        <img src="{{ asset('assets/images/icons/lock.svg') }}"
                            style="position: absolute; width: 20px; height: 20px; top: 50%; transform: translateY(-50%); left: 16px;"
                            alt="icon">
                        <input type="password" name="password" id="password" required
                            style="width: 100%; appearance: none; outline: none; border-radius: 9999px; border: 1px solid #F2F2F4; padding: 12px 48px 12px 48px; font-weight: 600; font-size: 14px; box-sizing: border-box;">
                        {{-- Toggle Password Visibility --}}
                        <button type="button" id="toggle-password"
                            style="position: absolute; right: 16px; top: 50%; transform: translateY(-50%); background: none; border: none; cursor: pointer; padding: 0; display: flex; align-items: center; justify-content: center;">
                            {{-- Eye Icon (Show) --}}
                            <svg id="eye-show" width="20" height="20" viewBox="0 0 24 24" fill="none"
                                stroke="#8F91A2" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"></path>
                                <circle cx="12" cy="12" r="3"></circle>
                            </svg>
                            {{-- Eye Off Icon (Hide) --}}
                            <svg id="eye-hide" width="20" height="20" viewBox="0 0 24 24" fill="none"
                                stroke="#8F91A2" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
                                style="display: none;">
                                <path
                                    d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24">
                                </path>
                                <line x1="1" y1="1" x2="23" y2="23"></line>
                            </svg>
                        </button>
                    </div>
                    @error('password')
                        <p style="font-size: 12px; color: #FF3E3E;">{{ $message }}</p>
                    @enderror
                </div>

                {{-- Remember Me --}}
                <div style="display: flex; align-items: center; gap: 8px; margin-top: 4px;">
                    <input type="checkbox" name="remember" id="remember" style="width: 16px; height: 16px;"
                        {{ old('remember') ? 'checked' : '' }}>
                    <label for="remember" style="font-size: 14px; font-weight: 600;">Remember me</label>
                </div>

                {{-- Submit Button --}}
                <button type="submit"
                    style="width: 100%; border-radius: 9999px; padding: 12px 20px; background-color: #CEF27F; border: none; font-weight: 600; font-size: 14px; cursor: pointer; margin-top: 4px;">
                    Sign In
                </button>
            </form>

            {{-- Divider --}}
            <div style="display: flex; align-items: center; gap: 12px; margin: 20px 0;">
                <div style="flex: 1; height: 1px; background-color: #F2F2F4;"></div>
                <span style="font-size: 12px; font-weight: 600; color: #8F91A2;">or</span>
                <div style="flex: 1; height: 1px; background-color: #F2F2F4;"></div>
            </div>

            {{-- Sign Up Button --}}
            <a href="{{ route('register') }}"
                style="display: block; width: 100%; border-radius: 9999px; padding: 12px 20px; border: 1px solid #060922; text-align: center; font-weight: 600; font-size: 14px; text-decoration: none; color: #060922; box-sizing: border-box;">
                Create Account
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

    {{-- Password Toggle Script --}}
    <script>
        document.addEventListener('DOMContentLoaded', function() {
            const passwordInput = document.getElementById('password');
            const toggleButton = document.getElementById('toggle-password');
            const eyeShow = document.getElementById('eye-show');
            const eyeHide = document.getElementById('eye-hide');

            if (toggleButton) {
                toggleButton.addEventListener('click', function() {
                    if (passwordInput.type === 'password') {
                        passwordInput.type = 'text';
                        eyeShow.style.display = 'none';
                        eyeHide.style.display = 'block';
                    } else {
                        passwordInput.type = 'password';
                        eyeShow.style.display = 'block';
                        eyeHide.style.display = 'none';
                    }
                });
            }
        });
    </script>
@endsection
