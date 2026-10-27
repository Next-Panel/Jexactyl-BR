#!/bin/ash -eu
# shellcheck shell=dash
# Entrypoint do Jexactyl: segue o entrypoint oficial
# (.github/docker/entrypoint.sh do upstream) com os blocos de locale padrão
# e admin inicial ao final. Manter sincronizado com o upstream.
#
# Invariantes:
# - todas as variáveis têm default (o container também sobe via `docker run`);
# - espera do banco é limitada (falha alto em vez de travar o boot);
# - `nginx -t` valida o config gerado antes de subir o supervisord;
# - o nginx só é reconfigurado quando domínio/LE mudam (o volume
#   panel-nginx persiste o panel.conf entre boots).
#
# Nota: o Dockerfile usa ENTRYPOINT exec-form ("/bin/ash", script), que
# ignora flags do shebang; por isso o `set -eu` abaixo e explicito.
set -eu
cd /app

# Defaults (mesmos do compose; permitem `docker run` sem compose).
DB_HOST="${DB_HOST:-database}"
DB_PORT="${DB_PORT:-3306}"
DB_DATABASE="${DB_DATABASE:-jexactyl}"
DB_USERNAME="${DB_USERNAME:-jexactyl}"
DB_PASSWORD="${DB_PASSWORD:-jexactyl_dev}"
APP_URL="${APP_URL:-http://localhost:8085}"
LE_EMAIL="${LE_EMAIL:-}"
PANEL_ADMIN_EMAIL="${PANEL_ADMIN_EMAIL:-}"
PANEL_ADMIN_PASSWORD="${PANEL_ADMIN_PASSWORD:-}"

mkdir -p /var/log/panel/ /var/log/supervisord/ /var/log/nginx/ /var/log/php8/ \
  && ln -sfn /app/storage/logs/ /var/log/panel/logs

## check for .env file and generate app keys if missing
if [ -f /app/var/.env ]; then
  echo "external vars exist."
  rm -rf /app/.env
  ln -sfn /app/var/.env /app/.env
else
  echo "external vars don't exist."
  rm -rf /app/.env
  touch /app/var/.env

  ## manually generate a key because key generate --force fails
  ## Desvio intencional do oficial: aqui o valor gerado NUNCA vai para o log.
  if [ -z "${APP_KEY:-}" ]; then
    echo "Generating key."
    APP_KEY="$(cat /dev/urandom | tr -dc 'A-Za-z0-9' | fold -w 32 | head -n 1)"
    echo "Generated app key (valor omitido nos logs)."
    echo "APP_KEY=${APP_KEY}" > /app/var/.env
  else
    echo "APP_KEY exists in environment, using that."
    echo "APP_KEY=${APP_KEY}" > /app/var/.env
  fi

  ## generate a random salt for hashids if not provided.
  ## Charset restrito a alfanuméricos: o valor vai cru para o .env, onde
  ## espaços, `#` ou aspas quebrariam o parse (culpa comum de boot falho).
  if [ -z "${HASHIDS_SALT:-}" ]; then
    echo "Generating hashids salt."
    HASHIDS_SALT="$(cat /dev/urandom | tr -dc 'A-Za-z0-9' | fold -w 20 | head -n 1)"
    echo "Generated hashids salt (valor omitido nos logs)."
    echo "HASHIDS_SALT=${HASHIDS_SALT}" >> /app/var/.env
  else
    echo "HASHIDS_SALT exists in the environment, using that."
    echo "HASHIDS_SALT=${HASHIDS_SALT}" >> /app/var/.env
  fi

  ln -sfn /app/var/.env /app/.env
fi

APP_DOMAIN="$(printf '%s' "$APP_URL" | sed -e 's~^[a-zA-Z][a-zA-Z0-9+.-]*://~~' -e 's~/.*$~~' -e 's~:.*$~~')"
if [ -z "$APP_DOMAIN" ]; then
  echo "error: APP_URL inválida: $APP_URL" >&2
  exit 1
fi

echo "Checking if https is required."
PANEL_CONF=/etc/nginx/http.d/panel.conf
INPUTS_FILE=/etc/nginx/http.d/.panel-inputs
INPUTS_FINGERPRINT="${APP_DOMAIN}|le_set=$([ -n "$LE_EMAIL" ] && echo yes || echo no)"
if [ -f "$PANEL_CONF" ] && [ -f "$INPUTS_FILE" ] && [ "$(cat "$INPUTS_FILE")" = "$INPUTS_FINGERPRINT" ]; then
  echo "Using nginx config already in place."
  if [ -n "$LE_EMAIL" ]; then
    echo "Checking for cert update"
    certbot certonly -d "$APP_DOMAIN" --standalone -m "$LE_EMAIL" --agree-tos -n || \
      echo "warning: certbot renewal check failed for $APP_DOMAIN; keeping current config." >&2
  else
    echo "No letsencrypt email is set"
  fi
