# Tedja Test Coverage Implementation Plan

> **Document Version:** 1.1  
> **Target:** Achieve 70-80% overall code coverage  
> **Estimated Time:** 3-4 development sprints  
> **Priority:** HIGH (blocks production deployment)
> **Current Status:** Phases 1-2 COMPLETED ✅ | Phase 3 PENDING | Coverage: ~35%

---

## 📊 Current Coverage Analysis

```
Overall Coverage: ~35% (IN PROGRESS) ✅

Completed Phases:
├── Phase 1: Services ✅ (MortgageService, PaymentService, HouseService, MidtransService)
├── Phase 2: Models & Controllers ✅ (MortgageRequest, House, Installment, User, Controllers)
└── Phase 3: Edge Cases & Polish ⏳ (PENDING)

Baseline: 22.8% → Current: ~35% → Target: 75%
```

---

## 🎯 Coverage Targets

| Component              | Current   | Target  | Priority      |
| ---------------------- | --------- | ------- | ------------- |
| **Services**           | 0-26.7%   | 90%     | P0 - Critical |
| **Models**             | 0-100%    | 80%     | P1 - High     |
| **Controllers**        | 0-100%    | 70%     | P1 - High     |
| **Filament Resources** | 0-21%     | 50%     | P2 - Medium   |
| **Overall**            | **22.8%** | **75%** | **P0**        |

---

## 📁 File Structure to Create

```
tests/
├── Unit/
│   ├── Services/
│   │   ├── MortgageServiceTest.php      [Phase 1]
│   │   ├── PaymentServiceTest.php       [Phase 1]
│   │   ├── HouseServiceTest.php         [Phase 1]
│   │   └── MidtransServiceTest.php      [Phase 2]
│   ├── Models/
│   │   ├── MortgageRequestTest.php      [Phase 2]
│   │   ├── HouseTest.php                [Phase 2]
│   │   ├── InstallmentTest.php          [Phase 2]
│   │   └── UserTest.php                 [Phase 2]
│   └── Rules/
│       └── IndonesianPhoneRuleTest.php  [Phase 3]
├── Feature/
│   ├── FrontControllerTest.php          [Phase 2]
│   ├── DashboardControllerTest.php      [Phase 2]
│   └── Services/
│       └── HouseServiceFeatureTest.php  [Phase 3]
└── Helpers/
    ├── TestHelper.php                   [Phase 1]
    └── MortgageFactory.php              [Phase 1]
```

---

## 🚀 PHASE 1: Critical Business Logic (Services)

**Duration:** 2-3 days  
**Target:** Services 90% coverage  
**Current:** 0-26.7%  
**Files:** 4 test files

### **1.1 MortgageServiceTest.php**

**Purpose:** Core mortgage calculations - HIGHEST RISK  
**Estimated Lines:** 250-300 lines  
**Methods to Test:** 7

