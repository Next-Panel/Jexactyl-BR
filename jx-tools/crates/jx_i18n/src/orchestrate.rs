//! Orquestração da auditoria i18n sobre git + filesystem.
//!
//! Port da função `audit()` de `scripts/i18n/i18n_toolkit/audit.py` (linhas
//! 1069-1385): resolve a unidade/categoria no lock, monta o conjunto de
//! arquivos alterados + untracked, compara pares de revisão, confere os
//! overlays pt-BR contra o en, valida chaves de lang, aplica a allowlist e
//! devolve o relatório com as mesmas chaves do `--json` do `audit.sh`.
//!
//! Correspondência com `audit.py`:
//!
//! - Seleção de unidade/categoria espelha `find_unit`/`find_category`
//!   (linhas 886-898) e as validações de `audit()` (linhas 1075-1117).
//! - Posse de arquivo (`owner_of`) espelha `build_ownership` (linhas
//!   928-955): exato primeiro, depois globs menos `exclude`, depois a lista
//!   de excluídos.
//! - Regra da base espelha as linhas 1129-1140: `BR_I18N_BASE` ou `--base`
//!   resolvida via `rev_parse`; árvore limpa sem base explícita é erro de
//!   uso. A resolução de `BR_I18N_BASE` vive na CLI (aqui só chega
//!   [`AuditOptions::base`]), para a biblioteca não ler ambiente.
//! - Contrapartes de lang espelham `counterpart_paths`/`resolve_counterpart`
//!   (linhas 970-994): `overlays/` tem preferência sobre `resources/`.
//! - Comparação de overlays espelha as linhas 1241-1290 via
//!   [`crate::php::compare_lang_entries`] +
//!   [`crate::audit::analyze_overlay_values_with_glossary`].
//! - Chaves de lang espelham `build_lang_index`/`lang_key_exists` (linhas
//!   1027-1058), incluindo o fallback de namespace.
//! - `audit-incomplete` espelha as linhas 1311-1322; a allowlist espelha
//!   `read_audit_allowlist` + subtração exata (linhas 120-135 e 1328-1340).
//! - `ok`/`blocking_ok`/`stats` espelham as linhas 1341-1360, com as chaves
//!   de severidade `blockers`/`major`/`minor`/`nit`.
//!
//! Divergências conscientes (limites do [`jx_git::GitBackend`]):
//!
//! - **D2 (ancestralidade).** O `merge-base --is-ancestor` (linha 1141-1145)
//!   é pulado: toda base resolvível por `rev_parse` é aceita.
//! - **D3 (estabilidade do HEAD).** A rechecagem final do HEAD (linhas
//!   1324-1326) é pulada: sem relógio transacional na biblioteca, a
//!   verificação seria apenas decorativa.
//! - **D4 (`is_source_file`).** O conjunto de extensões é o do Python
//!   (linhas 964-967), mais amplo que [`crate::catalog::is_source_file`].
//! - **D5 (relógio).** `audit.py` não carimba tempo no resultado; o
//!   parâmetro `now` existe por compatibilidade de assinatura e é ignorado.
//! - **D6 (lock com forma inválida).** O Python trata como `UsageError`;
//!   aqui um lock sem `categories` é [`AuditError::Config`], e lock ausente
//!   ou com JSON inválido continua [`AuditError::Usage`].
//! - **D7 (saída humana).** O modo texto imprime os achados *efetivos*
//!   (pós-allowlist); o Python imprime a lista completa pré-allowlist.
//!   JSON, estatísticas e códigos de saída são idênticos.
//! - **D8 (allowlist tolerante).** Linha com espaços (`" 12 "`) casa, onde o
//!   Python exige dígitos exatos (`isdigit` no token cru); negativos nunca
//!   casam em nenhum dos dois. Mantido por ser estritamente mais permissivo
//!   sem mudar nenhum caso real.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use jx_core::{BrError, FileSystem};
use jx_git::{GitBackend, GitError, Sha40, StatusEntry};
use serde_json::Value as JsonValue;
use thiserror::Error;

use crate::audit::{
    analyze_english_source, analyze_lang_key_mismatch, analyze_overlay_values_with_glossary,
    analyze_pair, audit_incomplete_finding, glossary_missing_finding, out_of_scope_finding,
    translate_lang_file_finding, AnalyzeOptions, NlpConfig, MAX_FINDINGS_PER_FILE,
};
use crate::catalog::matches_any;
use crate::core::{Finding, Severity};
use crate::lang::{EmbeddedZipf, GlossaryEntry, LinguaDetector, NlpOptions};
use crate::php::{compare_lang_entries, extract_php_lang_entries, LangEntry};
use crate::scan::{scan_source, source_kind};
use crate::text::normalize_newlines;

/// Caminhos de entrada da auditoria.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditPaths {
    /// Raiz do repositório (dona de `overlays/` e `i18n/`).
    pub root: PathBuf,
    /// Checkout auditado (upstream com a tradução aplicada).
    pub workspace_dir: PathBuf,
    /// Trava do catálogo (`i18n/catalog.lock.json`).
    pub catalog_lock: PathBuf,
    /// Glossário (`i18n/glossary.tsv`).
    pub glossary_tsv: PathBuf,
    /// Allowlist (`i18n/audit-allowlist.tsv`).
    pub allowlist_tsv: PathBuf,
}

/// Opções da auditoria (espelha `AuditOptions` de `audit.py`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AuditOptions {
    /// Unidade a auditar (exclusiva com `category`).
    pub unit: Option<String>,
    /// Categoria a auditar (exclusiva com `unit`).
    pub category: Option<String>,
    /// Revisão git de comparação (padrão `HEAD` em árvore suja).
    pub base: Option<String>,
    /// Aceito por compatibilidade; o gate sempre falha com qualquer achado.
    pub strict: bool,
}

/// Contagens por severidade (chaves iguais às do `--json` do `audit.sh`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SeverityCounts {
    /// Achados `blocker`.
    pub blockers: usize,
    /// Achados `major`.
    pub major: usize,
    /// Achados `minor`.
    pub minor: usize,
    /// Achados `nit`.
    pub nit: usize,
}

/// Relatório da auditoria (campos do documento `--json` do `audit.sh`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditReport {
    /// Verdadeiro quando não há achados após a allowlist.
    pub ok: bool,
    /// Verdadeiro quando não há `blocker`/`major` após a allowlist.
    pub blocking_ok: bool,
    /// Modo de escopo (`unit` ou `category`, chave dinâmica do JSON).
    pub mode: String,
    /// Alvo auditado (id da unidade ou da categoria).
    pub target: String,
    /// Revisão pedida (`--base` ou `HEAD`).
    pub base: String,
    /// SHA resolvido da base.
    pub base_sha: String,
    /// HEAD do workspace no início da auditoria.
    pub workspace_head: String,
    /// Arquivos verificados (inclui fora de escopo e lang).
    pub files_checked: usize,
    /// Arquivos efetivamente analisados.
    pub files_analyzed: usize,
    /// Chaves de lang comparadas (comuns + faltantes).
    pub keys_compared: usize,
    /// Falso quando o glossário está ausente.
    pub glossary_loaded: bool,
    /// Verdadeiro quando algum arquivo estourou o teto de achados.
    pub truncated: bool,
    /// Achados removidos pela allowlist.
    pub allowed: usize,
    /// Achados efetivos (após a allowlist).
    pub findings: Vec<Finding>,
}

