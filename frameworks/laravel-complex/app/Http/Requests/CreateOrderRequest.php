<?php

namespace App\Http\Requests;

use App\Models\Customer;
use Closure;
use Illuminate\Foundation\Http\FormRequest;

class CreateOrderRequest extends FormRequest
{
    public function authorize(): bool
    {
        return true;
    }

    public function rules(): array
    {
        return [
            'customer_id' => ['required', 'integer', 'exists:customers,id'],
            'items' => [
                'required',
                'array',
                'min:1',
                function (string $attribute, mixed $value, Closure $fail) {
                    if (!is_array($value)) {
                        return;
                    }

                    $customerId = $this->input('customer_id');
                    if (!$customerId) {
                        return;
                    }

                    $customer = Customer::find($customerId);
                    if (!$customer) {
                        return;
                    }

                    $totalQuantity = array_sum(array_column($value, 'quantity'));
                    $maxAllowed = $customer->tier->maxOrderItems();

                    if ($totalQuantity > $maxAllowed) {
                        $fail("Customer tier [{$customer->tier->value}] allows a maximum of {$maxAllowed} items per order (requested: {$totalQuantity}).");
                    }
                },
            ],
            'items.*.product_id' => ['required', 'integer', 'exists:products,id'],
            'items.*.quantity' => ['required', 'integer', 'min:1'],
            'payment_method' => ['nullable', 'string', 'in:credit_card,paypal,stripe,wire'],
        ];
    }

    public function messages(): array
    {
        return [
            'items.required' => 'An order must contain at least one item.',
            'customer_id.exists' => 'The selected customer profile does not exist in our records.',
        ];
    }
}
