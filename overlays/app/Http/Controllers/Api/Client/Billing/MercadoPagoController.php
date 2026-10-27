<?php

namespace Everest\Http\Controllers\Api\Client\Billing;

use Everest\Models\Egg;
use Everest\Models\Node;
use Everest\Models\User;
use Everest\Models\Server;
use Everest\Models\Billing\Order;
use Illuminate\Http\Request;
use Illuminate\Http\JsonResponse;
use Illuminate\Support\Facades\DB;
use Everest\Models\Billing\Product;
use Everest\Exceptions\DisplayException;
use Everest\Models\Billing\DiscountCode;
use Everest\Models\Billing\BillingException;
use Everest\Services\Billing\DiscountService;
use Everest\Services\Billing\CreateOrderService;
use Everest\Services\Billing\ServerRenewalService;
use Everest\Services\Billing\ServerDeploymentService;
use Everest\Services\Billing\MercadoPagoPaymentService;
use Illuminate\Database\Eloquent\ModelNotFoundException;
use Everest\Http\Controllers\Api\Client\ClientApiController;
use Everest\Transformers\Api\Client\ServerTransformer;
use Everest\Http\Requests\Api\Client\Billing\CreateMercadoPagoPaymentRequest;
use Everest\Http\Requests\Api\Client\Billing\ProcessMercadoPagoPaymentRequest;

// Checkout via Mercado Pago (Checkout Pro). Espelha o StripeController para
// pedidos novos e renovacoes; upgrades seguem no Stripe (fase 2).
class MercadoPagoController extends ClientApiController
{
    public function __construct(
        private MercadoPagoPaymentService $payments,
        private DiscountService $discountService,
        private CreateOrderService $orderService,
        private ServerRenewalService $renewalService,
        private ServerDeploymentService $deploymentService,
    ) {
        parent::__construct();
    }

    /**
     * Cria uma preferencia do Checkout Pro e devolve a URL de pagamento
     * com o id da preferencia (o frontend guarda para o processamento).
     */
    public function create(CreateMercadoPagoPaymentRequest $request): JsonResponse
    {
        $this->payments->assertEnabled();
        $this->assertBrlCurrency();

        $price = null;
        $server = null;
        $deploymentFee = 0.0;
        $node_id = $request->input('node_id') ?? null;
        $product = Product::findOrFail($request->input('product_id'));

        if (!$product->isPaid()) {
            throw new DisplayException('You cannot create a checkout session for a free product.');
        }

        if ($node_id) {
            $node = Node::findOrFail($node_id);

            if (!$node->deployable) {
                throw new DisplayException('Paid servers cannot be deployed to this node.');
            }

            $deploymentFee = (float) ($node->deployment_fee ?? 0);
        } else {
            try {
                $server = $request->user()->servers()
                    ->where('id', $request->input('server_id'))
                    ->firstOrFail();
            } catch (ModelNotFoundException $exception) {
                throw new DisplayException('This server ID does not exist on your account.');
            }
        }

        $order_type = $server ? Order::TYPE_RENEWAL : Order::TYPE_NEW;
        $egg_id = $this->resolveEggSelection($product, $request->input('egg_id'));

        $discountCode = $request->input('discount_code');

        if ($request->exists('discount_code')) {
            $price = $this->discountService->handle($product, $discountCode);
        }

        $total = ($price ?? $product->price) + $deploymentFee;

        // Contexto do checkout gravado no pedido: o process/webhook leem
        // daqui (o Mercado Pago nao devolve metadata arbitraria como o Stripe).
        $metadata = [
            'user_id' => (string) $request->user()->id,
            'customer_email' => $request->user()->email,
            'product_id' => (string) $product->id,
            'node_id' => (string) ($node_id ?? ''),
            'server_id' => (string) ($server?->id ?? 0),
            'egg_id' => (string) ($egg_id ?? ''),
            'variables' => json_encode($request->input('variables') ?? []),
            'order_type' => $order_type,
            'discount_code' => $request->input('discount_code') ?? null,
        ];

        $orderMetadata = array_filter([
            'deployment_fee' => $deploymentFee > 0 ? $deploymentFee : null,
            'discount_code' => $price !== null ? $discountCode : null,
            'subtotal' => $price !== null ? $product->price : null,
        ], fn ($value) => $value !== null) ?: [];

        $order = $this->orderService->create(
            null,
            $request->user(),
            $product,
            Order::STATUS_PENDING,
            $order_type,
            $price,
            array_merge($orderMetadata, $metadata),
        );

        if ($server) {
            $order->assignServer($server);
        }

        try {
            $preference = $this->payments->createPreference(
                $this->preferencePayload($request, $product, $total, $deploymentFee, (string) $order->id)
            );
        } catch (DisplayException $exception) {
            $order->setStatus(Order::STATUS_FAILED);
            throw $exception;
        }

        $preferenceId = (string) ($preference['id'] ?? '');
        $initPoint = $this->initPoint($preference);

        if ($preferenceId === '' || $initPoint === '') {
            $order->setStatus(Order::STATUS_FAILED);
            throw new DisplayException('Failed to create Mercado Pago preference.');
        }

        $order->update(['transaction_id' => $preferenceId]);

        // Vincula a preferencia ao pedido para reconciliacao via webhook/busca.
        try {
            $this->payments->updateExternalReference($preferenceId, (string) $order->id);
        } catch (DisplayException $exception) {
            // Nao e fatal: o process localiza pelo transaction_id.
        }

        return new JsonResponse(['url' => $initPoint, 'preference_id' => $preferenceId]);
    }

