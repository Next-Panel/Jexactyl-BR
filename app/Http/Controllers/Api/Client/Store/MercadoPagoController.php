<?php

namespace Pterodactyl\Http\Controllers\Api\Client\Store;

use Carbon\Carbon;
use MercadoPago\SDK;
use MercadoPago\Preference;
use Illuminate\Http\Request;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\RedirectResponse;
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
     * Constructs the MercadoPago preference and redirects
     * the user over to MercadoPago for credits purchase.
     *
     * @throws DisplayException
     */
    public function purchase(MercadoPagoRequest $request): JsonResponse
    {
        if ($this->settings->get('jexactyl::store:mpago:enabled') != 'true') {
            throw new DisplayException('Não é possível comprar via MercadoPago: módulo não ativado');
        }

        if (config('gateways.mpago.access_token') === '') {
            throw new DisplayException('Não é possível comprar via MercadoPago: Token não configurado.');
        }

        if (!str_contains(config('app.url'), 'https://')) {
            throw new DisplayException('Não é possível comprar via MercadoPago: APP_URL não está com HTTPS, exigido pelo Mercado Pago.');
        }

        if (str_ends_with(config('app.url'), '/')) {
            throw new DisplayException('Não é possível comprar via MercadoPago: APP_URL possue uma "/" no final do link, remova, exigido pelo Mercado Pago.');
        }

        $amount = $request->input('amount');
        $cost = config('gateways.cost', 1.00) / 100 * $amount;
        $currency = config('gateways.currency', 'CLP');

        $token = $this->generateToken();

        SDK::setAccessToken(config('gateways.mpago.access_token'));

        $preference = new Preference();
        $preference->back_urls = [
            'success' => route('api:client:store.mercadopago.callback'),
            'failure' => route('api:client:store.mercadopago.callback'),
            'pending' => route('api:client:store.mercadopago.callback'),
        ];
        $preference->notification_url = config('app.url') . '/mercadopago/listen';
        $preference->payer = new \MercadoPago\Payer();
        $preference->payer->email = $request->user()->email;
        $item = new \MercadoPago\Item();
        $item->title = $this->settings->get('settings::app:name', 'Jexactyl') . ' - ' . $amount . ' |  Creditos';
        $item->quantity = 1;
        $item->unit_price = $cost;
        $item->currency = $currency;
        $preference->items = [$item];
        $preference->metadata = [
            'credit_amount' => $amount,
            'user_id' => $request->user()->id,
            'user_email' => $request->user()->email,
            'internal_token' => $token,
        ];
        $preference->save();

        return new JsonResponse($preference->init_point ?? '/', 200, [], null, true);
    }

    private function generateToken(): string
    {
        // Gerar uma sequência de 20 caracteres aleatórios
        $token = '';
        $characters = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';
        $charactersLength = strlen($characters);

        for ($i = 0; $i < 20; ++$i) {
            $token .= $characters[random_int(0, $charactersLength - 1)];
        }

        // Adicionar informações de data no token
        $now = Carbon::now();
        $token .= '_' . $now->format('YmdHis');

        return $token;
    }

    public function callback(Request $request): RedirectResponse
    {
        return redirect('/store/credits');
    }
}
