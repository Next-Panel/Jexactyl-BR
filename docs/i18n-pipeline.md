# Pipeline de tradução (i18n)

## 1. Objetivo

Traduzir todas as páginas do painel para português brasileiro em **categorias**,
com agentes especializados por estágio. Cada categoria é dividida em unidades;
cada unidade percorre análise, tradução, revisão, correção, consistência e
verificação antes de ser considerada pronta.

O pipeline não cria uma camada de locale no frontend. No Jexactyl v4.0.7 o
painel e uma SPA React com os labels **inline** no TSX; a tradução é feita no
próprio arquivo. As mensagens do backend usam overlays em
`resources/lang/pt-BR/` sobre os arquivos de `resources/lang/en/`.

## 2. Estado atual

> Snapshot de outubro/2026, mantido como registro histórico. A fila foi
> concluída (23/23) e o ferramental migrou para `jx i18n-*` (ver seção 6);
> números atuais do lock: 565 arquivos, 428 atribuídos, 137 excluídos.

- **23 categorias**, **56 unidades** e **565 arquivos-fonte** no catálogo.
- **428 arquivos atribuídos** e **137 arquivos excluídos** por **7 globs** de
  exclusão em `i18n/catalog.lock.json`.
- **Zero sobreposições** e **zero arquivos não cobertos**: 428 + 137 = 565.
- (Snapshot de outubro/2026) No congelamento deste registro, 15/56 unidades
  estavam concluídas (`shared-navigation`, `shared-ui-core`,
  `shared-billing`, `shared-runtime`, `backend-messages` e `auth`) e a
  categoria ativa era `account-overview`, em `analysis`. A fila foi concluída
  em seguida (23/23).
- A auditoria estática estava implementada e a suíte passava, assim como os
  testes da fila (ferramental Python/JS, removido após cutover para o `jx`).
- O frontend v4.0.7 é uma SPA React com labels **inline** no TSX e sem
  inicialização de i18next; a tradução é feita no próprio arquivo.
- As mensagens de backend são publicadas como overlays em
  `overlays/resources/lang/pt-BR/`.

O catálogo é a fonte de verdade do escopo:

| Arquivo | Papel |
|---------|-------|
| `i18n/catalog.json` | manifesto editável: categorias, unidades, globs e exclusões |
| `i18n/catalog.lock.json` | lock gerado: arquivos expandidos, estatísticas e policy |
| `i18n/categories/<id>.md` | perfil legível da categoria, gerado sob demanda por `jx i18n-catalog --write` (não versionado) |
| `i18n/POLICY.md` | política de labels, tom, rubrica e definition of done |
| `i18n/glossary.tsv` | terminologia `en` -> `pt-BR` |
| `CACHE.md` | histórico gerado da fila antiga (congelado; não editar) |
| `.br-cache/reports/` | relatórios JSON por categoria/unidade/estágio |

## 3. Categorias

A ordem abaixo é a ordem de execução. A fila ativa uma categoria por vez e só
libera a próxima quando a anterior chega a `done`.

| # | Categoria | Escopo | Unidades |
|---|-----------|--------|----------|
| 1 | `shared-navigation` | Navegação compartilhada | 3 |
| 2 | `shared-ui-core` | Elementos de interface | 5 |
| 3 | `shared-billing` | Elementos de billing | 1 |
| 4 | `shared-runtime` | Mensagens de runtime | 1 |
| 5 | `backend-messages` | Mensagens do backend | 4 |
| 6 | `auth` | Autenticação | 1 |
| 7 | `account-overview` | Conta - visão geral | 2 |
| 8 | `account-security` | Conta - segurança | 3 |
| 9 | `account-tickets` | Conta - tickets | 1 |
| 10 | `account-billing` | Conta - billing | 2 |
| 11 | `server-shared` | Servidor - compartilhado | 2 |
| 12 | `server-console` | Servidor - console | 1 |
| 13 | `server-data` | Servidor - dados | 3 |
| 14 | `server-configuration` | Servidor - configuração | 4 |
| 15 | `server-billing` | Servidor - billing | 1 |
| 16 | `server-activity` | Servidor - atividade | 1 |
| 17 | `admin-shared` | Admin - compartilhado | 1 |
| 18 | `admin-general` | Admin - geral | 4 |
| 19 | `admin-management` | Admin - gerenciamento | 6 |
| 20 | `admin-modules` | Admin - módulos | 4 |
| 21 | `admin-appearance` | Admin - aparência | 3 |
| 22 | `admin-services` | Admin - serviços | 2 |
| 23 | `admin-setup` | Admin - setup | 1 |
| | **Total** | **23 categorias** | **56** |

