<?php

namespace App\Http\Controllers\Api;

use App\Http\Controllers\Controller;
use App\Models\House;
use App\Models\Interest;
use App\Models\MortgageRequest;
use App\Services\MortgageService;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\Auth;
use OpenApi\Attributes as OA;

class MortgageController extends Controller
{
    public function __construct(
        private MortgageService $mortgageService
    ) {}

    #[OA\Post(
        path: '/api/mortgages/calculate',
        operationId: 'calculateMortgage',
        summary: 'Calculate mortgage details without creating a record',
        description: 'Returns monthly payment, total loan, DP, and total cost based on house, interest rate, and down payment percentage.',
        tags: ['Mortgages'],
        requestBody: new OA\RequestBody(
            required: true,
            content: new OA\JsonContent(
                required: ['house_id', 'interest_id', 'dp_percentage'],
                properties: [
                    new OA\Property(property: 'house_id', type: 'integer', example: 1),
                    new OA\Property(property: 'interest_id', type: 'integer', example: 1),
                    new OA\Property(property: 'dp_percentage', type: 'integer', description: 'Down payment percentage (0-100)', example: 20),
                ],
                type: 'object'
            )
        ),
        responses: [
            new OA\Response(
                response: 200,
                description: 'Mortgage calculation result',
                content: new OA\JsonContent(
                    properties: [
                        new OA\Property(property: 'house_price', type: 'number', example: 1500000000),
                        new OA\Property(property: 'dp_total_amount', type: 'number', example: 300000000),
                        new OA\Property(property: 'dp_percentage', type: 'integer', example: 20),
                        new OA\Property(property: 'loan_total_amount', type: 'number', example: 1200000000),
                        new OA\Property(property: 'monthly_amount', type: 'number', example: 11660833),
                        new OA\Property(property: 'loan_interest_total_amount', type: 'number', example: 2798600000),
                    ],
                    type: 'object'
                )
            ),
            new OA\Response(response: 422, description: 'Validation error'),
        ]
    )]
    public function calculate(Request $request): JsonResponse
    {
        $validated = $request->validate([
            'house_id' => 'required|integer|exists:houses,id',
            'interest_id' => 'required|integer|exists:interests,id',
            'dp_percentage' => 'required|integer|min:0|max:100',
        ]);

        $interest = Interest::findOrFail($validated['interest_id']);
        $house = House::findOrFail($validated['house_id']);

        $details = $this->mortgageService->calculateMortgageDetails($house, $interest, $validated['dp_percentage']);

        return response()->json($details);
    }

    #[OA\Post(
        path: '/api/mortgages',
        operationId: 'submitMortgageRequest',
        summary: 'Submit a mortgage application',
        description: 'Creates a new mortgage request with status "Waiting for Bank". Requires multipart/form-data for document upload.',
        tags: ['Mortgages'],
        security: [['sanctum' => []]],
        requestBody: new OA\RequestBody(
            required: true,
            content: new OA\MediaType(
                mediaType: 'multipart/form-data',
                schema: new OA\Schema(
                    required: ['interest_id', 'dp_percentage', 'documents'],
                    properties: [
                        new OA\Property(property: 'interest_id', type: 'integer', example: 1),
                        new OA\Property(property: 'dp_percentage', type: 'integer', example: 20),
                        new OA\Property(property: 'documents', description: 'PDF document (max 2MB)', type: 'string', format: 'binary'),
                    ],
                    type: 'object'
                )
            )
        ),
        responses: [
            new OA\Response(
                response: 201,
                description: 'Mortgage request created',
                content: new OA\JsonContent(
                    properties: [
                        new OA\Property(property: 'message', type: 'string', example: 'Mortgage request submitted successfully'),
                        new OA\Property(property: 'data', type: 'object'),
                    ],
                    type: 'object'
                )
            ),
            new OA\Response(response: 422, description: 'Validation error'),
            new OA\Response(response: 401, description: 'Unauthenticated'),
        ]
    )]
    public function store(Request $request): JsonResponse
    {
        $mortgageRequest = $this->mortgageService->handleInterestRequest($request);

        return response()->json([
            'message' => 'Mortgage request submitted successfully',
            'data' => $mortgageRequest,
        ], 201);
    }

    #[OA\Get(
        path: '/api/mortgages',
        operationId: 'listUserMortgages',
        summary: "List authenticated user's mortgage requests",
        tags: ['Mortgages'],
        security: [['sanctum' => []]],
        parameters: [
            new OA\Parameter(name: 'search', description: 'Search by status, bank name, house name, city, or category', in: 'query', required: false, schema: new OA\Schema(type: 'string')),
        ],
        responses: [
            new OA\Response(response: 200, description: 'List of mortgage requests'),
            new OA\Response(response: 401, description: 'Unauthenticated'),
        ]
    )]
    public function index(Request $request): JsonResponse
    {
        $mortgages = $this->mortgageService->getUserMortgages(Auth::id(), $request->input('search'));

        return response()->json($mortgages);
    }

    #[OA\Get(
        path: '/api/mortgages/{id}',
        operationId: 'getMortgageDetail',
        summary: 'Get mortgage request detail with installments',
        tags: ['Mortgages'],
        security: [['sanctum' => []]],
        parameters: [
            new OA\Parameter(name: 'id', description: 'Mortgage request ID', in: 'path', required: true, schema: new OA\Schema(type: 'integer')),
        ],
        responses: [
            new OA\Response(response: 200, description: 'Mortgage request detail'),
            new OA\Response(response: 404, description: 'Mortgage request not found'),
            new OA\Response(response: 401, description: 'Unauthenticated'),
        ]
    )]
    public function show(MortgageRequest $mortgageRequest): JsonResponse
    {
        $details = $this->mortgageService->getMortgageDetails($mortgageRequest);

        return response()->json($details);
    }
}
