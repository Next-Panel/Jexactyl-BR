# Série de patches

Este diretório guarda as alterações do Jexactyl sobre o upstream oficial.
Nenhum código do upstream é versionado aqui: o upstream vive no workspace
(`.upstream/`) e os patches são aplicados por cima dele.

## Formato

Cada patch é um diff unificado gerado por `git`, com cabeçalho no estilo
DEP-3 (o mesmo usado pelo Debian) e terminador `---` antes do corpo do diff.
O `git apply`, o `git am` e o `patch -p1` aceitam esse formato.

```
From: Nome <email@example.com>
Date: Thu, 22 Oct 2026 10:30:00 -0300
Subject: locale: registra os locales suportados pelo painel

Description:
Adiciona a lista de locales suportados em config/app.php, incluindo
pt-BR como traducao mantida pelo overlay Jexactyl.

Origin: vendor
Forwarded: not-needed
Bug-Next-Panel: https://github.com/Next-Panel/Jexactyl-BR/issues/1
Applied-Upstream: no
Last-Update: 2026-10-22
Signed-off-by: Nome <email@example.com>

---
diff --git a/config/app.php b/config/app.php
...
```

Campos validados por `jx lint`:

| Campo               | Obrigatório | Valores aceitos                                              |
| ------------------- | ----------- | ------------------------------------------------------------ |
| `From:`             | sim         | nome e email do autor                                         |
| `Date:`             | sim         | data RFC 2822                                                 |
| `Subject:`          | sim         | uma linha, sem o prefixo `[PATCH n]`                          |
| `Description:`      | sim         | texto livre, quebrado em 79 colunas                           |
| `Origin:`           | sim         | `upstream`, `backport`, `vendor`, `other` (opcionalmente com URL) |
| `Forwarded:`        | sim         | `no`, `not-needed`, `yes` ou URL                              |
| `Applied-Upstream:` | sim         | `no`, `yes`, `v4.0.8`, `commit:<sha>`, URL                     |
| `Last-Update:`      | sim         | `YYYY-MM-DD`                                                  |
| `Bug-Next-Panel:`   | não         | URL de um issue específico (o `Bug:` do DEP-3 e do upstream)   |
| `Signed-off-by:`    | sim         | quem aprovado o patch (`Reviewed-by:` também é aceito)         |

`Forwarded: not-needed` é o valor padrão: os patches não vão para o
upstream.

## Numeração

O prefixo numérico é uma **sequência global** que segue a ordem de criação
dos patches, que é também a ordem em que eles são aplicados. Um patch só
pode depender dos anteriores, por isso a série é sempre crescente.

O **domínio** vira o rótulo do arquivo, logo depois do número:

| Domínio   | Rótulo    | Escopo                                         |
| --------- | --------- | ---------------------------------------------- |
| `01`      | `i18n`    | i18n e locales                                 |
| `02`      | `gateway` | gateways de pagamento (Mercado Pago)           |
| `03`      | `billing` | billing e faturas                              |
| `04`-`09` | `core`    | comportamento geral do painel               |
| `10`+     | `ui`      | interface, marca e templates                   |

Exemplos: `0001-i18n-add-supported-locales.patch` (primeiro patch de i18n),
`0002-gateway-add-mercadopago.patch` (primeiro patch do gateway).

O domínio é passado no dump:

```bash
jx dump add-pt-br-locale --domain 01
```

O arquivo `patches/series` mantém a ordem: um caminho por linha, na ordem de
aplicação. Comentários com `#` são permitidos, no estilo quilt (comentário
de linha inteira começa na coluna 0; comentário ao final da linha precisa de
um espaço antes do `#`).

## Regras de Manutenção e Sequência Numérica

1. **Um patch canônico por categoria/feature (`0001..00NN` contíguo)**:
   - A sequência numérica não pode ter buracos (`0001, 0002, 0003...`).
   - **Nunca apague um patch intermediário (ex.: `0004`) para recriá-lo no fim da fila (`atual + 1`)**: isso quebraria a contiguidade da série, destruiria a ordem topológica de dependências e poluíria o histórico do Git.
   - **Nunca crie um patch `atual + 1` para remendar arquivos que já pertencem a um patch anterior (ex.: `0004`)**: tocar o mesmo arquivo em múltiplos patches gera conflitos em cascata quando o upstream atualiza.
