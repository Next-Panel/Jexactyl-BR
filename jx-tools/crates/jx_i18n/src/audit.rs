//! Nucleo de analise da auditoria i18n: pares, overlay pt-BR e fonte en.
//!
//! Port fiel do nucleo de ANALISE de `scripts/i18n/i18n_toolkit/audit.py`
//! (linhas 138-883): deteccao de idioma dos labels, pareamento por slot,
//! diff posicional de comentarios, diff de esqueleto por hunks de tokens e
//! comparacao de placeholders/URLs por multiconjunto. Sem pontes, sem git e
//! sem filesystem: a orquestracao (catalogo, `base`, contrapartes, allowlist,
//! saida JSON) fica para uma camada posterior.
//!
//! Correspondencia com `audit.py`:
//!
//! - `label_english_findings` espelha `label_english_findings` (linhas 138-178).
//! - `label_slot` / `pair_labels` espelham as funcoes homonimas (linhas 181-220).
//! - `is_translatable_label` espelha a funcao homonima (linhas 223-251); o
//!   parametro `kind` do Python nao e lido no corpo original, entao foi
//!   retirado da assinatura.
//! - `analyze_category_labels` (privada) espelha a funcao homonima (linhas
//!   254-346).
//! - `collapse_skeleton`, `skeleton_tokens`, `drop_jsx_wrap_parens`,
//!   `align_token_hunks` e `legacy_hunks` (privadas) espelham as funcoes
//!   homonimas (linhas 349-509), incluindo o teto `height * width > 6000000`
//!   que degrada para o diff legado por linha.
//! - [`analyze_pair`] espelha `analyze_pair` (linhas 512-760) na mesma ordem:
//!   fast path identico, labels de categoria, comentarios, esqueleto,
//!   placeholders, URLs, candidatos, teto de achados.
//! - [`analyze_overlay_values`] espelha `analyze_overlay_values` (linhas
//!   763-821); recebe os pares `common` ja comparados (via
//!   [`crate::php::compare_lang_entries`]) em vez dos textos brutos.
//! - [`analyze_english_source`] espelha `analyze_english_source` (linhas
//!   824-883).
//! - [`analyze_lang_key_mismatch`] espelha os lacos `missing`/`extra` do
//!   `audit` (linhas 1269-1290).
//! - Os construtores [`translate_lang_file_finding`],
//!   [`out_of_scope_finding`], [`glossary_missing_finding`] e
//!   [`audit_incomplete_finding`] apenas materializam os achados cujos dados
//!   (indice de lang, escopo git, glossario) pertencem a orquestracao.
//!
//! Divergencias conscientes:
//!
//! - Severidade do marcador `findings-truncated`: `blocker`, igual ao
//!   Python (`audit.py`, linha 536), para paridade do gate `blocking_ok`.
//! - Vereditos de idioma usam [`crate::lang`] com fontes injetaveis
//!   ([`crate::lang::ZipfSource`] / [`crate::lang::LanguageDetector`]); o
//!   caminho padrao usa as tabelas embarcadas + Lingua, e os testes usam o
//!   detector fixo offline.
//! - `Finding::new` rejeita arquivo vazio e linha zero; entradas degeneradas
//!   sao higienizadas (`"(unknown)"`, `max(1)`) em vez de falhar.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use regex::Regex;

use crate::core::{Finding, Severity};
use crate::lang::{
    english_terms_in_label_with, find_glossary_terms_with, portuguese_evidence_with, EmbeddedZipf,
    GlossaryEntry, LanguageDetector, LinguaDetector, NlpOptions, ZipfSource,
};
use crate::php::{entry_path_key, extract_php_lang_entries, LangEntry};
use crate::scan::{
    blank_preserving_newlines, context_identifier, extract_category_labels, extract_comments,
    normalize_blade_echo_labels, normalize_kind, normalize_yup_label_args,
    replace_category_lookups, scan_source, Piece, SkeletonLine, SourceKind, CODE_FIX_MESSAGE,
    LABEL_ATTRIBUTES,
};
use crate::text::{
    clean_label_text, extract_placeholders, extract_urls, multiset_difference, normalize_newlines,
    normalize_placeholder_literals, same_sorted_multiset, strip_non_label_tokens,
};

/// Teto padrao de achados por arquivo (`MAX_FINDINGS_PER_FILE`, `audit.py`, linha 27).
pub const MAX_FINDINGS_PER_FILE: usize = 50;

/// Configuracao de idioma da analise (limiares do classificador).
///
/// Espelha `NlpOptions` de `nlp.py` (linhas 30-44); os padroes sao os mesmos
/// do Python (ver [`NlpOptions::default`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NlpConfig {
    /// Limiares do classificador EN/PT.
    pub options: NlpOptions,
}

impl NlpConfig {
    /// Cria a configuracao a partir dos limiares.
    pub fn new(options: NlpOptions) -> Self {
        Self { options }
    }
}

impl Default for NlpConfig {
    /// Limiares padrao (identicos aos de `nlp.py`).
    fn default() -> Self {
        Self {
            options: NlpOptions::default(),
        }
    }
}

/// Opcoes da analise de um par (revisao antiga x nova do mesmo arquivo).
#[derive(Debug, Clone, PartialEq)]
pub struct AnalyzeOptions {
    /// Caminho exibido nos achados (ex.: `resources/scripts/App.tsx`).
    pub file: String,
    /// Especie (`js`|`ts`|`php`|`blade`|`html`); vazio deriva do `file`.
    pub kind: String,
    /// Entradas do glossario para `glossary-english`/`english-label`.
    pub glossary: Vec<GlossaryEntry>,
    /// Arquivo de teste: so assercoes contam como label.
    pub test_file: bool,
    /// Arquivo de idioma: toda string conta como label.
    pub lang_file: bool,
    /// Teto de achados antes do marcador `findings-truncated`.
    pub max_findings: usize,
    /// Limiares do classificador de idioma.
    pub nlp: NlpConfig,
}

impl Default for AnalyzeOptions {
    /// Padroes: sem arquivo/especie, sem glossario, `max_findings` 50.
    fn default() -> Self {
        Self {
            file: String::new(),
            kind: String::new(),
            glossary: Vec::new(),
            test_file: false,
            lang_file: false,
            max_findings: MAX_FINDINGS_PER_FILE,
            nlp: NlpConfig::default(),
        }
    }
}

/// Compila padrao estatico.
///
/// O `expect` aqui e proposital e localizado: so padroes constantes
/// revisadas chegam aqui, como nos modulos irmaos.
#[allow(clippy::expect_used)]
fn pattern(raw: &str) -> Regex {
    Regex::new(raw).expect("padrao regex estatico invalido")
}

fn cached(cell: &'static OnceLock<Regex>, raw: &str) -> &'static Regex {
    cell.get_or_init(|| pattern(raw))
}

static LETTER_CELL: OnceLock<Regex> = OnceLock::new();
static AUDIT_URL_CELL: OnceLock<Regex> = OnceLock::new();
static AUDIT_EMAIL_CELL: OnceLock<Regex> = OnceLock::new();
static AUDIT_DOMAIN_CELL: OnceLock<Regex> = OnceLock::new();
static AUDIT_FILE_CELL: OnceLock<Regex> = OnceLock::new();
static CONSOLE_CELL: OnceLock<Regex> = OnceLock::new();
static IMPORT_FROM_CELL: OnceLock<Regex> = OnceLock::new();
static REQUIRE_CELL: OnceLock<Regex> = OnceLock::new();
static CLASS_NAME_CELL: OnceLock<Regex> = OnceLock::new();
static CLASS_ATTR_CELL: OnceLock<Regex> = OnceLock::new();
static CATEGORY_USE_CELL: OnceLock<Regex> = OnceLock::new();

/// Colapsa espacos e trunca em 120 caracteres (`clip_text`, `common.py`, linha 82).
fn clip_text(value: &str) -> String {
    const MAX_LENGTH: usize = 120;
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() > MAX_LENGTH {
        let kept: String = collapsed.chars().take(MAX_LENGTH - 1).collect();
        format!("{kept}…")
    } else {
        collapsed
    }
}

/// Remove duplicadas preservando a ordem (`unique_strings`, `common.py`, linha 89).
fn unique_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut ordered = Vec::new();
    for value in values {
        if seen.insert(value.clone()) {
            ordered.push(value);
        }
    }
    ordered
}

/// Acumulador com teto (`push` de `analyze_pair`, `audit.py`, linhas 528-543).
///
/// Quando `findings.len() >= max`, novos achados sao descartados e um unico
/// marcador `findings-truncated` (severidade `blocker`, como no Python)
/// e anexado uma vez.
struct Capped {
    findings: Vec<Finding>,
    max: usize,
    truncated: bool,
}

impl Capped {
    fn new(max: usize) -> Self {
        Self {
            findings: Vec::new(),
            max,
            truncated: false,
        }
    }

    /// Anexa um achado respeitando o teto.
    fn push(
        &mut self,
        severity: Severity,
        code: &str,
        file: &str,
        line: u32,
        problem: String,
        fix: &str,
    ) {
        let file = if file.is_empty() { "(unknown)" } else { file };
        let line = line.max(1);
        let Ok(finding) = Finding::new(severity, code, file, line, &problem, fix) else {
            return;
        };
        self.push_ready(finding);
    }

    /// Anexa um achado ja construido respeitando o teto.
    fn push_ready(&mut self, finding: Finding) {
        if self.findings.len() < self.max {
            self.findings.push(finding);
            return;
        }
        if self.truncated {
            return;
        }
        self.truncated = true;
        // O marcador reusa arquivo/linha do achado descartado (sempre
        // validos), entao a construcao nunca falha; sem `unwrap` por lint.
        let problem = format!(
            "Mais de {} achados neste arquivo; a lista foi truncada.",
            self.max
        );
        if let Ok(marker) = Finding::new(
            Severity::Blocker,
            "findings-truncated",
            finding.file(),
            finding.line(),
            &problem,
            "Corrija os achados listados e rode a auditoria novamente.",
        ) {
            self.findings.push(marker);
        }
    }
}