impl AuditReport {
    /// Conta os achados efetivos por severidade.
    pub fn severity_counts(&self) -> SeverityCounts {
        let mut counts = SeverityCounts::default();
        for finding in &self.findings {
            match finding.severity() {
                Severity::Blocker => counts.blockers += 1,
                Severity::Major => counts.major += 1,
                Severity::Minor => counts.minor += 1,
                Severity::Nit => counts.nit += 1,
            }
        }
        counts
    }

    /// Serializa o relatório com as chaves exatas do `--json` do `audit.sh`.
    ///
    /// Chaves: `ok`, `blocking_ok`, `unit`|`category`, `base`, `base_sha`,
    /// `workspace_head`, `files_checked`, `files_analyzed`, `keys_compared`,
    /// `glossary_loaded`, `truncated`, `allowed`, `findings` (com
    /// `severity`/`code`/`file`/`line`/`problem`/`fix`) e `stats` (com
    /// `blockers`/`major`/`minor`/`nit`).
    pub fn to_json_value(&self) -> JsonValue {
        let counts = self.severity_counts();
        let findings: Vec<JsonValue> = self
            .findings
            .iter()
            .map(|finding| {
                serde_json::json!({
                    "severity": finding.severity().as_str(),
                    "code": finding.code(),
                    "file": finding.file(),
                    "line": finding.line(),
                    "problem": finding.problem(),
                    "fix": finding.fix(),
                })
            })
            .collect();
        let mut map = serde_json::Map::new();
        map.insert("ok".to_string(), JsonValue::Bool(self.ok));
        map.insert("blocking_ok".to_string(), JsonValue::Bool(self.blocking_ok));
        map.insert(self.mode.clone(), JsonValue::String(self.target.clone()));
        map.insert("base".to_string(), JsonValue::String(self.base.clone()));
        map.insert(
            "base_sha".to_string(),
            JsonValue::String(self.base_sha.clone()),
        );
        map.insert(
            "workspace_head".to_string(),
            JsonValue::String(self.workspace_head.clone()),
        );
        map.insert(
            "files_checked".to_string(),
            JsonValue::from(self.files_checked as u64),
        );
        map.insert(
            "files_analyzed".to_string(),
            JsonValue::from(self.files_analyzed as u64),
        );
        map.insert(
            "keys_compared".to_string(),
            JsonValue::from(self.keys_compared as u64),
        );
        map.insert(
            "glossary_loaded".to_string(),
            JsonValue::Bool(self.glossary_loaded),
        );
        map.insert("truncated".to_string(), JsonValue::Bool(self.truncated));
        map.insert("allowed".to_string(), JsonValue::from(self.allowed as u64));
        map.insert("findings".to_string(), JsonValue::Array(findings));
        map.insert(
            "stats".to_string(),
            serde_json::json!({
                "blockers": counts.blockers as u64,
                "major": counts.major as u64,
                "minor": counts.minor as u64,
                "nit": counts.nit as u64,
            }),
        );
        JsonValue::Object(map)
    }
}

