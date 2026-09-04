@extends('layouts.master')

@section('title', 'Sign Up - Tedja')

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
                <h1 style="font-weight: 700; font-size: 24px; line-height: 36px; margin-bottom: 4px;">Create Account</h1>
                <p style="font-weight: 600; color: #8F91A2; font-size: 14px;">Sign up to get started</p>
            </div>

            {{-- Form --}}
            <form action="{{ route('register') }}" method="POST" enctype="multipart/form-data"
                style="display: flex; flex-direction: column; gap: 14px;">
                @csrf

                {{-- Photo Upload --}}
                <div style="display: flex; align-items: center; justify-content: center; gap: 16px; margin-bottom: 8px;">
                    <div
                        style="width: 60px; height: 60px; border-radius: 50%; overflow: hidden; border: 2px solid #F2F2F4; background-color: #F2F2F4;">
                        <img id="photo-container" src="{{ asset('assets/images/icons/default-avatar.svg') }}"
                            style="width: 100%; height: 100%; object-fit: cover;" alt="photo">
                    </div>
                    <input id="file-input" name="photo" type="file" accept="image/*"
                        style="position: absolute; opacity: 0;">
                    <div style="display: flex; flex-direction: column; gap: 6px;">
                        <p style="font-weight: 600; font-size: 14px;">Profile Photo</p>
                        <div style="display: flex; gap: 8px;">
                            <button type="button" id="add-photo"
                                style="border-radius: 9999px; padding: 6px 12px; background-color: #CEF27F; border: none; font-weight: 600; font-size: 12px; cursor: pointer;">Add</button>
                            <button type="button" id="edit-photo"
                                style="display: none; border-radius: 9999px; padding: 6px 12px; background-color: #CEF27F; border: none; font-weight: 600; font-size: 12px; cursor: pointer;">Edit</button>
                            <button type="button" id="remove-photo"
                                style="display: none; border-radius: 9999px; padding: 6px 12px; background-color: #FF3E3E; color: white; border: none; font-weight: 600; font-size: 12px; cursor: pointer;">Remove</button>
                        </div>
                    </div>
                    @error('photo')
                        <p style="font-size: 12px; color: #FF3E3E;">{{ $message }}</p>
                    @enderror
                </div>

                {{-- Name --}}
                <div style="display: flex; flex-direction: column; gap: 6px;">
                    <label style="font-weight: 600; font-size: 14px;">Full Name</label>
                    <div style="position: relative;">
                        <img src="{{ asset('assets/images/icons/profile.svg') }}"
                            style="position: absolute; width: 20px; height: 20px; top: 50%; transform: translateY(-50%); left: 16px;"
                            alt="icon">
                        <input type="text" name="name" value="{{ old('name') }}" required
                            style="width: 100%; appearance: none; outline: none; border-radius: 9999px; border: 1px solid #F2F2F4; padding: 12px 16px 12px 48px; font-weight: 600; font-size: 14px; box-sizing: border-box;">
                    </div>
                    @error('name')
                        <p style="font-size: 12px; color: #FF3E3E;">{{ $message }}</p>
                    @enderror
                </div>

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

                {{-- Phone --}}
                <div style="display: flex; flex-direction: column; gap: 6px;">
                    <label style="font-weight: 600; font-size: 14px;">WhatsApp</label>
                    <div style="position: relative;">
                        <img src="{{ asset('assets/images/icons/messages.svg') }}"
                            style="position: absolute; width: 20px; height: 20px; top: 50%; transform: translateY(-50%); left: 16px;"
                            alt="icon">
                        <input type="tel" name="phone" value="{{ old('phone') }}" required
                            style="width: 100%; appearance: none; outline: none; border-radius: 9999px; border: 1px solid #F2F2F4; padding: 12px 16px 12px 48px; font-weight: 600; font-size: 14px; box-sizing: border-box;">
                    </div>
                    @error('phone')
                        <p style="font-size: 12px; color: #FF3E3E;">{{ $message }}</p>
                    @enderror
                </div>

                {{-- Password --}}
                <div style="display: flex; flex-direction: column; gap: 6px;">
                    <label style="font-weight: 600; font-size: 14px;">Password</label>
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

                {{-- Password Confirmation --}}
                <div style="display: flex; flex-direction: column; gap: 6px;">
                    <label style="font-weight: 600; font-size: 14px;">Confirm Password</label>
                    <div style="position: relative;">
                        <img src="{{ asset('assets/images/icons/lock.svg') }}"
                            style="position: absolute; width: 20px; height: 20px; top: 50%; transform: translateY(-50%); left: 16px;"
                            alt="icon">
                        <input type="password" name="password_confirmation" id="password-confirm" required
                            style="width: 100%; appearance: none; outline: none; border-radius: 9999px; border: 1px solid #F2F2F4; padding: 12px 48px 12px 48px; font-weight: 600; font-size: 14px; box-sizing: border-box;">
                        {{-- Toggle Password Visibility --}}
                        <button type="button" id="toggle-password-confirm"
                            style="position: absolute; right: 16px; top: 50%; transform: translateY(-50%); background: none; border: none; cursor: pointer; padding: 0; display: flex; align-items: center; justify-content: center;">
                            {{-- Eye Icon (Show) --}}
                            <svg id="eye-show-confirm" width="20" height="20" viewBox="0 0 24 24" fill="none"
                                stroke="#8F91A2" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"></path>
                                <circle cx="12" cy="12" r="3"></circle>
                            </svg>
                            {{-- Eye Off Icon (Hide) --}}
                            <svg id="eye-hide-confirm" width="20" height="20" viewBox="0 0 24 24" fill="none"
                                stroke="#8F91A2" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
                                style="display: none;">
                                <path
                                    d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24">
                                </path>
                                <line x1="1" y1="1" x2="23" y2="23"></line>
                            </svg>
                        </button>
                    </div>
                </div>

                {{-- Terms --}}
                <div style="display: flex; align-items: flex-start; gap: 8px; margin-top: 4px;">
                    <input type="checkbox" name="terms" id="terms" required
                        style="width: 16px; height: 16px; margin-top: 2px;">
                    <label for="terms" style="font-size: 12px; font-weight: 600; line-height: 1.5;">
                        I agree to <a href="#" style="color: inherit; text-decoration: underline;">Terms</a> & <a
                            href="#" style="color: inherit; text-decoration: underline;">Privacy</a>
                    </label>
                </div>

                {{-- Submit Button --}}
                <button type="submit"
                    style="width: 100%; border-radius: 9999px; padding: 12px 20px; background-color: #CEF27F; border: none; font-weight: 600; font-size: 14px; cursor: pointer; margin-top: 4px;">
                    Create Account
                </button>
            </form>

            {{-- Divider --}}
            <div style="display: flex; align-items: center; gap: 12px; margin: 20px 0;">
                <div style="flex: 1; height: 1px; background-color: #F2F2F4;"></div>
                <span style="font-size: 12px; font-weight: 600; color: #8F91A2;">or</span>
                <div style="flex: 1; height: 1px; background-color: #F2F2F4;"></div>
            </div>

            {{-- Sign In Button --}}
            <a href="{{ route('login') }}"
                style="display: block; width: 100%; border-radius: 9999px; padding: 12px 20px; border: 1px solid #060922; text-align: center; font-weight: 600; font-size: 14px; text-decoration: none; color: #060922; box-sizing: border-box;">
                Sign In
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

    {{-- Scripts --}}
    <script src="{{ asset('js/photo-upload.js') }}"></script>
    <script>
        document.addEventListener('DOMContentLoaded', function() {
            // Password toggle
            const passwordInput = document.getElementById('password');
            const togglePassword = document.getElementById('toggle-password');
            const eyeShow = document.getElementById('eye-show');
            const eyeHide = document.getElementById('eye-hide');

            if (togglePassword) {
                togglePassword.addEventListener('click', function() {
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

            // Confirm password toggle
            const passwordConfirmInput = document.getElementById('password-confirm');
            const togglePasswordConfirm = document.getElementById('toggle-password-confirm');
            const eyeShowConfirm = document.getElementById('eye-show-confirm');
            const eyeHideConfirm = document.getElementById('eye-hide-confirm');

            if (togglePasswordConfirm) {
                togglePasswordConfirm.addEventListener('click', function() {
                    if (passwordConfirmInput.type === 'password') {
                        passwordConfirmInput.type = 'text';
                        eyeShowConfirm.style.display = 'none';
                        eyeHideConfirm.style.display = 'block';
                    } else {
                        passwordConfirmInput.type = 'password';
                        eyeShowConfirm.style.display = 'block';
                        eyeHideConfirm.style.display = 'none';
                    }
                });
            }
        });
    </script>
@endsection
