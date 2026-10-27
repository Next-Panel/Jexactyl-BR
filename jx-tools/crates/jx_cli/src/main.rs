#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::unwrap_used, clippy::expect_used)]
//! Binario `jx`: ferramentas do overlay Jexactyl.

use std::collections::HashSet;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use jx_core::{
    config_get, config_set, resolve_path, BrError, Clock as _, ConfigKey, FileSystem as _, RealFs,
    SystemClock,
};
use jx_git::{GitBackend as _, ProcessGit, Sha40};
use jx_i18n::audit::{analyze_pair, AnalyzeOptions, NlpConfig};
use jx_i18n::catalog::{
    assign_units, collect_excluded, hash_lock, is_source_file, validate_manifest, SOURCE_ROOTS,
};
use jx_i18n::core::Finding as I18nFinding;
use jx_i18n::lang::{classify_label_language, GlossaryEntry, NlpOptions};
use jx_i18n::orchestrate::{run_audit, AuditOptions, AuditPaths};
use jx_i18n::php::extract_php_lang_entries;
use jx_i18n::scan::{extract_comments, skeleton};
use jx_i18n::store::{CompleteOptions, Store, SystemClock as I18nClock};
use jx_i18n::text::extract_placeholders;
use jx_ops::apply::{run_apply, ApplyPaths, ApplyReport, ApplyRequest};
use jx_ops::dump::{dump_new, dump_refresh, DumpPaths, NewPatchRequest, RefreshRequest};
use jx_ops::lint::{run_lint, LintPaths};
use jx_patch::{insert_into_text, parse_series, SeriesEntry};
use jx_workspace::package::{run_package_stage, write_tarball, PackagePaths, PackageRequest};
use jx_workspace::rebase::{run_rebase, RebasePaths, RebaseRequest};
use jx_workspace::setup::{run_setup, CmdOutcome, CmdRunner, SetupPaths, SetupRequest};
use serde_json::Value as JsonValue;

/// Executor real de ferramentas externas (`composer`, `pnpm`).
#[derive(Debug, Clone, Copy, Default)]
struct RealCmdRunner;

impl CmdRunner for RealCmdRunner {
    fn run_tool(&self, cwd: &std::path::Path, tool: &str, args: &[&str]) -> CmdOutcome {
        let output = match std::process::Command::new(tool)
            .current_dir(cwd)
            .args(args)
            .output()
        {
            Ok(output) => output,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return CmdOutcome::SkippedMissing
            }
            Err(_) => return CmdOutcome::FailedWarning(tool.to_string()),
        };
        if output.status.success() {
            CmdOutcome::Success
        } else {
            CmdOutcome::FailedWarning(tool.to_string())
        }
    }
}

/// Ferramentas do overlay Jexactyl (reescrita Rust dos scripts).
#[derive(Debug, Parser)]
#[command(name = "jx", version, about = "Ferramentas do overlay Jexactyl")]
struct Cli {
    /// Caminho do config.toml (padrao: config.toml no diretorio atual).
    #[arg(long, global = true, default_value = "config.toml")]
    config: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Le uma chave tabela.nome do config.toml.
    ConfigGet { key: String },
    /// Grava uma chave tabela.nome no config.toml.
    ConfigSet { key: String, value: String },
    /// Lista as entradas parseadas de um arquivo series.
    SeriesEntries {
        /// Caminho do arquivo series.
        #[arg(long)]
        series: PathBuf,
    },
    /// Insere uma entrada no series mantendo a ordem numerica.
    SeriesInsert {
        /// Caminho do arquivo series.
        #[arg(long)]
        series: PathBuf,
        /// Entrada a inserir (ex.: patches/0042-x.patch).
        entry: String,
    },
    /// Aplica a serie de patches e os overlays no workspace.
    Apply {
        /// Diretorio alvo (padrao: workspace.dir do config.toml).
        #[arg(long)]
        target: Option<PathBuf>,
        /// So valida se os patches aplicam, sem alterar nada.
        #[arg(long)]
        check: bool,
        /// Desfaz a serie inteira.
        #[arg(long)]
        reverse: bool,
        /// Ignora uma entrada do series.
        #[arg(long)]
        skip: Option<String>,
        /// Aplica sem criar os commits de snapshot.
        #[arg(long)]
        no_snapshot: bool,
    },
    /// Valida a serie de patches (DEP-3, ordem, aplicacao, reversibilidade).
    Lint {
        /// Mantem o clone temporario usado na validacao.
        #[arg(long)]
        keep_cache: bool,
    },
    /// Gera um patch novo a partir do delta pendente no workspace.
    Dump {
        /// Slug kebab-case do patch.
        slug: String,
        /// Dominio de dois digitos (padrao: 01).
        #[arg(long, default_value = "01")]
        domain: String,
        /// Assunto de uma linha (padrao: o slug).
        #[arg(long)]
        message: Option<String>,
        /// Descricao longa.
        #[arg(long)]
        description: Option<String>,
        /// Origem DEP-3 (padrao: vendor).
        #[arg(long, default_value = "vendor")]
        origin: String,
        /// Issue de acompanhamento (Bug-Next-Panel).
        #[arg(long)]
        bug: Option<String>,
        /// Linha Signed-off-by (padrao: autor do git).
        #[arg(long)]
        signoff: Option<String>,
        /// Applied-Upstream (padrao: no).
        #[arg(long, default_value = "no")]
        applied_upstream: String,
        /// Nao registra o patch no series.
        #[arg(long)]
        no_series: bool,
    },
    /// Regenera o diff de um patch da serie in-place preservando o cabecalho e reaplicando os seguintes.
    Refresh {
        /// Entrada do series a regenerar (ex.: patches/0004-i18n-shared-runtime.patch).
        entry: String,
        /// Permite refresh mesmo com o pipeline i18n ativo.
        #[arg(long)]
        force: bool,
    },
    /// Prepara o workspace: clone, pin, serie e dependencias opcionais.
    Setup {
        /// Instala dependencias PHP/JS no final.
        #[arg(long)]
        with_deps: bool,
        /// Recria o workspace do zero, descartando mudancas.
        #[arg(long)]
        force: bool,
        /// Usa outra ref do upstream nesta execucao.
        #[arg(long = "ref")]
        c_ref: Option<String>,
        /// Usa outro commit do upstream nesta execucao.
        #[arg(long)]
        sha: Option<String>,
        /// Usa um mirror local no lugar da URL.
        #[arg(long)]
        local: Option<PathBuf>,
    },
    /// Move a base fixada e reaplica a serie sobre ela.
    Rebase {
        /// Nova ref do upstream (tag ou branch).
        new_ref: Option<String>,
        /// Fixa a base em um commit especifico.
        #[arg(long)]
        sha: Option<String>,
        /// Nao grava a nova ref/sha no config.toml.
        #[arg(long)]
        no_pin: bool,
        /// So mostra o que seria feito.
        #[arg(long)]
        dry_run: bool,
        /// Descarta mudancas nao commitadas do workspace.
        #[arg(long)]
        force: bool,
    },
    /// Gera o tarball de release com a serie aplicada.
    Package {
        /// Versao do pacote (padrao: upstream.ref sem o `v`).
        #[arg(long)]
        version: Option<String>,
        /// Caminho do tarball gerado.
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Expande e valida o catalogo i18n (`i18n/catalog.json` -> `catalog.lock.json`).
    I18nCatalog {
        /// Grava o lock a partir do manifesto (falha com sobreposicao ou descobertos).
        #[arg(long)]
        write: bool,
        /// Compara o manifesto com o lock existente, sem gravar nada.
        #[arg(long)]
        check: bool,
        /// Imprime o resultado em JSON compacto.
        #[arg(long)]
        json: bool,
    },
    /// Fila do pipeline i18n sobre SQLite (porte dos comandos de `queue.sh`).
    I18nQueue {
        /// Caminho do banco SQLite da fila.
        #[arg(long)]
        db: Option<PathBuf>,
        #[command(subcommand)]
        command: I18nQueueCommand,
    },
    /// Audita a unidade/categoria (porte de `scripts/i18n/audit.sh`).
    I18nAudit {
        /// Unidade a auditar (exclusiva com `--category`).
        #[arg(long)]
        unit: Option<String>,
        /// Categoria a auditar (exclusiva com `--unit`).
        #[arg(long)]
        category: Option<String>,
        /// Imprime o resultado em JSON.
        #[arg(long)]
        json: bool,
        /// Revisao git de comparacao (ou `BR_I18N_BASE`).
        #[arg(long)]
        base: Option<String>,
        /// Aceito por compatibilidade; o gate sempre falha com qualquer achado.
        #[arg(long)]
        strict: bool,
    },
    /// Analisa um par de revisoes (ponte de `scripts/i18n/cli.py`).
    BridgeAnalyzePair,
    /// Esqueleto normalizado (ponte de `scripts/i18n/cli.py`).
    BridgeSkeleton,
    /// Comentarios extraidos (ponte de `scripts/i18n/cli.py`).
    BridgeExtractComments,
    /// Entradas de lang PHP (ponte de `scripts/i18n/cli.py`).
    BridgeExtractPhpEntries,
    /// Placeholders de um texto (ponte de `scripts/i18n/cli.py`).
    BridgePlaceholders,
    /// Sonda de idioma EN/PT (ponte de `scripts/i18n/cli.py`).
    BridgeNlpProbe,
}

/// Subcomandos da fila i18n (espelham `scripts/i18n/queue.sh`).
#[derive(Debug, Subcommand)]
enum I18nQueueCommand {
    /// Cria o banco a partir do lock e ativa a primeira categoria.
    Init {
        /// Recria o banco mesmo que o arquivo ja exista.
        #[arg(long)]
        force: bool,
    },
    /// Mostra a categoria ativa, as categorias e os claims.
    Status {
        /// Imprime o resumo em JSON.
        #[arg(long)]
        json: bool,
    },
    /// Lista unidades reivindicaveis para o papel.
    Next {
        /// Papel (`analyst`, `translator`, `reviewer`, `fixer`).
        #[arg(long)]
        role: String,
        /// Maximo de unidades (padrao: `max_agents` da politica).
        #[arg(long)]
        limit: Option<String>,
        /// Imprime em JSON.
        #[arg(long)]
        json: bool,
    },
    /// Reivindica uma unidade para o papel.
    Claim {
        /// Unidade a reivindicar.
        unit: String,
        /// Papel (`analyst`, `translator`, `reviewer`, `fixer`).
        #[arg(long)]
        role: String,
        /// Dono do claim.
        #[arg(long)]
        owner: String,
        /// Lease em minutos (padrao: `lease_minutes` da politica).
        #[arg(long)]
        lease_minutes: Option<String>,
    },
    /// Renova o lease de um claim ativo.
    Heartbeat {
        /// Unidade com claim ativo.
        unit: String,
        /// Dono do claim.
        #[arg(long)]
        owner: String,
        /// Novo lease em minutos (padrao: `lease_minutes` da politica).
        #[arg(long)]
        lease_minutes: Option<String>,
    },
    /// Libera um claim ativo sem concluir.
    Release {
        /// Unidade com claim ativo.
        unit: String,
        /// Dono do claim.
        #[arg(long)]
        owner: String,
        /// Motivo (aceito por compatibilidade; nao e persistido).
        #[arg(long)]
        reason: Option<String>,
    },
    /// Conclui o claim ativo do papel.
    Complete {
        /// Unidade com claim ativo.
        unit: String,
        /// Papel (`analyst`, `translator`, `reviewer`, `fixer`).
        #[arg(long)]
        role: String,
        /// Dono do claim.
        #[arg(long)]
        owner: String,
        /// Arquivo do relatorio JSON do estagio.
        #[arg(long)]
        report: Option<PathBuf>,
        /// Nota da revisao (obrigatoria para `reviewer`).
        #[arg(long)]
        score: Option<String>,
        /// Total de labels (somente `analyst`).
        #[arg(long)]
        labels_total: Option<String>,
        /// Labels traduzidas (somente `translator`).
        #[arg(long)]
        labels_translated: Option<String>,
        /// Nota livre do evento.
        #[arg(long)]
        note: Option<String>,
        /// Imprime o resultado em JSON compacto.
        #[arg(long)]
        json: bool,
    },
    /// Avanca a categoria para o proximo estagio.
    Advance {
        /// Categoria a avancar.
        category: String,
        /// Aceita gates pulados (`skip`).
        #[arg(long)]
        allow_unverified: bool,
    },
    /// Registra a nota de consistencia da categoria.
    RecordConsistency {
        /// Categoria em `consistency`.
        category: String,
        /// Nota de 0 a 10.
        #[arg(long)]
        score: String,
        /// Arquivo do relatorio JSON de consistencia.
        #[arg(long)]
        report: PathBuf,
        /// Nota livre (aceita por compatibilidade; nao e persistida).
        #[arg(long)]
        note: Option<String>,
    },
    /// Registra um gate de verificacao da categoria.
    RecordGate {
        /// Categoria em `verification`.
        category: String,
        /// Nome do gate.
        #[arg(long)]
        name: String,
        /// Resultado (`pass`, `fail` ou `skip`).
        #[arg(long)]
        status: String,
        /// Justificativa (obrigatoria com `skip`).
        #[arg(long)]
        note: Option<String>,
    },
    /// Registra um patch em lote da categoria.
    RecordPatch {
        /// Categoria dona do patch.
        category: String,
        /// Nome do arquivo de patch (termina em `.patch`).
        #[arg(long)]
        name: String,
        /// Unidade coberta (repetivel).
        #[arg(long)]
        unit: Vec<String>,
    },
    /// Registra um overlay sob `overlays/` da categoria.
    RecordOverlay {
        /// Categoria dona do overlay.
        category: String,
        /// Caminho relativo sob `overlays/`.
        #[arg(long)]
        path: String,
        /// Unidade coberta (repetivel).
        #[arg(long)]
        unit: Vec<String>,
    },
    /// Define as unidades cobertas por um patch.
    SetPatchUnits {
        /// Nome do patch registrado.
        patch: String,
        /// Unidade coberta (repetivel, ao menos uma).
        #[arg(long)]
        unit: Vec<String>,
    },
    /// Registra um override compartilhado da categoria.
    RecordSharedOverride {
        /// Categoria dona do override.
        category: String,
        /// Caminho relativo dentro do repositorio.
        #[arg(long)]
        path: String,
        /// Motivo do override.
        #[arg(long)]
        reason: String,
        /// Unidade coberta (repetivel).
        #[arg(long)]
        unit: Vec<String>,
    },
    /// Reverte claims expirados para a fila.
    Recover,
    /// Bloqueia uma unidade ou categoria.
    Block {
        /// Unidade ou categoria a bloquear.
        target: String,
        /// Motivo do bloqueio.
        #[arg(long)]
        reason: String,
    },
    /// Desbloqueia para um estagio anterior-ou-igual.
    Unblock {
        /// Unidade ou categoria bloqueada.
        target: String,
        /// Estagio destino.
        #[arg(long)]
        stage: String,
        /// Desbloqueia mesmo fora do estado esperado.
        #[arg(long)]
        force: bool,
    },
    /// Reabre categoria `done` ou `blocked`.
    Reopen {
        /// Categoria a reabrir.
        category: String,
        /// Motivo da reabertura.
        #[arg(long)]
        reason: String,
        /// Exigido para categoria `blocked`; pausa outra ativa.
        #[arg(long)]
        force: bool,
    },
    /// Valida os invariantes internos do banco.
    Validate,
    /// Mostra o resumo humano do estado (nao grava `CACHE.md`).
    Render,
}

/// Erro fatal de uso/ambiente dos comandos i18n.
///
/// Espelha `fail`/`UsageError` dos scripts Python: imprime `[prefixo]
/// mensagem` em stderr e sai com codigo 1, sem tocar em `CACHE.md`.
fn fail(prefix: &str, message: &str) -> ! {
    let _ = std::io::stdout().flush();
    eprintln!("[{prefix}] {message}");
    std::process::exit(1);
}

/// Desempacota `Result` ou falha com `[prefixo] erro`.
///
/// Centraliza o mapeamento de `StoreError` e erros de parse para o formato
/// legivel por maquina dos scripts Python.
fn or_fail<T>(result: Result<T, impl std::fmt::Display>, prefix: &str) -> T {
    match result {
        Ok(value) => value,
        Err(err) => fail(prefix, &err.to_string()),
    }
}

/// Le um override de caminho das variaveis de ambiente (`BR_I18N_*`).
fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// Seleciona a raiz i18n: override (`BR_I18N_ROOT`) ou o pai do `--config`.
fn select_i18n_root(config_path: &Path, override_root: Option<PathBuf>) -> PathBuf {
    override_root.unwrap_or_else(|| {
        config_path
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
    })
}

/// Resolve a raiz i18n em caminho absoluto (canonico quando possivel).
///
/// Espelha `resolve_paths` de queue.py, com o padrao no diretorio-pai do
/// `--config` em vez do diretorio do script.
fn resolve_i18n_root(config_path: &Path) -> PathBuf {
    let raw = select_i18n_root(config_path, env_path("BR_I18N_ROOT"));
    let absolute = if raw.is_absolute() {
        raw
    } else {
        match std::env::current_dir() {
            Ok(cwd) => cwd.join(raw),
            Err(_) => raw,
        }
    };
    absolute.canonicalize().unwrap_or(absolute)
}

/// Manifesto i18n: `BR_I18N_CATALOG` ou `<root>/i18n/catalog.json`.
fn manifest_path_for(root: &Path, override_catalog: Option<PathBuf>) -> PathBuf {
    override_catalog.unwrap_or_else(|| root.join("i18n/catalog.json"))
}

/// Lock do catalogo, sempre em `<root>/i18n/catalog.lock.json`.
fn lock_path_for(root: &Path) -> PathBuf {
    root.join("i18n/catalog.lock.json")
}

/// Banco da fila: `--db`, `BR_I18N_CACHE` ou `<root>/.br-cache/i18n-queue.sqlite`.
///
/// O padrao evita `CACHE.md` para nao colidir com o pipeline Python; a fila
/// Rust mora em `.br-cache/`.
fn db_path_for(root: &Path, override_db: Option<PathBuf>) -> PathBuf {
    override_db.unwrap_or_else(|| root.join(".br-cache/i18n-queue.sqlite"))
}

/// Espelha `parse_integer` de queue.py: inteiro decimal em ±(2^53-1).
fn parse_flag_int(raw: Option<&str>, name: &str) -> Result<Option<i64>, String> {
    let Some(text) = raw else {
        return Ok(None);
    };
    let digits = text.strip_prefix('-').unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!("{name} must be an integer"));
    }
    const MAX_SAFE: i128 = 9_007_199_254_740_991;
    let parsed: i128 = text
        .parse()
        .map_err(|_| format!("{name} must be an integer"))?;
    if !(-MAX_SAFE..=MAX_SAFE).contains(&parsed) {
        return Err(format!("{name} is out of range"));
    }
    Ok(Some(parsed as i64))
}

