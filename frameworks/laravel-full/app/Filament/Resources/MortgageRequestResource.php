<?php

namespace App\Filament\Resources;

use App\Filament\Resources\MortgageRequestResource\Pages;
use App\Filament\Resources\MortgageRequestResource\RelationManagers\InstallmentsRelationManager;
use App\Models\House;
use App\Models\Interest;
use App\Models\MortgageRequest;
use App\Models\User;
use Filament\Forms;
use Filament\Forms\Components\Grid;
use Filament\Forms\Form;
use Filament\Resources\Resource;
use Filament\Tables;
use Filament\Tables\Table;
use Illuminate\Database\Eloquent\Builder;
use Illuminate\Database\Eloquent\SoftDeletingScope;

class MortgageRequestResource extends Resource
{
    protected static ?string $model = MortgageRequest::class;

    protected static ?string $navigationIcon = 'heroicon-o-credit-card';

    protected static ?string $navigationGroup = 'Transactions';

    public static function form(Form $form): Form
    {
        return $form
            ->schema([
                Forms\Components\Wizard::make([
                    Forms\Components\Wizard\Step::make('Product and Price')
                        ->schema([
                            Grid::make(3)
                                ->schema([

                                    Forms\Components\Select::make('house_id')
                                        ->label('House')
                                        ->options(House::query()->pluck('name', 'id'))
                                        ->searchable()
                                        ->preload()
                                        ->required()
                                        ->live()
                                        ->afterStateUpdated(function ($state, callable $set) {
                                            $house = House::find($state);
                                            if ($house) {
                                                $set('house_price', $house->price ?? 0);
                                            }
                                        }),

                                    Forms\Components\Select::make('interest_id')
                                        ->label('Annual Interest in %')
                                        ->options(function (callable $get) {
                                            $houseId = $get('house_id');
                                            if ($houseId) {
                                                return Interest::where('house_id', $houseId)->get()->pluck('interest', 'id');
                                            }

                                            return [];
                                        })
                                        ->searchable()
                                        ->preload()
                                        ->required()
                                        ->live()
                                        ->afterStateUpdated(function ($state, callable $set) {
                                            $interest = Interest::find($state);
                                            if ($interest) {
                                                $set('bank_name', $interest->bank->name ?? '');
                                                $set('interest', $interest->interest ?? 0);
                                                $set('duration', $interest->duration ?? 0);
                                            }
                                        }),

                                    Forms\Components\TextInput::make('bank_name')
                                        ->label('Bank Name')
                                        ->readOnly()
                                        ->required(),

                                    Forms\Components\TextInput::make('duration')
                                        ->label('Duration In Years')
                                        ->readOnly()
                                        ->required()
                                        ->numeric()
                                        ->suffix('Years'),

                                    Forms\Components\TextInput::make('interest')
                                        ->label('Interest Rate')
                                        ->readOnly()
                                        ->required()
                                        ->numeric()
                                        ->suffix('%'),

                                    Forms\Components\TextInput::make('house_price')
                                        ->label('House Price')
                                        ->readOnly()
                                        ->required()
                                        ->numeric()
                                        ->prefix('IDR'),

                                    Forms\Components\Select::make('dp_percentage')
                                        ->label('Down Payment (%)')
                                        ->options([
                                            10 => '10%',
                                            20 => '20%',
                                            40 => '40%',
                                            50 => '50%',
                                            80 => '80%',
                                        ])
                                        ->live()
                                        ->required()
                                        ->afterStateUpdated(function ($state, callable $get, callable $set) {
                                            $housePrice = $get('house_price') ?? 0;
                                            $dpAmount = $housePrice * ($state / 100);
                                            $loanAmount = max($housePrice - $dpAmount, 0);

                                            $set('dp_total_amount', round($dpAmount, 0));
                                            $set('loan_total_amount', round($loanAmount, 0));

                                            // Calculate monthly payment
                                            $durationYears = $get('duration') ?? 0;
                                            $interestRate = $get('interest') ?? 0;

                                            if ($durationYears > 0 && $loanAmount > 0 && $interestRate > 0) {
                                                $totalPayment = $durationYears * 12; // Total Number of Payments
                                                $monthlyInterestRate = $interestRate / 100 / 12; // Monthly Interest Rate

                                                // Amortization Formula
                                                $numerator = $loanAmount * $monthlyInterestRate * pow(1 + $monthlyInterestRate, $totalPayment);
                                                $denominator = pow(1 + $monthlyInterestRate, $totalPayment) - 1;
                                                $monthlyPayment = $denominator > 0 ? $numerator / $denominator : 0;

                                                $set('monthly_amount', round($monthlyPayment));

                                                // Total Loan With Interest
                                                $loanInterestTotalAmount = $monthlyPayment * $totalPayment;
                                                $set('loan_interest_total_amount', round($loanInterestTotalAmount));
                                            } else {
                                                $set('monthly_amount', 0);
                                                $set('loan_interest_total_amount', 0);
                                            }
                                        }),

                                    // Down Payment Amount
                                    Forms\Components\TextInput::make('dp_total_amount')
                                        ->label('Down Payment Amount')
                                        ->readOnly()
                                        ->required()
                                        ->numeric()
                                        ->prefix('IDR'),

                                    // Loan Amount
                                    Forms\Components\TextInput::make('loan_total_amount')
                                        ->label('Loan Amount')
                                        ->readOnly()
                                        ->required()
                                        ->numeric()
                                        ->prefix('IDR'),

                                    // Monthly Amount
                                    Forms\Components\TextInput::make('monthly_amount')
                                        ->label('Monthly Payment')
                                        ->readOnly()
                                        ->required()
                                        ->numeric()
                                        ->prefix('IDR'),

                                    // Total Payment Amount Field
                                    Forms\Components\TextInput::make('loan_interest_total_amount')
                                        ->label('Total Payment Amount')
                                        ->readOnly()
                                        ->numeric()
                                        ->prefix('IDR'),
                                ]),
                        ]),

                    Forms\Components\Wizard\Step::make('Customer Information')
                        ->schema([
                            Forms\Components\Select::make('user_id')
                                ->relationship('customer', 'email')
                                ->searchable()
                                ->preload()
                                ->required()
                                ->live()
                                ->afterStateUpdated(function ($state, callable $set) {
                                    $user = User::find($state);

                                    if ($user) {
                                        $name = $user->name;
                                        $email = $user->email;

                                        $set('name', $name);
                                        $set('email', $email);
                                    } else {
                                        // Clear the fields when user is not selected
                                        $set('name', null);
                                        $set('email', null);
                                    }
                                })
                                ->afterStateUpdated(function (callable $set, $state) {
                                    $userId = $state;
                                    if ($userId) {
                                        $user = User::find($userId);
                                        if ($user) {
                                            $name = $user->name;
                                            $email = $user->email;
                                            $set('name', $name);
                                            $set('email', $email);
                                        }
                                    } else {
                                        // Clear the fields when user is not selected
                                        $set('name', null);
                                        $set('email', null);
                                    }
                                }),

                            Forms\Components\TextInput::make('name')
                                ->label('Customer Name')
                                ->maxLength(255)
                                ->readOnly()
                                ->required()
                                ->afterStateHydrated(function ($state, $record, callable $set) {
                                    // For view/edit: populate name from the related user
                                    if ($record && empty($state) && $record->customer) {
                                        $set('name', $record->customer->name);
                                    }
                                }),

                            Forms\Components\TextInput::make('email')
                                ->label('Customer Email')
                                ->maxLength(255)
                                ->readOnly()
                                ->required()
                                ->afterStateHydrated(function ($state, $record, callable $set) {
                                    // For view/edit: populate email from the related user
                                    if ($record && empty($state) && $record->customer) {
                                        $set('email', $record->customer->email);
                                    }
                                }),
                        ]),

                    Forms\Components\Wizard\Step::make('Bank Approval')
                        ->schema([
                            Forms\Components\FileUpload::make('documents')
                                ->acceptedFileTypes(['application/pdf'])
                                ->required(),

                            Forms\Components\Select::make('status')
                                ->label('ApprovalStatus')
                                ->options([
                                    'Waiting for Bank' => 'Waiting for Bank',
                                    'approved' => 'Approved',
                                    'rejected' => 'Rejected',
                                ])
                                ->required(),
                        ]),
                ])
                    ->columnSpan('full')
                    ->columns(1)
                    ->skippable(),
            ]);
    }

