# jx-tools — reescrita Rust dos scripts do overlay

Binário `jx` + crates por domínio. Substituiu `scripts/*.sh` e o núcleo
git/config dos scripts; os scripts Python/JS (`scripts/i18n/*`, `tests/i18n.sh`,
`tests/i18n.mjs`, `tests/audit.mjs`) foram removidos após paridade verificada.

## Uso

```bash
cd jx-tools
cargo build
./target/debug/jx --config ../config.toml <comando>
```

Gates: `cargo fmt --check`, `cargo test --workspace`,
`cargo clippy --workspace --all-targets -- -D warnings`.

## Mapa shell -> `jx`

| Script | Comando |
|---|---|
| `lib/common.sh` (config/série) | `jx config-get`, `jx config-set`, `jx series-entries`, `jx series-insert` |
| `apply.sh` | `jx apply [--check] [--reverse] [--skip E] [--target D] [--no-snapshot]` |
| `lint-patches.sh` | `jx lint [--keep-cache]` |
| `dump-patch.sh` | `jx dump <slug> [...]`, `jx refresh <entrada> [--force]` |
| `dev-setup.sh` | `jx setup [--with-deps] [--force] [--ref R] [--sha S] [--local P]` |
| `rebase.sh` | `jx rebase [REF] [--sha S] [--no-pin] [--dry-run] [--force]` |
| `package.sh` | `jx package [--version V] [--output F]` (`SOURCE_DATE_EPOCH` suportado) |
| `scripts/i18n/catalog.sh` (removido) | `jx i18n-catalog (--write\|--check) [--json]` |
| `jx i18n-queue <subcomando>` | `init`, `status`, `next`, `claim`, `heartbeat`, `release`, `complete`, `advance`, `record-consistency`, `record-gate`, `record-patch`, `record-overlay`, `set-patch-units`, `record-shared-override`, `recover`, `block`, `unblock`, `reopen`, `validate`, `render` |
| `scripts/i18n/audit.sh` (removido) | `jx i18n-audit --unit\|--category [--json] [--base] [--strict]` |
| `jx bridge-analyze-pair`, `jx bridge-skeleton`, `jx bridge-extract-comments`, `jx bridge-extract-php-entries`, `jx bridge-placeholders`, `jx bridge-nlp-probe` | harness JSON compatível (stdin/stdout, mesmos campos) |

O `jx i18n-queue` implementa os subcomandos da fila sobre SQLite, com o banco
padrão em `<root>/.br-cache/i18n-queue.sqlite` (`--db <caminho>` ou
`BR_I18N_CACHE` sobrescrevem). Ele deliberadamente **não grava `CACHE.md`**
(`render` só imprime o resumo humano).

## Ferramental i18n (motor Rust, concluído)

| Módulo original | Crate | Estado |
|---|---|---|
| `i18n_toolkit/models.py` | `jx_i18n::core` | concluído (`Severity`, `Finding`) |
| `i18n_toolkit/text.py` | `jx_i18n::text` | concluído, paridade verificada |
| `i18n_toolkit/php_lang.py` | `jx_i18n::php` | concluído, paridade byte-a-byte em 3 arquivos reais |
| `i18n_toolkit/catalog.py` (manifesto, globs, atribuição, hash, perfis) | `jx_i18n::catalog` | concluído, hash golden verificado |
| `nlp.py` (língua + wordfreq) | `jx_i18n_lang` | concluído (motor: `lingua-rs` EN/PT + Zipf integral) |
| `scanner.py` | `jx_i18n_scan` | concluído (híbrido: tree-sitter p/ estrutura JS-TS-PHP + regras portadas; Blade manual) |
| `audit.py` | `jx_i18n_audit` | concluído |
| `queue.py` | `jx_i18n_store` (SQLite + TM Jaccard) | concluído |

Decisões dos spikes: tree-sitter sozinho e NO-GO (sem gramática Blade);
`lingua-rs` aprovado (confianças bit-idênticas, offline, Apache-2.0);
SQLite + TM Jaccard aprovado com condição (reaper de lease como queries).

## Typesafety (regras do workspace)

- Newtypes em vez de `String`: `Sha40`, `PatchNumber`, `SeriesEntry`,
  `PatchFileName`, `Origin`, `Forwarded`, `AppliedUpstream`, `IsoDate`,
  `Subject`, `Signoff`, `ConfigKey`.
- `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`,
  `#![deny(clippy::unwrap_used, clippy::expect_used)]` em todas as crates.
- Costuras de teste: `GitBackend` (+ `ProcessGit` / `ScriptGit`),
  `FileSystem` (+ `RealFs` / `MemFs`), `Clock` (+ `SystemClock` / fixo),
  `CmdRunner` (deps opcionais).
- Erros tipados por operação; `anyhow` só na borda do CLI.

## Paridades conscientes (divergências intencionais)

- `dev-setup --sha X`: checkout em `X`, mas overlays/reverso validados
  contra o pin do config (igual ao shell).
- `release.json`: escapa JSON corretamente (o shell interpola sem escape).
- `jx lint` valida o formato do SHA antes (o shell conta como falha).
- `set_path` do tarball usa `append_data` (LongLink) para paths >100 bytes.

## Fora do escopo

- `scripts/consolidate-patches.sh`: one-shot 47->26 já executado; legado
  (removido junto aos demais scripts; o procedimento está registrado em
  `patches/README.md`).
- `--with-deps` roda `composer`/`pnpm` via `CmdRunner` (sem sandbox).
