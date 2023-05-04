<?php

namespace Pterodactyl\Http\Controllers\Api\Client\Store;

use Illuminate\Http\Request;
use Illuminate\Http\Response;
use Illuminate\Http\JsonResponse;
use Illuminate\Support\Facades\DB;
use Illuminate\Http\RedirectResponse;
use MercadoPago\SDK;
use MercadoPago\Payment;
use MercadoPago\Preference;
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


        $amount = $request->input('amount');
        if ($amount === '0') {
            throw new Exception('Um Valor deve ser selecionado.');
        }
        $cost = config('gateways.mpago.cost', 1) / 100 * $amount;
        $currency = config('gateways.currency', 'BRL');

        DB::table('mercado_pago')->insert([
            'user_id' => $request->user()->id,
            'amount' => $amount,
        ]);

        SDK::setAccessToken(config('gateways.mpago.access_token'));

        $preference = new Preference();
        $preference->back_urls = [
            'success' => route('api:client:store.mercadopago.callback'),
            'failure' => config('app.url') . '/store/credits',
            'pending' => route('api:client:store.mercadopago.callback'),
        ];
        $preference->auto_return = 'approved';
        $preference->payer = new \MercadoPago\Payer();
        $preference->payer->email = $request->user()->email;
        $item = new \MercadoPago\Item();
        $item->title = $this->settings->get('settings::app:name', 'Jexactyl').' - ' . $amount . ' | ' . ' Creditos';
        $item->quantity = 1;
        $item->unit_price = $cost;
        $preference->items = [$item];
        $preference->save();

        return new JsonResponse($preference->init_point ?? '/', 200, [], null, true);
    }

    /**
     * Add balance to a user when the purchase is successful.
     *
     * @throws DisplayException
     */
    public function callback(Request $request): RedirectResponse
    {
        $user = $request->user();
        $data = DB::table('mercado_pago')->where('user_id', $user->id)->first();

        SDK::setAccessToken(config('gateways.mpago.access_token'));
        
        $payment = Payment::find_by_id($request->input('payment_id'));
        
        if ($payment->status == 'approved') {
            $user->update([
                'store_balance' => $user->store_balance + $data->amount,
            ]);
        }

        DB::table('mercado_pago')->where('user_id', $user->id)->delete();

        return redirect('/store');
    }
}