    public static function table(Table $table): Table
    {
        return $table
            ->columns([
                Tables\Columns\ImageColumn::make('house.thumbnail'),
                Tables\Columns\TextColumn::make('customer.name')->searchable(),
                Tables\Columns\TextColumn::make('house.name')->searchable(),
                Tables\Columns\TextColumn::make('status'),
            ])
            ->filters([
                Tables\Filters\TrashedFilter::make(),
            ])
            ->actions([
                Tables\Actions\ViewAction::make(),
                Tables\Actions\EditAction::make(),
                Tables\Actions\Action::make('download_documents')
                    ->label('Download')
                    ->icon('heroicon-o-arrow-down-tray')
                    ->url(fn (MortgageRequest $record) => asset('storage/'.$record->documents))  // Generate a URL to the file
                    ->openUrlInNewTab(),  // Open the file in a new tab
            ])
            ->bulkActions([
                Tables\Actions\BulkActionGroup::make([
                    Tables\Actions\DeleteBulkAction::make(),
                    Tables\Actions\ForceDeleteBulkAction::make(),
                    Tables\Actions\RestoreBulkAction::make(),
                ]),
            ]);
    }

    public static function getRelations(): array
    {
        return [
            InstallmentsRelationManager::class,
        ];
    }

    public static function getPages(): array
    {
        return [
            'index' => Pages\ListMortgageRequests::route('/'),
            'create' => Pages\CreateMortgageRequest::route('/create'),
            'view' => Pages\ViewMortgageRequest::route('/{record}'),
            'edit' => Pages\EditMortgageRequest::route('/{record}/edit'),
        ];
    }

    public static function getEloquentQuery(): Builder
    {
        return parent::getEloquentQuery()
            ->withoutGlobalScopes([
                SoftDeletingScope::class,
            ]);
    }
}