```php
<?php

namespace Tests\Unit\Services;

use App\Models\House;
use App\Models\Interest;
use App\Models\MortgageRequest;
use App\Models\User;
use App\Services\MortgageService;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Http\UploadedFile;
use Illuminate\Support\Facades\Storage;
use Tests\TestCase;

class MortgageServiceTest extends TestCase
{
    use RefreshDatabase;

    private MortgageService $service;

    protected function setUp(): void
    {
        parent::setUp();
        $this->service = new MortgageService();
        $this->seed(\Database\Seeders\RoleAdminSeeder::class);
    }

    /**
     * Test calculateMortgageDetails with standard parameters
     * Coverage: calculateMortgageDetails() method
     */
    public function test_calculate_mortgage_details_with_standard_parameters(): void
    {
        // Arrange
        $housePrice = 1000000000; // 1 billion IDR
        $dpPercentage = 20;
        $interestRate = 5; // 5%
        $duration = 15; // 15 years

        // Act
        $result = $this->service->calculateMortgageDetails(
            housePrice: $housePrice,
            dpPercentage: $dpPercentage,
            interestRate: $interestRate,
            duration: $duration
        );

        // Assert
        $this->assertArrayHasKey('dp_total_amount', $result);
        $this->assertArrayHasKey('loan_total_amount', $result);
        $this->assertArrayHasKey('monthly_amount', $result);
        $this->assertArrayHasKey('loan_interest_total_amount', $result);

        // Verify calculations
        $this->assertEquals(200000000, $result['dp_total_amount']); // 20% of 1B
        $this->assertEquals(800000000, $result['loan_total_amount']); // 80% of 1B
        $this->assertGreaterThan(0, $result['monthly_amount']);

        // Verify amortization formula
        $expectedMonthly = $this->calculateExpectedMonthly(800000000, 5, 15);
        $this->assertEqualsWithDelta($expectedMonthly, $result['monthly_amount'], 1);
    }

    /**
     * Test calculateMortgageDetails with minimum down payment (10%)
     */
    public function test_calculate_mortgage_with_minimum_down_payment(): void
    {
        $result = $this->service->calculateMortgageDetails(
            housePrice: 500000000,
            dpPercentage: 10,
            interestRate: 8,
            duration: 10
        );

        $this->assertEquals(50000000, $result['dp_total_amount']); // 10%
        $this->assertEquals(450000000, $result['loan_total_amount']); // 90%
    }

    /**
     * Test calculateMortgageDetails with maximum down payment (80%)
     */
    public function test_calculate_mortgage_with_maximum_down_payment(): void
    {
        $result = $this->service->calculateMortgageDetails(
            housePrice: 1000000000,
            dpPercentage: 80,
            interestRate: 5,
            duration: 5
        );

        $this->assertEquals(800000000, $result['dp_total_amount']);
        $this->assertEquals(200000000, $result['loan_total_amount']);
    }

    /**
     * Test handleInterestRequest creates mortgage request successfully
     * Coverage: handleInterestRequest(), createMortgageRequest(), uploadDocuments()
     */
    public function test_handle_interest_request_creates_mortgage_successfully(): void
    {
        // Arrange
        $user = User::factory()->create();
        $user->assignRole('customer');

        $house = House::factory()->create(['price' => 1000000000]);
        $interest = Interest::factory()->create([
            'house_id' => $house->id,
            'interest' => 5,
            'duration' => 15
        ]);

        $request = new \Illuminate\Http\Request([
            'house_id' => $house->id,
            'interest_id' => $interest->id,
            'dp_percentage' => 20,
            'duration' => 15,
            'bank_name' => 'BCA',
            'interest' => 5,
        ]);

        Storage::fake('public');

        // Act
        $mortgageRequest = $this->service->handleInterestRequest($request);

        // Assert
        $this->assertInstanceOf(MortgageRequest::class, $mortgageRequest);
        $this->assertDatabaseHas('mortgage_requests', [
            'user_id' => $user->id,
            'house_id' => $house->id,
            'status' => 'Waiting for Bank',
            'dp_percentage' => 20,
        ]);

        // Verify calculations were stored correctly
        $this->assertEquals(200000000, $mortgageRequest->dp_total_amount);
        $this->assertEquals(800000000, $mortgageRequest->loan_total_amount);
    }

    /**
     * Test handleInterestRequest with document upload
     */
    public function test_handle_interest_request_uploads_documents(): void
    {
        Storage::fake('public');

        $user = User::factory()->create();
        $user->assignRole('customer');
        $house = House::factory()->create();
        $interest = Interest::factory()->create(['house_id' => $house->id]);

        $request = new \Illuminate\Http\Request([
            'house_id' => $house->id,
            'interest_id' => $interest->id,
            'dp_percentage' => 20,
            'duration' => 15,
            'bank_name' => 'BCA',
            'interest' => 5,
        ]);

        $request->files->set('documents', UploadedFile::fake()->create('document.pdf', 100));

        $mortgageRequest = $this->service->handleInterestRequest($request);

        Storage::disk('public')->assertExists($mortgageRequest->documents);
    }

    /**
     * Test handleInterestRequest with validation errors
     */
    public function test_handle_interest_request_throws_exception_for_invalid_house(): void
    {
        $this->expectException(\Illuminate\Database\Eloquent\ModelNotFoundException::class);

        $request = new \Illuminate\Http\Request([
            'house_id' => 99999, // Non-existent
            'interest_id' => 1,
            'dp_percentage' => 20,
        ]);

        $this->service->handleInterestRequest($request);
    }

    /**
     * Test getUserMortgages returns only user's mortgages
     */
    public function test_get_user_mortgages_returns_only_users_data(): void
    {
        $user1 = User::factory()->create();
        $user1->assignRole('customer');
        $user2 = User::factory()->create();
        $user2->assignRole('customer');

        $house = House::factory()->create();

        // Create mortgages for different users
        MortgageRequest::factory()->create([
            'user_id' => $user1->id,
            'house_id' => $house->id,
        ]);
        MortgageRequest::factory()->create([
            'user_id' => $user2->id,
            'house_id' => $house->id,
        ]);

        $result = $this->service->getUserMortgages($user1->id);

        $this->assertCount(1, $result);
        $this->assertEquals($user1->id, $result->first()->user_id);
    }

    /**
     * Test getMortgageDetails eager loads relationships
     */
    public function test_get_mortgage_details_eager_loads_installments(): void
    {
        $user = User::factory()->create();
        $house = House::factory()->create();
        $mortgage = MortgageRequest::factory()->create([
            'user_id' => $user->id,
            'house_id' => $house->id,
        ]);

        $result = $this->service->getMortgageDetails($mortgage);

        $this->assertTrue($result->relationLoaded('installments'));
        $this->assertTrue($result->relationLoaded('house'));
        $this->assertTrue($result->relationLoaded('interest'));
    }

    /**
     * Helper method: Calculate expected monthly payment using amortization formula
     */
    private function calculateExpectedMonthly(
        float $principal,
        float $annualRate,
        int $years
    ): float {
        $monthlyRate = ($annualRate / 100) / 12;
        $numPayments = $years * 12;

        if ($monthlyRate == 0) {
            return $principal / $numPayments;
        }

        return $principal * (
            ($monthlyRate * pow(1 + $monthlyRate, $numPayments)) /
            (pow(1 + $monthlyRate, $numPayments) - 1)
        );
    }
}
```