/// Anexa um achado higienizando arquivo vazio e linha zero.
fn emit_finding(
    findings: &mut Vec<Finding>,
    severity: Severity,
    code: &str,
    file: &str,
    line: u32,
    problem: String,
    fix: &str,
) {
    let file = if file.is_empty() { "(unknown)" } else { file };
    let line = line.max(1);
    if let Ok(finding) = Finding::new(severity, code, file, line, &problem, fix) {
        findings.push(finding);
    }
}

/// Texto do label (`audit.py` usa `getattr(label, "text", "")`).
fn piece_text(piece: &Piece) -> &str {
    match piece {
        Piece::Str(entry) => entry.text.as_str(),
        Piece::Text(entry) => entry.text.as_str(),
        Piece::Code(entry) => entry.text.as_str(),
    }
}

/// Aponta labels novos em ingles (`audit.py`, linhas 138-178).
///
/// Limpa o texto (`clean_label_text` + `strip_non_label_tokens`), consulta o
/// glossario primeiro (`glossary-english`, `minor`) e depois os termos em
/// ingles (`english-label`, `minor`, com a variante `sem-evidencia-pt`).
pub fn label_english_findings_with<S, D>(
    file: &str,
    line: u32,
    text: &str,
    glossary: &[GlossaryEntry],
    options: &NlpOptions,
    zipf: &S,
    detector: &D,
) -> Vec<Finding>
where
    S: ZipfSource + ?Sized,
    D: LanguageDetector + ?Sized,
{
    let cleaned = strip_non_label_tokens(&clean_label_text(text));
    if cleaned.is_empty() {
        return Vec::new();
    }
    let words = english_terms_in_label_with(&cleaned, glossary, options, zipf, detector);
    let portuguese_evidence =
        portuguese_evidence_with(&cleaned, &words, glossary, options, zipf, detector);
    let terms = find_glossary_terms_with(&cleaned, glossary, portuguese_evidence, zipf, detector);
    if !terms.is_empty() {
        let listed = terms
            .iter()
            .map(|entry| format!("\"{}\" (usar \"{}\")", entry.term_en, entry.term_pt))
            .collect::<Vec<_>>()
            .join(", ");
        let fixed = terms
            .iter()
            .map(|entry| format!("\"{}\"", entry.term_pt))
            .collect::<Vec<_>>()
            .join(", ");
        let problem = format!("Label usa termo(s) em ingles: {listed}.");
        let fix = format!("Substitua por: {fixed}.");
        let file = if file.is_empty() { "(unknown)" } else { file };
        let line = line.max(1);
        return Finding::new(
            Severity::Minor,
            "glossary-english",
            file,
            line,
            &problem,
            &fix,
        )
        .ok()
        .into_iter()
        .collect();
    }
    if !words.is_empty() {
        let problem = if words.len() == 1 && words[0] == "sem-evidencia-pt" {
            format!(
                "Novo label sem evidencia de pt-BR: \"{}\".",
                clip_text(&cleaned)
            )
        } else {
            format!(
                "Novo label visivel em ingles: \"{}\" (termos: {}).",
                clip_text(&cleaned),
                words.join(", ")
            )
        };
        let file = if file.is_empty() { "(unknown)" } else { file };
        let line = line.max(1);
        return Finding::new(
            Severity::Minor,
            "english-label",
            file,
            line,
            &problem,
            "Traduza o label para pt-BR natural, mantendo placeholders e tom.",
        )
        .ok()
        .into_iter()
        .collect();
    }
    Vec::new()
}

/// Aponta labels novos em ingles com o stack padrao (Zipf embarcado + Lingua).
///
/// Equivale a [`label_english_findings_with`] com [`NlpOptions::default`],
/// [`EmbeddedZipf::global`] e [`LinguaDetector::global`].
pub fn label_english_findings(
    file: &str,
    line: u32,
    text: &str,
    glossary: &[GlossaryEntry],
) -> Vec<Finding> {
    label_english_findings_with(
        file,
        line,
        text,
        glossary,
        &NlpOptions::default(),
        EmbeddedZipf::global(),
        LinguaDetector::global(),
    )
}

/// Slot de pareamento (`label_slot`, `audit.py`, linhas 181-190).
fn label_slot(piece: &Piece) -> String {
    match piece {
        Piece::Str(entry) => {
            if let Some(attribute) = entry.attribute.as_ref() {
                return format!("attr:{}", attribute.to_lowercase());
            }
            if let Some(info) = context_identifier(entry.context.as_str()) {
                return format!("ctx:{}:{}", info.kind.as_str(), info.name);
            }
            "string".to_string()
        }
        Piece::Text(entry) => format!("tag:{}", entry.tag),
        Piece::Code(_) => "string".to_string(),
    }
}

/// Pareia labels antigos e novos por slot (`pair_labels`, `audit.py`, linhas 193-220).
///
/// Cada label antigo e usado no maximo uma vez (o mais antigo livre do mesmo
/// slot); labels novos sem par voltam em `unpaired`.
fn pair_labels(before: &[Piece], after: &[Piece]) -> (Vec<(Piece, Piece)>, Vec<Piece>) {
    let mut slots: HashMap<String, Vec<(Piece, bool)>> = HashMap::new();
    for label in before {
        slots
            .entry(label_slot(label))
            .or_default()
            .push((label.clone(), false));
    }
    let mut pairs = Vec::new();
    let mut unpaired = Vec::new();
    for label in after {
        let bucket = slots.entry(label_slot(label)).or_default();
        let position = bucket.iter().position(|(_, used)| !used);
        match position {
            Some(index) => {
                bucket[index].1 = true;
                pairs.push((bucket[index].0.clone(), label.clone()));
            }
            None => unpaired.push(label.clone()),
        }
    }
    (pairs, unpaired)
}

/// Informa se o label e traduzivel (`is_translatable_label`, `audit.py`, linhas 223-251).
///
/// Regras: ate 500 caracteres, com letra, sem URL/e-mail/dominio/caminho de
/// arquivo; texto JSX sempre vale; strings valem pelo atributo permitir
/// (`LABEL_ATTRIBUTES`) e pelo contexto nao ser `console.*`, `import/from`,
/// `require/import(` nem `className`/`class`.
fn is_translatable_label(piece: &Piece) -> bool {
    let raw = piece_text(piece).trim();
    if raw.is_empty() || raw.chars().count() > 500 {
        return false;
    }
    if !cached(&LETTER_CELL, r"[A-Za-zÀ-ÿ]").is_match(raw) {
        return false;
    }
    if cached(
        &AUDIT_URL_CELL,
        r"(?i)^(?:(?:https?|ftps?|sftp|file)://|(?:mailto|tel):|//)",
    )
    .is_match(raw)
        || raw.contains("://")
    {
        return false;
    }
    if cached(&AUDIT_EMAIL_CELL, r"^[^\s@]+@[^\s@]+\.[^\s@]+$").is_match(raw)
        || cached(
            &AUDIT_DOMAIN_CELL,
            r"^(?:[A-Za-z0-9-]+\.)+[A-Za-z]{2,}(?::\d+)?(?:/.*)?$",
        )
        .is_match(raw)
    {
        return false;
    }
    if cached(
        &AUDIT_FILE_CELL,
        r"(?i)^[A-Za-z0-9_./-]+\.(?:tsx?|jsx?|php|css|scss|json|svg|png|jpe?g|gif|webp|woff2?|ttf|eot)$",
    )
    .is_match(raw)
    {
        return false;
    }
    let Piece::Str(entry) = piece else {
        return matches!(piece, Piece::Text(_));
    };
    if let Some(attribute) = entry.attribute.as_ref() {
        if !LABEL_ATTRIBUTES.contains(&attribute.to_lowercase().as_str()) {
            return false;
        }
    }
    let context = entry.context.as_str();
    if cached(
        &CONSOLE_CELL,
        r"console\.(?:log|warn|error|debug|info|trace|dir|table|assert)\s*\([^)]*$",
    )
    .is_match(context)
    {
        return false;
    }
    if cached(&IMPORT_FROM_CELL, r"\b(?:import|export)\b[^;]*\bfrom\s*$").is_match(context) {
        return false;
    }
    if cached(&REQUIRE_CELL, r"\b(?:require|import)\s*\(\s*$").is_match(context) {
        return false;
    }
    if cached(&CLASS_NAME_CELL, r"className\s*[:=]\s*$").is_match(context)
        || cached(&CLASS_ATTR_CELL, r"\bclass\s*=\s*$").is_match(context)
    {
        return false;
    }
    true
}

