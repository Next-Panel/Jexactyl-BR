# Migração: instalação no host para Docker

Como mover um painel já instalado na máquina (host) para a stack
`docker compose` deste repositório — e como fazer uma instalação nova
do jeito fácil. Os dois fluxos usam o mesmo Compose; o que muda é de
onde vem os dados.

## Instalação nova (jeito fácil)

```bash
cp .env.example .env   # opcional; os padroes ja funcionam localmente
docker compose up -d
docker compose logs -f panel
```

Para criar o admin inicial, preencha `PANEL_ADMIN_EMAIL` e
`PANEL_ADMIN_PASSWORD` no `.env` **antes** do primeiro boot (apague as
duas linhas após o primeiro login — elas não são mais necessárias).
Alternativa sem senha em arquivo:

```bash
docker compose exec panel php artisan p:user:make
```

## Migração de um painel existente

### 1. Inventário no host

Anote do `.env` do host: `APP_KEY`, `APP_URL`, credenciais do banco,
`MAIL_*` e qualquer variável customizada. Localize o banco
(`mysqldump` será necessário) e `storage/app/public` (avatares e anexos).

`APP_KEY` é inegociável: tem que ser **o mesmo** no Docker. Trocar a
chave invalida sessões e dados criptografados (o entrypoint gera uma
aleatória se a variável não existir).

### 2. Backup antes de tudo

```bash
mysqldump -u root -p <banco_do_painel> > painel-backup.sql
cp .env .env.host.bak
tar -czf storage-public-backup.tar.gz -C /caminho/do/painel/storage/app public
```

### 3. Preparar o `.env` do Compose

```bash
cp .env.example .env
```

Preencha com os valores do host:

| Origem (host) | Destino (`.env` do compose) | Observação |
|---|---|---|
| `APP_KEY` | `APP_KEY` (adicione a linha) | **obrigatório e idêntico**; entra via `env_file` |
| `APP_URL` | `APP_URL` | URL pública nova (usada nos retornos de pagamento) |
| Banco | `DB_DATABASE` / `DB_USERNAME` / `DB_PASSWORD` | padrão `jexactyl` / `jexactyl` / `jexactyl_dev` |
| `MAIL_*` | mesmas chaves no `.env` | entram via `env_file`; não precisa mexer no compose |
| `BILLING_*` / Mercado Pago | `.env` ou tela de Configurações do admin | pode reconfigurar pela UI depois |

Não use `PANEL_ADMIN_*` na migração: o admin **já vem no banco** e o
entrypoint pula a criação quando o e-mail existe. Para um admin extra,
use `docker compose exec panel php artisan p:user:make`.

### 4. Subir o banco e importar o dump

```bash
docker compose up -d database cache
for tentativa in $(seq 1 30); do
  CONTAINER_ID="$(docker compose ps -q database)"
  [ -n "${CONTAINER_ID}" ] && [ "$(docker inspect -f '{{.State.Health.Status}}' "${CONTAINER_ID}")" = "healthy" ] && break
  sleep 10
done
docker compose exec -T database mariadb -ujexactyl -pjexactyl_dev jexactyl < painel-backup.sql
```

Troque usuário/senha/banco pelos valores do seu `.env`. Se o host rodava
MySQL em vez de MariaDB, confira collation e grants após o import.

### 5. Subir o painel e copiar os uploads

```bash
docker compose up -d
docker compose logs -f panel   # aguarde migrate --seed concluir
```

Com o painel no ar, copie os uploads para o volume `panel-storage`:

```bash
docker compose cp storage-public-backup.tar.gz panel:/tmp/
docker compose exec panel sh -c 'tar -xzf /tmp/storage-public-backup.tar.gz -C /app/storage/app/ && rm /tmp/storage-public-backup.tar.gz'
```

O boot já rodou `migrate --seed` e refez o `storage:link`; os uploads
ficam preservados no volume `panel-storage`.

### 6. Verificar

- Login com o admin migrado em `APP_URL`.
- Faturamento: confira gateway, moeda (BRL) e chaves do Mercado Pago.
- Nodes/daemons (Wings): ficam onde estão — garanta que o novo
  `APP_URL` seja alcançável por eles e confira o FQDN de cada node.
- Fila/agendador: `crond` e `supervisord` já rodam no container.

### 7. Limpeza e rollback

- Remova `PANEL_ADMIN_*` do `.env` se tiver usado no bootstrap.
- Rollback: o host original continua intacto até você apontar o DNS e os
  daemons de volta para ele; guarde o dump e o `.env.host.bak`.

## Armadilhas conhecidas

- **Stacks antigas deste repositório**: o nome padrão do banco mudou
  (`jexactyl_br` → `jexactyl`) e o volume de uploads é novo
  (`panel-storage`). Com volumes velhos na máquina, recrie do zero
  (`docker compose down -v`) **após backup** — ou ajuste os nomes para
  os antigos.
- **Variáveis fora da lista**: o container só recebe o que está em
  `environment:` no compose **mais** tudo que estiver no `.env`
  (`env_file`). Variável customizada do host vai para o `.env`, sem
  precisar editar o compose.
- **HTTPS**: defina `APP_URL` com o domínio e `LE_EMAIL` para o
  entrypoint emitir o certificado, ou mantenha o reverse proxy atual.
