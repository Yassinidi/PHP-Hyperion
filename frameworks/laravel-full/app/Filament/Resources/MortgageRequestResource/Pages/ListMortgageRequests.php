<?php

namespace App\Filament\Resources\MortgageRequestResource\Pages;

use App\Filament\Resources\MortgageRequestResource;
use Filament\Actions;
use Filament\Resources\Pages\ListRecords;

class ListMortgageRequests extends ListRecords
{
    protected static string $resource = MortgageRequestResource::class;

    protected function getHeaderActions(): array
    {
        return [
            Actions\CreateAction::make(),
        ];
    }
}