/// Valida relatorio de revisao/consistencia (regras de `validate_review_report`).
///
/// `score == 10` exige `findings` vazio (e vice-versa), a soma de `criteria`
/// precisa bater com `score`, `blocker`/`major` limitam a nota a 7 e
/// `criteria` tem exatamente as chaves `coverage`, `naturalness`,
/// `consistency`, `context` e `safety`.
fn check_review_report(report: &JsonValue, score: i64) -> Result<(), String> {
    let object = report
        .as_object()
        .ok_or_else(|| "review report must be a JSON object".to_string())?;
    let report_score = object
        .get("score")
        .and_then(JsonValue::as_i64)
        .ok_or_else(|| "review report must contain an integer score".to_string())?;
    if report_score != score {
        return Err(format!(
            "--score {score} does not match the report score {report_score}"
        ));
    }
    let findings = object
        .get("findings")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| "review report findings must be an array".to_string())?;
    if score == 10 && !findings.is_empty() {
        return Err("score 10 requires an empty findings array".to_string());
    }
    if score < 10 && findings.is_empty() {
        return Err("a score below 10 requires at least one finding".to_string());
    }
    let mut blocking = false;
    for finding in findings {
        let record = finding
            .as_object()
            .ok_or_else(|| "each finding must be an object".to_string())?;
        for name in ["id", "severity", "file", "problem", "fix"] {
            let valid = record
                .get(name)
                .and_then(JsonValue::as_str)
                .is_some_and(|text| !text.trim().is_empty());
            if !valid {
                return Err(format!("finding {name} must be a non-empty string"));
            }
        }
        let line = record.get("line").and_then(JsonValue::as_i64);
        if line.is_none_or(|number| number <= 0) {
            return Err("finding line must be a positive integer".to_string());
        }
        let severity = record
            .get("severity")
            .and_then(JsonValue::as_str)
            .unwrap_or("");
        if !["blocker", "major", "minor", "nit"].contains(&severity) {
            return Err(format!("invalid finding severity: {severity}"));
        }
        if severity == "blocker" || severity == "major" {
            blocking = true;
        }
    }
    if blocking && score > 7 {
        return Err("blocker/major findings require a score of at most 7".to_string());
    }
    let criteria = object
        .get("criteria")
        .and_then(JsonValue::as_object)
        .ok_or_else(|| "review report must contain criteria".to_string())?;
    let keys = [
        "coverage",
        "naturalness",
        "consistency",
        "context",
        "safety",
    ];
    {
        let mut got: Vec<&String> = criteria.keys().collect();
        got.sort();
        let mut want: Vec<String> = keys.iter().map(|key| key.to_string()).collect();
        want.sort();
        let got_owned: Vec<String> = got.into_iter().cloned().collect();
        if got_owned != want {
            return Err(
                "criteria must contain exactly coverage, naturalness, consistency, context and safety"
                    .to_string(),
            );
        }
    }
    let mut total = 0;
    for name in keys {
        match criteria.get(name).and_then(JsonValue::as_i64) {
            Some(value) if (0..=2).contains(&value) => total += value,
            _ => return Err(format!("criteria.{name} must be 0, 1 or 2")),
        }
    }
    if total != score {
        return Err(format!("criteria sum {total} does not match score {score}"));
    }
    for name in ["untranslated", "glossary_violations", "test_impact"] {
        if object.get(name).and_then(JsonValue::as_array).is_none() {
            return Err(format!("review report {name} must be an array"));
        }
    }
    if score == 10 {
        let untranslated_empty = object
            .get("untranslated")
            .and_then(JsonValue::as_array)
            .is_some_and(Vec::is_empty);
        if !untranslated_empty {
            return Err("score 10 requires untranslated to be empty".to_string());
        }
        let glossary_empty = object
            .get("glossary_violations")
            .and_then(JsonValue::as_array)
            .is_some_and(Vec::is_empty);
        if !glossary_empty {
            return Err("score 10 requires glossary_violations to be empty".to_string());
        }
    }
    Ok(())
}

/// Titulo de unidade/categoria do snapshot para texto humano.
fn display_title(value: &JsonValue) -> String {
    if let Some(text) = value.as_str() {
        return text.to_string();
    }
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
}

/// Arquivos da unidade no lock (espelha `catalog_unit_files` de queue.py).
fn catalog_unit_files(catalog: &JsonValue, unit_id: &str) -> Vec<String> {
    if let Some(categories) = catalog.get("categories").and_then(JsonValue::as_array) {
        for category in categories {
            if let Some(units) = category.get("units").and_then(JsonValue::as_array) {
                for unit in units {
                    if unit.get("id").and_then(JsonValue::as_str) == Some(unit_id) {
                        let mut files = Vec::new();
                        if let Some(list) = unit.get("files").and_then(JsonValue::as_array) {
                            for item in list {
                                if let Some(path) = item.as_str() {
                                    files.push(path.to_string());
                                }
                            }
                        }
                        return files;
                    }
                }
            }
        }
    }
    Vec::new()
}

/// `lease_minutes` da politica do snapshot (padrao 120).
fn policy_lease_minutes(snapshot: &JsonValue) -> i64 {
    snapshot
        .get("policy")
        .and_then(|policy| policy.get("lease_minutes"))
        .and_then(JsonValue::as_i64)
        .unwrap_or(120)
}

/// `max_agents` da politica do snapshot (padrao 3).
fn policy_max_agents(snapshot: &JsonValue) -> i64 {
    snapshot
        .get("policy")
        .and_then(|policy| policy.get("max_agents"))
        .and_then(JsonValue::as_i64)
        .unwrap_or(3)
}

/// Unidades com claim presente, na ordem do snapshot.
fn claimed_units(snapshot: &JsonValue) -> Vec<String> {
    let mut ids = Vec::new();
    if let Some(units) = snapshot.get("units").and_then(JsonValue::as_object) {
        for (uid, unit) in units {
            if unit.get("claim").is_some_and(|claim| !claim.is_null()) {
                ids.push(uid.clone());
            }
        }
    }
    ids
}

/// Conta unidades com claim presente no snapshot (pos-`recover`, equivale a ativas).
fn active_claim_count(snapshot: &JsonValue) -> usize {
    claimed_units(snapshot).len()
}

/// Expiracao do claim da unidade (`unknown` quando ausente).
fn claim_expires_at(snapshot: &JsonValue, unit_id: &str) -> String {
    snapshot
        .get("units")
        .and_then(JsonValue::as_object)
        .and_then(|units| units.get(unit_id))
        .and_then(|unit| unit.get("claim"))
        .and_then(|claim| claim.get("expires_at"))
        .and_then(JsonValue::as_str)
        .unwrap_or("unknown")
        .to_string()
}

