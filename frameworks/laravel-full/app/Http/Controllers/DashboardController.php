<?php

namespace App\Http\Controllers;

use App\Models\Installment;
use App\Models\MortgageRequest;
use App\Services\MortgageService;
use App\Services\PaymentService;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\Auth;
use OpenApi\Attributes as OA;

class DashboardController extends Controller
{
    protected $paymentService;

    protected $mortgageService;

    public function __construct(PaymentService $paymentService, MortgageService $mortgageService)
    {
        $this->paymentService = $paymentService;
        $this->mortgageService = $mortgageService;
    }

    public function index(Request $request)
    {
        $userId = Auth::id();
        $search = $request->input('search');
        $mortgages = $this->mortgageService->getUserMortgages($userId, $search);

        return view('customer.mortgages.index', compact('mortgages'));
    }

    public function details(MortgageRequest $mortgageRequest)
    {
        $details = $this->mortgageService->getMortgageDetails($mortgageRequest);

        return view('customer.mortgages.details', $details);
    }

    public function installment_details(Installment $installment)
    {
        $installmentDetails = $this->mortgageService->getInstallmentDetails($installment);

        return view('customer.installments.details', compact('installmentDetails'));
    }

    public function installment_payment(MortgageRequest $mortgageRequest)
    {
        $paymentDetails = $this->mortgageService->getInstallmentPaymentDetails($mortgageRequest);

        return view('customer.installments.pay_installment', $paymentDetails);
    }

    public function payment_store_midtrans(Request $request)
    {
        try {
            $mortgageRequest = $this->mortgageService->getMortgageRequest($request->input('mortgage_request_id'));
            $snapToken = $this->paymentService->createPayment($mortgageRequest);

            return response()->json(['snap_token' => $snapToken], 200);
        } catch (\Exception $e) {
            return response()->json(['error' => 'Payment failed: '.$e->getMessage()], 500);
        }
    }

    #[OA\Post(
        path: '/api/webhook/midtrans',
        operationId: 'handleMidtransWebhook',
        summary: 'Handle Midtrans payment webhook callback',
        description: 'Processes Midtrans callback payload and updates installment status accordingly.',
        tags: ['Webhook'],
        requestBody: new OA\RequestBody(
            required: true,
            content: new OA\JsonContent(
                required: ['order_id', 'transaction_status'],
                properties: [
                    new OA\Property(property: 'order_id', type: 'string', example: 'INSTALLMENT-1-001'),
                    new OA\Property(property: 'transaction_status', type: 'string', example: 'settlement'),
                    new OA\Property(property: 'payment_type', type: 'string', nullable: true, example: 'bank_transfer'),
                    new OA\Property(property: 'gross_amount', type: 'number', nullable: true, example: 12090000),
                ],
                type: 'object'
            )
        ),
        responses: [
            new OA\Response(
                response: 200,
                description: 'Callback processed successfully',
                content: new OA\JsonContent(
                    properties: [
                        new OA\Property(property: 'status', type: 'string', example: 'success'),
                    ],
                    type: 'object'
                )
            ),
            new OA\Response(
                response: 500,
                description: 'Callback processing failed',
                content: new OA\JsonContent(
                    properties: [
                        new OA\Property(property: 'error', type: 'string', example: 'Failed to process notification: ...'),
                    ],
                    type: 'object'
                )
            ),
        ]
    )]
    public function payment_midtrans_notification(Request $request)
    {
        try {
            $this->paymentService->processNotification();

            return response()->json(['status' => 'success'], 200);
        } catch (\Exception $e) {
            return response()->json(['error' => 'Failed to process notification: '.$e->getMessage()], 500);
        }
    }
}
