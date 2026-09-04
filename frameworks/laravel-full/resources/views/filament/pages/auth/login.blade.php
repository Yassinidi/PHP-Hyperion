<div>
    {{-- Load Tailwind CSS for consistent form styling --}}
    <link href="{{ asset('css/output.css') }}" rel="stylesheet">

    {{-- Main Content --}}
    <div
        style="position: fixed; inset: 0; background-color: #FAFAFA; background-image: radial-gradient(#E5E5E5 1px, transparent 1px); background-size: 20px 20px; display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 40px 16px; overflow-y: auto; z-index: 9999;">

        {{-- Logo --}}
        <div style="margin-bottom: 30px;">
            <a href="{{ route('front.index') }}">
                <img src="{{ asset('assets/images/logos/logo-black.svg') }}" alt="Tedja"
                    style="height: 40px; width: auto;">
            </a>
        </div>

        {{-- Card --}}
        <div
            style="width: 100%; max-width: 500px; background: white; border: 1px solid #F2F2F4; border-radius: 20px; padding: 24px; box-shadow: 0px 8px 30px rgba(6,9,34,0.03);">

            {{-- Heading --}}
            <div style="text-align: center; margin-bottom: 24px;">
                <h1 style="font-weight: 700; font-size: 24px; line-height: 36px; margin-bottom: 4px; color: #060922;">
                    Admin Panel</h1>
                <p style="font-weight: 600; color: #8F91A2; font-size: 14px;">Sign in to manage properties</p>
            </div>

            {{-- Form - Custom HTML matching frontend exactly --}}
            <form wire:submit.prevent="authenticate" style="display: flex; flex-direction: column; gap: 14px;">

                {{-- Email --}}
                <div style="display: flex; flex-direction: column; gap: 6px;">
                    <label style="font-weight: 600; font-size: 14px; color: #060922;">Email</label>
                    <div style="position: relative;">
                        <img src="{{ asset('assets/images/icons/sms.svg') }}"
                            style="position: absolute; width: 20px; height: 20px; top: 50%; transform: translateY(-50%); left: 16px; pointer-events: none;"
                            alt="icon">
                        <input type="email" wire:model="data.email" required
                            style="width: 100%; border-radius: 9999px; border: 1px solid #F2F2F4; padding: 12px 16px 12px 48px; font-weight: 600; font-size: 14px; box-sizing: border-box; color: #060922;">
                    </div>
                    @error('data.email')
                        <p style="font-size: 12px; color: #FF3E3E;">{{ $message }}</p>
                    @enderror
                </div>

                {{-- Password --}}
                <div style="display: flex; flex-direction: column; gap: 6px;">
                    <label style="font-weight: 600; font-size: 14px; color: #060922;">Password</label>
                    <div style="position: relative;">
                        <img src="{{ asset('assets/images/icons/lock.svg') }}"
                            style="position: absolute; width: 20px; height: 20px; top: 50%; transform: translateY(-50%); left: 16px; pointer-events: none;"
                            alt="icon">
                        <input type="password" wire:model="data.password" id="password" required
                            style="width: 100%; border-radius: 9999px; border: 1px solid #F2F2F4; padding: 12px 48px 12px 48px; font-weight: 600; font-size: 14px; box-sizing: border-box; color: #060922;">
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
                    @error('data.password')
                        <p style="font-size: 12px; color: #FF3E3E;">{{ $message }}</p>
                    @enderror
                </div>

                {{-- Remember Me --}}
                <div style="display: flex; align-items: center; gap: 8px; margin-top: 4px;">
                    <input type="checkbox" wire:model="data.remember" id="remember"
                        style="width: 16px; height: 16px; border: 1px solid #F2F2F4; border-radius: 4px;">
                    <label for="remember" style="font-size: 14px; font-weight: 600; color: #060922;">Remember me</label>
                </div>

                {{-- Submit Button --}}
                <button type="submit"
                    style="width: 100%; border-radius: 9999px; padding: 12px 20px; background-color: #CEF27F; border: none; font-weight: 600; font-size: 14px; color: #060922; cursor: pointer; margin-top: 4px;">
                    Sign In
                </button>
            </form>
        </div>

        {{-- Back Link --}}
        <div style="margin-top: 24px;">
            <a href="{{ route('front.index') }}"
                style="display: flex; align-items: center; gap: 8px; font-size: 14px; font-weight: 600; color: #8F91A2; text-decoration: none;">
                <span style="font-size: 18px;">←</span> Back to homepage
            </a>
        </div>

        {{-- Styles --}}
        <style>
            /* Reset all backgrounds */
            body,
            .fi-body,
            .fi-simple-layout,
            .fi-simple-main,
            .fi-simple-main-ctn,
            .fi-page {
                background: #FAFAFA !important;
            }

            .fi-simple-layout {
                background: transparent !important;
                display: block !important;
            }

            .fi-simple-main {
                background: transparent !important;
                box-shadow: none !important;
                padding: 0 !important;
                margin: 0 !important;
                max-width: none !important;
            }

            .fi-simple-main-ctn {
                background: transparent !important;
                padding: 0 !important;
            }

            /* Override browser autofill styling to match frontend */
            input:-webkit-autofill,
            input:-webkit-autofill:hover,
            input:-webkit-autofill:focus,
            input:-webkit-autofill:active {
                -webkit-box-shadow: 0 0 0 30px white inset !important;
                -webkit-text-fill-color: #060922 !important;
                transition: background-color 5000s ease-in-out 0s;
            }
        </style>

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
    </div>