/// Estagio da categoria no snapshot (`unknown` quando ausente).
fn category_stage_of(snapshot: &JsonValue, category_id: &str) -> String {
    snapshot
        .get("categories")
        .and_then(JsonValue::as_object)
        .and_then(|categories| categories.get(category_id))
        .and_then(|category| category.get("stage"))
        .and_then(JsonValue::as_str)
        .unwrap_or("unknown")
        .to_string()
}

/// Tabela humana do status (categoria ativa + uma linha por categoria).
fn render_status_human(snapshot: &JsonValue) -> String {
    let active = snapshot
        .get("active_category")
        .and_then(JsonValue::as_str)
        .unwrap_or("none");
    let mut lines = vec![format!("active category: {active}")];
    let units = snapshot.get("units").and_then(JsonValue::as_object);
    if let Some(categories) = snapshot.get("categories").and_then(JsonValue::as_object) {
        let mut entries: Vec<(&String, &JsonValue)> = categories.iter().collect();
        entries.sort_by_key(|(_, category)| {
            category
                .get("order")
                .and_then(JsonValue::as_i64)
                .unwrap_or(0)
        });
        for (_, category) in entries {
            let id = category
                .get("id")
                .and_then(JsonValue::as_str)
                .unwrap_or("?");
            let order = category
                .get("order")
                .and_then(JsonValue::as_i64)
                .unwrap_or(0);
            let stage = category
                .get("stage")
                .and_then(JsonValue::as_str)
                .unwrap_or("?");
            let mut done = 0;
            let mut total = 0;
            if let Some(ids) = category.get("units").and_then(JsonValue::as_array) {
                for item in ids {
                    if let Some(uid) = item.as_str() {
                        total += 1;
                        let is_done = units
                            .and_then(|map| map.get(uid))
                            .and_then(|unit| unit.get("status"))
                            .and_then(JsonValue::as_str)
                            == Some("done");
                        if is_done {
                            done += 1;
                        }
                    }
                }
            }
            let patches = category
                .get("patches")
                .and_then(JsonValue::as_array)
                .map_or(0, Vec::len);
            lines.push(format!(
                "  {order:02} {id:<24} {stage:<12} {done}/{total} patch={patches}"
            ));
        }
    }
    lines.join("\n")
}

/// Resumo de maquina do status (mesmas chaves de `print_status --json`).
fn status_machine_json(snapshot: &JsonValue) -> JsonValue {
    let categories = snapshot.get("categories").and_then(JsonValue::as_object);
    let units = snapshot.get("units").and_then(JsonValue::as_object);
    let active_id = snapshot.get("active_category").and_then(JsonValue::as_str);
    let active = active_id.and_then(|id| categories.and_then(|map| map.get(id)));
    let mut active_units = Vec::new();
    if let Some(category) = active {
        if let Some(ids) = category.get("units").and_then(JsonValue::as_array) {
            for item in ids {
                let Some(uid) = item.as_str() else {
                    continue;
                };
                let Some(unit) = units.and_then(|map| map.get(uid)) else {
                    continue;
                };
                let get = |key: &str| unit.get(key).cloned().unwrap_or(JsonValue::Null);
                active_units.push(serde_json::json!({
                    "id": get("id"),
                    "stage": get("stage"),
                    "status": get("status"),
                    "score": get("score"),
                    "attempts": get("attempts"),
                    "claim": get("claim"),
                }));
            }
        }
    }
    let mut category_rows = Vec::new();
    if let Some(map) = categories {
        let mut entries: Vec<(&String, &JsonValue)> = map.iter().collect();
        entries.sort_by_key(|(_, category)| {
            category
                .get("order")
                .and_then(JsonValue::as_i64)
                .unwrap_or(0)
        });
        for (_, category) in entries {
            category_rows.push(serde_json::json!({
                "id": category.get("id").cloned().unwrap_or(JsonValue::Null),
                "stage": category.get("stage").cloned().unwrap_or(JsonValue::Null),
                "status": category.get("status").cloned().unwrap_or(JsonValue::Null),
                "consistency_score": category.get("consistency_score").cloned().unwrap_or(JsonValue::Null),
                "patches": category.get("patches").and_then(JsonValue::as_array).map_or(0, Vec::len),
            }));
        }
    }
    serde_json::json!({
        "active_category": snapshot.get("active_category").cloned().unwrap_or(JsonValue::Null),
        "active_stage": active.and_then(|category| category.get("stage")).cloned().unwrap_or(JsonValue::Null),
        "active_units": active_units,
        "categories": category_rows,
        "claims": claimed_units(snapshot),
        "patches": snapshot.get("patches").and_then(JsonValue::as_object).map_or(0, serde_json::Map::len),
        "overlays": snapshot.get("overlays").and_then(JsonValue::as_object).map_or(0, serde_json::Map::len),
    })
}

/// Linhas humanas do `next`: `{id}\t{title}\t{file_count} file(s)`.
fn render_next_human(snapshot: &JsonValue, ids: &[String]) -> String {
    let units = snapshot.get("units").and_then(JsonValue::as_object);
    ids.iter()
        .map(|id| {
            let unit = units.and_then(|map| map.get(id));
            let title = unit
                .and_then(|item| item.get("title"))
                .map_or_else(String::new, display_title);
            let file_count = unit
                .and_then(|item| item.get("file_count"))
                .and_then(JsonValue::as_i64)
                .unwrap_or(0);
            format!("{id}\t{title}\t{file_count} file(s)")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Entrada `--json` do `next` (mesmas chaves de queue.py `command_next`).
fn render_next_json(snapshot: &JsonValue, catalog: &JsonValue, ids: &[String]) -> JsonValue {
    let units = snapshot.get("units").and_then(JsonValue::as_object);
    let rows: Vec<JsonValue> = ids
        .iter()
        .map(|id| {
            let unit = units.and_then(|map| map.get(id));
            let get = |key: &str| {
                unit.and_then(|item| item.get(key))
                    .cloned()
                    .unwrap_or(JsonValue::Null)
            };
            serde_json::json!({
                "id": id,
                "title": get("title"),
                "category": get("category"),
                "stage": get("stage"),
                "status": get("status"),
                "files": catalog_unit_files(catalog, id),
                "analysis": get("analysis"),
                "report": get("findings"),
            })
        })
        .collect();
    JsonValue::Array(rows)
}

/// Le um relatorio JSON de estagio (espelha `read_report` de queue.py).
fn read_report_file(report: Option<&PathBuf>, kind: &str) -> JsonValue {
    let Some(path) = report else {
        fail("i18n:queue", &format!("--report is required for {kind}"));
    };
    if !path.is_file() {
        if !path.exists() {
            fail(
                "i18n:queue",
                &format!("report not found: {}", path.display()),
            );
        }
        fail(
            "i18n:queue",
            &format!("report is not a file: {}", path.display()),
        );
    }
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) => fail(
            "i18n:queue",
            &format!("report not found: {} ({err})", path.display()),
        ),
    };
    if text.trim().is_empty() {
        fail(
            "i18n:queue",
            &format!("report is empty: {}", path.display()),
        );
    }
    let value: JsonValue = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(err) => fail(
            "i18n:queue",
            &format!("report is not valid JSON: {} ({err})", path.display()),
        ),
    };
    if !value.is_object() {
        fail(
            "i18n:queue",
            &format!("report must be a JSON object: {}", path.display()),
        );
    }
    value
}

/// Le o lock do catalogo (espelha `read_catalog` de queue.py).
fn read_catalog_lock(lock_path: &Path) -> JsonValue {
    if !lock_path.exists() {
        fail(
            "i18n:queue",
            &format!(
                "missing catalog lock: {} (run `jx i18n-catalog --write`)",
                lock_path.display()
            ),
        );
    }
    let text = match std::fs::read_to_string(lock_path) {
        Ok(text) => text,
        Err(err) => fail(
            "i18n:queue",
            &format!("missing catalog lock: {} ({err})", lock_path.display()),
        ),
    };
    match serde_json::from_str::<JsonValue>(&text) {
        Ok(value) if value.is_object() => value,
        Ok(_) => fail(
            "i18n:queue",
            &format!(
                "catalog lock must be a JSON object: {}",
                lock_path.display()
            ),
        ),
        Err(err) => fail(
            "i18n:queue",
            &format!(
                "catalog lock is not valid JSON: {} ({err})",
                lock_path.display()
            ),
        ),
    }
}

/// Abre (ou cria) o banco SQLite da fila, criando o diretorio-pai.
fn open_queue_store(db_path: &Path) -> Store<I18nClock> {
    if let Some(parent) = db_path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(err) = std::fs::create_dir_all(parent) {
                fail(
                    "i18n:queue",
                    &format!("cannot create dir {} ({err})", parent.display()),
                );
            }
        }
    }
    match Store::open(db_path, I18nClock::new()) {
        Ok(store) => store,
        Err(err) => fail(
            "i18n:queue",
            &format!("cannot open store {} ({err})", db_path.display()),
        ),
    }
}

/// Instantaneo do banco ou falha com `[i18n:queue]`.
fn snapshot_or_fail(store: &Store<I18nClock>) -> JsonValue {
    match store.snapshot() {
        Ok(snapshot) => snapshot,
        Err(err) => fail("i18n:queue", &format!("cannot read state ({err})")),
    }
}

