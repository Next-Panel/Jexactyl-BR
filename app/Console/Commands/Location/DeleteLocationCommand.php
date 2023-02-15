<?php

namespace Jexactyl\Console\Commands\Location;

use Illuminate\Console\Command;
use Illuminate\Support\Collection;
use Jexactyl\Services\Locations\LocationDeletionService;
use Jexactyl\Contracts\Repository\LocationRepositoryInterface;

class DeleteLocationCommand extends Command
{
    protected $description = 'Exclui uma localização do Painel.';

    protected $signature = 'p:location:delete {--short= : O Nome Simples do local a ser excluído.}';

    protected Collection $locations;

    /**
     * DeleteLocationCommand constructor.
     */
    public function __construct(
        private LocationDeletionService $deletionService,
        private LocationRepositoryInterface $repository
    ) {
        parent::__construct();
    }

    /**
     * Respond to the command request.
     *
     * @throws \Jexactyl\Exceptions\Repository\RecordNotFoundException
     * @throws \Jexactyl\Exceptions\Service\Location\HasActiveNodesException
     */
    public function handle()
    {
        $this->locations = $this->locations ?? $this->repository->all();
        $short = $this->option('short') ?? $this->anticipate(
            trans('command/messages.location.ask_short'),
            $this->locations->pluck('short')->toArray()
        );

        $location = $this->locations->where('short', $short)->first();
        if (is_null($location)) {
            $this->error(trans('command/messages.location.no_location_found'));
            if ($this->input->isInteractive()) {
                $this->handle();
            }

            return;
        }

        $this->deletionService->handle($location->id);
        $this->line(trans('command/messages.location.deleted'));
    }
}