### Caso especial: `server-configuration`

A categoria 14 agrupa quatro telas que o menu do servidor apresenta em
"Schedules", "Users", "Network" e "Startup". As unidades são:

- `server/schedules` - Agendamentos;
- `server/users` - Subusuarios;
- `server/network` - Rede e alocações;
- `server/startup` - Inicialização e variáveis.

Elas compartilham o mesmo estágio e a mesma rodada de consistência; não são
categorias independentes.

## 4. Fluxo

```
pending
  -> analysis -> translation -> review <-> correction
                                      |
                                      +-> consistency -> verification -> done
```

| Estágio | O que acontece |
|---------|----------------|
| `analysis` | inventário de labels, dependências compartilhadas, candidatos a glossário, impacto em testes e riscos. Somente leitura. |
| `translation` | edição apenas dos arquivos da unidade, apenas labels visíveis. |
| `review` | rubrica de 0 a 10; relatório JSON com `unit`, `round`, `score`, `criteria`, `findings`, `untranslated`, `glossary_violations` e `test_impact`. |
| `correction` | corrige somente os achados da revisão; não amplia escopo. |
| `consistency` | revisão da categoria inteira por um ou três revisores read-only dedicados: glossário, tom, navegação e coerência entre unidades. |
| `verification` | auditoria estática + testes + typecheck + lint + build. |
| `done` | categoria encerrada; a próxima categoria pendente é ativada automaticamente. |

A revisão e a correção alternam até a nota **10**. A nota 10 exige zero
achados: qualquer achado, inclusive `nit`, impede o 10.

Na fila, `correction` não é um estágio de categoria: a unidade fica em
`fixing` e a categoria permanece em `review`. Por isso `advance` não é usado
entre revisão e correção; o ciclo é controlado pelos claims de `reviewer` e
`fixer`.

A consistência **não é uma unidade reivindicável**. Quando todas as unidades
chegam a `done`, o orquestrador spawna um revisor separado e somente leitura,
`.opencode/agents/i18n-consistency.md`, que revisa a categoria inteira (opcionalmente em três frentes: glossário, UX/cobertura e segurança) e
devolve um JSON. O orquestrador grava o relatório e registra a nota com
`record-consistency`. Se a nota for menor que 10, os achados são corrigidos e a
consistência pode ser re-executada, no máximo **3 rodadas**.

## 5. Fila (SQLite; `CACHE.md` é histórico congelado)

### Semântica da fila

- **Uma categoria ativa por vez.** `max_active_categories` é 1. Apenas a
  categoria ativa aceita claims; as demais ficam `pending` e só são ativadas
  quando a anterior termina.
- **No máximo 3 claims ativos no total**, em qualquer estágio
  (`max_agents` = 3). O limite é global, não por categoria.
- **3 claims = 3 workers.** Cada claim ativo pertence a um único worker
  (subagente). O orquestrador não faz claim, não executa a unidade e
  **não conta no limite de 3**; ele apenas spawna workers, grava relatórios e
  registra o progresso.
- **Claims são por unidade e por papel.** `claim` exige que a categoria esteja
  no estágio do papel e que a unidade esteja elegível naquele estágio.
- **Lease padrão de 120 minutos** (`lease_minutes` no catálogo). O lease pode
  ser renovado com `heartbeat` e encurtado/estendido no `claim` com
  `--lease-minutes`.
- **Lease expirado volta para a fila.** `recover` devolve a unidade ao estágio
  anterior ao claim e registra o evento `lease-expired`.
- **Máximo de 3 rodadas por unidade.** Cada revisão incrementa `attempts`. A
  terceira revisão com nota menor que 10 coloca a unidade e a categoria em
  `blocked`.
- **Consistência também tem no máximo 3 rodadas.** A terceira nota menor que
  10 bloqueia a categoria.
- **Bloqueio interrompe o pipeline.** Categoria `blocked` não aceita `next`,
  `claim` nem `advance`; é preciso decisão do usuário para `unblock` ou
  `reopen`.
