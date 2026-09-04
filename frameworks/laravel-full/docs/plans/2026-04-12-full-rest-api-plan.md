# Tedja Full REST API Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create a comprehensive REST API layer for Tedja with ~20 endpoints, full Swagger/OpenAPI 3.1 annotations, and tests — without modifying any existing Services, Models, or web routes.

**Architecture:** New thin API controllers in `app/Http/Controllers/Api/` that delegate to existing Services. Each controller returns JSON and has `#[OA\*]` attributes for Swagger docs. Routes registered in `routes/api.php`.

**Tech Stack:** Laravel 12, Sanctum (token auth), L5-Swagger (OpenAPI 3.1), PHPUnit

**Constraint:** ZERO changes to `app/Services/*`, `app/Models/*`, or `routes/web.php`. Only new files + `routes/api.php` updates.

---

## Task 1: Fix HouseController Filter Key Bug + Add Interests Endpoint

**Files:**

- Modify: `app/Http/Controllers/Api/HouseController.php`
- Modify: `routes/api.php`

**Problem:** `HouseService::searchHouses()` expects `$filters['city']` and `$filters['category']`, but the API passes `city_id` and `category_id`. Filters silently do nothing.

**Step 1: Fix the filter key mapping in HouseController::index()**

Change the `index()` method to map keys correctly before passing to service:

```php
public function index(Request $request): JsonResponse
{
    $filters = [
        'city' => $request->input('city_id'),
        'category' => $request->input('category_id'),
    ];
    $result = $this->houseService->searchHouses(array_filter($filters));

    return response()->json($result['houses']);
}
```

**Step 2: Add interests endpoint to HouseController**

Add a new method that returns bank interest rates for a specific house:

```php
#[OA\Get(
    path: '/api/houses/{slug}/interests',
    operationId: 'getHouseInterests',
    summary: 'Get bank interest rates for a house',
    description: 'Returns all available bank interest rates and durations for a specific property.',
    tags: ['Houses'],
    parameters: [
        new OA\Parameter(name: 'slug', description: 'House slug', in: 'path', required: true, schema: new OA\Schema(type: 'string')),
    ],
    responses: [
        new OA\Response(
            response: 200,
            description: 'List of interest rates',
            content: new OA\JsonContent(
                type: 'array',
                items: new OA\Items(
                    properties: [
                        new OA\Property(property: 'id', type: 'integer', example: 1),
                        new OA\Property(property: 'interest', type: 'number', description: 'Annual interest rate %', example: 7.5),
                        new OA\Property(property: 'duration', type: 'integer', description: 'Loan duration in years', example: 20),
                        new OA\Property(property: 'bank', properties: [
                            new OA\Property(property: 'id', type: 'integer', example: 1),
                            new OA\Property(property: 'name', type: 'string', example: 'BCA'),
                        ], type: 'object'),
                    ],
                    type: 'object'
                )
            )
        ),
        new OA\Response(response: 404, description: 'House not found'),
    ]
)]
public function interests(string $slug): JsonResponse
{
    $house = \App\Models\House::where('slug', $slug)->firstOrFail();
    $interests = $house->interests()->with('bank')->get();

    return response()->json($interests);
}
```

**Step 3: Register new route in `routes/api.php`**

Add after the existing houses routes:

```php
Route::get('/houses/{slug}/interests', [HouseController::class, 'interests']);
```

---

## Task 2: Create Master Data API Controllers (Categories, Cities, Banks, Facilities)

**Files:**

- Create: `app/Http/Controllers/Api/CategoryController.php`
- Create: `app/Http/Controllers/Api/CityController.php`
- Create: `app/Http/Controllers/Api/BankController.php`
- Create: `app/Http/Controllers/Api/FacilityController.php`
- Modify: `routes/api.php`

**Step 1: Create CategoryController**

