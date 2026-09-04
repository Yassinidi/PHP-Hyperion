<?php

namespace App\Http\Controllers\Api;

use App\Http\Controllers\Controller;
use App\Models\MortgageRequest;
use App\Services\MortgageService;
use App\Services\PaymentService;
use Illuminate\Http\JsonResponse;
use OpenApi\Attributes as OA;

class PaymentController extends Controller
{
    public function __construct(
        private PaymentService $paymentService,
        private MortgageService $mortgageService
    ) {}

    #[OA\Get(
        path: '/api/mortgages/{id}/payment-breakdown',
        operationId: 'getPaymentBreakdown',
        summary: 'Get payment breakdown for next installment',
        description: 'Returns monthly payment + 11% tax + 900,000 insurance = grand total, plus remaining loan before/after payment.',
        tags: ['Payments'],
        security: [['sanctum' => []]],
        parameters: [
            new OA\Parameter(name: 'id', description: 'Mortgage request ID', in: 'path', required: true, schema: new OA\Schema(type: 'integer')),
        ],
        responses: [
            new OA\Response(
                response: 200,
                description: 'Payment breakdown',
                content: new OA\JsonContent(
                    properties: [
                        new OA\Property(property: 'monthly_payment', type: 'number', example: 11660833),
                        new OA\Property(property: 'total_tax_amount', type: 'number', example: 1282692),
                        new OA\Property(property: 'insurance', type: 'integer', example: 900000),
                        new OA\Property(property: 'grand_total_amount', type: 'number', example: 13843525),
                        new OA\Property(property: 'remaining_loan_amount', type: 'number', example: 2798600000),
                        new OA\Property(property: 'remaining_loan_amount_after_payment', type: 'number', example: 2786939167),
                    ],
                    type: 'object'
                )
            ),
            new OA\Response(response: 404, description: 'Not found'),
            new OA\Response(response: 401, description: 'Unauthenticated'),
        ]
    )]
    public function breakdown(MortgageRequest $mortgageRequest): JsonResponse
    {
        $details = $this->mortgageService->getInstallmentPaymentDetails($mortgageRequest);

        return response()->json($details);
    }

    #[OA\Post(
        path: '/api/mortgages/{id}/pay',
        operationId: 'initiatePayment',
        summary: 'Initiate installment payment via Midtrans',
        description: 'Creates a Midtrans Snap transaction and returns the snap_token for frontend checkout popup.',
        tags: ['Payments'],
        security: [['sanctum' => []]],
        parameters: [
            new OA\Parameter(name: 'id', description: 'Mortgage request ID', in: 'path', required: true, schema: new OA\Schema(type: 'integer')),
        ],
        responses: [
            new OA\Response(
                response: 200,
                description: 'Snap token generated',
                content: new OA\JsonContent(
                    properties: [
                        new OA\Property(property: 'snap_token', type: 'string', example: 'abc123def456'),
                    ],
                    type: 'object'
                )
            ),
            new OA\Response(response: 500, description: 'Payment failed'),
            new OA\Response(response: 401, description: 'Unauthenticated'),
        ]
    )]
    public function pay(MortgageRequest $mortgageRequest): JsonResponse
    {
        try {
            $snapToken = $this->paymentService->createPayment($mortgageRequest);

            return response()->json(['snap_token' => $snapToken]);
        } catch (\Exception $e) {
            return response()->json(['error' => 'Payment failed: '.$e->getMessage()], 500);
        }
    }
}
