# Contribuindo com o Jexactyl

Este repositório não contém o código do painel: contém a série de patches e os
overlays que são aplicados sobre o [Jexactyl oficial](https://github.com/Jexactyl/Jexactyl).

## Fluxo de trabalho

1. `jx setup` prepara o workspace em `.upstream/`.
2. A mudança é feita dentro de `.upstream/`.
3. `jx dump <slug> --domain <NN>` gera o patch e registra na série.
4. `jx lint` valida a série (o mesmo gate do CI).
5. O pull request precisa passar pelo CI.

## Regras

- Um patch = uma mudança lógica.
- Arquivo novo = overlay (`overlays/`). Arquivo existente = patch.
- Nunca editar o arquivo `.patch` à mão: use `jx refresh`
  (fora do pipeline i18n; dentro dele os lotes são append-only).
- Nunca incluir lockfile, `vendor/`, `node_modules/`, `public/build` ou `.env`.
- O cabeçalho do patch segue DEP-3 e é validado pelo lint.

## Domínios

O número do arquivo é uma sequência global (a ordem de criação e a ordem de
aplicação). O domínio vira o rótulo do nome do arquivo:

| Domínio    | Rótulo    | Escopo                               |
| ---------- | --------- | ------------------------------------ |
| `01`       | `i18n`    | i18n e locales                       |
| `02`       | `gateway` | gateways de pagamento                |
| `03`       | `billing` | billing e faturas                    |
| `04`-`09`  | `core`    | comportamento geral do painel     |
| `10`+      | `ui`      | interface, marca e templates         |

O patch existente `patches/0001-i18n-add-supported-locales.patch` foi gerado
com:

```bash
jx dump add-supported-locales --domain 01
```

Escolha o domínio conforme o tipo da mudança (tabela acima); a numeração do
arquivo é global e vem de `patches/series`. O contexto das ondas está em
`docs/roadmap.md`.

## Tradução

A tradução segue o pipeline por categoria descrito em
[`docs/i18n-pipeline.md`](docs/i18n-pipeline.md), com a política de labels em
[`i18n/POLICY.md`](i18n/POLICY.md) e a terminologia em `i18n/glossary.tsv`.

- Traduza apenas labels visíveis ao usuário. **Comentários de código ficam
  intactos**, assim como identificadores, tipos, chaves e **caminhos/IDs de
  rota**. O texto visível de um item de navegação ou título de rota é
  traduzível; a rota em si (`/server/:id/console`, slug, nome interno) não é.
- Uma categoria ativa por vez, no máximo **3 claims ativos** e **um worker por
  claim**. O orquestrador não conta no limite de 3.
- Cada unidade passa por análise, tradução, revisão 0-10, correção e nova
  revisão. A nota **10 exige zero achados**; não arredonde.
- Patches são **append-only**: um lote por estágio/rodada e correções viram um
  novo patch. Não use `jx refresh` no meio do pipeline.
- O estado fica no SQLite (`.br-cache/i18n-queue.sqlite`); nunca edite arquivos
  de estado à mão.
- Relatórios JSON ficam em
  `.br-cache/reports/<categoria>/<unidade>/<estagio>-r<N>.json`; o orquestrador
  grava o arquivo devolvido pelos subagentes somente leitura.

```bash
jx i18n-queue recover
jx i18n-queue status
jx i18n-queue next --role analyst --limit 3
jx i18n-queue claim <unidade> --role <papel> --owner <nome>
jx i18n-queue complete <unidade> --role <papel> --owner <nome> --report <arquivo>
jx i18n-queue advance <categoria>
```

A ordem das categorias está em `i18n/catalog.json`. Use
`jx i18n-catalog --check` para validar o escopo antes de começar.

## Commits

Mensagens no estilo Conventional Commits, em inglês:

```
feat(i18n): register the pt-BR locale
fix(overlay): keep the series in ascending order
```

## Revisão

O lint é obrigatório. Ele confere cabeçalho, numeração, ordem, colisão de
overlay, aplicação limpa sobre o commit fixado, idempotência e
reversibilidade da série.
