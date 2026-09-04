<?php

namespace App\Http\Controllers\Auth;

use App\Http\Controllers\Controller;
use App\Http\Requests\Auth\RegisterRequest;
use App\Models\User;
use Illuminate\Auth\Events\Registered;
use Illuminate\Http\RedirectResponse;
use Illuminate\Support\Facades\Auth;
use Illuminate\Support\Facades\Hash;
use Illuminate\View\View;

/**
 * Controller for handling user registration.
 *
 * Manages the registration flow including:
 * - Displaying the registration form
 * - Processing registration with photo upload
 * - Auto-login after successful registration
 */
class RegisteredUserController extends Controller
{
    /**
     * Display the registration view.
     */
    public function create(): View
    {
        return view('auth.register');
    }

    /**
     * Handle an incoming registration request.
     *
     * Validates input via RegisterRequest, handles photo upload,
     * creates user, fires Registered event, and logs in the user.
     */
    public function store(RegisterRequest $request): RedirectResponse
    {
        $userData = [
            'name' => $request->name,
            'email' => $request->email,
            'phone' => $request->phone,
            'password' => Hash::make($request->password),
        ];

        // Handle photo upload if provided
        if ($request->hasFile('photo') && $request->file('photo')->isValid()) {
            $userData['photo'] = $request->file('photo')->store('photos', 'public');
        }

        $user = User::create($userData);

        $user->assignRole('customer');

        event(new Registered($user));

        Auth::login($user);

        return redirect(route('dashboard', absolute: false));
    }
}
