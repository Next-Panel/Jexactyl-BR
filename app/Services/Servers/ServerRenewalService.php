<?php

namespace Pterodactyl\Services\Servers;

use Pterodactyl\Models\Server;
use Pterodactyl\Exceptions\DisplayException;
use Pterodactyl\Http\Requests\Api\Client\ClientApiRequest;
use Pterodactyl\Contracts\Repository\SettingsRepositoryInterface;

class ServerRenewalService
{
    private SuspensionService $suspensionService;
    private SettingsRepositoryInterface $settings;

    /**
     * ServerRenewalService constructor.
     */
    public function __construct(
        SuspensionService $suspensionService,
        SettingsRepositoryInterface $settings
    ) {
        $this->settings = $settings;
        $this->suspensionService = $suspensionService;
    }

    /**
     * Renews a server.
     *
     * @throws DisplayException
     */
    public function handle(ClientApiRequest $request, Server $server): Server
    {
        $user = $request->user();
        $cost = $this->settings->get('jexactyl::renewal:cost', 200);

        if ($user->store_balance < $cost) {
            throw new DisplayException('Você não possui créditos suficientes para renovar seu servidor.');
        }

        try {
            $user->update(['store_balance' => $user->store_balance - $cost]);
            $server->update(['renewal' => $server->renewal + $this->settings->get('jexactyl::renewal:default', 7)]);
        } catch (DisplayException $ex) {
            throw new DisplayException('Ocorreu um erro inesperado ao tentar renovar seu servidor.');
        }

        if ($server->status == 'suspended' && $server->renewal >= 0) {
            $this->suspensionService->toggle($server, 'unsuspend');
        }

        return $server->refresh();
    }
}
