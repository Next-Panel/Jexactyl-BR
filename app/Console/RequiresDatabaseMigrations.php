<?php

namespace Pterodactyl\Console;

/**
 * @mixin \Illuminate\Console\Command
 */
trait RequiresDatabaseMigrations
{
    /**
     * Checks if the migrations have finished running by comparing the last migration file.
     */
    protected function hasCompletedMigrations(): bool
    {
        /** @var \Illuminate\Database\Migrations\Migrator $migrator */
        $migrator = $this->getLaravel()->make('migrator');

        $files = $migrator->getMigrationFiles(database_path('migrations'));

        if (!$migrator->repositoryExists()) {
            return false;
        }

        if (array_diff(array_keys($files), $migrator->getRepository()->getRan())) {
            return false;
        }

        return true;
    }

    /**
     * Throw a massive error into the console to hopefully catch the users attention and get
     * them to properly run the migrations rather than ignoring all of the other previous
     * errors...
     */
    protected function showMigrationWarning(): void
    {
        $this->getOutput()->writeln('<options=bold>
| @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@ |
|                                                                              |
|               Seu banco de dados não foi devidamente migrado!                  |
|                                                                              |
| @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@ |</>

Você deve executar o seguinte comando para finalizar a migração de seu banco de dados:

  <fg=green;options=bold>php artisan migrate --step --force</>

  Você não poderá usar o Painel do Jexactyl como esperado sem fixar seu
  de banco de dados, executando o comando acima.
');

        $this->getOutput()->error('Você deve corrigir o erro acima antes de continuar.');
    }
}