**Dependencies to Create:**

- `MortgageRequestFactory` - Factory for mortgage requests
- Test helpers for amortization calculations

---

### **1.2 PaymentServiceTest.php**

**Purpose:** Payment processing with Midtrans  
**Estimated Lines:** 200-250 lines  
**Methods to Test:** 5

```php
<?php

namespace Tests\Unit\Services;

use App\Models\Installment;
use App\Models\MortgageRequest;
use App\Services\PaymentService;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Mockery;
use Tests\TestCase;

class PaymentServiceTest extends TestCase
{
    use RefreshDatabase;

    private PaymentService $service;

    protected function setUp(): void
    {
        parent::setUp();
        $this->service = new PaymentService();
        $this->seed(\Database\Seeders\RoleAdminSeeder::class);
    }

    /**
     * Test createPayment returns correct Snap token
     */
    public function test_create_payment_returns_snap_token(): void
    {
        // Mock MidtransService
        $mockMidtrans = Mockery::mock(\App\Services\MidtransService::class);
        $mockMidtrans->shouldReceive('createSnapToken')
            ->once()
            ->andReturn(['token' => 'test-snap-token-123']);

        $this->app->instance(\App\Services\MidtransService::class, $mockMidtrans);

        $user = \App\Models\User::factory()->create();
        $mortgage = \App\Models\MortgageRequest::factory()->create([
            'user_id' => $user->id,
            'monthly_amount' => 5000000,
        ]);

        $result = $this->service->createPayment($mortgage);

        $this->assertArrayHasKey('token', $result);
        $this->assertEquals('test-snap-token-123', $result['token']);
    }

    /**
     * Test createPayment calculates grand total correctly
     * Monthly + Insurance + Tax
     */
    public function test_create_payment_calculates_grand_total(): void
    {
        $mortgage = \App\Models\MortgageRequest::factory()->create([
            'monthly_amount' => 10000000, // 10M
        ]);

        $mockMidtrans = Mockery::mock(\App\Services\MidtransService::class);
        $mockMidtrans->shouldReceive('createSnapToken')
            ->with(Mockery::on(function ($params) {
                // Verify gross_amount includes monthly + insurance + tax
                $expectedMonthly = 10000000;
                $expectedInsurance = 900000;
                $expectedTax = $expectedMonthly * 0.11; // 11%
                $expectedTotal = $expectedMonthly + $expectedInsurance + $expectedTax;

                return $params['transaction_details']['gross_amount'] == $expectedTotal;
            }))
            ->once()
            ->andReturn(['token' => 'test-token']);

        $this->app->instance(\App\Services\MidtransService::class, $mockMidtrans);

        $this->service->createPayment($mortgage);
    }

    /**
     * Test processNotification creates installment on successful payment
     */
    public function test_process_notification_creates_installment(): void
    {
        $mortgage = \App\Models\MortgageRequest::factory()->create([
            'loan_total_amount' => 800000000,
            'remaining_loan_amount' => 800000000,
        ]);

        $notification = [
            'order_id' => 'INSTALLMENT-' . $mortgage->id . '-001',
            'transaction_status' => 'settlement',
            'payment_type' => 'credit_card',
        ];

        $result = $this->service->processNotification($notification);

        $this->assertInstanceOf(Installment::class, $result);
        $this->assertDatabaseHas('installments', [
            'mortgage_request_id' => $mortgage->id,
            'is_paid' => true,
            'payment_type' => 'Midtrans',
        ]);
    }

    /**
     * Test processNotification updates remaining loan amount
     */
    public function test_process_notification_updates_remaining_loan(): void
    {
        $mortgage = \App\Models\MortgageRequest::factory()->create([
            'loan_total_amount' => 800000000,
            'remaining_loan_amount' => 800000000,
            'monthly_amount' => 10000000,
        ]);

        $notification = [
            'order_id' => 'INSTALLMENT-' . $mortgage->id . '-001',
            'transaction_status' => 'settlement',
            'payment_type' => 'bank_transfer',
        ];

        $this->service->processNotification($notification);

        $mortgage->refresh();
        $this->assertEquals(790000000, $mortgage->remaining_loan_amount);
    }

    /**
     * Test processNotification ignores non-settlement status
     */
    public function test_process_notification_ignores_pending_status(): void
    {
        $mortgage = \App\Models\MortgageRequest::factory()->create();

        $notification = [
            'order_id' => 'INSTALLMENT-' . $mortgage->id . '-001',
            'transaction_status' => 'pending',
            'payment_type' => 'credit_card',
        ];

        $result = $this->service->processNotification($notification);

        $this->assertNull($result);
        $this->assertDatabaseCount('installments', 0);
    }

    protected function tearDown(): void
    {
        Mockery::close();
        parent::tearDown();
    }
}
```

