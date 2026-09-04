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
