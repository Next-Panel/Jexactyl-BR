//! Maquina de estados da fila i18n sobre SQLite.
//!
//! Port do modelo de estado de `scripts/i18n/i18n_toolkit/queue.py` sem o
//! CLI e sem a escrita de `CACHE.md`: transicoes, consultas e um instantaneo
//! JSON com o mesmo formato de alto nivel do `STATE` Python
//! (`version/policy/active_category/categories/units/patches/overlays/shared_overrides/events`).
//!
//! Contrato de tempo (espelha o "contrato NaN" do Python em
//! `parse_time_ms`, queue.py ~linhas 351-361): `expires_at` ausente ou
//! inparseavel e tratado como expirado. Timestamps sao strings ISO-8601 com
//! milissegundos e `Z`, exatamente como `utc_now_iso` (ex.:
//! `2026-01-01T00:00:00.000Z`).

use std::cell::Cell;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde_json::Value;
use thiserror::Error;

/// Papeis validos de claim.
const ROLES: [&str; 4] = ["analyst", "translator", "reviewer", "fixer"];
/// Gates obrigatorios da verificacao (queue.py `REQUIRED_GATES`).
const REQUIRED_GATES: [&str; 7] = [
    "catalog",
    "audit",
    "lint",
    "tests",
    "typecheck",
    "lint-ui",
    "build",
];
/// Versao do estado.
const STATE_VERSION: i64 = 1;
/// Limite de eventos mantidos (queue.py `save_state` mantem os ultimos 200).
const MAX_EVENTS: i64 = 200;
/// SQL do esquema.
const SCHEMA_SQL: &str = "
PRAGMA foreign_keys = ON;
CREATE TABLE IF NOT EXISTS meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS categories(
  id TEXT PRIMARY KEY,
  ord INTEGER NOT NULL,
  title TEXT NOT NULL,
  depends_on TEXT NOT NULL,
  stage TEXT NOT NULL,
  status TEXT NOT NULL,
  score INTEGER,
  consistency_score INTEGER,
  consistency_report TEXT,
  consistency_round INTEGER NOT NULL DEFAULT 0,
  blocked_stage TEXT,
  done_at TEXT,
  gate_status TEXT,
  blockers TEXT NOT NULL DEFAULT '[]'
);
CREATE TABLE IF NOT EXISTS units(
  id TEXT PRIMARY KEY,
  category TEXT NOT NULL REFERENCES categories(id),
  title TEXT NOT NULL,
  file_count INTEGER NOT NULL DEFAULT 0,
  stage TEXT NOT NULL,
  status TEXT NOT NULL,
  score INTEGER,
  attempts INTEGER NOT NULL DEFAULT 0,
  blocked_stage TEXT,
  analysis TEXT,
  translation_report TEXT,
  report TEXT,
  correction_report TEXT,
  findings TEXT,
  labels_total INTEGER NOT NULL DEFAULT 0,
  labels_translated INTEGER NOT NULL DEFAULT 0,
  translated_by TEXT,
  fixed_by TEXT,
  last_reviewer TEXT,
  reviewed_by TEXT NOT NULL DEFAULT '[]',
  translated_at TEXT,
  fixed_at TEXT,
  reviewed_at TEXT,
  done_at TEXT
);
CREATE TABLE IF NOT EXISTS claims(
  unit TEXT PRIMARY KEY REFERENCES units(id) ON DELETE CASCADE,
  owner TEXT NOT NULL,
  role TEXT NOT NULL,
  claimed_at TEXT NOT NULL,
  expires_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS reviews(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  unit TEXT NOT NULL REFERENCES units(id) ON DELETE CASCADE,
  round INTEGER NOT NULL,
  score INTEGER NOT NULL,
  reviewer TEXT NOT NULL,
  report TEXT NOT NULL,
  at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS gates(
  category TEXT NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  status TEXT NOT NULL,
  note TEXT NOT NULL DEFAULT '',
  at TEXT NOT NULL,
  PRIMARY KEY(category, name)
);
CREATE TABLE IF NOT EXISTS patches(
  name TEXT PRIMARY KEY,
  category TEXT NOT NULL REFERENCES categories(id),
  at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS patch_units(
  patch TEXT NOT NULL REFERENCES patches(name) ON DELETE CASCADE,
  unit TEXT NOT NULL REFERENCES units(id) ON DELETE CASCADE,
  PRIMARY KEY(patch, unit)
);
CREATE TABLE IF NOT EXISTS overlays(
  path TEXT PRIMARY KEY,
  category TEXT NOT NULL REFERENCES categories(id),
  at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS overlay_units(
  path TEXT NOT NULL REFERENCES overlays(path) ON DELETE CASCADE,
  unit TEXT NOT NULL REFERENCES units(id) ON DELETE CASCADE,
  PRIMARY KEY(path, unit)
);
CREATE TABLE IF NOT EXISTS shared_overrides(
  path TEXT PRIMARY KEY,
  category TEXT NOT NULL REFERENCES categories(id),
  reason TEXT NOT NULL,
  at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS shared_override_units(
  path TEXT NOT NULL REFERENCES shared_overrides(path) ON DELETE CASCADE,
  unit TEXT NOT NULL REFERENCES units(id) ON DELETE CASCADE,
  PRIMARY KEY(path, unit)
);
CREATE TABLE IF NOT EXISTS events(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  at TEXT NOT NULL,
  type TEXT NOT NULL,
  target TEXT NOT NULL,
  details TEXT NOT NULL DEFAULT '{}'
);
";

/// Relogio injetavel (tempo real ou fixo para testes).
pub trait Clock {
    /// Milissegundos desde o epoch Unix.
    fn now_ms(&self) -> u64;
    /// Instante ISO-8601 com milissegundos e `Z`, como `utc_now_iso`.
    fn now_iso(&self) -> String;
}

/// Relogio do sistema (`SystemTime` -> epoch; falha vira 0).
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl SystemClock {
    /// Constroi o relogio do sistema.
    pub fn new() -> Self {
        Self
    }
}

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(duration) => duration.as_millis().try_into().unwrap_or(u64::MAX),
            Err(_) => 0,
        }
    }

    fn now_iso(&self) -> String {
        format_iso_ms(self.now_ms())
    }
}

/// Relogio fixo para testes (tempo controlado via interior mutavel).
#[derive(Debug, Default)]
pub struct FixedClock {
    /// Milissegundos atuais.
    ms: Cell<u64>,
}

impl FixedClock {
    /// Constroi com o instante inicial em ms desde o epoch.
    pub fn new(ms: u64) -> Self {
        Self { ms: Cell::new(ms) }
    }

    /// Fixa o instante atual.
    pub fn set_ms(&self, ms: u64) {
        self.ms.set(ms);
    }

    /// Avanca o instante atual.
    pub fn advance_ms(&self, delta: u64) {
        self.ms.set(self.ms.get().saturating_add(delta));
    }
}

impl Clock for FixedClock {
    fn now_ms(&self) -> u64 {
        self.ms.get()
    }

    fn now_iso(&self) -> String {
        format_iso_ms(self.ms.get())
    }
}

/// Erros da maquina de estados.
#[derive(Debug, Error)]
pub enum StoreError {
    /// Falha do SQLite.
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    /// Falha de (de)serializacao JSON.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// Entidade inexistente.
    #[error("not found: {0}")]
    NotFound(String),
    /// Pre-condicao de estagio/status violada.
    #[error("{0}")]
    Precondition(String),
    /// Entrada invalida.
    #[error("{0}")]
    InvalidInput(String),
}

/// Linha da tabela `units` (stage, status, category, attempts, translated_by,
/// fixed_by, last_reviewer, reviewed_by).
type UnitStateRow = (
    String,
    String,
    String,
    i64,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
);

/// Linha da tabela `categories` (id, ord, stage, status, consistency_score,
/// score, consistency_round, blocked_stage, blockers).
type CategoryRow = (
    String,
    i64,
    String,
    String,
    Option<i64>,
    Option<i64>,
    i64,
    Option<String>,
    String,
);

/// Linha completa da tabela `units` para o snapshot.
type UnitSnapshotRow = (
    String,
    String,
    String,
    String,
    Option<i64>,
    i64,
    i64,
    i64,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
);

/// Opcoes de `complete` (relatorios ficam em memoria, sem filesystem).
#[derive(Debug, Clone, Default)]
pub struct CompleteOptions {
    /// Relatorio JSON (analise/traducao/correcao: qualquer objeto; revisao: validado).
    pub report: Option<Value>,
    /// Nota da revisao (somente `reviewer`).
    pub score: Option<i64>,
    /// Total de labels (somente `analyst`).
    pub labels_total: Option<i64>,
    /// Labels traduzidas (somente `translator`).
    pub labels_translated: Option<i64>,
    /// Nota livre do evento.
    pub note: Option<String>,
}

impl CompleteOptions {
    /// Opcoes vazias.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Loja SQLite da fila i18n.
#[derive(Debug)]
pub struct Store<C: Clock> {
    /// Conexao SQLite.
    conn: Connection,
    /// Fonte de tempo.
    clock: C,
}

/// Politica efetiva (do catalogo + `max_rounds = 3` + gates obrigatorios).
#[derive(Debug, Clone)]
struct Policy {
    /// Maximo de claims ativos simultaneos.
    max_agents: i64,
    /// Categorias ativas simultaneas (sempre 1).
    max_active_categories: i64,
    /// Lease padrao em minutos.
    lease_minutes: i64,
    /// Nota de aprovacao (sempre 10).
    pass_score: i64,
    /// Maximo de rodadas (sempre 3).
    max_rounds: i64,
    /// Gates obrigatorios.
    required_gates: Vec<String>,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            max_agents: 3,
            max_active_categories: 1,
            lease_minutes: 120,
            pass_score: 10,
            max_rounds: 3,
            required_gates: REQUIRED_GATES.iter().map(|s| s.to_string()).collect(),
        }
    }
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let adjusted_year = if month <= 2 { year - 1 } else { year };
    let era = if adjusted_year >= 0 {
        adjusted_year
    } else {
        adjusted_year - 399
    } / 400;
    let yoe = adjusted_year - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146096
    } / 146097;
    let doe = shifted - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    if month <= 2 {
        year += 1;
    }
    (year, month, day)
}

/// Formata ms desde o epoch como `YYYY-MM-DDTHH:MM:SS.sssZ`.
fn format_iso_ms(ms: u64) -> String {
    let secs = (ms / 1000) as i64;
    let millis = (ms % 1000) as u32;
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let (year, month, day) = civil_from_days(days);
    let hour = (rem / 3600) as u32;
    let minute = ((rem % 3600) / 60) as u32;
    let second = (rem % 60) as u32;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
}

/// Converte `YYYY-MM-DDTHH:MM:SS[.frac][Z|z|±HH:MM]` em ms; `None` se inparseavel.
fn parse_iso_ms(raw: &str) -> Option<u64> {
    let text = raw.trim();
    if text.is_empty() {
        return None;
    }
    // Separa o sufixo de zona.
    let (core, offset_minutes): (&str, i64) = if text.ends_with('Z') || text.ends_with('z') {
        (&text[..text.len() - 1], 0)
    } else {
        // Procura o ultimo '+' ou '-' apos o 'T'.
        let bytes = text.as_bytes();
        let mut pos: Option<usize> = None;
        let mut seen_t = false;
        for (index, byte) in bytes.iter().enumerate() {
            if *byte == b'T' || *byte == b't' {
                seen_t = true;
            }
            if seen_t && (*byte == b'+' || *byte == b'-') {
                pos = Some(index);
            }
        }
        match pos {
            Some(index) => {
                let zone = text[index..].trim();
                let sign: i64 = if zone.starts_with('-') { -1 } else { 1 };
                let digits: String = zone[1..].chars().filter(|c| c.is_ascii_digit()).collect();
                if digits.len() != 2 && digits.len() != 4 {
                    return None;
                }
                let hours: i64 = digits[..2].parse().ok()?;
                let minutes: i64 = if digits.len() == 4 {
                    digits[2..].parse().ok()?
                } else {
                    0
                };
                if hours > 23 || minutes > 59 {
                    return None;
                }
                (&text[..index], sign * (hours * 60 + minutes))
            }
            None => (text, 0),
        }
    };
    let (date_part, time_part) = core.split_once('T').or_else(|| core.split_once('t'))?;
    let date_fields: Vec<&str> = date_part.split('-').collect();
    if date_fields.len() != 3 {
        return None;
    }
    let year: i64 = date_fields[0].parse().ok()?;
    let month: i64 = date_fields[1].parse().ok()?;
    let day: i64 = date_fields[2].parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || !(0..=9999).contains(&year) {
        return None;
    }
    let month_days: i64 = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap(year) {
                29
            } else {
                28
            }
        }
        _ => return None,
    };
    if day > month_days {
        return None;
    }
    let (hms, fraction) = match time_part.split_once('.') {
        Some((head, tail)) => (head, Some(tail)),
        None => (time_part, None),
    };
    let hms_fields: Vec<&str> = hms.split(':').collect();
    if hms_fields.len() != 3 {
        return None;
    }
    let hour: i64 = hms_fields[0].parse().ok()?;
    let minute: i64 = hms_fields[1].parse().ok()?;
    let second: i64 = hms_fields[2].parse().ok()?;
    if hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    let mut millis: i64 = 0;
    if let Some(frac) = fraction {
        let digits: String = frac.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() || digits.len() > 9 {
            return None;
        }
        let padded = format!("{digits:0<3}");
        let head: String = padded.chars().take(3).collect();
        millis = head.parse().ok()?;
    }
    let days = days_from_civil(year, month, day);
    let total_ms = days * 86_400_000 + hour * 3_600_000 + minute * 60_000 + second * 1000 + millis
        - offset_minutes * 60_000;
    if total_ms < 0 {
        Some(0)
    } else {
        u64::try_from(total_ms).ok()
    }
}

/// Informa se o claim expirou (ausente/inparseavel/`<= now` expira).
fn is_expired(expires_at: Option<&str>, now_ms: u64) -> bool {
    match expires_at {
        Some(raw) => match parse_iso_ms(raw) {
            Some(moment) => moment <= now_ms,
            None => true,
        },
        None => true,
    }
}

/// Normaliza estagios (`pending/analyzing` -> `analysis`, etc.; queue.py `normalize_stage`).
fn normalize_stage(stage: &str) -> &str {
    match stage {
        "pending" | "analyzing" | "analysis" => "analysis",
        "translating" | "translation" => "translation",
        "reviewing" | "fixing" | "review" => "review",
        other => other,
    }
}

/// Ordem dos estagios de categoria para a regra backward-only.
fn stage_order(stage: &str) -> Option<i64> {
    match stage {
        "analysis" => Some(0),
        "translation" => Some(1),
        "review" => Some(2),
        "consistency" => Some(3),
        "verification" => Some(4),
        _ => None,
    }
}

/// Estagio da categoria exigido por papel (`ROLE_STAGE`).
fn role_stage(role: &str) -> Option<&'static str> {
    match role {
        "analyst" => Some("analysis"),
        "translator" => Some("translation"),
        "reviewer" => Some("review"),
        "fixer" => Some("review"),
        _ => None,
    }
}

/// Estagio da unidade ao reivindicar por papel.
fn claiming_unit_stage(role: &str) -> Option<&'static str> {
    match role {
        "analyst" => Some("analyzing"),
        "translator" => Some("translating"),
        "reviewer" => Some("reviewing"),
        "fixer" => Some("fixing"),
        _ => None,
    }
}

/// Etapa/status de reversao por papel (`revert_claim`).
fn revert_target(role: &str) -> (&'static str, &'static str) {
    match role {
        "analyst" => ("pending", "pending"),
        "translator" => ("translation", "pending"),
        "reviewer" => ("review", "pending"),
        "fixer" => ("review", "needs_fix"),
        _ => ("pending", "pending"),
    }
}

/// Rejeita texto com marcadores reservados do STATE (queue.py `save_state`).
fn reject_state_marker(text: &str) -> Result<(), StoreError> {
    if text.contains("STATE:BEGIN") || text.contains("STATE:END") {
        return Err(StoreError::InvalidInput(
            "state contains a reserved STATE marker; remove it from the free text".to_string(),
        ));
    }
    Ok(())
}

/// Valida caminho de overlay sob `overlays/` (lexical, sem filesystem).
fn check_overlay_path(path: &str) -> Result<(), StoreError> {
    if path.is_empty() {
        return Err(StoreError::InvalidInput("--path is required".to_string()));
    }
    if path.starts_with('/') {
        return Err(StoreError::InvalidInput(
            "--path must be relative".to_string(),
        ));
    }
    // Normalizacao lexical de `.`/`..`; escapar da raiz e rejeitado.
    let mut stack: Vec<&str> = Vec::new();
    for part in path.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            if stack.pop().is_none() {
                return Err(StoreError::InvalidInput(
                    "--path must stay under overlays/".to_string(),
                ));
            }
            continue;
        }
        stack.push(part);
    }
    let normalized = stack.join("/");
    if normalized != "overlays" && !normalized.starts_with("overlays/") {
        return Err(StoreError::InvalidInput(
            "--path must stay under overlays/".to_string(),
        ));
    }
    Ok(())
}

/// Valida caminho de shared override (dentro do repo, sem `.env`).
fn check_repo_path(path: &str) -> Result<(), StoreError> {
    if path.is_empty() {
        return Err(StoreError::InvalidInput("--path is required".to_string()));
    }
    if path.starts_with('/') {
        return Err(StoreError::InvalidInput(
            "--path must be relative".to_string(),
        ));
    }
    let mut stack: Vec<&str> = Vec::new();
    for part in path.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            if stack.pop().is_none() {
                return Err(StoreError::InvalidInput(
                    "--path must stay inside the repository".to_string(),
                ));
            }
            continue;
        }
        stack.push(part);
    }
    for part in &stack {
        if *part == ".env" || part.ends_with(".env") && part.starts_with('.') {
            return Err(StoreError::InvalidInput(
                "--path must not point to an env file".to_string(),
            ));
        }
    }
    // Bloqueia qualquer segmento `.env*` (espelha `r"(^|/)\.env"` do Python).
    for part in path.split('/') {
        if part.starts_with(".env") {
            return Err(StoreError::InvalidInput(
                "--path must not point to an env file".to_string(),
            ));
        }
    }
    Ok(())
}

fn json_string_list(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw).unwrap_or_default()
}

fn json_string_list_text(items: &[String]) -> String {
    serde_json::to_string(items).unwrap_or_else(|_| "[]".to_string())
}

fn unique_push(items: &mut Vec<String>, value: &str) {
    if !items.iter().any(|item| item == value) {
        items.push(value.to_string());
    }
}

fn policy_from_value(value: &Value) -> Policy {
    let mut policy = Policy::default();
    if let Some(object) = value.as_object() {
        if let Some(number) = object.get("max_agents").and_then(Value::as_i64) {
            policy.max_agents = number;
        } else if let Some(number) = object.get("max_agents").and_then(Value::as_u64) {
            policy.max_agents = number.min(i64::MAX as u64) as i64;
        }
        if let Some(number) = object.get("max_active_categories").and_then(Value::as_i64) {
            policy.max_active_categories = number;
        } else if let Some(number) = object.get("max_active_categories").and_then(Value::as_u64) {
            policy.max_active_categories = number.min(i64::MAX as u64) as i64;
        }
        if let Some(number) = object.get("lease_minutes").and_then(Value::as_i64) {
            policy.lease_minutes = number;
        } else if let Some(number) = object.get("lease_minutes").and_then(Value::as_u64) {
            policy.lease_minutes = number.min(i64::MAX as u64) as i64;
        }
        if let Some(number) = object.get("pass_score").and_then(Value::as_i64) {
            policy.pass_score = number;
        } else if let Some(number) = object.get("pass_score").and_then(Value::as_u64) {
            policy.pass_score = number.min(i64::MAX as u64) as i64;
        }
    }
    policy
}

fn get_meta(conn: &Connection, key: &str) -> Result<Option<String>, StoreError> {
    let result: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            rusqlite::params![key],
            |row| row.get(0),
        )
        .optional()?;
    Ok(result)
}