2. **Quando atualizar o próprio patch in-place (`jx refresh`) vs criar `atual + 1` (`jx dump`)**:
   - **Atualização do upstream (`jx rebase`) ou ajuste em categoria/feature já existente**: atualize o próprio patch dono do arquivo (`jx refresh patches/0004-...patch --force`). O `jx refresh` aceita **qualquer** patch da série, rebobina até ele, incorpora a alteração mantendo o cabeçalho DEP-3 e o número `0004`, e reaplica automaticamente os patches seguintes (`0005..00NN`) e os `overlays/`.
   - **Nova feature/categoria inédita**: use `jx dump <slug> --domain <NN>` para criar `atual + 1` no final da série.
   - **Rodada ativa do pipeline multiagente (`jx i18n-queue`)**: durante rodadas concorrentes de revisão/correção de uma categoria ainda aberta, lotes temporários podem ser adicionados ao final e devem ser consolidados no patch canônico único da categoria ao encerrá-la.
3. Nada de lockfile (`composer.lock`, `pnpm-lock.yaml`), `vendor/`,
   `node_modules/`, `public/build` ou `.env` dentro de um patch.
4. Patch é sempre reversível: `git apply --reverse` precisa funcionar, e o
   lint verifica isso na série inteira.
5. Nada de edição manual de hunks no arquivo `.patch`. Ele é gerado por
   `jx dump` e atualizado in-place por `jx refresh`.
6. Arquivos que não existem no upstream (arquivos novos) vão em `overlays/`,
   não como patch. Ver `docs/overlays.md`.

## Fluxo

### 1. Criando uma feature/categoria nova (`atual + 1`)

```bash
jx setup                        # prepara o workspace com o upstream fixado
# ... edite os arquivos novos da feature dentro de .upstream/ ...
jx dump meu-patch --domain 02   # gera o patch NNNN+1 e registra no series
jx lint                         # valida a serie inteira
```

### 2. Alterando um patch existente in-place (ex.: `0004`)

Com a série aplicada em `.upstream/`, edite o arquivo pertencente ao escopo do
patch `0004` e rode:

```bash
jx refresh patches/0004-i18n-shared-runtime.patch --force
jx lint
```

O `jx refresh` captura o diff pendente no workspace, volta ao commit do `0004`,
aplica a mudança nele, regenera `patches/0004-i18n-shared-runtime.patch`
preservando seu cabeçalho DEP-3 e reaplica automaticamente `0005..0026` e
`overlays/`.

### 3. Atualizando o upstream (`jx rebase`) e recuperando de conflitos

Para atualizar o upstream:

```bash
jx rebase v4.0.8        # ou: jx rebase develop
```

O pin em `config.toml` só é gravado depois que a série inteira aplica sem
conflito. Quando um patch (ex.: `0004`) conflita com a nova base, o `jx` exibe
o diagnóstico completo (arquivos, hunks, linhas exatas dos marcadores
`<<<<<<<` e diff do que mudou no upstream) e mantém os marcadores em
`.upstream/`:

```bash
# 1. Resolva os marcadores de conflito nos arquivos de .upstream/
# 2. Atualize o próprio patch 0004 in-place (ele commita o 0004 e reaplica 0005..0026 + overlays):
jx refresh patches/0004-i18n-shared-runtime.patch --force
# 3. Valide a série:
jx lint
```

## Histórico

- **2026-10-24 — consolidação 47 -> 26.** A série cresceu um patch por
  estágio/rodada (base + `review-r1` por categoria, mais 6 patches de
  domínio pós-pipeline) e foi consolidada em 26 patches: 23 de i18n (`0001`
  locales + `0002..0023` categorias), 1 de gateway (`0024`), 1 de i18n-followup
  (`0025`) e 1 de core (`0026`).
- **2026-10-27 — consolidação de sobreposições (`0027` e `0025`) e suporte a
  `jx refresh` em qualquer patch da série.** As alterações de datas dinâmicas,
  labels de cobrança e PDF (`0027`) e os 7 arquivos catalogados de `0025` foram
  incorporados in-place aos seus respectivos patches canônicos de categoria
  (`0002`, `0003`, `0005`, `0007`..`0010`, `0012`..`0014`, `0018`..`0020`),
  eliminando 100% das sobreposições de arquivos entre patches de i18n com prova
  de equivalência exata de árvore Git (`5fd3c93bb1c02c94ab9e9538f0a940f4e0425341`).