```php
<?php

namespace App\Http\Controllers\Api;

use App\Http\Controllers\Controller;
use App\Models\Category;
use Illuminate\Http\JsonResponse;
use OpenApi\Attributes as OA;

class CategoryController extends Controller
{
    #[OA\Get(
        path: '/api/categories',
        operationId: 'listCategories',
        summary: 'List all property categories',
        tags: ['Categories'],
        responses: [
            new OA\Response(
                response: 200,
                description: 'List of categories',
                content: new OA\JsonContent(
                    type: 'array',
                    items: new OA\Items(
                        properties: [
                            new OA\Property(property: 'id', type: 'integer', example: 1),
                            new OA\Property(property: 'name', type: 'string', example: 'rumah'),
                            new OA\Property(property: 'slug', type: 'string', example: 'rumah'),
                            new OA\Property(property: 'photo', type: 'string', nullable: true),
                        ],
                        type: 'object'
                    )
                )
            ),
        ]
    )]
    public function index(): JsonResponse
    {
        return response()->json(Category::latest()->get());
    }

    #[OA\Get(
        path: '/api/categories/{slug}',
        operationId: 'getCategory',
        summary: 'Get category with its houses',
        tags: ['Categories'],
        parameters: [
            new OA\Parameter(name: 'slug', description: 'Category slug', in: 'path', required: true, schema: new OA\Schema(type: 'string')),
        ],
        responses: [
            new OA\Response(response: 200, description: 'Category detail with houses'),
            new OA\Response(response: 404, description: 'Category not found'),
        ]
    )]
    public function show(string $slug): JsonResponse
    {
        $category = Category::where('slug', $slug)->with(['houses'])->firstOrFail();

        return response()->json($category);
    }
}
```

**Step 2: Create CityController** (same pattern as CategoryController but for City model, City has no `houses()` relationship defined — just return city data)

```php
<?php

namespace App\Http\Controllers\Api;

use App\Http\Controllers\Controller;
use App\Models\City;
use App\Models\House;
use Illuminate\Http\JsonResponse;
use OpenApi\Attributes as OA;

class CityController extends Controller
{
    #[OA\Get(
        path: '/api/cities',
        operationId: 'listCities',
        summary: 'List all cities',
        tags: ['Cities'],
        responses: [
            new OA\Response(
                response: 200,
                description: 'List of cities',
                content: new OA\JsonContent(
                    type: 'array',
                    items: new OA\Items(
                        properties: [
                            new OA\Property(property: 'id', type: 'integer', example: 1),
                            new OA\Property(property: 'name', type: 'string', example: 'jakarta'),
                            new OA\Property(property: 'slug', type: 'string', example: 'jakarta'),
                            new OA\Property(property: 'photo', type: 'string', nullable: true),
                        ],
                        type: 'object'
                    )
                )
            ),
        ]
    )]
    public function index(): JsonResponse
    {
        return response()->json(City::latest()->get());
    }

    #[OA\Get(
        path: '/api/cities/{slug}',
        operationId: 'getCity',
        summary: 'Get city detail with houses',
        tags: ['Cities'],
        parameters: [
            new OA\Parameter(name: 'slug', description: 'City slug', in: 'path', required: true, schema: new OA\Schema(type: 'string')),
        ],
        responses: [
            new OA\Response(response: 200, description: 'City detail'),
            new OA\Response(response: 404, description: 'City not found'),
        ]
    )]
    public function show(string $slug): JsonResponse
    {
        $city = City::where('slug', $slug)->firstOrFail();
        $houses = House::where('city_id', $city->id)->get();

        return response()->json([
            'city' => $city,
            'houses' => $houses,
        ]);
    }
}
```

**Step 3: Create BankController**

