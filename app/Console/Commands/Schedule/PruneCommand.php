<?php

namespace Pterodactyl\Console\Commands\Schedule;

use Pterodactyl\Models\Server;
use Illuminate\Console\Command;
use Pterodactyl\Services\Servers\ServerDeletionService;

class PruneCommand extends Command
{
    /**
     * @var string
     */
    protected $signature = 'p:schedule:prune';

    /**
     * @var string
     */
    protected $description = 'Excluir todos os servidores suspensos.';

    /**
     * DeleteUserCommand constructor.
     */
    public function __construct(private ServerDeletionService $deletionService)
    {
        parent::__construct();
    }

    /**
     * Handle command execution.
     */
    public function handle(Server $server)
    {
        $this->line('Poda de servidor em execução...');
        $this->process($server);
        $this->line('Roteiro concluído com sucesso.');
    }

    /**
     * Takes one day off of the time a server has until it needs to be
     * renewed.
     */
    protected function process(Server $server)
    {
        $servers = $server->where('renewable', true)->get();
        $this->line('Processamento de renovações para ' . $servers->count() . ' servidores.');

        foreach ($servers as $s) {
            $this->line('Processando servidor ' . $s->name . ', ID: ' . $s->id, false);

            if ($s->isSuspended()) {
                $this->line('Excluindo servidor ' . $s->name, false);
                $this->deletionService->withForce(true)->returnResources(true)->handle($s);
            }
        }
    }
}
