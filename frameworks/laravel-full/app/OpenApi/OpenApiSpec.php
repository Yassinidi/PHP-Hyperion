<?php

namespace App\OpenApi;

use OpenApi\Attributes as OA;

#[OA\Info(
    version: '1.0.0',
    title: 'Tedja API',
    description: 'Official API documentation for Tedja — a property listing & KPR mortgage platform.'
)]
#[OA\Server(url: '/', description: 'API server base URL')]
#[OA\SecurityScheme(
    securityScheme: 'sanctum',
    type: 'apiKey',
    in: 'header',
    name: 'Authorization',
    description: 'Use token format: Bearer {token}'
)]
#[OA\Tag(name: 'Auth', description: 'Authentication endpoints')]
#[OA\Tag(name: 'Houses', description: 'Property listing and details')]
#[OA\Tag(name: 'Categories', description: 'Property categories')]
#[OA\Tag(name: 'Cities', description: 'City locations')]
#[OA\Tag(name: 'Banks', description: 'Bank information and interest rates')]
#[OA\Tag(name: 'Facilities', description: 'Property facilities')]
#[OA\Tag(name: 'Mortgages', description: 'Mortgage applications and calculations')]
#[OA\Tag(name: 'Payments', description: 'Installment payments and Midtrans integration')]
#[OA\Tag(name: 'Webhook', description: 'Payment webhook endpoints')]
class OpenApiSpec {}