```php
<?php

namespace App\Http\Controllers\Api;

use App\Http\Controllers\Controller;
use App\Models\Bank;
use Illuminate\Http\JsonResponse;
use OpenApi\Attributes as OA;

class BankController extends Controller
{
    #[OA\Get(
        path: '/api/banks',
        operationId: 'listBanks',
        summary: 'List all banks',
        tags: ['Banks'],
        responses: [
            new OA\Response(
                response: 200,
                description: 'List of banks',
                content: new OA\JsonContent(
                    type: 'array',
                    items: new OA\Items(
                        properties: [
                            new OA\Property(property: 'id', type: 'integer', example: 1),
                            new OA\Property(property: 'name', type: 'string', example: 'BCA'),
                            new OA\Property(property: 'photo', type: 'string', nullable: true),
                        ],
                        type: 'object'
                    )
                )
            ),
        ]
    )]
    public function index(): JsonResponse
    {
        return response()->json(Bank::all());
    }

    #[OA\Get(
        path: '/api/banks/{id}',
        operationId: 'getBank',
        summary: 'Get bank with its interest rates',
        tags: ['Banks'],
        parameters: [
            new OA\Parameter(name: 'id', description: 'Bank ID', in: 'path', required: true, schema: new OA\Schema(type: 'integer')),
        ],
        responses: [
            new OA\Response(response: 200, description: 'Bank detail with interest rates'),
            new OA\Response(response: 404, description: 'Bank not found'),
        ]
    )]
    public function show(int $id): JsonResponse
    {
        $bank = Bank::with('interests')->findOrFail($id);

        return response()->json($bank);
    }
}
```

**Step 4: Create FacilityController**

```php
<?php

namespace App\Http\Controllers\Api;

use App\Http\Controllers\Controller;
use App\Models\Facility;
use Illuminate\Http\JsonResponse;
use OpenApi\Attributes as OA;

class FacilityController extends Controller
{
    #[OA\Get(
        path: '/api/facilities',
        operationId: 'listFacilities',
        summary: 'List all facilities',
        tags: ['Facilities'],
        responses: [
            new OA\Response(
                response: 200,
                description: 'List of facilities',
                content: new OA\JsonContent(
                    type: 'array',
                    items: new OA\Items(
                        properties: [
                            new OA\Property(property: 'id', type: 'integer', example: 1),
                            new OA\Property(property: 'name', type: 'string', example: 'Swimming Pool'),
                            new OA\Property(property: 'photo', type: 'string', nullable: true),
                        ],
                        type: 'object'
                    )
                )
            ),
        ]
    )]
    public function index(): JsonResponse
    {
        return response()->json(Facility::all());
    }
}
```

**Step 5: Register all routes in `routes/api.php`**

Add to the public routes section:

```php
use App\Http\Controllers\Api\BankController;
use App\Http\Controllers\Api\CategoryController;
use App\Http\Controllers\Api\CityController;
use App\Http\Controllers\Api\FacilityController;

// Public API - Master Data
Route::get('/categories', [CategoryController::class, 'index']);
Route::get('/categories/{slug}', [CategoryController::class, 'show']);
Route::get('/cities', [CityController::class, 'index']);
Route::get('/cities/{slug}', [CityController::class, 'show']);
Route::get('/banks', [BankController::class, 'index']);
Route::get('/banks/{id}', [BankController::class, 'show']);
Route::get('/facilities', [FacilityController::class, 'index']);
```

---

## Task 3: Create Mortgage API Controller (Calculate, Submit, List, Detail)

**Files:**

- Create: `app/Http/Controllers/Api/MortgageController.php`
- Modify: `routes/api.php`

This controller wraps `MortgageService` methods as JSON API endpoints.

**Step 1: Create MortgageController**

