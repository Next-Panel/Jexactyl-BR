# Atalhos para o overlay Jexactyl (ferramental Rust em jx-tools/).
#
# make setup   prepara o workspace com o upstream fixado
# make test    roda a suite Rust (cargo test --workspace)
# make lint    valida a serie de patches (mesmo gate do CI)

SHELL := /usr/bin/env bash

JXBIN := jx-tools/target/debug/jx
JXSRCS := $(wildcard jx-tools/Cargo.toml jx-tools/Cargo.lock jx-tools/crates/*/Cargo.toml jx-tools/crates/*/src/*.rs)
JX := $(JXBIN) --config config.toml

$(JXBIN): $(JXSRCS)
	cargo build --manifest-path jx-tools/Cargo.toml

.PHONY: help setup deps apply check lint test dump rebase package clean \
	build doctor backup-db \
	i18n-catalog i18n-check i18n-status i18n-test i18n-audit

help:
	@printf '%s\n' \
	  'setup    prepara o workspace em .upstream/' \
	  'deps     instala as dependencias do painel no workspace' \
	  'apply    aplica a serie de patches e os overlays' \
	  'check    valida se a serie aplica, sem alterar nada' \
	  'lint     valida a serie de patches (gate do CI)' \
	  'test     roda a suite Rust (cargo test --workspace)' \
	  'dump     gera um patch: make dump SLUG=meu-patch DOMAIN=01' \
	  'rebase   atualiza a base: make rebase REF=v4.0.7' \
	  'i18n-catalog  regenera o catalogo (i18n/catalog.lock.json)' \
	  'i18n-check    valida o catalogo (gate catalog)' \
	  'i18n-status   mostra o estado da fila de traducao' \
	  'i18n-test     roda a suite i18n Rust (fila, auditoria e catalogo)' \
	  'i18n-audit    audita uma categoria com --strict: make i18n-audit CATEGORY=<id> [BASE=<rev>]' \
	  'package  gera o pacote de release em dist/' \
	  'build    compila localmente a imagem Docker do painel (pins de config.toml)' \
	  'doctor   confere pre-requisitos e sincronia do pin do upstream' \
	  'backup-db  dump do banco do compose em ./backups/' \
	  'clean    remove o workspace e o cache'

setup: $(JXBIN)
	@$(JX) setup

deps: $(JXBIN)
	@$(JX) setup --with-deps

apply: $(JXBIN)
	@$(JX) apply

check: $(JXBIN)
	@$(JX) apply --check

lint: $(JXBIN)
	@$(JX) lint

test:
	@cargo test --manifest-path jx-tools/Cargo.toml --workspace

i18n-catalog: $(JXBIN)
	@$(JX) i18n-catalog --write

i18n-check: $(JXBIN)
	@$(JX) i18n-catalog --check

i18n-status: $(JXBIN)
	@$(JX) i18n-queue status

i18n-test: $(JXBIN)
	@cargo test --manifest-path jx-tools/Cargo.toml -p jx_i18n

i18n-audit: $(JXBIN)
	@test -n "$(CATEGORY)" || { printf '%s\n' 'uso: make i18n-audit CATEGORY=<id> [BASE=<rev>]'; exit 1; }
	@$(JX) i18n-audit --category $(CATEGORY) --strict $(if $(BASE),--base $(BASE))

dump: $(JXBIN)
	@test -n "$(SLUG)" || { printf '%s\n' 'uso: make dump SLUG=meu-patch DOMAIN=01'; exit 1; }
	@$(JX) dump $(SLUG) --domain $(or $(DOMAIN),01)

rebase: $(JXBIN)
	@test -n "$(REF)" || { printf '%s\n' 'uso: make rebase REF=v4.0.7'; exit 1; }
	@$(JX) rebase $(REF)

package: $(JXBIN)
	@$(JX) package

# Compila a imagem localmente injetando o pin do upstream direto de config.toml.
build:
	@set -eu; \
	eval "$$(python3 -c "import tomllib; u=tomllib.load(open('config.toml','rb'))['upstream']; print('UPSTREAM_URL=%s\nUPSTREAM_REF=%s\nUPSTREAM_SHA=%s' % (u['url'],u['ref'],u['sha']))" | sed 's/^/export /')"; \
	docker build -f docker/Dockerfile \
	  --build-arg UPSTREAM_URL="$$UPSTREAM_URL" \
	  --build-arg UPSTREAM_REF="$$UPSTREAM_REF" \
	  --build-arg UPSTREAM_SHA="$$UPSTREAM_SHA" \
	  -t "$${PANEL_IMAGE:-ghcr.io/next-panel/jexactyl-br:latest}" .

# Falha alto quando o pin diverge entre config.toml e Dockerfile.
doctor:
	@set -eu; \
	python3 -c "import tomllib,sys; \
	u=tomllib.load(open('config.toml','rb'))['upstream']; \
	want={'URL':u['url'],'REF':u['ref'],'SHA':u['sha']}; \
	d={l.split('=',1)[0].strip():l.split('=',1)[1].strip() for l in open('docker/Dockerfile') if l.startswith('ARG UPSTREAM_') and '=' in l}; \
	bad=['Dockerfile:'+k for k,v in want.items() if d.get('ARG UPSTREAM_'+k)!=v]; \
	sys.exit('drift do pin: '+','.join(bad) if bad else print('Dockerfile em sincronia'))"; \
	command -v docker >/dev/null || { echo 'docker ausente' >&2; exit 1; }; \
	docker compose config --quiet && echo 'compose valido'

# Dump consistente do banco do compose (para migracao ou guarda).
# Credenciais extraidas do `docker compose config` (mesma interpolacao do up)
# e senha via MYSQL_PWD (nunca na linha de comando, invisivel ao `ps`).
backup-db:
	@set -eu; mkdir -p backups; \
	eval "$$(docker compose config --format json | python3 -c "import json,shlex,sys; e=json.load(sys.stdin)['services']['database']['environment']; print(';'.join('%s=%s' % (k, shlex.quote(e[m])) for k, m in (('DB_DATABASE','MYSQL_DATABASE'),('DB_USERNAME','MYSQL_USER'),('DB_PASSWORD','MYSQL_PASSWORD'))))")"; \
	stamp="$$(date +%Y%m%d-%H%M%S)"; \
	docker compose exec -T -e "MYSQL_PWD=$$DB_PASSWORD" database mariadb-dump -u"$$DB_USERNAME" "$$DB_DATABASE" > "backups/painel-$$stamp.sql"; \
	echo "backups/painel-$$stamp.sql"

clean:
	@rm -rf .upstream .br-cache dist
