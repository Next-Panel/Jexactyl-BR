# AGENTS.md — Jexactyl

Este repositório é um **overlay de patches** sobre o Jexactyl oficial v4.0.7.
Ele não versiona o upstream: versiona a série de patches, os overlays, os
scripts e o estado da tradução.

## Regras não negociáveis

1. **Nunca traduza comentários de código.** Traduza exclusivamente labels
   visíveis ao usuário.
2. **Uma categoria ativa por vez.** A ordem está em `i18n/catalog.json`.
3. **No máximo 3 claims ativos** no total, um worker por claim, em qualquer
   estágio. O orquestrador não conta para o limite.
4. **Toda unidade passa por**: análise -> tradução -> revisão (0-10) ->
   correção -> nova revisão, até nota **10**.
5. **Nota 10 exige zero achados.** Não arredonde, não aprove com pendências.
6. **Quem traduz não revisa; quem revisa não corrige a própria revisão.**
7. **Um patch canônico por categoria/feature e sequência contígua (`0001..00NN`).**
   - **Nunca quebre a sequência** apagando um patch intermediário (ex.: `0004`) para recriá-lo como `atual + 1`: a numeração `0001..00NN` é contígua e reflete a ordem topológica de dependências.
   - **Rebase de upstream ou manutenção/correção de categoria existente -> atualização in-place (`jx refresh`)**: se um arquivo de `0004` mudar no upstream ou precisar de ajuste de tradução, atualize o próprio `patches/0004-...patch` com `jx refresh patches/0004-...patch --force` (que regenera o `0004` e reaplica automaticamente `0005..00NN` + `overlays/`). Nunca crie um patch `atual + 1` tocando arquivos que já pertencem a `0001..0023`.
   - **Rodada ativa da fila (`jx i18n-queue`) ou feature inédita -> `jx dump` (`atual + 1`)**: durante rodadas ativas de `review`/`correction` de uma categoria em andamento, lotes temporários são append-only e devem ser consolidados no patch canônico da categoria ao concluí-la. Novos números permanentes (`atual + 1`) são exclusivos para novas categorias ou features independentes.
8. **Unidade concluída = revisão 10.** Categoria concluída = consistência 10 +
   auditoria + gates verdes (ou `skip` justificado).
9. **O estado da fila vive no SQLite; não edite arquivos de estado à mão.**
   Use `jx i18n-queue` (`--db` aponta para outro banco quando preciso).
10. **Nunca leia ou altere `.env`, chaves, tokens ou credenciais.**

## Fluxo obrigatório

```
pending
  -> analysis -> translation -> review <-> correction
                                      |
                                      +-> consistency -> verification -> done
```

- `analysis`: inventário de labels, dependências, riscos e impacto em testes.
- `translation`: edição apenas dos arquivos da unidade.
- `review`: nota 0-10 com a rubrica de `i18n/POLICY.md`.
- `correction`: corrige somente os achados da revisão.
- `consistency`: um revisor separado, somente leitura, avalia a categoria
  inteira (glossário, tom, navegação, duplicatas).
- `verification`: auditoria + testes + typecheck + lint + build.

A terceira revisão abaixo de 10 bloqueia a unidade e a categoria, mesmo que os
achados mudem. Se o mesmo bloqueio persistir, escale ao usuário com evidências.

## Fila e estado (`jx i18n-queue`)

O estado da fila vive no SQLite (`.br-cache/i18n-queue.sqlite`, `--db`
sobrescreve); o `CACHE.md` é histórico gerado, não fonte de verdade.
No início de cada sessão:

```bash
jx i18n-queue recover
jx i18n-queue status
```

Para trabalhar:

```bash
jx i18n-queue next --role analyst --limit 3
jx i18n-queue claim <unidade> --role <papel> --owner <nome>
jx i18n-queue complete <unidade> --role <papel> --owner <nome> --report <arquivo>
jx i18n-queue advance <categoria>
jx i18n-queue record-consistency <categoria> --score <n> --report <arquivo>
jx i18n-queue record-gate <categoria> --name <gate> --status pass|fail|skip
jx i18n-queue record-patch <categoria> --name <patch> --unit <unidade>
jx i18n-queue record-overlay <categoria> --path <arquivo> --unit <unidade>
jx i18n-queue record-shared-override <categoria> --path <arquivo> --reason <texto>
```

Papéis: `analyst`, `translator`, `reviewer`, `fixer`. O claim tem lease; se o
agente morrer, `recover` devolve a unidade para a fila.

Os relatórios JSON são escritos pelo orquestrador em
`.br-cache/reports/<categoria>/<unidade>/<estagio>-r<N>.json`; os subagentes
devolvem o JSON no chat e não escrevem nesse diretório. `complete` valida que o
arquivo existe, que o JSON é válido e que a nota do relatório bate com
`--score`; nota 10 exige `findings` vazio.

## Categorias

A ordem e o escopo estão em `i18n/catalog.json` (perfis legíveis gerados sob
demanda por `jx i18n-catalog --write`). As primeiras categorias são fundação:

1. `shared-navigation`
2. `shared-ui-core`
3. `shared-billing`
4. `shared-runtime`
5. `backend-messages`
6. `auth`
7. `account-overview`
8. `account-security`
9. `account-tickets`
10. `account-billing`
11. `server-shared`
12. `server-console`
13. `server-data`
14. `server-configuration` (Schedules, Users, Network, Startup)
15. `server-billing`
16. `server-activity`
17. `admin-shared`
18. `admin-general`
19. `admin-management`
20. `admin-modules`
21. `admin-appearance`
22. `admin-services`
23. `admin-setup`

## Agentes especializados

Os agentes ficam em `.opencode/agents/`:

