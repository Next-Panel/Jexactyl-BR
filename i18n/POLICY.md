# Política de tradução do Jexactyl

Este documento define o contrato dos agentes e das pessoas que trabalham na
tradução do painel. Ele complementa `AGENTS.md`; em caso de conflito, vale o
que estiver em `AGENTS.md`.

## 1. Objetivo

Traduzir **labels visíveis** do painel para português brasileiro natural,
mantendo o código, os comentários e o comportamento intactos. A qualidade é
medida por uma rubrica objetiva de 0 a 10; uma unidade só termina com **10**.

## 2. O que é um label

Traduzir:

- texto JSX entre tags;
- atributos `title`, `placeholder`, `label`, `description`, `message`,
  `alt`, `aria-label`;
- mensagens de validação de formulários (`yup`, `required`, `min`, `max`);
- toasts, flash messages, empty states, tooltips e confirmações;
- rótulos de navegação e itens de menu (não os caminhos/IDs de rota);
- mensagens de `resources/lang/pt-BR` (backend), PDFs e e-mails;
- assertions/fixtures de teste que verificam um label visível.

Não traduzir:

- **comentários de código** de qualquer forma (`//`, `/* */`, `<!-- -->`);
- identificadores, tipos, funções, variáveis, classes CSS, chaves de API,
  permissões, **caminhos/IDs de rota**, variáveis de ambiente e nomes de
  arquivos;
- logs de console, mensagens de framework e nomes de testes;
- dados dinâmicos vindos do banco ou da API;
- termos marcados como “manter” no glossário;
- textos de bibliotecas de terceiros.

Novos arquivos de overlay podem ter comentários em português; isso não é
“tradução de comentário”. Arquivos existentes do upstream nunca têm
comentários alterados.

## 3. Tom e estilo

- Português brasileiro, tratamento por “você”, sem formalidade excessiva.
- Ações no imperativo: “Salvar”, “Excluir”, “Criar”.
- Títulos e seções em caixa de frase: “Configurações do servidor”.
- Evitar literalidade: “Home” vira “Início”, não “Casa”; “Security” vira
  “Segurança”, não “Securanca”.
- Preservar acentos e cedilha.
- Não aumentar um botão ou rótulo a ponto de quebrar o layout; usar sinônimos
  curtos quando necessário.
- Placeholders (`:name`, `{name}`, `${...}`, `%s`, `%d`) são sagrados.
- Nomes de marcas e siglas seguem o glossário.

## 4. Glossário

`i18n/glossary.tsv` e a fonte única de terminologia. Colunas: `en`, `pt-BR`,
`notes`. O glossário pode ser estendido durante a análise, mas a mudança
precisa ser registrada no estado da fila e revisada.

Termos técnicos mantidos: `API`, `SFTP`, `SSH`, `OAuth`, `SSO`, `2FA`,
`WebSocket`, `Cron`, `Node`, `Nest`, `Egg`, `Preset`, `Passkey`, `RAM`, `CPU`,
`UUID`, `DNS`, `CIDR`, `Docker`, marcas (Jexactyl, Jexpanel, Discord, Google,
Stripe, Mercado Pago, PayPal, Steam, Hytale, Java, Bedrock).

## 5. Rubrica de revisão (0 a 10)

Cada critério vale até 2 pontos:

| Critério | 2 pontos | 1 ponto | 0 pontos |
|----------|----------|---------|----------|
| Cobertura | 100% dos labels visíveis | 1 label esquecido ou dúvida | mais de 1 label ou trecho relevante |
| Naturalidade | pt-BR fluido e natural | compreensível, mas literal | robô/literal/ilegível |
| Consistência | 100% do glossário | 1 desvio justificável | conflito claro de terminologia |
| Contexto/UX | tom, tamanho e placeholders corretos | ajuste pequeno de tom/tamanho | quebra layout ou placeholder |
| Segurança | só labels; comentários intactos; compila | risco pequeno e localizado | lógica/comentário/identificador alterado |

**10 = zero achados.** Qualquer achado, inclusive `nit`, impede o 10. O revisor
deve listar todos os achados com `file:line`, problema e correção sugerida.

Formato do relatório:

```json
{
  "unit": "server/schedules",
  "round": 1,
  "score": 8,
  "criteria": {
    "coverage": 2,
    "naturalness": 2,
    "consistency": 1,
    "context": 2,
    "safety": 1
  },
  "findings": [
    {
      "id": "F1",
      "severity": "major",
      "file": "resources/scripts/components/server/schedules/ScheduleRow.tsx",
      "line": 42,
      "problem": "Descricao do problema",
      "fix": "Correcao sugerida"
    }
  ],
  "untranslated": [],
  "glossary_violations": [],
  "test_impact": []
}
```

Severidades: `blocker`, `major`, `minor`, `nit`. Blocker/major impedem
qualquer nota acima de 7; minor impede 10; nit também impede 10.

## 6. Rodadas e escalonamento

