<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up()
    {
        Schema::create('logger_table', function (Blueprint $table) {
            $table->id();
            $table->text('message')->nullable();
            $table->string('channel')->nullable();
            $table->enum('level', ['emergency', 'alert', 'critical', 'error', 'warning', 'notice', 'info', 'debug']);
            $table->string('level_name', 20);
            $table->integer('unix_time');
            $table->string('datetime')->nullable();
            $table->longText('context')->nullable();
            $table->text('extra')->nullable();
            $table->timestamps();
        });
    }
};
