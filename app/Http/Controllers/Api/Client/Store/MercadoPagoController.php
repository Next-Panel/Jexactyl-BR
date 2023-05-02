<?php

namespace Pterodactyl\Http\Controllers\Api\Client\Store;

use Illuminate\Http\JsonResponse;
use MercadoPago\SDK;
use MercadoPago\Preference;
use MercadoPago\Exceptions\MercadoPagoException;
use Pterodactyl\Exceptions\DisplayException;
use Pterodactyl\Http\Controllers\Api\Client\ClientApiController;
use Pterodactyl\Http\Requests\Api\Client\Store\Gateways\MercadoPagoRequest;

class MercadoPagoController extends ClientApiController
{
    public function __construct()
    {
        parent::__construct();
    }

    /**
     * @throws DisplayException|MercadoPagoException
     */
    public function purchase(MercadoPagoRequest $request): JsonResponse
    {
        if (!$this->settings->get('jexactyl::store:mpago:enabled')) {
            throw new DisplayException('Não é possível comprar via Mercado Pago: módulo não ativado');
        }

        SDK::setAccessToken(config('gateways.mpago.client_id'));

        $amount = $request->input('amount');
        $cost = number_format(config('gateways.mpago.cost', 1.00) / 100 * $amount, 2);
        $currency = config('gateways.currency', 'BRL');

        $preference = new Preference();

        $preference->back_urls = [
            'success' => config('app.url') . '/store/credits',
            'failure' => config('app.url'),
            'pending' => config('app.url'),
        ];
        $preference->auto_return = 'approved';
        $preference->payment_methods = [
            'excluded_payment_types' => [
                ['id' => 'ticket'],
                ['id' => 'atm'],
            ],
        ];
        $preference->notification_url = config('app.url') . route('api:client:store.mpago.callback');
        $preference->external_reference = $request->user()->id;
        $preference->metadata = [
            'credit_amount' => $amount,
            'user_id' => $request->user()->id,
        ];
        $preference->items = [
            [
                'id' => uniqid(),
                'title' => $amount . ' Créditos | ' . $this->settings->get('settings::app:name'),
                'description' => $amount . ' Créditos',
                'quantity' => 1,
                'unit_price' => floatval($cost),
                'currency_id' => $currency,
            ],
        ];

        $preference->save();

        return new JsonResponse($preference->init_point, 200, [], null, true);
    }
}
