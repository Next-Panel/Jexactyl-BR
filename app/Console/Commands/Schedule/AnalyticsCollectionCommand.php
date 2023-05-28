<?php

namespace Pterodactyl\Console\Commands\Schedule;

use Pterodactyl\Models\Server;
use Illuminate\Console\Command;
use Pterodactyl\Models\AnalyticsData;
use Pterodactyl\Repositories\Wings\DaemonServerRepository;

class AnalyticsCollectionCommand extends Command
{
    /**
     * @var string
     */
    protected $signature = 'p:schedule:analytics';

    /**
     * @var string
     */
    protected $description = 'Coletar análises sobre o desempenho do servidor.';

    /**
     * AnalyticsCollectionCommand constructor.
     */
    public function __construct(private DaemonServerRepository $repository)
    {
        parent::__construct();
    }

    /**
     * Handle command execution.
     */
    public function handle()
    {
        foreach (Server::all() as $server) {
            $this->line($server->id . ' está sendo processado');

            $stats = $this->repository->setServer($server)->getDetails();
            $usage = $stats['utilization'];

            if ($stats['state'] === 'offline') {
                $this->line($server->id . ' está offline, pulando');
                continue;
            }

            if (AnalyticsData::where('server_id', $server->id)->count() >= 6) {
                $this->line($server->id . ' excede 6 entradas, eliminando as mais antigas');
                AnalyticsData::where('server_id', $server->id)->orderBy('created_at', 'asc')->first()->delete();
            }

            try {
                AnalyticsData::create([
                    'server_id' => $server->id,
                    'cpu' => $usage['cpu_absolute'] / ($server->cpu / 100),
                    'memory' => ($usage['memory_bytes'] / 1024) / $server->memory / 10,
                    'disk' => ($usage['disk_bytes'] / 1024) / $server->disk / 10,
                ]);

                $this->line($server->id . ' as análises foram salvas no Database');
            } catch (\Exception $ex) {
                $this->error($server->id . ' não conseguiu escrever as estatísticas: ' . $ex->getMessage());
            }
        }
    }
}
