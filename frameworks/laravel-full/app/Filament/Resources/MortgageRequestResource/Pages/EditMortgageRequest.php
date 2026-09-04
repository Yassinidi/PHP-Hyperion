<?php

namespace App\Filament\Resources\MortgageRequestResource\Pages;

use App\Filament\Resources\MortgageRequestResource;
use Filament\Actions;
use Filament\Resources\Pages\EditRecord;

class EditMortgageRequest extends EditRecord
{
    protected static string $resource = MortgageRequestResource::class;

    protected function getHeaderActions(): array
    {
        return [
            Actions\DeleteAction::make(),
            Actions\ForceDeleteAction::make(),
            Actions\RestoreAction::make(),
        ];
    }
}
