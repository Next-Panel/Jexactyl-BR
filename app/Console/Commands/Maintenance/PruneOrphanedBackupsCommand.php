<?php

namespace Pterodactyl\Console\Commands\Maintenance;

use Carbon\CarbonImmutable;
use Illuminate\Console\Command;
use Pterodactyl\Repositories\Eloquent\BackupRepository;

class PruneOrphanedBackupsCommand extends Command
{
    protected $signature = 'p:maintenance:prune-backups {--prune-age=}';

    protected $description = 'Marca todos os backups que não foram concluídos nos últimos "n" minutos como falhados.';

    /**
     * PruneOrphanedBackupsCommand constructor.
     */
    public function __construct(private BackupRepository $backupRepository)
    {
        parent::__construct();
    }

    public function handle()
    {
        $since = $this->option('prune-age') ?? config('backups.prune_age', 360);
        if (!$since || !is_digit($since)) {
            throw new \InvalidArgumentException('The "--prune-age" argument must be a value greater than 0.');
        }

        $query = $this->backupRepository->getBuilder()
            ->whereNull('completed_at')
            ->where('created_at', '<=', CarbonImmutable::now()->subMinutes($since)->toDateTimeString());

        $count = $query->count();
        if (!$count) {
            $this->info('Não há backups órfãos para serem marcados como com falha.');

            return;
        }

        $this->warn("Marcando $count backups que não foram marcados como concluídos nos últimos $since minutos como com falha.");

        $query->update([
            'is_successful' => false,
            'completed_at' => CarbonImmutable::now(),
            'updated_at' => CarbonImmutable::now(),
        ]);
    }
}
