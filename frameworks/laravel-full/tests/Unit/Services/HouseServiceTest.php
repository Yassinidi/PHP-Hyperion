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
        $this->service = new HouseService;
    }

    /**
     * Test getCategoriesAndCities returns both lists.
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

    /**
     * Test searchHouses returns all houses when no filters.
     */
    public function test_search_houses_returns_all_without_filters(): void
    {
        $city = City::factory()->create();
        $category = Category::factory()->create();
        House::factory()->count(5)->create([
            'city_id' => $city->id,
            'category_id' => $category->id,
        ]);

        $result = $this->service->searchHouses([]);

        $this->assertArrayHasKey('houses', $result);
        $this->assertCount(5, $result['houses']);
    }

    /**
     * Test searchHouses filters by city.
     */
    public function test_search_houses_filters_by_city(): void
    {
        $cityJakarta = City::factory()->create();
        $cityBandung = City::factory()->create();
        $category = Category::factory()->create();

        House::factory()->count(3)->create([
            'city_id' => $cityJakarta->id,
            'category_id' => $category->id,
        ]);
        House::factory()->count(2)->create([
            'city_id' => $cityBandung->id,
            'category_id' => $category->id,
        ]);

        $result = $this->service->searchHouses(['city' => $cityJakarta->id]);

        $this->assertCount(3, $result['houses']);
    }

    /**
     * Test searchHouses filters by category.
     */
    public function test_search_houses_filters_by_category(): void
    {
        $city = City::factory()->create();
        $categoryRumah = Category::factory()->create();
        $categoryApartemen = Category::factory()->create();

        House::factory()->count(4)->create([
            'city_id' => $city->id,
            'category_id' => $categoryRumah->id,
        ]);
        House::factory()->count(1)->create([
            'city_id' => $city->id,
            'category_id' => $categoryApartemen->id,
        ]);

        $result = $this->service->searchHouses(['category' => $categoryRumah->id]);

        $this->assertCount(4, $result['houses']);
    }

    /**
     * Test searchHouses filters by both city and category.
     */
    public function test_search_houses_filters_by_city_and_category(): void
    {
        $cityJakarta = City::factory()->create();
        $cityBandung = City::factory()->create();
        $categoryRumah = Category::factory()->create();
        $categoryApartemen = Category::factory()->create();

        House::factory()->create([
            'city_id' => $cityJakarta->id,
            'category_id' => $categoryRumah->id,
        ]);
        House::factory()->create([
            'city_id' => $cityJakarta->id,
            'category_id' => $categoryApartemen->id,
        ]);
        House::factory()->create([
            'city_id' => $cityBandung->id,
            'category_id' => $categoryRumah->id,
        ]);

        $result = $this->service->searchHouses([
            'city' => $cityJakarta->id,
            'category' => $categoryRumah->id,
        ]);

        $this->assertCount(1, $result['houses']);
    }

    /**
     * Test getHouseDetails eager loads relationships.
     */
    public function test_get_house_details_eager_loads_relationships(): void
    {
        $city = City::factory()->create();
        $category = Category::factory()->create();
        $house = House::factory()->create([
            'city_id' => $city->id,
            'category_id' => $category->id,
        ]);

        $result = $this->service->getHouseDetails($house);

        $this->assertInstanceOf(House::class, $result);
        $this->assertEquals($house->id, $result->id);
    }
}
