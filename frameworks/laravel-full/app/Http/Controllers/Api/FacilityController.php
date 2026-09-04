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
