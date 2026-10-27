# Jexactyl — imagem do painel com a tradução pt-BR embutida.
#
# Os ARGs abaixo espelham config.toml. Não edite à mão: `make build` injeta
# os valores de config.toml e o job pin-drift do CI recusa drift.
#   UPSTREAM_URL / UPSTREAM_REF / UPSTREAM_SHA.
#
# Estágio src: clona o upstream fixado e aplica a série de patches
# (patches/series, na ordem) mais os overlays, via docker/apply-series.sh
# (mesma semântica do `jx apply` para árvore limpa + checagens do `jx lint`).
# Estágios seguintes repetem o Dockerfile oficial, só que sobre o
# fonte já traduzido.

ARG UPSTREAM_URL=https://github.com/Jexactyl/Jexactyl.git
ARG UPSTREAM_REF=v4.0.7
ARG UPSTREAM_SHA=5927ca45f62a2ce288f001f8c07cf6c06bcbbf3d

# Stage src: fonte do upstream + patches + overlays.
FROM alpine/git AS src
ARG UPSTREAM_URL
ARG UPSTREAM_REF
ARG UPSTREAM_SHA
WORKDIR /build
RUN set -eu \
    && git clone "${UPSTREAM_URL}" upstream \
    && cd upstream \
    && git fetch origin "${UPSTREAM_REF}" \
    && git checkout "${UPSTREAM_SHA}" \
    && test "$(git rev-parse HEAD)" = "${UPSTREAM_SHA}"
COPY docker/apply-series.sh /build/apply-series.sh
RUN chmod +x /build/apply-series.sh
COPY patches/series /build/patches.series
COPY patches/*.patch /build/patches/
COPY overlays /build/overlays
RUN set -eu \
    && /build/apply-series.sh /build/upstream /build /build/patches.series /build/overlays "${UPSTREAM_SHA}" \
    && test -f /build/upstream/resources/lang/pt-BR/auth.php \
    && test -f /build/upstream/config/app.php \
    && grep -q "pt-BR" /build/upstream/config/app.php

# Stage 0: compila o frontend (já traduzido) e descarta o Node em seguida.
FROM --platform=$TARGETOS/$TARGETARCH node:22-alpine AS assets
WORKDIR /app
COPY --from=src /build/upstream ./
# pnpm pinado pelo packageManager do upstream; --frozen-lockfile garante
# que o build só usa o lockfile versionado (nunca resolve de novo).
RUN --mount=type=cache,target=/root/.local/share/pnpm \
    set -eu \
    && corepack enable \
    && corepack prepare pnpm@9.0.6 --activate \
    && pnpm install --frozen-lockfile \
    && pnpm build \
    && test -f public/build/manifest.json

# Stage 1: runtime PHP com as dependências do painel.
FROM --platform=$TARGETOS/$TARGETARCH php:8.4-fpm-alpine
WORKDIR /app
COPY --from=src /build/upstream ./
COPY --from=assets /app/public/build ./public/build
COPY --from=composer:2 /usr/bin/composer /usr/local/bin/composer
RUN set -eu \
    && apk add --no-cache --update ca-certificates dcron curl git supervisor tar unzip nginx libpng-dev libxml2-dev libzip-dev certbot certbot-nginx mariadb-client \
    && command -v mysql || command -v mariadb \
    && command -v nc \
    && docker-php-ext-configure zip \
    && docker-php-ext-install bcmath gd pdo_mysql zip \
    && cp .env.example .env \
    && mkdir -p bootstrap/cache/ storage/logs storage/framework/sessions storage/framework/views storage/framework/cache \
    && composer install --no-dev --no-interaction --no-progress --optimize-autoloader \
    && rm -rf .env bootstrap/cache/*.php \
    && mkdir -p /app/storage/logs/ \
    && chown -R nginx:nginx storage bootstrap/cache \
    && chmod -R 775 storage bootstrap/cache
RUN set -eu \
    && rm -f /usr/local/etc/php-fpm.conf \
    && echo "* * * * * su -s /bin/sh nginx -c '/usr/local/bin/php /app/artisan schedule:run' >> /dev/null 2>&1" >> /var/spool/cron/crontabs/root \
    && echo "0 23 * * * certbot renew --nginx --quiet" >> /var/spool/cron/crontabs/root \
    && sed -i 's/ssl_session_cache/#ssl_session_cache/g' /etc/nginx/nginx.conf \
    && mkdir -p /var/run/php /var/run/nginx
COPY --from=src /build/upstream/.github/docker/default.conf /etc/nginx/http.d/default.conf
COPY --from=src /build/upstream/.github/docker/www.conf /usr/local/etc/php-fpm.conf
COPY --from=src /build/upstream/.github/docker/supervisord.conf /etc/supervisord.conf
# Entrypoint: igual ao oficial + admin inicial e padrões deste projeto.
COPY docker/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh
EXPOSE 80 443
ENTRYPOINT [ "/bin/ash", "/entrypoint.sh" ]
CMD [ "supervisord", "-n", "-c", "/etc/supervisord.conf" ]