/// Erros da orquestração da auditoria.
#[derive(Debug, Error)]
pub enum AuditError {
    /// Estrutura de configuração inválida (lock sem `categories`).
    #[error("{0}")]
    Config(String),
    /// Falha de git.
    #[error(transparent)]
    Git(#[from] GitError),
    /// Falha de I/O anotada com o caminho.
    #[error(transparent)]
    Repo(#[from] BrError),
    /// Uso inválido (unidade/categoria/base), espelha `UsageError`.
    #[error("{0}")]
    Usage(String),
}

/// Unidade do lock (arquivos + regras de posse).
#[derive(Debug, Clone, Default)]
struct LockUnit {
    /// Identificador.
    id: String,
    /// Arquivos atribuídos.
    files: Vec<String>,
    /// Globs de posse.
    globs: Vec<String>,
    /// Exclusões de posse.
    exclude: Vec<String>,
}

/// Extrai a lista de strings de um campo (ausente ou malformado vira vazio).
fn str_list(value: Option<&JsonValue>) -> Vec<String> {
    value
        .and_then(JsonValue::as_array)
        .map_or_else(Vec::new, |items| {
            items
                .iter()
                .filter_map(JsonValue::as_str)
                .map(str::to_string)
                .collect()
        })
}

/// Tabelas do lock: categorias com unidades + regras de `excluded`.
type LockTables = (Vec<(String, Vec<LockUnit>)>, Vec<(String, Vec<String>)>);

/// Lê as categorias/unidades e as regras de `excluded` do lock.
fn lock_tables(lock: &JsonValue, lock_path: &Path) -> Result<LockTables, AuditError> {
    let categories = lock
        .get("categories")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| {
            AuditError::Config(format!(
                "catalog lock sem categories: {}",
                lock_path.display()
            ))
        })?;
    let mut tables = Vec::new();
    for category in categories {
        let Some(map) = category.as_object() else {
            continue;
        };
        let Some(category_id) = map.get("id").and_then(JsonValue::as_str) else {
            continue;
        };
        let mut units = Vec::new();
        if let Some(raw_units) = map.get("units").and_then(JsonValue::as_array) {
            for raw_unit in raw_units {
                let Some(unit_map) = raw_unit.as_object() else {
                    continue;
                };
                let Some(unit_id) = unit_map.get("id").and_then(JsonValue::as_str) else {
                    continue;
                };
                if unit_id.is_empty() {
                    continue;
                }
                units.push(LockUnit {
                    id: unit_id.to_string(),
                    files: str_list(unit_map.get("files")),
                    globs: str_list(unit_map.get("globs")),
                    exclude: str_list(unit_map.get("exclude")),
                });
            }
        }
        tables.push((category_id.to_string(), units));
    }
    let mut excluded = Vec::new();
    if let Some(raw_excluded) = lock.get("excluded").and_then(JsonValue::as_array) {
        for entry in raw_excluded {
            let Some(map) = entry.as_object() else {
                continue;
            };
            let glob = map
                .get("glob")
                .and_then(JsonValue::as_str)
                .unwrap_or("")
                .to_string();
            excluded.push((glob, str_list(map.get("files"))));
        }
    }
    Ok((tables, excluded))
}

/// Posse de arquivos (espelha `build_ownership` de `audit.py`).
#[derive(Debug, Default)]
struct Ownership {
    /// Arquivo exato -> unidade (primeira vence).
    exact: HashMap<String, String>,
    /// Regras de glob por unidade, na ordem do lock (`id`, `globs`, `exclude`).
    matchers: Vec<(String, Vec<String>, Vec<String>)>,
    /// Arquivos excluídos.
    excluded_files: HashSet<String>,
    /// Globs excluídos.
    excluded_globs: Vec<String>,
}

impl Ownership {
    /// Devolve a unidade dona, `"excluded"` ou `None` (fora de qualquer regra).
    fn owner_of(&self, file: &str) -> Option<String> {
        if let Some(unit) = self.exact.get(file) {
            return Some(unit.clone());
        }
        for (unit, globs, exclude) in &self.matchers {
            if matches_any(file, globs) && !matches_any(file, exclude) {
                return Some(unit.clone());
            }
        }
        if self.excluded_files.contains(file) || matches_any(file, &self.excluded_globs) {
            return Some("excluded".to_string());
        }
        None
    }
}

/// Monta a posse a partir das tabelas do lock.
fn build_ownership(
    tables: &[(String, Vec<LockUnit>)],
    excluded: &[(String, Vec<String>)],
) -> Ownership {
    let mut ownership = Ownership::default();
    for (_, units) in tables {
        for unit in units {
            for file in &unit.files {
                ownership
                    .exact
                    .entry(file.clone())
                    .or_insert_with(|| unit.id.clone());
            }
            ownership
                .matchers
                .push((unit.id.clone(), unit.globs.clone(), unit.exclude.clone()));
        }
    }
    for (glob, files) in excluded {
        if !glob.is_empty() {
            ownership.excluded_globs.push(glob.clone());
        }
        for file in files {
            ownership.excluded_files.insert(file.clone());
        }
    }
    ownership
}

/// Informa se é arquivo-fonte varrível (conjunto do Python, ver D4).
fn is_source_file(name: &str) -> bool {
    const SUFFIXES: [&str; 14] = [
        ".blade.php",
        ".tsx",
        ".ts",
        ".jsx",
        ".js",
        ".mjs",
        ".cjs",
        ".php",
        ".html",
        ".htm",
        ".vue",
        ".svelte",
        ".css",
        ".scss",
    ];
    let lower = name.to_lowercase();
    SUFFIXES.iter().any(|suffix| lower.ends_with(suffix))
}

/// Informa se é arquivo de teste (`tests/` + extensão de código).
fn is_test_file(name: &str) -> bool {
    if !name.starts_with("tests/") {
        return false;
    }
    let lower = name.to_lowercase();
    const SUFFIXES: [&str; 8] = [
        ".tsx",
        ".ts",
        ".jsx",
        ".js",
        ".mjs",
        ".cjs",
        ".php",
        ".blade.php",
    ];
    SUFFIXES.iter().any(|suffix| lower.ends_with(suffix))
}

/// Informa se é arquivo de lang inglesa.
fn is_english_lang_file(name: &str) -> bool {
    name.starts_with("resources/lang/en/") && name.ends_with(".php")
}

/// Contrapartes pt-BR de um arquivo en (`overlays/` primeiro).
fn counterpart_paths(english_file: &str) -> Vec<String> {
    const PREFIX: &str = "resources/lang/en/";
    if !english_file.starts_with(PREFIX) {
        return Vec::new();
    }
    let relative = &english_file[PREFIX.len()..];
    vec![
        format!("overlays/resources/lang/pt-BR/{relative}"),
        format!("resources/lang/pt-BR/{relative}"),
    ]
}

/// Resolve a contraparte existente (`overlays/` sob a raiz, resto no workspace).
fn resolve_counterpart<FS: FileSystem>(
    fs: &FS,
    root: &Path,
    workspace: &Path,
    english_file: &str,
) -> Option<(String, PathBuf)> {
    for relative in counterpart_paths(english_file) {
        let absolute = if relative.starts_with("overlays/") {
            root.join(&relative)
        } else {
            workspace.join(&relative)
        };
        if fs.exists(&absolute) {
            return Some((relative, absolute));
        }
    }
    None
}

/// Lê o arquivo do worktree (`""` quando ausente, como `worktree_content`).
fn worktree_text<FS: FileSystem>(fs: &FS, path: &Path) -> String {
    fs.read_to_string(path).unwrap_or_default()
}

/// Exibe o caminho relativo à raiz (`/` em qualquer plataforma).
fn display_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map_or_else(|_| path.to_path_buf(), Path::to_path_buf)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Extrai o glossário do TSV (pula a linha de cabeçalho `en`).
fn parse_glossary(text: &str) -> Vec<GlossaryEntry> {
    let mut entries = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let columns: Vec<&str> = line.split('\t').collect();
        let english = columns.first().copied().unwrap_or("");
        if english.is_empty() || english == "en" {
            continue;
        }
        let portuguese = columns.get(1).copied().unwrap_or("");
        let notes = if columns.len() > 2 {
            columns[2..].join(" ").trim().to_string()
        } else {
            String::new()
        };
        entries.push(GlossaryEntry::new(english, portuguese, &notes));
    }
    entries
}

/// Entrada da allowlist (`arquivo`, `linha`, `código` exatos).
#[derive(Debug, Clone, PartialEq, Eq)]
struct AllowEntry {
    /// Arquivo relativo.
    file: String,
    /// Linha (0 quando ilegível; nunca casa com achados).
    line: u32,
    /// Código da regra.
    code: String,
}

/// Extrai a allowlist do TSV (`#` comenta a linha).
fn parse_allowlist(text: &str) -> Vec<AllowEntry> {
    let mut entries = Vec::new();
    for line in text.lines() {
        let stripped = line.trim();
        if stripped.is_empty() || stripped.starts_with('#') {
            continue;
        }
        let columns: Vec<&str> = line.split('\t').collect();
        let file = columns.first().copied().unwrap_or("").to_string();
        let line_number = columns
            .get(1)
            .and_then(|raw| raw.trim().parse::<u32>().ok())
            .unwrap_or(0);
        let code = columns.get(2).copied().unwrap_or("").to_string();
        entries.push(AllowEntry {
            file,
            line: line_number,
            code,
        });
    }
    entries
}