else
  echo "Checking if letsencrypt email is set."
  if [ -z "$LE_EMAIL" ]; then
    echo "No letsencrypt email is set using http config."
    cp .github/docker/default.conf "$PANEL_CONF"
    printf '%s' "$INPUTS_FINGERPRINT" > "$INPUTS_FILE"
  else
    echo "generating certs"
    if certbot certonly -d "$APP_DOMAIN" --standalone -m "$LE_EMAIL" --agree-tos -n; then
      echo "writing ssl config"
      cp .github/docker/default_ssl.conf "$PANEL_CONF"
      echo "updating ssl config for domain"
      sed -i "s|<domain>|${APP_DOMAIN}|g" "$PANEL_CONF"
      printf '%s' "$INPUTS_FINGERPRINT" > "$INPUTS_FILE"
    else
      echo "warning: certbot failed for $APP_DOMAIN; falling back to HTTP config." >&2
      cp .github/docker/default.conf "$PANEL_CONF"
      rm -f "$INPUTS_FILE"
    fi
  fi
  echo "Removing the default nginx config"
  rm -f /etc/nginx/http.d/default.conf
fi

## valida o config gerado antes de qualquer efeito colateral (migrate etc.)
echo "Testing nginx config."
nginx -t

## make sure the db port default is sane
if [ -z "${DB_PORT}" ]; then
  echo "DB_PORT not specified, defaulting to 3306"
  DB_PORT=3306
fi

## check log folder permissions
echo "Checking log folder permissions."
if [ "$(stat -c %U:%G /app/storage/logs)" != "nginx:nginx" ]; then
  echo "Fixing log folder permissions."
  chown -R nginx: /app/storage/logs/
fi

## check for DB up before starting the panel (bounded: fail fast on misconfig)
echo "Checking database status."
ATTEMPT=0
until nc -z "$DB_HOST" "$DB_PORT"; do
  ATTEMPT=$((ATTEMPT + 1))
  if [ "$ATTEMPT" -ge 60 ]; then
    echo "error: database ${DB_HOST}:${DB_PORT} unreachable after ~120s" >&2
    exit 1
  fi
  echo "Waiting for database connection... (${ATTEMPT}/60)"
  # wait for 2 seconds before check again
  sleep 2
done

## make sure the db is set up (isolated: parallel replicas don't race)
echo "Migrating and Seeding D.B"
php artisan migrate --isolated --seed --force

## Cria o admin inicial quando PANEL_ADMIN_EMAIL/PANEL_ADMIN_PASSWORD
## estão definidos e o e-mail ainda não existe (idempotente entre boots).
if [ -n "$PANEL_ADMIN_EMAIL" ] && [ -n "$PANEL_ADMIN_PASSWORD" ]; then
  echo "Checking admin user."
  # Cliente SQL: `mysql` (compat) ou `mariadb`, o que a imagem tiver.
  if command -v mysql >/dev/null 2>&1; then
    DBCLI=mysql
  elif command -v mariadb >/dev/null 2>&1; then
    DBCLI=mariadb
  else
    echo "error: nenhum cliente SQL (mysql/mariadb) na imagem" >&2
    exit 1
  fi
  # Escapa aspas simples: o e-mail vai interpolado no SQL abaixo.
  ADMIN_ESCAPED="$(printf '%s' "$PANEL_ADMIN_EMAIL" | sed "s/'/''/g")"
  ADMIN_EXISTS="$(MYSQL_PWD="$DB_PASSWORD" "$DBCLI" -h"$DB_HOST" -P"$DB_PORT" -u"$DB_USERNAME" "$DB_DATABASE" -N -e "SELECT COUNT(*) FROM users WHERE email='${ADMIN_ESCAPED}'" 2>/dev/null || echo "unknown")"
  if [ "$ADMIN_EXISTS" = "0" ]; then
    echo "Creating admin user $PANEL_ADMIN_EMAIL."
    php artisan p:user:make \
      --email="$PANEL_ADMIN_EMAIL" \
      --username="${PANEL_ADMIN_USERNAME:-admin}" \
      --name-first="${PANEL_ADMIN_NAME_FIRST:-Admin}" \
      --name-last="${PANEL_ADMIN_NAME_LAST:-Painel}" \
      --password="$PANEL_ADMIN_PASSWORD" \
      --admin=1
  else
    echo "Admin user already present (or unreachable check: $ADMIN_EXISTS), skipping."
  fi
fi

## make sure uploaded files (e.g. avatars) are publicly reachable
echo "Linking storage."
php artisan storage:link || true

## Garante que quaisquer logs/caches criados pelos comandos artisan acima (rodados como root)
## pertencem ao usuario nginx antes de iniciar o crond e o supervisord.
chown -R nginx:nginx /app/storage /app/bootstrap/cache

## start cronjobs for the queue
echo "Starting cron jobs."
crond -L /var/log/crond -l 5

echo "Starting supervisord."
exec "$@"
