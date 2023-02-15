<?php

namespace Jexactyl\Exceptions\Service\Database;

use Jexactyl\Exceptions\DisplayException;

class NoSuitableDatabaseHostException extends DisplayException
{
    /**
     * NoSuitableDatabaseHostException constructor.
     */
    public function __construct()
    {
        parent::__construct('Nenhum host de database foi encontrado que atenda aos requisitos para este servidor.');
    }
}
