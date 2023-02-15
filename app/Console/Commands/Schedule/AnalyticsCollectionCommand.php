<?php

namespace Jexactyl\Console\Commands\Schedule;

use Jexactyl\Models\Server;
use Illuminate\Console\Command;
use Jexactyl\Models\AnalyticsData;
use Jexactyl\Repositories\Wings\DaemonServerRepository;

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

            if (AnalyticsData::where('server_id', $server->id)->count() >= 12) {
                $this->line($server->id . ' excede 12 entradas, eliminando as mais antigas');
                AnalyticsData::where('server_id', $server->id)->orderBy('id', 'asc')->first()->delete();
            }

            try {
                AnalyticsData::create([
                    'server_id' => $server->id,
                    'cpu' => $usage['cpu_absolute'] / ($server->cpu / 100),
                    'memory' => ($usage['memory_bytes'] / 1024) / $server->memory / 10,
                    'disk' => ($usage['disk_bytes'] / 1024) / $server->disk / 10,
                ]);

                $this->line($server->id . ' as análises foram salvas no banco de dados');
            } catch (\Exception $ex) {
                $this->error($server->id . ' não conseguiu escrever as estatísticas: ' . $ex->getMessage());
            }
        }
    }
}