- **`next` não reserva unidade.** Ele apenas lista o que pode ser reivindicado;
  o claim é que reserva.

### Formato do `CACHE.md` (congelado; registro histórico da fila antiga)

Na fila antiga, o `CACHE.md` era gerado e tinha duas partes (o `jx i18n-queue`
atual guarda o estado no SQLite e não lê nem grava este arquivo):

1. **Relatório legível:** progresso geral, tabela das 23 categorias, detalhe da
   categoria ativa, bloqueios, eventos recentes e o bloco "Como retomar".
2. **Bloco canônico JSON:** delimitado por `<!-- STATE:BEGIN -->` e
   `<!-- STATE:END -->`. A fila lê e grava esse bloco; ele guarda categorias,
   unidades, claims, patches, overlays e eventos.

Regras da fila antiga (registro histórico; o `CACHE.md` atual está congelado
e não recebe mais escritas):

- **Nunca edite `CACHE.md` à mão.** Na fila antiga, toda mudança passava por
  um comando da fila.
- As escritas eram atômicas e serializadas por um lock em `CACHE.md.lock`.
- `init` criava o arquivo a partir de `i18n/catalog.lock.json`; `init --force`
  recriava o estado e descartava o progresso.
- `render` regenerava o relatório a partir do bloco JSON.
- `validate` conferia o estado contra o catálogo e as invariantes da policy.

### Relatórios de estágio

Os relatórios são JSON e seguem um caminho fixo:

```
.br-cache/reports/<categoria>/<unidade>/<estagio>-r<N>.json
```

- `<estagio>`: `analysis`, `translation`, `review` ou `correction`.
- `analysis` e `translation` usam **`-r0`** (relatório único, antes das
  rodadas de revisão).
- `review` começa em **`-r1`** e avança com as tentativas (`-r2`, `-r3`). O
  campo `round` do JSON deve bater com a tentativa.
- `correction` usa a rodada da revisão que gerou os achados
  (`correction-r1.json`, `correction-r2.json`, ...).
- O ID da unidade pode conter `/` (por exemplo, `server/schedules`), então o
  caminho espelha essa barra em subdiretórios.
- A consistência é da categoria, não de uma unidade:
  `.br-cache/reports/<categoria>/consistency-r<N>.json`, começando em `-r1`.
- **Subagentes somente leitura não escrevem arquivos.** Eles devolvem o JSON
  ao orquestrador, que valida o conteúdo e grava o relatório no caminho acima.
- `complete` (para analyst, translator, reviewer e fixer) e
  `record-consistency` recebem o caminho do relatório. A fila valida que o
  arquivo existe e que a nota é coerente com a policy antes de gravar o
  estado.

## 6. Comandos

Todos os comandos vivem em `jx i18n-queue` (implementação em Rust no crate
`jx_i18n`, estado em SQLite). Os `jx bridge-*` dão compatibilidade de
harness (`analyze-pair`, `skeleton`, `extract-comments`, `extract-php-entries`,
`placeholders`, `nlp-probe`).

| Comando | Uso | Quando usar |
|---------|-----|-------------|
| `init` | `init [--force]` | inicializar o banco a partir do lock; `--force` recria |
| `status` | `status [--json]` | ver a categoria ativa, estágios e claims |
| `next` | `next --role <papel> [--limit N] [--json]` | listar unidades reivindicáveis do papel |
| `claim` | `claim <unidade> --role <papel> --owner <nome> [--lease-minutes N]` | reservar uma unidade |
| `complete` | `complete <unidade> --role <papel> --report <arquivo> --owner <nome> [--score N] [--labels-total N] [--labels-translated N] [--note <texto>]` | registrar o fim de um estágio |
| `advance` | `advance <categoria> [--allow-unverified]` | avançar a categoria de estágio |
| `record-consistency` | `record-consistency <categoria> --score N --report <arquivo>` | registrar a revisão da categoria |
| `record-gate` | `record-gate <categoria> --name <gate> --status pass\|fail\|skip [--note <texto>]` | registrar um gate na verificação |
| `record-patch` | `record-patch <categoria> --name <patch> [--unit <unidade>]...` | vincular um patch a categoria/unidades |
| `record-overlay` | `record-overlay <categoria> --path <caminho> [--unit <unidade>]...` | vincular um overlay a categoria/unidades |
| `recover` | `recover` | liberar leases expirados |
| `validate` | `validate` | validar o estado contra o catálogo |