/// Compara o mapa `categoryLabels` (`analyze_category_labels`, `audit.py`, linhas 254-346).
///
/// Devolve os textos normalizados (mapa apagado + lookups canonizados) e os
/// achados do mapa: diff de comentarios, tipo/chaves (`code-changed`),
/// remocao, uso sem declaracao e verificacao de idioma dos valores.
#[allow(clippy::too_many_arguments)]
fn analyze_category_labels_with<S, D>(
    old_text: &str,
    new_text: &str,
    file: &str,
    glossary: &[GlossaryEntry],
    options: &NlpOptions,
    zipf: &S,
    detector: &D,
) -> (String, String, Vec<Finding>)
where
    S: ZipfSource + ?Sized,
    D: LanguageDetector + ?Sized,
{
    let mut findings = Vec::new();
    let mut emit = |severity: Severity, code: &str, line: u32, problem: String, fix: &str| {
        let file = if file.is_empty() { "(unknown)" } else { file };
        let line = line.max(1);
        if let Ok(finding) = Finding::new(severity, code, file, line, &problem, fix) {
            findings.push(finding);
        }
    };
    let old_map = extract_category_labels(old_text);
    let new_map = extract_category_labels(new_text);
    let mut normalized_old = old_text.to_string();
    let mut normalized_new = new_text.to_string();
    if let Some(block) = old_map.as_ref() {
        normalized_old = blank_preserving_newlines(&normalized_old, block.start, block.end);
    }
    if let Some(block) = new_map.as_ref() {
        normalized_new = blank_preserving_newlines(&normalized_new, block.start, block.end);
    }
    normalized_old = replace_category_lookups(&normalized_old);
    normalized_new = replace_category_lookups(&normalized_new);
    if let (Some(old_block), Some(new_block)) = (old_map.as_ref(), new_map.as_ref()) {
        let old_comments = extract_comments(old_block.raw_block.as_str(), "ts");
        let new_comments = extract_comments(new_block.raw_block.as_str(), "ts");
        let limit = old_comments.len().max(new_comments.len());
        for position in 0..limit {
            let old_comment = old_comments.get(position);
            let new_comment = new_comments.get(position);
            let old_signature = old_comment.map(|c| format!("{}\x00{}", c.syntax, c.text));
            let new_signature = new_comment.map(|c| format!("{}\x00{}", c.syntax, c.text));
            if old_signature == new_signature {
                continue;
            }
            let line = match (new_comment, old_comment) {
                (Some(new_entry), _) => new_block
                    .line
                    .saturating_add(new_entry.line)
                    .saturating_sub(1),
                (None, Some(old_entry)) => old_block
                    .line
                    .saturating_add(old_entry.line.saturating_sub(1)),
                (None, None) => old_block.line,
            };
            emit(
                Severity::Blocker,
                "comment-changed",
                line.max(1),
                "Comentario alterado dentro do mapa categoryLabels.".to_string(),
                "Restaure o comentario original.",
            );
        }
        let type_changed = old_block.type_name != new_block.type_name;
        let keys_changed = old_block.keys != new_block.keys;
        if type_changed || keys_changed {
            let mut details = Vec::new();
            if type_changed {
                details.push(format!(
                    "tipo \"{}\" -> \"{}\"",
                    old_block.type_name, new_block.type_name
                ));
            }
            if keys_changed {
                details.push(format!(
                    "chaves [{}] -> [{}]",
                    old_block.keys.join(", "),
                    new_block.keys.join(", ")
                ));
            }
            emit(
                Severity::Blocker,
                "code-changed",
                new_block.line,
                format!("Mapa categoryLabels alterado: {}.", details.join("; ")),
                "Mantenha as chaves e o tipo do mapa; apenas os valores visiveis podem ser traduzidos.",
            );
        }
    } else if let Some(old_block) = old_map.as_ref() {
        if new_map.is_none() {
            emit(
                Severity::Blocker,
                "code-changed",
                old_block.line,
                "Mapa categoryLabels removido.".to_string(),
                "Restaure o mapa categoryLabels ou mantenha o render antigo de forma consistente.",
            );
        }
    }
    if new_map.is_none() && cached(&CATEGORY_USE_CELL, r"\bcategoryLabels\s*\[").is_match(new_text)
    {
        emit(
            Severity::Blocker,
            "code-changed",
            1,
            "Uso de categoryLabels sem a declaracao do mapa.".to_string(),
            "Declare o mapa categoryLabels antes de usa-lo.",
        );
    }
    if let Some(new_block) = new_map.as_ref() {
        for entry in &new_block.entries {
            findings.extend(label_english_findings_with(
                file,
                entry.line,
                entry.value.as_str(),
                glossary,
                options,
                zipf,
                detector,
            ));
        }
    }
    (normalized_old, normalized_new, findings)
}

/// Colapsa o esqueleto em uma linha (`collapse_skeleton`, `audit.py`, linhas 349-355).
fn collapse_skeleton(lines: &[SkeletonLine]) -> String {
    let joined = lines
        .iter()
        .map(|entry| entry.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let collapsed = cached(&COLLAPSE_WS_CELL, r"\s+")
        .replace_all(&joined, " ")
        .into_owned();
    let collapsed = cached(&COLLAPSE_BRACKET_CELL, r"\s*([<>{}])\s*")
        .replace_all(&collapsed, "$1")
        .into_owned();
    let collapsed = cached(&COLLAPSE_DOT_CELL, r"\s*\.\s*")
        .replace_all(&collapsed, ".")
        .into_owned();
    let collapsed = collapsed.replace("{\" \"}", "");
    cached(&COLLAPSE_COMMA_CELL, r",\s*([}\]])")
        .replace_all(&collapsed, "$1")
        .trim()
        .to_string()
}

static COLLAPSE_WS_CELL: OnceLock<Regex> = OnceLock::new();
static COLLAPSE_BRACKET_CELL: OnceLock<Regex> = OnceLock::new();
static COLLAPSE_DOT_CELL: OnceLock<Regex> = OnceLock::new();
static COLLAPSE_COMMA_CELL: OnceLock<Regex> = OnceLock::new();
static SKEL_WS_CELL: OnceLock<Regex> = OnceLock::new();
static SKEL_PARENS_CELL: OnceLock<Regex> = OnceLock::new();
static SKEL_DOT_CELL: OnceLock<Regex> = OnceLock::new();
static SKEL_COMMA_CELL: OnceLock<Regex> = OnceLock::new();
static SKEL_TRAIL_CELL: OnceLock<Regex> = OnceLock::new();
static SKEL_DOT_MERGE_CELL: OnceLock<Regex> = OnceLock::new();
static SKEL_Q_MERGE_CELL: OnceLock<Regex> = OnceLock::new();

/// Token do esqueleto com a linha e o indice da entrada de origem.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SkelToken {
    text: String,
    line: u32,
    entry: usize,
}

/// Tokeniza o esqueleto (`skeleton_tokens`, `audit.py`, linhas 358-383).
fn skeleton_tokens(entries: &[SkeletonLine]) -> Vec<SkelToken> {
    let mut tokens = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let mut normalized = cached(&SKEL_WS_CELL, r"\s+")
            .replace_all(entry.text.as_str(), " ")
            .into_owned();
        normalized = cached(&SKEL_PARENS_CELL, r"\s*([<>{}()])\s*")
            .replace_all(&normalized, " $1 ")
            .into_owned();
        normalized = cached(&SKEL_DOT_CELL, r"\s*\.\s*")
            .replace_all(&normalized, ".")
            .into_owned();
        normalized = normalized.replace("{\" \"}", "");
        normalized = cached(&SKEL_COMMA_CELL, r",\s*([}\]])")
            .replace_all(&normalized, "$1")
            .into_owned();
        normalized = cached(&SKEL_TRAIL_CELL, r",\s*$")
            .replace_all(&normalized, "")
            .trim()
            .to_string();
        if normalized.is_empty() {
            continue;
        }
        for raw in normalized.split_whitespace() {
            if raw.is_empty() {
                continue;
            }
            let clean = if raw.len() > 1 && raw.ends_with(',') {
                raw[..raw.len() - 1].to_string()
            } else {
                raw.to_string()
            };
            if clean.is_empty() {
                continue;
            }
            let merge_dot = cached(&SKEL_DOT_MERGE_CELL, r"^\.[^. ]").is_match(&clean);
            let merge_question = cached(&SKEL_Q_MERGE_CELL, r"^\?(\.|$)").is_match(&clean);
            if (merge_dot || merge_question) && !tokens.is_empty() {
                if let Some(previous) = tokens.last_mut() as Option<&mut SkelToken> {
                    previous.text.push_str(&clean);
                    continue;
                }
            }
            tokens.push(SkelToken {
                text: clean,
                line: entry.line,
                entry: index,
            });
        }
    }
    tokens
}

/// Remove parenteses de quebra do prettier (`drop_jsx_wrap_parens`, `audit.py`, linhas 386-420).
fn drop_jsx_wrap_parens(tokens: Vec<SkelToken>) -> Vec<SkelToken> {
    const PRECEDERS: [&str; 7] = ["&&", "||", "??", "?", ":", "return", "=>"];
    let mut current = tokens;
    loop {
        let mut dropped: HashSet<usize> = HashSet::new();
        for (position, token) in current.iter().enumerate() {
            if token.text != "(" {
                continue;
            }
            let previous = position.checked_sub(1).and_then(|i| current.get(i));
            let opener = match previous {
                Some(prev) => {
                    PRECEDERS.contains(&prev.text.as_str())
                        || (prev.text.len() > 1
                            && prev.text.ends_with('?')
                            && !prev.text.ends_with("?."))
                }
                None => false,
            };
            if !opener {
                continue;
            }
            let mut depth = 0;
            let mut closing: Option<usize> = None;
            let mut has_jsx = false;
            for (cursor, candidate) in current.iter().enumerate().skip(position) {
                if candidate.text == "(" {
                    depth += 1;
                } else if candidate.text == ")" {
                    depth -= 1;
                    if depth == 0 {
                        closing = Some(cursor);
                        break;
                    }
                } else if candidate.text == "<" {
                    has_jsx = true;
                }
            }
            if let Some(end) = closing {
                if has_jsx {
                    dropped.insert(position);
                    dropped.insert(end);
                }
            }
        }
        if dropped.is_empty() {
            return current;
        }
        current = current
            .into_iter()
            .enumerate()
            .filter(|(index, _)| !dropped.contains(index))
            .map(|(_, token)| token)
            .collect();
    }
}

/// Trecho alterado do diff de tokens (`audit.py`, linhas 471-488).
#[derive(Debug, Clone, PartialEq, Eq)]
struct TokenHunk {
    old: Vec<SkelToken>,
    new: Vec<SkelToken>,
}

