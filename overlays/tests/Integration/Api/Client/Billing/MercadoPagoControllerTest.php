<?php

namespace Everest\Tests\Integration\Api\Client\Billing;

use Ramsey\Uuid\Uuid;
use Everest\Models\Billing\Order;
use Everest\Models\Billing\Product;
use Everest\Models\Billing\Category;
use Illuminate\Support\Facades\Http;
use Everest\Services\Billing\MercadoPagoPaymentService;
use Everest\Tests\Traits\Integration\CreatesBillingTestModels;
use Everest\Tests\Integration\Api\Client\ClientApiIntegrationTestCase;

class MercadoPagoControllerTest extends ClientApiIntegrationTestCase
{
    use CreatesBillingTestModels;

    public function setUp(): void
    {
        parent::setUp();

        $this->enableBilling();
        config([
            'modules.billing.currency.code' => 'BRL',
            'modules.billing.currency.symbol' => 'R$',
            'modules.billing.mercadopago.access_token' => 'TEST-123',
            'modules.billing.mercadopago.webhook_secret' => 'whsec_test',
        ]);
    }

    protected function tearDown(): void
    {
        $this->cleanupBillingModels();

        parent::tearDown();
    }

    private function fakePreference(string $preferenceId = 'pref_123'): void
    {
        Http::fake([
            'api.mercadopago.com/checkout/preferences' => Http::response([
                'id' => $preferenceId,
                'init_point' => "https://www.mercadopago.com.br/checkout/v1/redirect?pref_id={$preferenceId}",
                'sandbox_init_point' => "https://sandbox.mercadopago.com.br/checkout/v1/redirect?pref_id={$preferenceId}",
            ], 201),
            'api.mercadopago.com/checkout/preferences/*' => Http::response(['id' => $preferenceId], 200),
        ]);
    }

    private function fakePaymentSearch(array $results): void
    {
        Http::fake([
            'api.mercadopago.com/v1/payments/search*' => Http::response(['results' => $results], 200),
        ]);
    }

    public function testCreateReturnsInitPointAndPendingOrder(): void
    {
        $user = \Everest\Models\User::factory()->create();
        $product = $this->makeProduct(['price' => 50.00]);
        $node = \Everest\Models\Node::factory()->create(['deployable' => true, 'deployment_fee' => 5.00]);

        $this->fakePreference('pref_create');

        $response = $this->actingAs($user)->postJson('/api/client/billing/mercadopago/create', [
            'product_id' => $product->id,
            'node_id' => $node->id,
        ]);

        $response->assertOk();
        $response->assertJsonPath('preference_id', 'pref_create');

        $order = Order::where('transaction_id', 'pref_create')->firstOrFail();
        $this->assertSame(Order::STATUS_PENDING, $order->status);
        $this->assertEquals(55.00, $order->total);
    }

    public function testCreateRequiresBrlCurrency(): void
    {
        config(['modules.billing.currency.code' => 'USD']);

        $user = \Everest\Models\User::factory()->create();
        $product = $this->makeProduct(['price' => 50.00]);
        $node = \Everest\Models\Node::factory()->create(['deployable' => true]);

        $response = $this->actingAs($user)->postJson('/api/client/billing/mercadopago/create', [
            'product_id' => $product->id,
            'node_id' => $node->id,
        ]);

        $response->assertStatus(400);
    }

    public function testMatchingApprovedPaymentIsAccepted(): void
    {
        $user = \Everest\Models\User::factory()->create();
        $server = $this->createServerModel(['user_id' => $user->id, 'renewal_date' => now()->subDays(30)]);
        $product = $this->makeProduct(['price' => 100.00]);
        $server->update(['billing_product_id' => $product->id]);

        $order = Order::create([
            'name' => Uuid::uuid4()->toString(),
            'transaction_id' => 'pref_accept',
            'user_id' => $user->id,
            'description' => 'renewal',
            'total' => 100.00,
            'status' => Order::STATUS_PENDING,
            'product_id' => $product->id,
            'type' => Order::TYPE_RENEWAL,
            'metadata' => ['server_id' => (string) $server->id],
        ]);
        $order->assignServer($server);

        $this->fakePaymentSearch([[
            'id' => 111,
            'status' => 'approved',
            'transaction_amount' => 100.00,
            'currency_id' => 'BRL',
            'external_reference' => (string) $order->id,
        ]]);

        $response = $this->actingAs($user)->postJson('/api/client/billing/mercadopago/process', [
            'preference_id' => 'pref_accept',
        ]);

        $response->assertOk();
        $this->assertSame(Order::STATUS_PROCESSED, $order->refresh()->status);
    }

    public function testMismatchedAmountPaymentIsRejected(): void
    {
        $user = \Everest\Models\User::factory()->create();
        $server = $this->createServerModel(['user_id' => $user->id, 'renewal_date' => now()->subDays(30)]);
        $product = $this->makeProduct(['price' => 100.00]);
        $server->update(['billing_product_id' => $product->id]);

        $order = Order::create([
            'name' => Uuid::uuid4()->toString(),
            'transaction_id' => 'pref_reject',
            'user_id' => $user->id,
            'description' => 'renewal',
            'total' => 100.00,
            'status' => Order::STATUS_PENDING,
            'product_id' => $product->id,
            'type' => Order::TYPE_RENEWAL,
            'metadata' => ['server_id' => (string) $server->id],
        ]);
        $order->assignServer($server);

        $this->fakePaymentSearch([[
            'id' => 222,
            'status' => 'approved',
            'transaction_amount' => 10.00,
            'currency_id' => 'BRL',
            'external_reference' => (string) $order->id,
        ]]);

        $response = $this->actingAs($user)->postJson('/api/client/billing/mercadopago/process', [
            'preference_id' => 'pref_reject',
        ]);

        $response->assertStatus(400);
        $this->assertSame(Order::STATUS_PENDING, $order->refresh()->status);
    }

