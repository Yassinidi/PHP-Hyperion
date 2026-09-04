<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Factories\HasFactory;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\SoftDeletes;

class MortgageRequest extends Model
{
    use HasFactory, SoftDeletes;

    protected $fillable = [
        'user_id',
        'house_id',
        'interest_id',
        'duration',
        'bank_name',
        'interest',
        'dp_total_amount',
        'loan_total_amount',
        'monthly_amount',
        'dp_percentage',
        'status',
        'documents',
        'loan_interest_total_amount',
        'house_price',
    ];

    public function customer()
    {
        return $this->belongsTo(User::class, 'user_id');
    }

    public function house()
    {
        return $this->belongsTo(House::class, 'house_id');
    }

    // Renamed from interest() to interestModel() to avoid conflict with
    // the 'interest' column (integer) in mortgage_requests table.
    // $mortgageRequest->interest returns the column value (int),
    // $mortgageRequest->interestModel returns the Interest model (with bank relationship).
    public function interestModel()
    {
        return $this->belongsTo(Interest::class, 'interest_id');
    }

    public function installments()
    {
        return $this->hasMany(Installment::class);
    }

    public function getRemainingLoanAmountAttribute()
    {
        // Check if there are any installments
        if ($this->installments()->count() === 0) {
            // Default to the total loan interest amount if no installments exist
            return $this->loan_interest_total_amount;
        }

        // Calculate the total paid amount from installments
        $totalPaid = $this->installments()->where('is_paid', true)->sum('sub_total_amount');

        // Substract the total paid from the total loan amount
        return max($this->loan_interest_total_amount - $totalPaid, 0);
    }
}