```php
<?php

namespace App\Http\Controllers\Api;

use App\Http\Controllers\Controller;
use App\Models\MortgageRequest;
use App\Services\MortgageService;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\Auth;
use OpenApi\Attributes as OA;

class MortgageController extends Controller
{
    public function __construct(
        private MortgageService $mortgageService
    ) {}

    #[OA\Post(
        path: '/api/mortgages/calculate',
        operationId: 'calculateMortgage',
        summary: 'Calculate mortgage details without creating a record',
        description: 'Returns monthly payment, total loan, DP, and total cost based on house, interest rate, and down payment percentage.',
        tags: ['Mortgages'],
        requestBody: new OA\RequestBody(
            required: true,
            content: new OA\JsonContent(
                required: ['house_id', 'interest_id', 'dp_percentage'],
                properties: [
                    new OA\Property(property: 'house_id', type: 'integer', example: 1),
                    new OA\Property(property: 'interest_id', type: 'integer', example: 1),
                    new OA\Property(property: 'dp_percentage', type: 'integer', description: 'Down payment percentage (0-100)', example: 20),
                ],
                type: 'object'
            )
        ),
        responses: [
            new OA\Response(
                response: 200,
                description: 'Mortgage calculation result',
                content: new OA\JsonContent(
                    properties: [
                        new OA\Property(property: 'house_price', type: 'number', example: 1500000000),
                        new OA\Property(property: 'dp_total_amount', type: 'number', example: 300000000),
                        new OA\Property(property: 'dp_percentage', type: 'integer', example: 20),
                        new OA\Property(property: 'loan_total_amount', type: 'number', example: 1200000000),
                        new OA\Property(property: 'monthly_amount', type: 'number', example: 11660833),
                        new OA\Property(property: 'loan_interest_total_amount', type: 'number', example: 2798600000),
                    ],
                    type: 'object'
                )
            ),
            new OA\Response(response: 422, description: 'Validation error'),
        ]
    )]
    public function calculate(Request $request): JsonResponse
    {
        $validated = $request->validate([
            'house_id' => 'required|integer|exists:houses,id',
            'interest_id' => 'required|integer|exists:interests,id',
            'dp_percentage' => 'required|integer|min:0|max:100',
        ]);

        $interest = \App\Models\Interest::findOrFail($validated['interest_id']);
        $house = \App\Models\House::findOrFail($validated['house_id']);

        $details = $this->mortgageService->calculateMortgageDetails($house, $interest, $validated['dp_percentage']);

        return response()->json($details);
    }

    #[OA\Post(
        path: '/api/mortgages',
        operationId: 'submitMortgageRequest',
        summary: 'Submit a mortgage application',
        description: 'Creates a new mortgage request with status "Waiting for Bank". Requires multipart/form-data for document upload.',
        tags: ['Mortgages'],
        security: [['sanctum' => []]],
        requestBody: new OA\RequestBody(
            required: true,
            content: new OA\MediaType(
                mediaType: 'multipart/form-data',
                schema: new OA\Schema(
                    required: ['interest_id', 'dp_percentage', 'documents'],
                    properties: [
                        new OA\Property(property: 'interest_id', type: 'integer', example: 1),
                        new OA\Property(property: 'dp_percentage', type: 'integer', example: 20),
                        new OA\Property(property: 'documents', description: 'PDF document (max 2MB)', type: 'string', format: 'binary'),
                    ],
                    type: 'object'
                )
            )
        ),
        responses: [
            new OA\Response(
                response: 201,
                description: 'Mortgage request created',
                content: new OA\JsonContent(
                    properties: [
                        new OA\Property(property: 'message', type: 'string', example: 'Mortgage request submitted successfully'),
                        new OA\Property(property: 'data', ref: '#/components/schemas/MortgageRequest'),
                    ],
                    type: 'object'
                )
            ),
            new OA\Response(response: 422, description: 'Validation error'),
            new OA\Response(response: 401, description: 'Unauthenticated'),
        ]
    )]
    public function store(Request $request): JsonResponse
    {
        $mortgageRequest = $this->mortgageService->handleInterestRequest($request);

        return response()->json([
            'message' => 'Mortgage request submitted successfully',
            'data' => $mortgageRequest,
        ], 201);
    }

    #[OA\Get(
        path: '/api/mortgages',
        operationId: 'listUserMortgages',
        summary: "List authenticated user's mortgage requests",
        tags: ['Mortgages'],
        security: [['sanctum' => []]],
        parameters: [
            new OA\Parameter(name: 'search', description: 'Search by status, bank name, house name, city, or category', in: 'query', required: false, schema: new OA\Schema(type: 'string')),
        ],
        responses: [
            new OA\Response(response: 200, description: 'List of mortgage requests'),
            new OA\Response(response: 401, description: 'Unauthenticated'),
        ]
    )]
    public function index(Request $request): JsonResponse
    {
        $mortgages = $this->mortgageService->getUserMortgages(Auth::id(), $request->input('search'));

        return response()->json($mortgages);
    }

    #[OA\Get(
        path: '/api/mortgages/{id}',
        operationId: 'getMortgageDetail',
        summary: 'Get mortgage request detail with installments',
        tags: ['Mortgages'],
        security: [['sanctum' => []]],
        parameters: [
            new OA\Parameter(name: 'id', description: 'Mortgage request ID', in: 'path', required: true, schema: new OA\Schema(type: 'integer')),
        ],
        responses: [
            new OA\Response(response: 200, description: 'Mortgage request detail'),
            new OA\Response(response: 404, description: 'Mortgage request not found'),
            new OA\Response(response: 401, description: 'Unauthenticated'),
        ]
    )]
    public function show(MortgageRequest $mortgageRequest): JsonResponse
    {
        $details = $this->mortgageService->getMortgageDetails($mortgageRequest);

        return response()->json($details);
    }
}
```

