<?php

namespace Jexactyl\Console\Commands\Location;

use Illuminate\Console\Command;
use Jexactyl\Services\Locations\LocationCreationService;

class MakeLocationCommand extends Command
{
    protected $signature = 'p:location:make
                            {--short= : O nome do shortcode deste localização (ex. us1).}
                            {--long= : Uma descrição mais longa deste localização.}';

    protected $description = 'Cria um novo local no sistema por meio da CLI.';

    /**
     * Create a new command instance.
     */
    public function __construct(private LocationCreationService $creationService)
    {
        parent::__construct();
    }

    /**
     * Handle the command execution process.
     *
     * @throws \Jexactyl\Exceptions\Model\DataValidationException
     */
    public function handle()
    {
        $short = $this->option('short') ?? $this->ask(trans('command/messages.location.ask_short'));
        $long = $this->option('long') ?? $this->ask(trans('command/messages.location.ask_long'));

        $location = $this->creationService->handle(compact('short', 'long'));
        $this->line(trans('command/messages.location.created', [
            'name' => $location->short,
            'id' => $location->id,
        ]));
    }
}
