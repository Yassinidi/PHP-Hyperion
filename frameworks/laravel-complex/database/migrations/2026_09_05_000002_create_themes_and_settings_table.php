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
        Schema::create('themes', function (Blueprint $table) {
            $table->id();
            $table->string('name');
            $table->string('slug')->unique();
            $table->string('description')->nullable();
            $table->boolean('is_active')->default(false)->index();
            $table->string('primary_color', 30)->default('#6366f1');
            $table->string('accent_color', 30)->default('#10b981');
            $table->string('bg_base', 30)->default('#0a0e17');
            $table->string('bg_card', 30)->default('#131b2e');
            $table->string('bg_card_hover', 30)->default('#1b2640');
            $table->string('text_main', 30)->default('#f8fafc');
            $table->string('text_muted', 30)->default('#94a3b8');
            $table->string('border_color', 60)->default('rgba(255, 255, 255, 0.08)');
            $table->string('border_highlight', 60)->default('rgba(99, 102, 241, 0.4)');
            $table->string('border_radius', 20)->default('12px');
            $table->string('font_family', 100)->default("'Plus Jakarta Sans', sans-serif");
            $table->text('custom_css')->nullable();
            $table->timestamps();
        });

        Schema::create('store_settings', function (Blueprint $table) {
            $table->id();
            $table->string('key')->unique();
            $table->text('value')->nullable();
            $table->string('group')->default('general')->index();
            $table->timestamps();
        });
    }

    /**
     * Reverse the migrations.
     */
    public function down(): void
    {
        Schema::dropIfExists('store_settings');
        Schema::dropIfExists('themes');
    }
};
