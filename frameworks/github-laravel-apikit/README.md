# Laravel API Kit

A production-ready, API-only Laravel 13 starter kit following the 2025-2026 REST API ecosystem best practices. No frontend dependencies - purely headless API for mobile apps, SPAs, or microservices.

[![PHP Version](https://img.shields.io/badge/PHP-8.4%2B-blue)](https://php.net)
[![Laravel Version](https://img.shields.io/badge/Laravel-13.x-red)](https://laravel.com)
[![License](https://img.shields.io/badge/License-MIT-green)](LICENSE)

## Features

- **API-Only** - No Blade, Vite, or frontend assets
- **Token Authentication** - Laravel Sanctum for mobile/SPA auth
- **Email Verification** - Built-in email verification flow with signed URLs
- **Password Reset** - Secure password reset with token-based flow
- **API Versioning** - URI-based versioning with deprecation support via [grazulex/laravel-apiroute](https://github.com/Grazulex/laravel-apiroute)
- **Query Building** - Filtering, sorting, includes via [spatie/laravel-query-builder](https://github.com/spatie/laravel-query-builder)
- **Data Objects** - Type-safe DTOs via [spatie/laravel-data](https://github.com/spatie/laravel-data)
- **Auto Documentation** - Zero-annotation OpenAPI 3.1 via [dedoc/scramble](https://github.com/dedoc/scramble)
- **Modern Testing** - Pest PHP with Laravel HTTP testing
- **Code Quality** - PHPStan (max level), Rector, and Pint with strict rules
- **Rate Limiting** - Configurable per-route rate limiters
- **Reusable Middleware** - ForceJsonResponse, LogApiRequests, EnsureEmailVerified
- **Standardized Responses** - Consistent JSON response format
- **Optional: API Idempotency** - RFC-compliant idempotency via [grazulex/laravel-api-idempotency](https://github.com/Grazulex/laravel-api-idempotency)
- **Optional: Smart Rate Limiting** - Plan-aware throttling with quotas via [grazulex/laravel-api-throttle-smart](https://github.com/Grazulex/laravel-api-throttle-smart)

## Requirements

- Docker & Docker Compose
- Or: PHP 8.4+, Composer 2.x

## Quick Start

### With Docker (Recommended)

```bash
# Clone the repository
git clone https://github.com/grazulex/laravel-api-kit.git
cd laravel-api-kit

# Copy environment file
cp .env.example .env

# Build and start containers
docker compose build
docker compose up -d

# Install dependencies
docker compose run --rm app composer install

# Generate application key
docker compose run --rm app php artisan key:generate

# Run migrations
docker compose run --rm app php artisan migrate

# Run tests to verify installation
docker compose run --rm app ./vendor/bin/pest
```

### Without Docker

```bash
# Clone and install
git clone https://github.com/grazulex/laravel-api-kit.git
cd laravel-api-kit
composer install

# Configure
cp .env.example .env
php artisan key:generate

# Database (SQLite by default)
touch database/database.sqlite
php artisan migrate

# Verify
./vendor/bin/pest
```

## API Documentation

Once running, access the auto-generated documentation:

- **Swagger UI**: [http://localhost:8080/docs/api](http://localhost:8080/docs/api)
- **OpenAPI JSON**: [http://localhost:8080/docs/api.json](http://localhost:8080/docs/api.json)

## Authentication

This kit uses **Laravel Sanctum** with token-based authentication (ideal for mobile apps and third-party API consumers).

### Register a New User

```bash
curl -X POST http://localhost:8080/api/v1/register \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{
    "name": "John Doe",
    "email": "john@example.com",
    "password": "password123",
    "password_confirmation": "password123"
  }'
```

**Response:**
```json
{
  "success": true,
  "message": "User registered successfully. Please check your email to verify your account.",
  "data": {
    "user": {
      "id": 1,
      "name": "John Doe",
      "email": "john@example.com",
      "email_verified_at": null,
      "created_at": "2025-01-15T10:30:00+00:00"
    },
    "token": "1|abc123..."
  }
}
```

### Login

```bash
curl -X POST http://localhost:8080/api/v1/login \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{
    "email": "john@example.com",
    "password": "password123"
  }'
```

### Using the Token

Include the token in the `Authorization` header for protected routes:

```bash
curl -X GET http://localhost:8080/api/v1/me \
  -H "Authorization: Bearer 1|abc123..." \
  -H "Accept: application/json"
```

### Logout

```bash
curl -X POST http://localhost:8080/api/v1/logout \
  -H "Authorization: Bearer 1|abc123..." \
  -H "Accept: application/json"
```

### Email Verification

After registration, users receive a verification email. The kit integrates with Laravel's `MustVerifyEmail` contract.

**Verify Email (via signed URL from email):**
```bash
curl -X POST "http://localhost:8080/api/v1/email/verify/{id}/{hash}?signature=..." \
  -H "Authorization: Bearer 1|abc123..." \
  -H "Accept: application/json"
```

**Resend Verification Email:**
```bash
curl -X POST http://localhost:8080/api/v1/email/resend \
  -H "Authorization: Bearer 1|abc123..." \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{"email": "john@example.com"}'
```

**Response:**
```json
{
  "success": true,
  "message": "Verification email sent successfully",
  "data": null
}
```

### Password Reset

**Request Password Reset Link:**
```bash
curl -X POST http://localhost:8080/api/v1/forgot-password \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{"email": "john@example.com"}'
```

**Response:**
```json
{
  "success": true,
  "message": "Password reset link sent to your email",
  "data": null
}
```

**Reset Password (with token from email):**
```bash
curl -X POST http://localhost:8080/api/v1/reset-password \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{
    "email": "john@example.com",
    "token": "reset-token-from-email",
    "password": "newpassword123",
    "password_confirmation": "newpassword123"
  }'
```

**Response:**
```json
{
  "success": true,
  "message": "Password reset successfully",
  "data": null
}
```

> **Note:** After a successful password reset, all user tokens are revoked for security.

## API Endpoints

### Version 1 (`/api/v1`)

| Method | Endpoint                     | Auth | Description                   | Rate Limit |
|--------|------------------------------|------|-------------------------------|------------|
| POST   | /register                    | No   | Register new user             | 5/min      |
| POST   | /login                       | No   | Get authentication token      | 5/min      |
| POST   | /logout                      | Yes  | Revoke current token          | 120/min    |
| GET    | /me                          | Yes  | Get current user profile      | 120/min    |
| POST   | /email/verify/{id}/{hash}    | Yes  | Verify email address          | 120/min    |
| POST   | /email/resend                | Yes  | Resend verification email     | 6/min      |
| POST   | /forgot-password             | No   | Request password reset link   | 6/min      |
| POST   | /reset-password              | No   | Reset password with token     | 6/min      |

## Response Format

All API responses follow a consistent format:

### Success Response

```json
{
  "success": true,
  "message": "Operation successful",
  "data": {
    // Response data here
  }
}
```

### Error Response

```json
{
  "success": false,
  "message": "Error description",
  "errors": {
    "field": ["Validation error message"]
  }
}
```

### HTTP Status Codes

| Code | Description |
|------|-------------|
| 200  | Success |
| 201  | Resource created |
| 204  | No content |
| 400  | Bad request |
| 401  | Unauthorized |
| 403  | Forbidden |
| 404  | Not found |
| 422  | Validation error |
| 429  | Too many requests |
| 500  | Server error |

## Project Structure

```
laravel-api-kit/
├── app/
│   ├── Actions/                    # Single-purpose action classes
│   ├── DTOs/                       # Data Transfer Objects (spatie/laravel-data)
│   ├── Http/
│   │   ├── Controllers/
│   │   │   └── Api/
│   │   │       ├── ApiController.php      # Base controller with ApiResponse
│   │   │       └── V1/                    # Version 1 controllers
│   │   │           └── AuthController.php
│   │   ├── Requests/
│   │   │   └── Api/V1/                    # Form Requests per version
│   │   │       ├── LoginRequest.php
│   │   │       └── RegisterRequest.php
│   │   └── Resources/                     # API Resources
│   │       └── UserResource.php
│   ├── Models/
│   │   └── User.php                       # With HasApiTokens trait
│   ├── Providers/
│   │   └── AppServiceProvider.php         # Rate limiting config
│   ├── Services/                          # Business logic services
│   └── Traits/
│       └── ApiResponse.php                # Standardized responses
├── config/
│   ├── apiroute.php                       # API versioning config
│   ├── cors.php                           # CORS settings
│   ├── sanctum.php                        # Token auth config
│   └── scramble.php                       # API docs config
├── routes/
│   ├── api.php                            # API routes entry point
│   └── api/
│       └── v1.php                         # Version 1 routes
├── tests/
│   └── Feature/Api/V1/
│       └── AuthTest.php                   # Authentication tests
├── docker-compose.yml
├── Dockerfile
└── CLAUDE.md                              # AI assistant instructions
```

## API Versioning

This kit uses [grazulex/laravel-apiroute](https://github.com/Grazulex/laravel-apiroute) v2.x for API versioning with support for:

- **URI Path** (default): `/api/v1/users`, `/api/v2/users`
- **Header**: `X-API-Version: 2`
- **Query Parameter**: `?api_version=2`
- **Accept Header**: `Accept: application/vnd.api.v2+json`

### Adding a New API Version

1. Create controllers in `app/Http/Controllers/Api/V2/`
2. Create requests in `app/Http/Requests/Api/V2/`
3. Create route file `routes/api/v2.php`:

```php
<?php

use App\Http\Controllers\Api\V2\AuthController;
use Illuminate\Support\Facades\Route;

Route::post('register', [AuthController::class, 'register']);
// ... more routes
```

4. Update `config/apiroute.php`:

```php
'versions' => [
    'v1' => [
        'routes' => base_path('routes/api/v1.php'),
        'status' => 'deprecated',
        'deprecated_at' => '2025-06-01',
        'sunset_at' => '2025-12-01',
        'successor' => 'v2',
    ],
    'v2' => [
        'routes' => base_path('routes/api/v2.php'),
        'status' => 'active',
    ],
],
```

### Deprecation Headers

When accessing deprecated versions, responses include RFC-compliant headers:

```http
Deprecation: Sun, 01 Jun 2025 00:00:00 GMT
Sunset: Mon, 01 Dec 2025 00:00:00 GMT
Link: </api/v2>; rel="successor-version"
```

## Query Building

Use [spatie/laravel-query-builder](https://spatie.be/docs/laravel-query-builder) for filtering, sorting, and including relationships:

```php
use Spatie\QueryBuilder\QueryBuilder;
use Spatie\QueryBuilder\AllowedFilter;

// In your controller
$users = QueryBuilder::for(User::class)
    ->allowedFilters([
        'name',
        'email',
        AllowedFilter::exact('id'),
        AllowedFilter::scope('active'),
    ])
    ->allowedSorts(['name', 'created_at'])
    ->allowedIncludes(['posts', 'comments'])
    ->paginate();

return UserResource::collection($users);
```

**Request examples:**
```
GET /api/v1/users?filter[name]=john
GET /api/v1/users?sort=-created_at
GET /api/v1/users?include=posts,comments
GET /api/v1/users?filter[name]=john&sort=name&include=posts
```

## Data Transfer Objects

Use [spatie/laravel-data](https://spatie.be/docs/laravel-data) for type-safe DTOs:

```php
// app/DTOs/UserData.php
use Spatie\LaravelData\Data;

class UserData extends Data
{
    public function __construct(
        public string $name,
        public string $email,
        public ?string $password = null,
    ) {}
}

// In controller - validates and transforms automatically
public function store(UserData $data): JsonResponse
{
    $user = User::create($data->toArray());
    return $this->created(UserResource::make($user));
}
```

## Rate Limiting

Configured in `app/Providers/AppServiceProvider.php`:

| Limiter | Limit | Use Case |
|---------|-------|----------|
| `api` | 60/min | Default for all API routes |
| `auth` | 5/min | Login/register (brute force protection) |
| `authenticated` | 120/min | Logged-in users |

### Applying Rate Limiters

```php
// In routes/api.php
Route::middleware('throttle:auth')->group(function () {
    Route::post('login', [AuthController::class, 'login']);
    Route::post('register', [AuthController::class, 'register']);
});

Route::middleware(['auth:sanctum', 'throttle:authenticated'])->group(function () {
    // Protected routes with higher limits
});
```

### Rate Limit Headers

Responses include rate limit information:

```http
X-RateLimit-Limit: 60
X-RateLimit-Remaining: 59
Retry-After: 60  # When limit exceeded
```

## Optional Packages

The following packages are **suggested** (not required) and can be installed individually to extend the kit's capabilities. They are fully opt-in and will not affect existing behavior.

### API Idempotency

[grazulex/laravel-api-idempotency](https://github.com/Grazulex/laravel-api-idempotency) provides RFC-compliant idempotency for your API endpoints. It prevents duplicate operations when clients retry requests (critical for payments, order creation, etc.).

**Install:**
```bash
composer require grazulex/laravel-api-idempotency
```

**Publish config (optional):**
```bash
php artisan vendor:publish --tag="api-idempotency-config"
```

**Usage — apply the middleware to mutation routes:**
```php
// routes/api/v1.php
Route::middleware(['auth:sanctum', 'throttle:authenticated'])->group(function () {
    Route::post('orders', [OrderController::class, 'store'])
        ->middleware('idempotent');

    Route::post('payments', [PaymentController::class, 'store'])
        ->middleware('idempotent:required'); // Require Idempotency-Key header
});
```

**Client-side — include the `Idempotency-Key` header:**
```bash
curl -X POST http://localhost:8080/api/v1/orders \
  -H "Authorization: Bearer 1|abc123..." \
  -H "Idempotency-Key: order_unique_key_123" \
  -H "Content-Type: application/json" \
  -d '{"product_id": 1, "quantity": 2}'
```

> **Attention:**
> - Only apply the `idempotent` middleware to mutation routes (POST, PUT, PATCH). GET requests are naturally idempotent.
> - The default storage driver is `cache`. For production with multiple servers, use the `redis` or `database` driver.
> - Keys are scoped per user by default. Two different users can use the same key without conflict.

---

### Smart Rate Limiting

[grazulex/laravel-api-throttle-smart](https://github.com/Grazulex/laravel-api-throttle-smart) provides plan-aware rate limiting with quotas, multiple algorithms (fixed window, sliding window, token bucket), and multi-tenant support. Ideal for SaaS APIs with subscription tiers.

**Install:**
```bash
composer require grazulex/laravel-api-throttle-smart
```

**Publish config:**
```bash
php artisan vendor:publish --tag="throttle-smart-config"
```

**Usage — apply to routes where plan-based limiting is needed:**
```php
// routes/api/v1.php
Route::middleware(['auth:sanctum', 'throttle.smart'])->group(function () {
    Route::apiResource('posts', PostController::class);
});
```

> **Attention:**
> - This package **coexists** with Laravel's built-in `throttle:` middleware. You do not need to remove the existing rate limiters.
> - If you want to **replace** the native throttle on specific routes, swap `throttle:authenticated` with `throttle.smart` on those routes only.
> - Do **not** apply both `throttle:authenticated` and `throttle.smart` on the same route group — choose one per group to avoid double rate limiting.
> - The default driver is `cache`. For production, `redis` is recommended for performance and distributed consistency.
> - Configure your subscription plans in `config/throttle-smart.php` to match your business model (Free, Pro, Enterprise, etc.).

---

## Middleware

The kit includes three production-ready middleware patterns that you can apply to your routes as needed.

### Available Middleware

| Alias | Class | Description |
|-------|-------|-------------|
| `force.json` | `ForceJsonResponse` | Ensures all responses are JSON formatted |
| `log.api` | `LogApiRequests` | Logs API requests with timing information |
| `verified` | `EnsureEmailVerified` | Requires verified email to access route |

### ForceJsonResponse

Automatically sets `Accept: application/json` header and converts non-JSON responses to JSON format.

```php
Route::middleware('force.json')->group(function () {
    // All responses will be JSON
});
```

### LogApiRequests

Logs API requests with detailed information and adds `X-Response-Time` header to responses.

**Logged data:** timestamp, method, URL, IP, user ID, status code, duration (ms), user agent.

**Enable logging via environment:**
```env
APP_LOG_API_REQUESTS=true
```

```php
Route::middleware('log.api')->group(function () {
    // Requests will be logged
});
```

### EnsureEmailVerified

Protects routes that require a verified email address. Returns 403 if email is not verified.

```php
Route::middleware(['auth:sanctum', 'verified'])->group(function () {
    // Only users with verified emails can access
});
```

**Response when email not verified:**
```json
{
  "success": false,
  "message": "Your email address is not verified. Please verify your email to continue."
}
```

## Testing

This kit uses [Pest PHP](https://pestphp.com/) for testing:

```bash
# Run all tests
docker compose run --rm app ./vendor/bin/pest

# Run specific test file
docker compose run --rm app ./vendor/bin/pest tests/Feature/Api/V1/AuthTest.php

# Run with coverage
docker compose run --rm app ./vendor/bin/pest --coverage

# Run in parallel
docker compose run --rm app ./vendor/bin/pest --parallel
```

### Writing Tests

```php
// tests/Feature/Api/V1/UserTest.php
use App\Models\User;
use Illuminate\Foundation\Testing\RefreshDatabase;

uses(RefreshDatabase::class);

it('lists users for authenticated user', function () {
    $user = User::factory()->create();
    $token = $user->createToken('test')->plainTextToken;

    User::factory()->count(5)->create();

    $response = $this->withHeader('Authorization', "Bearer {$token}")
        ->getJson('/api/v1/users');

    $response->assertStatus(200)
        ->assertJsonStructure([
            'success',
            'data' => [
                '*' => ['id', 'name', 'email']
            ]
        ]);
});

it('requires authentication', function () {
    $this->getJson('/api/v1/users')
        ->assertStatus(401);
});
```

## Code Quality

This kit includes strict code quality tools configured following [nunomaduro/laravel-starter-kit](https://github.com/nunomaduro/laravel-starter-kit) standards.

### Tools

| Tool | Purpose | Config |
|------|---------|--------|
| [PHPStan](https://phpstan.org/) + [Larastan](https://github.com/larastan/larastan) | Static analysis (level max) | `phpstan.neon` |
| [Rector](https://getrector.com/) | Automated refactoring | `rector.php` |
| [Pint](https://laravel.com/docs/pint) | Code style (strict rules) | `pint.json` |

### Composer Scripts

```bash
# Apply all fixes (Rector + Pint)
composer lint

# Check without fixing (CI mode)
composer test:lint

# Static analysis only
composer test:types

# Unit tests only
composer test:unit

# Full test suite (lint + types + unit)
composer test
```

### With Docker

```bash
docker compose exec app composer lint
docker compose exec app composer test
```

### Strict Rules Applied

- `declare(strict_types=1)` on all files
- `final` classes by default
- Type declarations enforced
- Dead code removal
- Early returns
- Strict comparisons

### GitHub Actions

Tests run automatically on push/PR to `main` via `.github/workflows/tests.yml`.

## Development Commands

```bash
# List all routes
docker compose run --rm app php artisan route:list

# Clear all caches
docker compose run --rm app php artisan optimize:clear

# Generate IDE helper files (if using Laravel IDE Helper)
docker compose run --rm app php artisan ide-helper:generate
docker compose run --rm app php artisan ide-helper:models -N

# Export OpenAPI spec to file
docker compose run --rm app php artisan scramble:export
```

## Environment Configuration

Key `.env` variables:

```env
# Application
APP_NAME="Laravel API Kit"
APP_ENV=local
APP_DEBUG=true
APP_URL=http://localhost:8080

# Database (SQLite for development)
DB_CONNECTION=sqlite
DB_DATABASE=/var/www/database/database.sqlite

# For MySQL/PostgreSQL
# DB_CONNECTION=mysql
# DB_HOST=mysql
# DB_PORT=3306
# DB_DATABASE=laravel_api_kit
# DB_USERNAME=laravel
# DB_PASSWORD=secret

# Sanctum
SANCTUM_STATEFUL_DOMAINS=localhost,localhost:3000,127.0.0.1

# API Versioning
API_VERSION_STRATEGY=uri
API_DEFAULT_VERSION=latest

# Rate Limiting
API_RATE_LIMIT=60

# Documentation
API_DOCS_URL=http://localhost:8080/docs/api
```

## Deployment

### Production Checklist

- [ ] Set `APP_ENV=production` and `APP_DEBUG=false`
- [ ] Configure proper database (MySQL/PostgreSQL)
- [ ] Set `APP_URL` to your production URL
- [ ] Configure `SANCTUM_STATEFUL_DOMAINS` for your frontend domains
- [ ] Review and tighten CORS settings in `config/cors.php`
- [ ] Set up proper rate limiting for production load
- [ ] Configure caching (Redis recommended)
- [ ] Set up queue worker for background jobs
- [ ] Enable HTTPS and update URLs

### Docker Production

```dockerfile
# Example production Dockerfile additions
FROM php:8.4-fpm-alpine

# Install opcache for performance
RUN docker-php-ext-install opcache

# Production PHP settings
COPY docker/php/opcache.ini /usr/local/etc/php/conf.d/
COPY docker/php/php.ini /usr/local/etc/php/conf.d/
```

## Extending the Kit

### Adding a New Resource (CRUD Example)

1. **Create Model & Migration:**
```bash
docker compose run --rm app php artisan make:model Post -m
```

2. **Create Controller:**
```php
// app/Http/Controllers/Api/V1/PostController.php
namespace App\Http\Controllers\Api\V1;

use App\Http\Controllers\Api\ApiController;
use App\Http\Resources\PostResource;
use App\Models\Post;
use Spatie\QueryBuilder\QueryBuilder;

class PostController extends ApiController
{
    public function index()
    {
        $posts = QueryBuilder::for(Post::class)
            ->allowedFilters(['title', 'status'])
            ->allowedSorts(['title', 'created_at'])
            ->allowedIncludes(['author', 'comments'])
            ->paginate();

        return $this->success(PostResource::collection($posts));
    }

    public function show(Post $post)
    {
        return $this->success(new PostResource($post));
    }

    // ... store, update, destroy methods
}
```

3. **Create Resource:**
```php
// app/Http/Resources/PostResource.php
namespace App\Http\Resources;

use Illuminate\Http\Resources\Json\JsonResource;

class PostResource extends JsonResource
{
    public function toArray($request): array
    {
        return [
            'id' => $this->id,
            'title' => $this->title,
            'content' => $this->content,
            'author' => new UserResource($this->whenLoaded('author')),
            'created_at' => $this->created_at?->toIso8601String(),
        ];
    }
}
```

4. **Add Routes:**
```php
// routes/api/v1.php
Route::middleware('auth:sanctum')->group(function () {
    // ... existing routes
    Route::apiResource('posts', PostController::class);
});
```

5. **Create Tests:**
```php
// tests/Feature/Api/V1/PostTest.php
uses(RefreshDatabase::class);

it('lists posts', function () {
    $user = User::factory()->create();
    Post::factory()->count(3)->create();

    $this->actingAs($user)
        ->getJson('/api/v1/posts')
        ->assertStatus(200)
        ->assertJsonCount(3, 'data');
});
```

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is open-sourced software licensed under the [MIT license](LICENSE).

## Credits

- [Laravel](https://laravel.com) - The PHP Framework
- [Laravel Sanctum](https://laravel.com/docs/sanctum) - API Token Authentication
- [grazulex/laravel-apiroute](https://github.com/Grazulex/laravel-apiroute) - API Versioning
- [spatie/laravel-query-builder](https://github.com/spatie/laravel-query-builder) - Query Building
- [spatie/laravel-data](https://github.com/spatie/laravel-data) - Data Transfer Objects
- [dedoc/scramble](https://github.com/dedoc/scramble) - API Documentation
- [grazulex/laravel-api-idempotency](https://github.com/Grazulex/laravel-api-idempotency) - API Idempotency (optional)
- [grazulex/laravel-api-throttle-smart](https://github.com/Grazulex/laravel-api-throttle-smart) - Smart Rate Limiting (optional)
- [Pest PHP](https://pestphp.com) - Testing Framework

## Support

- [Documentation](https://github.com/grazulex/laravel-api-kit/wiki)
- [Issues](https://github.com/grazulex/laravel-api-kit/issues)
- [Discussions](https://github.com/grazulex/laravel-api-kit/discussions)