**Step 2: Register routes in `routes/api.php`**

Add to the authenticated section:

```php
use App\Http\Controllers\Api\MortgageController;

Route::middleware('auth:sanctum')->group(function () {
    Route::get('/user', CurrentUserController::class);

    // Mortgages
    Route::get('/mortgages', [MortgageController::class, 'index']);
    Route::post('/mortgages', [MortgageController::class, 'store']);
    Route::get('/mortgages/{mortgageRequest}', [MortgageController::class, 'show']);
});

// Public mortgage calculator
Route::post('/mortgages/calculate', [MortgageController::class, 'calculate']);
```

---

## Task 4: Create Payment & Installment API Controllers

**Files:**

- Create: `app/Http/Controllers/Api/PaymentController.php`
- Create: `app/Http/Controllers/Api/InstallmentController.php`
- Modify: `routes/api.php`

**Step 1: Create PaymentController**

```php
<?php

namespace App\Http\Controllers\Api;

use App\Http\Controllers\Controller;
use App\Models\MortgageRequest;
use App\Services\MortgageService;
use App\Services\PaymentService;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use OpenApi\Attributes as OA;

class PaymentController extends Controller
{
    public function __construct(
        private PaymentService $paymentService,
        private MortgageService $mortgageService
    ) {}

    #[OA\Get(
        path: '/api/mortgages/{id}/payment-breakdown',
        operationId: 'getPaymentBreakdown',
        summary: 'Get payment breakdown for next installment',
        description: 'Returns monthly payment + 11% tax + 900,000 insurance = grand total, plus remaining loan before/after payment.',
        tags: ['Payments'],
        security: [['sanctum' => []]],
        parameters: [
            new OA\Parameter(name: 'id', description: 'Mortgage request ID', in: 'path', required: true, schema: new OA\Schema(type: 'integer')),
        ],
        responses: [
            new OA\Response(
                response: 200,
                description: 'Payment breakdown',
                content: new OA\JsonContent(
                    properties: [
                        new OA\Property(property: 'monthly_payment', type: 'number', example: 11660833),
                        new OA\Property(property: 'total_tax_amount', type: 'number', example: 1282692),
                        new OA\Property(property: 'insurance', type: 'integer', example: 900000),
                        new OA\Property(property: 'grand_total_amount', type: 'number', example: 13843525),
                        new OA\Property(property: 'remaining_loan_amount', type: 'number', example: 2798600000),
                        new OA\Property(property: 'remaining_loan_amount_after_payment', type: 'number', example: 2786939167),
                    ],
                    type: 'object'
                )
            ),
            new OA\Response(response: 404, description: 'Not found'),
            new OA\Response(response: 401, description: 'Unauthenticated'),
        ]
    )]
    public function breakdown(MortgageRequest $mortgageRequest): JsonResponse
    {
        $details = $this->mortgageService->getInstallmentPaymentDetails($mortgageRequest);

        return response()->json($details);
    }

    #[OA\Post(
        path: '/api/mortgages/{id}/pay',
        operationId: 'initiatePayment',
        summary: 'Initiate installment payment via Midtrans',
        description: 'Creates a Midtrans Snap transaction and returns the snap_token for frontend checkout popup.',
        tags: ['Payments'],
        security: [['sanctum' => []]],
        parameters: [
            new OA\Parameter(name: 'id', description: 'Mortgage request ID', in: 'path', required: true, schema: new OA\Schema(type: 'integer')),
        ],
        responses: [
            new OA\Response(
                response: 200,
                description: 'Snap token generated',
                content: new OA\JsonContent(
                    properties: [
                        new OA\Property(property: 'snap_token', type: 'string', example: 'abc123def456'),
                    ],
                    type: 'object'
                )
            ),
            new OA\Response(response: 500, description: 'Payment failed'),
            new OA\Response(response: 401, description: 'Unauthenticated'),
        ]
    )]
    public function pay(MortgageRequest $mortgageRequest): JsonResponse
    {
        try {
            $snapToken = $this->paymentService->createPayment($mortgageRequest);

            return response()->json(['snap_token' => $snapToken]);
        } catch (\Exception $e) {
            return response()->json(['error' => 'Payment failed: '.$e->getMessage()], 500);
        }
    }
}
```