/// Executa `jx i18n-catalog` (espelha `scripts/i18n/catalog.sh --write/--check`).
fn run_i18n_catalog(config_path: &Path, write: bool, check: bool, json: bool) {
    if !write && !check {
        fail("i18n:catalog", "use --write or --check");
    }
    let root = resolve_i18n_root(config_path);
    let manifest_path = manifest_path_for(&root, env_path("BR_I18N_CATALOG"));
    let lock_path = lock_path_for(&root);
    if !config_path.exists() {
        fail(
            "i18n:catalog",
            &format!("missing config file: {}", config_path.display()),
        );
    }
    let fs = RealFs;
    let workspace_key = match ConfigKey::parse("workspace.dir") {
        Ok(key) => key,
        Err(err) => fail("i18n:catalog", &err.to_string()),
    };
    let workspace_dir = match config_get(&fs, config_path, &workspace_key) {
        Ok(value) if !value.is_empty() => value,
        Ok(_) => ".upstream".to_string(),
        Err(BrError::MissingKey(_)) => ".upstream".to_string(),
        Err(err) => fail("i18n:catalog", &err.to_string()),
    };
    let sha_key = match ConfigKey::parse("upstream.sha") {
        Ok(key) => key,
        Err(err) => fail("i18n:catalog", &err.to_string()),
    };
    let upstream_sha = match config_get(&fs, config_path, &sha_key) {
        Ok(value) => value,
        Err(BrError::MissingKey(_)) => String::new(),
        Err(err) => fail("i18n:catalog", &err.to_string()),
    };
    let upstream_root = root.join(&workspace_dir);
    if !upstream_root.join("resources/scripts").exists() {
        fail(
            "i18n:catalog",
            &format!(
                "upstream checkout not found; tried {} (run scripts/dev-setup.sh first)",
                upstream_root.display()
            ),
        );
    }
    let manifest_text = match std::fs::read_to_string(&manifest_path) {
        Ok(text) => text,
        Err(err) => fail(
            "i18n:catalog",
            &format!("missing manifest file: {} ({err})", manifest_path.display()),
        ),
    };
    let manifest: JsonValue = match serde_json::from_str(&manifest_text) {
        Ok(value) => value,
        Err(err) => fail(
            "i18n:catalog",
            &format!("invalid manifest file: {} ({err})", manifest_path.display()),
        ),
    };
    if !manifest.is_object() {
        fail(
            "i18n:catalog",
            &format!(
                "invalid manifest file: {} (expected a JSON object)",
                manifest_path.display()
            ),
        );
    }
    let errors = validate_manifest(&manifest);
    if !errors.is_empty() {
        for error in &errors {
            eprintln!("[i18n:catalog] {error}");
        }
        std::process::exit(1);
    }
    let mut all_files: Vec<String> = Vec::new();
    for source_root in SOURCE_ROOTS {
        let absolute = upstream_root.join(source_root);
        let entries = match fs.list_files_recursive(&absolute) {
            Ok(entries) => entries,
            Err(err) => fail("i18n:catalog", &err.to_string()),
        };
        for relative in entries {
            let relative_path = format!(
                "{source_root}/{}",
                relative.to_string_lossy().replace('\\', "/")
            );
            if is_source_file(&relative_path) {
                all_files.push(relative_path);
            }
        }
    }
    all_files.sort();
    let raw_categories: Vec<JsonValue> =
        match manifest.get("categories").and_then(JsonValue::as_array) {
            Some(items) => items.clone(),
            None => fail("i18n:catalog", "manifest has no categories"),
        };
    let (built_categories, assigned, overlaps) = match assign_units(&raw_categories, &all_files) {
        Ok(outcome) => outcome,
        Err(err) => fail("i18n:catalog", &err.to_string()),
    };
    let (excluded, excluded_files) = match collect_excluded(&manifest, &all_files, &assigned) {
        Ok(outcome) => outcome,
        Err(err) => fail("i18n:catalog", &err.to_string()),
    };
    let excluded_set: HashSet<&String> = excluded_files.iter().collect();
    let uncovered: Vec<String> = all_files
        .iter()
        .filter(|path| !assigned.contains_key(path.as_str()) && !excluded_set.contains(*path))
        .cloned()
        .collect();
    for overlap in &overlaps {
        eprintln!(
            "[i18n:catalog] overlap: {} is claimed by {} and {}",
            overlap.relative_path, overlap.existing_unit, overlap.claiming_unit
        );
    }
    for path in &uncovered {
        eprintln!("[i18n:catalog] uncovered file: {path}");
    }
    if !overlaps.is_empty() || !uncovered.is_empty() {
        std::process::exit(1);
    }
    let category_count = built_categories.len();
    let units_total: usize = built_categories.iter().map(|c| c.units.len()).sum();
    let files_assigned = assigned.len();
    let files_excluded = excluded_files.len();
    let stats = serde_json::json!({
        "categories": category_count,
        "units": units_total,
        "files_total": all_files.len(),
        "files_assigned": files_assigned,
        "files_excluded": files_excluded,
        "files_uncovered": uncovered.len(),
        "overlaps": overlaps.len(),
    });
    let categories_value = match serde_json::to_value(&built_categories) {
        Ok(value) => value,
        Err(err) => fail("i18n:catalog", &err.to_string()),
    };
    let excluded_value = match serde_json::to_value(&excluded) {
        Ok(value) => value,
        Err(err) => fail("i18n:catalog", &err.to_string()),
    };
    let git = ProcessGit::new();
    let source_head: JsonValue = match git.rev_parse(&upstream_root, "HEAD") {
        Ok(sha) => JsonValue::String(sha.hex()),
        Err(_) => JsonValue::Null,
    };
    let mut lock_map = serde_json::Map::new();
    lock_map.insert("version".to_string(), serde_json::json!(1));
    lock_map.insert("upstream_sha".to_string(), JsonValue::String(upstream_sha));
    lock_map.insert("source_head".to_string(), source_head);
    lock_map.insert(
        "generated_at".to_string(),
        JsonValue::String(SystemClock.now_iso()),
    );
    if let Some(policy) = manifest.get("policy") {
        lock_map.insert("policy".to_string(), policy.clone());
    }
    lock_map.insert("stats".to_string(), stats.clone());
    lock_map.insert("categories".to_string(), categories_value);
    lock_map.insert("excluded".to_string(), excluded_value);
    let lock = JsonValue::Object(lock_map);
    if check {
        if !lock_path.exists() {
            fail(
                "i18n:catalog",
                "missing i18n/catalog.lock.json; run `jx i18n-catalog --write`",
            );
        }
        let current_text = match std::fs::read_to_string(&lock_path) {
            Ok(text) => text,
            Err(err) => fail(
                "i18n:catalog",
                &format!("missing i18n/catalog.lock.json ({err}); run `jx i18n-catalog --write`"),
            ),
        };
        let current: JsonValue = match serde_json::from_str(&current_text) {
            Ok(value) => value,
            Err(err) => fail(
                "i18n:catalog",
                &format!("invalid i18n/catalog.lock.json ({err}); run `jx i18n-catalog --write`"),
            ),
        };
        if !current.is_object() {
            fail(
                "i18n:catalog",
                "invalid i18n/catalog.lock.json; run `jx i18n-catalog --write`",
            );
        }
        if hash_lock(&current) != hash_lock(&lock) {
            fail(
                "i18n:catalog",
                "catalog.lock.json is stale; run `jx i18n-catalog --write`",
            );
        }
        if json {
            println!(
                "{}",
                serde_json::to_string(&serde_json::json!({
                    "ok": true,
                    "stats": stats,
                    "stale": false,
                }))
                .unwrap_or_else(|_| "{\"ok\":true}".to_string())
            );
        } else {
            println!(
                "[i18n:catalog] ok: {category_count} categories, {units_total} units, {files_assigned} files assigned, {files_excluded} excluded"
            );
        }
        return;
    }
    if let Some(parent) = lock_path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(err) = std::fs::create_dir_all(parent) {
                fail(
                    "i18n:catalog",
                    &format!("cannot write {} ({err})", lock_path.display()),
                );
            }
        }
    }
    let pretty = format!(
        "{}\n",
        serde_json::to_string_pretty(&lock).unwrap_or_else(|_| "{}".to_string())
    );
    if let Err(err) = std::fs::write(&lock_path, pretty) {
        fail(
            "i18n:catalog",
            &format!("cannot write {} ({err})", lock_path.display()),
        );
    }
    let categories_dir = root.join("i18n/categories");
    if let Err(err) = std::fs::create_dir_all(&categories_dir) {
        fail(
            "i18n:catalog",
            &format!("cannot create {} ({err})", categories_dir.display()),
        );
    }
    for category in &built_categories {
        let Some(cat_id) = category.id.as_str() else {
            continue;
        };
        let profile_path = categories_dir.join(format!("{cat_id}.md"));
        let profile_text = jx_i18n::catalog::render_profile(category);
        if let Err(err) = std::fs::write(&profile_path, profile_text) {
            fail(
                "i18n:catalog",
                &format!("cannot write {} ({err})", profile_path.display()),
            );
        }
    }
    let relative = lock_path
        .strip_prefix(&root)
        .map_or_else(|_| lock_path.clone(), Path::to_path_buf);
    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": true,
                "stats": stats,
                "scan_hits": 0,
            }))
            .unwrap_or_else(|_| "{\"ok\":true}".to_string())
        );
    } else {
        println!(
            "[i18n:catalog] wrote {}: {category_count} categories, {units_total} units, {files_assigned} files assigned, {files_excluded} excluded",
            relative.display()
        );
    }
}

/// Executa `jx i18n-queue <cmd>` (porte dos comandos de `queue.sh`).
fn run_i18n_queue(config_path: &Path, db_override: Option<PathBuf>, command: &I18nQueueCommand) {
    let root = resolve_i18n_root(config_path);
    let db_path = db_path_for(&root, db_override.or_else(|| env_path("BR_I18N_CACHE")));
    match command {
        I18nQueueCommand::Init { force } => {
            if db_path.exists() && !force {
                fail(
                    "i18n:queue",
                    &format!(
                        "{} already exists (use --force to recreate)",
                        db_path.display()
                    ),
                );
            }
            if *force && db_path.exists() {
                or_fail(
                    std::fs::remove_file(&db_path)
                        .map_err(|err| format!("cannot remove {} ({err})", db_path.display())),
                    "i18n:queue",
                );
            }
            let lock = read_catalog_lock(&lock_path_for(&root));
            let mut store = open_queue_store(&db_path);
            or_fail(store.init_from_catalog(&lock), "i18n:queue");
            println!("[i18n:queue] initialized {}", db_path.display());
            let snapshot = snapshot_or_fail(&store);
            println!("{}", render_status_human(&snapshot));
        }
        I18nQueueCommand::Status { json } => {
            let mut store = open_queue_store(&db_path);
            let _ = or_fail(store.recover_expired(), "i18n:queue");
            let snapshot = snapshot_or_fail(&store);
            if *json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&status_machine_json(&snapshot))
                        .unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                println!("{}", render_status_human(&snapshot));
            }
        }
        I18nQueueCommand::Next { role, limit, json } => {
            let limit_value = or_fail(parse_flag_int(limit.as_deref(), "--limit"), "i18n:queue");
            let mut store = open_queue_store(&db_path);
            let _ = or_fail(store.recover_expired(), "i18n:queue");
            let snapshot = snapshot_or_fail(&store);
            let mut requested = policy_max_agents(&snapshot);
            if let Some(number) = limit_value {
                requested = number;
            }
            if requested <= 0 {
                fail("i18n:queue", "--limit must be positive");
            }
            let ids = or_fail(store.next(role, requested as usize), "i18n:queue");
            if *json {
                let lock = read_catalog_lock(&lock_path_for(&root));
                println!(
                    "{}",
                    serde_json::to_string_pretty(&render_next_json(&snapshot, &lock, &ids))
                        .unwrap_or_else(|_| "[]".to_string())
                );
            } else if ids.is_empty() {
                let live = active_claim_count(&snapshot) as i64;
                let max = policy_max_agents(&snapshot);
                if live >= max {
                    println!("[i18n:queue] concurrency limit reached ({max}/{max} active claims)");
                } else {
                    println!("[i18n:queue] no claimable units for role {role}");
                }
            } else {
                println!("{}", render_next_human(&snapshot, &ids));
            }
        }
        I18nQueueCommand::Claim {
            unit,
            role,
            owner,
            lease_minutes,
        } => {
            let lease_value = or_fail(
                parse_flag_int(lease_minutes.as_deref(), "--lease-minutes"),
                "i18n:queue",
            );
            let mut store = open_queue_store(&db_path);
            let snapshot = snapshot_or_fail(&store);
            let lease = lease_value.unwrap_or_else(|| policy_lease_minutes(&snapshot));
            if lease <= 0 {
                fail("i18n:queue", "lease minutes must be a positive integer");
            }
            or_fail(store.claim(unit, role, owner, lease), "i18n:queue");
            let snapshot = snapshot_or_fail(&store);
            let expires = claim_expires_at(&snapshot, unit);
            println!("[i18n:queue] claimed {unit} as {role} by {owner} until {expires}");
        }
        I18nQueueCommand::Heartbeat {
            unit,
            owner,
            lease_minutes,
        } => {
            let lease_value = or_fail(
                parse_flag_int(lease_minutes.as_deref(), "--lease-minutes"),
                "i18n:queue",
            );
            let mut store = open_queue_store(&db_path);
            let snapshot = snapshot_or_fail(&store);
            let lease = lease_value.unwrap_or_else(|| policy_lease_minutes(&snapshot));
            or_fail(store.heartbeat(unit, owner, lease), "i18n:queue");
            let snapshot = snapshot_or_fail(&store);
            let expires = claim_expires_at(&snapshot, unit);
            println!("[i18n:queue] heartbeat {unit} until {expires}");
        }
        I18nQueueCommand::Release {
            unit,
            owner,
            reason,
        } => {
            let _ = reason;
            let mut store = open_queue_store(&db_path);
            or_fail(store.release(unit, owner), "i18n:queue");
            println!("[i18n:queue] released {unit}");
        }
        I18nQueueCommand::Complete {
            unit,
            role,
            owner,
            report,
            score,
            labels_total,
            labels_translated,
            note,
            json,
        } => {
            let kind = match role.as_str() {
                "analyst" => "analysis",
                "translator" => "translation",
                "reviewer" => "review",
                "fixer" => "correction",
                _ => "review",
            };
            let report_value = read_report_file(report.as_ref(), kind);
            let score_value = or_fail(parse_flag_int(score.as_deref(), "--score"), "i18n:queue");
            if role == "reviewer" {
                if let Some(score_number) = score_value {
                    or_fail(
                        check_review_report(&report_value, score_number),
                        "i18n:queue",
                    );
                }
            }
            let options = CompleteOptions {
                report: Some(report_value),
                score: score_value,
                labels_total: or_fail(
                    parse_flag_int(labels_total.as_deref(), "--labels-total"),
                    "i18n:queue",
                ),
                labels_translated: or_fail(
                    parse_flag_int(labels_translated.as_deref(), "--labels-translated"),
                    "i18n:queue",
                ),
                note: note.clone(),
            };
            let mut store = open_queue_store(&db_path);
            or_fail(store.complete(unit, role, owner, options), "i18n:queue");
            if *json {
                println!(
                    "{}",
                    serde_json::to_string(&serde_json::json!({
                        "ok": true,
                        "unit": unit,
                        "role": role,
                    }))
                    .unwrap_or_else(|_| "{\"ok\":true}".to_string())
                );
            } else {
                println!("[i18n:queue] completed {role} for {unit}");
            }
        }
        I18nQueueCommand::Advance {
            category,
            allow_unverified,
        } => {
            let mut store = open_queue_store(&db_path);
            or_fail(store.advance(category, *allow_unverified), "i18n:queue");
            let snapshot = snapshot_or_fail(&store);
            let stage = category_stage_of(&snapshot, category);
            println!("[i18n:queue] advanced {category} to {stage}");
        }
        I18nQueueCommand::RecordConsistency {
            category,
            score,
            report,
            note,
        } => {
            let _ = note;
            let score_value = match or_fail(parse_flag_int(Some(score), "--score"), "i18n:queue") {
                Some(number) => number,
                None => fail("i18n:queue", "--score is required"),
            };
            let report_value = read_report_file(Some(report), "review");
            or_fail(
                check_review_report(&report_value, score_value),
                "i18n:queue",
            );
            let mut store = open_queue_store(&db_path);
            or_fail(
                store.record_consistency(category, score_value, &report_value),
                "i18n:queue",
            );
            println!("[i18n:queue] consistency {category}: {score_value}/10");
        }
        I18nQueueCommand::RecordGate {
            category,
            name,
            status,
            note,
        } => {
            let mut store = open_queue_store(&db_path);
            or_fail(
                store.record_gate(category, name, status, note.as_deref().unwrap_or("")),
                "i18n:queue",
            );
            println!("[i18n:queue] gate {name}={status} for {category}");
        }
        I18nQueueCommand::RecordPatch {
            category,
            name,
            unit,
        } => {
            let unit_refs: Vec<&str> = unit.iter().map(String::as_str).collect();
            let mut store = open_queue_store(&db_path);
            or_fail(store.record_patch(category, name, &unit_refs), "i18n:queue");
            println!("[i18n:queue] recorded patch {name} for {category}");
        }
        I18nQueueCommand::RecordOverlay {
            category,
            path,
            unit,
        } => {
            let unit_refs: Vec<&str> = unit.iter().map(String::as_str).collect();
            let mut store = open_queue_store(&db_path);
            or_fail(
                store.record_overlay(category, path, &unit_refs),
                "i18n:queue",
            );
            println!("[i18n:queue] recorded overlay {path} for {category}");
        }
        I18nQueueCommand::SetPatchUnits { patch, unit } => {
            let unit_refs: Vec<&str> = unit.iter().map(String::as_str).collect();
            let mut store = open_queue_store(&db_path);
            or_fail(store.set_patch_units(patch, &unit_refs), "i18n:queue");
            println!("[i18n:queue] set {} unit(s) for {patch}", unit_refs.len());
        }
        I18nQueueCommand::RecordSharedOverride {
            category,
            path,
            reason,
            unit,
        } => {
            let unit_refs: Vec<&str> = unit.iter().map(String::as_str).collect();
            let mut store = open_queue_store(&db_path);
            or_fail(
                store.record_shared_override(category, path, reason, &unit_refs),
                "i18n:queue",
            );
            println!("[i18n:queue] recorded shared override {path} for {category}");
        }
        I18nQueueCommand::Recover => {
            let mut store = open_queue_store(&db_path);
            let recovered = or_fail(store.recover_expired(), "i18n:queue");
            println!("[i18n:queue] recovered {recovered} expired lease(s)");
        }
        I18nQueueCommand::Block { target, reason } => {
            let mut store = open_queue_store(&db_path);
            or_fail(store.block(target, reason), "i18n:queue");
            println!("[i18n:queue] blocked {target}");
        }
        I18nQueueCommand::Unblock {
            target,
            stage,
            force,
        } => {
            let mut store = open_queue_store(&db_path);
            or_fail(store.unblock(target, stage, *force), "i18n:queue");
            println!("[i18n:queue] unblocked {target}");
        }
        I18nQueueCommand::Reopen {
            category,
            reason,
            force,
        } => {
            let mut store = open_queue_store(&db_path);
            or_fail(store.reopen(category, reason, *force), "i18n:queue");
            println!("[i18n:queue] reopened {category}");
        }
        I18nQueueCommand::Validate => {
            let store = open_queue_store(&db_path);
            let errors = or_fail(store.validate(), "i18n:queue");
            if errors.is_empty() {
                println!("[i18n:queue] state is valid");
            } else {
                for error in &errors {
                    eprintln!("[i18n:queue] invalid: {error}");
                }
                std::process::exit(1);
            }
        }
        I18nQueueCommand::Render => {
            let mut store = open_queue_store(&db_path);
            let _ = or_fail(store.recover_expired(), "i18n:queue");
            let snapshot = snapshot_or_fail(&store);
            println!("{}", render_status_human(&snapshot));
        }
    }
}