/// Informa se a chave existe no índice de lang (com fallback de namespace).
fn lang_key_exists(index: &HashMap<String, Vec<LangEntry>>, raw_key: &str) -> bool {
    let mut key = raw_key.to_string();
    for prefix in ["lang::", "lang."] {
        if let Some(rest) = key.strip_prefix(prefix) {
            key = rest.to_string();
        }
    }
    while let Some(rest) = key.strip_prefix('/') {
        key = rest.to_string();
    }
    if key.is_empty() {
        return true;
    }
    for (file, entries) in index {
        if entries.iter().any(|entry| entry.key == key) {
            return true;
        }
        let mut namespace = file
            .strip_prefix("resources/lang/en/")
            .unwrap_or(file)
            .to_string();
        if let Some(stripped) = namespace.strip_suffix(".php") {
            namespace = stripped.to_string();
        }
        let namespace = namespace.replace('/', ".");
        if let Some(rest) = key.strip_prefix(&format!("{namespace}.")) {
            if entries.iter().any(|entry| entry.key == rest) {
                return true;
            }
        }
    }
    false
}

/// Executa a auditoria da unidade/categoria (port de `audit()` de `audit.py`).
///
/// `now` é aceito por compatibilidade de assinatura e ignorado (ver D5): o
/// documento `--json` do Python não carimba tempo.
pub fn run_audit<G: GitBackend, FS: FileSystem>(
    git: &G,
    fs: &FS,
    paths: &AuditPaths,
    opts: &AuditOptions,
    now: &str,
) -> Result<AuditReport, AuditError> {
    let _ = now;
    if opts.unit.is_some() == opts.category.is_some() {
        if opts.unit.is_some() {
            return Err(AuditError::Usage(
                "use --unit ou --category, nao os dois".to_string(),
            ));
        }
        return Err(AuditError::Usage(
            "informe --unit <id> ou --category <id>".to_string(),
        ));
    }

    // Workspace antes do lock, como no Python (linhas ~1082-1091).
    if !fs.exists(&paths.workspace_dir) {
        return Err(AuditError::Usage(format!(
            "workspace nao encontrado em {}; rode `jx setup`",
            paths.workspace_dir.display()
        )));
    }
    if !git.is_git_repo(&paths.workspace_dir) {
        return Err(AuditError::Usage(format!(
            "o workspace nao e um repositorio git: {}",
            paths.workspace_dir.display()
        )));
    }

    if !fs.exists(&paths.catalog_lock) {
        return Err(AuditError::Usage(format!(
            "catalog lock ausente: {} (rode `jx i18n-catalog --write`)",
            paths.catalog_lock.display()
        )));
    }
    let lock_text = fs.read_to_string(&paths.catalog_lock)?;
    let lock: JsonValue = serde_json::from_str(&lock_text).map_err(|err| {
        AuditError::Usage(format!(
            "catalog lock invalido: {} ({err})",
            paths.catalog_lock.display()
        ))
    })?;
    let (tables, excluded_rules) = lock_tables(&lock, &paths.catalog_lock)?;

    let (mode, target, units) = if let Some(unit_id) = opts.unit.as_deref() {
        let mut found: Option<Vec<LockUnit>> = None;
        for (_, lock_units) in &tables {
            for unit in lock_units {
                if unit.id == unit_id {
                    found = Some(vec![unit.clone()]);
                    break;
                }
            }
            if found.is_some() {
                break;
            }
        }
        let Some(resolved) = found else {
            return Err(AuditError::Usage(format!(
                "unidade desconhecida: {unit_id}"
            )));
        };
        ("unit", unit_id.to_string(), resolved)
    } else if let Some(category_id) = opts.category.as_deref() {
        let mut found: Option<Vec<LockUnit>> = None;
        for (locked_id, lock_units) in &tables {
            if locked_id == category_id {
                found = Some(lock_units.clone());
                break;
            }
        }
        let Some(resolved) = found else {
            return Err(AuditError::Usage(format!(
                "categoria desconhecida: {category_id}"
            )));
        };
        ("category", category_id.to_string(), resolved)
    } else {
        return Err(AuditError::Usage(
            "informe --unit <id> ou --category <id>".to_string(),
        ));
    };

    let unit_ids: HashSet<String> = units.iter().map(|unit| unit.id.clone()).collect();
    let ownership = build_ownership(&tables, &excluded_rules);
    let unit_files: Vec<String> = {
        let mut files = Vec::new();
        for unit in &units {
            files.extend(unit.files.iter().cloned());
        }
        files.sort();
        files.dedup();
        files
    };
    let mut audited_paths: HashSet<String> = unit_files.iter().cloned().collect();
    let mut counterpart_of: HashMap<String, String> = HashMap::new();
    for file in &unit_files {
        for relative in counterpart_paths(file) {
            counterpart_of.insert(relative.clone(), file.clone());
            audited_paths.insert(relative);
        }
    }

    let statuses = git.status(&paths.workspace_dir)?;
    let dirty = !statuses.is_empty();
    let Some(explicit_base) = opts.base.as_deref() else {
        if !dirty {
            return Err(AuditError::Usage(
                "working tree limpo: informe --base <git-rev> ou defina BR_I18N_BASE para auditar trabalho commitado"
                    .to_string(),
            ));
        }
        // Árvore suja sem base explícita: o padrão do Python é `HEAD`.
        return run_with_base(
            git,
            fs,
            paths,
            &tables,
            &unit_ids,
            &ownership,
            &unit_files,
            &audited_paths,
            &counterpart_of,
            &statuses,
            mode,
            &target,
            "HEAD",
        );
    };
    run_with_base(
        git,
        fs,
        paths,
        &tables,
        &unit_ids,
        &ownership,
        &unit_files,
        &audited_paths,
        &counterpart_of,
        &statuses,
        mode,
        &target,
        explicit_base,
    )
}

