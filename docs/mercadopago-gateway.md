# Gateway Mercado Pago (Checkout Pro)

Integração do Mercado Pago ao módulo de faturamento (produtos/pedidos),
espelhando o fluxo do Stripe: `create` gera o checkout, `process` confirma
o pagamento e executa deploy/renovação. Sem SDK externo (HTTP direto).

## Escopo do MVP

- Pedidos novos e renovações via Mercado Pago (Pix, cartão, boleto).
- Moeda: `BRL` (o controller recusa outra moeda com erro explícito).
- Upgrades de plano seguem no Stripe (fase 2).
- Webhook implementado para pagamentos assíncronos (Pix pendente).

## Configuração

Variáveis de ambiente (também editáveis pela tela Configurações do admin):

- `BILLING_GATEWAY=stripe|mercadopago` — gateway pré-selecionado no checkout.
- `MP_ACCESS_TOKEN` — token de acesso (nunca exposto ao frontend).
- `MP_WEBHOOK_SECRET` — segredo para validar `x-signature` do IPN.
- `MP_SANDBOX=true` — usa `sandbox_init_point` para homologação.

No admin (Faturamento → Configurações), o bloco **Provedores de pagamento**
mostra o estado de cada gateway (configurado ou não) com ações
Configurar/Redefinir, além do seletor de **gateway padrão** (pré-seleção do
checkout). Enquanto nenhum provedor estiver configurado, a página abre um
diálogo neutro para **escolher qual provedor configurar** (nunca cai direto
no formulário da Stripe); o mesmo fluxo aparece na Visão geral. Os diálogos
de configuração abrem somente por clique ou escolha. As chaves do Stripe
continuam independentes; o checkout oferece os gateways configurados e
pré-seleciona o padrão (`BILLING_GATEWAY`).

## Fluxo

1. `POST /api/client/billing/mercadopago/create` — valida produto/node/egg/
   desconto, cria o pedido `pending`, cria a preferência com
   `external_reference = order.id` e devolve `{url, preference_id}`.
   O frontend guarda `preference_id` no `localStorage` e redireciona.
2. Retorno: `back_urls.success/pending` → `/account/billing/processing?
   gateway=mercadopago`; o frontend chama
   `POST /mercadopago/process {preference_id}`.
3. `process` — reivindica o pedido (`lockForUpdate`, só `pending`),
   busca pagamento **aprovado** por `external_reference`, confere
   valor/moeda e executa o mesmo fulfillment do Stripe
   (deploy/renovação idempotentes). Sem pagamento aprovado, o pedido
   volta a `pending`; se houver pagamento pendente, avisa para aguardar.
4. `POST /mercadopago/webhook` (público, sem auth) — valida `x-signature`,
   busca o pagamento e processa pedidos `pending` (cobre Pix aprovado
   depois do retorno). Idempotente pelo claim do pedido.

## Arquivos

Novos (overlays): `MercadoPagoPaymentService`,
`MercadoPagoController`, `Create/ProcessMercadoPagoPaymentRequest`,
`guides/SetupMercadoPago.tsx`, teste `MercadoPagoControllerTest`.

Alterados (patch domínio `02-gateway`): `config/modules/billing.php`,
`SettingsServiceProvider`, `EverestComposer`, rotas client/application,
`BillingController::resetMercadoPagoKeys`, `state/everest.ts`,
`orders/process.ts`, `PaymentButton` (seletor), `Processing`, `Cancel`
(texto neutro de gateway), `SettingsContainer`, `admin/billing.ts`.

## Homologação

1. `MP_SANDBOX=true` + credenciais de teste; criar produto BRL.
2. Cartão aprovado → pedido `processed` + servidor implantado.
3. Pix pendente → `process` mantém `pending` com aviso; após aprovação,
   webhook ou nova tentativa conclui (sem duplicar deploy).
4. Valor divergente → pedido volta a `pending`, sem fulfillment.
5. Webhook com assinatura inválida → `401`, sem efeito.
6. `php artisan test --filter MercadoPagoControllerTest`.
