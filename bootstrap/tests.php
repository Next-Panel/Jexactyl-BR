<?php

use Illuminate\Support\Str;
use NunoMaduro\Collision\Provider;
use Illuminate\Contracts\Console\Kernel;
use Symfony\Component\Console\Output\ConsoleOutput;

require __DIR__ . '/../vendor/autoload.php';

$app = require __DIR__ . '/app.php';

/** @var \Pterodactyl\Console\Kernel $kernel */
$kernel = $app->make(Kernel::class);

/*
 * Bootstrap the kernel and prepare application for testing.
 */
$kernel->bootstrap();

// Register the collision service provider so that errors during the test
// setup process are output nicely.
(new Provider())->register();

$output = new ConsoleOutput();

$prefix = 'database.connections.' . config('database.default');
if (!Str::contains(config("$prefix.database"), 'test')) {
    $output->writeln(PHP_EOL . '<error>Não é possível executar o processo de teste em um banco de dados sem teste.</error>');
    $output->writeln(PHP_EOL . '<error>Ambiente está atualmente apontado para: "' . config("$prefix.database") . '".</error>');
    exit(1);
}

/*
 * Perform database migrations and reseeding before continuing with
 * running the tests.
 */
if (!env('SKIP_MIGRATIONS')) {
    $output->writeln(PHP_EOL . '<info>Atualizando banco de dados para testes de integração...</info>');
    $kernel->call('migrate:fresh');

    $output->writeln('<info>Semeando banco de dados para testes de integração...</info>' . PHP_EOL);
    $kernel->call('db:seed');
} else {
    $output->writeln(PHP_EOL . '<comment>Ignorando migrações de banco de dados...</comment>' . PHP_EOL);
}