Papéis aceitos: `analyst`, `translator`, `reviewer`, `fixer`.

### Ciclo típico

```bash
# 1. Analise das tres unidades (3 claims = limite; nao inicie uma quarta)
jx i18n-queue next --role analyst --limit 3
jx i18n-queue claim shared/navigation-shell --role analyst --owner analyst-1
jx i18n-queue complete shared/navigation-shell --role analyst --owner analyst-1 \
    --report .br-cache/reports/shared-navigation/shared/navigation-shell/analysis-r0.json \
    --labels-total 42
jx i18n-queue claim shared/routers-shell --role analyst --owner analyst-2
jx i18n-queue complete shared/routers-shell --role analyst --owner analyst-2 \
    --report .br-cache/reports/shared-navigation/shared/routers-shell/analysis-r0.json \
    --labels-total 10
jx i18n-queue claim shared/route-labels --role analyst --owner analyst-3
jx i18n-queue complete shared/route-labels --role analyst --owner analyst-3 \
    --report .br-cache/reports/shared-navigation/shared/route-labels/analysis-r0.json \
    --labels-total 8
jx i18n-queue advance shared-navigation   # analysis -> translation

# 2. Traducao das tres unidades
jx i18n-queue next --role translator --limit 3
jx i18n-queue claim shared/navigation-shell --role translator --owner translator-1
jx i18n-queue complete shared/navigation-shell --role translator --owner translator-1 \
    --report .br-cache/reports/shared-navigation/shared/navigation-shell/translation-r0.json \
    --labels-translated 42
jx i18n-queue claim shared/routers-shell --role translator --owner translator-2
jx i18n-queue complete shared/routers-shell --role translator --owner translator-2 \
    --report .br-cache/reports/shared-navigation/shared/routers-shell/translation-r0.json \
    --labels-translated 10
jx i18n-queue claim shared/route-labels --role translator --owner translator-3
jx i18n-queue complete shared/route-labels --role translator --owner translator-3 \
    --report .br-cache/reports/shared-navigation/shared/route-labels/translation-r0.json \
    --labels-translated 8
jx i18n-queue advance shared-navigation   # translation -> review

# 3. Revisao: shell 8 e routers-shell 9 reprovam; route-labels passa; r2 fecha 10
jx i18n-queue next --role reviewer --limit 3
jx i18n-queue claim shared/navigation-shell --role reviewer --owner reviewer-1
jx i18n-queue complete shared/navigation-shell --role reviewer --owner reviewer-1 \
    --score 8 --report .br-cache/reports/shared-navigation/shared/navigation-shell/review-r1.json
jx i18n-queue claim shared/routers-shell --role reviewer --owner reviewer-2
jx i18n-queue complete shared/routers-shell --role reviewer --owner reviewer-2 \
    --score 9 --report .br-cache/reports/shared-navigation/shared/routers-shell/review-r1.json
jx i18n-queue claim shared/route-labels --role reviewer --owner reviewer-3
jx i18n-queue complete shared/route-labels --role reviewer --owner reviewer-3 \
    --score 10 --report .br-cache/reports/shared-navigation/shared/route-labels/review-r1.json

jx i18n-queue claim shared/navigation-shell --role fixer --owner fixer-1
jx i18n-queue complete shared/navigation-shell --role fixer --owner fixer-1 \
    --report .br-cache/reports/shared-navigation/shared/navigation-shell/correction-r1.json
jx i18n-queue claim shared/routers-shell --role fixer --owner fixer-2
jx i18n-queue complete shared/routers-shell --role fixer --owner fixer-2 \
    --report .br-cache/reports/shared-navigation/shared/routers-shell/correction-r1.json

jx i18n-queue claim shared/navigation-shell --role reviewer --owner reviewer-4
jx i18n-queue complete shared/navigation-shell --role reviewer --owner reviewer-4 \
    --score 10 --report .br-cache/reports/shared-navigation/shared/navigation-shell/review-r2.json
jx i18n-queue claim shared/routers-shell --role reviewer --owner reviewer-5
jx i18n-queue complete shared/routers-shell --role reviewer --owner reviewer-5 \
    --score 10 --report .br-cache/reports/shared-navigation/shared/routers-shell/review-r2.json
jx i18n-queue advance shared-navigation   # review -> consistency

# 4. Consistencia: agente i18n-consistency separado, fora da fila de claims
jx i18n-queue record-consistency shared-navigation --score 10 \
    --report .br-cache/reports/shared-navigation/consistency-r1.json
jx i18n-queue advance shared-navigation   # consistency -> verification

# 5. Verificacao: os sete gates
jx i18n-queue record-gate shared-navigation --name catalog --status pass
jx i18n-queue record-gate shared-navigation --name audit --status pass
jx i18n-queue record-gate shared-navigation --name lint --status pass
jx i18n-queue record-gate shared-navigation --name tests --status pass
jx i18n-queue record-gate shared-navigation --name typecheck --status pass
jx i18n-queue record-gate shared-navigation --name lint-ui --status pass
jx i18n-queue record-gate shared-navigation --name build --status pass

# 6. Os dois lotes reais, append-only; overlay so se houver arquivo novo
jx i18n-queue record-patch shared-navigation \
    --name 0002-i18n-shared-navigation.patch \
    --unit shared/navigation-shell --unit shared/routers-shell --unit shared/route-labels
jx i18n-queue record-patch shared-navigation \
    --name 0003-i18n-shared-navigation-review-r1.patch \
    --unit shared/routers-shell
# jx i18n-queue record-overlay shared-navigation \
#     --path overlays/resources/lang/pt-BR/<arquivo>.php --unit <unidade>

# 7. Fecha a categoria e ativa a proxima
jx i18n-queue advance shared-navigation   # verification -> done
```

