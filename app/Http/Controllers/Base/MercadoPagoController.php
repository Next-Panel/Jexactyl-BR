<?php

namespace Pterodactyl\Http\Controllers\Base;

use Illuminate\Http\Request;
use Illuminate\Http\Response;
use Pterodactyl\Models\User;
use Pterodactyl\Http\Controllers\Controller;

class MercadoPagoController extends Controller
{
    public function index(Request $request): Response
    {
        $paymentId = $request->input('data.id');
        $paymentStatus = $request->input('type');

        if ($paymentStatus !== 'payment') {
            return new Response('Ignored - Unhandled Event', 200);
        }

        $payment = $this->getPaymentInfo($paymentId);

        if ($payment->status === 'approved') {
            $bal = User::query()->select('store_balance')->where('id', '=', $payment->external_reference)->first()->store_balance;
            User::query()->where('id', '=', $payment->external_reference)->update(['store_balance' => $bal + $payment->metadata->credit_amount]);
            return new Response('Success - Credits Added', 200);
        }

        return new Response('Failed - Payment Issue', 200);
    }

    private function getPaymentInfo($paymentId) {
        $accessToken = config('gateways.mpago.client_id');
        $url = 'https://api.mercadopago.com/v1/payments/' . $paymentId . '?access_token=' . $accessToken;

        $ch = curl_init();
        curl_setopt($ch, CURLOPT_URL, $url);
        curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
        curl_setopt($ch, CURLOPT_FOLLOWLOCATION, true);
        curl_setopt($ch, CURLOPT_SSL_VERIFYPEER, false);
        curl_setopt($ch, CURLOPT_TIMEOUT, 30);
        $response = curl_exec($ch);
        curl_close($ch);

        return json_decode($response);
    }
}