fn set_meta(tx: &Transaction<'_>, key: &str, value: &str) -> Result<(), StoreError> {
    tx.execute(
        "INSERT INTO meta(key, value) VALUES(?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    )?;
    Ok(())
}

fn get_policy_tx(tx: &Transaction<'_>) -> Result<Policy, StoreError> {
    let raw: Option<String> = tx
        .query_row("SELECT value FROM meta WHERE key = 'policy'", [], |row| {
            row.get(0)
        })
        .optional()?;
    match raw {
        Some(text) => match serde_json::from_str::<Value>(&text) {
            Ok(value) => Ok(policy_from_value(&value)),
            Err(_) => Ok(Policy::default()),
        },
        None => Ok(Policy::default()),
    }
}

fn get_active_tx(tx: &Transaction<'_>) -> Result<Option<String>, StoreError> {
    let raw: Option<String> = tx
        .query_row(
            "SELECT value FROM meta WHERE key = 'active_category'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    match raw {
        Some(text) if !text.is_empty() => Ok(Some(text)),
        _ => Ok(None),
    }
}

fn insert_event_tx(
    tx: &Transaction<'_>,
    at: &str,
    kind: &str,
    target: &str,
    details: &Value,
) -> Result<(), StoreError> {
    let payload = serde_json::to_string(details)?;
    tx.execute(
        "INSERT INTO events(at, type, target, details) VALUES(?1, ?2, ?3, ?4)",
        rusqlite::params![at, kind, target, payload],
    )?;
    tx.execute(
        "DELETE FROM events WHERE id NOT IN (SELECT id FROM events ORDER BY id DESC LIMIT ?1)",
        rusqlite::params![MAX_EVENTS],
    )?;
    Ok(())
}

fn revert_unit_tx(tx: &Transaction<'_>, unit_id: &str, role: &str) -> Result<(), StoreError> {
    let (stage, status) = revert_target(role);
    tx.execute(
        "UPDATE units SET stage = ?1, status = ?2 WHERE id = ?3",
        rusqlite::params![stage, status, unit_id],
    )?;
    tx.execute(
        "DELETE FROM claims WHERE unit = ?1",
        rusqlite::params![unit_id],
    )?;
    Ok(())
}

/// Recupera claims expirados dentro da transacao; devolve a contagem.
fn recover_expired_tx(
    tx: &Transaction<'_>,
    now_ms: u64,
    now_iso: &str,
) -> Result<usize, StoreError> {
    let mut due: Vec<(String, String, Option<String>, Option<String>)> = Vec::new();
    {
        let mut stmt = tx.prepare("SELECT unit, role, owner, expires_at FROM claims")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;
        for row in rows {
            let (unit, role, owner, expires_at) = row?;
            if is_expired(expires_at.as_deref(), now_ms) {
                due.push((unit, role, owner, expires_at));
            }
        }
    }
    let mut count = 0;
    for (unit, role, owner, _) in due {
        revert_unit_tx(tx, &unit, &role)?;
        let mut details = serde_json::Map::new();
        details.insert("role".to_string(), Value::String(role.clone()));
        if let Some(name) = owner {
            details.insert("owner".to_string(), Value::String(name));
        }
        insert_event_tx(tx, now_iso, "lease-expired", &unit, &Value::Object(details))?;
        count += 1;
    }
    Ok(count)
}

fn active_claim_count_tx(tx: &Transaction<'_>, now_ms: u64) -> Result<i64, StoreError> {
    let mut count = 0;
    let mut stmt = tx.prepare("SELECT expires_at FROM claims")?;
    let rows = stmt.query_map([], |row| row.get::<_, Option<String>>(0))?;
    for row in rows {
        let expires_at: Option<String> = row?;
        if !is_expired(expires_at.as_deref(), now_ms) {
            count += 1;
        }
    }
    Ok(count)
}

fn has_active_claim_tx(
    tx: &Transaction<'_>,
    unit_id: &str,
    now_ms: u64,
) -> Result<bool, StoreError> {
    let expires_at: Option<Option<String>> = tx
        .query_row(
            "SELECT expires_at FROM claims WHERE unit = ?1",
            rusqlite::params![unit_id],
            |row| row.get(0),
        )
        .optional()?;
    match expires_at {
        Some(inner) => Ok(!is_expired(inner.as_deref(), now_ms)),
        None => Ok(false),
    }
}

/// Bloqueia a categoria dentro da transacao (libera claims das irmas).
fn block_category_tx(
    tx: &Transaction<'_>,
    category_id: &str,
    reason: &str,
    now_ms: u64,
    now_iso: &str,
    active: Option<String>,
) -> Result<(), StoreError> {
    let (stage, blockers_raw, blocked_stage_raw): (String, String, Option<String>) = tx
        .query_row(
            "SELECT stage, blockers, blocked_stage FROM categories WHERE id = ?1",
            rusqlite::params![category_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => {
                StoreError::NotFound(format!("unknown category: {category_id}"))
            }
            other => StoreError::Db(other),
        })?;
    let next_blocked_stage = if stage == "blocked" {
        blocked_stage_raw.unwrap_or_else(|| normalize_stage(&stage).to_string())
    } else {
        normalize_stage(&stage).to_string()
    };
    let mut blockers = json_string_list(&blockers_raw);
    unique_push(&mut blockers, reason);
    let blockers_text = json_string_list_text(&blockers);
    tx.execute(
        "UPDATE categories SET stage = 'blocked', status = 'blocked', blocked_stage = ?1, blockers = ?2 WHERE id = ?3",
        rusqlite::params![next_blocked_stage, blockers_text, category_id],
    )?;
    // Libera claims das unidades da categoria.
    let mut claimed: Vec<(String, String, Option<String>, String)> = Vec::new();
    {
        let mut stmt =
            tx.prepare("SELECT u.id, c.role, c.owner, c.expires_at FROM units u JOIN claims c ON c.unit = u.id WHERE u.category = ?1")?;
        let rows = stmt.query_map(rusqlite::params![category_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        for row in rows {
            let (unit, role, owner, expires_at) = row?;
            if !is_expired(Some(expires_at.as_str()), now_ms) {
                claimed.push((unit, role, owner, expires_at));
            }
        }
    }
    for (unit, role, owner, _) in claimed {
        revert_unit_tx(tx, &unit, &role)?;
        let mut details = serde_json::Map::new();
        if let Some(name) = owner {
            details.insert("owner".to_string(), Value::String(name));
        }
        details.insert("role".to_string(), Value::String(role));
        details.insert(
            "reason".to_string(),
            Value::String(format!("category {category_id} blocked")),
        );
        insert_event_tx(tx, now_iso, "release", &unit, &Value::Object(details))?;
    }
    let was_focus = active.as_deref() == Some(category_id);
    if was_focus || active.is_none() {
        set_meta(tx, "active_category", category_id)?;
    }
    let mut details = serde_json::Map::new();
    details.insert("reason".to_string(), Value::String(reason.to_string()));
    insert_event_tx(tx, now_iso, "blocked", category_id, &Value::Object(details))?;
    Ok(())
}

/// Garante foco unico; pausa outra categoria ativa quando `force`.
fn ensure_only_active_tx(
    tx: &Transaction<'_>,
    category_id: &str,
    force: bool,
    reason: Option<&str>,
    now_ms: u64,
    now_iso: &str,
) -> Result<(), StoreError> {
    let mut other: Option<(String, String)> = None;
    {
        let mut stmt =
            tx.prepare("SELECT id, stage FROM categories WHERE status IN ('active','needs_fix')")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (id, stage) = row?;
            if id != category_id {
                other = Some((id, stage));
                break;
            }
        }
    }
    if let Some((other_id, other_stage)) = other {
        if !force {
            return Err(StoreError::Precondition(format!(
                "another category is active: {other_id}; finish or block it before activating {category_id} (or use --force)"
            )));
        }
        let mut live: Vec<String> = Vec::new();
        {
            let mut stmt = tx.prepare(
                "SELECT u.id, c.expires_at FROM units u JOIN claims c ON c.unit = u.id WHERE u.category = ?1",
            )?;
            let rows = stmt.query_map(rusqlite::params![other_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
            })?;
            for row in rows {
                let (unit, expires_at) = row?;
                if !is_expired(expires_at.as_deref(), now_ms) {
                    live.push(unit);
                }
            }
        }
        if !live.is_empty() {
            return Err(StoreError::Precondition(format!(
                "cannot pause {other_id}: it still has active claim(s): {}",
                live.join(", ")
            )));
        }
        let paused_stage = normalize_stage(&other_stage).to_string();
        let current_blockers: String = tx
            .query_row(
                "SELECT blockers FROM categories WHERE id = ?1",
                rusqlite::params![other_id],
                |row| row.get(0),
            )
            .map_err(StoreError::Db)?;
        let mut blockers = json_string_list(&current_blockers);
        let suffix = match reason {
            Some(text) if !text.is_empty() => format!(": {text}"),
            _ => String::new(),
        };
        blockers.push(format!("pausada para reabrir {category_id}{suffix}"));
        let blockers_text = json_string_list_text(&blockers);
        tx.execute(
            "UPDATE categories SET stage = 'blocked', status = 'blocked', blocked_stage = ?1, blockers = ?2 WHERE id = ?3",
            rusqlite::params![paused_stage, blockers_text, other_id],
        )?;
        let mut details = serde_json::Map::new();
        details.insert(
            "reason".to_string(),
            Value::String(format!("paused for reopen of {category_id}")),
        );
        insert_event_tx(tx, now_iso, "blocked", &other_id, &Value::Object(details))?;
    }
    set_meta(tx, "active_category", category_id)?;
    Ok(())
}

/// Valida relatorio de revisao/consistencia em memoria (sem filesystem).
fn validate_review_report_value(
    report: &Value,
    score: i64,
    expected_unit: Option<&str>,
    expected_category: Option<&str>,
    expected_round: i64,
) -> Result<(), StoreError> {
    let object = report.as_object().ok_or_else(|| {
        StoreError::InvalidInput("review report must be a JSON object".to_string())
    })?;
    let report_score = object
        .get("score")
        .and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
        })
        .ok_or_else(|| {
            StoreError::InvalidInput("review report must contain an integer score".to_string())
        })?;
    if report_score != score {
        return Err(StoreError::InvalidInput(format!(
            "--score {score} does not match the report score {report_score}"
        )));
    }
    let findings = object
        .get("findings")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            StoreError::InvalidInput("review report findings must be an array".to_string())
        })?;
    if score == 10 && !findings.is_empty() {
        return Err(StoreError::InvalidInput(
            "score 10 requires an empty findings array".to_string(),
        ));
    }
    if score < 10 && findings.is_empty() {
        return Err(StoreError::InvalidInput(
            "a score below 10 requires at least one finding".to_string(),
        ));
    }
    let mut has_blocking = false;
    for finding in findings {
        let record = finding.as_object().ok_or_else(|| {
            StoreError::InvalidInput("each finding must be an object".to_string())
        })?;
        for name in ["id", "severity", "file", "problem", "fix"] {
            let valid = record
                .get(name)
                .and_then(Value::as_str)
                .is_some_and(|text| !text.trim().is_empty());
            if !valid {
                return Err(StoreError::InvalidInput(format!(
                    "finding {name} must be a non-empty string"
                )));
            }
        }
        let line = record.get("line").and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
        });
        if line.is_none_or(|number| number <= 0) {
            return Err(StoreError::InvalidInput(
                "finding line must be a positive integer".to_string(),
            ));
        }
        let severity = record.get("severity").and_then(Value::as_str).unwrap_or("");
        if !["blocker", "major", "minor", "nit"].contains(&severity) {
            return Err(StoreError::InvalidInput(format!(
                "invalid finding severity: {severity}"
            )));
        }
        if severity == "blocker" || severity == "major" {
            has_blocking = true;
        }
    }
    if has_blocking && score > 7 {
        return Err(StoreError::InvalidInput(
            "blocker/major findings require a score of at most 7".to_string(),
        ));
    }
    if !findings.is_empty() && score == 10 {
        return Err(StoreError::InvalidInput(
            "any finding prevents a score of 10".to_string(),
        ));
    }
    let criteria = object
        .get("criteria")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            StoreError::InvalidInput("review report must contain criteria".to_string())
        })?;
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
            return Err(StoreError::InvalidInput(
                "criteria must contain exactly coverage, naturalness, consistency, context and safety"
                    .to_string(),
            ));
        }
    }
    let mut total = 0;
    for name in keys {
        let value = criteria.get(name).and_then(|entry| {
            entry
                .as_i64()
                .or_else(|| entry.as_u64().and_then(|number| i64::try_from(number).ok()))
        });
        match value {
            Some(number) if (0..=2).contains(&number) => total += number,
            _ => {
                return Err(StoreError::InvalidInput(format!(
                    "criteria.{name} must be 0, 1 or 2"
                )))
            }
        }
    }
    if total != score {
        return Err(StoreError::InvalidInput(format!(
            "criteria sum {total} does not match score {score}"
        )));
    }
    for name in ["untranslated", "glossary_violations", "test_impact"] {
        if object.get(name).and_then(Value::as_array).is_none() {
            return Err(StoreError::InvalidInput(format!(
                "review report {name} must be an array"
            )));
        }
    }
    if score == 10 {
        let untranslated_empty = object
            .get("untranslated")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty);
        if !untranslated_empty {
            return Err(StoreError::InvalidInput(
                "score 10 requires untranslated to be empty".to_string(),
            ));
        }
        let glossary_empty = object
            .get("glossary_violations")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty);
        if !glossary_empty {
            return Err(StoreError::InvalidInput(
                "score 10 requires glossary_violations to be empty".to_string(),
            ));
        }
    }
    if let Some(unit) = expected_unit {
        let actual = object.get("unit").and_then(Value::as_str).unwrap_or("");
        if actual.is_empty() {
            return Err(StoreError::InvalidInput(
                "review report must contain unit".to_string(),
            ));
        }
        if actual != unit {
            return Err(StoreError::InvalidInput(
                "review report unit does not match".to_string(),
            ));
        }
        let round = object.get("round").and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
        });
        if round.is_none() {
            return Err(StoreError::InvalidInput(
                "review report must contain round".to_string(),
            ));
        }
        if round != Some(expected_round) {
            return Err(StoreError::InvalidInput(
                "review report round does not match".to_string(),
            ));
        }
    }
    if let Some(category) = expected_category {
        let actual = object.get("category").and_then(Value::as_str).unwrap_or("");
        if actual.is_empty() {
            return Err(StoreError::InvalidInput(
                "review report must contain category".to_string(),
            ));
        }
        if actual != category {
            return Err(StoreError::InvalidInput(
                "review report category does not match".to_string(),
            ));
        }
        let round = object.get("round").and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
        });
        if round.is_none() {
            return Err(StoreError::InvalidInput(
                "review report must contain round".to_string(),
            ));
        }
        if round != Some(expected_round) {
            return Err(StoreError::InvalidInput(
                "review report round does not match".to_string(),
            ));
        }
    }
    Ok(())
}