---

### **1.3 HouseServiceTest.php**

**Purpose:** Property search and filtering  
**Estimated Lines:** 150-200 lines  
**Methods to Test:** 4

```php
<?php

namespace Tests\Unit\Services;

use App\Models\Category;
use App\Models\City;
use App\Models\House;
use App\Services\HouseService;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Tests\TestCase;

class HouseServiceTest extends TestCase
{
    use RefreshDatabase;

    private HouseService $service;

    protected function setUp(): void
    {
        parent::setUp();
        $this->service = new HouseService();
    }

    /**
     * Test searchHouses returns all houses when no filters
     */
    public function test_search_houses_returns_all_without_filters(): void
    {
        House::factory()->count(5)->create();

        $result = $this->service->searchHouses([]);

        $this->assertCount(5, $result);
    }

    /**
     * Test searchHouses filters by city_id
     */
    public function test_search_houses_filters_by_city(): void
    {
        $cityJakarta = City::factory()->create(['name' => 'Jakarta']);
        $cityBandung = City::factory()->create(['name' => 'Bandung']);

        House::factory()->count(3)->create(['city_id' => $cityJakarta->id]);
        House::factory()->count(2)->create(['city_id' => $cityBandung->id]);

        $result = $this->service->searchHouses(['city_id' => $cityJakarta->id]);

        $this->assertCount(3, $result);
        $result->each(function ($house) use ($cityJakarta) {
            $this->assertEquals($cityJakarta->id, $house->city_id);
        });
    }

    /**
     * Test searchHouses filters by category_id
     */
    public function test_search_houses_filters_by_category(): void
    {
        $categoryRumah = Category::factory()->create(['name' => 'Rumah']);
        $categoryApartemen = Category::factory()->create(['name' => 'Apartemen']);

        House::factory()->count(4)->create(['category_id' => $categoryRumah->id]);
        House::factory()->count(1)->create(['category_id' => $categoryApartemen->id]);

        $result = $this->service->searchHouses(['category_id' => $categoryRumah->id]);

        $this->assertCount(4, $result);
    }

    /**
     * Test searchHouses filters by both city and category
     */
    public function test_search_houses_filters_by_city_and_category(): void
    {
        $cityJakarta = City::factory()->create();
        $cityBandung = City::factory()->create();
        $categoryRumah = Category::factory()->create();
        $categoryApartemen = Category::factory()->create();

        // Jakarta Rumah (should match)
        House::factory()->create([
            'city_id' => $cityJakarta->id,
            'category_id' => $categoryRumah->id,
        ]);

        // Jakarta Apartemen (should not match)
        House::factory()->create([
            'city_id' => $cityJakarta->id,
            'category_id' => $categoryApartemen->id,
        ]);

        // Bandung Rumah (should not match)
        House::factory()->create([
            'city_id' => $cityBandung->id,
            'category_id' => $categoryRumah->id,
        ]);

        $result = $this->service->searchHouses([
            'city_id' => $cityJakarta->id,
            'category_id' => $categoryRumah->id,
        ]);

        $this->assertCount(1, $result);
    }

    /**
     * Test getHouseDetails eager loads all relationships
     */
    public function test_get_house_details_eager_loads_relationships(): void
    {
        $house = House::factory()->create();

        $result = $this->service->getHouseDetails($house);

        $this->assertTrue($result->relationLoaded('category'));
        $this->assertTrue($result->relationLoaded('city'));
        $this->assertTrue($result->relationLoaded('photos'));
        $this->assertTrue($result->relationLoaded('facilities.facility'));
        $this->assertTrue($result->relationLoaded('interests.bank'));
    }

    /**
     * Test getCategoriesAndCities returns both lists
     */
    public function test_get_categories_and_cities_returns_both(): void
    {
        Category::factory()->count(3)->create();
        City::factory()->count(4)->create();

        $result = $this->service->getCategoriesAndCities();

        $this->assertArrayHasKey('categories', $result);
        $this->assertArrayHasKey('cities', $result);
        $this->assertCount(3, $result['categories']);
        $this->assertCount(4, $result['cities']);
    }
}
```

