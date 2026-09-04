<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    /**
     * Run the migrations.
     */
    public function up(): void
    {
        Schema::create('installments', function (Blueprint $table) {
            $table->id();
            $table->foreignId('mortgage_request_id')->constrained()->cascadeOnDelete();
            $table->integer('no_of_payment');
            $table->integer('total_tax_amount');
            $table->integer('grand_total_amount');
            $table->integer('sub_total_amount');
            $table->integer('insurance_amount');
            $table->string('proof')->nullable();
            $table->boolean('is_paid');
            $table->string('payment_type');
            $table->integer('remaining_loan_amount');
            $table->timestamps();
            $table->softDeletes();
        });
    }

    /**
     * Reverse the migrations.
     */
    public function down(): void
    {
        Schema::dropIfExists('installments');
    }
};
