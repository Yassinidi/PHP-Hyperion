<?php

use Illuminate\Support\Facades\Artisan;

Artisan::command('inspire', function () {
    $this->comment('Hyperion: High Performance PHP Runtime');
})->purpose('Display an inspiring quote');
