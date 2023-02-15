<?php

namespace Jexactyl\Exceptions\Service\User;

use Jexactyl\Exceptions\DisplayException;

class TwoFactorAuthenticationTokenInvalid extends DisplayException
{
    /**
     * TwoFactorAuthenticationTokenInvalid constructor.
     */
    public function __construct()
    {
        parent::__construct('O token de autenticação de dois fatores fornecido não era válido');
    }
}