/// Alinha hunks por LCS (`align_token_hunks`, `audit.py`, linhas 423-488).
///
/// Devolve `None` quando `height * width > 6000000`; o chamador degrada para
/// o diff legado por linha.
fn align_token_hunks(old_tokens: &[SkelToken], new_tokens: &[SkelToken]) -> Option<Vec<TokenHunk>> {
    let mut start = 0;
    while start < old_tokens.len()
        && start < new_tokens.len()
        && old_tokens[start].text == new_tokens[start].text
    {
        start += 1;
    }
    let mut old_end = old_tokens.len();
    let mut new_end = new_tokens.len();
    while old_end > start
        && new_end > start
        && old_tokens[old_end - 1].text == new_tokens[new_end - 1].text
    {
        old_end -= 1;
        new_end -= 1;
    }
    let height = old_end - start;
    let width = new_end - start;
    if height == 0 && width == 0 {
        return Some(Vec::new());
    }
    if height * width > 6_000_000 {
        return None;
    }
    let mut lengths = vec![vec![0usize; width + 1]; height + 1];
    for row in 1..=height {
        for column in 1..=width {
            if old_tokens[start + row - 1].text == new_tokens[start + column - 1].text {
                lengths[row][column] = lengths[row - 1][column - 1] + 1;
            } else {
                lengths[row][column] = lengths[row - 1][column].max(lengths[row][column - 1]);
            }
        }
    }
    let mut operations: Vec<i8> = Vec::new();
    let (mut row, mut column) = (height, width);
    while row > 0 && column > 0 {
        if old_tokens[start + row - 1].text == new_tokens[start + column - 1].text {
            operations.push(0);
            row -= 1;
            column -= 1;
        } else if lengths[row - 1][column] >= lengths[row][column - 1] {
            operations.push(-1);
            row -= 1;
        } else {
            operations.push(1);
            column -= 1;
        }
    }
    operations.extend(std::iter::repeat_n(-1, row));
    operations.extend(std::iter::repeat_n(1, column));
    operations.reverse();
    let mut hunks: Vec<TokenHunk> = Vec::new();
    let mut current: Option<usize> = None;
    let (mut old_cursor, mut new_cursor) = (start, start);
    for operation in operations {
        if operation == 0 {
            current = None;
            old_cursor += 1;
            new_cursor += 1;
            continue;
        }
        let index = match current {
            Some(index) => index,
            None => {
                hunks.push(TokenHunk {
                    old: Vec::new(),
                    new: Vec::new(),
                });
                let index = hunks.len() - 1;
                current = Some(index);
                index
            }
        };
        if operation == -1 {
            if let Some(token) = old_tokens.get(old_cursor) {
                hunks[index].old.push(token.clone());
            }
            old_cursor += 1;
        } else {
            if let Some(token) = new_tokens.get(new_cursor) {
                hunks[index].new.push(token.clone());
            }
            new_cursor += 1;
        }
    }
    hunks.retain(|hunk| !hunk.old.is_empty() || !hunk.new.is_empty());
    Some(hunks)
}

/// Diff legado por linha (`legacy_hunks`, `audit.py`, linhas 491-509).
fn legacy_hunks(old_lines: &[SkeletonLine], new_lines: &[SkeletonLine]) -> Vec<TokenHunk> {
    let mut hunks = Vec::new();
    let limit = old_lines.len().max(new_lines.len());
    for position in 0..limit {
        let old_line = old_lines.get(position);
        let new_line = new_lines.get(position);
        // Igualdade so pelo texto, como o Python (`old_line[text] ==
        // new_line[text]`): linhas cujo texto nao mudou sao puladas mesmo que
        // os numeros tenham deslocado.
        let same = match (old_line, new_line) {
            (Some(old_entry), Some(new_entry)) => old_entry.text == new_entry.text,
            (None, None) => true,
            _ => false,
        };
        if same {
            continue;
        }
        hunks.push(TokenHunk {
            old: old_line
                .map(|entry| SkelToken {
                    text: entry.text.clone(),
                    line: entry.line,
                    entry: position,
                })
                .into_iter()
                .collect(),
            new: new_line
                .map(|entry| SkelToken {
                    text: entry.text.clone(),
                    line: entry.line,
                    entry: position,
                })
                .into_iter()
                .collect(),
        });
    }
    hunks
}