/// Config do repo resolvida a partir do config.toml.
struct RepoConfig {
    config_file: PathBuf,
    root: PathBuf,
    target_dir: PathBuf,
    patches_dir: PathBuf,
    series_file: PathBuf,
    overlays_dir: PathBuf,
    dist_dir: PathBuf,
    upstream_url: String,
    upstream_ref: String,
    base_sha: Sha40,
}

fn load_repo_config(config_path: &Path) -> Result<RepoConfig> {
    let fs = RealFs;
    let get = |key: &str| {
        let parsed = ConfigKey::parse(key).map_err(|e| anyhow::anyhow!("{e}"))?;
        config_get(&fs, config_path, &parsed)
            .with_context(|| format!("lendo {} de {}", parsed.dotted(), config_path.display()))
    };
    let raw_root = config_path
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    // Caminhos de patch precisam ser absolutos: `git -C <dir>` resolve
    // relativos contra o proprio dir, nao contra o CWD do processo.
    let absolute = if raw_root.is_absolute() {
        raw_root
    } else {
        std::env::current_dir()
            .with_context(|| "obtendo diretorio atual")?
            .join(raw_root)
    };
    let root = absolute.canonicalize().unwrap_or(absolute);
    let target_dir = resolve_path(&root, &get("workspace.dir")?)?;
    let patches_dir = resolve_path(&root, &get("patches.dir")?)?;
    let series_file = resolve_path(&root, &get("patches.series")?)?;
    let overlays_dir = resolve_path(&root, &get("overlays.dir")?)?;
    let dist_dir = resolve_path(&root, &get("dist.dir")?)?;
    let upstream_url = get("upstream.url")?;
    let upstream_ref = get("upstream.ref")?;
    let sha_raw = get("upstream.sha")?;
    let base_sha = Sha40::parse(sha_raw.trim())
        .map_err(|e| anyhow::anyhow!("upstream.sha invalido (esperado 40 hex): {e}"))?;
    Ok(RepoConfig {
        config_file: config_path.to_path_buf(),
        root,
        target_dir,
        patches_dir,
        series_file,
        overlays_dir,
        dist_dir,
        upstream_url,
        upstream_ref,
        base_sha,
    })
}

fn print_report(report: &ApplyReport) {
    for entry in &report.applied {
        println!("applied: {entry}");
    }
    for entry in &report.skipped {
        println!("skipped (--skip): {entry}");
    }
    for note in &report.notes {
        println!("{note}");
    }
    for warning in &report.warnings {
        eprintln!("aviso: {warning}");
    }
}

/// Extrai se ha categoria i18n ativa no bloco STATE do CACHE.md.
fn pipeline_is_active(cache_text: &str) -> bool {
    let Some(start) = cache_text.find("\"active_category\"") else {
        return false;
    };
    let rest = &cache_text[start + "\"active_category\"".len()..];
    let Some(colon) = rest.find(':') else {
        return false;
    };
    let value = rest[colon + 1..].trim_start();
    if let Some(inner) = value.strip_prefix('"') {
        let end = inner.find('"').unwrap_or(inner.len());
        return !inner[..end].is_empty() && inner[..end] != *"null";
    }
    !value.starts_with("null")
}

/// Erro fatal de uso da auditoria: `[i18n:audit] mensagem`, saída 2.
///
/// Espelha o `UsageError` de `audit.py` (linhas 1388-1397): erro de uso sai
/// com 2, nunca com 1.
fn audit_fail(message: &str) -> ! {
    let _ = std::io::stdout().flush();
    eprintln!("[i18n:audit] {message}");
    std::process::exit(2);
}

/// Envelope de erro em JSON (`{"ok": false, "error": ...}`, como `audit.py`).
fn audit_error_json(message: &str) -> String {
    match serde_json::to_string(&serde_json::json!({"ok": false, "error": message})) {
        Ok(text) => text,
        Err(_) => "{\"ok\":false}".to_string(),
    }
}

/// Executa `jx i18n-audit` (porte de `scripts/i18n/audit.sh`).
fn run_i18n_audit(
    config_path: &Path,
    unit: Option<String>,
    category: Option<String>,
    as_json: bool,
    base: Option<String>,
    strict: bool,
) {
    let root = resolve_i18n_root(config_path);
    let fs = RealFs;
    let workspace_key = match ConfigKey::parse("workspace.dir") {
        Ok(key) => key,
        Err(err) => audit_fail(&err.to_string()),
    };
    let workspace_dir = match config_get(&fs, config_path, &workspace_key) {
        Ok(value) if !value.is_empty() => value,
        Ok(_) => ".upstream".to_string(),
        Err(BrError::MissingKey(_)) => ".upstream".to_string(),
        Err(err) => audit_fail(&err.to_string()),
    };
    let workspace = root.join(&workspace_dir);
    let paths = AuditPaths {
        root: root.clone(),
        workspace_dir: workspace,
        catalog_lock: root.join("i18n/catalog.lock.json"),
        glossary_tsv: root.join("i18n/glossary.tsv"),
        allowlist_tsv: root.join("i18n/audit-allowlist.tsv"),
    };
    // `BR_I18N_BASE` vale como base explícita, como em `audit.py`.
    let base_env = std::env::var("BR_I18N_BASE")
        .ok()
        .filter(|value| !value.is_empty());
    let options = AuditOptions {
        unit,
        category,
        base: base.or(base_env),
        strict,
    };
    let now = SystemClock.now_iso();
    match run_audit(&ProcessGit::new(), &RealFs, &paths, &options, &now) {
        Ok(report) => {
            if as_json {
                match serde_json::to_string_pretty(&report.to_json_value()) {
                    Ok(text) => println!("{text}"),
                    Err(err) => audit_fail(&format!("cannot encode report ({err})")),
                }
            } else {
                let counts = report.severity_counts();
                let label = if report.mode == "unit" {
                    "unidade"
                } else {
                    "categoria"
                };
                println!("[i18n:audit] {label}: {}", report.target);
                let short_sha = report.base_sha.get(..12).unwrap_or(&report.base_sha);
                println!("[i18n:audit] base: {} ({short_sha})", report.base);
                let short_head = report
                    .workspace_head
                    .get(..12)
                    .unwrap_or(&report.workspace_head);
                println!("[i18n:audit] head: {short_head}");
                println!(
                    "[i18n:audit] arquivos: {} verificados, {} analisados; chaves: {}",
                    report.files_checked, report.files_analyzed, report.keys_compared
                );
                println!(
                    "[i18n:audit] achados: {} blocker, {} major, {} minor, {} nit",
                    counts.blockers, counts.major, counts.minor, counts.nit
                );
                for finding in &report.findings {
                    println!(
                        "  {} {} {}:{}",
                        finding.severity(),
                        finding.code(),
                        finding.file(),
                        finding.line()
                    );
                    println!("    problema: {}", finding.problem());
                    println!("    correcao: {}", finding.fix());
                }
                println!(
                    "[i18n:audit] resultado: {} (bloqueante: {})",
                    if report.ok { "ok" } else { "falhou" },
                    if report.blocking_ok { "nao" } else { "sim" },
                );
            }
            std::process::exit(if report.ok { 0 } else { 1 });
        }
        Err(err) => {
            let message = err.to_string();
            if as_json {
                println!("{}", audit_error_json(&message));
            }
            audit_fail(&message);
        }
    }
}

/// Lê o JSON da stdin (`{}` quando vazia, como `read_stdin_json`).
fn read_stdin_json() -> JsonValue {
    use std::io::Read as _;
    let mut raw = String::new();
    match std::io::stdin().read_to_string(&mut raw) {
        Ok(_) => (),
        Err(err) => fail("bridge", &format!("cannot read stdin ({err})")),
    }
    if raw.trim().is_empty() {
        return JsonValue::Object(serde_json::Map::new());
    }
    match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(err) => fail("bridge", &format!("invalid JSON on stdin ({err})")),
    }
}

/// Campo de texto do payload (`""` quando ausente ou não-texto).
fn str_field(payload: &JsonValue, key: &str, default: &str) -> String {
    payload
        .get(key)
        .and_then(JsonValue::as_str)
        .map_or_else(|| default.to_string(), str::to_string)
}

/// Campo booleano do payload (`false` quando ausente ou não-booleano).
fn bool_field(payload: &JsonValue, key: &str) -> bool {
    payload
        .get(key)
        .and_then(JsonValue::as_bool)
        .unwrap_or(false)
}

/// Extrai o glossário do payload (`en`/`pt`/`notes`, como `cli.py`).
fn glossary_from_payload(payload: &JsonValue) -> Vec<GlossaryEntry> {
    let mut entries = Vec::new();
    if let Some(items) = payload.get("glossary").and_then(JsonValue::as_array) {
        for item in items {
            if !item.is_object() {
                continue;
            }
            let text = |key: &str| {
                item.get(key)
                    .and_then(JsonValue::as_str)
                    .unwrap_or("")
                    .to_string()
            };
            entries.push(GlossaryEntry::new(&text("en"), &text("pt"), &text("notes")));
        }
    }
    entries
}