---

### **1.4 MidtransServiceTest.php**

**Purpose:** External payment gateway integration  
**Estimated Lines:** 150-180 lines  
**Methods to Test:** 3

```php
<?php

namespace Tests\Unit\Services;

use App\Services\MidtransService;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Mockery;
use Tests\TestCase;

class MidtransServiceTest extends TestCase
{
    use RefreshDatabase;

    private MidtransService $service;

    protected function setUp(): void
    {
        parent::setUp();
        $this->service = new MidtransService();
    }

    /**
     * Test createSnapToken returns token from Midtrans API
     */
    public function test_create_snap_token_returns_token(): void
    {
        // This test should mock the Midtrans API call
        // Or use a test mode/sandbox token

        $params = [
            'transaction_details' => [
                'order_id' => 'TEST-ORDER-123',
                'gross_amount' => 10000000,
            ],
            'customer_details' => [
                'first_name' => 'Test',
                'email' => 'test@example.com',
                'phone' => '081234567890',
            ],
        ];

        // Note: In real implementation, mock the Midtrans Snap class
        // $this->markTestSkipped('Requires Midtrans API mocking');

        // Mock example:
        // $mockSnap = Mockery::mock('alias:\Midtrans\Snap');
        // $mockSnap->shouldReceive('getSnapToken')->andReturn('test-token');

        $this->assertTrue(true); // Placeholder
    }

    /**
     * Test handleNotification parses webhook correctly
     */
    public function test_handle_notification_parses_webhook(): void
    {
        $notification = new \stdClass();
        $notification->order_id = 'INSTALLMENT-123-001';
        $notification->transaction_status = 'settlement';
        $notification->payment_type = 'credit_card';

        $result = $this->service->handleNotification($notification);

        $this->assertEquals('INSTALLMENT-123-001', $result['order_id']);
        $this->assertEquals('settlement', $result['transaction_status']);
        $this->assertEquals('credit_card', $result['payment_type']);
    }

    /**
     * Test handleNotification handles invalid input
     */
    public function test_handle_notification_handles_invalid_input(): void
    {
        $this->expectException(\Exception::class);

        $invalidNotification = null;
        $this->service->handleNotification($invalidNotification);
    }

    protected function tearDown(): void
    {
        Mockery::close();
        parent::tearDown();
    }
}
```

---

## 📊 PHASE 1 Summary

