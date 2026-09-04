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
