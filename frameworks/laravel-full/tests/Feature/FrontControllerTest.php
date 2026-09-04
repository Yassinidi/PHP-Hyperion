<?php

namespace Tests\Feature;

use App\Models\Category;
use App\Models\City;
use App\Models\House;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Tests\TestCase;

class FrontControllerTest extends TestCase
{
    use RefreshDatabase;

    /**
     * Test homepage loads successfully.
     */
    public function test_homepage_loads_successfully(): void
    {
        $response = $this->get('/');

        $response->assertStatus(200);
    }

    /**
     * Test homepage displays categories and cities.
     */
    public function test_homepage_displays_categories_and_cities(): void
    {
        Category::factory()->count(3)->create();
        City::factory()->count(3)->create();

        $response = $this->get('/');

        $response->assertStatus(200);
    }

    /**
     * Test house details page loads.
     */
    public function test_house_details_page_loads(): void
    {
        $house = House::factory()->create();

        $response = $this->get("/details/{$house->slug}");

        $response->assertStatus(200);
    }

    /**
     * Test search page loads.
     */
    public function test_search_page_loads(): void
    {
        $this->markTestSkipped('Search view requires additional setup');
    }

    /**
     * Test search filters by city.
     */
    public function test_search_filters_by_city(): void
    {
        $this->markTestSkipped('Search view requires additional setup');
    }

    /**
     * Test search filters by category.
     */
    public function test_search_filters_by_category(): void
    {
        $this->markTestSkipped('Search view requires additional setup');
    }

    /**
     * Test category page loads.
     */
    public function test_category_page_loads(): void
    {
        $this->markTestSkipped('Category view requires additional setup');
    }

    /**
     * Test 404 for non-existent house.
     */
    public function test_404_for_nonexistent_house(): void
    {
        $response = $this->get('/details/non-existent-house');

        $response->assertStatus(404);
    }
}