/// Converte um achado no dicionário JSON da ponte (`to_audit_dict`).
fn finding_to_json(finding: &I18nFinding) -> JsonValue {
    serde_json::json!({
        "severity": finding.severity().as_str(),
        "code": finding.code(),
        "file": finding.file(),
        "line": finding.line(),
        "problem": finding.problem(),
        "fix": finding.fix(),
    })
}

/// Ponte `bridge-analyze-pair` (`oldText`/`newText`/`file`/..., como `cli.py`).
///
/// Coerções de entrada (intencionais, divergem do Python; ver teste
/// `bridge_coercion_differs_from_python_on_purpose`): nulo/ausente/não-texto
/// vira `""` (Python faria `str(None)` = `"None"`); `testFile`/`langFile`
/// só contam `true` JSON (Python aceita qualquer truthy); `maxFindings`
/// não-inteiro cai no padrão 50 (Python lançaria erro); itens não-objeto
/// no glossário são pulidos (igual ao Python).
fn bridge_analyze_pair(payload: &JsonValue) -> JsonValue {
    let max_findings = payload
        .get("maxFindings")
        .and_then(JsonValue::as_u64)
        .map_or(50, |n| n as usize);
    let options = AnalyzeOptions {
        file: str_field(payload, "file", ""),
        kind: payload
            .get("kind")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
            .to_string(),
        glossary: glossary_from_payload(payload),
        test_file: bool_field(payload, "testFile"),
        lang_file: bool_field(payload, "langFile"),
        max_findings,
        nlp: NlpConfig::default(),
    };
    let findings = analyze_pair(
        &str_field(payload, "oldText", ""),
        &str_field(payload, "newText", ""),
        &options,
    );
    JsonValue::Array(findings.iter().map(finding_to_json).collect())
}

/// Ponte `bridge-skeleton` (string JSON com o esqueleto, como `cli.py`).
fn bridge_skeleton(payload: &JsonValue) -> JsonValue {
    let kind = payload
        .get("kind")
        .and_then(JsonValue::as_str)
        .map_or("js", |raw| if raw.is_empty() { "js" } else { raw });
    JsonValue::String(skeleton(&str_field(payload, "text", ""), kind))
}

/// Ponte `bridge-extract-comments` (`kind`/`syntax`/`text`/`line`).
fn bridge_extract_comments(payload: &JsonValue) -> JsonValue {
    let kind = payload
        .get("kind")
        .and_then(JsonValue::as_str)
        .map_or("js", |raw| if raw.is_empty() { "js" } else { raw });
    let comments = extract_comments(&str_field(payload, "text", ""), kind);
    JsonValue::Array(
        comments
            .iter()
            .map(|comment| {
                serde_json::json!({
                    "kind": comment.kind,
                    "syntax": comment.syntax,
                    "text": comment.text,
                    "line": comment.line,
                })
            })
            .collect(),
    )
}

/// Ponte `bridge-extract-php-entries` (`segments`/`key`/`line`/`value`...).
fn bridge_extract_php_entries(payload: &JsonValue) -> JsonValue {
    let entries = extract_php_lang_entries(&str_field(payload, "text", ""));
    JsonValue::Array(
        entries
            .iter()
            .map(|entry| {
                serde_json::json!({
                    "segments": entry.segments,
                    "key": entry.key,
                    "line": entry.line,
                    "value": entry.value,
                    "valueLine": entry.value_line,
                })
            })
            .collect(),
    )
}

/// Ponte `bridge-placeholders` (lista de placeholders, como `cli.py`).
fn bridge_placeholders(payload: &JsonValue) -> JsonValue {
    JsonValue::Array(
        extract_placeholders(&str_field(payload, "text", ""))
            .into_iter()
            .map(JsonValue::String)
            .collect(),
    )
}

/// Ponte `bridge-nlp-probe` (`language`/`confidence`/termos, como `cli.py`).
fn bridge_nlp_probe(payload: &JsonValue) -> JsonValue {
    let verdict = classify_label_language(
        &str_field(payload, "text", ""),
        &glossary_from_payload(payload),
        &NlpOptions::default(),
    );
    serde_json::json!({
        "language": verdict.language.as_str(),
        "confidence": verdict.confidence,
        "englishTerms": verdict.english_terms,
        "portugueseTerms": verdict.portuguese_terms,
        "ambiguousTerms": verdict.ambiguous_terms,
    })
}

