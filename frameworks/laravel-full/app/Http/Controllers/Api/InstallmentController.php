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