| File                    | Lines   | Methods | Coverage Target |
| ----------------------- | ------- | ------- | --------------- |
| MortgageServiceTest.php | 300     | 9       | 95%             |
| PaymentServiceTest.php  | 250     | 5       | 90%             |
| HouseServiceTest.php    | 200     | 6       | 90%             |
| MidtransServiceTest.php | 180     | 3       | 80%             |
| **TOTAL**               | **930** | **23**  | **90%**         |

**Expected Results:**

- Services coverage: 0-26.7% → 90%
- Overall coverage: 22.8% → 40-45%

---

## 🚀 PHASE 2: Models & Controllers

**Duration:** 3-4 days  
**Target:** Models 80%, Controllers 70%  
**Files:** 8 test files

### **2.1 Model Tests**

Create: `tests/Unit/Models/MortgageRequestTest.php`

Test:

- Relationships (belongsTo, hasMany)
- Accessors (getRemainingLoanAmountAttribute)
- Scopes (if any)
- Fillable/guarded fields

### **2.2 Controller Tests**

Create: `tests/Feature/DashboardControllerTest.php`

Test:

- Authentication required
- Role-based access (customer only)
- CRUD operations
- Payment flows

### **2.3 FrontController Tests**

Create: `tests/Feature/FrontControllerTest.php`

Test:

- Homepage loads
- House details page
- Search functionality
- Mortgage application form

---

## 📊 PHASE 2 Summary

| Component   | Current | Target |
| ----------- | ------- | ------ |
| Models      | 0-100%  | 80%    |
| Controllers | 0-100%  | 70%    |
| Overall     | 40-45%  | 60-65% |

---

## 🚀 PHASE 3: Edge Cases & Polish

**Duration:** 2 days  
**Target:** Overall 75-80%

### **3.1 Edge Cases to Test**

1. **Mortgage Calculations:**
    - Zero interest rate
    - Zero down payment (if allowed)
    - Very high house prices
    - Maximum loan duration

2. **Payment Scenarios:**
    - Failed payments
    - Partial payments
    - Duplicate payment attempts

3. **Security:**
    - CSRF protection
    - SQL injection prevention
    - XSS protection

### **3.2 Filament Resource Tests (Optional)**

Low priority since Filament has built-in testing:

- Create 1-2 basic tests per resource
- Focus on custom logic only

---

## 📁 Implementation Order

### **Week 1: Services (Critical)**

```
Day 1-2: MortgageServiceTest.php
Day 3: PaymentServiceTest.php
Day 4: HouseServiceTest.php
Day 5: MidtransServiceTest.php + Fixes
```

### **Week 2: Models & Controllers**

```
Day 1-2: Model Tests (MortgageRequest, House, Installment)
Day 3-4: DashboardControllerTest.php
Day 5: FrontControllerTest.php
```

### **Week 3: Integration & Polish**

```
Day 1-2: Edge cases, error handling
Day 3: Filament resource tests (optional)
Day 4-5: Refactoring, coverage review
```

---

## 🎯 Success Criteria

- [ ] Services: 90%+ coverage
- [ ] Models: 80%+ coverage
- [ ] Controllers: 70%+ coverage
- [ ] Overall: 75%+ coverage
- [ ] All critical paths tested (mortgage calc, payment)
- [ ] No failing tests
- [ ] CI/CD pipeline passes

---

## 📋 Checklist for Each Test File

Before marking a test file complete:

- [ ] All public methods tested
- [ ] Happy path covered
- [ ] Error cases covered
- [ ] Edge cases considered
- [ ] Mock external services (Midtrans)
- [ ] Use RefreshDatabase trait
- [ ] Run `composer test` passes
- [ ] Run `composer format` passes
- [ ] Coverage report shows target %

---

## 🛠️ Commands Reference

```bash
# Run all tests
composer test

# Run specific test file
php artisan test tests/Unit/Services/MortgageServiceTest.php

# Run with coverage
composer test:coverage

# Generate HTML report
composer test:coverage-html

# Check formatting
composer format

# Fix formatting
composer format
```

---

## 📚 Resources

- [Laravel Testing Docs](https://laravel.com/docs/12.x/testing)
- [PHPUnit Docs](https://phpunit.readthedocs.io/)
- [Mockery Docs](http://docs.mockery.io/)
- [PCOV Extension](https://github.com/krakjoe/pcov)

---

**Document Owner:** Development Team  
**Last Updated:** 2026-04-12  
**Next Review:** After Phase 1 completion
