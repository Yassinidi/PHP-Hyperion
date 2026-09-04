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
