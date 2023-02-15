<?php

namespace Jexactyl\Exceptions\Service\Backup;

use Jexactyl\Exceptions\DisplayException;

class BackupLockedException extends DisplayException
{
    /**
     * TooManyBackupsException constructor.
     */
    public function __construct()
    {
        parent::__construct('Não é possível excluir um backup marcado como bloqueado.');
    }
}
