<?php

namespace App\Http\Controllers\Api;

use App\Http\Controllers\Controller;
use App\Services\HouseService;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use OpenApi\Attributes as OA;

class HouseController extends Controller
{
    public function __construct(
        private HouseService $houseService
    ) {}

    #[OA\Get(
        path: '/api/houses',
        operationId: 'listHouses',
        summary: 'List all houses with optional filters',
        description: 'Returns a paginated list of houses, optionally filtered by city and category.',
        tags: ['Houses'],
        parameters: [
            new OA\Parameter(name: 'city_id', description: 'Filter by city ID', in: 'query', required: false, schema: new OA\Schema(type: 'integer', example: 1)),
            new OA\Parameter(name: 'category_id', description: 'Filter by category ID', in: 'query', required: false, schema: new OA\Schema(type: 'integer', example: 1)),
        ],
        responses: [
            new OA\Response(
                response: 200,
                description: 'List of houses',
                content: new OA\JsonContent(
                    type: 'array',
                    items: new OA\Items(
                        properties: [
                            new OA\Property(property: 'id', type: 'integer', example: 1),
                            new OA\Property(property: 'name', type: 'string', example: 'Rumah Modern Jakarta'),
                            new OA\Property(property: 'slug', type: 'string', example: 'rumah-modern-jakarta'),
                            new OA\Property(property: 'price', type: 'integer', example: 1500000000),
                            new OA\Property(property: 'bedroom', type: 'integer', example: 3),
                            new OA\Property(property: 'bathroom', type: 'integer', example: 2),
                            new OA\Property(property: 'building_area', type: 'integer', example: 120),
                            new OA\Property(property: 'land_area', type: 'integer', example: 150),
                            new OA\Property(property: 'thumbnail', type: 'string', nullable: true),
                            new OA\Property(
                                property: 'category',
                                properties: [
                                    new OA\Property(property: 'id', type: 'integer', example: 1),
                                    new OA\Property(property: 'name', type: 'string', example: 'Rumah'),
                                    new OA\Property(property: 'slug', type: 'string', example: 'rumah'),
                                ],
                                type: 'object'
                            ),
                            new OA\Property(
                                property: 'city',
                                properties: [
                                    new OA\Property(property: 'id', type: 'integer', example: 1),
                                    new OA\Property(property: 'name', type: 'string', example: 'Jakarta'),
                                    new OA\Property(property: 'slug', type: 'string', example: 'jakarta'),
                                ],
                                type: 'object'
                            ),
                        ],
                        type: 'object'
                    )
                )
            ),
        ]
    )]
    public function index(Request $request): JsonResponse
    {
        $filters = [
            'city' => $request->input('city_id'),
            'category' => $request->input('category_id'),
        ];
        $result = $this->houseService->searchHouses(array_filter($filters));

        return response()->json($result['houses']);
    }

    #[OA\Get(
        path: '/api/houses/{slug}',
        operationId: 'getHouseDetail',
        summary: 'Get house detail by slug',
        description: 'Returns detailed information about a specific property including photos, facilities, and bank interest rates.',
        tags: ['Houses'],
        parameters: [
            new OA\Parameter(name: 'slug', description: 'House slug', in: 'path', required: true, schema: new OA\Schema(type: 'string')),
        ],
        responses: [
            new OA\Response(
                response: 200,
                description: 'House detail',
                content: new OA\JsonContent(
                    properties: [
                        new OA\Property(property: 'id', type: 'integer', example: 1),
                        new OA\Property(property: 'name', type: 'string', example: 'Rumah Modern Jakarta'),
                        new OA\Property(property: 'slug', type: 'string', example: 'rumah-modern-jakarta'),
                        new OA\Property(property: 'about', type: 'string', example: 'Modern house in the heart of Jakarta'),
                        new OA\Property(property: 'price', type: 'integer', example: 1500000000),
                        new OA\Property(property: 'bedroom', type: 'integer', example: 3),
                        new OA\Property(property: 'bathroom', type: 'integer', example: 2),
                        new OA\Property(property: 'building_area', type: 'integer', example: 120),
                        new OA\Property(property: 'land_area', type: 'integer', example: 150),
                        new OA\Property(property: 'certificate', type: 'string', example: 'SHM'),
                        new OA\Property(property: 'thumbnail', type: 'string', nullable: true),
                    ],
                    type: 'object'
                )
            ),
            new OA\Response(response: 404, description: 'House not found'),
        ]
    )]
    public function show(string $slug): JsonResponse
    {
        $house = \App\Models\House::where('slug', $slug)->firstOrFail();

        $house = $this->houseService->getHouseDetails($house);

        return response()->json($house);
    }

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
                            new OA\Property(
                                property: 'bank',
                                properties: [
                                    new OA\Property(property: 'id', type: 'integer', example: 1),
                                    new OA\Property(property: 'name', type: 'string', example: 'BCA'),
                                ],
                                type: 'object'
                            ),
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
}
