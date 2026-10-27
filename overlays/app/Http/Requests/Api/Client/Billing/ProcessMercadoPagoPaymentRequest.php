<?php

namespace Everest\Http\Requests\Api\Client\Billing;

use Everest\Http\Requests\Api\Client\ClientApiRequest;

class ProcessMercadoPagoPaymentRequest extends ClientApiRequest
{
    public function rules(): array
    {
        return [
            'preference_id' => 'required|string',
        ];
    }
}
