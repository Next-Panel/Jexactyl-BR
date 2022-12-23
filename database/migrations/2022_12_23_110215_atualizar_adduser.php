<?php

use Illuminate\Database\Schema\Blueprint;
use Illuminate\Database\Migrations\Migration;

class AddUsers extends Migration
{
    /**
     * Run the migrations.
     */
    public function up()
    {
        Schema::table('users', function (Blueprint $table) {
            $table->char('language', 5)->default('pt');
        });
    }

    /**
     * Reverse the migrations.
     */
    public function down()
    {
        {
            Schema::table('users', function (Blueprint $table) {
                $table->char('language', 5)->default('en');
            });
        }
    }
}
