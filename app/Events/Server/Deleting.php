<?php

namespace Jexactyl\Events\Server;

use Jexactyl\Events\Event;
use Jexactyl\Models\Server;
use Illuminate\Queue\SerializesModels;

class Deleting extends Event
{
    use SerializesModels;

    /**
     * Create a new event instance.
     */
    public function __construct(public Server $server)
    {
    }
}