O `complete` varia por papel:

- `analyst`: exige `--report` e `--owner`; aceita `--labels-total`.
- `translator`: exige `--report` e `--owner`; aceita `--labels-translated`.
- `reviewer`: exige `--score`, `--report` e `--owner`; nota menor que 10 deixa
  a unidade em `needs_fix` para o fixer e uma nova revisão é obrigatória.
- `fixer`: exige `--report` e `--owner`; devolve a unidade para nova revisão.

Cada unidade tem no máximo 3 revisões. Se a terceira não chegar a 10, a unidade
e a categoria ficam `blocked`. A consistência não usa `complete`: o
orquestrador chama `record-consistency` com o JSON do agente
`i18n-consistency`.

### Comandos auxiliares

| Comando | Uso | Efeito |
|---------|-----|--------|
| `heartbeat` | `heartbeat <unidade> --owner <nome> [--lease-minutes N]` | renova o lease do claim |
| `release` | `release <unidade> --owner <nome> [--reason <texto>]` | devolve a unidade sem concluir o estágio |
| `block` | `block <unidade\|categoria> --reason <texto>` | marca unidade/categoria como bloqueada |
| `unblock` | `unblock <unidade\|categoria> --stage <analysis\|translation\|review\|consistency\|verification> [--force]` | unidades: apenas os três primeiros; categorias: os cinco |
| `reopen` | `reopen <categoria> --reason <texto> [--force]` | reinicia apenas categoria `done` ou `blocked`; exige aprovação do usuário |
| `render` | `render` | mostra o resumo humano (não grava arquivo) |

`reopen` serve apenas para uma categoria `done` ou `blocked` e precisa de
aprovação do usuário. Reabrir uma categoria `blocked` **exige `--force`**. Se
outra categoria estiver ativa, `reopen` também falha sem `--force`; com a flag,
a categoria ativa e pausada (somente se não tiver claims abertos) e a reaberta
vira a ativa.

`unblock` usa a mesma lista do `--help`. Para **unidades**, os valores válidos
são apenas `analysis`, `translation` e `review`. Para **categorias**, valem
também `consistency` e `verification`. Para uma unidade ou categoria que não
está `blocked`, `unblock` exige `--force`.

## 7. Política de labels

Traduza apenas **labels visíveis ao usuário**:

- texto JSX entre tags e atributos `title`, `placeholder`, `label`,
  `description`, `message`, `alt` e `aria-label`;
- validações, toasts, empty states, tooltips e confirmações;
- labels visíveis de navegação e títulos/rótulos de rota apresentados ao
  usuário;
- mensagens de `resources/lang/en/**` que viram overlays em
  `resources/lang/pt-BR/`;
- templates Blade, PDFs e e-mails.

Nunca traduza:

- **comentários de código**, em nenhuma forma;
- identificadores, tipos, funções, variáveis, classes CSS, chaves,
  permissões ou nomes de arquivos;
- **caminhos e IDs de rota** (`/server/:id/console`, nomes de rota e slugs),
  mesmo quando o label visível correspondente é traduzido;
- logs, mensagens de framework e nomes de testes;
- dados dinâmicos vindos do banco ou da API;
- termos marcados como "manter" no glossário.

Placeholders (`:name`, `{name}`, `${...}`, `%s`, `%d`) são preservados. O tom é
pt-BR natural, com "você" e imperativo nas ações. A política completa está em
[`i18n/POLICY.md`](../i18n/POLICY.md) e a terminologia em
[`i18n/glossary.tsv`](../i18n/glossary.tsv).

## 8. Rubrica de revisão (0 a 10)

Cada critério vale até 2 pontos:

| Critério | 2 pontos | 1 ponto | 0 pontos |
|----------|----------|---------|----------|
| Cobertura | 100% dos labels visíveis | 1 label esquecido ou dúvida | mais de 1 label ou trecho relevante |
| Naturalidade | pt-BR fluido e natural | compreensível, mas literal | robô/literal/ilegível |
| Consistência | 100% do glossário | 1 desvio justificável | conflito claro de terminologia |
| Contexto/UX | tom, tamanho e placeholders corretos | ajuste pequeno de tom/tamanho | quebra layout ou placeholder |
| Segurança | só labels; comentários intactos; compila | risco pequeno e localizado | lógica/comentário/identificador alterado |

**10 = zero achados.** Não arredonde. `blocker`/`major` impedem nota acima de
7; `minor` e `nit` impedem 10. O relatório do revisor é JSON e lista cada
achado com `file`, `line`, `problem` e `fix`.

## 9. Patches e overlays

- **Arquivo existente no upstream -> patch.** A tradução entra como diff.
- **Arquivo novo -> overlay** em `overlays/`, espelhando o caminho do upstream.
- **Invariante de série consolidada (1 categoria = 1 patch canônico, numeração
  contígua `0001..0026`)**: cada categoria do catálogo possui um único patch
  canônico.
- **Manutenção e Rebase (In-Place via `jx refresh`)**: fora de uma rodada ativa
  da fila (ou durante um `jx rebase` para nova versão do upstream), qualquer
  alteração em arquivo de uma categoria existente deve atualizar o próprio
  patch canônico in-place (`jx refresh patches/00XX-...patch --force`), que
  regenera o patch `00XX` mantendo seu número e cabeçalho DEP-3 e reaplica
  automaticamente os patches seguintes e `overlays/`. Nunca crie patches
  avulsos no fim da série para arquivos já cobertos por `0002..0023` nem apague
  `00XX` para recriá-lo como `atual + 1`.
- **Durante uma rodada ativa da fila (`jx i18n-queue`)**: enquanto subagentes
  trabalham em uma categoria aberta, os lotes por estágio/rodada (`translate`,
  `review-r1`) são append-only e consolidados no patch canônico da categoria ao
  encerrar a categoria.
- Registre cada lote com `record-patch` e cada arquivo novo com
  `record-overlay`, vinculando as unidades com `--unit`.
- `jx lint` é o gate obrigatório da série.

## 10. Gates de verificação

A categoria só avança de `verification` para `done` com os sete gates
registrados e sem falhas:

| Gate | Comando | Verifica |
|------|---------|----------|
| `catalog` | `jx i18n-catalog --check` (`make i18n-check`) | lock atualizado, sem sobreposição ou arquivo não coberto |
| `audit` | `jx i18n-audit --category <id> --strict` (`make i18n-audit CATEGORY=<id>`) | comentários, identificadores, placeholders, glossário e escopo |
| `lint` | `jx lint` | cabeçalhos, ordem e aplicação limpa da série |
| `tests` | `cargo test -p jx_i18n` | motor i18n Rust (fila, auditoria, catálogo) |
| `typecheck` | `pnpm typecheck` | tipos do frontend |
| `lint-ui` | `pnpm lint` | lint do frontend |
| `build` | `pnpm build` | build de produção |

### Auditoria estática

```bash
jx i18n-audit --unit <unidade> [--base <rev>] [--json] [--strict]
jx i18n-audit --category <categoria> [--base <rev>] [--json] [--strict]
```