    /**
     * Confere o pagamento aprovado e processa o pedido (idempotente).
     */
    public function process(ProcessMercadoPagoPaymentRequest $request): array
    {
        $this->payments->assertEnabled();

        $preferenceId = (string) $request->input('preference_id');

        $order = Order::where('transaction_id', $preferenceId)->first();

        if (!$order) {
            throw new DisplayException('Payment session not found.');
        }

        if ($order->status !== Order::STATUS_PENDING) {
            throw new DisplayException('This order has already been processed.');
        }

        // Verifica no Mercado Pago antes de reivindicar: falhas externas
        // deixam o pedido em pending para nova tentativa.
        $payment = $this->payments->findApprovedPayment((string) $order->id);

        if ($payment === null) {
            if ($this->payments->hasPendingPayment((string) $order->id)) {
                throw new DisplayException('Payment is pending approval. Try again once it clears.');
            }

            throw new DisplayException('Payment not completed.');
        }

        $this->assertPaymentMatchesOrder($payment, $order);

        $claimed = $this->claimPendingOrder($order);

        if ($claimed === null) {
            throw new DisplayException('This order has already been processed.');
        }

        return $this->fulfill($claimed, $payment);
    }

    /**
     * Webhook do Mercado Pago (IPN): confirma pagamentos assincronos (Pix).
     */
    public function webhook(Request $request): JsonResponse
    {
        $this->payments->assertEnabled();

        $type = $request->input('type', $request->input('topic'));
        $dataId = (string) ($request->input('data.id') ?? $request->input('id', ''));

        if ($type === 'merchant_order') {
            return new JsonResponse(['received' => true]);
        }

        if ($type !== 'payment' || $dataId === '') {
            return new JsonResponse(['received' => true]);
        }

        $signature = $request->header('x-signature');
        $requestId = (string) ($request->header('x-request-id', ''));

        if (!$this->payments->validSignature($requestId, $dataId, $signature)) {
            return new JsonResponse(['error' => 'invalid signature'], 401);
        }

        try {
            $payment = $this->payments->getPayment($dataId);
        } catch (DisplayException $exception) {
            return new JsonResponse(['received' => true]);
        }

        if (($payment['status'] ?? null) !== MercadoPagoPaymentService::STATUS_APPROVED) {
            return new JsonResponse(['received' => true]);
        }

        $externalReference = (string) ($payment['external_reference'] ?? '');

        if ($externalReference === '' || !ctype_digit($externalReference)) {
            return new JsonResponse(['received' => true]);
        }

        $order = Order::where('id', (int) $externalReference)->first();

        if (!$order || $order->status !== Order::STATUS_PENDING) {
            return new JsonResponse(['received' => true]);
        }

        try {
            $this->assertPaymentMatchesOrder($payment, $order);
        } catch (DisplayException $exception) {
            $order->setStatus(Order::STATUS_PENDING);

            return new JsonResponse(['received' => true]);
        }

        $claimed = $this->claimPendingOrder($order);

        if ($claimed === null) {
            return new JsonResponse(['received' => true]);
        }

        try {
            $this->fulfill($claimed, $payment);
        } catch (DisplayException $exception) {
            return new JsonResponse(['received' => true]);
        }

        return new JsonResponse(['processed' => true]);
    }