/// Núcleo de [`run_audit`] com a base já escolhida (nome ou `HEAD`).
#[allow(clippy::too_many_arguments)]
fn run_with_base<G: GitBackend, FS: FileSystem>(
    git: &G,
    fs: &FS,
    paths: &AuditPaths,
    tables: &[(String, Vec<LockUnit>)],
    unit_ids: &HashSet<String>,
    ownership: &Ownership,
    unit_files: &[String],
    audited_paths: &HashSet<String>,
    counterpart_of: &HashMap<String, String>,
    statuses: &[StatusEntry],
    mode: &str,
    target: &str,
    base: &str,
) -> Result<AuditReport, AuditError> {
    let base_sha: Sha40 = git
        .rev_parse(&paths.workspace_dir, base)
        .map_err(|_| AuditError::Usage(format!("base git invalida: {base}")))?;
    let base_sha = base_sha.hex();
    let workspace_head = git.head_rev(&paths.workspace_dir)?;

    let untracked: HashSet<String> = statuses
        .iter()
        .filter(|entry| entry.index == '?' && entry.worktree == '?')
        .map(|entry| entry.path.clone())
        .collect();
    // Arquivos alterados contra a base (git) mais untracked (status).
    let mut all_changed: Vec<String> = git.diff_name_only(&paths.workspace_dir, &base_sha)?;
    all_changed.extend(untracked.iter().cloned());
    all_changed.sort();
    all_changed.dedup();
    let mut scoped: HashSet<String> = all_changed
        .iter()
        .filter(|name| audited_paths.contains(name.as_str()))
        .cloned()
        .collect();
    let mut out_of_scope: Vec<String> = Vec::new();
    for name in &all_changed {
        if scoped.contains(name) {
            continue;
        }
        match ownership.owner_of(name) {
            Some(owner) if unit_ids.contains(&owner) => {
                scoped.insert(name.clone());
            }
            _ if is_test_file(name) => {
                scoped.insert(name.clone());
            }
            Some(owner) if owner == "excluded" => {
                out_of_scope.push(name.clone());
            }
            None => {
                out_of_scope.push(name.clone());
            }
            Some(_) => {}
        }
    }

    let (glossary, glossary_missing) = if fs.exists(&paths.glossary_tsv) {
        (
            parse_glossary(&fs.read_to_string(&paths.glossary_tsv)?),
            false,
        )
    } else {
        (Vec::new(), true)
    };

    let mut checked: HashSet<String> = HashSet::new();
    let mut analyzed: HashSet<String> = HashSet::new();
    let mut findings: Vec<Finding> = Vec::new();
    let mut truncated = false;
    let mut keys_compared: usize = 0;

    if glossary_missing {
        if let Some(finding) =
            glossary_missing_finding(&display_relative(&paths.root, &paths.glossary_tsv))
        {
            findings.push(finding);
        }
    }

    out_of_scope.sort();
    for name in &out_of_scope {
        checked.insert(name.clone());
        if let Some(finding) = out_of_scope_finding(name) {
            findings.push(finding);
        }
    }

    let mut lang_keys: Vec<(String, String, u32)> = Vec::new();
    let mut scoped_sorted: Vec<&String> = scoped.iter().collect();
    scoped_sorted.sort();
    for name in scoped_sorted {
        if counterpart_of.contains_key(name.as_str()) {
            continue;
        }
        if !is_source_file(name) {
            checked.insert(name.clone());
            continue;
        }
        checked.insert(name.clone());
        analyzed.insert(name.clone());
        let new_text = worktree_text(fs, &paths.workspace_dir.join(name));
        // Conteudo da base via `git show`; untracked/ausente vira "".
        let old_text = if untracked.contains(name) {
            String::new()
        } else {
            git.show_file(&paths.workspace_dir, &base_sha, name)?
                .unwrap_or_default()
        };
        if is_english_lang_file(name) {
            if normalize_newlines(&old_text) != normalize_newlines(&new_text) {
                findings.extend(analyze_english_source(&old_text, &new_text, name));
            }
            continue;
        }
        let options = AnalyzeOptions {
            file: (*name).clone(),
            kind: source_kind(Some(name)).as_str().to_string(),
            glossary: glossary.clone(),
            test_file: is_test_file(name),
            lang_file: false,
            max_findings: MAX_FINDINGS_PER_FILE,
            nlp: NlpConfig::default(),
        };
        let batch = analyze_pair(&old_text, &new_text, &options);
        if batch.iter().any(|f| f.code() == "findings-truncated") {
            truncated = true;
        }
        findings.extend(batch);
        if !new_text.is_empty() {
            let scan = scan_source(
                &new_text,
                source_kind(Some(name)).as_str(),
                name,
                false,
                is_test_file(name),
            );
            for key in scan.lang_keys {
                lang_keys.push(((*name).clone(), key.key, key.line));
            }
        }
    }

    // Índice de lang de todas as unidades do lock (`build_lang_index`).
    let mut lang_index: HashMap<String, Vec<LangEntry>> = HashMap::new();
    for (_, lock_units) in tables {
        for unit in lock_units {
            for file in &unit.files {
                if !is_english_lang_file(file) || lang_index.contains_key(file) {
                    continue;
                }
                let text = worktree_text(fs, &paths.workspace_dir.join(file));
                if text.is_empty() {
                    continue;
                }
                lang_index.insert(file.clone(), extract_php_lang_entries(&text));
            }
        }
    }

    // Overlays pt-BR de todos os arquivos en da unidade.
    for english_file in unit_files.iter().filter(|f| is_english_lang_file(f)) {
        checked.insert(english_file.clone());
        let english_text = worktree_text(fs, &paths.workspace_dir.join(english_file));
        if english_text.is_empty() {
            continue;
        }
        analyzed.insert(english_file.clone());
        let Some((counterpart_rel, counterpart_abs)) =
            resolve_counterpart(fs, &paths.root, &paths.workspace_dir, english_file)
        else {
            let overlay_path = counterpart_paths(english_file)
                .first()
                .cloned()
                .unwrap_or_default();
            if let Ok(finding) = Finding::new(
                Severity::Major,
                "lang-key-mismatch",
                english_file,
                1,
                &format!("Arquivo pt-BR ausente para {english_file}."),
                &format!("Crie {overlay_path} com as mesmas chaves do arquivo em ingles."),
            ) {
                findings.push(finding);
            }
            continue;
        };
        checked.insert(counterpart_rel.clone());
        analyzed.insert(counterpart_rel.clone());
        let portuguese_text = worktree_text(fs, &counterpart_abs);
        let comparison = compare_lang_entries(&english_text, &portuguese_text);
        keys_compared += comparison.keys_compared + comparison.missing.len();
        findings.extend(analyze_overlay_values_with_glossary(
            &comparison.common,
            &counterpart_rel,
            &glossary,
            &NlpOptions::default(),
            EmbeddedZipf::global(),
            LinguaDetector::global(),
        ));
        findings.extend(analyze_lang_key_mismatch(
            &comparison.missing,
            &comparison.extra,
            english_file,
            &counterpart_rel,
        ));
    }

    // Chaves de tradução sem origem (`translate-lang-file`).
    lang_keys.sort();
    let mut reported: HashSet<String> = HashSet::new();
    for (file, key, line) in &lang_keys {
        if lang_key_exists(&lang_index, key) {
            continue;
        }
        let marker = format!("{file}\x00{key}\x00{line}");
        if !reported.insert(marker) {
            continue;
        }
        if let Some(finding) = translate_lang_file_finding(file, *line, key) {
            findings.push(finding);
        }
    }

    // Mudanças rastreadas sem nada analisado (`audit-incomplete`): usa o
    // diff contra a base (igual ao Python), nao o status da worktree.
    let mut tracked_in_unit: Vec<String> = all_changed
        .iter()
        .filter(|name| !untracked.contains(*name))
        .filter(|name| unit_files.binary_search(name).is_ok())
        .cloned()
        .collect();
    tracked_in_unit.sort();
    if !tracked_in_unit.is_empty() && analyzed.is_empty() {
        if let Some(name) = tracked_in_unit.first() {
            if let Some(finding) = audit_incomplete_finding(name) {
                findings.push(finding);
            }
        }
    }

    // Allowlist: subtração exata por arquivo + linha + código.
    let allowlist = if fs.exists(&paths.allowlist_tsv) {
        parse_allowlist(&worktree_text(fs, &paths.allowlist_tsv))
    } else {
        Vec::new()
    };
    let mut allowed = 0_usize;
    let mut effective = Vec::new();
    for finding in findings {
        let hit = allowlist.iter().any(|entry| {
            entry.file == finding.file()
                && entry.line == finding.line()
                && entry.code == finding.code()
        });
        if hit {
            allowed += 1;
        } else {
            effective.push(finding);
        }
    }
    let mut counts = SeverityCounts::default();
    for finding in &effective {
        match finding.severity() {
            Severity::Blocker => counts.blockers += 1,
            Severity::Major => counts.major += 1,
            Severity::Minor => counts.minor += 1,
            Severity::Nit => counts.nit += 1,
        }
    }

    Ok(AuditReport {
        ok: effective.is_empty(),
        blocking_ok: counts.blockers + counts.major == 0,
        mode: mode.to_string(),
        target: target.to_string(),
        base: base.to_string(),
        base_sha,
        workspace_head,
        files_checked: checked.len(),
        files_analyzed: analyzed.len(),
        keys_compared,
        glossary_loaded: !glossary_missing,
        truncated,
        allowed,
        findings: effective,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use jx_core::MemFs;

    /// SHA fixo de 40 hex para o git fake.
    const HEAD_SHA: &str = "0123456789abcdef0123456789abcdef01234567";

    /// Backend git programado (status + HEAD + base resolúvel ou não).
    #[derive(Debug, Default)]
    struct FakeGit {
        /// Entradas de `status --porcelain`.
        status: Vec<StatusEntry>,
        /// Resposta de `head_rev`.
        head: String,
        /// Quando falso, `rev_parse` falha (base inválida).
        rev_ok: bool,
        /// Conteúdo da base por caminho (`git show`); ausente vira "".
        show: HashMap<String, String>,
        /// `diff --name-only` scriptado; `None` deriva do status (só rastreados).
        diff_override: Option<Vec<String>>,
        /// Quando verdadeiro, `is_git_repo` retorna falso.
        not_a_repo: bool,
    }

    impl FakeGit {
        /// SHA fixo quando a base é aceita.
        fn fixed_sha() -> Result<Sha40, GitError> {
            Sha40::parse(HEAD_SHA).map_err(GitError::from)
        }
    }

    impl GitBackend for FakeGit {
        fn status(&self, _dir: &Path) -> Result<Vec<StatusEntry>, GitError> {
            Ok(self.status.clone())
        }

        fn rev_parse(&self, _dir: &Path, _rev: &str) -> Result<Sha40, GitError> {
            if self.rev_ok {
                Self::fixed_sha()
            } else {
                Err(GitError::InvalidOutput("base invalida".to_string()))
            }
        }

        fn cat_file_exists(&self, _dir: &Path, _rev: &str, _path: &str) -> Result<bool, GitError> {
            Ok(false)
        }

        fn apply_check(&self, _dir: &Path, _patch: &Path, _reverse: bool) -> Result<(), GitError> {
            Ok(())
        }

        fn apply(&self, _dir: &Path, _patch: &Path) -> Result<(), GitError> {
            Ok(())
        }

        fn apply_reverse(&self, _dir: &Path, _patch: &Path) -> Result<(), GitError> {
            Ok(())
        }

        fn apply_3way(&self, _dir: &Path, _patch: &Path) -> Result<(), GitError> {
            Ok(())
        }

        fn ls_unmerged(&self, _dir: &Path) -> Result<Vec<String>, GitError> {
            Ok(Vec::new())
        }

        fn add_all(&self, _dir: &Path) -> Result<(), GitError> {
            Ok(())
        }

        fn commit(&self, _dir: &Path, _message: &str) -> Result<(), GitError> {
            Ok(())
        }

        fn reset_hard(&self, _dir: &Path, _rev: &str) -> Result<(), GitError> {
            Ok(())
        }

        fn reset_soft(&self, _dir: &Path, _rev: &str) -> Result<(), GitError> {
            Ok(())
        }

        fn reset_path(&self, _dir: &Path, _path: &str) -> Result<(), GitError> {
            Ok(())
        }

        fn clean(&self, _dir: &Path) -> Result<(), GitError> {
            Ok(())
        }

        fn checkout_detach(&self, _dir: &Path, _rev: &str) -> Result<(), GitError> {
            Ok(())
        }

        fn head_rev(&self, _dir: &Path) -> Result<String, GitError> {
            Ok(self.head.clone())
        }

        fn head_subject(&self, _dir: &Path) -> Result<String, GitError> {
            Ok(String::new())
        }

        fn head_parent_rev(&self, _dir: &Path) -> Result<Option<String>, GitError> {
            Ok(None)
        }

        fn log_subjects_range(&self, _dir: &Path, _range: &str) -> Result<Vec<String>, GitError> {
            Ok(Vec::new())
        }

        fn log_grep_commit(&self, _dir: &Path, _pattern: &str) -> Result<Option<String>, GitError> {
            Ok(None)
        }

        fn diff_cached(&self, _dir: &Path) -> Result<String, GitError> {
            Ok(String::new())
        }

        fn diff_cached_quiet(&self, _dir: &Path, _path: &str) -> Result<bool, GitError> {
            Ok(true)
        }

        fn diff_quiet_head(&self, _dir: &Path, _path: &str) -> Result<bool, GitError> {
            Ok(true)
        }

        fn ls_ignored_others(&self, _dir: &Path) -> Result<Vec<String>, GitError> {
            Ok(Vec::new())
        }

        fn clone_shared_no_checkout(&self, _src: &Path, _dst: &Path) -> Result<(), GitError> {
            Ok(())
        }

        fn clone_no_checkout(&self, _url: &str, _dst: &Path) -> Result<(), GitError> {
            Ok(())
        }

        fn fetch_tags(&self, _dir: &Path) -> Result<(), GitError> {
            Ok(())
        }

        fn git_config(&self, _dir: &Path, _key: &str) -> Result<Option<String>, GitError> {
            Ok(None)
        }

        fn remote_set_url(&self, _dir: &Path, _name: &str, _url: &str) -> Result<(), GitError> {
            Ok(())
        }

        fn ls_remote(
            &self,
            _dir: &Path,
            _url: &str,
            _pattern: &str,
        ) -> Result<Option<String>, GitError> {
            Ok(None)
        }

        fn tag_points_at(&self, _dir: &Path, _sha: &str) -> Result<Vec<String>, GitError> {
            Ok(Vec::new())
        }

        fn rev_parse_opt(&self, _dir: &Path, _rev: &str) -> Result<Option<Sha40>, GitError> {
            if self.rev_ok {
                Ok(Self::fixed_sha().ok())
            } else {
                Ok(None)
            }
        }

        fn reset_hard_head(&self, _dir: &Path) -> Result<(), GitError> {
            Ok(())
        }

        fn is_git_repo(&self, _dir: &Path) -> bool {
            !self.not_a_repo
        }

        fn show_file(
            &self,
            _dir: &Path,
            _rev: &str,
            path: &str,
        ) -> Result<Option<String>, GitError> {
            Ok(self.show.get(path).cloned())
        }

        fn diff_name_only(&self, _dir: &Path, _rev: &str) -> Result<Vec<String>, GitError> {
            if let Some(names) = &self.diff_override {
                let mut sorted = names.clone();
                sorted.sort();
                sorted.dedup();
                return Ok(sorted);
            }
            let mut names: Vec<String> = self
                .status
                .iter()
                .filter(|entry| !(entry.index == '?' && entry.worktree == '?'))
                .map(|entry| entry.path.clone())
                .collect();
            names.sort();
            names.dedup();
            Ok(names)
        }

        fn archive_extract(&self, _dir: &Path, _rev: &str, _dest: &Path) -> Result<(), GitError> {
            Ok(())
        }
    }

    /// Entrada rastreada de `status --porcelain` (` M <path>`).
    fn tracked(path: &str) -> StatusEntry {
        StatusEntry {
            index: ' ',
            worktree: 'M',
            path: path.to_string(),
            renamed_from: None,
        }
    }

    /// Caminhos padrão dos testes (`/r` raiz, `/w` workspace).
    fn test_paths() -> AuditPaths {
        AuditPaths {
            root: PathBuf::from("/r"),
            workspace_dir: PathBuf::from("/w"),
            catalog_lock: PathBuf::from("/r/i18n/catalog.lock.json"),
            glossary_tsv: PathBuf::from("/r/i18n/glossary.tsv"),
            allowlist_tsv: PathBuf::from("/r/i18n/audit-allowlist.tsv"),
        }
    }

    /// Lock mínimo com uma unidade.
    fn lock_with_unit(unit_id: &str, files: &[&str], globs: &[&str]) -> String {
        let files_json = files
            .iter()
            .map(|f| format!("\"{f}\""))
            .collect::<Vec<_>>()
            .join(",");
        let globs_json = globs
            .iter()
            .map(|g| format!("\"{g}\""))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"categories\":[{{\"id\":\"cat-a\",\"units\":[{{\"id\":\"{unit_id}\",\"files\":[{files_json}],\"globs\":[{globs_json}],\"exclude\":[]}}]}}],\"excluded\":[]}}"
        )
    }

    /// Git sujo com HEAD fixo e base resolúvel.
    fn dirty_git(status: Vec<StatusEntry>, show: &[(&str, &str)]) -> FakeGit {
        FakeGit {
            status,
            head: HEAD_SHA.to_string(),
            rev_ok: true,
            show: show
                .iter()
                .map(|(path, text)| (path.to_string(), text.to_string()))
                .collect(),
            diff_override: None,
            not_a_repo: false,
        }
    }

    /// Opções de unidade sem base explícita.
    fn unit_opts(unit: &str) -> AuditOptions {
        AuditOptions {
            unit: Some(unit.to_string()),
            category: None,
            base: None,
            strict: false,
        }
    }

    /// Executa a auditoria e desembrulha o `Ok` (falha o teste no `Err`).
    fn run_ok(git: &FakeGit, fs: &MemFs, paths: &AuditPaths, opts: &AuditOptions) -> AuditReport {
        match run_audit(git, fs, paths, opts, "2026-10-27T00:00:00Z") {
            Ok(report) => report,
            Err(err) => panic!("auditoria deveria rodar: {err}"),
        }
    }

    #[test]
    fn glossary_missing_yields_major_finding() {
        let fs = MemFs::default()
            .with_file(
                "/r/i18n/catalog.lock.json",
                &lock_with_unit("u1", &["a.ts"], &["*.ts"]),
            )
            .with_file("/w/a.ts", "export const x = 1;\n");
        let git = dirty_git(vec![tracked("a.ts")], &[("a.ts", "export const x = 1;\n")]);
        let report = run_ok(&git, &fs, &test_paths(), &unit_opts("u1"));
        assert!(!report.glossary_loaded);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].code(), "glossary-missing");
        assert_eq!(report.findings[0].severity(), Severity::Major);
        assert!(!report.ok);
        assert!(!report.blocking_ok);
    }

    #[test]
    fn clean_tree_without_base_is_usage_error() {
        let fs = MemFs::default()
            .with_file(
                "/r/i18n/catalog.lock.json",
                &lock_with_unit("u1", &["a.ts"], &["*.ts"]),
            )
            .with_file("/w/.keep", "");
        let git = FakeGit {
            status: Vec::new(),
            head: HEAD_SHA.to_string(),
            rev_ok: true,
            show: HashMap::new(),
            diff_override: None,
            not_a_repo: false,
        };
        match run_audit(&git, &fs, &test_paths(), &unit_opts("u1"), "") {
            Err(AuditError::Usage(message)) => {
                assert!(
                    message.contains("working tree limpo"),
                    "inesperado: {message}"
                );
            }
            Err(other) => panic!("erro errado: {other}"),
            Ok(_) => panic!("arvore limpa sem base deveria falhar"),
        }

        // Unidade e categoria juntas, ou nenhuma, também são erro de uso.
        let both = AuditOptions {
            unit: Some("u1".to_string()),
            category: Some("cat-a".to_string()),
            base: Some("HEAD".to_string()),
            strict: false,
        };
        assert!(matches!(
            run_audit(&git, &fs, &test_paths(), &both, ""),
            Err(AuditError::Usage(_))
        ));
        let neither = AuditOptions::default();
        assert!(matches!(
            run_audit(
                &dirty_git(vec![tracked("a.ts")], &[]),
                &fs,
                &test_paths(),
                &neither,
                ""
            ),
            Err(AuditError::Usage(_))
        ));
    }

    #[test]
    fn workspace_checks_come_before_lock_and_unit_errors() {
        let fs = MemFs::default();
        let mut git = dirty_git(vec![], &[]);
        git.not_a_repo = true;
        // Sem workspace no FS: erro de workspace, nao de lock ausente.
        match run_audit(&git, &fs, &test_paths(), &unit_opts("u1"), "") {
            Err(AuditError::Usage(message)) => {
                assert!(
                    message.contains("workspace nao encontrado"),
                    "inesperado: {message}"
                );
            }
            Err(other) => panic!("erro errado: {other}"),
            Ok(_) => panic!("sem workspace deveria falhar"),
        }
        // Workspace existe mas nao e repo git: erro proprio, nao de lock.
        let fs = MemFs::default().with_file("/w/.keep", "");
        match run_audit(&git, &fs, &test_paths(), &unit_opts("u1"), "") {
            Err(AuditError::Usage(message)) => {
                assert!(
                    message.contains("nao e um repositorio git"),
                    "inesperado: {message}"
                );
            }
            Err(other) => panic!("erro errado: {other}"),
            Ok(_) => panic!("sem repo git deveria falhar"),
        }
    }

    #[test]
    fn out_of_scope_file_is_blocker() {
        let fs = MemFs::default()
            .with_file(
                "/r/i18n/catalog.lock.json",
                &lock_with_unit("u1", &["a.ts"], &["src/**"]),
            )
            .with_file("/r/i18n/glossary.tsv", "en\tpt-BR\tnotes\n")
            .with_file("/w/.keep", "");
        let git = dirty_git(vec![tracked("other.ts")], &[]);
        let report = run_ok(&git, &fs, &test_paths(), &unit_opts("u1"));
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].code(), "out-of-scope");
        assert_eq!(report.findings[0].file(), "other.ts");
        assert_eq!(report.findings[0].severity(), Severity::Blocker);
        assert!(!report.ok);
        assert!(!report.blocking_ok);
    }

    #[test]
    fn overlay_placeholder_swap_and_allowlist_subtraction() {
        let lock = lock_with_unit(
            "u1",
            &["resources/lang/en/msg.php"],
            &["resources/lang/en/**"],
        );
        let english = "<?php return ['hello' => 'Hello :name'];\n";
        let portuguese = "<?php return ['hello' => 'Olá, tudo bem :nome'];\n";
        let fs = MemFs::default()
            .with_file("/r/i18n/catalog.lock.json", &lock)
            .with_file("/r/i18n/glossary.tsv", "en\tpt-BR\tnotes\n")
            .with_file("/w/resources/lang/en/msg.php", english)
            .with_file("/r/overlays/resources/lang/pt-BR/msg.php", portuguese);
        let git = dirty_git(
            vec![tracked("resources/lang/en/msg.php")],
            &[("resources/lang/en/msg.php", english)],
        );
        let paths = test_paths();
        let report = run_ok(&git, &fs, &paths, &unit_opts("u1"));
        let swapped: Vec<&Finding> = report
            .findings
            .iter()
            .filter(|f| f.code() == "placeholder-changed")
            .collect();
        assert_eq!(swapped.len(), 1);
        assert_eq!(swapped[0].severity(), Severity::Blocker);
        assert_eq!(swapped[0].file(), "overlays/resources/lang/pt-BR/msg.php");
        assert!(!report.ok);

        // Allowlist exata (arquivo + linha + código) remove o achado.
        let line = swapped[0].line();
        let fs_allowed = MemFs::default()
            .with_file("/r/i18n/catalog.lock.json", &lock)
            .with_file("/r/i18n/glossary.tsv", "en\tpt-BR\tnotes\n")
            .with_file("/w/resources/lang/en/msg.php", english)
            .with_file("/r/overlays/resources/lang/pt-BR/msg.php", portuguese)
            .with_file(
                "/r/i18n/audit-allowlist.tsv",
                &format!(
                    "# motivo\noverlays/resources/lang/pt-BR/msg.php\t{line}\tplaceholder-changed\tjustificado\n"
                ),
            );
        let rerun = run_ok(&git, &fs_allowed, &paths, &unit_opts("u1"));
        assert!(
            rerun
                .findings
                .iter()
                .all(|f| f.code() != "placeholder-changed"),
            "allowlist deveria remover: {:?}",
            rerun.findings
        );
        assert_eq!(rerun.allowed, 1);
        assert!(rerun.ok);
        assert!(rerun.blocking_ok);
    }

    #[test]
    fn json_shape_uses_exact_audit_keys() {
        let fs = MemFs::default()
            .with_file(
                "/r/i18n/catalog.lock.json",
                &lock_with_unit("u1", &["a.ts"], &["*.ts"]),
            )
            .with_file("/w/a.ts", "export const x = 1;\n");
        let git = dirty_git(vec![tracked("a.ts")], &[("a.ts", "export const x = 1;\n")]);
        let report = run_ok(&git, &fs, &test_paths(), &unit_opts("u1"));
        let value = report.to_json_value();
        let Some(map) = value.as_object() else {
            panic!("relatorio deveria ser objeto");
        };
        let mut keys: Vec<&String> = map.keys().collect();
        keys.sort();
        let expected = [
            "allowed",
            "base",
            "base_sha",
            "blocking_ok",
            "files_analyzed",
            "files_checked",
            "findings",
            "glossary_loaded",
            "keys_compared",
            "ok",
            "stats",
            "truncated",
            "unit",
            "workspace_head",
        ];
        assert_eq!(keys, expected);
        let Some(stats) = map.get("stats").and_then(JsonValue::as_object) else {
            panic!("stats deveria ser objeto");
        };
        let mut stat_keys: Vec<&String> = stats.keys().collect();
        stat_keys.sort();
        assert_eq!(stat_keys, ["blockers", "major", "minor", "nit"]);
        let Some(findings) = map.get("findings").and_then(JsonValue::as_array) else {
            panic!("findings deveria ser lista");
        };
        assert_eq!(findings.len(), 1);
        let Some(finding) = findings[0].as_object() else {
            panic!("achado deveria ser objeto");
        };
        let mut finding_keys: Vec<&String> = finding.keys().collect();
        finding_keys.sort();
        assert_eq!(
            finding_keys,
            ["code", "file", "fix", "line", "problem", "severity"]
        );
        assert_eq!(
            finding.get("severity").and_then(JsonValue::as_str),
            Some("major")
        );
    }

    #[test]
    fn audit_incomplete_uses_base_diff_not_worktree_status() {
        // Arvore limpa + base explicita: mudanca commitada (so no diff da
        // base) em arquivo nao-fonte da unidade gera audit-incomplete.
        let fs = MemFs::default()
            .with_file(
                "/r/i18n/catalog.lock.json",
                &lock_with_unit("u1", &["notes.txt"], &["*.txt"]),
            )
            .with_file("/r/i18n/glossary.tsv", "en\tpt-BR\tnotas\n")
            .with_file("/w/notes.txt", "novo\n");
        let mut git = dirty_git(vec![], &[]);
        git.diff_override = Some(vec!["notes.txt".to_string()]);
        let mut opts = unit_opts("u1");
        opts.base = Some("HEAD".to_string());
        let report = run_ok(&git, &fs, &test_paths(), &opts);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].code(), "audit-incomplete");
        assert_eq!(report.findings[0].severity(), Severity::Blocker);
        assert!(!report.ok);
        assert!(!report.blocking_ok);
    }
}