- A primeira revisão acontece depois da tradução.
- Nota menor que 10 gera correção e nova revisão.
- Máximo de **3 rodadas** por unidade. A terceira revisão abaixo de 10
  bloqueia a unidade e a categoria, mesmo que os achados mudem.
- Nunca “arredondar” para 10 nem aprovar com achados abertos.
- A consistência da categoria pode ser feita por três revisores separados e
  somente leitura (glossário, UX/cobertura e segurança); a nota registrada é a
  menor das três e os achados são unidos. Use `record-consistency`.
- Relatórios JSON ficam em
  `.br-cache/reports/<categoria>/<unidade>/<estagio>-r<N>.json`, escritos pelo
  orquestrador. O `complete` valida existência, JSON, nota e `findings`.

## 7. Testes

- Testes são código: nomes, comentários e estrutura permanecem em inglês.
- Se um teste verifica um label visível, a assertion/fixture é atualizada no
  mesmo patch da tradução.
- Não traduzir mensagens do PHPUnit, chaves de API, campos JSON ou nomes de
  métodos.
- O gate `tests` roda `cargo test -p jx_i18n`; o gate
  `php-tests` roda a suíte PHP quando o ambiente tiver PHP/Composer.

## 8. Patches e overlays

- A tradução de arquivos existentes vira **patch**.
- Arquivos novos (ex.: `resources/lang/pt-BR/*.php`) viram **overlay**.
- **Invariante pós-consolidação (1 categoria = 1 patch canônico)**: cada
  categoria possui um único patch canônico na sequência contígua `0001..0026`.
- **Manutenção e Rebase (In-Place)**: ajustes posteriores ou resoluções de
  conflito em arquivos de uma categoria concluída devem atualizar o próprio
  patch canônico da categoria via `jx refresh patches/00XX-...patch --force`
  (que atualiza `00XX` no lugar e reaplica os patches seguintes). Nunca apague
  `00XX` para criar `atual + 1` (deixando buraco na numeração) nem crie patches
  extras no fim da série tocando arquivos de categorias anteriores.
- **Durante rodadas ativas da fila (`jx i18n-queue`)**: enquanto uma categoria
  estiver em andamento com workers concorrentes, lotes de rodada (`translate`,
  `review-r1`) são append-only sem `jx refresh` no meio da rodada, sendo
  consolidados no patch canônico da categoria ao concluí-la.
- Apenas uma categoria fica ativa por vez, o que mantém a ordem da série.
- Arquivos compartilhados (`elements/**`, `routers/routes/*.ts`) só podem ser
  alterados na correção da categoria ativa quando o achado for um label
  compartilhado; registre com `jx i18n-queue record-shared-override` (o estado
  vive no SQLite, nunca edite arquivos de estado à mão).

## 9. Fila (`jx i18n-queue`)

- O estado da fila vive no SQLite; não editar arquivos de estado à mão.
- Antes de trabalhar: `jx i18n-queue recover` e `jx i18n-queue status`.
- Reivindicar: `jx i18n-queue claim <unidade> --role <papel> --owner <agente>`.
- Concluir: `jx i18n-queue complete ... --owner <agente> --report <arquivo>`.
- No máximo **3** claims ativos no total, um worker por claim; o orquestrador
  não conta.
- Um lease expirado volta para a fila; `jx i18n-queue recover` limpa leases órfãos.
- O orquestrador registra patches, overlays, shared overrides e gates com
  `jx i18n-queue record-patch`, `record-overlay`, `record-shared-override` e `record-gate`.
- `complete`, `heartbeat` e `release` exigem que o `--owner` bata com o claim.
- `record-gate --status skip` exige `--note`.
- Para retomar em outra sessão, consulte `jx i18n-queue status`.

## 10. Definition of Done

Uma categoria só termina quando:

- [ ] todas as unidades tiverem nota 10 na revisão;
- [ ] a consistência da categoria tiver nota 10;
- [ ] a auditoria estática passar (comentários, identificadores, placeholders,
      glossário e escopo);
- [ ] `catalog`, `audit`, `lint`, `tests`, `typecheck`, `lint-ui` e `build`
      estiverem `pass` ou `skip` justificado;
- [ ] patches e overlays estiverem registrados na fila;
- [ ] o commit do overlay estiver feito e a fila atualizada.

## 11. Regras por categoria

- Uma categoria ativa por vez; a ordem está em `i18n/catalog.json`.
- `server-configuration` é uma única categoria com quatro unidades:
  Schedules, Users, Network e Startup.
- Unidade concluída = revisão 10. Categoria concluída = consistência 10 +
  gates.
- Rótulos de navegação são traduzíveis; caminhos e IDs de rota não são.

## 12. Segurança

- Nunca ler ou editar `.env`, chaves, tokens ou credenciais.
- Não executar comandos destrutivos no workspace.
- Não commitar `node_modules`, `vendor`, `public/build`, lockfiles ou `.env`.
- Em caso de dúvida, bloquear e perguntar ao usuário.
