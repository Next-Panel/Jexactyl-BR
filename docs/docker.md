# Docker — painel com banco configurado

Sobe o Jexactyl com a tradução pt-BR embutida na imagem e o banco
(MariaDB) e o cache (Redis) já ligados. Equivale a `make setup + make deps`
empacotados numa imagem reprodutível.

## Subida simples (imagem remota)

```bash
cp .env.example .env   # opcional; os padrões já funcionam localmente
docker compose up -d
docker compose logs -f panel
```

Aguarde o `migrate --seed` do entrypoint e abra `http://localhost:8085` (ou o `PANEL_PORT` configurado).

## Build local (desenvolvimento)

```bash
make build   # compila imagem local com o pin de config.toml
docker compose up -d
```

O `make build` injeta `UPSTREAM_URL/REF/SHA` direto de `config.toml`. Use quando alterar patches, overlays ou o Dockerfile.

Pipeline da imagem (`docker/Dockerfile`):

1. `src`: clona o upstream no SHA fixado e roda
   `docker/apply-series.sh` — mesma semântica do `jx apply` para árvore
   limpa: confere o SHA, valida que toda entrada do `series` tem patch,
   recusa overlay colidindo com arquivo do upstream, aplica em ordem e
   prova com `apply --check --reverse` que a série inteira está aplicada;
2. `assets`: compila o frontend já traduzido com pnpm pinado e
   `--frozen-lockfile`;
3. runtime PHP: instala dependências com composer travado, ajusta permissões
   (`nginx:nginx` só em `storage`/`bootstrap/cache`) e checa no build que
   os CLIs do entrypoint existem (`mysql`, `nc`).

## O que já vem configurado

- **Tradução:** a imagem aplica `patches/series` + `overlays/` sobre o
  upstream fixado em `config.toml`, depois compila o frontend. O
  `APP_LOCALE` padrão é `pt-BR` (sobrescreva se quiser `en`).
- **Banco:** serviço `database` (MariaDB 11) com volume `db-data`;
  o painel recebe `DB_HOST/DATABASE/USERNAME/PASSWORD` já apontados.
- **Cache/sessão/fila:** Redis local (`cache`).
- **Admin inicial:** defina `PANEL_ADMIN_EMAIL` e `PANEL_ADMIN_PASSWORD`
  no `.env` antes do primeiro boot; o entrypoint cria o admin uma vez
  (idempotente — boots seguintes pulam).
- **Faturamento:** `BILLING_ENABLED=true` por padrão; `BILLING_GATEWAY`
  escolhe o gateway pré-selecionado (`stripe` ou `mercadopago`).
  Stripe usa `BILLING_SECRET_KEY`; Mercado Pago usa
  `MP_ACCESS_TOKEN`/`MP_WEBHOOK_SECRET` (ou pela tela Configurações
  do admin). Moeda: ajuste para BRL na tela de Configurações do faturamento após o primeiro boot.

## Variáveis principais

| Variável | Padrão | Para que serve |
| --- | --- | --- |
| `PANEL_IMAGE` | `ghcr.io/next-panel/jexactyl-br:latest` | imagem do painel (remota com pt-BR; `up --build` compila local) |
| `PANEL_PORT` | `8085` | porta HTTP publicada |
| `APP_URL` | `http://localhost:8085` | URL pública (usada nos retornos de pagamento) |
| `APP_LOCALE` | `pt-BR` | idioma do painel |
| `DB_DATABASE/DB_USERNAME/DB_PASSWORD` | `jexactyl*` | banco do compose |
| `PANEL_ADMIN_EMAIL/PANEL_ADMIN_PASSWORD` | vazio | cria o admin no 1o boot |
| `BILLING_GATEWAY` | `stripe` | gateway pré-selecionado no checkout |
| `BILLING_SECRET_KEY` | vazio | chave secreta da Stripe |
| `MP_ACCESS_TOKEN/MP_WEBHOOK_SECRET/MP_SANDBOX` | vazio/`true` | Mercado Pago |
| `LE_EMAIL` | vazio | ativa HTTPS via Let's Encrypt (exige domínio) |

## Operação

```bash
docker compose up -d           # sobe os serviços em background
docker compose logs -f panel   # acompanha o log do painel
docker compose down            # derruba mantendo os volumes (banco preservado)
make doctor                    # confere pin config.toml=Dockerfile e valida o compose
make backup-db                 # dump do banco em ./backups/
```

Uploads (avatares/anexos) ficam no volume `panel-storage` e sobrevivem
a rebuilds. Migrando de uma instalação no host?
Ver `docs/migracao-host-para-docker.md`.

## Solução de problemas

- **Boot falha com `database ... unreachable after ~120s`:** o painel
  desiste em ~120s em vez de travar. Confira `DB_*` no `.env` e se o
  serviço `database` está saudável (`docker compose ps`, `docker compose logs database`).
- **Troquei `APP_URL`/`LE_EMAIL` e o nginx ignorou:** o `panel.conf`
  gerado fica no volume `panel-nginx` e só é regenerado quando o
  domínio ou o modo LE muda (carimbo em `.panel-inputs`). Se editou à mão,
  apague o volume (`docker compose down -v` **apaga o banco** — faça
  `make backup-db` antes) ou remova só o `panel.conf` do volume.
- **`HASHIDS_SALT`/`APP_KEY` com erro de parse:** o entrypoint gera os dois
  só em charset alfanumérico; se você definiu valores com `#`, espaço ou
  aspas no `.env`, cite com aspas simples.
- **Imagem local dessincronizada:** `make doctor` acusa drift entre
  `config.toml` e `docker/Dockerfile`; `make build` sempre usa o pin do
  `config.toml`.

## Produção

Troque `DB_PASSWORD`/`DB_ROOT_PASSWORD`, defina `APP_URL` com o domínio
e `LE_EMAIL` para HTTPS, e guarde o `.env` fora do repo. A imagem é
reprodutível: rebuilds usam o SHA fixado em `config.toml`
(ver `docker/Dockerfile` e `docker/apply-series.sh`).