**Step 2: Create InstallmentController**

```php
<?php

namespace App\Http\Controllers\Api;

use App\Http\Controllers\Controller;
use App\Models\Installment;
use App\Models\MortgageRequest;
use App\Services\MortgageService;
use Illuminate\Http\JsonResponse;
use OpenApi\Attributes as OA;

class InstallmentController extends Controller
{
    public function __construct(
        private MortgageService $mortgageService
    ) {}

    #[OA\Get(
        path: '/api/mortgages/{id}/installments',
        operationId: 'listInstallments',
        summary: 'List all installments for a mortgage',
        tags: ['Payments'],
        security: [['sanctum' => []]],
        parameters: [
            new OA\Parameter(name: 'id', description: 'Mortgage request ID', in: 'path', required: true, schema: new OA\Schema(type: 'integer')),
        ],
        responses: [
            new OA\Response(response: 200, description: 'List of installments'),
            new OA\Response(response: 404, description: 'Mortgage not found'),
            new OA\Response(response: 401, description: 'Unauthenticated'),
        ]
    )]
    public function index(MortgageRequest $mortgageRequest): JsonResponse
    {
        $mortgageRequest->load('installments');

        return response()->json($mortgageRequest->installments);
    }

    #[OA\Get(
        path: '/api/installments/{id}',
        operationId: 'getInstallmentDetail',
        summary: 'Get installment detail',
        tags: ['Payments'],
        security: [['sanctum' => []]],
        parameters: [
            new OA\Parameter(name: 'id', description: 'Installment ID', in: 'path', required: true, schema: new OA\Schema(type: 'integer')),
        ],
        responses: [
            new OA\Response(response: 200, description: 'Installment detail with mortgage and house info'),
            new OA\Response(response: 404, description: 'Installment not found'),
            new OA\Response(response: 401, description: 'Unauthenticated'),
        ]
    )]
    public function show(Installment $installment): JsonResponse
    {
        $details = $this->mortgageService->getInstallmentDetails($installment);

        return response()->json($details);
    }
}
```

**Step 3: Register routes in `routes/api.php`**

Add to the authenticated section:

```php
use App\Http\Controllers\Api\PaymentController;
use App\Http\Controllers\Api\InstallmentController;

// Inside auth:sanctum group:
    // Payments
    Route::get('/mortgages/{mortgageRequest}/payment-breakdown', [PaymentController::class, 'breakdown']);
    Route::post('/mortgages/{mortgageRequest}/pay', [PaymentController::class, 'pay']);

    // Installments
    Route::get('/mortgages/{mortgageRequest}/installments', [InstallmentController::class, 'index']);
    Route::get('/installments/{installment}', [InstallmentController::class, 'show']);
```

---

## Task 5: Update OpenAPI Tags & Regenerate Docs

**Files:**

- Modify: `app/OpenApi/OpenApiSpec.php`
- Regenerate: `storage/api-docs/api-docs.json`

**Step 1: Update tags in OpenApiSpec.php**

Add missing tags for the new endpoint groups:

