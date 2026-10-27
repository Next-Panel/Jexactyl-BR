# Tabelas de frequência Zipf (EN/PT)

`zipf_en.tsv.gz` e `zipf_pt.tsv.gz`: uma linha `palavra\tzipf` por vocábulo,
comprimidas com gzip e embutidas no binário via `include_bytes!`.

## Proveniência e licença

Geradas a partir do pacote [`wordfreq`](https://github.com/rspeer/wordfreq)
(tabelas `large_en` e `large_pt`), sem poda: todos os vocábulos foram
mantidos porque os limiares do classificador (`token_margin`,
`zipf_ambiguity_gap`, `function_word_pt_zipf`) dependem dos valores exatos.

- Código do `wordfreq`: Apache-2.0.
- **Dados: Creative Commons Attribution-ShareAlike 4.0**
  (<https://creativecommons.org/licenses/by-sa/4.0/>).
  Dados extraídos do Google Books Ngrams
  (<http://books.google.com/ngrams>) e do Google Books Syntactic Ngrams.

Ao redistribuir este repositório (incluindo o binário `jx`, que embute
essas tabelas), preserve este aviso e a atribuição acima, conforme exige
a licença CC-BY-SA 4.0.

## Regeneração

```bash
python3 - <<'EOF'
import gzip
from wordfreq import zipf_frequency, top_n_list
for lang, name in (("en", "zipf_en"), ("pt", "zipf_pt")):
    with gzip.open(f"jx-tools/crates/jx_i18n/data/{name}.tsv.gz", "wt", encoding="utf-8") as out:
        for word in top_n_list(lang, 400_000):
            out.write(f"{word}\t{zipf_frequency(word, lang):.2f}\n")
EOF
```

Após regenerar, rode `cargo test -p jx_i18n lang` — os testes de paridade
detectam qualquer divergência nos valores.
