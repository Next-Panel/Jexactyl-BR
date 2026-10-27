# Overlays

Overlays são arquivos **novos** do Jexactyl que não existem no upstream.
Eles são copiados por cima da copia de trabalho depois que os patches são
aplicados, então tem precedência sobre qualquer coisa alterada por patch.

Use overlay quando:

- o arquivo é inteiramente novo (ex.: `resources/lang/pt-BR/auth.php`);
- o arquivo precisa ser substituído por completo, sem diff confuso
  (ex.: um template reescrito);
- o conteúdo é grande demais para viver dentro de um patch revisável.

Não use overlay quando:

- o arquivo já existe no upstream e você só quer mudar algumas linhas
  (use patch);
- você precisa que a mudança sobreviva a um rebase do upstream com conflito
  explícito (patch mostra o conflito, overlay não).

> **Regra dura:** um overlay nunca pode sobrescrever um arquivo que existe no
> upstream. O `jx apply` e o `jx lint` recusam isso e
> mandam usar um patch. Isso impede que um overlay apague silenciosamente um
> arquivo do upstream.

## Estrutura

O caminho do overlay espelha o caminho no upstream:

```
overlays/
  resources/
    lang/
      pt-BR/
        auth.php
```

`overlays/resources/lang/pt-BR/auth.php` vira
`.upstream/resources/lang/pt-BR/auth.php` no workspace.

## Regras

1. Arquivo novo = overlay. Arquivo existente = patch.
2. Um overlay por arquivo. Pastas inteiras são permitidas.
3. Overlays não passam pelo `git apply`, então não tem cabeçalho DEP-3.
4. A documentação do overlay vive em `docs/`, nunca dentro de `overlays/`
   (o diretório inteiro é copiado para o workspace).
5. Os arquivos copiados são registrados no manifesto
   `.upstream/.git/br-overlays.txt`, o que permite remover em caso de
   rebase ou de reverso da série.