impl<C: Clock> Store<C> {
    /// Abre uma base em memoria e cria o esquema.
    pub fn open_in_memory(clock: C) -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory()?;
        let store = Self { conn, clock };
        store.conn.execute_batch(SCHEMA_SQL)?;
        Ok(store)
    }

    /// Abre (ou cria) uma base em arquivo e cria o esquema.
    pub fn open(path: impl AsRef<Path>, clock: C) -> Result<Self, StoreError> {
        let conn = Connection::open(path.as_ref())?;
        let store = Self { conn, clock };
        store.conn.execute_batch(SCHEMA_SQL)?;
        Ok(store)
    }

    /// Acede ao relogio (permite avancar um `FixedClock` nos testes).
    pub fn clock(&self) -> &C {
        &self.clock
    }

    /// Inicializa o estado a partir do catalogo (`catalog.lock.json` como JSON).
    ///
    /// Espelha `initialize_state` (queue.py ~887-966): politica com
    /// `max_rounds = 3` e gates obrigatorios, primeira categoria ativa em
    /// `analysis`, resto pendente, evento `init`.
    pub fn init_from_catalog(&mut self, catalog: &Value) -> Result<(), StoreError> {
        let categories_value = catalog
            .get("categories")
            .and_then(Value::as_array)
            .ok_or_else(|| StoreError::InvalidInput("manifest has no categories".to_string()))?;
        if categories_value.is_empty() {
            return Err(StoreError::InvalidInput(
                "manifest has no categories".to_string(),
            ));
        }
        let policy = policy_from_value(catalog.get("policy").unwrap_or(&Value::Null));
        let now_iso = self.clock.now_iso();
        let tx = self.conn.transaction()?;
        for table in [
            "events",
            "reviews",
            "claims",
            "patch_units",
            "patches",
            "overlay_units",
            "overlays",
            "shared_override_units",
            "shared_overrides",
            "gates",
            "units",
            "categories",
            "meta",
        ] {
            tx.execute(&format!("DELETE FROM {table}"), [])?;
        }
        // Ordena categorias por `order`.
        let mut ordered: Vec<&Value> = categories_value.iter().collect();
        ordered.sort_by_key(|entry| {
            entry
                .get("order")
                .and_then(Value::as_i64)
                .or_else(|| {
                    entry
                        .get("order")
                        .and_then(Value::as_u64)
                        .and_then(|number| i64::try_from(number).ok())
                })
                .unwrap_or(0)
        });
        let policy_value = serde_json::json!({
            "max_agents": policy.max_agents,
            "max_active_categories": policy.max_active_categories,
            "lease_minutes": policy.lease_minutes,
            "pass_score": policy.pass_score,
            "max_rounds": policy.max_rounds,
            "required_gates": policy.required_gates,
        });
        set_meta(&tx, "policy", &serde_json::to_string(&policy_value)?)?;
        set_meta(&tx, "created_at", &now_iso)?;
        set_meta(&tx, "updated_at", &now_iso)?;
        set_meta(&tx, "version", &STATE_VERSION.to_string())?;
        let mut first_id: Option<String> = None;
        for entry in ordered {
            let object = entry.as_object().ok_or_else(|| {
                StoreError::InvalidInput(
                    "manifesto invalido: categoria fora do formato esperado".to_string(),
                )
            })?;
            let id = object
                .get("id")
                .and_then(Value::as_str)
                .filter(|text| !text.is_empty())
                .ok_or_else(|| {
                    StoreError::InvalidInput(
                        "manifesto invalido: categoria sem id textual".to_string(),
                    )
                })?;
            let order = object
                .get("order")
                .and_then(Value::as_i64)
                .or_else(|| {
                    object
                        .get("order")
                        .and_then(Value::as_u64)
                        .and_then(|number| i64::try_from(number).ok())
                })
                .unwrap_or(0);
            let title_text = serde_json::to_string(object.get("title").unwrap_or(&Value::Null))?;
            let depends_text = serde_json::to_string(
                object
                    .get("depends_on")
                    .unwrap_or(&Value::Array(Vec::new())),
            )?;
            let raw_units = object
                .get("units")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    StoreError::InvalidInput(
                        "manifesto invalido: categoria sem lista de unidades".to_string(),
                    )
                })?;
            if first_id.is_none() {
                first_id = Some(id.to_string());
            }
            tx.execute(
                "INSERT INTO categories(id, ord, title, depends_on, stage, status, consistency_round, blockers) VALUES(?1, ?2, ?3, ?4, 'pending', 'pending', 0, '[]')",
                rusqlite::params![id, order, title_text, depends_text],
            )?;
            let mut unit_ids: Vec<String> = Vec::new();
            for item in raw_units {
                let unit_object = item.as_object().ok_or_else(|| {
                    StoreError::InvalidInput(
                        "manifesto invalido: unidade fora do formato esperado".to_string(),
                    )
                })?;
                let unit_id = unit_object
                    .get("id")
                    .and_then(Value::as_str)
                    .filter(|text| !text.is_empty())
                    .ok_or_else(|| {
                        StoreError::InvalidInput(
                            "manifesto invalido: unidade sem id textual".to_string(),
                        )
                    })?;
                let unit_title =
                    serde_json::to_string(unit_object.get("title").unwrap_or(&Value::Null))?;
                let file_count = unit_object
                    .get("files")
                    .and_then(Value::as_array)
                    .map_or(0, Vec::len) as i64;
                tx.execute(
                    "INSERT INTO units(id, category, title, file_count, stage, status, attempts, reviewed_by) VALUES(?1, ?2, ?3, ?4, 'pending', 'pending', 0, '[]')",
                    rusqlite::params![unit_id, id, unit_title, file_count],
                )?;
                unit_ids.push(unit_id.to_string());
            }
            let _ = unit_ids;
        }
        if let Some(first) = first_id.clone() {
            tx.execute(
                "UPDATE categories SET stage = 'analysis', status = 'active' WHERE id = ?1",
                rusqlite::params![first],
            )?;
            set_meta(&tx, "active_category", &first)?;
        } else {
            set_meta(&tx, "active_category", "")?;
        }
        let mut details = serde_json::Map::new();
        details.insert(
            "categories".to_string(),
            Value::from(categories_value.len() as u64),
        );
        insert_event_tx(&tx, &now_iso, "init", "pipeline", &Value::Object(details))?;
        set_meta(&tx, "updated_at", &now_iso)?;
        tx.commit()?;
        Ok(())
    }

    /// Reivindica uma unidade (`BEGIN IMMEDIATE`; queue.py `claim_unit` ~620-649).
    pub fn claim(
        &mut self,
        unit_id: &str,
        role: &str,
        owner: &str,
        lease_minutes: i64,
    ) -> Result<(), StoreError> {
        if !ROLES.contains(&role) {
            return Err(StoreError::InvalidInput(format!(
                "--role must be one of {}",
                ROLES.join(", ")
            )));
        }
        if owner.trim().is_empty() {
            return Err(StoreError::InvalidInput("--owner is required".to_string()));
        }
        if lease_minutes <= 0 {
            return Err(StoreError::InvalidInput(
                "lease minutes must be a positive integer".to_string(),
            ));
        }
        reject_state_marker(owner)?;
        reject_state_marker(unit_id)?;
        let now_ms = self.clock.now_ms();
        let now_iso = self.clock.now_iso();
        let expires_iso =
            format_iso_ms(now_ms.saturating_add((lease_minutes as u64).saturating_mul(60_000)));
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        recover_expired_tx(&tx, now_ms, &now_iso)?;
        let policy = get_policy_tx(&tx)?;
        let active = get_active_tx(&tx)?;
        let (unit_stage, unit_status, unit_category, translated_by, fixed_by, last_reviewer): (
            String,
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
        ) = tx
            .query_row(
                "SELECT stage, status, category, translated_by, fixed_by, last_reviewer FROM units WHERE id = ?1",
                rusqlite::params![unit_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    StoreError::NotFound(format!("unknown unit: {unit_id}"))
                }
                other => StoreError::Db(other),
            })?;
        let (category_stage, category_status): (String, String) = tx
            .query_row(
                "SELECT stage, status FROM categories WHERE id = ?1",
                rusqlite::params![unit_category],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    StoreError::NotFound(format!("unknown category: {unit_category}"))
                }
                other => StoreError::Db(other),
            })?;
        if category_status == "blocked" {
            return Err(StoreError::Precondition(format!(
                "category {unit_category} is blocked"
            )));
        }
        if active.as_deref() != Some(unit_category.as_str()) {
            return Err(StoreError::Precondition(format!(
                "category {unit_category} is not the active category"
            )));
        }
        match role {
            "analyst" => {
                if category_stage != "analysis" {
                    return Err(StoreError::Precondition(format!(
                        "category {unit_category} is not in analysis"
                    )));
                }
                if unit_stage != "pending" {
                    return Err(StoreError::Precondition(format!(
                        "unit {unit_id} is not pending analysis"
                    )));
                }
            }
            "translator" => {
                if category_stage != "translation" {
                    return Err(StoreError::Precondition(format!(
                        "category {unit_category} is not in translation"
                    )));
                }
                if unit_stage != "translation" {
                    return Err(StoreError::Precondition(format!(
                        "unit {unit_id} is not ready for translation"
                    )));
                }
                if unit_status != "pending" {
                    return Err(StoreError::Precondition(format!(
                        "unit {unit_id} is not pending translation"
                    )));
                }
            }
            "reviewer" => {
                if category_stage != "review" {
                    return Err(StoreError::Precondition(format!(
                        "category {unit_category} is not in review"
                    )));
                }
                if unit_stage != "review" {
                    return Err(StoreError::Precondition(format!(
                        "unit {unit_id} is not ready for review"
                    )));
                }
                if unit_status != "pending" {
                    return Err(StoreError::Precondition(format!(
                        "unit {unit_id} is not pending review"
                    )));
                }
            }
            "fixer" => {
                if category_stage != "review" {
                    return Err(StoreError::Precondition(format!(
                        "category {unit_category} is not in review"
                    )));
                }
                if unit_stage != "review" {
                    return Err(StoreError::Precondition(format!(
                        "unit {unit_id} is not ready for correction"
                    )));
                }
                if unit_status != "needs_fix" {
                    return Err(StoreError::Precondition(format!(
                        "unit {unit_id} has no pending findings"
                    )));
                }
            }
            _ => {
                return Err(StoreError::InvalidInput(format!(
                    "--role must be one of {}",
                    ROLES.join(", ")
                )));
            }
        }
        if has_active_claim_tx(&tx, unit_id, now_ms)? {
            return Err(StoreError::Precondition(format!(
                "unit {unit_id} already has an active claim"
            )));
        }
        let claimed = active_claim_count_tx(&tx, now_ms)?;
        if claimed >= policy.max_agents {
            return Err(StoreError::Precondition(format!(
                "concurrency limit reached ({claimed}/{})",
                policy.max_agents
            )));
        }
        if role == "reviewer" {
            if let Some(author) = translated_by {
                if author == owner {
                    return Err(StoreError::Precondition(format!(
                        "reviewer {owner} translated {unit_id}; use an independent reviewer"
                    )));
                }
            }
            if let Some(author) = fixed_by {
                if author == owner {
                    return Err(StoreError::Precondition(format!(
                        "reviewer {owner} fixed {unit_id}; use an independent reviewer"
                    )));
                }
            }
        }
        if role == "fixer" {
            if let Some(author) = last_reviewer {
                if author == owner {
                    return Err(StoreError::Precondition(format!(
                        "fixer {owner} reviewed {unit_id}; use an independent fixer"
                    )));
                }
            }
        }
        tx.execute(
            "INSERT INTO claims(unit, owner, role, claimed_at, expires_at) VALUES(?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![unit_id, owner, role, now_iso, expires_iso],
        )?;
        let next_stage = claiming_unit_stage(role).unwrap_or("pending");
        tx.execute(
            "UPDATE units SET stage = ?1, status = 'active' WHERE id = ?2",
            rusqlite::params![next_stage, unit_id],
        )?;
        let mut details = serde_json::Map::new();
        details.insert("role".to_string(), Value::String(role.to_string()));
        details.insert("owner".to_string(), Value::String(owner.to_string()));
        details.insert("lease_minutes".to_string(), Value::from(lease_minutes));
        insert_event_tx(&tx, &now_iso, "claim", unit_id, &Value::Object(details))?;
        set_meta(&tx, "updated_at", &now_iso)?;
        tx.commit()?;
        Ok(())
    }

    /// Lista unidades reivindicaveis para o papel (somente leitura; queue.py `claimable_units`).
    pub fn next(&self, role: &str, limit: usize) -> Result<Vec<String>, StoreError> {
        if !ROLES.contains(&role) {
            return Err(StoreError::InvalidInput(format!(
                "--role must be one of {}",
                ROLES.join(", ")
            )));
        }
        if limit == 0 {
            return Err(StoreError::InvalidInput(
                "--limit must be positive".to_string(),
            ));
        }
        let now_ms = self.clock.now_ms();
        let policy_text = get_meta(&self.conn, "policy")?.unwrap_or_default();
        let policy = match serde_json::from_str::<Value>(&policy_text) {
            Ok(value) => policy_from_value(&value),
            Err(_) => Policy::default(),
        };
        let active = get_meta(&self.conn, "active_category")?.unwrap_or_default();
        if active.is_empty() {
            return Ok(Vec::new());
        }
        let (category_stage, category_status): (String, String) = self
            .conn
            .query_row(
                "SELECT stage, status FROM categories WHERE id = ?1",
                rusqlite::params![active],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    StoreError::NotFound(format!("unknown category: {active}"))
                }
                other => StoreError::Db(other),
            })?;
        if category_status == "blocked" {
            return Ok(Vec::new());
        }
        if role_stage(role).unwrap_or("") != category_stage {
            return Ok(Vec::new());
        }
        let mut live = 0;
        {
            let mut stmt = self.conn.prepare("SELECT expires_at FROM claims")?;
            let rows = stmt.query_map([], |row| row.get::<_, Option<String>>(0))?;
            for row in rows {
                let expires_at: Option<String> = row?;
                if !is_expired(expires_at.as_deref(), now_ms) {
                    live += 1;
                }
            }
        }
        if live >= policy.max_agents {
            return Ok(Vec::new());
        }
        let slots = (policy.max_agents - live).max(0) as usize;
        let mut result: Vec<String> = Vec::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT u.id, u.stage, u.status, c.expires_at FROM units u LEFT JOIN claims c ON c.unit = u.id WHERE u.category = ?1 ORDER BY u.id ASC",
            )?;
            let rows = stmt.query_map(rusqlite::params![active], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })?;
            for row in rows {
                let (id, stage, status, expires_at) = row?;
                if !is_expired(expires_at.as_deref(), now_ms) && expires_at.is_some() {
                    continue;
                }
                let wanted = match role {
                    "analyst" => stage == "pending",
                    "translator" => stage == "translation" && status == "pending",
                    "reviewer" => stage == "review" && status == "pending",
                    "fixer" => stage == "review" && status == "needs_fix",
                    _ => false,
                };
                if wanted {
                    result.push(id);
                }
            }
        }
        result.truncate(limit.min(slots));
        Ok(result)
    }

    /// Conclui o claim ativo (queue.py `complete_unit` ~761-810 + `complete_review`).
    ///
    /// Simplificacao documentada: relatorios sao JSON em memoria (sem
    /// leitura de arquivos); a validacao da revisao segue as mesmas regras do
    /// Python (`score` bate, `findings`/`criteria`/severidades, 10 exige
    /// vazio).
    #[allow(clippy::too_many_lines)]
    pub fn complete(
        &mut self,
        unit_id: &str,
        role: &str,
        owner: &str,
        opts: CompleteOptions,
    ) -> Result<(), StoreError> {
        if !ROLES.contains(&role) {
            return Err(StoreError::InvalidInput(format!(
                "--role must be one of {}",
                ROLES.join(", ")
            )));
        }
        if owner.trim().is_empty() {
            return Err(StoreError::InvalidInput("--owner is required".to_string()));
        }
        if let Some(note) = opts.note.as_deref() {
            reject_state_marker(note)?;
        }
        let now_ms = self.clock.now_ms();
        let now_iso = self.clock.now_iso();
        let tx = self.conn.transaction()?;
        recover_expired_tx(&tx, now_ms, &now_iso)?;
        let policy = get_policy_tx(&tx)?;
        let active = get_active_tx(&tx)?;
        let (
            unit_stage,
            unit_status,
            unit_category,
            attempts,
            translated_by,
            fixed_by,
            _last_reviewer,
            reviewed_by_raw,
        ): UnitStateRow = tx
            .query_row(
                "SELECT stage, status, category, attempts, translated_by, fixed_by, last_reviewer, reviewed_by FROM units WHERE id = ?1",
                rusqlite::params![unit_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                    ))
                },
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    StoreError::NotFound(format!("unknown unit: {unit_id}"))
                }
                other => StoreError::Db(other),
            })?;
        let (claim_owner, claim_role): (String, String) = tx
            .query_row(
                "SELECT owner, role FROM claims WHERE unit = ?1",
                rusqlite::params![unit_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    StoreError::Precondition(format!("unit {unit_id} has no active claim"))
                }
                other => StoreError::Db(other),
            })?;
        if claim_role != role {
            return Err(StoreError::Precondition(format!(
                "unit {unit_id} is claimed as {claim_role}, not {role}"
            )));
        }
        if claim_owner != owner {
            return Err(StoreError::Precondition(format!(
                "unit {unit_id} is claimed by {claim_owner}, not {owner}"
            )));
        }
        let (category_stage, category_status): (String, String) = tx
            .query_row(
                "SELECT stage, status FROM categories WHERE id = ?1",
                rusqlite::params![unit_category],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    StoreError::NotFound(format!("unknown category: {unit_category}"))
                }
                other => StoreError::Db(other),
            })?;
        if active.as_deref() != Some(unit_category.as_str()) {
            return Err(StoreError::Precondition(format!(
                "category {unit_category} is not the active category"
            )));
        }
        if category_status == "blocked" {
            return Err(StoreError::Precondition(format!(
                "category {unit_category} is blocked; unblock it before completing work"
            )));
        }
        let _ = (unit_stage, unit_status, category_stage);
        match role {
            "analyst" => {
                let report = opts.report.as_ref().ok_or_else(|| {
                    StoreError::InvalidInput("--report is required for analysis".to_string())
                })?;
                if let Value::String(text) = report {
                    reject_state_marker(text)?;
                }
                let report_text = serde_json::to_string(report)?;
                reject_state_marker(&report_text)?;
                tx.execute(
                    "UPDATE units SET analysis = ?1, stage = 'translation', status = 'pending' WHERE id = ?2",
                    rusqlite::params![report_text, unit_id],
                )?;
                if let Some(total) = opts.labels_total {
                    if total < 0 {
                        return Err(StoreError::InvalidInput(
                            "--labels-total must be non-negative".to_string(),
                        ));
                    }
                    tx.execute(
                        "UPDATE units SET labels_total = ?1 WHERE id = ?2",
                        rusqlite::params![total, unit_id],
                    )?;
                }
                let mut details = serde_json::Map::new();
                details.insert(
                    "report".to_string(),
                    Value::String(report_text.chars().take(512).collect()),
                );
                if let Some(note) = opts.note {
                    details.insert("note".to_string(), Value::String(note));
                }
                tx.execute(
                    "DELETE FROM claims WHERE unit = ?1",
                    rusqlite::params![unit_id],
                )?;
                insert_event_tx(
                    &tx,
                    &now_iso,
                    "analysis-done",
                    unit_id,
                    &Value::Object(details),
                )?;
            }
            "translator" => {
                let report = opts.report.as_ref().ok_or_else(|| {
                    StoreError::InvalidInput("--report is required for translation".to_string())
                })?;
                let report_text = serde_json::to_string(report)?;
                reject_state_marker(&report_text)?;
                tx.execute(
                    "UPDATE units SET translation_report = ?1, stage = 'review', status = 'pending', translated_at = ?2, translated_by = ?3 WHERE id = ?4",
                    rusqlite::params![report_text, now_iso, owner, unit_id],
                )?;
                if let Some(translated) = opts.labels_translated {
                    if translated < 0 {
                        return Err(StoreError::InvalidInput(
                            "--labels-translated must be non-negative".to_string(),
                        ));
                    }
                    let total: i64 = tx.query_row(
                        "SELECT labels_total FROM units WHERE id = ?1",
                        rusqlite::params![unit_id],
                        |row| row.get(0),
                    )?;
                    if total != 0 && translated > total {
                        return Err(StoreError::Precondition(
                            "--labels-translated cannot exceed --labels-total".to_string(),
                        ));
                    }
                    tx.execute(
                        "UPDATE units SET labels_translated = ?1 WHERE id = ?2",
                        rusqlite::params![translated, unit_id],
                    )?;
                }
                let mut details = serde_json::Map::new();
                details.insert(
                    "report".to_string(),
                    Value::String(report_text.chars().take(512).collect()),
                );
                if let Some(note) = opts.note {
                    details.insert("note".to_string(), Value::String(note));
                }
                tx.execute(
                    "DELETE FROM claims WHERE unit = ?1",
                    rusqlite::params![unit_id],
                )?;
                insert_event_tx(
                    &tx,
                    &now_iso,
                    "translation-done",
                    unit_id,
                    &Value::Object(details),
                )?;
            }
            "reviewer" => {
                let score = opts.score.ok_or_else(|| {
                    StoreError::InvalidInput("--score must be an integer".to_string())
                })?;
                if !(0..=10).contains(&score) {
                    return Err(StoreError::InvalidInput(
                        "score must be between 0 and 10".to_string(),
                    ));
                }
                let report = opts.report.as_ref().ok_or_else(|| {
                    StoreError::InvalidInput("--report is required for review".to_string())
                })?;
                validate_review_report_value(report, score, Some(unit_id), None, attempts + 1)?;
                let report_text = serde_json::to_string(report)?;
                reject_state_marker(&report_text)?;
                let next_attempts = attempts + 1;
                let mut reviewed_by = json_string_list(&reviewed_by_raw);
                unique_push(&mut reviewed_by, owner);
                let reviewed_text = json_string_list_text(&reviewed_by);
                let findings_text = if score == policy.pass_score {
                    None
                } else {
                    report.get("findings").map(|value| {
                        serde_json::to_string(value).unwrap_or_else(|_| "[]".to_string())
                    })
                };
                tx.execute(
                    "UPDATE units SET attempts = ?1, score = ?2, report = ?3, reviewed_at = ?4, last_reviewer = ?5, reviewed_by = ?6, findings = ?7 WHERE id = ?8",
                    rusqlite::params![
                        next_attempts,
                        score,
                        report_text,
                        now_iso,
                        owner,
                        reviewed_text,
                        findings_text,
                        unit_id
                    ],
                )?;
                tx.execute(
                    "INSERT INTO reviews(unit, round, score, reviewer, report, at) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![unit_id, next_attempts, score, owner, report_text, now_iso],
                )?;
                tx.execute(
                    "DELETE FROM claims WHERE unit = ?1",
                    rusqlite::params![unit_id],
                )?;
                if score == policy.pass_score {
                    tx.execute(
                        "UPDATE units SET stage = 'done', status = 'done', findings = NULL, done_at = ?1 WHERE id = ?2",
                        rusqlite::params![now_iso, unit_id],
                    )?;
                    let mut details = serde_json::Map::new();
                    details.insert("score".to_string(), Value::from(score));
                    details.insert("reviewer".to_string(), Value::String(owner.to_string()));
                    if let Some(note) = opts.note {
                        details.insert("note".to_string(), Value::String(note));
                    }
                    insert_event_tx(
                        &tx,
                        &now_iso,
                        "review-pass",
                        unit_id,
                        &Value::Object(details),
                    )?;
                } else {
                    tx.execute(
                        "UPDATE units SET stage = 'review', status = 'needs_fix' WHERE id = ?1",
                        rusqlite::params![unit_id],
                    )?;
                    let mut details = serde_json::Map::new();
                    details.insert("score".to_string(), Value::from(score));
                    details.insert("reviewer".to_string(), Value::String(owner.to_string()));
                    if let Some(note) = opts.note {
                        details.insert("note".to_string(), Value::String(note));
                    }
                    insert_event_tx(
                        &tx,
                        &now_iso,
                        "review-fail",
                        unit_id,
                        &Value::Object(details),
                    )?;
                    if next_attempts >= policy.max_rounds {
                        tx.execute(
                            "UPDATE units SET stage = 'blocked', status = 'blocked', blocked_stage = 'review' WHERE id = ?1",
                            rusqlite::params![unit_id],
                        )?;
                        let reason = format!(
                            "unidade {unit_id} nao alcancou 10 em {} rodadas (ultima nota {score})",
                            policy.max_rounds
                        );
                        block_category_tx(
                            &tx,
                            &unit_category,
                            &reason,
                            now_ms,
                            &now_iso,
                            active.clone(),
                        )?;
                    }
                }
                let _ = (translated_by, fixed_by);
            }
            "fixer" => {
                let report = opts.report.as_ref().ok_or_else(|| {
                    StoreError::InvalidInput("--report is required for correction".to_string())
                })?;
                let report_text = serde_json::to_string(report)?;
                reject_state_marker(&report_text)?;
                tx.execute(
                    "UPDATE units SET correction_report = ?1, stage = 'review', status = 'pending', fixed_at = ?2, fixed_by = ?3 WHERE id = ?4",
                    rusqlite::params![report_text, now_iso, owner, unit_id],
                )?;
                let mut details = serde_json::Map::new();
                details.insert(
                    "report".to_string(),
                    Value::String(report_text.chars().take(512).collect()),
                );
                if let Some(note) = opts.note {
                    details.insert("note".to_string(), Value::String(note));
                }
                tx.execute(
                    "DELETE FROM claims WHERE unit = ?1",
                    rusqlite::params![unit_id],
                )?;
                insert_event_tx(
                    &tx,
                    &now_iso,
                    "correction-done",
                    unit_id,
                    &Value::Object(details),
                )?;
            }
            _ => {
                return Err(StoreError::InvalidInput(format!(
                    "--role must be one of {}",
                    ROLES.join(", ")
                )));
            }
        }
        set_meta(&tx, "updated_at", &now_iso)?;
        tx.commit()?;
        Ok(())
    }

    /// Avanca a categoria (`advance_category` ~813-884).
    pub fn advance(&mut self, category_id: &str, allow_unverified: bool) -> Result<(), StoreError> {
        let now_ms = self.clock.now_ms();
        let now_iso = self.clock.now_iso();
        let tx = self.conn.transaction()?;
        recover_expired_tx(&tx, now_ms, &now_iso)?;
        let policy = get_policy_tx(&tx)?;
        let (stage, status): (String, String) = tx
            .query_row(
                "SELECT stage, status FROM categories WHERE id = ?1",
                rusqlite::params![category_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    StoreError::NotFound(format!("unknown category: {category_id}"))
                }
                other => StoreError::Db(other),
            })?;
        if status == "blocked" {
            return Err(StoreError::Precondition(format!(
                "category {category_id} is blocked"
            )));
        }
        let mut unit_rows: Vec<(String, String, String, Option<i64>)> = Vec::new();
        {
            let mut stmt =
                tx.prepare("SELECT id, stage, status, score FROM units WHERE category = ?1")?;
            let rows = stmt.query_map(rusqlite::params![category_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                ))
            })?;
            for row in rows {
                unit_rows.push(row?);
            }
        }
        match stage.as_str() {
            "analysis" => {
                let pending: Vec<String> = unit_rows
                    .iter()
                    .filter(|(_, unit_stage, _, _)| {
                        unit_stage == "pending" || unit_stage == "analyzing"
                    })
                    .map(|(id, _, _, _)| id.clone())
                    .collect();
                if !pending.is_empty() {
                    return Err(StoreError::Precondition(format!(
                        "analysis is not complete: {}",
                        pending.join(", ")
                    )));
                }
                tx.execute(
                    "UPDATE categories SET stage = 'translation' WHERE id = ?1",
                    rusqlite::params![category_id],
                )?;
                let mut details = serde_json::Map::new();
                details.insert(
                    "stage".to_string(),
                    Value::String("translation".to_string()),
                );
                insert_event_tx(
                    &tx,
                    &now_iso,
                    "category-stage",
                    category_id,
                    &Value::Object(details),
                )?;
            }
            "translation" => {
                let pending: Vec<String> = unit_rows
                    .iter()
                    .filter(|(_, unit_stage, _, _)| {
                        unit_stage == "translation" || unit_stage == "translating"
                    })
                    .map(|(id, _, _, _)| id.clone())
                    .collect();
                if !pending.is_empty() {
                    return Err(StoreError::Precondition(format!(
                        "translation is not complete: {}",
                        pending.join(", ")
                    )));
                }
                tx.execute(
                    "UPDATE categories SET stage = 'review' WHERE id = ?1",
                    rusqlite::params![category_id],
                )?;
                let mut details = serde_json::Map::new();
                details.insert("stage".to_string(), Value::String("review".to_string()));
                insert_event_tx(
                    &tx,
                    &now_iso,
                    "category-stage",
                    category_id,
                    &Value::Object(details),
                )?;
            }
            "review" => {
                let pending: Vec<String> = unit_rows
                    .iter()
                    .filter(|(_, unit_stage, unit_status, _)| {
                        !(unit_stage == "done" && unit_status == "done")
                    })
                    .map(|(id, _, _, _)| id.clone())
                    .collect();
                if !pending.is_empty() {
                    return Err(StoreError::Precondition(format!(
                        "review is not complete: {}",
                        pending.join(", ")
                    )));
                }
                if unit_rows.is_empty() {
                    return Err(StoreError::Precondition(
                        "review is not complete: categoria sem unidades".to_string(),
                    ));
                }
                let minimum = unit_rows
                    .iter()
                    .map(|(_, _, _, score)| score.unwrap_or(0))
                    .min()
                    .unwrap_or(0);
                tx.execute(
                    "UPDATE categories SET stage = 'consistency', score = ?1 WHERE id = ?2",
                    rusqlite::params![minimum, category_id],
                )?;
                let mut details = serde_json::Map::new();
                details.insert(
                    "stage".to_string(),
                    Value::String("consistency".to_string()),
                );
                details.insert("score".to_string(), Value::from(minimum));
                insert_event_tx(
                    &tx,
                    &now_iso,
                    "category-stage",
                    category_id,
                    &Value::Object(details),
                )?;
            }
            "consistency" => {
                let current: Option<i64> = tx.query_row(
                    "SELECT consistency_score FROM categories WHERE id = ?1",
                    rusqlite::params![category_id],
                    |row| row.get(0),
                )?;
                if current != Some(policy.pass_score) {
                    let shown = current.map_or("none".to_string(), |number| number.to_string());
                    return Err(StoreError::Precondition(format!(
                        "consistency score must be {} (current: {shown})",
                        policy.pass_score
                    )));
                }
                tx.execute(
                    "UPDATE categories SET stage = 'verification' WHERE id = ?1",
                    rusqlite::params![category_id],
                )?;
                let mut details = serde_json::Map::new();
                details.insert(
                    "stage".to_string(),
                    Value::String("verification".to_string()),
                );
                insert_event_tx(
                    &tx,
                    &now_iso,
                    "category-stage",
                    category_id,
                    &Value::Object(details),
                )?;
            }
            "verification" => {
                let mut gates: Vec<(String, String)> = Vec::new();
                {
                    let mut stmt =
                        tx.prepare("SELECT name, status FROM gates WHERE category = ?1")?;
                    let rows = stmt.query_map(rusqlite::params![category_id], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })?;
                    for row in rows {
                        gates.push(row?);
                    }
                }
                let mut missing: Vec<String> = Vec::new();
                let mut failed: Vec<String> = Vec::new();
                let mut skipped: Vec<String> = Vec::new();
                for required in &policy.required_gates {
                    match gates.iter().find(|(name, _)| name == required) {
                        None => missing.push(required.clone()),
                        Some((_, status)) if status == "fail" => failed.push(required.clone()),
                        Some((_, status)) if status == "skip" => skipped.push(required.clone()),
                        _ => {}
                    }
                }
                if !missing.is_empty() {
                    return Err(StoreError::Precondition(format!(
                        "missing gates: {}",
                        missing.join(", ")
                    )));
                }
                if !failed.is_empty() {
                    return Err(StoreError::Precondition(format!(
                        "failed gates: {}",
                        failed.join(", ")
                    )));
                }
                if !skipped.is_empty() && !allow_unverified {
                    return Err(StoreError::Precondition(format!(
                        "gates skipped (use --allow-unverified to accept): {}",
                        skipped.join(", ")
                    )));
                }
                let gate_status = if skipped.is_empty() {
                    "pass"
                } else {
                    "unverified"
                };
                tx.execute(
                    "UPDATE categories SET status = 'done', stage = 'done', score = ?1, done_at = ?2, gate_status = ?3 WHERE id = ?4",
                    rusqlite::params![policy.pass_score, now_iso, gate_status, category_id],
                )?;
                let mut details = serde_json::Map::new();
                details.insert(
                    "gate_status".to_string(),
                    Value::String(gate_status.to_string()),
                );
                insert_event_tx(
                    &tx,
                    &now_iso,
                    "category-done",
                    category_id,
                    &Value::Object(details),
                )?;
                // Ativa a proxima categoria nao concluida.
                let following: Option<(String, String, String)> = {
                    let mut stmt = tx.prepare(
                        "SELECT id, stage, status FROM categories WHERE status != 'done' ORDER BY ord ASC LIMIT 1",
                    )?;
                    let rows = stmt.query_map([], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    })?;
                    let mut found = None;
                    for row in rows {
                        found = Some(row?);
                    }
                    found
                };
                match following {
                    Some((next_id, next_stage, next_status)) => {
                        set_meta(&tx, "active_category", &next_id)?;
                        if next_status == "pending" {
                            tx.execute(
                                "UPDATE categories SET status = 'active', stage = 'analysis' WHERE id = ?1",
                                rusqlite::params![next_id],
                            )?;
                            let mut activated = serde_json::Map::new();
                            activated
                                .insert("stage".to_string(), Value::String("analysis".to_string()));
                            insert_event_tx(
                                &tx,
                                &now_iso,
                                "category-activated",
                                &next_id,
                                &Value::Object(activated),
                            )?;
                        } else {
                            let mut focused = serde_json::Map::new();
                            focused.insert("stage".to_string(), Value::String(next_stage));
                            focused
                                .insert("status".to_string(), Value::String(next_status.clone()));
                            insert_event_tx(
                                &tx,
                                &now_iso,
                                "category-focus",
                                &next_id,
                                &Value::Object(focused),
                            )?;
                            if next_status == "blocked" {
                                let mut blocked = serde_json::Map::new();
                                blocked.insert(
                                    "reason".to_string(),
                                    Value::String("next category is blocked".to_string()),
                                );
                                insert_event_tx(
                                    &tx,
                                    &now_iso,
                                    "pipeline-blocked",
                                    &next_id,
                                    &Value::Object(blocked),
                                )?;
                            }
                        }
                    }
                    None => {
                        set_meta(&tx, "active_category", "")?;
                        insert_event_tx(
                            &tx,
                            &now_iso,
                            "pipeline-done",
                            "all",
                            &Value::Object(serde_json::Map::new()),
                        )?;
                    }
                }
            }
            other => {
                return Err(StoreError::Precondition(format!(
                    "category {category_id} cannot advance from stage {other}"
                )));
            }
        }
        set_meta(&tx, "updated_at", &now_iso)?;
        tx.commit()?;
        Ok(())
    }

    /// Regista a consistencia da categoria (queue.py `command_consistency`).
    pub fn record_consistency(
        &mut self,
        category_id: &str,
        score: i64,
        report: &Value,
    ) -> Result<(), StoreError> {
        if !(0..=10).contains(&score) {
            return Err(StoreError::InvalidInput(
                "score must be between 0 and 10".to_string(),
            ));
        }
        let now_ms = self.clock.now_ms();
        let now_iso = self.clock.now_iso();
        let tx = self.conn.transaction()?;
        recover_expired_tx(&tx, now_ms, &now_iso)?;
        let policy = get_policy_tx(&tx)?;
        let active = get_active_tx(&tx)?;
        if active.as_deref() != Some(category_id) {
            return Err(StoreError::Precondition(format!(
                "category {category_id} is not the active category"
            )));
        }
        let (stage, status, consistency_score, consistency_round): (String, String, Option<i64>, i64) = tx
            .query_row(
                "SELECT stage, status, consistency_score, consistency_round FROM categories WHERE id = ?1",
                rusqlite::params![category_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    StoreError::NotFound(format!("unknown category: {category_id}"))
                }
                other => StoreError::Db(other),
            })?;
        if stage != "consistency" {
            return Err(StoreError::Precondition(format!(
                "category {category_id} is not in consistency"
            )));
        }
        if status == "blocked" {
            return Err(StoreError::Precondition(format!(
                "category {category_id} is blocked"
            )));
        }
        if consistency_score == Some(policy.pass_score) {
            return Err(StoreError::Precondition(format!(
                "category {category_id} consistency already passed"
            )));
        }
        if consistency_round >= policy.max_rounds {
            return Err(StoreError::Precondition(format!(
                "category {category_id} reached the consistency round cap; unblock it to consistency to restart"
            )));
        }
        validate_review_report_value(
            report,
            score,
            None,
            Some(category_id),
            consistency_round + 1,
        )?;
        let report_text = serde_json::to_string(report)?;
        reject_state_marker(&report_text)?;
        let next_round = consistency_round + 1;
        tx.execute(
            "UPDATE categories SET consistency_round = ?1, consistency_score = ?2, consistency_report = ?3 WHERE id = ?4",
            rusqlite::params![next_round, score, report_text, category_id],
        )?;
        if score == policy.pass_score {
            tx.execute(
                "UPDATE categories SET status = 'active', blockers = '[]' WHERE id = ?1",
                rusqlite::params![category_id],
            )?;
            let mut details = serde_json::Map::new();
            details.insert("score".to_string(), Value::from(score));
            insert_event_tx(
                &tx,
                &now_iso,
                "consistency-pass",
                category_id,
                &Value::Object(details),
            )?;
        } else if next_round >= policy.max_rounds {
            let reason = format!(
                "consistencia nao alcancou 10 em {} rodadas (ultima nota {score})",
                policy.max_rounds
            );
            block_category_tx(&tx, category_id, &reason, now_ms, &now_iso, active.clone())?;
        } else {
            tx.execute(
                "UPDATE categories SET status = 'needs_fix' WHERE id = ?1",
                rusqlite::params![category_id],
            )?;
            let mut details = serde_json::Map::new();
            details.insert("score".to_string(), Value::from(score));
            insert_event_tx(
                &tx,
                &now_iso,
                "consistency-fail",
                category_id,
                &Value::Object(details),
            )?;
        }
        set_meta(&tx, "updated_at", &now_iso)?;
        tx.commit()?;
        Ok(())
    }

    /// Regista um gate (`--status skip` exige `--note`; queue.py `command_gate`).
    pub fn record_gate(
        &mut self,
        category_id: &str,
        name: &str,
        status: &str,
        note: &str,
    ) -> Result<(), StoreError> {
        if name.trim().is_empty() {
            return Err(StoreError::InvalidInput("--name is required".to_string()));
        }
        if !["pass", "fail", "skip"].contains(&status) {
            return Err(StoreError::InvalidInput(
                "--status must be pass, fail or skip".to_string(),
            ));
        }
        if status == "skip" && note.trim().is_empty() {
            return Err(StoreError::InvalidInput(
                "--note is required when a gate is skipped".to_string(),
            ));
        }
        reject_state_marker(note)?;
        reject_state_marker(name)?;
        let now_iso = self.clock.now_iso();
        let tx = self.conn.transaction()?;
        let stage: String = tx
            .query_row(
                "SELECT stage FROM categories WHERE id = ?1",
                rusqlite::params![category_id],
                |row| row.get(0),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    StoreError::NotFound(format!("unknown category: {category_id}"))
                }
                other => StoreError::Db(other),
            })?;
        if stage != "verification" {
            return Err(StoreError::Precondition(format!(
                "category {category_id} is not in verification"
            )));
        }
        tx.execute(
            "INSERT INTO gates(category, name, status, note, at) VALUES(?1, ?2, ?3, ?4, ?5) ON CONFLICT(category, name) DO UPDATE SET status = excluded.status, note = excluded.note, at = excluded.at",
            rusqlite::params![category_id, name, status, note, now_iso],
        )?;
        let mut details = serde_json::Map::new();
        details.insert("name".to_string(), Value::String(name.to_string()));
        details.insert("status".to_string(), Value::String(status.to_string()));
        details.insert("note".to_string(), Value::String(note.to_string()));
        insert_event_tx(&tx, &now_iso, "gate", category_id, &Value::Object(details))?;
        set_meta(&tx, "updated_at", &now_iso)?;
        tx.commit()?;
        Ok(())
    }

    /// Regista um patch em lote (idempotente; queue.py `command_patch`).
    pub fn record_patch(
        &mut self,
        category_id: &str,
        name: &str,
        units: &[&str],
    ) -> Result<(), StoreError> {
        if name.trim().is_empty() {
            return Err(StoreError::InvalidInput("--name is required".to_string()));
        }
        if name.contains('/') || name.contains('\\') || !name.ends_with(".patch") {
            return Err(StoreError::InvalidInput(
                "--name must be a patch file name ending in .patch".to_string(),
            ));
        }
        reject_state_marker(name)?;
        let now_iso = self.clock.now_iso();
        let tx = self.conn.transaction()?;
        tx.query_row(
            "SELECT id FROM categories WHERE id = ?1",
            rusqlite::params![category_id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => {
                StoreError::NotFound(format!("unknown category: {category_id}"))
            }
            other => StoreError::Db(other),
        })?;
        if let Some(existing_category) = tx
            .query_row(
                "SELECT category FROM patches WHERE name = ?1",
                rusqlite::params![name],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        {
            if existing_category != category_id {
                return Err(StoreError::Precondition(format!(
                    "patch {name} is already registered under {existing_category}"
                )));
            }
        } else {
            tx.execute(
                "INSERT INTO patches(name, category, at) VALUES(?1, ?2, ?3)",
                rusqlite::params![name, category_id, now_iso],
            )?;
        }
        for unit in units {
            let unit_category: String = tx
                .query_row(
                    "SELECT category FROM units WHERE id = ?1",
                    rusqlite::params![unit],
                    |row| row.get(0),
                )
                .map_err(|error| match error {
                    rusqlite::Error::QueryReturnedNoRows => {
                        StoreError::NotFound(format!("unknown unit: {unit}"))
                    }
                    other => StoreError::Db(other),
                })?;
            if unit_category != category_id {
                return Err(StoreError::Precondition(format!(
                    "unit {unit} does not belong to {category_id}"
                )));
            }
            tx.execute(
                "INSERT INTO patch_units(patch, unit) VALUES(?1, ?2) ON CONFLICT(patch, unit) DO NOTHING",
                rusqlite::params![name, unit],
            )?;
        }
        let mut details = serde_json::Map::new();
        details.insert("name".to_string(), Value::String(name.to_string()));
        insert_event_tx(&tx, &now_iso, "patch", category_id, &Value::Object(details))?;
        set_meta(&tx, "updated_at", &now_iso)?;
        tx.commit()?;
        Ok(())
    }

    /// Define as unidades de um patch (queue.py `set-patch-units`).
    pub fn set_patch_units(&mut self, patch: &str, units: &[&str]) -> Result<(), StoreError> {
        if units.is_empty() {
            return Err(StoreError::InvalidInput(
                "at least one --unit is required".to_string(),
            ));
        }
        let now_iso = self.clock.now_iso();
        let tx = self.conn.transaction()?;
        let category: String = tx
            .query_row(
                "SELECT category FROM patches WHERE name = ?1",
                rusqlite::params![patch],
                |row| row.get(0),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    StoreError::NotFound(format!("unknown patch: {patch}"))
                }
                other => StoreError::Db(other),
            })?;
        tx.query_row(
            "SELECT id FROM categories WHERE id = ?1",
            rusqlite::params![category],
            |row| row.get::<_, String>(0),
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => {
                StoreError::NotFound(format!("patch {patch} has no category"))
            }
            other => StoreError::Db(other),
        })?;
        tx.execute(
            "DELETE FROM patch_units WHERE patch = ?1",
            rusqlite::params![patch],
        )?;
        for unit in units {
            let unit_category: String = tx
                .query_row(
                    "SELECT category FROM units WHERE id = ?1",
                    rusqlite::params![unit],
                    |row| row.get(0),
                )
                .map_err(|error| match error {
                    rusqlite::Error::QueryReturnedNoRows => {
                        StoreError::NotFound(format!("unknown unit: {unit}"))
                    }
                    other => StoreError::Db(other),
                })?;
            if unit_category != category {
                return Err(StoreError::Precondition(format!(
                    "unit {unit} is not in {category}"
                )));
            }
            tx.execute(
                "INSERT INTO patch_units(patch, unit) VALUES(?1, ?2) ON CONFLICT(patch, unit) DO NOTHING",
                rusqlite::params![patch, unit],
            )?;
        }
        let mut details = serde_json::Map::new();
        details.insert(
            "units".to_string(),
            Value::Array(
                units
                    .iter()
                    .map(|unit| Value::String(unit.to_string()))
                    .collect(),
            ),
        );
        insert_event_tx(&tx, &now_iso, "patch-units", patch, &Value::Object(details))?;
        set_meta(&tx, "updated_at", &now_iso)?;
        tx.commit()?;
        Ok(())
    }

    /// Regista um overlay sob `overlays/` (idempotente; queue.py `command_overlay`).
    pub fn record_overlay(
        &mut self,
        category_id: &str,
        path: &str,
        units: &[&str],
    ) -> Result<(), StoreError> {
        check_overlay_path(path)?;
        reject_state_marker(path)?;
        let now_iso = self.clock.now_iso();
        let tx = self.conn.transaction()?;
        tx.query_row(
            "SELECT id FROM categories WHERE id = ?1",
            rusqlite::params![category_id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => {
                StoreError::NotFound(format!("unknown category: {category_id}"))
            }
            other => StoreError::Db(other),
        })?;
        if let Some(existing_category) = tx
            .query_row(
                "SELECT category FROM overlays WHERE path = ?1",
                rusqlite::params![path],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        {
            if existing_category != category_id {
                return Err(StoreError::Precondition(format!(
                    "overlay {path} is already registered under {existing_category}"
                )));
            }
        } else {
            tx.execute(
                "INSERT INTO overlays(path, category, at) VALUES(?1, ?2, ?3)",
                rusqlite::params![path, category_id, now_iso],
            )?;
        }
        for unit in units {
            let unit_category: String = tx
                .query_row(
                    "SELECT category FROM units WHERE id = ?1",
                    rusqlite::params![unit],
                    |row| row.get(0),
                )
                .map_err(|error| match error {
                    rusqlite::Error::QueryReturnedNoRows => {
                        StoreError::NotFound(format!("unknown unit: {unit}"))
                    }
                    other => StoreError::Db(other),
                })?;
            if unit_category != category_id {
                return Err(StoreError::Precondition(format!(
                    "unit {unit} does not belong to {category_id}"
                )));
            }
            tx.execute(
                "INSERT INTO overlay_units(path, unit) VALUES(?1, ?2) ON CONFLICT(path, unit) DO NOTHING",
                rusqlite::params![path, unit],
            )?;
        }
        let mut details = serde_json::Map::new();
        details.insert("path".to_string(), Value::String(path.to_string()));
        insert_event_tx(
            &tx,
            &now_iso,
            "overlay",
            category_id,
            &Value::Object(details),
        )?;
        set_meta(&tx, "updated_at", &now_iso)?;
        tx.commit()?;
        Ok(())
    }

    /// Regista um shared override (idempotente no caminho; queue.py `command_shared_override`).
    pub fn record_shared_override(
        &mut self,
        category_id: &str,
        path: &str,
        reason: &str,
        units: &[&str],
    ) -> Result<(), StoreError> {
        if reason.trim().is_empty() {
            return Err(StoreError::InvalidInput("--reason is required".to_string()));
        }
        check_repo_path(path)?;
        reject_state_marker(path)?;
        reject_state_marker(reason)?;
        let now_iso = self.clock.now_iso();
        let tx = self.conn.transaction()?;
        tx.query_row(
            "SELECT id FROM categories WHERE id = ?1",
            rusqlite::params![category_id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => {
                StoreError::NotFound(format!("unknown category: {category_id}"))
            }
            other => StoreError::Db(other),
        })?;
        if let Some(existing_category) = tx
            .query_row(
                "SELECT category FROM shared_overrides WHERE path = ?1",
                rusqlite::params![path],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        {
            if existing_category != category_id {
                return Err(StoreError::Precondition(format!(
                    "shared override {path} is already registered under {existing_category}"
                )));
            }
        }
        tx.execute(
            "INSERT INTO shared_overrides(path, category, reason, at) VALUES(?1, ?2, ?3, ?4) ON CONFLICT(path) DO UPDATE SET reason = excluded.reason, at = excluded.at",
            rusqlite::params![path, category_id, reason, now_iso],
        )?;
        for unit in units {
            let unit_category: String = tx
                .query_row(
                    "SELECT category FROM units WHERE id = ?1",
                    rusqlite::params![unit],
                    |row| row.get(0),
                )
                .map_err(|error| match error {
                    rusqlite::Error::QueryReturnedNoRows => {
                        StoreError::NotFound(format!("unknown unit: {unit}"))
                    }
                    other => StoreError::Db(other),
                })?;
            if unit_category != category_id {
                return Err(StoreError::Precondition(format!(
                    "unit {unit} does not belong to {category_id}"
                )));
            }
            tx.execute(
                "INSERT INTO shared_override_units(path, unit) VALUES(?1, ?2) ON CONFLICT(path, unit) DO NOTHING",
                rusqlite::params![path, unit],
            )?;
        }
        let mut details = serde_json::Map::new();
        details.insert("path".to_string(), Value::String(path.to_string()));
        details.insert("reason".to_string(), Value::String(reason.to_string()));
        insert_event_tx(
            &tx,
            &now_iso,
            "shared-override",
            category_id,
            &Value::Object(details),
        )?;
        set_meta(&tx, "updated_at", &now_iso)?;
        tx.commit()?;
        Ok(())
    }

    /// Bloqueia uma unidade ou categoria (queue.py `command_block` ~1834-1859).
    pub fn block(&mut self, target: &str, reason: &str) -> Result<(), StoreError> {
        if reason.trim().is_empty() {
            return Err(StoreError::InvalidInput("--reason is required".to_string()));
        }
        reject_state_marker(reason)?;
        let now_ms = self.clock.now_ms();
        let now_iso = self.clock.now_iso();
        let tx = self.conn.transaction()?;
        let active = get_active_tx(&tx)?;
        let is_unit: bool = tx
            .query_row(
                "SELECT 1 FROM units WHERE id = ?1",
                rusqlite::params![target],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        let is_category: bool = tx
            .query_row(
                "SELECT 1 FROM categories WHERE id = ?1",
                rusqlite::params![target],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if is_unit {
            let (stage, status, category, blocked_stage_raw): (
                String,
                String,
                String,
                Option<String>,
            ) = tx
                .query_row(
                    "SELECT stage, status, category, blocked_stage FROM units WHERE id = ?1",
                    rusqlite::params![target],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .map_err(StoreError::Db)?;
            if status == "done" {
                return Err(StoreError::Precondition(format!(
                    "unit {target} is done; reopen the category instead"
                )));
            }
            let next_blocked = if stage == "blocked" {
                blocked_stage_raw.unwrap_or_else(|| normalize_stage(&stage).to_string())
            } else {
                normalize_stage(&stage).to_string()
            };
            tx.execute(
                "DELETE FROM claims WHERE unit = ?1",
                rusqlite::params![target],
            )?;
            tx.execute(
                "UPDATE units SET stage = 'blocked', status = 'blocked', blocked_stage = ?1 WHERE id = ?2",
                rusqlite::params![next_blocked, target],
            )?;
            let full_reason = format!("{target}: {reason}");
            block_category_tx(&tx, &category, &full_reason, now_ms, &now_iso, active)?;
            let mut details = serde_json::Map::new();
            details.insert("reason".to_string(), Value::String(reason.to_string()));
            insert_event_tx(&tx, &now_iso, "blocked", target, &Value::Object(details))?;
        } else if is_category {
            let status: String = tx.query_row(
                "SELECT status FROM categories WHERE id = ?1",
                rusqlite::params![target],
                |row| row.get(0),
            )?;
            if status == "done" {
                return Err(StoreError::Precondition(format!(
                    "category {target} is done; use reopen"
                )));
            }
            block_category_tx(&tx, target, reason, now_ms, &now_iso, active)?;
        } else {
            return Err(StoreError::NotFound(format!(
                "unknown unit or category: {target}"
            )));
        }
        set_meta(&tx, "updated_at", &now_iso)?;
        tx.commit()?;
        Ok(())
    }

    /// Desbloqueia para um estagio anterior-ou-igual (queue.py `command_unblock`).
    #[allow(clippy::too_many_lines)]
    pub fn unblock(&mut self, target: &str, stage: &str, force: bool) -> Result<(), StoreError> {
        let valid = [
            "analysis",
            "translation",
            "review",
            "consistency",
            "verification",
        ];
        if !valid.contains(&stage) {
            return Err(StoreError::InvalidInput(
                "--stage must be analysis, translation, review, consistency or verification"
                    .to_string(),
            ));
        }
        let now_ms = self.clock.now_ms();
        let now_iso = self.clock.now_iso();
        let tx = self.conn.transaction()?;
        recover_expired_tx(&tx, now_ms, &now_iso)?;
        let policy = get_policy_tx(&tx)?;
        let is_unit: bool = tx
            .query_row(
                "SELECT 1 FROM units WHERE id = ?1",
                rusqlite::params![target],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        let is_category: bool = tx
            .query_row(
                "SELECT 1 FROM categories WHERE id = ?1",
                rusqlite::params![target],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if is_unit {
            if !["analysis", "translation", "review"].contains(&stage) {
                return Err(StoreError::InvalidInput(
                    "a unit can only be unblocked into analysis, translation or review".to_string(),
                ));
            }
            let (
                unit_status,
                unit_stage,
                unit_category,
                blocked_stage_raw,
                translation_report,
                translated_by,
            ): (String, String, String, Option<String>, Option<String>, Option<String>) = tx
                .query_row(
                    "SELECT status, stage, category, blocked_stage, translation_report, translated_by FROM units WHERE id = ?1",
                    rusqlite::params![target],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                        ))
                    },
                )
                .map_err(StoreError::Db)?;
            if unit_status != "blocked" && !force {
                return Err(StoreError::Precondition(format!(
                    "unit {target} is not blocked (use --force to override)"
                )));
            }
            let blocked_stage = if let Some(value) = blocked_stage_raw {
                value
            } else if force && unit_status == "done" && stage == "review" {
                "review".to_string()
            } else {
                normalize_stage(&unit_stage).to_string()
            };
            if unit_status == "done" && stage != "review" {
                return Err(StoreError::Precondition(format!(
                    "a done unit can only be reopened into review (got {stage}); use reopen for a full reset"
                )));
            }
            let wanted = stage_order(stage).unwrap_or(-1);
            let was = stage_order(&blocked_stage).unwrap_or(-1);
            if stage_order(&blocked_stage).is_none() || wanted > was {
                return Err(StoreError::Precondition(format!(
                    "cannot unblock {target} from {blocked_stage} to {stage}; only earlier stages are allowed"
                )));
            }
            if stage == "review" {
                if translation_report.is_none() {
                    return Err(StoreError::Precondition(format!(
                        "unit {target} has no translation report"
                    )));
                }
                if translated_by.is_none() {
                    return Err(StoreError::Precondition(format!(
                        "unit {target} has no translator identity"
                    )));
                }
                let mut siblings: Vec<String> = Vec::new();
                {
                    let mut stmt =
                        tx.prepare("SELECT id, status FROM units WHERE category = ?1")?;
                    let rows = stmt.query_map(rusqlite::params![unit_category], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })?;
                    for row in rows {
                        let (id, status) = row?;
                        if id != target && status == "blocked" {
                            siblings.push(id);
                        }
                    }
                }
                if !siblings.is_empty() {
                    return Err(StoreError::Precondition(format!(
                        "category has other blocked unit(s): {}; unblock the category instead",
                        siblings.join(", ")
                    )));
                }
            }
            if stage != "review" {
                let (next_stage, next_status) = if stage == "analysis" {
                    ("pending", "pending")
                } else {
                    ("translation", "pending")
                };
                tx.execute(
                    "UPDATE units SET stage = ?1, status = ?2, score = NULL, attempts = 0, findings = NULL, done_at = NULL, reviewed_at = NULL, blocked_stage = NULL, translated_by = NULL, fixed_by = NULL, last_reviewer = NULL, reviewed_by = '[]' WHERE category = ?3",
                    rusqlite::params![next_stage, next_status, unit_category],
                )?;
                tx.execute(
                    "DELETE FROM claims WHERE unit IN (SELECT id FROM units WHERE category = ?1)",
                    rusqlite::params![unit_category],
                )?;
            } else {
                tx.execute(
                    "UPDATE units SET stage = 'review', status = 'pending', score = NULL, attempts = 0, findings = NULL, done_at = NULL, reviewed_at = NULL, blocked_stage = NULL WHERE id = ?1",
                    rusqlite::params![target],
                )?;
                tx.execute(
                    "DELETE FROM claims WHERE unit = ?1",
                    rusqlite::params![target],
                )?;
            }
            ensure_only_active_tx(&tx, &unit_category, force, None, now_ms, &now_iso)?;
            tx.execute(
                "UPDATE categories SET stage = ?1, status = 'active', blocked_stage = NULL, blockers = '[]' WHERE id = ?2",
                rusqlite::params![stage, unit_category],
            )?;
            if stage != "review" {
                tx.execute(
                    "UPDATE categories SET score = NULL, consistency_score = NULL, consistency_report = NULL, consistency_round = 0, done_at = NULL, gate_status = NULL WHERE id = ?1",
                    rusqlite::params![unit_category],
                )?;
                tx.execute(
                    "DELETE FROM gates WHERE category = ?1",
                    rusqlite::params![unit_category],
                )?;
            }
            let mut details = serde_json::Map::new();
            details.insert("stage".to_string(), Value::String(stage.to_string()));
            insert_event_tx(&tx, &now_iso, "unblocked", target, &Value::Object(details))?;
        } else if is_category {
            let (category_status, category_stage, blocked_stage_raw): (
                String,
                String,
                Option<String>,
            ) = tx
                .query_row(
                    "SELECT status, stage, blocked_stage FROM categories WHERE id = ?1",
                    rusqlite::params![target],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .map_err(StoreError::Db)?;
            if category_status != "blocked" && !force {
                return Err(StoreError::Precondition(format!(
                    "category {target} is not blocked (use --force to override)"
                )));
            }
            let blocked_stage =
                blocked_stage_raw.unwrap_or_else(|| normalize_stage(&category_stage).to_string());
            let wanted = stage_order(stage).unwrap_or(-1);
            let was = stage_order(&blocked_stage).unwrap_or(-1);
            if stage_order(&blocked_stage).is_none() || wanted > was {
                return Err(StoreError::Precondition(format!(
                    "cannot unblock {target} from {blocked_stage} to {stage}; only earlier stages are allowed"
                )));
            }
            if ["consistency", "verification"].contains(&stage) {
                let mut unfinished: Vec<String> = Vec::new();
                {
                    let mut stmt =
                        tx.prepare("SELECT id, status, score FROM units WHERE category = ?1")?;
                    let rows = stmt.query_map(rusqlite::params![target], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, Option<i64>>(2)?,
                        ))
                    })?;
                    for row in rows {
                        let (id, status, score) = row?;
                        if status != "done" || score != Some(policy.pass_score) {
                            unfinished.push(id);
                        }
                    }
                }
                if !unfinished.is_empty() {
                    return Err(StoreError::Precondition(format!(
                        "cannot unblock into {stage}: unit(s) without score 10: {}",
                        unfinished.join(", ")
                    )));
                }
            }
            if stage == "verification" {
                let consistency: Option<i64> = tx.query_row(
                    "SELECT consistency_score FROM categories WHERE id = ?1",
                    rusqlite::params![target],
                    |row| row.get(0),
                )?;
                if consistency != Some(policy.pass_score) {
                    return Err(StoreError::Precondition(
                        "consistency score must be 10 before verification".to_string(),
                    ));
                }
            }
            if stage == "review" {
                let mut blocked_units: Vec<String> = Vec::new();
                {
                    let mut stmt =
                        tx.prepare("SELECT id, status FROM units WHERE category = ?1")?;
                    let rows = stmt.query_map(rusqlite::params![target], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })?;
                    for row in rows {
                        let (id, status) = row?;
                        if status == "blocked" {
                            blocked_units.push(id);
                        }
                    }
                }
                for unit in blocked_units {
                    let (translation_report, translated_by): (Option<String>, Option<String>) = tx
                        .query_row(
                            "SELECT translation_report, translated_by FROM units WHERE id = ?1",
                            rusqlite::params![unit],
                            |row| Ok((row.get(0)?, row.get(1)?)),
                        )?;
                    if translation_report.is_none() {
                        return Err(StoreError::Precondition(format!(
                            "unit {unit} has no translation report"
                        )));
                    }
                    if translated_by.is_none() {
                        return Err(StoreError::Precondition(format!(
                            "unit {unit} has no translator identity"
                        )));
                    }
                    tx.execute(
                        "UPDATE units SET stage = 'review', status = 'pending', score = NULL, attempts = 0, findings = NULL, done_at = NULL, reviewed_at = NULL, blocked_stage = NULL WHERE id = ?1",
                        rusqlite::params![unit],
                    )?;
                    // So unidades bloqueadas perdem o claim; as demais mantem
                    // o estado e os claims, como no Python.
                    tx.execute(
                        "DELETE FROM claims WHERE unit = ?1",
                        rusqlite::params![unit],
                    )?;
                }
            } else if ["consistency", "verification"].contains(&stage) {
                tx.execute(
                    "DELETE FROM claims WHERE unit IN (SELECT id FROM units WHERE category = ?1)",
                    rusqlite::params![target],
                )?;
            } else {
                let (next_stage, next_status) = if stage == "analysis" {
                    ("pending", "pending")
                } else {
                    ("translation", "pending")
                };
                tx.execute(
                    "UPDATE units SET stage = ?1, status = ?2, score = NULL, attempts = 0, findings = NULL, done_at = NULL, reviewed_at = NULL, blocked_stage = NULL, translated_by = NULL, fixed_by = NULL, last_reviewer = NULL, reviewed_by = '[]' WHERE category = ?3",
                    rusqlite::params![next_stage, next_status, target],
                )?;
                tx.execute(
                    "DELETE FROM claims WHERE unit IN (SELECT id FROM units WHERE category = ?1)",
                    rusqlite::params![target],
                )?;
            }
            ensure_only_active_tx(&tx, target, force, None, now_ms, &now_iso)?;
            tx.execute(
                "UPDATE categories SET stage = ?1, status = 'active', blocked_stage = NULL, blockers = '[]' WHERE id = ?2",
                rusqlite::params![stage, target],
            )?;
            if ["review", "analysis", "translation"].contains(&stage) {
                tx.execute(
                    "UPDATE categories SET score = NULL, consistency_score = NULL, consistency_report = NULL, consistency_round = 0, done_at = NULL, gate_status = NULL WHERE id = ?1",
                    rusqlite::params![target],
                )?;
                tx.execute(
                    "DELETE FROM gates WHERE category = ?1",
                    rusqlite::params![target],
                )?;
            }
            if stage == "verification" {
                tx.execute(
                    "DELETE FROM gates WHERE category = ?1",
                    rusqlite::params![target],
                )?;
            }
            if stage == "consistency" {
                tx.execute(
                    "UPDATE categories SET consistency_score = NULL, consistency_report = NULL, consistency_round = 0, done_at = NULL, gate_status = NULL WHERE id = ?1",
                    rusqlite::params![target],
                )?;
                tx.execute(
                    "DELETE FROM gates WHERE category = ?1",
                    rusqlite::params![target],
                )?;
            }
            let mut details = serde_json::Map::new();
            details.insert("stage".to_string(), Value::String(stage.to_string()));
            insert_event_tx(&tx, &now_iso, "unblocked", target, &Value::Object(details))?;
        } else {
            return Err(StoreError::NotFound(format!(
                "unknown unit or category: {target}"
            )));
        }
        set_meta(&tx, "updated_at", &now_iso)?;
        tx.commit()?;
        Ok(())
    }

    /// Reabre categoria `done`/`blocked` (queue.py `command_reopen` ~2055-2101).
    pub fn reopen(
        &mut self,
        category_id: &str,
        reason: &str,
        force: bool,
    ) -> Result<(), StoreError> {
        if reason.trim().is_empty() {
            return Err(StoreError::InvalidInput("--reason is required".to_string()));
        }
        reject_state_marker(reason)?;
        let now_ms = self.clock.now_ms();
        let now_iso = self.clock.now_iso();
        let tx = self.conn.transaction()?;
        recover_expired_tx(&tx, now_ms, &now_iso)?;
        let (status, blocked_stage_raw): (String, Option<String>) = tx
            .query_row(
                "SELECT status, blocked_stage FROM categories WHERE id = ?1",
                rusqlite::params![category_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    StoreError::NotFound(format!("unknown category: {category_id}"))
                }
                other => StoreError::Db(other),
            })?;
        if status != "done" && status != "blocked" {
            return Err(StoreError::Precondition(format!(
                "only done or blocked categories can be reopened (current: {status})"
            )));
        }
        if status == "blocked" && !force {
            return Err(StoreError::Precondition(format!(
                "reopening blocked category {category_id} requires --force"
            )));
        }
        ensure_only_active_tx(&tx, category_id, force, Some(reason), now_ms, &now_iso)?;
        let target_stage = if status == "blocked" {
            match blocked_stage_raw.as_deref() {
                Some("analysis" | "translation") => blocked_stage_raw
                    .clone()
                    .unwrap_or_else(|| "review".to_string()),
                _ => "review".to_string(),
            }
        } else {
            "review".to_string()
        };
        tx.execute(
            "UPDATE categories SET stage = ?1, status = 'active', blocked_stage = NULL, done_at = NULL, gate_status = NULL, score = NULL, consistency_score = NULL, consistency_report = NULL, consistency_round = 0, blockers = '[]' WHERE id = ?2",
            rusqlite::params![target_stage, category_id],
        )?;
        tx.execute(
            "DELETE FROM gates WHERE category = ?1",
            rusqlite::params![category_id],
        )?;
        let (unit_stage, unit_status) = if target_stage == "analysis" {
            ("pending", "pending")
        } else {
            (target_stage.as_str(), "pending")
        };
        if target_stage == "review" {
            tx.execute(
                "UPDATE units SET stage = ?1, status = ?2, score = NULL, attempts = 0, findings = NULL, done_at = NULL, reviewed_at = NULL, blocked_stage = NULL WHERE category = ?3",
                rusqlite::params![unit_stage, unit_status, category_id],
            )?;
        } else {
            tx.execute(
                "UPDATE units SET stage = ?1, status = ?2, score = NULL, attempts = 0, findings = NULL, done_at = NULL, reviewed_at = NULL, blocked_stage = NULL, translated_by = NULL, fixed_by = NULL, last_reviewer = NULL, reviewed_by = '[]' WHERE category = ?3",
                rusqlite::params![unit_stage, unit_status, category_id],
            )?;
        }
        tx.execute(
            "DELETE FROM claims WHERE unit IN (SELECT id FROM units WHERE category = ?1)",
            rusqlite::params![category_id],
        )?;
        let mut details = serde_json::Map::new();
        details.insert("reason".to_string(), Value::String(reason.to_string()));
        insert_event_tx(
            &tx,
            &now_iso,
            "reopened",
            category_id,
            &Value::Object(details),
        )?;
        set_meta(&tx, "updated_at", &now_iso)?;
        tx.commit()?;
        Ok(())
    }

    /// Libera o claim ativo (queue.py `command_release`).
    pub fn release(&mut self, unit_id: &str, owner: &str) -> Result<(), StoreError> {
        if owner.trim().is_empty() {
            return Err(StoreError::InvalidInput("--owner is required".to_string()));
        }
        let now_ms = self.clock.now_ms();
        let now_iso = self.clock.now_iso();
        let tx = self.conn.transaction()?;
        let (claim_owner, claim_role, expires_at): (String, String, String) = tx
            .query_row(
                "SELECT owner, role, expires_at FROM claims WHERE unit = ?1",
                rusqlite::params![unit_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    StoreError::Precondition(format!("unit {unit_id} has no active claim"))
                }
                other => StoreError::Db(other),
            })?;
        if is_expired(Some(expires_at.as_str()), now_ms) {
            return Err(StoreError::Precondition(format!(
                "unit {unit_id} has an expired claim; run recover"
            )));
        }
        if claim_owner != owner {
            return Err(StoreError::Precondition(format!(
                "unit {unit_id} is claimed by {claim_owner}, not {owner}"
            )));
        }
        revert_unit_tx(&tx, unit_id, &claim_role)?;
        let mut details = serde_json::Map::new();
        details.insert("owner".to_string(), Value::String(owner.to_string()));
        details.insert("role".to_string(), Value::String(claim_role));
        insert_event_tx(&tx, &now_iso, "release", unit_id, &Value::Object(details))?;
        set_meta(&tx, "updated_at", &now_iso)?;
        tx.commit()?;
        Ok(())
    }

    /// Renova o lease (`heartbeat`; queue.py `command_heartbeat`).
    pub fn heartbeat(
        &mut self,
        unit_id: &str,
        owner: &str,
        lease_minutes: i64,
    ) -> Result<(), StoreError> {
        if owner.trim().is_empty() {
            return Err(StoreError::InvalidInput("--owner is required".to_string()));
        }
        if lease_minutes <= 0 {
            return Err(StoreError::InvalidInput(
                "--lease-minutes must be positive".to_string(),
            ));
        }
        let now_ms = self.clock.now_ms();
        let now_iso = self.clock.now_iso();
        let tx = self.conn.transaction()?;
        let (claim_owner, expires_at): (String, String) = tx
            .query_row(
                "SELECT owner, expires_at FROM claims WHERE unit = ?1",
                rusqlite::params![unit_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    StoreError::Precondition(format!("unit {unit_id} has no active claim"))
                }
                other => StoreError::Db(other),
            })?;
        if is_expired(Some(expires_at.as_str()), now_ms) {
            return Err(StoreError::Precondition(format!(
                "unit {unit_id} has an expired claim; run recover"
            )));
        }
        if claim_owner != owner {
            return Err(StoreError::Precondition(format!(
                "unit {unit_id} is claimed by {claim_owner}, not {owner}"
            )));
        }
        let next_expiry =
            format_iso_ms(now_ms.saturating_add((lease_minutes as u64).saturating_mul(60_000)));
        tx.execute(
            "UPDATE claims SET expires_at = ?1 WHERE unit = ?2",
            rusqlite::params![next_expiry, unit_id],
        )?;
        let mut details = serde_json::Map::new();
        details.insert("owner".to_string(), Value::String(owner.to_string()));
        details.insert("lease_minutes".to_string(), Value::from(lease_minutes));
        insert_event_tx(&tx, &now_iso, "heartbeat", unit_id, &Value::Object(details))?;
        set_meta(&tx, "updated_at", &now_iso)?;
        tx.commit()?;
        Ok(())
    }

    /// Reverte claims expirados; devolve quantos (queue.py `recover_expired`).
    pub fn recover_expired(&mut self) -> Result<usize, StoreError> {
        let now_ms = self.clock.now_ms();
        let now_iso = self.clock.now_iso();
        let tx = self.conn.transaction()?;
        let count = recover_expired_tx(&tx, now_ms, &now_iso)?;
        if count > 0 {
            set_meta(&tx, "updated_at", &now_iso)?;
        }
        tx.commit()?;
        Ok(count)
    }

    /// Valida invariantes internos (subconjunto de `validate_state` sem catalogo).
    ///
    /// Simplificacao documentada: verificacoes que exigem o catalogo
    /// (`not in catalog`, `unit membership`, `file_count`) sao omitidas porque
    /// o store nao retem o catalogo bruto (queue.py ~1124-1130, 1211-1221,
    /// 1339-1346).
    pub fn validate(&self) -> Result<Vec<String>, StoreError> {
        let now_ms = self.clock.now_ms();
        let mut errors: Vec<String> = Vec::new();
        let version = get_meta(&self.conn, "version")?.unwrap_or_default();
        if version != STATE_VERSION.to_string() {
            errors.push(format!("unsupported state version: {version}"));
        }
        let policy_text = get_meta(&self.conn, "policy")?.unwrap_or_default();
        if policy_text.is_empty() {
            errors.push("missing policy".to_string());
            return Ok(errors);
        }
        let policy = match serde_json::from_str::<Value>(&policy_text) {
            Ok(value) => policy_from_value(&value),
            Err(_) => {
                errors.push("missing policy".to_string());
                return Ok(errors);
            }
        };
        if !(1..=3).contains(&policy.max_agents) {
            errors.push("max_agents must be an integer between 1 and 3".to_string());
        }
        if policy.max_active_categories != 1 {
            errors.push("max_active_categories must be 1".to_string());
        }
        if policy.pass_score != 10 {
            errors.push("pass_score must be 10".to_string());
        }
        if policy.max_rounds != 3 {
            errors.push("max_rounds must be exactly 3".to_string());
        }
        let active = get_meta(&self.conn, "active_category")?.unwrap_or_default();
        let active_opt = if active.is_empty() {
            None
        } else {
            Some(active.clone())
        };
        let mut categories: Vec<CategoryRow> = Vec::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT id, ord, stage, status, consistency_score, score, consistency_round, blocked_stage, blockers FROM categories",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, String>(8)?,
                ))
            })?;
            for row in rows {
                categories.push(row?);
            }
        }
        let valid_category_stages = [
            "pending",
            "analysis",
            "translation",
            "review",
            "consistency",
            "verification",
            "done",
            "blocked",
        ];
        if let Some(focus) = active_opt.clone() {
            let found = categories
                .iter()
                .find(|(id, _, _, _, _, _, _, _, _)| id == &focus);
            match found {
                None => errors.push(format!("unknown active category: {focus}")),
                Some((_, _, stage, status, _, _, _, _, _)) => {
                    if !["active", "blocked", "needs_fix"].contains(&status.as_str()) {
                        errors.push(format!(
                            "focus category {focus} must be active, blocked or needs_fix, found {status}"
                        ));
                    }
                    if status == "needs_fix" && stage != "consistency" {
                        errors.push(format!(
                            "needs_fix focus category {focus} must be in consistency"
                        ));
                    }
                    if status == "blocked" && stage != "blocked" {
                        errors.push(format!(
                            "blocked focus category {focus} must be in stage blocked"
                        ));
                    }
                    let counted = categories
                        .iter()
                        .filter(|(_, _, _, status, _, _, _, _, _)| {
                            status == "active" || status == "needs_fix"
                        })
                        .count();
                    if counted > 1 {
                        errors.push(format!(
                            "expected at most one active category, found {counted}"
                        ));
                    }
                    if counted == 1 {
                        let first = categories.iter().find(|(_, _, _, status, _, _, _, _, _)| {
                            status == "active" || status == "needs_fix"
                        });
                        if let Some((id, _, _, _, _, _, _, _, _)) = first {
                            if id != &focus {
                                errors
                                    .push(format!("active category {id} is not the focus {focus}"));
                            }
                        }
                    }
                    if counted == 0 && (status == "active" || status == "needs_fix") {
                        errors.push(format!("focus category {focus} is not counted as active"));
                    }
                }
            }
        } else if categories
            .iter()
            .any(|(_, _, _, status, _, _, _, _, _)| status != "done")
        {
            errors.push("active_category is null but not every category is done".to_string());
        }
        for (
            id,
            _,
            stage,
            status,
            consistency_score,
            _,
            consistency_round,
            blocked_stage,
            blockers_raw,
        ) in &categories
        {
            if !valid_category_stages.contains(&stage.as_str()) {
                errors.push(format!("{id}: invalid stage {stage}"));
            }
            let valid_pairs: &[(&str, &[&str])] = &[
                ("pending", &["pending"]),
                ("analysis", &["active", "needs_fix"]),
                ("translation", &["active", "needs_fix"]),
                ("review", &["active", "needs_fix"]),
                ("consistency", &["active", "needs_fix"]),
                ("verification", &["active", "needs_fix"]),
                ("done", &["done"]),
                ("blocked", &["blocked"]),
            ];
            let allowed = valid_pairs
                .iter()
                .find(|(candidate, _)| candidate == stage)
                .map(|(_, statuses)| *statuses)
                .unwrap_or(&[]);
            if !allowed.contains(&status.as_str()) {
                errors.push(format!("{id}: invalid stage/status pair {stage}/{status}"));
            }
            let blockers = json_string_list(blockers_raw);
            if status == "blocked" && blockers.is_empty() {
                errors.push(format!("{id}: blocked category has no blocker"));
            }
            if status == "blocked"
                && blocked_stage
                    .as_deref()
                    .is_none_or(|value| stage_order(value).is_none())
            {
                errors.push(format!("{id}: blocked category has no valid blocked_stage"));
            }
            if blocked_stage.is_some() && status != "blocked" {
                errors.push(format!("{id}: blocked_stage set on a non-blocked category"));
            }
            if *consistency_round > policy.max_rounds {
                errors.push(format!("{id}: consistency_round exceeds max_rounds"));
            }
            if status == "done" {
                if *consistency_score != Some(policy.pass_score) {
                    errors.push(format!(
                        "{id}: done category must have consistency score 10"
                    ));
                }
                let mut stmt = self
                    .conn
                    .prepare("SELECT id, status FROM units WHERE category = ?1")?;
                let rows = stmt.query_map(rusqlite::params![id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?;
                for row in rows {
                    let (unit, unit_status) = row?;
                    if unit_status != "done" {
                        errors.push(format!(
                            "{id}: done category has units that are not done ({unit})"
                        ));
                    }
                }
                for required in &policy.required_gates {
                    let gate: Option<(String, String)> = self
                        .conn
                        .query_row(
                            "SELECT status, note FROM gates WHERE category = ?1 AND name = ?2",
                            rusqlite::params![id, required],
                            |row| Ok((row.get(0)?, row.get(1)?)),
                        )
                        .optional()?;
                    match gate {
                        None => {
                            errors.push(format!("{id}: done category is missing gate {required}"))
                        }
                        Some((gate_status, _note)) if gate_status == "fail" => {
                            errors.push(format!("{id}: gate {required} failed"));
                        }
                        Some((gate_status, note))
                            if gate_status == "skip" && note.trim().is_empty() =>
                        {
                            errors.push(format!("{id}: skipped gate {required} has no note"));
                        }
                        _ => {}
                    }
                }
            }
            // Unidades compativeis com o estagio da categoria.
            let allowed_units: &[&str] = match stage.as_str() {
                "pending" => &["pending", "analyzing"],
                "analysis" => &["pending", "analyzing", "translation"],
                "translation" => &["translation", "translating", "review"],
                "review" => &["review", "reviewing", "fixing", "done"],
                "consistency" => &["done"],
                "verification" => &["done"],
                "done" => &["done"],
                "blocked" => &["blocked", "pending", "translation", "review", "done"],
                _ => &[],
            };
            let mut stmt = self
                .conn
                .prepare("SELECT id, stage FROM units WHERE category = ?1")?;
            let rows = stmt.query_map(rusqlite::params![id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for row in rows {
                let (unit, unit_stage) = row?;
                if !allowed_units.contains(&unit_stage.as_str()) {
                    errors.push(format!(
                        "{id}: unit {unit} has stage {unit_stage} incompatible with category stage {stage}"
                    ));
                }
            }
            // Ligacoes de patches/overlays/shared.
            let category_patches: Vec<String> = {
                let mut patch_stmt = self
                    .conn
                    .prepare("SELECT name FROM patches WHERE category = ?1")?;
                let patch_rows =
                    patch_stmt.query_map(rusqlite::params![id], |row| row.get::<_, String>(0))?;
                patch_rows.collect::<Result<Vec<_>, _>>()?
            };
            for patch in category_patches {
                let name: String = patch;
                let listed: Vec<String> = {
                    let mut stmt = self
                        .conn
                        .prepare("SELECT unit FROM patch_units WHERE patch = ?1")?;
                    let unit_rows = stmt.query_map(rusqlite::params![name], |row| row.get(0))?;
                    unit_rows.collect::<Result<Vec<String>, _>>()?
                };
                for unit in listed {
                    let unit_category: Option<String> = self
                        .conn
                        .query_row(
                            "SELECT category FROM units WHERE id = ?1",
                            rusqlite::params![unit],
                            |row| row.get(0),
                        )
                        .optional()?;
                    if unit_category.as_deref() != Some(id) {
                        errors.push(format!("patch {name}: unit {unit} is in another category"));
                    }
                }
            }
        }
        // Unidades.
        let mut units: Vec<UnitSnapshotRow> = Vec::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT id, category, stage, status, score, attempts, labels_total, labels_translated, translated_by, fixed_by, last_reviewer, done_at, blocked_stage, reviewed_by, translated_at, reviewed_at FROM units",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, String>(13)?,
                    row.get::<_, Option<String>>(14)?,
                    row.get::<_, Option<String>>(15)?,
                ))
            })?;
            for row in rows {
                units.push(row?);
            }
        }
        let valid_unit_stages = [
            "pending",
            "analyzing",
            "translation",
            "translating",
            "review",
            "reviewing",
            "fixing",
            "done",
            "blocked",
        ];
        for (
            id,
            category,
            stage,
            status,
            score,
            attempts,
            labels_total,
            labels_translated,
            translated_by,
            fixed_by,
            last_reviewer,
            done_at,
            blocked_stage,
            _reviewed_by,
            _translated_at,
            _reviewed_at,
        ) in &units
        {
            if !valid_unit_stages.contains(&stage.as_str()) {
                errors.push(format!("{id}: invalid stage {stage}"));
            }
            let known_category = categories
                .iter()
                .any(|(candidate, _, _, _, _, _, _, _, _)| candidate == category);
            if !known_category {
                errors.push(format!("{id}: unknown category {category}"));
            }
            if let Some(number) = score {
                if !matches!(number, 0..=10) {
                    errors.push(format!("{id}: score out of range"));
                }
            }
            if *attempts > policy.max_rounds && status != "blocked" {
                errors.push(format!(
                    "{id}: attempts exceed max_rounds without blocked status"
                ));
            }
            if *labels_total < 0 || *labels_translated < 0 {
                errors.push(format!("{id}: label counts must be non-negative"));
            }
            if *labels_total > 0 && *labels_translated > *labels_total {
                errors.push(format!("{id}: labels_translated exceeds labels_total"));
            }
            if status == "done" && *score != Some(policy.pass_score) {
                errors.push(format!("{id}: done unit must have score 10"));
            }
            if status == "done" {
                if translated_by.is_none() {
                    errors.push(format!("{id}: done unit has no translated_by"));
                }
                if last_reviewer.is_none() {
                    errors.push(format!("{id}: done unit has no last_reviewer"));
                }
                if translated_by.is_some() && last_reviewer.as_deref() == translated_by.as_deref() {
                    errors.push(format!("{id}: reviewer must differ from translator"));
                }
                if fixed_by.is_some() && last_reviewer.as_deref() == fixed_by.as_deref() {
                    errors.push(format!("{id}: reviewer must differ from fixer"));
                }
            }
            if done_at.is_some() && status != "done" {
                errors.push(format!("{id}: done_at set on a non-done unit"));
            }
            if blocked_stage.is_some() && status != "blocked" {
                errors.push(format!("{id}: blocked_stage set on a non-blocked unit"));
            }
            if status == "blocked"
                && blocked_stage
                    .as_deref()
                    .is_none_or(|value| stage_order(value).is_none())
            {
                errors.push(format!("{id}: blocked unit has no valid blocked_stage"));
            }
            let valid_pairs: &[(&str, &[&str])] = &[
                ("pending", &["pending"]),
                ("analyzing", &["active"]),
                ("translation", &["pending"]),
                ("translating", &["active"]),
                ("review", &["pending", "needs_fix"]),
                ("reviewing", &["active"]),
                ("fixing", &["active"]),
                ("done", &["done"]),
                ("blocked", &["blocked"]),
            ];
            let allowed = valid_pairs
                .iter()
                .find(|(candidate, _)| candidate == stage)
                .map(|(_, statuses)| *statuses)
                .unwrap_or(&[]);
            if !allowed.contains(&status.as_str()) {
                errors.push(format!("{id}: invalid stage/status pair {stage}/{status}"));
            }
            if ["analyzing", "translating", "reviewing", "fixing"].contains(&stage.as_str()) {
                let claimed: bool = self
                    .conn
                    .query_row(
                        "SELECT 1 FROM claims WHERE unit = ?1",
                        rusqlite::params![id],
                        |_| Ok(()),
                    )
                    .optional()?
                    .is_some();
                if !claimed {
                    errors.push(format!("{id}: active stage {stage} has no claim"));
                }
            }
            if let Some((role, owner, expires_at)) = self
                .conn
                .query_row(
                    "SELECT role, owner, expires_at FROM claims WHERE unit = ?1",
                    rusqlite::params![id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    },
                )
                .optional()?
            {
                let expires_text: String = expires_at;
                if !ROLES.contains(&role.as_str()) {
                    errors.push(format!("{id}: invalid claim role"));
                }
                if owner.trim().is_empty() {
                    errors.push(format!("{id}: claim without owner"));
                }
                if is_expired(Some(expires_text.as_str()), now_ms) {
                    errors.push(format!("{id}: expired claim not recovered"));
                }
                if status != "active" {
                    errors.push(format!("{id}: claimed unit must have active status"));
                }
                if active_opt.as_deref() != Some(category.as_str()) {
                    errors.push(format!("{id}: claim outside the focus category"));
                }
                let expected = match role.as_str() {
                    "analyst" => Some("analyzing"),
                    "translator" => Some("translating"),
                    "reviewer" => Some("reviewing"),
                    "fixer" => Some("fixing"),
                    _ => None,
                };
                if expected.is_some_and(|wanted| wanted != stage) {
                    errors.push(format!(
                        "{id}: claim role {role} does not match stage {stage}"
                    ));
                }
            }
        }
        let mut live_claims = 0;
        {
            let mut stmt = self.conn.prepare("SELECT expires_at FROM claims")?;
            let rows = stmt.query_map([], |row| row.get::<_, Option<String>>(0))?;
            for row in rows {
                let expires_at: Option<String> = row?;
                if !is_expired(expires_at.as_deref(), now_ms) {
                    live_claims += 1;
                }
            }
        }
        if live_claims > policy.max_agents {
            errors.push(format!(
                "too many active claims: {live_claims} > {}",
                policy.max_agents
            ));
        }
        // Patches/overlays/shared: categoria e unidade apontam para o mesmo dono.
        {
            let mut stmt = self.conn.prepare("SELECT name, category FROM patches")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for row in rows {
                let (name, category) = row?;
                let known = categories
                    .iter()
                    .any(|(id, _, _, _, _, _, _, _, _)| id == &category);
                if !known {
                    errors.push(format!("patch {name}: unknown category"));
                    continue;
                }
                let listed: Vec<String> = {
                    let mut inner = self
                        .conn
                        .prepare("SELECT unit FROM patch_units WHERE patch = ?1")?;
                    let listed_rows =
                        inner.query_map(rusqlite::params![name], |entry| entry.get(0))?;
                    listed_rows.collect::<Result<Vec<String>, _>>()?
                };
                for unit in listed {
                    let owner: Option<String> = self
                        .conn
                        .query_row(
                            "SELECT category FROM units WHERE id = ?1",
                            rusqlite::params![unit],
                            |entry| entry.get(0),
                        )
                        .optional()?;
                    match owner {
                        None => errors.push(format!("patch {name}: unknown unit {unit}")),
                        Some(found) if found != category => {
                            errors
                                .push(format!("patch {name}: unit {unit} is in another category"));
                        }
                        _ => {}
                    }
                }
            }
        }
        {
            let mut stmt = self.conn.prepare("SELECT path, category FROM overlays")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for row in rows {
                let (path, category) = row?;
                if !categories
                    .iter()
                    .any(|(id, _, _, _, _, _, _, _, _)| id == &category)
                {
                    errors.push(format!("overlay {path}: unknown category"));
                }
            }
        }
        {
            let mut stmt = self
                .conn
                .prepare("SELECT path, category FROM shared_overrides")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for row in rows {
                let (path, category) = row?;
                if !categories
                    .iter()
                    .any(|(id, _, _, _, _, _, _, _, _)| id == &category)
                {
                    errors.push(format!("shared override {path}: unknown category"));
                }
            }
        }
        {
            let mut stmt = self.conn.prepare("SELECT patch, unit FROM patch_units")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for row in rows {
                let (patch, unit) = row?;
                let patch_category: Option<String> = self
                    .conn
                    .query_row(
                        "SELECT category FROM patches WHERE name = ?1",
                        rusqlite::params![patch],
                        |entry| entry.get(0),
                    )
                    .optional()?;
                let unit_category: Option<String> = self
                    .conn
                    .query_row(
                        "SELECT category FROM units WHERE id = ?1",
                        rusqlite::params![unit],
                        |entry| entry.get(0),
                    )
                    .optional()?;
                match (patch_category, unit_category) {
                    (Some(left), Some(right)) if left != right => {
                        errors.push(format!("{unit}: patch {patch} belongs to another category"));
                    }
                    (None, _) => errors.push(format!("{unit}: unknown patch {patch}")),
                    (_, None) => errors.push(format!("patch {patch}: unknown unit {unit}")),
                    _ => {}
                }
            }
        }
        {
            let mut stmt = self.conn.prepare("SELECT path, unit FROM overlay_units")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for row in rows {
                let (path, unit) = row?;
                let overlay_category: Option<String> = self
                    .conn
                    .query_row(
                        "SELECT category FROM overlays WHERE path = ?1",
                        rusqlite::params![path],
                        |entry| entry.get(0),
                    )
                    .optional()?;
                let unit_category: Option<String> = self
                    .conn
                    .query_row(
                        "SELECT category FROM units WHERE id = ?1",
                        rusqlite::params![unit],
                        |entry| entry.get(0),
                    )
                    .optional()?;
                match (overlay_category, unit_category) {
                    (Some(left), Some(right)) if left != right => {
                        errors.push(format!(
                            "{unit}: overlay {path} belongs to another category"
                        ));
                    }
                    (None, _) => errors.push(format!("{unit}: unknown overlay {path}")),
                    (_, None) => errors.push(format!("overlay {path}: unknown unit {unit}")),
                    _ => {}
                }
            }
        }
        {
            let mut stmt = self
                .conn
                .prepare("SELECT path, unit FROM shared_override_units")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for row in rows {
                let (path, unit) = row?;
                let owner_category: Option<String> = self
                    .conn
                    .query_row(
                        "SELECT category FROM shared_overrides WHERE path = ?1",
                        rusqlite::params![path],
                        |entry| entry.get(0),
                    )
                    .optional()?;
                let unit_category: Option<String> = self
                    .conn
                    .query_row(
                        "SELECT category FROM units WHERE id = ?1",
                        rusqlite::params![unit],
                        |entry| entry.get(0),
                    )
                    .optional()?;
                match (owner_category, unit_category) {
                    (Some(left), Some(right)) if left != right => {
                        errors.push(format!(
                            "{unit}: shared override {path} belongs to another category"
                        ));
                    }
                    (None, _) => errors.push(format!("{unit}: unknown shared override {path}")),
                    (_, None) => {
                        errors.push(format!("shared override {path}: unknown unit {unit}"))
                    }
                    _ => {}
                }
            }
        }
        Ok(errors)
    }

    /// Instantaneo JSON com o formato de alto nivel do `STATE` Python.
    pub fn snapshot(&self) -> Result<Value, StoreError> {
        let version_text =
            get_meta(&self.conn, "version")?.unwrap_or_else(|| STATE_VERSION.to_string());
        let version: i64 = version_text.parse().unwrap_or(STATE_VERSION);
        let policy_text = get_meta(&self.conn, "policy")?.unwrap_or_else(|| "{}".to_string());
        let policy_value: Value = serde_json::from_str(&policy_text).unwrap_or(Value::Null);
        let created_at = get_meta(&self.conn, "created_at")?.unwrap_or_default();
        let updated_at = get_meta(&self.conn, "updated_at")?.unwrap_or_default();
        let active_raw = get_meta(&self.conn, "active_category")?.unwrap_or_default();
        let active_category = if active_raw.is_empty() {
            Value::Null
        } else {
            Value::String(active_raw)
        };
        // Categorias.
        let mut categories = serde_json::Map::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT id, ord, title, depends_on, stage, status, score, consistency_score, consistency_report, consistency_round, blocked_stage, done_at, gate_status, blockers FROM categories ORDER BY ord ASC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                    row.get::<_, Option<i64>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, String>(13)?,
                ))
            })?;
            for row in rows {
                let (
                    id,
                    ord,
                    title_raw,
                    depends_raw,
                    stage,
                    status,
                    score,
                    consistency_score,
                    consistency_report_raw,
                    consistency_round,
                    blocked_stage,
                    done_at,
                    gate_status,
                    blockers_raw,
                ) = row?;
                let title: Value =
                    serde_json::from_str(&title_raw).unwrap_or(Value::String(title_raw.clone()));
                let depends_on: Value =
                    serde_json::from_str(&depends_raw).unwrap_or(Value::Array(Vec::new()));
                let unit_ids: Vec<Value> = {
                    let mut inner = self
                        .conn
                        .prepare("SELECT id FROM units WHERE category = ?1 ORDER BY id ASC")?;
                    let rows = inner
                        .query_map(rusqlite::params![id], |entry| entry.get::<_, String>(0))?;
                    let names: Vec<String> = rows.collect::<Result<Vec<_>, _>>()?;
                    names.into_iter().map(Value::String).collect()
                };
                let patch_names: Vec<Value> = {
                    let mut inner = self.conn.prepare(
                        "SELECT name FROM patches WHERE category = ?1 ORDER BY name ASC",
                    )?;
                    let rows = inner
                        .query_map(rusqlite::params![id], |entry| entry.get::<_, String>(0))?;
                    let names: Vec<String> = rows.collect::<Result<Vec<_>, _>>()?;
                    names.into_iter().map(Value::String).collect()
                };
                let overlay_paths: Vec<Value> = {
                    let mut inner = self.conn.prepare(
                        "SELECT path FROM overlays WHERE category = ?1 ORDER BY path ASC",
                    )?;
                    let rows = inner
                        .query_map(rusqlite::params![id], |entry| entry.get::<_, String>(0))?;
                    let names: Vec<String> = rows.collect::<Result<Vec<_>, _>>()?;
                    names.into_iter().map(Value::String).collect()
                };
                let shared_paths: Vec<Value> = {
                    let mut inner = self.conn.prepare(
                        "SELECT path FROM shared_overrides WHERE category = ?1 ORDER BY path ASC",
                    )?;
                    let rows = inner
                        .query_map(rusqlite::params![id], |entry| entry.get::<_, String>(0))?;
                    let names: Vec<String> = rows.collect::<Result<Vec<_>, _>>()?;
                    names.into_iter().map(Value::String).collect()
                };
                let gates = {
                    let mut inner = self
                        .conn
                        .prepare("SELECT name, status, note, at FROM gates WHERE category = ?1")?;
                    let entries = inner.query_map(rusqlite::params![id], |entry| {
                        Ok((
                            entry.get::<_, String>(0)?,
                            entry.get::<_, String>(1)?,
                            entry.get::<_, String>(2)?,
                            entry.get::<_, String>(3)?,
                        ))
                    })?;
                    let mut map = serde_json::Map::new();
                    for entry in entries {
                        let (name, gate_status, note, at) = entry?;
                        map.insert(
                            name,
                            serde_json::json!({"status": gate_status, "note": note, "at": at}),
                        );
                    }
                    Value::Object(map)
                };
                let blockers: Value =
                    serde_json::from_str(&blockers_raw).unwrap_or(Value::Array(Vec::new()));
                let consistency_report: Value = match consistency_report_raw {
                    Some(raw) => serde_json::from_str(&raw).unwrap_or(Value::String(raw)),
                    None => Value::Null,
                };
                categories.insert(
                    id.clone(),
                    serde_json::json!({
                        "id": id,
                        "order": ord,
                        "title": title,
                        "depends_on": depends_on,
                        "stage": stage,
                        "status": status,
                        "units": unit_ids,
                        "score": score.map_or(Value::Null, Value::from),
                        "consistency_score": consistency_score.map_or(Value::Null, Value::from),
                        "consistency_report": consistency_report,
                        "consistency_round": consistency_round,
                        "blocked_stage": blocked_stage.map_or(Value::Null, Value::String),
                        "patches": patch_names,
                        "overlays": overlay_paths,
                        "shared_overrides": shared_paths,
                        "blockers": blockers,
                        "gates": gates,
                        "done_at": done_at.map_or(Value::Null, Value::String),
                        "gate_status": gate_status.map_or(Value::Null, Value::String),
                    }),
                );
            }
        }
        // Unidades.
        let mut units = serde_json::Map::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT id, category, title, file_count, stage, status, score, attempts, blocked_stage, analysis, translation_report, report, correction_report, findings, labels_total, labels_translated, translated_by, fixed_by, last_reviewer, reviewed_by, translated_at, fixed_at, reviewed_at, done_at FROM units ORDER BY id ASC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, Option<String>>(13)?,
                    row.get::<_, i64>(14)?,
                    row.get::<_, i64>(15)?,
                    row.get::<_, Option<String>>(16)?,
                    row.get::<_, Option<String>>(17)?,
                    row.get::<_, Option<String>>(18)?,
                    row.get::<_, String>(19)?,
                    row.get::<_, Option<String>>(20)?,
                    row.get::<_, Option<String>>(21)?,
                    row.get::<_, Option<String>>(22)?,
                    row.get::<_, Option<String>>(23)?,
                ))
            })?;
            for row in rows {
                let (
                    id,
                    category,
                    title_raw,
                    file_count,
                    stage,
                    status,
                    score,
                    attempts,
                    blocked_stage,
                    analysis_raw,
                    translation_report_raw,
                    report_raw,
                    correction_report_raw,
                    findings_raw,
                    labels_total,
                    labels_translated,
                    translated_by,
                    fixed_by,
                    last_reviewer,
                    reviewed_by_raw,
                    translated_at,
                    fixed_at,
                    reviewed_at,
                    done_at,
                ) = row?;
                let title: Value =
                    serde_json::from_str(&title_raw).unwrap_or(Value::String(title_raw.clone()));
                let reviewed_by: Value =
                    serde_json::from_str(&reviewed_by_raw).unwrap_or(Value::Array(Vec::new()));
                let claim: Value = match self
                    .conn
                    .query_row(
                        "SELECT owner, role, claimed_at, expires_at FROM claims WHERE unit = ?1",
                        rusqlite::params![id],
                        |entry| {
                            Ok((
                                entry.get::<_, String>(0)?,
                                entry.get::<_, String>(1)?,
                                entry.get::<_, String>(2)?,
                                entry.get::<_, String>(3)?,
                            ))
                        },
                    )
                    .optional()?
                {
                    Some((owner, role, claimed_at, expires_at)) => {
                        serde_json::json!({"owner": owner, "role": role, "claimed_at": claimed_at, "expires_at": expires_at})
                    }
                    None => Value::Null,
                };
                let inline_or_null = |raw: Option<String>| -> Value {
                    match raw {
                        Some(text) => serde_json::from_str(&text).unwrap_or(Value::String(text)),
                        None => Value::Null,
                    }
                };
                let unit_patches: Vec<Value> = {
                    let mut inner = self.conn.prepare(
                        "SELECT patch FROM patch_units WHERE unit = ?1 ORDER BY patch ASC",
                    )?;
                    let rows = inner
                        .query_map(rusqlite::params![id], |entry| entry.get::<_, String>(0))?;
                    let names: Vec<String> = rows.collect::<Result<Vec<_>, _>>()?;
                    names.into_iter().map(Value::String).collect()
                };
                let unit_overlays: Vec<Value> = {
                    let mut inner = self.conn.prepare(
                        "SELECT path FROM overlay_units WHERE unit = ?1 ORDER BY path ASC",
                    )?;
                    let rows = inner
                        .query_map(rusqlite::params![id], |entry| entry.get::<_, String>(0))?;
                    let names: Vec<String> = rows.collect::<Result<Vec<_>, _>>()?;
                    names.into_iter().map(Value::String).collect()
                };
                let unit_shared: Vec<Value> = {
                    let mut inner = self.conn.prepare(
                        "SELECT path FROM shared_override_units WHERE unit = ?1 ORDER BY path ASC",
                    )?;
                    let rows = inner
                        .query_map(rusqlite::params![id], |entry| entry.get::<_, String>(0))?;
                    let names: Vec<String> = rows.collect::<Result<Vec<_>, _>>()?;
                    names.into_iter().map(Value::String).collect()
                };
                units.insert(
                    id.clone(),
                    serde_json::json!({
                        "id": id,
                        "category": category,
                        "title": title,
                        "file_count": file_count,
                        "stage": stage,
                        "status": status,
                        "score": score.map_or(Value::Null, Value::from),
                        "attempt": 0,
                        "attempts": attempts,
                        "claim": claim,
                        "blocked_stage": blocked_stage.map_or(Value::Null, Value::String),
                        "analysis": inline_or_null(analysis_raw),
                        "translation_report": inline_or_null(translation_report_raw),
                        "report": inline_or_null(report_raw),
                        "correction_report": inline_or_null(correction_report_raw),
                        "findings": inline_or_null(findings_raw),
                        "labels_total": labels_total,
                        "labels_translated": labels_translated,
                        "translated_by": translated_by.map_or(Value::Null, Value::String),
                        "fixed_by": fixed_by.map_or(Value::Null, Value::String),
                        "last_reviewer": last_reviewer.map_or(Value::Null, Value::String),
                        "reviewed_by": reviewed_by,
                        "patches": unit_patches,
                        "overlays": unit_overlays,
                        "shared_overrides": unit_shared,
                        "translated_at": translated_at.map_or(Value::Null, Value::String),
                        "fixed_at": fixed_at.map_or(Value::Null, Value::String),
                        "reviewed_at": reviewed_at.map_or(Value::Null, Value::String),
                        "done_at": done_at.map_or(Value::Null, Value::String),
                    }),
                );
            }
        }
        // Globais.
        let mut patches = serde_json::Map::new();
        {
            let mut stmt = self
                .conn
                .prepare("SELECT name, category, at FROM patches ORDER BY name ASC")?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;
            for row in rows {
                let (name, category, at) = row?;
                let linked: Vec<Value> = {
                    let mut inner = self.conn.prepare(
                        "SELECT unit FROM patch_units WHERE patch = ?1 ORDER BY unit ASC",
                    )?;
                    let rows = inner
                        .query_map(rusqlite::params![name], |entry| entry.get::<_, String>(0))?;
                    let names: Vec<String> = rows.collect::<Result<Vec<_>, _>>()?;
                    names.into_iter().map(Value::String).collect()
                };
                patches.insert(
                    name.clone(),
                    serde_json::json!({"name": name, "category": category, "units": linked, "at": at}),
                );
            }
        }
        let mut overlays = serde_json::Map::new();
        {
            let mut stmt = self
                .conn
                .prepare("SELECT path, category, at FROM overlays ORDER BY path ASC")?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;
            for row in rows {
                let (path, category, at) = row?;
                overlays.insert(
                    path.clone(),
                    serde_json::json!({"path": path, "category": category, "at": at}),
                );
            }
        }
        let mut shared = serde_json::Map::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT path, category, reason, at FROM shared_overrides ORDER BY path ASC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?;
            for row in rows {
                let (path, category, reason, at) = row?;
                shared.insert(
                    path.clone(),
                    serde_json::json!({"path": path, "category": category, "reason": reason, "at": at}),
                );
            }
        }
        let mut events: Vec<Value> = Vec::new();
        {
            let mut stmt = self
                .conn
                .prepare("SELECT at, type, target, details FROM events ORDER BY id ASC")?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?;
            for row in rows {
                let (at, kind, target, details_raw) = row?;
                let mut entry = serde_json::Map::new();
                entry.insert("at".to_string(), Value::String(at));
                entry.insert("type".to_string(), Value::String(kind));
                entry.insert("target".to_string(), Value::String(target));
                if let Ok(Value::Object(extra)) = serde_json::from_str::<Value>(&details_raw) {
                    for (key, value) in extra {
                        entry.insert(key, value);
                    }
                }
                events.push(Value::Object(entry));
            }
        }
        Ok(serde_json::json!({
            "version": version,
            "created_at": created_at,
            "updated_at": updated_at,
            "policy": policy_value,
            "active_category": active_category,
            "categories": categories,
            "units": units,
            "patches": patches,
            "overlays": overlays,
            "shared_overrides": shared,
            "events": events,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const BASE_MS: u64 = 1_700_000_000_000;

    fn fixture_catalog() -> Value {
        json!({
            "version": 1,
            "policy": {
                "max_agents": 3,
                "max_active_categories": 1,
                "lease_minutes": 120,
                "pass_score": 10,
                "max_rounds": 3
            },
            "categories": [
                {
                    "id": "fixture-alpha",
                    "order": 1,
                    "title": "Fixture Alpha",
                    "depends_on": [],
                    "units": [
                        {"id": "alpha/one", "title": "Alpha unit 1", "files": ["fixture.tsx"]},
                        {"id": "alpha/two", "title": "Alpha unit 2", "files": ["fixture.tsx"]},
                        {"id": "alpha/three", "title": "Alpha unit 3", "files": ["fixture.tsx"]}
                    ]
                },
                {
                    "id": "fixture-beta",
                    "order": 2,
                    "title": "Fixture Beta",
                    "depends_on": [],
                    "units": [
                        {"id": "beta/one", "title": "Beta unit", "files": ["fixture.tsx"]}
                    ]
                }
            ]
        })
    }

    fn test_store() -> Store<FixedClock> {
        let clock = FixedClock::new(BASE_MS);
        let mut store = Store::open_in_memory(clock).unwrap();
        store.init_from_catalog(&fixture_catalog()).unwrap();
        store
    }

    fn review_report(unit: Option<&str>, category: Option<&str>, round: i64, score: i64) -> Value {
        let mut remaining = 10 - score;
        let mut criteria = serde_json::Map::new();
        for key in [
            "coverage",
            "naturalness",
            "consistency",
            "context",
            "safety",
        ] {
            let current = 2;
            let take = current.min(remaining);
            criteria.insert(key.to_string(), Value::from(current - take));
            remaining -= take;
        }
        let findings = if score == 10 {
            Vec::new()
        } else {
            vec![json!({
                "id": "F1",
                "severity": if score >= 8 { "minor" } else { "major" },
                "file": "fixture.tsx",
                "line": 1,
                "problem": "fixture finding",
                "fix": "fixture fix"
            })]
        };
        let mut report = serde_json::Map::new();
        report.insert("score".to_string(), Value::from(score));
        report.insert("criteria".to_string(), Value::Object(criteria));
        report.insert("findings".to_string(), Value::Array(findings));
        report.insert("untranslated".to_string(), Value::Array(Vec::new()));
        report.insert("glossary_violations".to_string(), Value::Array(Vec::new()));
        report.insert("test_impact".to_string(), Value::Array(Vec::new()));
        if let Some(unit_id) = unit {
            report.insert("unit".to_string(), Value::String(unit_id.to_string()));
            report.insert("round".to_string(), Value::from(round));
        }
        if let Some(category_id) = category {
            report.insert(
                "category".to_string(),
                Value::String(category_id.to_string()),
            );
            report.insert("round".to_string(), Value::from(round));
        }
        Value::Object(report)
    }

    fn complete_simple(
        store: &mut Store<FixedClock>,
        unit: &str,
        role: &str,
        owner: &str,
        opts: CompleteOptions,
    ) {
        store.claim(unit, role, owner, 120).unwrap();
        store.complete(unit, role, owner, opts).unwrap();
    }

    fn drive_to_review(store: &mut Store<FixedClock>) {
        for unit in ["alpha/one", "alpha/two", "alpha/three"] {
            complete_simple(
                store,
                unit,
                "analyst",
                "analyst",
                CompleteOptions {
                    report: Some(json!({"ok": true})),
                    ..CompleteOptions::default()
                },
            );
        }
        store.advance("fixture-alpha", false).unwrap();
        for unit in ["alpha/one", "alpha/two", "alpha/three"] {
            complete_simple(
                store,
                unit,
                "translator",
                "translator",
                CompleteOptions {
                    report: Some(json!({"ok": true})),
                    ..CompleteOptions::default()
                },
            );
        }
        store.advance("fixture-alpha", false).unwrap();
    }

    fn review_unit(store: &mut Store<FixedClock>, unit: &str, score: i64, owner: &str) {
        let snapshot = store.snapshot().unwrap();
        let attempts = snapshot["units"][unit]["attempts"].as_i64().unwrap_or(0);
        let report = review_report(Some(unit), None, attempts + 1, score);
        store.claim(unit, "reviewer", owner, 120).unwrap();
        store
            .complete(
                unit,
                "reviewer",
                owner,
                CompleteOptions {
                    report: Some(report),
                    score: Some(score),
                    ..CompleteOptions::default()
                },
            )
            .unwrap();
    }

    fn fix_unit(store: &mut Store<FixedClock>, unit: &str, owner: &str) {
        complete_simple(
            store,
            unit,
            "fixer",
            owner,
            CompleteOptions {
                report: Some(json!({"ok": true})),
                ..CompleteOptions::default()
            },
        );
    }

    #[test]
    fn claim_rejects_reviewer_equal_to_translator() {
        let mut store = test_store();
        drive_to_review(&mut store);
        let error = store
            .claim("alpha/one", "reviewer", "translator", 120)
            .unwrap_err();
        assert!(error.to_string().contains("translated"), "{error}");
        store
            .claim("alpha/one", "reviewer", "reviewer", 120)
            .unwrap();
    }

    #[test]
    fn claim_rejects_reviewer_equal_to_fixer_and_fixer_equal_to_reviewer() {
        let mut store = test_store();
        drive_to_review(&mut store);
        review_unit(&mut store, "alpha/one", 8, "reviewer");
        // Fixer igual ao ultimo revisor e rejeitado.
        let error = store
            .claim("alpha/one", "fixer", "reviewer", 120)
            .unwrap_err();
        assert!(error.to_string().contains("reviewed"), "{error}");
        fix_unit(&mut store, "alpha/one", "fixer");
        // Revisor igual ao fixer e rejeitado.
        let error = store
            .claim("alpha/one", "reviewer", "fixer", 120)
            .unwrap_err();
        assert!(error.to_string().contains("fixed"), "{error}");
    }

    #[test]
    fn three_strikes_block_unit_and_category() {
        let mut store = test_store();
        drive_to_review(&mut store);
        for round in 1..=2 {
            review_unit(&mut store, "alpha/one", 7, "reviewer");
            let snapshot = store.snapshot().unwrap();
            assert_eq!(snapshot["units"]["alpha/one"]["status"], json!("needs_fix"));
            assert_eq!(snapshot["units"]["alpha/one"]["attempts"], json!(round));
            fix_unit(&mut store, "alpha/one", "fixer");
        }
        review_unit(&mut store, "alpha/one", 7, "reviewer");
        let snapshot = store.snapshot().unwrap();
        assert_eq!(snapshot["units"]["alpha/one"]["stage"], json!("blocked"));
        assert_eq!(snapshot["units"]["alpha/one"]["status"], json!("blocked"));
        assert_eq!(
            snapshot["categories"]["fixture-alpha"]["stage"],
            json!("blocked")
        );
        assert_eq!(
            snapshot["categories"]["fixture-alpha"]["status"],
            json!("blocked")
        );
        let blockers = snapshot["categories"]["fixture-alpha"]["blockers"].clone();
        assert!(blockers.as_array().unwrap().iter().any(|value| value
            .as_str()
            .unwrap_or("")
            .contains("unidade alpha/one nao alcancou 10 em 3 rodadas (ultima nota 7)")));
        // Categoria bloqueada nao avanca.
        let error = store.advance("fixture-alpha", false).unwrap_err();
        assert!(error.to_string().contains("blocked"), "{error}");
        // Sem unidades reivindicaveis apos o bloqueio.
        assert!(store.next("reviewer", 3).unwrap().is_empty());
    }

    #[test]
    fn skip_without_note_rejected() {
        let mut store = test_store();
        drive_to_review(&mut store);
        for unit in ["alpha/one", "alpha/two", "alpha/three"] {
            review_unit(&mut store, unit, 10, "reviewer");
        }
        store.advance("fixture-alpha", false).unwrap();
        let snapshot = store.snapshot().unwrap();
        let round = snapshot["categories"]["fixture-alpha"]["consistency_round"]
            .as_i64()
            .unwrap_or(0);
        let report = review_report(None, Some("fixture-alpha"), round + 1, 10);
        store
            .record_consistency("fixture-alpha", 10, &report)
            .unwrap();
        store.advance("fixture-alpha", false).unwrap();
        let error = store
            .record_gate("fixture-alpha", "audit", "skip", "")
            .unwrap_err();
        assert!(error.to_string().contains("--note is required"), "{error}");
        store
            .record_gate("fixture-alpha", "audit", "skip", "tool missing")
            .unwrap();
        let snapshot = store.snapshot().unwrap();
        assert_eq!(
            snapshot["categories"]["fixture-alpha"]["gates"]["audit"]["status"],
            json!("skip")
        );
    }

    #[test]
    fn advance_blocked_on_missing_and_failed_gates() {
        let mut store = test_store();
        drive_to_review(&mut store);
        for unit in ["alpha/one", "alpha/two", "alpha/three"] {
            review_unit(&mut store, unit, 10, "reviewer");
        }
        store.advance("fixture-alpha", false).unwrap();
        let snapshot = store.snapshot().unwrap();
        let round = snapshot["categories"]["fixture-alpha"]["consistency_round"]
            .as_i64()
            .unwrap_or(0);
        store
            .record_consistency(
                "fixture-alpha",
                10,
                &review_report(None, Some("fixture-alpha"), round + 1, 10),
            )
            .unwrap();
        store.advance("fixture-alpha", false).unwrap();
        let error = store.advance("fixture-alpha", false).unwrap_err();
        assert!(error.to_string().contains("missing gates"), "{error}");
        for gate in ["catalog", "audit", "lint", "tests", "typecheck", "lint-ui"] {
            store
                .record_gate("fixture-alpha", gate, "pass", "")
                .unwrap();
        }
        store
            .record_gate("fixture-alpha", "build", "fail", "")
            .unwrap();
        let error = store.advance("fixture-alpha", false).unwrap_err();
        assert!(error.to_string().contains("failed gates"), "{error}");
        store
            .record_gate("fixture-alpha", "build", "pass", "")
            .unwrap();
        store.advance("fixture-alpha", false).unwrap();
        let snapshot = store.snapshot().unwrap();
        assert_eq!(
            snapshot["categories"]["fixture-alpha"]["status"],
            json!("done")
        );
    }

    #[test]
    fn advance_requires_allow_unverified_for_skipped_gates() {
        let mut store = test_store();
        drive_to_review(&mut store);
        for unit in ["alpha/one", "alpha/two", "alpha/three"] {
            review_unit(&mut store, unit, 10, "reviewer");
        }
        store.advance("fixture-alpha", false).unwrap();
        let snapshot = store.snapshot().unwrap();
        let round = snapshot["categories"]["fixture-alpha"]["consistency_round"]
            .as_i64()
            .unwrap_or(0);
        store
            .record_consistency(
                "fixture-alpha",
                10,
                &review_report(None, Some("fixture-alpha"), round + 1, 10),
            )
            .unwrap();
        store.advance("fixture-alpha", false).unwrap();
        for gate in ["catalog", "audit", "lint", "tests", "typecheck", "lint-ui"] {
            store
                .record_gate("fixture-alpha", gate, "pass", "")
                .unwrap();
        }
        store
            .record_gate("fixture-alpha", "build", "skip", "sem toolchain")
            .unwrap();
        let error = store.advance("fixture-alpha", false).unwrap_err();
        assert!(error.to_string().contains("--allow-unverified"), "{error}");
        store.advance("fixture-alpha", true).unwrap();
        let snapshot = store.snapshot().unwrap();
        assert_eq!(
            snapshot["categories"]["fixture-alpha"]["gate_status"],
            json!("unverified")
        );
    }

    #[test]
    fn recover_expired_reverts_and_counts() {
        let mut store = test_store();
        store
            .claim("alpha/one", "analyst", "lease-test", 1)
            .unwrap();
        let snapshot = store.snapshot().unwrap();
        assert_eq!(snapshot["units"]["alpha/one"]["stage"], json!("analyzing"));
        store.clock().advance_ms(120_000);
        let recovered = store.recover_expired().unwrap();
        assert_eq!(recovered, 1);
        let snapshot = store.snapshot().unwrap();
        assert_eq!(snapshot["units"]["alpha/one"]["claim"], Value::Null);
        assert_eq!(snapshot["units"]["alpha/one"]["stage"], json!("pending"));
        assert_eq!(snapshot["units"]["alpha/one"]["status"], json!("pending"));
        assert!(store
            .next("analyst", 3)
            .unwrap()
            .contains(&"alpha/one".to_string()));
        assert_eq!(store.recover_expired().unwrap(), 0);
    }

    #[test]
    fn unblock_backward_only_rule() {
        let mut store = test_store();
        store.claim("alpha/one", "analyst", "analyst", 120).unwrap();
        store.block("alpha/one", "blocked in analysis").unwrap();
        let error = store.unblock("alpha/one", "review", true).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("only earlier stages are allowed"),
            "{error}"
        );
        store.unblock("alpha/one", "analysis", true).unwrap();
        let snapshot = store.snapshot().unwrap();
        // Volta a review? Nao: analysis reseta as irmas para pending.
        assert_eq!(snapshot["units"]["alpha/one"]["stage"], json!("pending"));
        assert_eq!(snapshot["units"]["alpha/one"]["status"], json!("pending"));
        assert_eq!(
            snapshot["categories"]["fixture-alpha"]["stage"],
            json!("analysis")
        );
    }

    #[test]
    fn reopen_resets_done_category() {
        let mut store = test_store();
        drive_to_review(&mut store);
        for unit in ["alpha/one", "alpha/two", "alpha/three"] {
            review_unit(&mut store, unit, 10, "reviewer");
        }
        store.advance("fixture-alpha", false).unwrap();
        let snapshot = store.snapshot().unwrap();
        let round = snapshot["categories"]["fixture-alpha"]["consistency_round"]
            .as_i64()
            .unwrap_or(0);
        store
            .record_consistency(
                "fixture-alpha",
                10,
                &review_report(None, Some("fixture-alpha"), round + 1, 10),
            )
            .unwrap();
        store.advance("fixture-alpha", false).unwrap();
        for gate in [
            "catalog",
            "audit",
            "lint",
            "tests",
            "typecheck",
            "lint-ui",
            "build",
        ] {
            store
                .record_gate("fixture-alpha", gate, "pass", "")
                .unwrap();
        }
        store.advance("fixture-alpha", false).unwrap();
        let snapshot = store.snapshot().unwrap();
        assert_eq!(
            snapshot["categories"]["fixture-alpha"]["status"],
            json!("done")
        );
        assert_eq!(snapshot["active_category"], json!("fixture-beta"));
        // Sem --force, outra categoria ativa bloqueia o reopen.
        let error = store
            .reopen("fixture-alpha", "regression found", false)
            .unwrap_err();
        assert!(
            error.to_string().contains("another category is active"),
            "{error}"
        );
        store
            .reopen("fixture-alpha", "regression found", true)
            .unwrap();
        let snapshot = store.snapshot().unwrap();
        assert_eq!(snapshot["active_category"], json!("fixture-alpha"));
        assert_eq!(
            snapshot["categories"]["fixture-alpha"]["stage"],
            json!("review")
        );
        assert_eq!(
            snapshot["categories"]["fixture-alpha"]["status"],
            json!("active")
        );
        assert_eq!(
            snapshot["categories"]["fixture-alpha"]["consistency_score"],
            Value::Null
        );
        assert_eq!(
            snapshot["categories"]["fixture-alpha"]["consistency_round"],
            json!(0)
        );
        for unit in ["alpha/one", "alpha/two", "alpha/three"] {
            assert_eq!(snapshot["units"][unit]["stage"], json!("review"));
            assert_eq!(snapshot["units"][unit]["status"], json!("pending"));
            assert_eq!(snapshot["units"][unit]["attempts"], json!(0));
            assert_eq!(snapshot["units"][unit]["claim"], Value::Null);
        }
        assert!(store.validate().unwrap().is_empty());
    }

    #[test]
    fn snapshot_has_python_top_level_shape() {
        let store = test_store();
        let snapshot = store.snapshot().unwrap();
        for key in [
            "version",
            "policy",
            "active_category",
            "categories",
            "units",
            "patches",
            "overlays",
            "shared_overrides",
            "events",
        ] {
            assert!(snapshot.get(key).is_some(), "missing key {key}");
        }
        assert_eq!(snapshot["version"], json!(1));
        assert_eq!(snapshot["active_category"], json!("fixture-alpha"));
    }

    #[test]
    fn validate_accepts_fresh_state() {
        let store = test_store();
        assert!(store.validate().unwrap().is_empty());
    }

    #[test]
    fn concurrency_limit_is_three() {
        let mut store = test_store();
        store.claim("alpha/one", "analyst", "a", 120).unwrap();
        store.claim("alpha/two", "analyst", "b", 120).unwrap();
        store.claim("alpha/three", "analyst", "c", 120).unwrap();
        assert!(store.next("analyst", 3).unwrap().is_empty());
        let snapshot = store.snapshot().unwrap();
        assert_eq!(snapshot["active_category"], json!("fixture-alpha"));
        // Quarta unidade (beta) nao e da categoria ativa de qualquer forma.
        let error = store.claim("beta/one", "analyst", "d", 120).unwrap_err();
        assert!(
            error.to_string().contains("not the active category")
                || error.to_string().contains("concurrency limit reached"),
            "{error}"
        );
    }

    fn wide_catalog() -> Value {
        let units: Vec<Value> = (1..=5)
            .map(|n| json!({"id": format!("wide/u{n}"), "title": format!("Wide {n}"), "files": ["w.tsx"]}))
            .collect();
        json!({
            "version": 1,
            "policy": {"max_agents": 3, "max_active_categories": 1, "lease_minutes": 120, "pass_score": 10, "max_rounds": 3},
            "categories": [{"id": "wide", "order": 1, "title": "Wide", "depends_on": [], "units": units}]
        })
    }

    #[test]
    fn next_respects_remaining_slots_not_just_limit() {
        let mut store = Store::open_in_memory(FixedClock::new(BASE_MS)).unwrap();
        store.init_from_catalog(&wide_catalog()).unwrap();
        store.claim("wide/u1", "analyst", "a", 120).unwrap();
        store.claim("wide/u2", "analyst", "b", 120).unwrap();
        // 2 vivos de 3: resta 1 vaga, mesmo com limite maior e 3 livres.
        let listed = store.next("analyst", 10).unwrap();
        assert_eq!(listed, vec!["wide/u3".to_string()]);
    }

    #[test]
    fn category_unblock_to_review_keeps_live_claims() {
        let mut store = test_store();
        drive_to_review(&mut store);
        store.claim("alpha/one", "reviewer", "r", 120).unwrap();
        store
            .unblock("fixture-alpha", "review", true)
            .expect("force unblock de categoria ativa deve passar");
        let snapshot = store.snapshot().unwrap();
        assert_eq!(
            snapshot["units"]["alpha/one"]["claim"]["owner"],
            json!("r"),
            "claim vivo de unidade nao bloqueada deve sobreviver ao unblock"
        );
    }
}