O estado da fila no novo caminho vive no SQLite em
`.br-cache/i18n-queue.sqlite` (`jx i18n-queue ... [--db <caminho>]`);
o `jx i18n-queue render` mostra o resumo humano sem gravar `CACHE.md`.

- `--unit` audita os arquivos de uma unidade; `--category` audita a categoria
  inteira. Use um ou outro, nunca os dois.
- `--base` compara contra uma revisão git; o padrão é `HEAD` (ou
  `BR_I18N_BASE`). Em árvore limpa, informe `--base <rev>` explicitamente;
  sem isso o auditor falha em vez de auditar nada.
- `--json` imprime um único documento JSON, útil para o orquestrador gravar o
  relatório.
- `--strict` é aceito por compatibilidade. O gate **sempre** falha com
  qualquer achado, inclusive `minor`/`nit`; **zero achados** e o critério de
  aprovação.

O `make i18n-audit CATEGORY=<id>` chama o auditor com `--strict` e exige zero
achados. O `make i18n-test` roda a suíte Rust localmente. No CI, o job
`i18n` roda `jx i18n-catalog --check` e `cargo test -p jx_i18n`, e o job
`catalog` roda `jx i18n-catalog --check`.

Registro:

```bash
jx i18n-queue record-gate <categoria> --name catalog --status pass
jx i18n-queue record-gate <categoria> --name typecheck --status skip \
    --note "node_modules ausente no workspace"
```

- `fail` impede o avanço; corrija e registre de novo.
- `record-gate --status skip` **exige `--note`** com a justificativa. Sem a
  nota, o skip não é aceito.
- `advance` recusa skips sem `--allow-unverified`. Com a flag, só aceita
  **skips registrados com `--note`** em `record-gate`; gates ausentes ou com
  `fail` continuam bloqueando. A fila marca `gate_status: unverified`.
- Nunca registre `pass` para um comando que não foi executado.
- Se PHP/Composer estiverem disponíveis, a suíte PHP pode ser registrada como
  gate adicional (`php-tests`); ela não substitui os sete gates exigidos.

## 11. Papéis e agentes

Os agentes ficam em `.opencode/agents/` e o roteiro operacional em
`.opencode/skills/i18n-pipeline/SKILL.md`.

| Papel | Arquivo | Permissões | Saída |
|-------|---------|------------|-------|
| `analyst` | `.opencode/agents/i18n-analyst.md` | somente leitura; sem shell | JSON com `labels`, `shared_dependencies`, `glossary_candidates`, `test_impact`, `risks` e `estimated_labels` |
| `translator` | `.opencode/agents/i18n-translator.md` | edita apenas `.upstream/resources/**` e `overlays/**` | JSON com `files_changed`, `labels_translated`, `left_untranslated` e `notes` |
| `reviewer` | `.opencode/agents/i18n-reviewer.md` | somente leitura; sem shell; independente do tradutor e do fixer | JSON da rubrica com `unit`, `round`, `score`, `criteria`, `findings`, `untranslated`, `glossary_violations` e `test_impact` |
| `fixer` | `.opencode/agents/i18n-fixer.md` | edita apenas os arquivos dos achados | JSON com `findings_fixed`, `files_changed`, `blocked_findings` e `notes` |
| `consistency` | `.opencode/agents/i18n-consistency.md` | somente leitura; revisa a categoria inteira; **não é claimable** | JSON com `category`, `round`, `score`, `criteria`, `findings`, `untranslated`, `glossary_violations` e `test_impact` |

Cada agente lê, nesta ordem: `AGENTS.md`, `i18n/POLICY.md`,
`i18n/glossary.tsv`, o escopo da categoria em `i18n/catalog.json` e o material do
estágio. Quem traduz não revisa; quem revisa não corrige a própria revisão.

O revisor de consistência é spawnado pelo orquestrador quando a categoria
entra em `consistency`; ele não aparece em `next` nem aceita `claim`. O
orquestrador grava o JSON devolvido em
`.br-cache/reports/<categoria>/consistency-r<N>.json` e chama
`record-consistency`.

Nenhum subagente cria patches, commita, edita `CACHE.md` ou grava relatórios:
quem grava os relatórios é o orquestrador.

## 12. Retomada

