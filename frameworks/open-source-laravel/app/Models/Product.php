<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Factories\HasFactory;
use Illuminate\Database\Eloquent\Model;

class Product extends Model
{
    use HasFactory;

    protected $fillable = [
        'name',
        'sku',
        'barcode',
        'description',
        'published_at',       
        'status',
        'price',
        'quantity',
    ];
    
    protected $casts = [
        'status' => 'bool',
        'option_values' => 'json',
        'published_at' => 'date:Y-m-d H:i'
    ];

    protected $dates = ['published_at'];

 
}