<?php

namespace App\Http\Controllers;

use App\Models\Category;
use App\Models\House;
use App\Models\Interest;
use App\Services\HouseService;
use App\Services\MortgageService;
use Illuminate\Http\Request;

class FrontController extends Controller
{
    protected $houseService;

    protected $mortgageService;

    public function __construct(HouseService $houseService, MortgageService $mortgageService)
    {
        $this->houseService = $houseService;
        $this->mortgageService = $mortgageService;
    }

    public function index()
    {
        $data = $this->houseService->getCategoriesAndCities();

        return view('front.index', $data);
    }

    public function search(Request $request)
    {
        $data = $this->houseService->searchHouses($request->all());

        return view('front.search', $data);
    }

    public function category(Category $category)
    {
        $category->load(['houses']);

        return view('front.category', compact('category'));
    }

    public function details(House $house)
    {
        $houseDetails = $this->houseService->getHouseDetails($house);

        return view('front.details', compact('houseDetails'));
    }

    public function interest(Interest $interest)
    {
        return view('customer.mortgages.request_mortgage', compact('interest'));
    }

    public function request_interest(Request $request)
    {
        $this->mortgageService->handleInterestRequest($request);

        return redirect()->route('front.request_success');
    }

    public function request_success()
    {
        $interest = $this->mortgageService->getInterestFromSession();
        if (! $interest) {
            return redirect()->route('front.index')->with('error', 'Invalid request. Please try again.');
        }

        return view('customer.mortgages.request_success', compact('interest'));
    }
}
