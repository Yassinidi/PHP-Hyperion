<?php

namespace App\Http\Controllers;

use Illuminate\Http\Request;
use Illuminate\Http\JsonResponse;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Auth;
use App\Models\User;

class AuthController extends Controller
{
    /**
     * Get the authenticated user.
     */
    public function me(Request $request): JsonResponse
    {
        $user = Auth::user();

        $cookieUserId = $_COOKIE['blog_user'] ?? $request->cookie('blog_user');
        if (!$user && $cookieUserId) {
            $user = User::find((int)$cookieUserId);
        }

        if ($user) {
            return response()->json([
                'authenticated' => true,
                'user' => [
                    'id' => $user->id,
                    'name' => $user->name,
                    'email' => $user->email,
                    'bio' => $user->bio,
                    'avatar_url' => $user->getAvatarUrl(),
                ],
            ]);
        }

        return response()->json([
            'authenticated' => false,
            'user' => null,
        ]);
    }

    /**
     * Handle user registration.
     */
    public function register(Request $request): JsonResponse
    {
        $name = trim($request->input('name', ''));
        $email = trim($request->input('email', ''));
        $password = trim($request->input('password', ''));
        $bio = trim($request->input('bio', 'Writer & Software Engineer'));

        if (empty($name) || empty($email) || empty($password)) {
            return response()->json(['error' => 'Name, email, and password are required.'], 422);
        }

        if (User::where('email', $email)->exists()) {
            return response()->json(['error' => 'Email address is already in use.'], 409);
        }

        $user = User::create([
            'name' => $name,
            'email' => $email,
            'password' => Hash::make($password),
            'bio' => $bio,
        ]);

        Auth::login($user);

        return response()->json([
            'status' => 'success',
            'message' => 'Registered successfully',
            'user' => [
                'id' => $user->id,
                'name' => $user->name,
                'email' => $user->email,
                'bio' => $user->bio,
                'avatar_url' => $user->getAvatarUrl(),
            ],
        ], 201)->cookie('blog_user', $user->id, 60 * 24 * 7, '/', null, false, true);
    }

    /**
     * Handle user authentication.
     */
    public function login(Request $request): JsonResponse
    {
        $email = trim($request->input('email', ''));
        $password = trim($request->input('password', ''));

        if (empty($email) || empty($password)) {
            return response()->json(['error' => 'Email and password are required.'], 422);
        }

        $user = User::where('email', $email)->first();

        if (!$user || !Hash::check($password, $user->password)) {
            return response()->json(['error' => 'Invalid email or password credentials.'], 401);
        }

        Auth::login($user);

        return response()->json([
            'status' => 'success',
            'message' => 'Logged in successfully',
            'user' => [
                'id' => $user->id,
                'name' => $user->name,
                'email' => $user->email,
                'bio' => $user->bio,
                'avatar_url' => $user->getAvatarUrl(),
            ],
        ])->cookie('blog_user', $user->id, 60 * 24 * 7, '/', null, false, true);
    }

    /**
     * Handle user logout.
     */
    public function logout(Request $request): JsonResponse
    {
        Auth::logout();

        return response()->json([
            'status' => 'success',
            'message' => 'Logged out successfully',
        ])->withoutCookie('blog_user');
    }

    /**
     * Handle avatar profile photo upload.
     */
    public function updatePhoto(Request $request): JsonResponse
    {
        $user = Auth::user();
        if (!$user && ($userId = $request->cookie('blog_user'))) {
            $user = User::find($userId);
        }

        if (!$user) {
            return response()->json(['error' => 'Unauthorized.'], 401);
        }

        if ($request->hasFile('photo') && $request->file('photo')->isValid()) {
            $path = $request->file('photo')->store('avatars', 'public');
            $user->profile_photo = $path;
            $user->save();

            return response()->json([
                'status' => 'success',
                'avatar_url' => '/storage/' . $path,
            ]);
        }

        return response()->json(['error' => 'Photo upload failed.'], 400);
    }
}