- `i18n-analyst` — somente leitura; inventaria labels e riscos.
- `i18n-translator` — edita apenas os arquivos da unidade.
- `i18n-reviewer` — somente leitura; aplica a rubrica e devolve JSON.
- `i18n-fixer` — corrige apenas os achados da revisão.
- `i18n-consistency` — somente leitura; revisa a categoria inteira e devolve a
  nota de consistência.

O escopo de cada categoria está em `i18n/catalog.json` (perfis legíveis podem
ser gerados com `jx i18n-catalog --write`). O prompt do agente
sempre inclui `AGENTS.md`, `i18n/POLICY.md`, `i18n/glossary.tsv`, o escopo da
categoria e os valores injetados pelo orquestrador: categoria, unidade, rodada,
owner do claim, base do diff e caminho do relatório. Se algum valor faltar, o
agente deve parar e pedir ao orquestrador.

## Tradução

- Labels: texto JSX, `title`, `placeholder`, `label`, `description`, `message`,
  `alt`, `aria-label`, validações, toasts, rótulos de navegação, lang files,
  PDFs/e-mails.
- Nunca: comentários, identificadores, tipos, CSS, chaves, **caminhos/IDs de
  rota**, logs, dados dinâmicos, nomes de testes, mensagens de framework.
- Placeholders (`:name`, `{name}`, `${...}`, `%s`) são preservados.
- Tom: pt-BR natural, “você”, imperativo em ações, acentos corretos.
- Glossário: `i18n/glossary.tsv`. Termos técnicos marcados como “manter” não
  são traduzidos.

## Revisão

Rubrica de 0 a 10 em `i18n/POLICY.md` (5 critérios x 2 pontos). O relatório é
JSON com `score`, `criteria`, `findings`, `untranslated`,
`glossary_violations` e `test_impact`. Qualquer achado impede 10. A terceira
revisão abaixo de 10 bloqueia a unidade/categoria; não existe quarta rodada
sem intervenção do usuário.

## Patches e overlays

- Arquivo existente -> patch.
- Arquivo novo -> `overlays/`.
- **Invariante de propriedade única (1 categoria/feature = 1 patch canônico)**:
  cada categoria de `i18n/catalog.json` possui exatamente um patch canônico em
  `patches/series` (`0002` a `0023`), precedido por `0001` (locales) e seguido
  pelos patches funcionais (`0024` gateway Mercado Pago, `0025` exceções e
  transformers de backend, `0026` core X-Frame-Options).
- **Manutenção e Rebase (In-Place)**: para alterar ou resolver conflito em
  qualquer patch existente (ex.: `0004`), edite os arquivos em `.upstream/` e
  rode `jx refresh patches/0004-i18n-shared-runtime.patch --force`. O `jx`
  atualiza o patch `0004` no lugar e reaplica automaticamente `0005..0026` e
  `overlays/`.
- **Fila Multiagente Ativa**: enquanto uma categoria está aberta na fila
  (`translation` -> `review-r1`), o orquestrador pode gerar lotes por rodada e
  consolidá-los no patch canônico da categoria ao final da verificação.
- `jx lint` é o gate obrigatório da série.

## Testes e verificação

```bash
jx i18n-catalog --check
jx i18n-audit --category <id> --strict
jx lint
cargo test -p jx_i18n
```

O `jx i18n-queue` implementa a fila sobre SQLite em
`.br-cache/i18n-queue.sqlite` (`--db` sobrescreve); ele não grava `CACHE.md`.
Os `jx bridge-*` dão compatibilidade de harness (`analyze-pair`, `skeleton`,
`extract-comments`, `extract-php-entries`, `placeholders`, `nlp-probe`).

Gates obrigatórios da categoria: `catalog`, `audit`, `lint`, `tests`,
`typecheck`, `lint-ui`, `build`. `record-gate --status skip` exige `--note`.

Se `node_modules` existir no workspace:

```bash
pnpm typecheck && pnpm lint && pnpm build
```

Se PHP/Composer existirem, rode a suíte PHP relevante. Sem essas ferramentas,
registre o gate como `skip` com justificativa (`jx i18n-queue record-gate ...
--status skip --note <motivo>`); o gate ficara como `unverified`.

## Retomada

1. Leia o resumo do `CACHE.md` (histórico; a fila viva está no SQLite).
2. Rode `jx i18n-queue recover` e `jx i18n-queue status`.
3. Continue a categoria ativa no estágio indicado.
4. `reopen` só aceita categoria `done` ou `blocked`; categoria `blocked` exige
   `--force`. Se outra categoria estiver ativa, `--force` pausa a atual.
5. `unblock` aceita `analysis`, `translation`, `review`, `consistency` ou
   `verification`; unidade só aceita os três primeiros. Desbloquear uma unidade
   para `review` exige que nenhuma irma continue bloqueada; nesse caso, use o
   desbloqueio da categoria.

## Memoria (ai-memory)

O projeto usa `workspace = "default"` e `project = "jexactyl-br"` no
`.ai-memory.toml`. As regras duráveis ficam em `_rules/i18n-pipeline.md`, o
procedimento em `procedures/i18n-queue-cycle.md` e as armadilhas em
`gotchas/i18n-pipeline.md`. Chamadas de subagentes devem passar o escopo
explícito; não confie no slot compartilhado.

## Referencias

- `i18n/POLICY.md` — política detalhada e rubrica.
- `i18n/glossary.tsv` — terminologia.
- `i18n/catalog.json` — categorias, unidades e globs.
- `docs/i18n-pipeline.md` — arquitetura e operação.
- `docs/overlays.md` — regras de overlay.
- `patches/README.md` — formato DEP-3 e fluxo de patches.
