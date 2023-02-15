<?php

namespace Jexactyl\Exceptions\Service\Database;

use Jexactyl\Exceptions\JexactylException;

class DatabaseClientFeatureNotEnabledException extends JexactylException
{
    public function __construct()
    {
        parent::__construct('A criação do banco de dados do cliente não está habilitada neste painel.');
    }
}
