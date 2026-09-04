<?php

namespace App\Services;

use App\Models\Installment;
use App\Models\Interest;
use App\Models\MortgageRequest;
use Illuminate\Support\Facades\Auth;

class MortgageService
{
    public function handleInterestRequest($request)
    {
        $validatedData = $request->validate([
            'dp_percentage' => 'required|integer|min:0|max:100',
            'interest_id' => 'required|integer|exists:interests,id',
            'documents' => 'required|file|mimes:pdf|max:2048',
        ]);

        $interest = Interest::findorfail($validatedData['interest_id']);
        $house = $interest->house;

        $mortgageDetails = $this->calculateMortgageDetails($house, $interest, $validatedData['dp_percentage']);

        $documentPath = $this->uploadDocuments($request);

        return $this->createMortgageRequest($mortgageDetails, $documentPath);
    }

    public function calculateMortgageDetails($house, $interest, $dpPercentage)
    {
        $housePrice = $house->price;
        $dpTotalAmount = $housePrice * ($dpPercentage / 100);
        $loanTotalAmount = $housePrice - $dpTotalAmount;
        $durationYears = $interest->duration;
        $totalPayments = $durationYears * 12; // Total number of monthly payments
        $monthlyInterestRate = $interest->interest / 100 / 12; // Monthly interest rate

        // Amortization formula for monthly payment
        $numerator = $loanTotalAmount * $monthlyInterestRate * pow(1 + $monthlyInterestRate, $totalPayments);
        $denominator = pow(1 + $monthlyInterestRate, $totalPayments) - 1;
        $monthlyAmount = $denominator > 0 ? $numerator / $denominator : 0;

        $loanInterestTotalAmount = $monthlyAmount * $totalPayments;

        return compact(
            'house',
            'interest',
            'housePrice',
            'dpTotalAmount',
            'dpPercentage',
            'loanTotalAmount',
            'monthlyAmount',
            'loanInterestTotalAmount'
        );
    }

    public function uploadDocuments($request)
    {
        if ($request->hasFile('documents')) {
            return $request->file('documents')->store('documents', 'public');
        }

        return null;
    }

    public function createMortgageRequest($details, $documentPath)
    {
        $mortgageRequest = MortgageRequest::create([
            'user_id' => Auth::id(),
            'house_id' => $details['house']->id,
            'interest_id' => $details['interest']->id,
            'interest' => $details['interest']->interest,
            'duration' => $details['interest']->duration,
            'bank_name' => $details['interest']->bank->name,
            'dp_percentage' => $details['dpPercentage'],
            'house_price' => $details['housePrice'],
            'dp_total_amount' => $details['dpTotalAmount'],
            'loan_total_amount' => $details['loanTotalAmount'],
            'loan_interest_total_amount' => $details['loanInterestTotalAmount'],
            'monthly_amount' => $details['monthlyAmount'],
            'status' => 'Waiting for Bank',
            'documents' => $documentPath,
        ]);

        session(['interest_id' => $details['interest']->id]);

        return $mortgageRequest;
    }

    public function getInterestFromSession()
    {
        $interestId = session('interest_id');

        return $interestId ? Interest::findorfail($interestId) : null;
    }

    public function getUserMortgages($userId, $search = null)
    {
        $query = MortgageRequest::with(['house', 'house.city', 'house.category'])
            ->where('user_id', $userId);

        if ($search) {
            $query->where(function ($q) use ($search) {
                $q->where('status', 'like', "%{$search}%")
                    ->orWhere('bank_name', 'like', "%{$search}%")
                    ->orWhereHas('house', function ($q) use ($search) {
                        $q->where('name', 'like', "%{$search}%");
                    })
                    ->orWhereHas('house.city', function ($q) use ($search) {
                        $q->where('name', 'like', "%{$search}%");
                    })
                    ->orWhereHas('house.category', function ($q) use ($search) {
                        $q->where('name', 'like', "%{$search}%");
                    });
            });
        }

        return $query->get();
    }

    public function getMortgageDetails(MortgageRequest $mortgageRequest)
    {
        $mortgageRequest->load(['house', 'house.city', 'house.category', 'installments']);
        $monthlyPayemnt = $mortgageRequest->monthly_amount;
        $insurance = 900000;
        $totalTaxAmount = round($monthlyPayemnt * 0.11);

        return compact('mortgageRequest', 'totalTaxAmount', 'insurance');
    }

    public function getInstallmentDetails(Installment $installment)
    {
        return $installment->load(['mortgageRequest.house.city']);
    }

    public function getInstallmentPaymentDetails(MortgageRequest $mortgageRequest)
    {

        $remainingLoanAmount = $mortgageRequest->remaining_loan_amount;
        $mortgageRequest->load(['house.city', 'house.category']);

        $monthlyPayment = $mortgageRequest->monthly_amount;
        $insurance = 900000;
        $totalTaxAmount = round($monthlyPayment * 0.11);
        $grandTotalAmount = $monthlyPayment + $totalTaxAmount + $insurance;
        $remainingLoanAmountAfterPayment = $remainingLoanAmount - $monthlyPayment;

        return compact(
            'mortgageRequest',
            'grandTotalAmount',
            'monthlyPayment',
            'totalTaxAmount',
            'insurance',
            'remainingLoanAmount',
            'remainingLoanAmountAfterPayment'
        );
    }

    public function getMortgageRequest($mortgageRequestId)
    {
        return MortgageRequest::findorfail($mortgageRequestId);
    }
}
