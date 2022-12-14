<?php

namespace Pterodactyl\Exceptions\Http\Server;

use Throwable;
use Pterodactyl\Models\Server;
use Symfony\Component\HttpKernel\Exception\ConflictHttpException;

class ServerStateConflictException extends ConflictHttpException
{
    /**
     * Exception thrown when the server is in an unsupported state for API access or
     * certain operations within the codebase.
     */
    public function __construct(Server $server, Throwable $previous = null)
    {
        $message = 'Este servidor está atualmente em um estado sem suporte, tente novamente mais tarde.';
        if ($server->isSuspended()) {
            $message = 'Este servidor está suspenso no momento e a funcionalidade solicitada não está disponível.';
        } elseif ($server->node->isUnderMaintenance()) {
            $message = 'O node deste servidor está atualmente em manutenção e a funcionalidade solicitada não está disponível.';
        } elseif (!$server->isInstalled()) {
            $message = 'Este servidor ainda não concluiu o processo de instalação, por favor, tente novamente mais tarde.';
        } elseif ($server->status === Server::STATUS_RESTORING_BACKUP) {
            $message = 'Este servidor está restaurando a partir de um backup, tente novamente mais tarde.';
        } elseif (!is_null($server->transfer)) {
            $message = 'Este servidor está sendo transferido para uma nova máquina, tente novamente mais tarde.';
        }

        parent::__construct($message, $previous);
    }
}