/// Compara duas revisoes do mesmo arquivo (`analyze_pair`, `audit.py`, linhas 512-760).
///
/// Pipeline, na ordem do Python: fast path identico; labels de categoria
/// (comentarios/tipo/chaves/valores); diff posicional de comentarios
/// (`comment-changed`, `blocker`); diff de esqueleto por hunks
/// (`code-changed`, `blocker`); placeholders por label
/// (`placeholder-changed`, `blocker`, com normalizacao de literais);
/// URLs por label (`url-changed`, `blocker`); candidatos
/// (`english-label` / `glossary-english`, `minor`); teto com marcador
/// `findings-truncated`.
pub fn analyze_pair_with<S, D>(
    old_text: &str,
    new_text: &str,
    opts: &AnalyzeOptions,
    zipf: &S,
    detector: &D,
) -> Vec<Finding>
where
    S: ZipfSource + ?Sized,
    D: LanguageDetector + ?Sized,
{
    let file = opts.file.as_str();
    let mut capped = Capped::new(opts.max_findings);
    if normalize_newlines(old_text) == normalize_newlines(new_text) {
        return Vec::new();
    }
    let kind_key = if opts.kind.is_empty() {
        opts.file.clone()
    } else {
        opts.kind.clone()
    };
    let resolved = normalize_kind(Some(kind_key.as_str()));
    let kind_name = resolved.as_str().to_string();

    let (normalized_old, normalized_new, category_findings) = analyze_category_labels_with(
        old_text,
        new_text,
        file,
        &opts.glossary,
        &opts.nlp.options,
        zipf,
        detector,
    );
    let normalize_for_kind = |text: &str| -> String {
        if resolved == SourceKind::Blade {
            normalize_blade_echo_labels(text)
        } else {
            normalize_yup_label_args(text)
        }
    };

    let before = scan_source(
        normalized_old.as_str(),
        kind_name.as_str(),
        file,
        opts.lang_file,
        opts.test_file,
    );
    let after = scan_source(
        normalized_new.as_str(),
        kind_name.as_str(),
        file,
        opts.lang_file,
        opts.test_file,
    );
    let skeleton_before = scan_source(
        normalize_for_kind(normalized_old.as_str()).as_str(),
        kind_name.as_str(),
        file,
        opts.lang_file,
        opts.test_file,
    );
    let skeleton_after = scan_source(
        normalize_for_kind(normalized_new.as_str()).as_str(),
        kind_name.as_str(),
        file,
        opts.lang_file,
        opts.test_file,
    );

    for finding in category_findings {
        capped.push_ready(finding);
    }

    let comment_limit = before.comments.len().max(after.comments.len());
    for position in 0..comment_limit {
        let old_comment = before.comments.get(position);
        let new_comment = after.comments.get(position);
        let old_signature = old_comment.map(|c| format!("{}\x00{}", c.syntax, c.text));
        let new_signature = new_comment.map(|c| format!("{}\x00{}", c.syntax, c.text));
        if old_signature == new_signature {
            continue;
        }
        let line = new_comment
            .map(|c| c.line)
            .or_else(|| old_comment.map(|c| c.line))
            .unwrap_or(1);
        let problem = match (old_comment, new_comment) {
            (None, Some(new_entry)) => format!(
                "Comentario adicionado ({}): \"{}\".",
                new_entry.syntax,
                clip_text(&new_entry.text)
            ),
            (Some(old_entry), None) => format!(
                "Comentario removido ({}): \"{}\".",
                old_entry.syntax,
                clip_text(&old_entry.text)
            ),
            (Some(old_entry), Some(new_entry)) => format!(
                "Comentario alterado ({} -> {}): \"{}\" -> \"{}\".",
                old_entry.syntax,
                new_entry.syntax,
                clip_text(&old_entry.text),
                clip_text(&new_entry.text)
            ),
            (None, None) => continue,
        };
        capped.push(
            Severity::Blocker,
            "comment-changed",
            file,
            line,
            problem,
            "Restaure o comentario original; comentarios de codigo nao podem ser traduzidos.",
        );
    }

    if collapse_skeleton(&skeleton_before.lines) != collapse_skeleton(&skeleton_after.lines) {
        let old_tokens = drop_jsx_wrap_parens(skeleton_tokens(&skeleton_before.lines));
        let new_tokens = drop_jsx_wrap_parens(skeleton_tokens(&skeleton_after.lines));
        let hunks = align_token_hunks(&old_tokens, &new_tokens)
            .unwrap_or_else(|| legacy_hunks(&skeleton_before.lines, &skeleton_after.lines));
        for hunk in &hunks {
            let mut old_entries: Vec<usize> = Vec::new();
            for token in &hunk.old {
                if !old_entries.contains(&token.entry) {
                    old_entries.push(token.entry);
                }
            }
            let mut new_entries: Vec<usize> = Vec::new();
            for token in &hunk.new {
                if !new_entries.contains(&token.entry) {
                    new_entries.push(token.entry);
                }
            }
            let line = new_entries
                .first()
                .and_then(|index| skeleton_after.lines.get(*index))
                .map(|entry| entry.line)
                .or_else(|| {
                    old_entries
                        .first()
                        .and_then(|index| skeleton_before.lines.get(*index))
                        .map(|entry| entry.line)
                })
                .unwrap_or(1);
            let problem = if !hunk.old.is_empty()
                && !hunk.new.is_empty()
                && old_entries.len() == 1
                && new_entries.len() == 1
                && skeleton_before
                    .lines
                    .get(old_entries[0])
                    .map(|entry| entry.line)
                    == skeleton_after
                        .lines
                        .get(new_entries[0])
                        .map(|entry| entry.line)
            {
                let old_entry = &skeleton_before.lines[old_entries[0]];
                let new_entry = &skeleton_after.lines[new_entries[0]];
                format!(
                    "Codigo alterado: \"{}\" -> \"{}\".",
                    clip_text(&old_entry.text),
                    clip_text(&new_entry.text)
                )
            } else if hunk.new.is_empty() {
                let span = if old_entries.len() > 1 {
                    if let (Some(first), Some(last)) = (
                        skeleton_before.lines.get(old_entries[0]),
                        skeleton_before
                            .lines
                            .get(old_entries[old_entries.len() - 1]),
                    ) {
                        format!(" (linhas {}-{})", first.line, last.line)
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };
                let first = old_entries
                    .first()
                    .and_then(|index| skeleton_before.lines.get(*index))
                    .map(|entry| entry.text.as_str())
                    .unwrap_or("");
                format!("Codigo removido{span}: \"{}\".", clip_text(first))
            } else if hunk.old.is_empty() {
                let span = if new_entries.len() > 1 {
                    if let (Some(first), Some(last)) = (
                        skeleton_after.lines.get(new_entries[0]),
                        skeleton_after.lines.get(new_entries[new_entries.len() - 1]),
                    ) {
                        format!(" (linhas {}-{})", first.line, last.line)
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };
                let first = new_entries
                    .first()
                    .and_then(|index| skeleton_after.lines.get(*index))
                    .map(|entry| entry.text.as_str())
                    .unwrap_or("");
                format!("Codigo adicionado{span}: \"{}\".", clip_text(first))
            } else {
                let span_lines = if new_entries.is_empty() {
                    &old_entries
                } else {
                    &new_entries
                };
                let span = if span_lines.len() > 1 {
                    let (table, _) = if !new_entries.is_empty() {
                        (&skeleton_after.lines, true)
                    } else {
                        (&skeleton_before.lines, false)
                    };
                    if let (Some(first), Some(last)) = (
                        table.get(span_lines[0]),
                        table.get(span_lines[span_lines.len() - 1]),
                    ) {
                        format!(" (linhas {}-{})", first.line, last.line)
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };
                let old_first = old_entries
                    .first()
                    .and_then(|index| skeleton_before.lines.get(*index))
                    .map(|entry| entry.text.as_str())
                    .unwrap_or("");
                let new_first = new_entries
                    .first()
                    .and_then(|index| skeleton_after.lines.get(*index))
                    .map(|entry| entry.text.as_str())
                    .unwrap_or("");
                format!(
                    "Codigo alterado{span}: \"{}\" -> \"{}\". (+{} trecho(s) na regiao; ver diff completo)",
                    clip_text(old_first),
                    clip_text(new_first),
                    hunk.old.len() + hunk.new.len() - 2
                )
            };
            capped.push(
                Severity::Blocker,
                "code-changed",
                file,
                line,
                problem,
                CODE_FIX_MESSAGE,
            );
        }
    }

    let (pairs, unpaired_new) = pair_labels(&before.labels, &after.labels);
    for (old_label, new_label) in &pairs {
        let old_value = piece_text(old_label);
        let new_value = piece_text(new_label);
        if normalize_newlines(old_value) == normalize_newlines(new_value) {
            continue;
        }
        let old_placeholders = extract_placeholders(old_value);
        let new_placeholders = extract_placeholders(new_value);
        let old_normalized = old_placeholders
            .iter()
            .map(|item| normalize_placeholder_literals(item))
            .collect::<Vec<_>>();
        let new_normalized = new_placeholders
            .iter()
            .map(|item| normalize_placeholder_literals(item))
            .collect::<Vec<_>>();
        if same_sorted_multiset(&old_normalized, &new_normalized) {
            continue;
        }
        let missing = multiset_difference(&old_placeholders, &new_placeholders);
        let added = multiset_difference(&new_placeholders, &old_placeholders);
        let mut parts = Vec::new();
        if !missing.is_empty() {
            parts.push(format!("faltando [{}]", unique_strings(missing).join(", ")));
        }
        if !added.is_empty() {
            parts.push(format!(
                "adicionados [{}]",
                unique_strings(added).join(", ")
            ));
        }
        capped.push(
            Severity::Blocker,
            "placeholder-changed",
            file,
            new_label.line(),
            format!(
                "Placeholders alterados no label \"{}\": {}.",
                clip_text(new_value),
                parts.join("; ")
            ),
            "Preserve os placeholders exatamente como estavam no texto original.",
        );
    }
    for (old_label, new_label) in &pairs {
        let old_value = piece_text(old_label);
        let new_value = piece_text(new_label);
        if normalize_newlines(old_value) == normalize_newlines(new_value) {
            continue;
        }
        let old_urls = extract_urls(old_value);
        let new_urls = extract_urls(new_value);
        if same_sorted_multiset(&old_urls, &new_urls) {
            continue;
        }
        if old_urls.is_empty() && new_urls.is_empty() {
            continue;
        }
        let missing = multiset_difference(&old_urls, &new_urls);
        let added = multiset_difference(&new_urls, &old_urls);
        let mut parts = Vec::new();
        if !missing.is_empty() {
            parts.push(format!("faltando [{}]", unique_strings(missing).join(", ")));
        }
        if !added.is_empty() {
            parts.push(format!(
                "adicionados [{}]",
                unique_strings(added).join(", ")
            ));
        }
        capped.push(
            Severity::Blocker,
            "url-changed",
            file,
            new_label.line(),
            format!(
                "URLs alteradas no label \"{}\": {}.",
                clip_text(new_value),
                parts.join("; ")
            ),
            "Preserve as URLs exatamente como estavam no texto original.",
        );
    }

    let mut candidates: Vec<(Option<Piece>, Piece)> = unpaired_new
        .into_iter()
        .map(|label| (None, label))
        .collect();
    for (old_label, new_label) in &pairs {
        if clean_label_text(piece_text(old_label)) != clean_label_text(piece_text(new_label)) {
            candidates.push((Some(old_label.clone()), new_label.clone()));
        }
    }
    let mut seen: HashSet<String> = HashSet::new();
    for (_, label) in &candidates {
        if !is_translatable_label(label) {
            continue;
        }
        let key = format!(
            "{}\x00{}",
            label.line(),
            clean_label_text(piece_text(label))
        );
        if !seen.insert(key) {
            continue;
        }
        for finding in label_english_findings_with(
            file,
            label.line(),
            piece_text(label),
            &opts.glossary,
            &opts.nlp.options,
            zipf,
            detector,
        ) {
            capped.push_ready(finding);
        }
    }
    capped.findings
}

/// Compara duas revisoes do mesmo arquivo com o stack padrao.
///
/// Equivale a [`analyze_pair_with`] com [`EmbeddedZipf::global`] e
/// [`LinguaDetector::global`].
pub fn analyze_pair(old_text: &str, new_text: &str, opts: &AnalyzeOptions) -> Vec<Finding> {
    analyze_pair_with(
        old_text,
        new_text,
        opts,
        EmbeddedZipf::global(),
        LinguaDetector::global(),
    )
}

/// Confere os valores do overlay pt-BR (`analyze_overlay_values`, `audit.py`, linhas 763-821).
///
/// Para cada par `(en, pt)` ja comparado: traducao vazia (`empty-translation`,
/// `major`); placeholders com comparacao RAW por multiconjunto
/// (`placeholder-changed`, `blocker`); depois das guardas de
/// tamanho/letra/URL, verificacao de idioma (`english-label` /
/// `glossary-english`, `minor`). Comentarios do overlay sao permitidos: nunca
/// ha diff de comentarios aqui.
pub fn analyze_overlay_values_with_glossary<S, D>(
    common: &[(LangEntry, LangEntry)],
    file: &str,
    glossary: &[GlossaryEntry],
    options: &NlpOptions,
    zipf: &S,
    detector: &D,
) -> Vec<Finding>
where
    S: ZipfSource + ?Sized,
    D: LanguageDetector + ?Sized,
{
    let mut findings = Vec::new();
    for (english_entry, portuguese_entry) in common {
        let Some(value) = portuguese_entry.value.as_ref() else {
            continue;
        };
        let line = if portuguese_entry.value_line == 0 {
            portuguese_entry.line
        } else {
            portuguese_entry.value_line
        };
        if value.trim().is_empty() {
            emit_finding(
                &mut findings,
                Severity::Major,
                "empty-translation",
                file,
                line,
                format!("Traducao vazia para a chave \"{}\".", portuguese_entry.key),
                &format!(
                    "Preencha \"{}\" com o texto em pt-BR.",
                    portuguese_entry.key
                ),
            );
            continue;
        }
        if let Some(english_value) = english_entry.value.as_ref() {
            // Comparacao RAW (sem normalizacao de literais), como no Python.
            let old_placeholders = extract_placeholders(english_value);
            let new_placeholders = extract_placeholders(value);
            if !same_sorted_multiset(&old_placeholders, &new_placeholders) {
                let missing = multiset_difference(&old_placeholders, &new_placeholders);
                let added = multiset_difference(&new_placeholders, &old_placeholders);
                let mut parts = Vec::new();
                if !missing.is_empty() {
                    parts.push(format!("faltando [{}]", unique_strings(missing).join(", ")));
                }
                if !added.is_empty() {
                    parts.push(format!(
                        "adicionados [{}]",
                        unique_strings(added).join(", ")
                    ));
                }
                emit_finding(
                    &mut findings,
                    Severity::Blocker,
                    "placeholder-changed",
                    file,
                    line,
                    format!(
                        "Placeholders alterados na chave \"{}\": {}.",
                        portuguese_entry.key,
                        parts.join("; ")
                    ),
                    "Preserve os placeholders exatamente como estavam no texto original.",
                );
            }
        }
        let stripped = value.trim();
        if !stripped.is_empty()
            && stripped.chars().count() <= 500
            && cached(&LETTER_CELL, r"[A-Za-zÀ-ÿ]").is_match(stripped)
            && !cached(
                &AUDIT_URL_CELL,
                r"(?i)^(?:(?:https?|ftps?|sftp|file)://|(?:mailto|tel):|//)",
            )
            .is_match(stripped)
            && !stripped.contains("://")
            && !cached(&AUDIT_EMAIL_CELL, r"^[^\s@]+@[^\s@]+\.[^\s@]+$").is_match(stripped)
            && !cached(
                &AUDIT_DOMAIN_CELL,
                r"^(?:[A-Za-z0-9-]+\.)+[A-Za-z]{2,}(?::\d+)?(?:/.*)?$",
            )
            .is_match(stripped)
            && !cached(
                &AUDIT_FILE_CELL,
                r"(?i)^[A-Za-z0-9_./-]+\.(?:tsx?|jsx?|php|css|scss|json|svg|png|jpe?g|gif|webp|woff2?|ttf|eot)$",
            )
            .is_match(stripped)
        {
            findings.extend(label_english_findings_with(
                file,
                line,
                value,
                glossary,
                options,
                zipf,
                detector,
            ));
        }
    }
    findings
}

/// Confere os valores do overlay pt-BR com o stack padrao.
///
/// Equivale a [`analyze_overlay_values_with_glossary`] sem glossario (a
/// orquestracao usa a variante com glossario quando disponivel), com
/// [`NlpOptions::default`], [`EmbeddedZipf::global`] e
/// [`LinguaDetector::global`].
pub fn analyze_overlay_values(common: &[(LangEntry, LangEntry)], file: &str) -> Vec<Finding> {
    analyze_overlay_values_with_glossary(
        common,
        file,
        &[],
        &NlpOptions::default(),
        EmbeddedZipf::global(),
        LinguaDetector::global(),
    )
}

/// Aponta chaves ausentes ou extras no overlay (`audit`, `audit.py`, linhas 1269-1290).
///
/// `missing` sao entradas do en sem par; `extra`, do pt-BR sem origem.
/// Ambos viram `lang-key-mismatch` (`major`).
pub fn analyze_lang_key_mismatch(
    missing: &[LangEntry],
    extra: &[LangEntry],
    english_file: &str,
    portuguese_file: &str,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for entry in missing {
        emit_finding(
            &mut findings,
            Severity::Major,
            "lang-key-mismatch",
            english_file,
            entry.line,
            format!("Chave ausente em pt-BR: \"{}\".", entry.key),
            &format!("Adicione a chave \"{}\" em {portuguese_file}.", entry.key),
        );
    }
    for entry in extra {
        emit_finding(
            &mut findings,
            Severity::Major,
            "lang-key-mismatch",
            portuguese_file,
            entry.line,
            format!(
                "Chave extra em pt-BR: \"{}\" (nao existe em {english_file}).",
                entry.key
            ),
            &format!(
                "Remova a chave \"{}\" ou adicione-a em {english_file}.",
                entry.key
            ),
        );
    }
    findings
}

/// Garante que o arquivo en original nao foi alterado (`analyze_english_source`, linhas 824-883).
///
/// Qualquer chave adicionada, removida ou com valor trocado vira
/// `english-source-changed` (`blocker`); sem mudanca de chaves mas com texto
/// diferente, o primeiro comentario divergente colapsa em um unico
/// `comment-changed` (`blocker`).
pub fn analyze_english_source(old_text: &str, new_text: &str, file: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let before = extract_php_lang_entries(old_text);
    let after = extract_php_lang_entries(new_text);
    let mut before_map: HashMap<String, &LangEntry> = HashMap::new();
    let mut before_order: Vec<String> = Vec::new();
    for entry in &before {
        let key = entry_path_key(&entry.segments);
        if !before_map.contains_key(&key) {
            before_order.push(key.clone());
        }
        before_map.insert(key, entry);
    }
    let mut after_map: HashMap<String, &LangEntry> = HashMap::new();
    let mut after_order: Vec<String> = Vec::new();
    for entry in &after {
        let key = entry_path_key(&entry.segments);
        if !after_map.contains_key(&key) {
            after_order.push(key.clone());
        }
        after_map.insert(key, entry);
    }
    let mut order = before_order.clone();
    for key in &after_order {
        if !before_map.contains_key(key) {
            order.push(key.clone());
        }
    }
    for key in &order {
        let old_entry = before_map.get(key).copied();
        let new_entry = after_map.get(key).copied();
        let (problem, line) = match (old_entry, new_entry) {
            (None, Some(new_item)) => (
                Some(format!(
                    "Chave adicionada no arquivo en: \"{}\".",
                    new_item.key
                )),
                new_item.line,
            ),
            (Some(old_item), None) => (
                Some(format!(
                    "Chave removida do arquivo en: \"{}\".",
                    old_item.key
                )),
                old_item.line,
            ),
            (Some(old_item), Some(new_item)) if old_item.value != new_item.value => (
                Some(format!(
                    "Valor alterado no arquivo en para \"{}\".",
                    new_item.key
                )),
                if new_item.value_line == 0 {
                    new_item.line
                } else {
                    new_item.value_line
                },
            ),
            _ => (None, 1),
        };
        if let Some(detail) = problem {
            emit_finding(
                &mut findings,
                Severity::Blocker,
                "english-source-changed",
                file,
                line,
                format!("Arquivo de origem en e imutavel. {detail}"),
                "Reverta a alteracao em resources/lang/en e traduza apenas o overlay pt-BR.",
            );
        }
    }
    if findings.is_empty() && normalize_newlines(old_text) != normalize_newlines(new_text) {
        let old_comments = extract_comments(old_text, "php");
        let new_comments = extract_comments(new_text, "php");
        let limit = old_comments.len().max(new_comments.len());
        for position in 0..limit {
            let old_comment = old_comments.get(position);
            let new_comment = new_comments.get(position);
            let old_signature = old_comment.map(|c| format!("{}\x00{}", c.syntax, c.text));
            let new_signature = new_comment.map(|c| format!("{}\x00{}", c.syntax, c.text));
            if old_signature == new_signature {
                continue;
            }
            let line = new_comment
                .map(|c| c.line)
                .or_else(|| old_comment.map(|c| c.line))
                .unwrap_or(1);
            emit_finding(
                &mut findings,
                Severity::Blocker,
                "comment-changed",
                file,
                line,
                "Comentario alterado em arquivo en imutavel.".to_string(),
                "Restaure o comentario original.",
            );
            break;
        }
    }
    findings
}

/// Construtor do achado de orquestracao `translate-lang-file` (`major`).
///
/// A resolucao da chave contra o indice de lang pertence a orquestracao;
/// aqui so se materializa o achado (`audit.py`, linhas 1300-1309).
pub fn translate_lang_file_finding(file: &str, line: u32, key: &str) -> Option<Finding> {
    let file = if file.is_empty() { "(unknown)" } else { file };
    Finding::new(
        Severity::Major,
        "translate-lang-file",
        file,
        line.max(1),
        &format!("Chave de traducao \"{key}\" nao existe em resources/lang/en."),
        &format!(
            "Adicione \"{key}\" ao arquivo de lang correspondente ou use uma chave existente."
        ),
    )
    .ok()
}

/// Construtor do achado de orquestracao `out-of-scope` (`blocker`, `audit.py`, linhas 1191-1202).
pub fn out_of_scope_finding(file: &str) -> Option<Finding> {
    let file = if file.is_empty() { "(unknown)" } else { file };
    Finding::new(
        Severity::Blocker,
        "out-of-scope",
        file,
        1,
        &format!("Arquivo alterado fora do escopo da unidade/categoria: {file}."),
        "Reverta a alteracao ou mova o arquivo para a unidade correta.",
    )
    .ok()
}

/// Construtor do achado de orquestracao `glossary-missing` (`major`, `audit.py`, linhas 1179-1189).
pub fn glossary_missing_finding(glossary_path: &str) -> Option<Finding> {
    let file = if glossary_path.is_empty() {
        "(unknown)"
    } else {
        glossary_path
    };
    Finding::new(
        Severity::Major,
        "glossary-missing",
        file,
        1,
        "Glossario ausente; as verificacoes de terminologia nao podem rodar.",
        "Crie i18n/glossary.tsv com as colunas en, pt-BR e notes.",
    )
    .ok()
}

/// Construtor do achado de orquestracao `audit-incomplete` (`blocker`, `audit.py`, linhas 1311-1322).
pub fn audit_incomplete_finding(file: &str) -> Option<Finding> {
    let file = if file.is_empty() { "(unknown)" } else { file };
    Finding::new(
        Severity::Blocker,
        "audit-incomplete",
        file,
        1,
        "Ha mudancas rastreadas na unidade, mas nenhum arquivo foi analisado.",
        "Verifique a base informada e os caminhos da unidade antes de aprovar.",
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::StubDetector;

    fn test_glossary(rows: &[(&str, &str, &str)]) -> Vec<GlossaryEntry> {
        rows.iter()
            .map(|(en, pt, notes)| GlossaryEntry::new(en, pt, notes))
            .collect()
    }

    fn testing_options(glossary: Vec<GlossaryEntry>) -> AnalyzeOptions {
        AnalyzeOptions {
            file: "x.tsx".to_string(),
            kind: "js".to_string(),
            glossary,
            max_findings: MAX_FINDINGS_PER_FILE,
            ..Default::default()
        }
    }

    /// Analise com Zipf real + detector fixo (offline, deterministico).
    fn audit(old: &str, new: &str, opts: &AnalyzeOptions) -> Vec<Finding> {
        analyze_pair_with(
            old,
            new,
            opts,
            EmbeddedZipf::global(),
            &StubDetector::new(0.9, 0.0),
        )
    }

    fn audit_overlay(common: &[(LangEntry, LangEntry)], file: &str) -> Vec<Finding> {
        analyze_overlay_values_with_glossary(
            common,
            file,
            &[],
            &NlpOptions::default(),
            EmbeddedZipf::global(),
            &StubDetector::new(0.9, 0.0),
        )
    }

    fn has_code(findings: &[Finding], code: &str) -> bool {
        findings.iter().any(|f| f.code() == code)
    }

    fn count_code(findings: &[Finding], code: &str) -> usize {
        findings.iter().filter(|f| f.code() == code).count()
    }

    fn lang_pair(en_value: &str, pt_value: Option<&str>) -> (LangEntry, LangEntry) {
        (
            LangEntry {
                segments: vec!["messages".to_string()],
                key: "messages".to_string(),
                line: 2,
                value: Some(en_value.to_string()),
                value_line: 2,
            },
            LangEntry {
                segments: vec!["messages".to_string()],
                key: "messages".to_string(),
                line: 2,
                value: pt_value.map(str::to_string),
                value_line: 2,
            },
        )
    }

    #[test]
    fn identical_pair_returns_empty() {
        let opts = testing_options(vec![]);
        assert!(audit("const x = 1;\n", "const x = 1;\n", &opts).is_empty());
        // Fast path normaliza `\r\n` como o Python.
        assert!(audit("const x = 1;\r\n", "const x = 1;\n", &opts).is_empty());
    }

    #[test]
    fn empty_file_becomes_unknown_instead_of_failing() {
        // `Finding` exige arquivo nao vazio; o Python aceita `""`, aqui vai
        // `"(unknown)"` (ver F6 da revisao wave-3).
        let mut opts = testing_options(vec![]);
        opts.file = String::new();
        let findings = audit("// um\n", "// dois\n", &opts);
        assert!(findings.iter().all(|f| f.file() == "(unknown)"));
        assert!(has_code(&findings, "comment-changed"));
    }

    #[test]
    fn kind_falls_back_to_file_name() {
        let opts = AnalyzeOptions {
            file: "x.ts".to_string(),
            kind: String::new(),
            ..Default::default()
        };
        let findings = audit("// um\nconst x = 1;\n", "// dois\nconst x = 1;\n", &opts);
        assert!(has_code(&findings, "comment-changed"));
    }

    #[test]
    fn comment_add_remove_change() {
        let opts = testing_options(vec![]);
        let added = audit("const x = 1;\n", "// novo\nconst x = 1;\n", &opts);
        assert_eq!(count_code(&added, "comment-changed"), 1);
        assert!(added[0].problem().contains("adicionado"));

        let removed = audit("// velho\nconst x = 1;\n", "const x = 1;\n", &opts);
        assert_eq!(count_code(&removed, "comment-changed"), 1);
        assert!(removed[0].problem().contains("removido"));

        let changed = audit("// um\nconst x = 1;\n", "// dois\nconst x = 1;\n", &opts);
        assert_eq!(count_code(&changed, "comment-changed"), 1);
        assert!(changed[0].problem().contains("alterado"));
        assert_eq!(changed[0].severity(), Severity::Blocker);
    }

    #[test]
    fn comment_kind_change_flags() {
        let opts = testing_options(vec![]);
        let findings = audit(
            "// nota\nconst x = 1;\n",
            "/* nota */\nconst x = 1;\n",
            &opts,
        );
        assert!(has_code(&findings, "comment-changed"));
    }

    #[test]
    fn multiple_comments_accumulate() {
        let opts = testing_options(vec![]);
        let findings = audit("// um\n// dois\n", "// three\n// four\n", &opts);
        assert!(count_code(&findings, "comment-changed") >= 2);
    }

    #[test]
    fn code_changed_single_line_message() {
        let mut opts = testing_options(vec![]);
        opts.kind = "ts".to_string();
        let findings = audit(
            "export const alpha = 1;\n",
            "export const beta = 1;\n",
            &opts,
        );
        assert!(has_code(&findings, "code-changed"));
        let first = findings
            .iter()
            .find(|f| f.code() == "code-changed")
            .unwrap();
        assert!(first.problem().starts_with("Codigo alterado: "));
        assert!(first.problem().contains("->"));
    }

    #[test]
    fn code_removed_and_added_spans() {
        let mut opts = testing_options(vec![]);
        opts.kind = "ts".to_string();
        let removed = audit(
            "export const alpha = 1;\nexport const beta = 2;\n",
            "export const beta = 2;\n",
            &opts,
        );
        assert!(has_code(&removed, "code-changed"));
        let first = removed.iter().find(|f| f.code() == "code-changed").unwrap();
        assert!(first.problem().contains("Codigo removido"));

        let added = audit(
            "export const beta = 2;\n",
            "export const beta = 2;\nexport const gamma = 3;\n",
            &opts,
        );
        assert!(has_code(&added, "code-changed"));
        let first = added.iter().find(|f| f.code() == "code-changed").unwrap();
        assert!(first.problem().contains("Codigo adicionado"));
    }

    #[test]
    fn valid_translation_passes() {
        let opts = testing_options(vec![]);
        let findings = audit(
            "export default <div>Save</div>;\n",
            "export default <div>Salvar alterações</div>;\n",
            &opts,
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn placeholders_moved_across_labels() {
        let opts = testing_options(vec![]);
        let moved = audit(
            "export const message = 'Hello :name';\nexport const title = 'Goodbye friend';\n",
            "export const message = 'Ola';\nexport const title = 'Tchau :name';\n",
            &opts,
        );
        assert_eq!(count_code(&moved, "placeholder-changed"), 2);
    }

    #[test]
    fn placeholders_swapped_across_labels() {
        let opts = testing_options(vec![]);
        let swapped = audit(
            "const min = 'Min :min';\nconst max = 'Max :max';\n",
            "const min = 'Min :max';\nconst max = 'Max :min';\n",
            &opts,
        );
        assert_eq!(count_code(&swapped, "placeholder-changed"), 2);
    }

    #[test]
    fn placeholder_duplicate_counts() {
        let opts = testing_options(vec![]);
        let findings = audit(
            "const message = 'Hello :name :name';\n",
            "const message = 'Ola :name';\n",
            &opts,
        );
        assert_eq!(count_code(&findings, "placeholder-changed"), 1);
        let first = findings
            .iter()
            .find(|f| f.code() == "placeholder-changed")
            .unwrap();
        assert!(first.problem().contains(":name"));
        assert_eq!(first.severity(), Severity::Blocker);
    }

    #[test]
    fn placeholder_literal_normalization_matches_python() {
        let opts = testing_options(vec![]);
        let findings = audit(
            "({ title: \"Ver ${a('x')} agora\" })\n",
            "({ title: \"Ver ${a('y')} agora\" })\n",
            &opts,
        );
        assert!(
            !has_code(&findings, "placeholder-changed"),
            "literais normalizados nao deveriam diferir: {findings:?}"
        );
    }

    #[test]
    fn url_changed_per_label() {
        let opts = testing_options(vec![]);
        let findings = audit(
            "({ title: 'Ver https://exemplo.test/a' })\n",
            "({ title: 'Ver https://exemplo.test/b' })\n",
            &opts,
        );
        assert_eq!(count_code(&findings, "url-changed"), 1);
        assert_eq!(
            findings
                .iter()
                .find(|f| f.code() == "url-changed")
                .unwrap()
                .severity(),
            Severity::Blocker
        );
    }

    #[test]
    fn url_preserved_translation_passes() {
        let opts = testing_options(vec![]);
        let findings = audit(
            "({ title: 'See https://exemplo.test/a now' })\n",
            "({ title: 'Ver https://exemplo.test/a agora' })\n",
            &opts,
        );
        assert!(
            !has_code(&findings, "url-changed"),
            "URL preservada nao deveria sinalizar: {findings:?}"
        );
    }

    #[test]
    fn english_label_without_portuguese_evidence() {
        let opts = testing_options(vec![]);
        let findings = audit("<div>Other</div>;\n", "<div>Xyz Abc</div>;\n", &opts);
        assert_eq!(count_code(&findings, "english-label"), 1);
        let first = findings
            .iter()
            .find(|f| f.code() == "english-label")
            .unwrap();
        assert!(first.problem().contains("sem evidencia de pt-BR"));
        assert_eq!(first.severity(), Severity::Minor);
    }

    #[test]
    fn english_label_lists_terms() {
        let opts = testing_options(vec![]);
        let findings = audit("<div>Other</div>;\n", "<div>Save changes</div>;\n", &opts);
        assert!(has_code(&findings, "english-label"));
        let first = findings
            .iter()
            .find(|f| f.code() == "english-label")
            .unwrap();
        assert!(first.problem().contains("termos:"));
    }

    #[test]
    fn glossary_english_wins_over_generic_detection() {
        let opts = testing_options(test_glossary(&[("Save", "Salvar", "")]));
        let findings = audit("<div>Other</div>;\n", "<div>Save</div>;\n", &opts);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code(), "glossary-english");
        assert_eq!(findings[0].severity(), Severity::Minor);
    }

    #[test]
    fn findings_capped_with_truncation_marker() {
        let mut old = String::new();
        let mut new = String::new();
        for index in 0..60 {
            old.push_str(&format!("// c{index}\n"));
            new.push_str(&format!("// d{index}\n"));
        }
        let mut opts = testing_options(vec![]);
        opts.max_findings = 5;
        let findings = audit(&old, &new, &opts);
        assert_eq!(count_code(&findings, "comment-changed"), 5);
        assert!(has_code(&findings, "findings-truncated"));
        let marker = findings
            .iter()
            .find(|f| f.code() == "findings-truncated")
            .unwrap();
        assert_eq!(marker.severity(), Severity::Blocker);
        assert_eq!(findings.len(), 6);
    }

    #[test]
    fn category_labels_insertion_is_display_only() {
        let opts = testing_options(vec![]);
        let old = "const categories = ['general', 'modules'] as const;\nexport function AdminRouter() {\n  return <Sidebar.Section>{category[0]!.toUpperCase() + category.slice(1)}</Sidebar.Section>;\n}\n";
        let new = "const categories = ['general', 'modules'] as const;\nconst categoryLabels: Record<(typeof categories)[number], string> = {\n  general: 'Geral',\n  modules: 'Módulos',\n};\nexport function AdminRouter() {\n  return <Sidebar.Section>{categoryLabels[category]}</Sidebar.Section>;\n}\n";
        let findings = audit(old, new, &opts);
        assert!(findings.is_empty(), "insercao display-only: {findings:?}");
    }

    #[test]
    fn category_labels_value_in_english_flags() {
        let opts = testing_options(vec![]);
        let old = "const categoryLabels = {\n  general: 'Geral',\n  modules: 'Módulos',\n};\n";
        let new = old.replace("general: 'Geral'", "general: 'General'");
        let findings = audit(old, &new, &opts);
        assert!(
            has_code(&findings, "english-label") || has_code(&findings, "glossary-english"),
            "valor em ingles deveria sinalizar: {findings:?}"
        );
    }

    #[test]
    fn category_labels_keys_and_type_stay_validated() {
        let opts = testing_options(vec![]);
        let old = "const categoryLabels: Record<(typeof categories)[number], string> = {\n  general: 'Geral',\n  modules: 'Módulos',\n};\n";
        let key_changed = old.replace("modules:", "services:");
        assert!(has_code(&audit(old, &key_changed, &opts), "code-changed"));
        let type_changed = old.replace("string>", "number>");
        assert!(has_code(&audit(old, &type_changed, &opts), "code-changed"));
    }

    #[test]
    fn category_labels_removal_flags() {
        let opts = testing_options(vec![]);
        let old = "const categoryLabels = {\n  general: 'Geral',\n};\nconst t = categoryLabels[category];\n";
        let new = "const t = category;\n";
        let findings = audit(old, new, &opts);
        assert!(has_code(&findings, "code-changed"));
    }

    #[test]
    fn category_labels_use_without_declaration_flags() {
        let opts = testing_options(vec![]);
        let findings = audit(
            "const t = category;\n",
            "const t = categoryLabels[category];\n",
            &opts,
        );
        assert!(has_code(&findings, "code-changed"));
    }

    #[test]
    fn test_file_flag_gates_assertion_labels() {
        let old = "getByText('Save')\n";
        let new = "getByText('Salvar')\n";
        let mut test_opts = testing_options(vec![]);
        test_opts.test_file = true;
        assert!(
            audit(old, new, &test_opts).is_empty(),
            "assercao traduzida deveria passar"
        );
        let plain_opts = testing_options(vec![]);
        assert!(
            has_code(&audit(old, new, &plain_opts), "code-changed"),
            "fora de teste a string nao e label"
        );
    }

    #[test]
    fn lang_file_flag_labels_every_string() {
        let old = "const internal_code = \"Save\";\n";
        let new = "const internal_code = \"Salvar\";\n";
        let mut lang_opts = testing_options(vec![]);
        lang_opts.lang_file = true;
        assert!(
            audit(old, new, &lang_opts).is_empty(),
            "arquivo de idioma traduzido deveria passar"
        );
        let plain_opts = testing_options(vec![]);
        assert!(
            has_code(&audit(old, new, &plain_opts), "code-changed"),
            "fora de lang a string e codigo"
        );
    }

    #[test]
    fn php_comment_pair_flags() {
        let mut opts = testing_options(vec![]);
        opts.file = "x.php".to_string();
        opts.kind = "php".to_string();
        let findings = audit("<?php // oi\n", "<?php // ola\n", &opts);
        assert!(has_code(&findings, "comment-changed"));
    }

    #[test]
    fn html_translation_passes() {
        let mut opts = testing_options(vec![]);
        opts.file = "x.html".to_string();
        opts.kind = "html".to_string();
        let findings = audit("<div>Save</div>\n", "<div>Salvar</div>\n", &opts);
        assert!(findings.is_empty(), "traducao html: {findings:?}");
    }

    #[test]
    fn overlay_empty_translation_flags() {
        let pair = lang_pair("Hello", Some(""));
        let findings = audit_overlay(&[pair], "resources/lang/pt-BR/x.php");
        assert_eq!(count_code(&findings, "empty-translation"), 1);
        assert_eq!(findings[0].severity(), Severity::Major, "vazia e major");
    }

    #[test]
    fn overlay_none_value_is_skipped() {
        let pair = lang_pair("Hello", None);
        assert!(audit_overlay(&[pair], "resources/lang/pt-BR/x.php").is_empty());
    }

    #[test]
    fn overlay_placeholder_swap_flags() {
        let pair = lang_pair("Hello :name", Some("Olá :nome"));
        let findings = audit_overlay(&[pair], "resources/lang/pt-BR/x.php");
        assert_eq!(count_code(&findings, "placeholder-changed"), 1);
        let first = findings
            .iter()
            .find(|f| f.code() == "placeholder-changed")
            .unwrap();
        assert_eq!(first.severity(), Severity::Blocker);
        assert!(first.problem().contains(":nome") || first.problem().contains(":name"));
    }

    #[test]
    fn overlay_valid_translation_passes() {
        let pair = lang_pair("Hello :name", Some("Olá :name"));
        let findings = audit_overlay(&[pair], "resources/lang/pt-BR/x.php");
        assert!(findings.is_empty(), "overlay valido: {findings:?}");
    }

    #[test]
    fn overlay_allows_comments_without_comment_diff() {
        // O overlay nao compara comentarios: texto com `//` traduzido passa.
        let pair = lang_pair("Use // flag", Some("Use a // sinalizacao"));
        let findings = audit_overlay(&[pair], "resources/lang/pt-BR/x.php");
        assert!(
            !has_code(&findings, "comment-changed"),
            "overlay nunca emite comment-changed: {findings:?}"
        );
    }

    #[test]
    fn lang_key_mismatch_reports_both_sides() {
        let missing = vec![LangEntry {
            segments: vec!["gone".to_string()],
            key: "gone".to_string(),
            line: 4,
            value: Some("Gone".to_string()),
            value_line: 4,
        }];
        let extra = vec![LangEntry {
            segments: vec!["sobra".to_string()],
            key: "sobra".to_string(),
            line: 7,
            value: Some("Sobra".to_string()),
            value_line: 7,
        }];
        let findings = analyze_lang_key_mismatch(
            &missing,
            &extra,
            "resources/lang/en/x.php",
            "overlays/resources/lang/pt-BR/x.php",
        );
        assert_eq!(count_code(&findings, "lang-key-mismatch"), 2);
        assert!(findings.iter().all(|f| f.severity() == Severity::Major));
    }

    #[test]
    fn english_source_add_remove_change() {
        let base = "<?php return ['a' => 'x'];\n";
        let added = audit_incomplete_swap(base, "<?php return ['a' => 'x', 'b' => 'y'];\n", "add");
        assert!(added.problem().contains("adicionada"));
        let removed = audit_incomplete_swap(base, "<?php return [];\n", "remove");
        assert!(removed.problem().contains("removida"));
        let changed = audit_incomplete_swap(base, "<?php return ['a' => 'z'];\n", "change");
        assert!(changed.problem().contains("alterado"));
        for finding in [&added, &removed, &changed] {
            assert_eq!(finding.code(), "english-source-changed");
            assert_eq!(finding.severity(), Severity::Blocker);
        }
    }

    /// Helper local: primeira descoberta de `analyze_english_source`.
    fn audit_incomplete_swap(base: &str, new: &str, _kind: &str) -> Finding {
        analyze_english_source(base, new, "resources/lang/en/x.php")
            .into_iter()
            .next()
            .unwrap()
    }

    #[test]
    fn english_source_comment_only_collapses_to_one() {
        let findings = analyze_english_source(
            "<?php // base\nreturn ['a' => 'x'];\n",
            "<?php // mudado\nreturn ['a' => 'x'];\n",
            "resources/lang/en/x.php",
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code(), "comment-changed");
    }

    #[test]
    fn orchestration_constructors_cover_remaining_codes() {
        let lang_file = translate_lang_file_finding("x.tsx", 3, "messages.title").unwrap();
        assert_eq!(lang_file.code(), "translate-lang-file");
        assert_eq!(lang_file.severity(), Severity::Major);

        let scoped = out_of_scope_finding("other/file.ts").unwrap();
        assert_eq!(scoped.code(), "out-of-scope");
        assert_eq!(scoped.line(), 1);

        let glossary = glossary_missing_finding("i18n/glossary.tsv").unwrap();
        assert_eq!(glossary.code(), "glossary-missing");
        assert_eq!(glossary.severity(), Severity::Major);

        let incomplete = audit_incomplete_finding("a.ts").unwrap();
        assert_eq!(incomplete.code(), "audit-incomplete");
        assert_eq!(incomplete.severity(), Severity::Blocker);
    }

    #[test]
    fn real_stack_integration_portuguese_passes_english_flags() {
        // Unico teste com o stack embarcado completo (Lingua + Zipf).
        let opts = testing_options(vec![]);
        let passing = analyze_pair(
            "export default <div>Save</div>;\n",
            "export default <div>Salvar alterações</div>;\n",
            &opts,
        );
        assert!(passing.is_empty(), "pilha real: {passing:?}");
        let flagging = analyze_pair(
            "export default <div>Save</div>;\n",
            "export default <div>Save changes now please</div>;\n",
            &opts,
        );
        assert!(
            flagging
                .iter()
                .any(|f| f.code() == "english-label" || f.code() == "glossary-english"),
            "pilha real deveria sinalizar: {flagging:?}"
        );
    }
}