1. Valide o catálogo: `jx i18n-catalog --check` (`make i18n-check`).
2. Libere leases órfãos: `jx i18n-queue recover`.
3. Veja o estado: `jx i18n-queue status` (ou `status --json`).
4. Leia o resumo e a categoria ativa com `jx i18n-queue status`.
5. Liste as próximas unidades: `jx i18n-queue next --role <papel> --limit 3`.
6. Reivindique antes de editar: `jx i18n-queue claim ...`.
7. Continue a categoria ativa no estágio indicado; não pule estágios.
8. Antes de encerrar a sessão, rode `jx i18n-queue validate`.

Nunca reinicie uma categoria `done` sem `reopen` aprovado pelo usuário.

## 13. Escalonamento

Após **3 rodadas com o mesmo bloqueio**, pare e escale ao usuário. O mesmo vale
para consistência: 3 rodadas sem 10 bloqueiam a categoria.

Ao escalar, reúna evidências:

- categoria e unidade, estágio e rodada;
- última nota e o JSON de achados com `file:line`;
- correções tentadas e por que não resolveram;
- gates executados e seus status;
- comandos rodados.

Registre o bloqueio com `jx i18n-queue block <unidade|categoria> --reason <texto>` e
não edite mais a unidade até a decisão. Não arredonde a nota, não aprove com
achados abertos e não use `unblock`/`reopen` sem aprovação do usuário.

## 14. Limitações conhecidas

- **Agentes somente leitura não gravam relatórios.** Analyst, reviewer e
  `i18n-consistency` devolvem o JSON ao orquestrador; eles não podem escrever
  em `.br-cache/reports/`. O orquestrador é quem salva o arquivo e passa o
  caminho para a fila.
- **Activity log sem i18next inicializado.** O componente
  `resources/scripts/elements/Translate.tsx` usa `useTranslation`, mas a SPA
  não chama `i18next.init`. As telas de atividade
  (`elements/activity/ActivityLogEntry.tsx` e
  `components/admin/general/activity/ActivityLogsContainer.tsx`) passam
  `i18nKey` e não resolvem enquanto a inicialização não existir. Isso é uma
  limitação de infraestrutura, não um label esquecido; não adicione a
  inicialização do i18next dentro de uma unidade de tradução comum.
- **Strings dinâmicas da API não são labels.** Descrições de atividade, nomes
  de servidor, valores de variáveis e conteúdo digitado pelo usuário vem do
  banco/API e não entram na nota.
- **Não há troca de idioma em runtime no frontend.** A tradução é inline; o
  painel passa a falar pt-BR. No backend, chaves ausentes continuam caindo no
  fallback `en` do Laravel.

## 15. Memoria operacional (AI memory)

`.ai-memory.toml` usa `workspace = "default"` e `project = "jexactyl-br"`.
O registro do projeto ainda **não foi validado** e as páginas operacionais
abaixo estão **pendentes de validação**:

- `_rules/i18n-pipeline.md` — regras duras do pipeline;
- `procedures/i18n-queue-cycle.md` — ciclo operacional da fila;
- `gotchas/i18n-pipeline.md` — armadilhas conhecidas.

Enquanto não forem validadas, a fonte de verdade é este documento, `AGENTS.md`
e `i18n/POLICY.md`.

## 16. Estado de implementação

| Item | Estado |
|------|--------|
| Catálogo e lock (`jx i18n-catalog`, `catalog.json`, `catalog.lock.json`) | implementado (scripts Python/JS removidos no cutover) |
| Fila persistente (`jx i18n-queue` sobre SQLite; `CACHE.md` congelado como histórico) | implementado |
| Política, rubrica e glossário (`i18n/POLICY.md`, `i18n/glossary.tsv`) | implementado |
| Perfis de categoria (`i18n/categories/*.md`) | removidos do versionamento; regeneráveis via `jx i18n-catalog --write` |
| Agentes de unidade e skill (`.opencode/agents/`, `.opencode/skills/i18n-pipeline/`) | implementado |
| Agente de consistência (`.opencode/agents/i18n-consistency.md`) | implementado |
| Suíte da fila e do auditor (Rust, `cargo test -p jx_i18n`) | implementada e validada (ferramental Python/JS removido após cutover para o `jx`) |
| Memoria operacional (AI memory) | pendente: 3 páginas para validar |
| Execução da tradução (categorias, patches e overlays) | concluída (23/23) |
