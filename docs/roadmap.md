# Roadmap do Jexactyl

O overlay começa do zero sobre o Jexactyl oficial. A numeração dos patches é
uma **sequência global de quatro dígitos**, já usada por `patches/series`:
cada patch recebe o próximo número livre (`0001`, `0002`, ...) na ordem de
criação, que também é a ordem de aplicação. O número não identifica a onda nem
o domínio; o `--domain` do `jx dump` define apenas o rótulo do nome do
arquivo (`i18n`, `gateway`, `billing`, `core`, `ui`). Não existem faixas
numéricas reservadas por onda.

| Onda | Rótulo | Escopo | Estado |
| ---- | ------ | ------ | ------ |
| 00 | `core` | estrutura do overlay, série, lint, CI e testes | estrutura pronta; suíte Rust verde |
| 01 | `i18n` | pipeline de tradução por categoria | concluída (23/23 categorias; 56/56 unidades) |
| 02 | `gateway` | gateway Mercado Pago sobre o billing v4 | planejada |
| 03 | `billing` | ajustes finos de billing e faturas | planejada |
| 04 | `core` | comportamento geral do painel | planejada |
| 05 | `ui` | interface, marca e templates | planejada |

## Onda 00 - Estrutura

Status: estrutura pronta, **suíte verde**. `make test` roda os testes Rust
(`cargo test --workspace`), incluindo setup, refresh, rebase, aplicação e
reversibilidade da série.

## Onda 01 - i18n

Objetivo: o painel fala português brasileiro, com a tradução organizada por
categoria.

O modelo e o pipeline por categoria: 23 categorias e 56 unidades no catálogo,
uma categoria ativa por vez, revisão 0-10 e gates de verificação. O fluxo, os
comandos e o formato do estado estão em
[`docs/i18n-pipeline.md`](i18n-pipeline.md); o estado vivo da fila fica no
SQLite (`.br-cache/i18n-queue.sqlite`) e o `CACHE.md` e o histórico congelado
da fila antiga.

**Implementado:**

- catálogo e lock (`i18n/catalog.json`, `i18n/catalog.lock.json`) com 23
  categorias, 56 unidades, 565 arquivos-fonte, 428 atribuídos, 137 excluídos
  por 7 globs e zero sobreposições;
- fila persistente em SQLite (`jx i18n-queue`, banco em
  `.br-cache/i18n-queue.sqlite`) com o mesmo contrato da fila antiga;
- suíte da fila e do auditor em Rust (`cargo test -p jx_i18n`), passando;
- auditoria estática (`jx i18n-audit`) com paridade verificada contra a
  implementação anterior;
- política, rubrica e glossário (`i18n/POLICY.md`, `i18n/glossary.tsv`);
- perfis de categoria e agentes especializados (`.opencode/agents/`).

**Concluído:**

- tradução das 23 categorias na ordem do catálogo (56/56 unidades com
  consistência 10), com patches e overlays por estágio/rodada e gates
  registrados nas categorias.

**Cutover do ferramental (concluído):** o motor Rust `jx` implementa a
superfície i18n — `jx i18n-catalog (--write|--check) [--json]`,
`jx i18n-queue <subcomandos>` sobre SQLite em
`.br-cache/i18n-queue.sqlite` (`--db` sobrescreve),
`jx i18n-audit --unit|--category [--json] [--base] [--strict]` e 6 comandos
`jx bridge-*` para compat de harness. `make i18n-catalog`/`i18n-check`/
`i18n-status`/`i18n-test`/`i18n-audit` delegam ao `jx`; os scripts Python e as
suítes JS foram removidos após paridade verificada.

Observação: no v4.0.7 o frontend usa labels inline no TSX; não há uma camada
de locale com i18next para ligar. O componente de tradução do activity log
existe, mas o i18next não é inicializado: essa é uma limitação conhecida do
upstream, não um requisito para traduzir as demais páginas. As mensagens de
backend usam overlays em `resources/lang/pt-BR/` e mantém o fallback `en` do
Laravel.

Critério de saída:

- todas as 23 categorias `done` na fila (SQLite; `CACHE.md` preservado como
  histórico congelado);
- consistência 10 em cada categoria e gates `pass` ou `skip` justificado;
- nenhum comentário de código alterado;
- patches e overlays registrados na fila.

## Onda 02 - Mercado Pago

Objetivo: pagar servidores pelo Mercado Pago, usando o billing do v4.

O gateway antigo do Jexactyl (v3) não é portado: a estrutura `Store` foi
reescrita como `Billing` no v4 e o `app/Services/Store/*` não existe mais. O
gateway novo nasce do `app/Services/Billing/PaymentService.php`, usando o
Stripe como referencia.

Critério de saída:

- checkout criado pelo `PaymentService` v4;
- webhook de pagamento tratado e idempotente;
- fatura gerada e servidor liberado;
- `composer.json` com a dependência do SDK do Mercado Pago.

## Onda 03 - Billing

Ajustes de faturamento necessários: preços em reais, alíquota e regime
tributário, notas, vencimento e suspensão automática.

## Regra de ouro

Nenhum patch é aceito sem passar em `jx lint`, que é o mesmo
gate do CI.