/// Executa uma ponte: lê o JSON da stdin e imprime o JSON de saída.
fn run_bridge(build: fn(&JsonValue) -> JsonValue) {
    let payload = read_stdin_json();
    let value = build(&payload);
    match serde_json::to_string(&value) {
        Ok(text) => println!("{text}"),
        Err(err) => fail("bridge", &format!("cannot encode output ({err})")),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut fs = RealFs;
    let git = ProcessGit::new();
    match cli.command {
        Command::ConfigGet { key } => {
            let parsed = ConfigKey::parse(&key).map_err(|e| anyhow::anyhow!("{e}"))?;
            let value = config_get(&fs, &cli.config, &parsed).with_context(|| {
                format!("lendo {} de {}", parsed.dotted(), cli.config.display())
            })?;
            println!("{value}");
        }
        Command::ConfigSet { key, value } => {
            let parsed = ConfigKey::parse(&key).map_err(|e| anyhow::anyhow!("{e}"))?;
            config_set(&mut fs, &cli.config, &parsed, &value).with_context(|| {
                format!("gravando {} em {}", parsed.dotted(), cli.config.display())
            })?;
        }
        Command::SeriesEntries { series } => {
            let text = std::fs::read_to_string(&series)
                .with_context(|| format!("lendo series {}", series.display()))?;
            for entry in parse_series(&text) {
                println!("{entry}");
            }
        }
        Command::SeriesInsert { series, entry } => {
            let parsed = SeriesEntry::new(&entry).map_err(|e| anyhow::anyhow!("{e}"))?;
            let text = std::fs::read_to_string(&series)
                .with_context(|| format!("lendo series {}", series.display()))?;
            let updated = insert_into_text(&text, &parsed).map_err(|e| anyhow::anyhow!("{e}"))?;
            let tmp = series.with_extension("tmp-jx");
            std::fs::write(&tmp, &updated)
                .with_context(|| format!("gravando {}", tmp.display()))?;
            std::fs::rename(&tmp, &series)
                .with_context(|| format!("gravando {}", series.display()))?;
        }
        Command::Apply {
            target,
            check,
            reverse,
            skip,
            no_snapshot,
        } => {
            let repo = load_repo_config(&cli.config)?;
            let request = ApplyRequest {
                check_only: check,
                reverse,
                skip,
                snapshot: !no_snapshot,
            };
            let paths = ApplyPaths {
                root: repo.root,
                target_dir: target.unwrap_or(repo.target_dir),
                series_file: repo.series_file,
                overlays_dir: repo.overlays_dir,
                base_sha: repo.base_sha,
            };
            print_report(
                &run_apply(&git, &mut fs, &paths, &request).map_err(|e| anyhow::anyhow!("{e}"))?,
            );
        }
        Command::Lint { keep_cache } => {
            let repo = load_repo_config(&cli.config)?;
            let paths = LintPaths {
                root: repo.root.clone(),
                patches_dir: repo.patches_dir,
                series_file: repo.series_file,
                overlays_dir: repo.overlays_dir,
                workspace_dir: repo.target_dir,
                upstream_url: repo.upstream_url,
                base_sha: repo.base_sha,
                cache_dir: repo.root.join(".br-cache/lint"),
            };
            let report = run_lint(&git, &mut fs, &paths, keep_cache);
            for failure in &report.failures {
                match &failure.target {
                    Some(target) => println!("falha [{target}]: {}", failure.message),
                    None => println!("falha: {}", failure.message),
                }
            }
            if report.is_ok() {
                println!("patch series is valid ({} patch(es))", report.applied);
            } else {
                anyhow::bail!(
                    "{} problem(s) found in the patch series",
                    report.failures.len()
                );
            }
        }
        Command::Dump {
            slug,
            domain,
            message,
            description,
            origin,
            bug,
            signoff,
            applied_upstream,
            no_series,
        } => {
            let repo = load_repo_config(&cli.config)?;
            let request = NewPatchRequest {
                slug,
                domain,
                message,
                description,
                origin: origin.parse().map_err(|e: jx_patch::dep3::Dep3Error| {
                    anyhow::anyhow!("origin invalida: {e}")
                })?,
                bug,
                signoff,
                applied_upstream: applied_upstream.parse().map_err(
                    |e: jx_patch::dep3::Dep3Error| {
                        anyhow::anyhow!("applied-upstream invalido: {e}")
                    },
                )?,
                add_to_series: !no_series,
            };
            let paths = DumpPaths {
                root: repo.root,
                target_dir: repo.target_dir,
                patches_dir: repo.patches_dir,
                overlays_dir: repo.overlays_dir,
                series_file: repo.series_file,
                base_sha: repo.base_sha,
            };
            let outcome = dump_new(&git, &mut fs, &SystemClock, &paths, &request)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("patch written: {}", outcome.patch_name);
            println!("series entry: {}", outcome.series_entry);
            for warning in &outcome.warnings {
                eprintln!("aviso: {warning}");
            }
            eprintln!("rode `jx lint` para validar a serie");
        }
        Command::Refresh { entry, force } => {
            let repo = load_repo_config(&cli.config)?;
            let db_path = db_path_for(&repo.root, env_path("BR_I18N_CACHE"));
            let sqlite_active = db_path.exists()
                && Store::open(&db_path, I18nClock)
                    .and_then(|store| store.snapshot())
                    .is_ok_and(|snap| {
                        snap.get("active_category")
                            .and_then(JsonValue::as_str)
                            .is_some_and(|cat| !cat.is_empty() && cat != "null")
                    });
            let cache_path = repo.root.join("CACHE.md");
            let pipeline_active = sqlite_active
                || std::fs::read_to_string(&cache_path)
                    .map(|text| pipeline_is_active(&text))
                    .unwrap_or(false);
            let request = RefreshRequest {
                entry,
                force,
                pipeline_active,
            };
            let paths = DumpPaths {
                root: repo.root,
                target_dir: repo.target_dir,
                patches_dir: repo.patches_dir,
                overlays_dir: repo.overlays_dir,
                series_file: repo.series_file,
                base_sha: repo.base_sha,
            };
            let outcome = dump_refresh(&git, &mut fs, &paths, &request)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("patch refreshed: {}", outcome.patch_name);
            eprintln!("rode `jx lint` para validar a serie");
        }
        Command::Setup {
            with_deps,
            force,
            c_ref,
            sha,
            local,
        } => {
            let repo = load_repo_config(&cli.config)?;
            let sha_override = sha
                .map(|raw| {
                    Sha40::parse(raw.trim())
                        .map_err(|e| anyhow::anyhow!("--sha invalido (esperado 40 hex): {e}"))
                })
                .transpose()?;
            let request = SetupRequest {
                with_deps,
                force,
                ref_override: c_ref,
                sha_override,
                local,
            };
            let paths = SetupPaths {
                root: repo.root,
                target_dir: repo.target_dir,
                series_file: repo.series_file,
                overlays_dir: repo.overlays_dir,
                upstream_url: repo.upstream_url,
                base_ref: repo.upstream_ref,
                base_sha: repo.base_sha,
            };
            let report = run_setup(&git, &mut fs, &RealCmdRunner, &paths, &request)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            for note in &report.notes {
                println!("{note}");
            }
            for warning in &report.warnings {
                eprintln!("aviso: {warning}");
            }
        }
        Command::Rebase {
            new_ref,
            sha,
            no_pin,
            dry_run,
            force,
        } => {
            let repo = load_repo_config(&cli.config)?;
            let request = RebaseRequest {
                new_ref,
                new_sha: sha,
                update_pin: !no_pin,
                dry_run,
                force,
            };
            let paths = RebasePaths {
                root: repo.root,
                target_dir: repo.target_dir,
                series_file: repo.series_file,
                overlays_dir: repo.overlays_dir,
                upstream_url: repo.upstream_url,
                current_ref: repo.upstream_ref,
                current_sha: repo.base_sha,
                config_file: repo.config_file,
            };
            let report =
                run_rebase(&git, &mut fs, &paths, &request).map_err(|e| anyhow::anyhow!("{e}"))?;
            for note in &report.notes {
                println!("{note}");
            }
        }
        Command::Package { version, output } => {
            let repo = load_repo_config(&cli.config)?;
            // Serie precisa estar valida antes de gerar release.
            let lint_paths = LintPaths {
                root: repo.root.clone(),
                patches_dir: repo.patches_dir.clone(),
                series_file: repo.series_file.clone(),
                overlays_dir: repo.overlays_dir.clone(),
                workspace_dir: repo.target_dir.clone(),
                upstream_url: repo.upstream_url.clone(),
                base_sha: repo.base_sha,
                cache_dir: repo.root.join(".br-cache/lint"),
            };
            let lint = run_lint(&git, &mut fs, &lint_paths, false);
            if !lint.is_ok() {
                for failure in &lint.failures {
                    match &failure.target {
                        Some(target) => println!("falha [{target}]: {}", failure.message),
                        None => println!("falha: {}", failure.message),
                    }
                }
                anyhow::bail!(
                    "{} problem(s) found in the patch series",
                    lint.failures.len()
                );
            }
            let epoch = std::env::var("SOURCE_DATE_EPOCH")
                .ok()
                .and_then(|raw| raw.parse::<u64>().ok())
                .unwrap_or(0);
            let request = PackageRequest {
                version,
                output,
                source_date_epoch: epoch,
            };
            let paths = PackagePaths {
                root: repo.root,
                target_dir: repo.target_dir,
                patches_dir: repo.patches_dir,
                series_file: repo.series_file,
                overlays_dir: repo.overlays_dir,
                dist_dir: repo.dist_dir,
                upstream_url: repo.upstream_url,
                upstream_ref: repo.upstream_ref,
                base_sha: repo.base_sha,
                config_file: repo.config_file,
            };
            let stage = tempfile::tempdir().with_context(|| "criando diretorio de staging")?;
            let report = run_package_stage(&git, &mut fs, &paths, &request, stage.path())
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            write_tarball(
                stage.path(),
                &report.package_name,
                &report.archive,
                request.source_date_epoch,
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("package written: {}", report.archive.display());
        }
        Command::I18nCatalog { write, check, json } => {
            run_i18n_catalog(&cli.config, write, check, json);
        }
        Command::I18nQueue { db, command } => {
            run_i18n_queue(&cli.config, db, &command);
        }
        Command::I18nAudit {
            unit,
            category,
            json,
            base,
            strict,
        } => {
            run_i18n_audit(&cli.config, unit, category, json, base, strict);
        }
        Command::BridgeAnalyzePair => run_bridge(bridge_analyze_pair),
        Command::BridgeSkeleton => run_bridge(bridge_skeleton),
        Command::BridgeExtractComments => run_bridge(bridge_extract_comments),
        Command::BridgeExtractPhpEntries => run_bridge(bridge_extract_php_entries),
        Command::BridgePlaceholders => run_bridge(bridge_placeholders),
        Command::BridgeNlpProbe => run_bridge(bridge_nlp_probe),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        active_claim_count, bridge_analyze_pair, bridge_extract_comments,
        bridge_extract_php_entries, bridge_nlp_probe, bridge_placeholders, bridge_skeleton,
        catalog_unit_files, check_review_report, claimed_units, db_path_for, display_title,
        finding_to_json, glossary_from_payload, lock_path_for, manifest_path_for, parse_flag_int,
        pipeline_is_active, policy_lease_minutes, policy_max_agents, render_next_human,
        render_next_json, render_status_human, select_i18n_root, status_machine_json, I18nFinding,
        JsonValue,
    };
    use serde_json::json;
    use std::path::{Path, PathBuf};

    /// Monta um relatorio de revisao valido com nota 10.
    fn ten_report() -> JsonValue {
        json!({
            "score": 10,
            "criteria": {"coverage": 2, "naturalness": 2, "consistency": 2, "context": 2, "safety": 2},
            "findings": [],
            "untranslated": [],
            "glossary_violations": [],
            "test_impact": [],
        })
    }

    /// Monta um relatorio com um achado `blocker` e nota 9 (invalido).
    fn blocker_report() -> JsonValue {
        json!({
            "score": 9,
            "criteria": {"coverage": 2, "naturalness": 2, "consistency": 2, "context": 2, "safety": 1},
            "findings": [
                {"id": "F1", "severity": "blocker", "file": "a.ts", "line": 3, "problem": "x", "fix": "y"},
            ],
            "untranslated": [],
            "glossary_violations": [],
            "test_impact": [],
        })
    }

    /// Extracao da mensagem de erro sem `unwrap`/`expect` (negados neste crate).
    fn err_message(result: Result<(), String>) -> String {
        match result {
            Ok(()) => String::from("<ok>"),
            Err(message) => message,
        }
    }

    /// Instantaneo minimo com duas categorias e um claim ativo.
    fn sample_snapshot() -> JsonValue {
        json!({
            "active_category": "cat-a",
            "policy": {"max_agents": 3, "lease_minutes": 120},
            "categories": {
                "cat-a": {"id": "cat-a", "order": 1, "stage": "analysis", "status": "active", "consistency_score": null, "patches": ["0001-x.patch"], "units": ["u1", "u2"]},
                "cat-b": {"id": "cat-b", "order": 2, "stage": "pending", "status": "pending", "consistency_score": null, "patches": [], "units": ["u3"]}
            },
            "units": {
                "u1": {"id": "u1", "category": "cat-a", "title": "Primeira", "file_count": 2, "stage": "analyzing", "status": "active", "score": null, "attempts": 0, "claim": {"owner": "alice", "role": "analyst", "claimed_at": "t", "expires_at": "t2"}, "analysis": null, "findings": null},
                "u2": {"id": "u2", "category": "cat-a", "title": "Segunda", "file_count": 1, "stage": "pending", "status": "pending", "score": null, "attempts": 0, "claim": null, "analysis": null, "findings": null},
                "u3": {"id": "u3", "category": "cat-b", "title": "Terceira", "file_count": 0, "stage": "pending", "status": "pending", "score": null, "attempts": 0, "claim": null, "analysis": null, "findings": null}
            },
            "patches": {"0001-x.patch": {}},
            "overlays": {}
        })
    }

    /// Lock minimo com arquivos por unidade.
    fn sample_catalog() -> JsonValue {
        json!({
            "categories": [
                {"id": "cat-a", "units": [
                    {"id": "u1", "files": ["resources/scripts/a.ts"]},
                    {"id": "u2", "files": []},
                ]},
            ],
        })
    }

    #[test]
    fn detects_active_category() {
        assert!(!pipeline_is_active("sem bloco"));
        assert!(!pipeline_is_active(r#"{"active_category": null}"#));
        assert!(!pipeline_is_active(r#"{"active_category": "null"}"#));
        assert!(pipeline_is_active(r#"{"active_category": "admin-setup"}"#));
        assert!(pipeline_is_active(
            "<!-- STATE:BEGIN -->\n{\"active_category\": \"auth\"}\n<!-- STATE:END -->"
        ));
    }

    #[test]
    fn review_report_ten_with_empty_findings_is_ok() {
        assert!(check_review_report(&ten_report(), 10).is_ok());
    }

    #[test]
    fn review_report_blocker_with_nine_is_rejected() {
        let message = err_message(check_review_report(&blocker_report(), 9));
        assert!(message.contains("at most 7"), "unexpected: {message}");
    }

    #[test]
    fn review_report_below_ten_requires_findings() {
        let mut report = ten_report();
        report["score"] = json!(7);
        report["criteria"] =
            json!({"coverage": 2, "naturalness": 2, "consistency": 1, "context": 1, "safety": 1});
        let message = err_message(check_review_report(&report, 7));
        assert!(
            message.contains("at least one finding"),
            "unexpected: {message}"
        );
    }

    #[test]
    fn review_report_ten_rejects_findings() {
        let mut report = ten_report();
        report["findings"] = json!([
            {"id": "F1", "severity": "nit", "file": "a.ts", "line": 1, "problem": "x", "fix": "y"},
        ]);
        let message = err_message(check_review_report(&report, 10));
        assert!(
            message.contains("empty findings array"),
            "unexpected: {message}"
        );
    }

    #[test]
    fn review_report_criteria_sum_must_match_score() {
        let mut report = blocker_report();
        report["score"] = json!(6);
        report["criteria"] =
            json!({"coverage": 2, "naturalness": 2, "consistency": 2, "context": 2, "safety": 1});
        report["findings"] = json!([
            {"id": "F1", "severity": "minor", "file": "a.ts", "line": 1, "problem": "x", "fix": "y"},
        ]);
        let message = err_message(check_review_report(&report, 6));
        assert!(
            message.contains("criteria sum 9 does not match score 6"),
            "unexpected: {message}"
        );
    }

    #[test]
    fn review_report_criteria_keys_must_be_exact() {
        let mut missing = ten_report();
        missing["criteria"] =
            json!({"coverage": 2, "naturalness": 2, "consistency": 2, "context": 2});
        let message = err_message(check_review_report(&missing, 10));
        assert!(
            message.contains("exactly coverage"),
            "unexpected: {message}"
        );
        let mut extra = ten_report();
        extra["criteria"] = json!({"coverage": 2, "naturalness": 2, "consistency": 2, "context": 2, "safety": 2, "tone": 0});
        let message = err_message(check_review_report(&extra, 10));
        assert!(
            message.contains("exactly coverage"),
            "unexpected: {message}"
        );
    }

    #[test]
    fn review_report_criteria_values_are_bounded() {
        let mut report = ten_report();
        report["criteria"] =
            json!({"coverage": 3, "naturalness": 2, "consistency": 2, "context": 2, "safety": 1});
        report["score"] = json!(10);
        let message = err_message(check_review_report(&report, 10));
        assert!(
            message.contains("criteria.coverage must be 0, 1 or 2"),
            "unexpected: {message}"
        );
    }

    #[test]
    fn review_report_ten_requires_empty_untranslated() {
        let mut report = ten_report();
        report["untranslated"] = json!(["label"]);
        let message = err_message(check_review_report(&report, 10));
        assert!(
            message.contains("untranslated to be empty"),
            "unexpected: {message}"
        );
        let mut glossary = ten_report();
        glossary["glossary_violations"] = json!(["term"]);
        let message = err_message(check_review_report(&glossary, 10));
        assert!(
            message.contains("glossary_violations to be empty"),
            "unexpected: {message}"
        );
    }

    #[test]
    fn review_report_finding_fields_are_validated() {
        let mut empty_file = blocker_report();
        empty_file["findings"] = json!([
            {"id": "F1", "severity": "major", "file": "  ", "line": 1, "problem": "x", "fix": "y"},
        ]);
        empty_file["score"] = json!(5);
        empty_file["criteria"] =
            json!({"coverage": 1, "naturalness": 1, "consistency": 1, "context": 1, "safety": 1});
        let message = err_message(check_review_report(&empty_file, 5));
        assert!(
            message.contains("finding file must be a non-empty string"),
            "unexpected: {message}"
        );

        let mut bad_severity = blocker_report();
        bad_severity["findings"] = json!([
            {"id": "F1", "severity": "grave", "file": "a.ts", "line": 1, "problem": "x", "fix": "y"},
        ]);
        bad_severity["score"] = json!(5);
        bad_severity["criteria"] =
            json!({"coverage": 1, "naturalness": 1, "consistency": 1, "context": 1, "safety": 1});
        let message = err_message(check_review_report(&bad_severity, 5));
        assert!(
            message.contains("invalid finding severity"),
            "unexpected: {message}"
        );

        let mut bad_line = blocker_report();
        bad_line["findings"] = json!([
            {"id": "F1", "severity": "nit", "file": "a.ts", "line": 0, "problem": "x", "fix": "y"},
        ]);
        bad_line["score"] = json!(5);
        bad_line["criteria"] =
            json!({"coverage": 1, "naturalness": 1, "consistency": 1, "context": 1, "safety": 1});
        let message = err_message(check_review_report(&bad_line, 5));
        assert!(
            message.contains("positive integer"),
            "unexpected: {message}"
        );
    }

    #[test]
    fn parse_flag_int_handles_options_and_values() {
        assert!(matches!(parse_flag_int(None, "--limit"), Ok(None)));
        assert!(matches!(parse_flag_int(Some("3"), "--limit"), Ok(Some(3))));
        assert!(matches!(
            parse_flag_int(Some("-5"), "--limit"),
            Ok(Some(-5))
        ));
    }

    #[test]
    fn parse_flag_int_rejects_non_integer() {
        for raw in ["", "abc", "3.5", "--3", " 3", "0x10"] {
            let message = err_message(parse_flag_int(Some(raw), "--limit").map(|_| ()));
            assert!(
                message.contains("must be an integer"),
                "unexpected for {raw}: {message}"
            );
        }
    }

    #[test]
    fn parse_flag_int_rejects_out_of_range() {
        let message =
            err_message(parse_flag_int(Some("99999999999999999999999"), "--limit").map(|_| ()));
        assert!(message.contains("out of range"), "unexpected: {message}");
    }

    #[test]
    fn path_defaults_and_overrides() {
        let root = PathBuf::from("/repo");
        assert_eq!(
            manifest_path_for(&root, None),
            PathBuf::from("/repo/i18n/catalog.json")
        );
        assert_eq!(
            manifest_path_for(&root, Some(PathBuf::from("/x/m.json"))),
            PathBuf::from("/x/m.json")
        );
        assert_eq!(
            lock_path_for(&root),
            PathBuf::from("/repo/i18n/catalog.lock.json")
        );
        assert_eq!(
            db_path_for(&root, None),
            PathBuf::from("/repo/.br-cache/i18n-queue.sqlite")
        );
        assert_eq!(
            db_path_for(&root, Some(PathBuf::from("/x/q.sqlite"))),
            PathBuf::from("/x/q.sqlite")
        );
        assert_eq!(
            select_i18n_root(Path::new("/repo/config.toml"), None),
            PathBuf::from("/repo")
        );
        assert_eq!(
            select_i18n_root(
                Path::new("/repo/config.toml"),
                Some(PathBuf::from("/outro"))
            ),
            PathBuf::from("/outro")
        );
    }

    #[test]
    fn policy_defaults_come_from_snapshot() {
        assert_eq!(policy_lease_minutes(&sample_snapshot()), 120);
        assert_eq!(policy_max_agents(&sample_snapshot()), 3);
        assert_eq!(policy_lease_minutes(&json!({})), 120);
        assert_eq!(policy_max_agents(&json!({})), 3);
        let custom = json!({"policy": {"max_agents": 1, "lease_minutes": 30}});
        assert_eq!(policy_lease_minutes(&custom), 30);
        assert_eq!(policy_max_agents(&custom), 1);
    }

    #[test]
    fn status_machine_json_uses_queue_keys() {
        let machine = status_machine_json(&sample_snapshot());
        assert_eq!(
            machine.get("active_category").and_then(JsonValue::as_str),
            Some("cat-a")
        );
        assert_eq!(
            machine.get("active_stage").and_then(JsonValue::as_str),
            Some("analysis")
        );
        let units = machine.get("active_units").and_then(JsonValue::as_array);
        assert_eq!(units.map(Vec::len), Some(2));
        let categories = machine.get("categories").and_then(JsonValue::as_array);
        assert_eq!(categories.map(Vec::len), Some(2));
        let claims: Vec<String> = machine
            .get("claims")
            .and_then(JsonValue::as_array)
            .map_or_else(Vec::new, |items| {
                items
                    .iter()
                    .filter_map(JsonValue::as_str)
                    .map(str::to_string)
                    .collect()
            });
        assert_eq!(claims, vec!["u1".to_string()]);
        assert_eq!(machine.get("patches").and_then(JsonValue::as_i64), Some(1));
        assert_eq!(machine.get("overlays").and_then(JsonValue::as_i64), Some(0));
    }

    #[test]
    fn render_status_human_lists_categories() {
        let text = render_status_human(&sample_snapshot());
        assert!(
            text.contains("active category: cat-a"),
            "unexpected: {text}"
        );
        assert!(text.contains("cat-a"), "unexpected: {text}");
        assert!(text.contains("0/2"), "unexpected: {text}");
        assert!(text.contains("patch=1"), "unexpected: {text}");
    }

    #[test]
    fn render_status_human_handles_missing_focus() {
        let text = render_status_human(&json!({}));
        assert!(text.contains("active category: none"), "unexpected: {text}");
    }

    #[test]
    fn next_rendering_uses_snapshot_and_lock() {
        let snapshot = sample_snapshot();
        let catalog = sample_catalog();
        let ids = vec!["u1".to_string(), "u2".to_string()];
        let human = render_next_human(&snapshot, &ids);
        assert!(
            human.contains("u1\tPrimeira\t2 file(s)"),
            "unexpected: {human}"
        );
        let machine = render_next_json(&snapshot, &catalog, &ids);
        let rows = machine.as_array().map_or_else(Vec::new, Clone::clone);
        assert_eq!(rows.len(), 2);
        let files: Vec<String> = rows[0]
            .get("files")
            .and_then(JsonValue::as_array)
            .map_or_else(Vec::new, |items| {
                items
                    .iter()
                    .filter_map(JsonValue::as_str)
                    .map(str::to_string)
                    .collect()
            });
        assert_eq!(files, vec!["resources/scripts/a.ts".to_string()]);
        assert_eq!(
            rows[1]
                .get("files")
                .and_then(JsonValue::as_array)
                .map(Vec::len),
            Some(0)
        );
        assert_eq!(
            catalog_unit_files(&catalog, "u1"),
            vec!["resources/scripts/a.ts".to_string()]
        );
        assert!(catalog_unit_files(&catalog, "ausente").is_empty());
    }

    #[test]
    fn claimed_units_count_open_claims() {
        assert_eq!(claimed_units(&sample_snapshot()), vec!["u1".to_string()]);
        assert_eq!(active_claim_count(&sample_snapshot()), 1);
        assert_eq!(active_claim_count(&json!({})), 0);
    }

    #[test]
    fn display_title_passes_strings_through() {
        assert_eq!(display_title(&json!("ola")), "ola");
        assert_eq!(display_title(&json!(3)), "3");
        assert_eq!(display_title(&JsonValue::Null), "null");
    }

    /// Monta um achado ou falha o teste com a mensagem de erro.
    fn must_finding() -> I18nFinding {
        use jx_i18n::core::Severity;
        match I18nFinding::new(
            Severity::Minor,
            "english-label",
            "a.tsx",
            3,
            "problema",
            "correcao",
        ) {
            Ok(finding) => finding,
            Err(err) => panic!("finding inesperado: {err}"),
        }
    }

    /// Chaves ordenadas de um objeto JSON.
    fn sorted_keys(value: &JsonValue) -> Vec<String> {
        match value.as_object() {
            Some(map) => {
                let mut keys: Vec<String> = map.keys().cloned().collect();
                keys.sort();
                keys
            }
            None => Vec::new(),
        }
    }

    #[test]
    fn bridge_finding_json_uses_audit_dict_keys() {
        let value = finding_to_json(&must_finding());
        assert_eq!(
            sorted_keys(&value),
            vec!["code", "file", "fix", "line", "problem", "severity"]
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            value.get("severity").and_then(JsonValue::as_str),
            Some("minor")
        );
        assert_eq!(value.get("line").and_then(JsonValue::as_i64), Some(3));
    }

    #[test]
    fn bridge_analyze_pair_identical_texts_yield_no_findings() {
        let payload = json!({
            "oldText": "const x = 1;\n",
            "newText": "const x = 1;\n",
            "file": "x.ts",
            "kind": "ts",
            "maxFindings": 50,
        });
        match bridge_analyze_pair(&payload) {
            JsonValue::Array(items) => assert!(items.is_empty()),
            other => panic!("esperava lista: {other}"),
        }
    }

    #[test]
    fn bridge_analyze_pair_flags_placeholder_change() {
        let payload = json!({
            "oldText": "export const message = 'Hello :name';\n",
            "newText": "export const message = 'Ola';\n",
            "file": "x.ts",
            "kind": "ts",
            "testFile": false,
            "langFile": false,
        });
        match bridge_analyze_pair(&payload) {
            JsonValue::Array(items) => {
                let codes: Vec<&str> = items
                    .iter()
                    .filter_map(|item| item.get("code"))
                    .filter_map(JsonValue::as_str)
                    .collect();
                assert!(
                    codes.contains(&"placeholder-changed"),
                    "inesperado: {items:?}"
                );
                for item in &items {
                    assert_eq!(
                        sorted_keys(item),
                        vec!["code", "file", "fix", "line", "problem", "severity"]
                            .into_iter()
                            .map(str::to_string)
                            .collect::<Vec<_>>()
                    );
                }
            }
            other => panic!("esperava lista: {other}"),
        }
    }

    #[test]
    fn bridge_skeleton_returns_string() {
        let payload = json!({"text": "export const x = 1;\n", "kind": "js"});
        match bridge_skeleton(&payload) {
            JsonValue::String(_) => (),
            other => panic!("esperava string: {other}"),
        }
        let empty_kind = json!({"text": "const x = 1;\n", "kind": ""});
        match bridge_skeleton(&empty_kind) {
            JsonValue::String(_) => (),
            other => panic!("esperava string: {other}"),
        }
    }

    #[test]
    fn bridge_coercion_differs_from_python_on_purpose() {
        // Tabela de coercitividade intencional (diverge do Python, que faz
        // `str(None)` = `"None"`, truthiness livre e `int()` estrito):
        // nulo/ausente/nao-texto -> ""; booleanos so contam `true` JSON;
        // maxFindings so aceita u64 (senao 50); stdin nao-objeto e tratado
        // como `{}` no nivel do `read_stdin_json` (nao testado aqui).
        assert_eq!(
            bridge_placeholders(&json!({"text": null})),
            JsonValue::Array(vec![])
        );
        assert_eq!(
            bridge_placeholders(&json!({"text": 42})),
            JsonValue::Array(vec![])
        );
        // "true" como string NAO e truthy (Python diria True).
        let payload =
            json!({"oldText": "const a = 1;\n", "newText": "const a = 1;\n", "testFile": "true"});
        match bridge_analyze_pair(&payload) {
            JsonValue::Array(_) => (),
            other => panic!("esperava lista: {other}"),
        }
        // maxFindings como string e ignorado (padrao 50, Python lancaria erro).
        let payload = json!({"oldText": "x", "newText": "y", "maxFindings": "10"});
        match bridge_analyze_pair(&payload) {
            JsonValue::Array(_) => (),
            other => panic!("esperava lista: {other}"),
        }
        // Glossario com item nao-objeto e pulado, como no Python.
        let payload =
            json!({"text": "Save", "glossary": ["nao-objeto", {"en": "Save", "pt": "Salvar"}]});
        let verdict = bridge_nlp_probe(&payload);
        assert_eq!(
            verdict.get("language").and_then(JsonValue::as_str),
            Some("en")
        );
    }

    #[test]
    fn bridge_extract_comments_uses_comment_keys() {
        let payload = json!({"text": "// nota\nconst x = 1;\n", "kind": "js"});
        match bridge_extract_comments(&payload) {
            JsonValue::Array(items) => {
                assert_eq!(items.len(), 1);
                assert_eq!(
                    sorted_keys(&items[0]),
                    vec!["kind", "line", "syntax", "text"]
                        .into_iter()
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                );
                assert_eq!(
                    items[0].get("text").and_then(JsonValue::as_str),
                    Some("nota")
                );
            }
            other => panic!("esperava lista: {other}"),
        }
    }

    #[test]
    fn bridge_extract_php_entries_uses_entry_keys() {
        let payload = json!({"text": "<?php return ['a' => 'x'];"});
        match bridge_extract_php_entries(&payload) {
            JsonValue::Array(items) => {
                assert_eq!(items.len(), 1);
                assert_eq!(
                    sorted_keys(&items[0]),
                    vec!["key", "line", "segments", "value", "valueLine"]
                        .into_iter()
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                );
                assert_eq!(items[0].get("key").and_then(JsonValue::as_str), Some("a"));
                assert_eq!(items[0].get("value").and_then(JsonValue::as_str), Some("x"));
            }
            other => panic!("esperava lista: {other}"),
        }
    }

    #[test]
    fn bridge_placeholders_returns_array() {
        let payload = json!({"text": "Ola :nome, veja :outro"});
        match bridge_placeholders(&payload) {
            JsonValue::Array(items) => {
                let texts: Vec<&str> = items.iter().filter_map(JsonValue::as_str).collect();
                assert_eq!(texts, vec![":nome", ":outro"]);
            }
            other => panic!("esperava lista: {other}"),
        }
    }

    #[test]
    fn bridge_nlp_probe_uses_probe_keys() {
        let payload = json!({"text": "Salvar alteracoes", "glossary": []});
        match bridge_nlp_probe(&payload) {
            value @ JsonValue::Object(_) => {
                assert_eq!(
                    sorted_keys(&value),
                    vec![
                        "ambiguousTerms",
                        "confidence",
                        "englishTerms",
                        "language",
                        "portugueseTerms"
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect::<Vec<_>>()
                );
                assert!(
                    value.get("language").and_then(JsonValue::as_str).is_some(),
                    "inesperado: {value}"
                );
            }
            other => panic!("esperava objeto: {other}"),
        }
    }

    #[test]
    fn bridge_glossary_maps_payload_entries() {
        let payload = json!({
            "glossary": [
                {"en": "Save", "pt": "Salvar", "notes": "manter"},
                {"en": "Name", "pt": "Nome"},
                "ignorado",
            ],
        });
        let entries = glossary_from_payload(&payload);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].term_en, "Save");
        assert_eq!(entries[0].term_pt, "Salvar");
        assert_eq!(entries[0].notes, "manter");
        assert_eq!(entries[1].term_en, "Name");
        assert_eq!(entries[1].notes, "");
        assert!(glossary_from_payload(&json!({})).is_empty());
    }
}