```php
<?php

namespace App\OpenApi;

use OpenApi\Attributes as OA;

#[OA\Info(
    version: '1.0.0',
    title: 'Tedja API',
    description: 'Official API documentation for Tedja — a property listing & KPR mortgage platform.'
)]
#[OA\Server(url: '/', description: 'API server base URL')]
#[OA\SecurityScheme(
    securityScheme: 'sanctum',
    type: 'apiKey',
    in: 'header',
    name: 'Authorization',
    description: 'Use token format: Bearer {token}'
)]
#[OA\Tag(name: 'Auth', description: 'Authentication endpoints')]
#[OA\Tag(name: 'Houses', description: 'Property listing and details')]
#[OA\Tag(name: 'Categories', description: 'Property categories')]
#[OA\Tag(name: 'Cities', description: 'City locations')]
#[OA\Tag(name: 'Banks', description: 'Bank information and interest rates')]
#[OA\Tag(name: 'Facilities', description: 'Property facilities')]
#[OA\Tag(name: 'Mortgages', description: 'Mortgage applications and calculations')]
#[OA\Tag(name: 'Payments', description: 'Installment payments and Midtrans integration')]
#[OA\Tag(name: 'Webhook', description: 'Payment webhook endpoints')]
class OpenApiSpec {}
```

**Step 2: Regenerate docs**

Run: `php artisan l5-swagger:generate`

---

## Task 6: Write Tests for All New API Endpoints

**Files:**

- Modify: `tests/Feature/ApiDocumentationTest.php`

**Step 1: Add tests for all new endpoints**

Add these test methods to the existing `ApiDocumentationTest` class:

```php
// Master Data tests
public function test_api_categories_endpoint_returns_json(): void
{
    $response = $this->getJson('/api/categories');
    $response->assertOk()->assertJsonIsArray();
}

public function test_api_cities_endpoint_returns_json(): void
{
    $response = $this->getJson('/api/cities');
    $response->assertOk()->assertJsonIsArray();
}

public function test_api_banks_endpoint_returns_json(): void
{
    $response = $this->getJson('/api/banks');
    $response->assertOk()->assertJsonIsArray();
}

public function test_api_facilities_endpoint_returns_json(): void
{
    $response = $this->getJson('/api/facilities');
    $response->assertOk()->assertJsonIsArray();
}

// Mortgage tests
public function test_mortgage_calculate_endpoint_validates_input(): void
{
    $response = $this->postJson('/api/mortgages/calculate', []);
    $response->assertUnprocessable();
}

public function test_mortgages_index_requires_auth(): void
{
    $response = $this->getJson('/api/mortgages');
    $response->assertUnauthorized();
}

public function test_mortgages_store_requires_auth(): void
{
    $response = $this->postJson('/api/mortgages', []);
    $response->assertUnauthorized();
}

// Payment tests
public function test_payment_breakdown_requires_auth(): void
{
    $response = $this->getJson('/api/mortgages/1/payment-breakdown');
    $response->assertUnauthorized();
}

public function test_payment_pay_requires_auth(): void
{
    $response = $this->postJson('/api/mortgages/1/pay');
    $response->assertUnauthorized();
}

// Installment tests
public function test_installments_index_requires_auth(): void
{
    $response = $this->getJson('/api/mortgages/1/installments');
    $response->assertUnauthorized();
}

public function test_installments_show_requires_auth(): void
{
    $response = $this->getJson('/api/installments/1');
    $response->assertUnauthorized();
}
```

**Step 2: Run all tests**

Run: `php artisan test tests/Feature/ApiDocumentationTest.php`
Expected: All tests pass

---

## Task 7: Final Cleanup

**Step 1: Run Pint**
Run: `vendor/bin/pint --dirty`

**Step 2: Regenerate Swagger docs**
Run: `php artisan l5-swagger:generate`

**Step 3: Run full test suite**
Run: `php artisan test`
Expected: All existing + new tests pass

**Step 4: Verify endpoint count**

Run: `php artisan route:list --path=api`
Expected: ~20 API routes listed
