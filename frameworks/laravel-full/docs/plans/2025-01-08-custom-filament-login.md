# Custom Filament Admin Login Page Implementation Plan

**Goal:** Customize the Filament admin login page (`/admin/login`) to match the existing frontend auth design while preserving all login logic.

**Architecture:** Create a custom Login page class extending Filament's base Login, override the view to use a custom Blade template that matches the frontend auth styling (radial gradient background, centered card, green buttons, rounded inputs with icons).

**Tech Stack:** Laravel 12, Filament 3.3, Blade, Tailwind CSS

---

## Task 1: Create Custom Login Page Class

**Files:**

- Create: `app/Filament/Pages/Auth/Login.php`

**Step 1: Create the custom login page class**

```php
<?php

namespace App\Filament\Pages\Auth;

use Filament\Auth\Pages\Login as BaseLogin;

class Login extends BaseLogin
{
    protected static string $view = 'filament.pages.auth.login';
}
```

**Step 2: Verify file was created**

Run: `ls -la app/Filament/Pages/Auth/`
Expected: Shows `Login.php`

---

## Task 2: Create Custom Blade View

**Files:**

- Create: `resources/views/filament/pages/auth/login.blade.php`

**Step 1: Create the Blade template matching frontend auth design**

Template should include:

- Radial gradient dot background (#FAFAFA base)
- Centered white card (max-width: 500px, rounded-20px, shadow)
- Tedja logo with link to homepage
- Email input with sms icon
- Password input with lock icon + visibility toggle
- "Remember me" checkbox
- Green "Sign In" button (#CEF27F)
- "Back to homepage" link
- CSRF token and form submission to Filament's login route
- Error message display

**Step 2: Verify file was created**

Run: `ls -la resources/views/filament/pages/auth/`
Expected: Shows `login.blade.php`

---

## Task 3: Update AdminPanelProvider

**Files:**

- Modify: `app/Providers/Filament/AdminPanelProvider.php`

**Step 1: Add import and update login configuration**

Add import:

```php
use App\Filament\Pages\Auth\Login;
```

Change:

```php
->login()
```

To:

```php
->login(Login::class)
```

**Step 2: Verify changes**

Run: `grep -n "login" app/Providers/Filament/AdminPanelProvider.php`
Expected: Shows `->login(Login::class)`

---

## Task 4: Test the Implementation

**Step 1: Clear cache**

Run: `php artisan optimize:clear`

**Step 2: Test login page loads**

Visit: `http://127.0.0.1:8000/admin/login`
Expected: Custom styled login page appears with radial gradient background

**Step 3: Test login functionality**

Try logging in with valid credentials
Expected: Successfully logs in and redirects to admin dashboard

**Step 4: Test validation**

Try logging in with invalid credentials
Expected: Shows error messages properly styled

---

## Task 5: Commit Changes

```bash
git add app/Filament/Pages/Auth/Login.php resources/views/filament/pages/auth/login.blade.php app/Providers/Filament/AdminPanelProvider.php
git commit -m "update(admin): customize Filament login page to match frontend auth design

- Create custom Login page class extending Filament's base Login
- Create custom Blade view with radial gradient background and centered card
- Match frontend auth styling: green buttons, rounded inputs with icons
- Add password visibility toggle and remember me checkbox
- Update AdminPanelProvider to use custom login page
- Preserve all original login logic and validation"
```
