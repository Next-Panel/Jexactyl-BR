<?php

namespace Everest\Services\Billing;

use Everest\Exceptions\DisplayException;
use Illuminate\Support\Facades\Http;

// Cliente HTTP para a API do Mercado Pago (Checkout Pro), sem SDK externo.
// Mantem o patch livre de novas dependencias do composer.
class MercadoPagoPaymentService
{
    public const BASE_URL = 'https://api.mercadopago.com';

    public const STATUS_APPROVED = 'approved';

    public const STATUS_PENDING = ['pending', 'in_process', 'authorized'];

    public function accessToken(): string
    {
        return (string) config('modules.billing.mercadopago.access_token', '');
    }

    public function enabled(): bool
    {
        return $this->accessToken() !== '';
    }

    public function assertEnabled(): void
    {
        if (!$this->enabled()) {
            throw new DisplayException('Unable to use Mercado Pago: module not enabled or access token missing.');
        }
    }

    /**
     * Cria uma preferencia do Checkout Pro e devolve o payload da resposta.
     */
    public function createPreference(array $body): array
    {
        $this->assertEnabled();

        $response = Http::baseUrl(self::BASE_URL)
            ->withToken($this->accessToken())
            ->acceptJson()
            ->post('/checkout/preferences', $body);

        if ($response->failed() || !is_array($response->json())) {
            throw new DisplayException('Failed to create Mercado Pago preference.');
        }

        return $response->json();
    }

    /**
     * Atualiza a referencia externa de uma preferencia ja criada.
     */
    public function updateExternalReference(string $preferenceId, string $externalReference): void
    {
        $this->assertEnabled();

        $response = Http::baseUrl(self::BASE_URL)
            ->withToken($this->accessToken())
            ->acceptJson()
            ->put("/checkout/preferences/{$preferenceId}", [
                'external_reference' => $externalReference,
            ]);

        if ($response->failed()) {
            throw new DisplayException('Failed to update Mercado Pago preference.');
        }
    }

    /**
     * Busca um pagamento pelo id.
     */
    public function getPayment(string $paymentId): array
    {
        $this->assertEnabled();

        $response = Http::baseUrl(self::BASE_URL)
            ->withToken($this->accessToken())
            ->acceptJson()
            ->get("/v1/payments/{$paymentId}");

        if ($response->failed() || !is_array($response->json())) {
            throw new DisplayException('Failed to retrieve Mercado Pago payment.');
        }

        return $response->json();
    }

    /**
     * Procura o pagamento aprovado mais recente vinculado a referencia externa.
     */
    public function findApprovedPayment(string $externalReference): ?array
    {
        $this->assertEnabled();

        $response = Http::baseUrl(self::BASE_URL)
            ->withToken($this->accessToken())
            ->acceptJson()
            ->get('/v1/payments/search', [
                'external_reference' => $externalReference,
                'sort' => 'date_created',
                'criteria' => 'desc',
            ]);

        if ($response->failed()) {
            throw new DisplayException('Failed to search Mercado Pago payments.');
        }

        foreach ((array) ($response->json('results', [])) as $payment) {
            if (!is_array($payment)) {
                continue;
            }
            if (($payment['status'] ?? null) === self::STATUS_APPROVED) {
                return $payment;
            }
        }

        return null;
    }

    /**
     * Indica se existe ao menos um pagamento pendente para a referencia.
     */
    public function hasPendingPayment(string $externalReference): bool
    {
        $this->assertEnabled();

        $response = Http::baseUrl(self::BASE_URL)
            ->withToken($this->accessToken())
            ->acceptJson()
            ->get('/v1/payments/search', [
                'external_reference' => $externalReference,
                'sort' => 'date_created',
                'criteria' => 'desc',
            ]);

        if ($response->failed()) {
            return false;
        }

        foreach ((array) ($response->json('results', [])) as $payment) {
            if (!is_array($payment)) {
                continue;
            }
            if (in_array($payment['status'] ?? null, self::STATUS_PENDING, true)) {
                return true;
            }
        }

        return false;
    }

    /**
     * Valida a assinatura do webhook (cabecalho x-signature) com o segredo.
     */
    public function validSignature(string $requestId, string $dataId, ?string $header): bool
    {
        $secret = (string) config('modules.billing.mercadopago.webhook_secret', '');

        if ($secret === '' || $header === null || $header === '') {
            return false;
        }

        $parts = [];
        foreach (explode(',', $header) as $chunk) {
            $pair = explode('=', trim($chunk), 2);
            if (count($pair) === 2) {
                $parts[trim($pair[0])] = trim($pair[1]);
            }
        }

        if (!isset($parts['ts'], $parts['v1'])) {
            return false;
        }

        $manifest = "id:{$dataId};request-id:{$requestId};ts:{$parts['ts']};";
        $expected = hash_hmac('sha256', $manifest, $secret);

        return hash_equals($expected, $parts['v1']);
    }
}