    /**
     * Reivindica o pedido para fulfillment (lock + consumo do desconto).
     * Devolve null quando outro processo ja o reivindicou.
     */
    private function claimPendingOrder(Order $order): ?Order
    {
        return DB::transaction(function () use ($order) {
            $claimed = Order::where('id', $order->id)->lockForUpdate()->first();

            if (!$claimed || $claimed->status !== Order::STATUS_PENDING) {
                return null;
            }

            $claimed->setStatus(Order::STATUS_PROCESSING);

            if (!empty($claimed->metadata['discount_code'] ?? null)) {
                $discount_code = DiscountCode::where('code', $claimed->metadata['discount_code'])->lockForUpdate()->first();

                if ($discount_code && $discount_code->isValid()) {
                    $discount_code->use();
                }
            }

            return $claimed;
        });
    }

    /**
     * Executa deploy/renovacao e marca o pedido como processado.
     */
    private function fulfill(Order $order, array $payment): array
    {
        $metadata = $order->metadata ?? [];

        try {
            $server = isset($metadata['server_id']) ? Server::find($metadata['server_id']) : null;

            if ($server === null) {
                $server = $order->server;
            }

            $user = User::findOrFail($order->user_id);
            $product = Product::findOrFail($order->product_id);

            switch ($order->type) {
                case Order::TYPE_RENEWAL:
                    if ($server === null) {
                        throw new DisplayException('The server for this order no longer exists.');
                    }
                    $server = $this->renewalService->handle($server);
                    break;
                case Order::TYPE_NEW:
                    $server = $this->deploymentService->handle($user, $product, $metadata, $order);
                    $order->assignServer($server);
                    break;
                default:
                    break;
            }

            $order->setStatus(Order::STATUS_PROCESSED);
        } catch (DisplayException $exception) {
            $this->failOrder($order, $exception->getMessage());
            throw $exception;
        } catch (\Throwable $exception) {
            $this->failOrder($order, $exception->getMessage());
            throw new DisplayException('Failed to fulfill the order.');
        }

        return $this->transform($server, ServerTransformer::class);
    }

    private function failOrder(Order $order, string $message): void
    {
        $order->setStatus(Order::STATUS_FAILED);

        BillingException::create([
            'order_id' => $order->id,
            'exception_type' => BillingException::TYPE_DEPLOYMENT,
            'title' => __('exceptions.billing.deployment_renewal_failed_title'),
            'description' => $message,
        ]);
    }

    private function assertBrlCurrency(): void
    {
        if (strtolower((string) config('modules.billing.currency.code', '')) !== 'brl') {
            throw new DisplayException('Mercado Pago requires the billing currency to be BRL.');
        }
    }

    private function assertPaymentMatchesOrder(array $payment, Order $order): void
    {
        $expected = (int) round($order->total * 100);
        $paid = (int) round((float) ($payment['transaction_amount'] ?? 0) * 100);
        $currency = strtolower((string) ($payment['currency_id'] ?? ''));

        if ($currency !== 'brl' || $paid < $expected) {
            throw new DisplayException('Payment amount or currency does not match the order.');
        }
    }

    private function preferencePayload(Request $request, Product $product, float $total, float $deploymentFee, string $externalReference): array
    {
        $baseUrl = rtrim((string) config('app.url'), '/');

        return [
            'items' => [[
                'title' => $product->name,
                'quantity' => 1,
                'unit_price' => round($total, 2),
                'currency_id' => 'BRL',
            ]],
            'payer' => ['email' => $request->user()->email],
            'external_reference' => $externalReference,
            'statement_descriptor' => mb_substr((string) config('app.name', 'Jexactyl'), 0, 22),
            'back_urls' => [
                'success' => "{$baseUrl}/account/billing/processing?gateway=mercadopago",
                'failure' => "{$baseUrl}/account/billing/cancel",
                'pending' => "{$baseUrl}/account/billing/processing?gateway=mercadopago",
            ],
            'auto_return' => 'approved',
            'notification_url' => "{$baseUrl}/api/client/billing/mercadopago/webhook",
            'metadata' => [
                'deployment_fee' => $deploymentFee,
                'product_id' => $product->id,
            ],
        ];
    }

    private function initPoint(array $preference): string
    {
        $sandbox = (bool) config('modules.billing.mercadopago.sandbox', false);

        if ($sandbox) {
            return (string) ($preference['sandbox_init_point'] ?? $preference['init_point'] ?? '');
        }

        return (string) ($preference['init_point'] ?? '');
    }

    private function resolveEggSelection(Product $product, mixed $submittedEggId): ?int
    {
        if ($product->category->egg_id) {
            return null;
        }

        if (!$submittedEggId) {
            throw new DisplayException('An egg must be selected to deploy this product.');
        }

        $egg = Egg::findOrFail($submittedEggId);

        if ((int) $egg->nest_id !== (int) $product->category->nest_id) {
            throw new DisplayException('The selected egg does not belong to this product\'s nest.');
        }

        return $egg->id;
    }
}
