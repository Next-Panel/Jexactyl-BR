<?php

namespace Jexactyl\Exceptions\Service\Backup;

use Jexactyl\Exceptions\DisplayException;

class TooManyBackupsException extends DisplayException
{
    /**
     * TooManyBackupsException constructor.
     */
    public function __construct(int $backupLimit)
    {
        parent::__construct(
            sprintf('Não é possível criar um novo backup, este servidor atingiu seu limite de %d backups.', $backupLimit)
        );
    }
}
