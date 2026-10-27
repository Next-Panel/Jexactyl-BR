# Migração: instalação no host para Docker

Como mover um painel já instalado na máquina (host) para a stack
`docker compose` deste repositório — e como fazer uma instalação nova
do jeito fácil. Os dois fluxos usam o mesmo Compose; o que muda é de
onde vem os dados.

## Instalação nova (jeito fácil)

```bash
cp .env.example .env   # opcional; os padrões já funcionam localmente
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

---

## Migração de um painel existente

### 1. Inventário no host

Anote do `.env` do host: `APP_KEY`, `APP_URL`, credenciais do banco,
`MAIL_*` e qualquer variável customizada.

`APP_KEY` e `HASHIDS_SALT` são inegociáveis: têm que ser **os mesmos** no Docker. Trocar a
chave invalida sessões e dados criptografados (o entrypoint gera uma
aleatória se a variável não existir).

> **Aviso sobre os arquivos dos servidores (Wings):**
> Os servidores de jogos, uploads e backups dos clientes **não ficam no painel**, eles vivem exclusivamente no daemon **Wings** (`/var/lib/pterodactyl/volumes/`). A migração transfere apenas o banco de dados e a interface web; os servidores nas Wings continuam intactos e funcionando.

### 2. Backup antes de tudo

```bash
# 1. Pausa o worker de filas do host
sudo systemctl stop pteroq.service
# Caso seu serviço tenha sido configurado como panel.service:
# sudo systemctl stop panel.service

# 2. Exporta o banco de dados do painel atual
mariadb-dump -u root -p jexactyl > painel-backup.sql
# ou: mysqldump -u root -p jexactyl > painel-backup.sql

# 3. Salva uma cópia do .env
cp /var/www/jexactyl/.env ./env.host.bak
```

### 3. Preparar o `.env` do Compose

Clone o repositório na pasta desejada (o Git cria o diretório automaticamente):

```bash
git clone https://github.com/Next-Panel/Jexactyl-BR.git /var/www/jexactyl-docker
cd /var/www/jexactyl-docker
cp .env.example .env
```

Preencha com os valores do host:

| Origem (host) | Destino (`.env` do compose) | Observação |
|---|---|---|
| `APP_KEY` | `APP_KEY` | **obrigatório e idêntico**; entra via `env_file` |
| `HASHIDS_SALT` | `HASHIDS_SALT` | **obrigatório e idêntico** se existia no host |
| `APP_URL` | `APP_URL` | URL pública (usada nos retornos de pagamento e links) |
| Banco | `DB_DATABASE` / `DB_USERNAME` / `DB_PASSWORD` | credenciais do novo MariaDB no Docker |
| `MAIL_*` | mesmas chaves no `.env` | entram via `env_file`; não precisa mexer no compose |
| `BILLING_*` / Mercado Pago | `.env` ou tela de Configurações do admin | pode reconfigurar pela UI depois |

> **Nota sobre credenciais do banco no Docker:**
> As credenciais `DB_DATABASE`, `DB_USERNAME` e `DB_PASSWORD` pertencem ao container MariaDB criado pelo Compose. Você pode usar as mesmas credenciais da instalação antiga ou definir novas senhas; apenas garanta que coincidam com o que estiver configurado no `.env`.

Não use `PANEL_ADMIN_*` na migração: o admin **já vem no banco** e o
entrypoint pula a criação quando o e-mail existe.

### 4. Subir o banco e importar o dump

```bash
# Sobe banco e cache
docker compose up -d database cache

# Aguarda o banco ficar saudável (healthy)
docker compose ps
```

Assim que o container `database` estiver com status `healthy`:

```bash
docker compose exec -T database mariadb -u<DB_USERNAME> -p<DB_PASSWORD> <DB_DATABASE> < painel-backup.sql
```

### 5. Subir o painel (Migrações automáticas)

```bash
docker compose up -d panel
docker compose logs -f panel
```

O entrypoint executará automaticamente `php artisan migrate --force`, adaptando as tabelas importadas da versão anterior para o schema moderno da **v4** sem qualquer necessidade de comandos manuais.

### 6. Desativar serviços antigos do host

```bash
sudo systemctl disable --now pteroq.service
# Caso seu serviço tenha sido configurado como panel.service:
# sudo systemctl disable --now panel.service
```

### 7. Verificar

- Login com o admin migrado em `APP_URL`.
- Faturamento: confira gateway, moeda (BRL) e chaves do Mercado Pago.
- Servidores: valide se os servidores e nós continuam comunicando com as Wings normalmente.