    public function testPendingPaymentKeepsOrderPending(): void
    {
        $user = \Everest\Models\User::factory()->create();
        $product = $this->makeProduct(['price' => 100.00]);

        $order = Order::create([
            'name' => Uuid::uuid4()->toString(),
            'transaction_id' => 'pref_pending',
            'user_id' => $user->id,
            'description' => 'new',
            'total' => 100.00,
            'status' => Order::STATUS_PENDING,
            'product_id' => $product->id,
            'type' => Order::TYPE_NEW,
        ]);

        // Nenhum pagamento aprovado, mas ha um Pix pendente.
        $this->fakePaymentSearch([[
            'id' => 333,
            'status' => 'pending',
            'transaction_amount' => 100.00,
            'currency_id' => 'BRL',
            'external_reference' => (string) $order->id,
        ]]);

        $response = $this->actingAs($user)->postJson('/api/client/billing/mercadopago/process', [
            'preference_id' => 'pref_pending',
        ]);

        $response->assertStatus(400);
        $this->assertSame(Order::STATUS_PENDING, $order->refresh()->status);
    }

    public function testWebhookRejectsInvalidSignature(): void
    {
        $response = $this->postJson('/api/client/billing/mercadopago/webhook', [
            'type' => 'payment',
            'data' => ['id' => '111'],
        ], ['x-signature' => 'ts=1,v1=invalid', 'x-request-id' => 'req-1']);

        $response->assertStatus(401);
    }

    public function testWebhookIgnoresNonPaymentEvents(): void
    {
        $response = $this->postJson('/api/client/billing/mercadopago/webhook', [
            'type' => 'merchant_order',
            'data' => ['id' => '999'],
        ]);

        $response->assertOk();
    }

    public function testWebhookFulfillsApprovedPayment(): void
    {
        $user = \Everest\Models\User::factory()->create();
        $server = $this->createServerModel(['user_id' => $user->id, 'renewal_date' => now()->subDays(30)]);
        $product = $this->makeProduct(['price' => 100.00]);
        $server->update(['billing_product_id' => $product->id]);

        $order = Order::create([
            'name' => Uuid::uuid4()->toString(),
            'transaction_id' => 'pref_webhook',
            'user_id' => $user->id,
            'description' => 'renewal',
            'total' => 100.00,
            'status' => Order::STATUS_PENDING,
            'product_id' => $product->id,
            'type' => Order::TYPE_RENEWAL,
            'metadata' => ['server_id' => (string) $server->id],
        ]);
        $order->assignServer($server);

        Http::fake([
            'api.mercadopago.com/v1/payments/777' => Http::response([
                'id' => 777,
                'status' => 'approved',
                'transaction_amount' => 100.00,
                'currency_id' => 'BRL',
                'external_reference' => (string) $order->id,
            ], 200),
        ]);

        $manifest = 'id:777;request-id:req-9;ts:456;';
        $signature = 'ts=456,v1=' . hash_hmac('sha256', $manifest, 'whsec_test');

        $response = $this->postJson('/api/client/billing/mercadopago/webhook', [
            'type' => 'payment',
            'data' => ['id' => '777'],
        ], ['x-signature' => $signature, 'x-request-id' => 'req-9']);

        $response->assertOk();
        $response->assertJsonPath('processed', true);
        $this->assertSame(Order::STATUS_PROCESSED, $order->refresh()->status);
    }

    public function testPreferenceFailureMarksOrderFailed(): void
    {
        $user = \Everest\Models\User::factory()->create();
        $product = $this->makeProduct(['price' => 50.00]);
        $node = \Everest\Models\Node::factory()->create(['deployable' => true]);

        Http::fake([
            'api.mercadopago.com/checkout/preferences' => Http::response(['message' => 'bad'], 400),
        ]);

        $response = $this->actingAs($user)->postJson('/api/client/billing/mercadopago/create', [
            'product_id' => $product->id,
            'node_id' => $node->id,
        ]);

        $response->assertStatus(400);
        $this->assertSame(0, Order::where('status', Order::STATUS_PENDING)->count());
    }

    public function testCreateUsesSandboxInitPoint(): void
    {
        config(['modules.billing.mercadopago.sandbox' => true]);

        $user = \Everest\Models\User::factory()->create();
        $product = $this->makeProduct(['price' => 50.00]);
        $node = \Everest\Models\Node::factory()->create(['deployable' => true]);

        $this->fakePreference('pref_sandbox');

        $response = $this->actingAs($user)->postJson('/api/client/billing/mercadopago/create', [
            'product_id' => $product->id,
            'node_id' => $node->id,
        ]);

        $response->assertOk();
        $this->assertStringContainsString('sandbox.mercadopago.com.br', (string) $response->json('url'));
    }

    public function testSignatureValidation(): void
    {
        $service = new MercadoPagoPaymentService();
        $manifest = 'id:111;request-id:req-1;ts:123;';
        $valid = 'ts=123,v1=' . hash_hmac('sha256', $manifest, 'whsec_test');

        $this->assertTrue($service->validSignature('req-1', '111', $valid));
        $this->assertFalse($service->validSignature('req-1', '111', 'ts=123,v1=wrong'));
        $this->assertFalse($service->validSignature('req-1', '111', null));
    }
}
