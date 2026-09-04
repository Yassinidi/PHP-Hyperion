<?php

namespace Database\Seeders;

use App\Models\User;
use Illuminate\Database\Seeder;
use Spatie\Permission\Models\Role;

class RoleAdminSeeder extends Seeder
{
    /**
     * Run the database seeds.
     */
    public function run(): void
    {
        $adminRole = Role::create(['name' => 'admin']);
        $lenderRole = Role::create(['name' => 'lender']);
        $agentRole = Role::create(['name' => 'agent']);
        $customerRole = Role::create(['name' => 'customer']);

        $user = User::create([
            'name' => 'Admin Tedja',
            'email' => 'admin@admin.com',
            'phone' => 'password',
            'photo' => 'admin.png',
            'password' => bcrypt('password'),
        ]);

        $user->assignRole('admin');
    }
}
