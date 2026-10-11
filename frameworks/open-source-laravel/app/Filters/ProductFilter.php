<?php

namespace App\Filters;

class ProductFilter extends ApiFilter
{
    protected $safeParms = [
        'name' => ['eq'],
        'sku' => ['eq'],
        'status' => ['eq', 'ne'],
        'quantity' => ['eq','gte','lte'],
        'price' => ['eq','gte','lte', 'lt', 'gt'],
        'publishedAt' => ['eq','gt','lt'],
    ];

    protected $columnMap = [
        'publishedAt' => 'published_at',
    ];

    protected $operatorMap = [
        'eq' => '=',
        'lt' => '<',
        'lte' => '<=',
        'gt' => '>',
        'gte' => '>=',
        'in' => 'in',
        'ne'=> '!='
    ];
}