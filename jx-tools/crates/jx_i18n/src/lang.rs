//! Classificação de idioma de rótulos com evidência dinâmica (sem listas fixas).
//!
//! Porte fiel de `scripts/i18n/i18n_toolkit/nlp.py`: detecção EN/PT com
//! Lingua restrita a inglês e português, evidência léxica por frequências
//! Zipf e regras estruturais (glossário, acentos, sentinelas) preservadas.
//!
//! Tokenização e acentos vêm de [`crate::text`] exatamente como o original
//! usa (`word_tokens` e o padrão de acentos portugueses; `clean_label_text`
//! e `strip_non_label_tokens` não são chamados por `nlp.py`, então também
//! não são chamados aqui — a limpeza acontece antes, no chamador).
//!
//! Tabelas Zipf: `data/zipf_en.tsv.gz` e `data/zipf_pt.tsv.gz`, uma linha
//! `palavra\tzipf` por vocábulo, geradas a partir do pacote `wordfreq`
//! (`large_en`/`large_pt`, verificadas entrada a entrada contra
//! `zipf_frequency`). As tabelas são completas (sem poda: a fidelidade dos
//! limiares depende disso) e descomprimidas uma única vez por processo via
//! `flate2` + [`std::sync::OnceLock`].
//!
//! Aviso de redistribuição: os dados de frequência derivam do `wordfreq`,
//! cujas listas agregam fontes com copyleft; ao redistribuir as tabelas
//! embarcadas, observe a exigência de CC-BY-SA 4.0 indicada pelo projeto
//! de origem dos dados.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io::Read as _;
use std::sync::OnceLock;

use flate2::read::GzDecoder;
use lingua::{Language, LanguageDetectorBuilder};
use regex::Regex;

use crate::text::{has_portuguese_accents, word_tokens};

/// Compila padrão estático.
///
/// O `expect` aqui é proposital e localizado: só padrões constantes
/// revisados chegam aqui, e qualquer regressão quebra os testes na hora.
#[allow(clippy::expect_used)]
fn static_pattern(raw: &str) -> Regex {
    Regex::new(raw).expect("padrão regex estático inválido")
}

/// Padrão `mantid|manter|mant[eê]m` (`nlp.py`, linha 109; `IGNORECASE`).
fn keep_term_pattern() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| static_pattern(r"(?i)mantid|manter|mant[eê]m"))
}

/// Padrão `^[A-Za-z][A-Za-z0-9 ,.'!?:;()/&+-]*$` (`nlp.py`, linha 89).
fn ascii_label_pattern() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| static_pattern(r"^[A-Za-z][A-Za-z0-9 ,.'!?:;()/&+-]*$"))
}

/// Limiares do classificador. Valores calibrados na suíte de testes.
///
/// Espelha `NlpOptions` de `nlp.py` (linhas 30-44) com os mesmos padrões.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NlpOptions {
    /// Margem do `gap` pt−en para um token contar como evidência (padrão `1.0`).
    pub token_margin: f64,
    /// Tamanho máximo para a regra do token curto (padrão `3`).
    pub short_token_max_length: usize,
    /// Zipf EN mínimo para o token curto sinalizar inglês (padrão `5.0`).
    pub short_token_en_zipf: f64,
    /// Teto da confiança PT para o token curto sinalizar inglês (padrão `0.7`).
    pub short_token_pt_ceiling: f64,
    /// Confiança EN mínima para token único isolado (padrão `0.6`).
    pub single_token_en_clear: f64,
    /// Confiança PT mínima do texto corrido (padrão `0.65`).
    pub text_portuguese_threshold: f64,
    /// Confiança EN mínima da varredura rápida (padrão `0.6`).
    pub english_label_threshold: f64,
    /// Margem de ambiguidade entre confianças EN/PT (padrão `0.4`).
    pub ambiguity_margin: f64,
    /// `gap` abaixo do qual o token é conteúdo ambíguo (padrão `1.0`).
    pub zipf_ambiguity_gap: f64,
    /// Zipf PT mínimo da palavra funcional de resgate (padrão `6.0`).
    pub function_word_pt_zipf: f64,
    /// `gap` mínimo (exclusivo) da palavra funcional (padrão `-1.0`).
    pub function_word_max_gap: f64,
}

impl Default for NlpOptions {
    /// Padrões idênticos aos de `nlp.py`.
    fn default() -> Self {
        Self {
            token_margin: 1.0,
            short_token_max_length: 3,
            short_token_en_zipf: 5.0,
            short_token_pt_ceiling: 0.7,
            single_token_en_clear: 0.6,
            text_portuguese_threshold: 0.65,
            english_label_threshold: 0.6,
            ambiguity_margin: 0.4,
            zipf_ambiguity_gap: 1.0,
            function_word_pt_zipf: 6.0,
            function_word_max_gap: -1.0,
        }
    }
}

/// Entrada do glossário (`termo_en`, `termo_pt`, `notas`).
///
/// Espelha `GlossaryEntry` de `models.py` (aliases `en`/`pt`/`notes`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlossaryEntry {
    /// Termo em inglês (alias `en` no Python).
    pub term_en: String,
    /// Termo em português (alias `pt` no Python).
    pub term_pt: String,
    /// Notas de uso (alias `notes`; ex.: `manter`, `usar "X" como substantivo`).
    pub notes: String,
}

impl GlossaryEntry {
    /// Cria uma entrada a partir dos três campos textuais.
    pub fn new(term_en: &str, term_pt: &str, notes: &str) -> Self {
        Self {
            term_en: term_en.to_string(),
            term_pt: term_pt.to_string(),
            notes: notes.to_string(),
        }
    }
}

/// Fonte de frequências Zipf por idioma.
///
/// O token desconhecido (OOV) devolve `0.0`, como `wordfreq.zipf_frequency`
/// com o mínimo padrão.
pub trait ZipfSource {
    /// Frequência Zipf do `token` em `lang` (`"en"`/`"pt"`); OOV devolve `0.0`.
    fn zipf(&self, token: &str, lang: &str) -> f64;
}

/// Tabelas Zipf EN/PT embarcadas (descomprimidas uma vez por processo).
#[derive(Debug)]
pub struct EmbeddedZipf {
    english: HashMap<String, f64>,
    portuguese: HashMap<String, f64>,
}

/// Interpreta um `tsv.gz` de `palavra\tzipf`; linhas inválidas são ignoradas.
fn parse_zipf_tsv_gz(bytes: &[u8]) -> HashMap<String, f64> {
    let mut table = HashMap::new();
    let mut text = String::new();
    if GzDecoder::new(bytes).read_to_string(&mut text).is_err() {
        return table;
    }
    for line in text.lines() {
        let Some((word, value)) = line.split_once('\t') else {
            continue;
        };
        if word.is_empty() {
            continue;
        }
        let Ok(zipf) = value.parse::<f64>() else {
            continue;
        };
        table.insert(word.to_string(), zipf);
    }
    table
}

impl EmbeddedZipf {
    /// Tabelas compartilhadas (descompressão única e tardia por processo).
    pub fn global() -> &'static Self {
        static CELL: OnceLock<EmbeddedZipf> = OnceLock::new();
        CELL.get_or_init(|| Self {
            english: parse_zipf_tsv_gz(include_bytes!("../data/zipf_en.tsv.gz")),
            portuguese: parse_zipf_tsv_gz(include_bytes!("../data/zipf_pt.tsv.gz")),
        })
    }
}

impl ZipfSource for EmbeddedZipf {
    /// Busca com o token em minúsculas; idioma desconhecido devolve `0.0`.
    fn zipf(&self, token: &str, lang: &str) -> f64 {
        let normalized = lang.to_lowercase();
        let table = if normalized == "en" || normalized == "english" {
            &self.english
        } else if normalized == "pt"
            || normalized == "portuguese"
            || normalized.starts_with("pt-")
            || normalized.starts_with("pt_")
        {
            &self.portuguese
        } else {
            return 0.0;
        };
        table
            .get(token.to_lowercase().as_str())
            .copied()
            .unwrap_or(0.0)
    }
}

/// Detector de idiomas restrito a inglês e português.
///
/// Devolve o par `(confiança_en, confiança_pt)`, na mesma ordem em que
/// `nlp.py` lê `language_confidences` (`english`, `portuguese`).
pub trait LanguageDetector {
    /// Confidências `(en, pt)` para o texto (somam `1.0`, salvo texto vazio).
    fn confidences(&self, text: &str) -> (f64, f64);
}

/// Detector EN/PT de produção sobre o `lingua-rs`, compartilhado no processo.
pub struct LinguaDetector {
    inner: lingua::LanguageDetector,
}

impl LinguaDetector {
    /// Constrói um detector novo (carrega os modelos; prefira [`Self::global`]).
    pub fn new() -> Self {
        let mut builder =
            LanguageDetectorBuilder::from_languages(&[Language::English, Language::Portuguese]);
        Self {
            inner: builder.build(),
        }
    }

    /// Detector compartilhado (construção única e tardia por processo).
    ///
    /// Espelha `language_detector()` de `nlp.py` (linhas 59-68).
    pub fn global() -> &'static Self {
        static CELL: OnceLock<LinguaDetector> = OnceLock::new();
        CELL.get_or_init(Self::new)
    }
}

impl Default for LinguaDetector {
    /// Equivale a [`LinguaDetector::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageDetector for LinguaDetector {
    /// Confidências do Lingua para EN/PT (ausente vira `0.0`).
    fn confidences(&self, text: &str) -> (f64, f64) {
        let mut english = 0.0;
        let mut portuguese = 0.0;
        for (language, value) in self.inner.compute_language_confidence_values(text) {
            match language {
                Language::English => english = value,
                Language::Portuguese => portuguese = value,
            }
        }
        (english, portuguese)
    }
}

/// Detector fixo para testes (confidências constantes, sem modelos).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StubDetector {
    /// Confiança de inglês devolvida para qualquer texto.
    pub english: f64,
    /// Confiança de português devolvida para qualquer texto.
    pub portuguese: f64,
}

impl StubDetector {
    /// Cria o stub com o par `(english, portuguese)`.
    pub fn new(english: f64, portuguese: f64) -> Self {
        Self {
            english,
            portuguese,
        }
    }
}

impl LanguageDetector for StubDetector {
    /// Devolve o par constante, ignorando o texto.
    fn confidences(&self, _text: &str) -> (f64, f64) {
        (self.english, self.portuguese)
    }
}

/// Idioma atribuído a um rótulo.
///
/// Espelha `LabelLanguage` de `nlp.py` (linhas 22-27).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LabelLanguage {
    /// Português.
    Pt,
    /// Inglês.
    En,
    /// Misto (termos em inglês + tokens em português).
    Mixed,
    /// Ambíguo (sentinela `sem-evidencia-pt` ou sem evidência).
    Ambiguous,
    /// Técnico (sem letras).
    Technical,
}

impl LabelLanguage {
    /// Código canônico (`pt`, `en`, `mixed`, `ambiguous`, `technical`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pt => "pt",
            Self::En => "en",
            Self::Mixed => "mixed",
            Self::Ambiguous => "ambiguous",
            Self::Technical => "technical",
        }
    }
}

impl fmt::Display for LabelLanguage {
    /// Exibe o código canônico.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Veredito da classificação de um rótulo.
///
/// Espelha `LabelVerdict` de `nlp.py` (linhas 47-54).
#[derive(Debug, Clone, PartialEq)]
pub struct LabelVerdict {
    /// Idioma atribuído.
    pub language: LabelLanguage,
    /// Confiança do veredito.
    pub confidence: f64,
    /// Termos em inglês (vazio quando há evidência de português).
    pub english_terms: Vec<String>,
    /// Tokens com evidência de português.
    pub portuguese_terms: Vec<String>,
    /// Tokens sem sinal claro em nenhum idioma.
    pub ambiguous_terms: Vec<String>,
}

/// Idiomas suportados pelo detector (escopo restrito EN/PT).
///
/// Espelha `SupportedLanguages` de `nlp.py` (linhas 361-367).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupportedLanguages {
    /// Idioma principal.
    pub principal: &'static str,
    /// Idioma secundário.
    pub secundario: &'static str,
}

/// Idiomas configurados (`pt` principal, `en` secundário).
pub const SUPPORTED_LANGUAGES: SupportedLanguages = SupportedLanguages {
    principal: "pt",
    secundario: "en",
};

/// Idiomas configurados no detector (escopo restrito EN/PT).
///
/// Espelha `detector_languages()` de `nlp.py` (linhas 370-372).
pub fn detector_languages() -> (&'static str, &'static str) {
    (
        SUPPORTED_LANGUAGES.principal,
        SUPPORTED_LANGUAGES.secundario,
    )
}

/// Confidências `(en, pt)` do detector compartilhado.
///
/// Espelha `language_confidences()` de `nlp.py` (linhas 71-73).
pub fn language_confidences(text: &str) -> (f64, f64) {
    language_confidences_with(LinguaDetector::global(), text)
}

/// Confidências `(en, pt)` com o detector injetado (para testes).
pub fn language_confidences_with<D: LanguageDetector + ?Sized>(
    detector: &D,
    text: &str,
) -> (f64, f64) {
    detector.confidences(text)
}

/// Diferença pt menos en na escala Zipf (positiva puxa para pt).
///
/// Espelha `token_zipf_gap()` de `nlp.py` (linhas 76-79).
pub fn token_zipf_gap(token: &str) -> f64 {
    token_zipf_gap_with(EmbeddedZipf::global(), token)
}

/// Diferença pt menos en com a fonte Zipf injetada (para testes).
pub fn token_zipf_gap_with<S: ZipfSource + ?Sized>(zipf: &S, token: &str) -> f64 {
    let lowered = token.to_lowercase();
    zipf.zipf(&lowered, "pt") - zipf.zipf(&lowered, "en")
}

/// Informa se o rótulo é alfabético ASCII (sem acentos, só letras/dígitos).
///
/// Espelha `is_ascii_alphabetic_label()` de `nlp.py` (linhas 82-90).
pub fn is_ascii_alphabetic_label(cleaned_text: &str) -> bool {
    let trimmed = cleaned_text.trim();
    if trimmed.is_empty() || has_portuguese_accents(trimmed) {
        return false;
    }
    if !trimmed.chars().any(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    ascii_label_pattern().is_match(trimmed)
}

/// Informa se o termo é cognato/ambíguo (sem sinal claro em nenhum idioma).
///
/// Espelha `is_lexically_ambiguous()` de `nlp.py` (linhas 93-104).
pub fn is_lexically_ambiguous(term: &str, options: &NlpOptions) -> bool {
    is_lexically_ambiguous_with(
        term,
        options,
        EmbeddedZipf::global(),
        LinguaDetector::global(),
    )
}

/// Variante com fontes injetadas de [`is_lexically_ambiguous`] (para testes).
pub fn is_lexically_ambiguous_with<S: ZipfSource + ?Sized, D: LanguageDetector + ?Sized>(
    term: &str,
    options: &NlpOptions,
    zipf: &S,
    detector: &D,
) -> bool {
    let lowered = term.trim().to_lowercase();
    if lowered.is_empty() || lowered.chars().count() <= 1 {
        return true;
    }
    if token_zipf_gap_with(zipf, &lowered).abs() < options.zipf_ambiguity_gap {
        return true;
    }
    let (english, portuguese) = language_confidences_with(detector, term.trim());
    (english - portuguese).abs() < options.ambiguity_margin
}

/// Informa se a entrada do glossário manda manter o termo (`manter`/etc.).
///
/// Espelha `is_keep_glossary_term()` de `nlp.py` (linhas 107-109).
pub fn is_keep_glossary_term(entry: &GlossaryEntry) -> bool {
    keep_term_pattern().is_match(&entry.notes)
}

/// Remove termos que o glossário manda manter (ex.: nomes próprios).
///
/// Espelha `strip_keep_terms()` de `nlp.py` (linhas 112-125): o termo só
/// casa com fronteira de palavra, e a busca é sensível a maiúsculas quando
/// o termo já é todo minúsculo (`case-sensitive-if-lowercase`). A fronteira
/// consumida é reposta, então o resultado só difere do Python na largura de
/// sequências de espaços — irrelevante após tokenização/`split`.
pub fn strip_keep_terms(cleaned_text: &str, entries: &[GlossaryEntry]) -> String {
    let mut output = cleaned_text.to_string();
    for entry in entries {
        if entry.term_en.is_empty() || !is_keep_glossary_term(entry) {
            continue;
        }
        let case_sensitive = entry.term_en == entry.term_en.to_lowercase();
        let spans = find_term_spans(&output, &entry.term_en, !case_sensitive);
        if spans.is_empty() {
            continue;
        }
        let mut rebuilt = String::with_capacity(output.len());
        let mut cursor = 0;
        for (start, end) in spans {
            if start < cursor {
                continue;
            }
            rebuilt.push_str(output.get(cursor..start).unwrap_or(""));
            rebuilt.push(' ');
            cursor = end;
        }
        rebuilt.push_str(output.get(cursor..).unwrap_or(""));
        output = rebuilt;
    }
    output
}

/// Termos que o glossário autoriza como empréstimo (ex.: substantivo).
///
/// Espelha `sanctioned_retention_terms()` de `nlp.py` (linhas 128-142):
/// casa `usar "TERMO" como substantivo` nas notas (`IGNORECASE`) e devolve
/// os termos em minúsculas.
pub fn sanctioned_retention_terms(entries: &[GlossaryEntry]) -> HashSet<String> {
    let mut sanctioned = HashSet::new();
    for entry in entries {
        if entry.term_en.is_empty() {
            continue;
        }
        let source = format!(
            r#"(?i)usar\s+"{}"\s+como\s+substantivo"#,
            python_escape(&entry.term_en)
        );
        let Ok(matcher) = Regex::new(&source) else {
            continue;
        };
        if matcher.is_match(&entry.notes) {
            sanctioned.insert(entry.term_en.to_lowercase());
        }
    }
    sanctioned
}

/// Separa tokens em pt, en e ambíguos por evidência léxica dinâmica.
///
/// Espelha `split_token_roles()` de `nlp.py` (linhas 145-164); devolve
/// `(portuguese_tokens, english_tokens, ambiguous_tokens)`.
pub fn split_token_roles(
    cleaned_text: &str,
    options: &NlpOptions,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    split_token_roles_with(cleaned_text, options, EmbeddedZipf::global())
}

/// Variante com fonte Zipf injetada de [`split_token_roles`] (para testes).
pub fn split_token_roles_with<S: ZipfSource + ?Sized>(
    cleaned_text: &str,
    options: &NlpOptions,
    zipf: &S,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut portuguese_tokens: Vec<String> = Vec::new();
    let mut english_tokens: Vec<String> = Vec::new();
    let mut ambiguous_tokens: Vec<String> = Vec::new();
    for token in word_tokens(cleaned_text) {
        if token.chars().count() <= 1 {
            ambiguous_tokens.push(token);
            continue;
        }
        let gap = token_zipf_gap_with(zipf, &token);
        if gap >= options.token_margin {
            portuguese_tokens.push(token);
        } else if gap <= -options.token_margin {
            english_tokens.push(token);
        } else {
            ambiguous_tokens.push(token);
        }
    }
    (portuguese_tokens, english_tokens, ambiguous_tokens)
}

/// Decide se o rótulo tem evidência suficiente de pt-BR.
///
/// Espelha `portuguese_evidence()` de `nlp.py` (linhas 167-213) com o mesmo
/// fluxo: atalho de acento, dois tokens de conteúdo em pt, `len(tokens) <=
/// 2`, cobertura total, conteúdo pt sem termos en, resgate da palavra
/// funcional e, por fim, o limiar do Lingua no texto corrido.
pub fn portuguese_evidence(
    cleaned_text: &str,
    english_terms: &[String],
    entries: &[GlossaryEntry],
    options: &NlpOptions,
) -> bool {
    portuguese_evidence_with(
        cleaned_text,
        english_terms,
        entries,
        options,
        EmbeddedZipf::global(),
        LinguaDetector::global(),
    )
}

/// Variante com fontes injetadas de [`portuguese_evidence`] (para testes).
pub fn portuguese_evidence_with<S: ZipfSource + ?Sized, D: LanguageDetector + ?Sized>(
    cleaned_text: &str,
    english_terms: &[String],
    entries: &[GlossaryEntry],
    options: &NlpOptions,
    zipf: &S,
    detector: &D,
) -> bool {
    if has_portuguese_accents(cleaned_text) {
        return true;
    }
    let scrubbed = strip_keep_terms(cleaned_text, entries);
    let tokens = word_tokens(&scrubbed);
    if tokens.is_empty() {
        return false;
    }
    let (_, english_leaning, _) = split_token_roles_with(&scrubbed, options, zipf);
    let mut content_tokens: Vec<String> = Vec::new();
    for token in &tokens {
        if token.chars().count() > 1
            && token_zipf_gap_with(zipf, token).abs() >= options.zipf_ambiguity_gap
        {
            content_tokens.push(token.clone());
        }
    }
    let mut content_portuguese: Vec<String> = Vec::new();
    for token in &content_tokens {
        if token_zipf_gap_with(zipf, token) >= options.token_margin {
            content_portuguese.push(token.clone());
        }
    }
    if content_portuguese.len() >= 2 {
        return true;
    }
    if !content_portuguese.is_empty() && tokens.len() <= 2 {
        return true;
    }
    if !content_tokens.is_empty() && content_portuguese.len() == content_tokens.len() {
        return true;
    }
    if !content_portuguese.is_empty() && english_terms.is_empty() {
        return true;
    }
    if content_portuguese.len() >= 2
        && english_terms
            .iter()
            .all(|term| is_lexically_ambiguous_with(term, options, zipf, detector))
    {
        return true;
    }
    if tokens.len() > 1 {
        let mut rescued = false;
        for token in &tokens {
            if token.chars().count() > 1
                && zipf.zipf(token, "pt") >= options.function_word_pt_zipf
                && token_zipf_gap_with(zipf, token) > options.function_word_max_gap
            {
                rescued = true;
                break;
            }
        }
        if rescued && english_leaning.is_empty() {
            return true;
        }
        let (_, portuguese_score) = language_confidences_with(detector, &scrubbed);
        if portuguese_score >= options.text_portuguese_threshold {
            return true;
        }
    }
    false
}

/// Termos em inglês restantes ou `sem-evidencia-pt` como sentinela.
///
/// Espelha `english_terms_in_label()` de `nlp.py` (linhas 216-248): sem
/// letras ASCII devolve vazio; evidência de pt zera os candidatos; token
/// único usa o Lingua e o atalho de token curto; rótulo ASCII com 2+
/// palavras vira a sentinela `sem-evidencia-pt`.
pub fn english_terms_in_label(
    cleaned_text: &str,
    entries: &[GlossaryEntry],
    options: &NlpOptions,
) -> Vec<String> {
    english_terms_in_label_with(
        cleaned_text,
        entries,
        options,
        EmbeddedZipf::global(),
        LinguaDetector::global(),
    )
}

/// Variante com fontes injetadas de [`english_terms_in_label`] (para testes).
pub fn english_terms_in_label_with<S: ZipfSource + ?Sized, D: LanguageDetector + ?Sized>(
    cleaned_text: &str,
    entries: &[GlossaryEntry],
    options: &NlpOptions,
    zipf: &S,
    detector: &D,
) -> Vec<String> {
    let scrubbed = strip_keep_terms(cleaned_text, entries);
    if !has_ascii_letter(&scrubbed) {
        return Vec::new();
    }
    let sanctioned = sanctioned_retention_terms(entries);
    let (_, english_tokens, _) = split_token_roles_with(&scrubbed, options, zipf);
    let candidates = dedup_strings(
        english_tokens
            .into_iter()
            .filter(|token| !sanctioned.contains(token))
            .collect(),
    );
    if portuguese_evidence_with(&scrubbed, &candidates, entries, options, zipf, detector) {
        return Vec::new();
    }
    if !candidates.is_empty() {
        return candidates;
    }
    let tokens = word_tokens(&scrubbed);
    if let [single] = tokens.as_slice() {
        if single.chars().count() > 1 {
            let (english_score, portuguese_score) = language_confidences_with(detector, &scrubbed);
            if english_score >= options.single_token_en_clear {
                return tokens;
            }
            let short_token = single.chars().count() <= options.short_token_max_length
                && zipf.zipf(single, "en") >= options.short_token_en_zipf;
            if short_token && portuguese_score < options.short_token_pt_ceiling {
                return tokens;
            }
        }
    }
    if is_ascii_alphabetic_label(&scrubbed) && scrubbed.split_whitespace().count() >= 2 {
        return vec!["sem-evidencia-pt".to_string()];
    }
    Vec::new()
}

/// Localiza termos do glossário com deduplicação pelo maior vão.
///
/// Espelha `find_glossary_terms()` de `nlp.py` (linhas 251-294): remove
/// termos `keep`, ignora sancionados e idênticos EN/PT, pula ambíguos de
/// palavra única sob evidência de pt (com as opções padrão, como no
/// original), casa com fronteira de palavra sem look-around, contém vãos
/// sobrepostos pelo maior e deduplica por `termo_en` exato.
pub fn find_glossary_terms(
    cleaned_text: &str,
    entries: &[GlossaryEntry],
    has_portuguese_evidence: bool,
) -> Vec<GlossaryEntry> {
    find_glossary_terms_with(
        cleaned_text,
        entries,
        has_portuguese_evidence,
        EmbeddedZipf::global(),
        LinguaDetector::global(),
    )
}

/// Variante com fontes injetadas de [`find_glossary_terms`] (para testes).
pub fn find_glossary_terms_with<S: ZipfSource + ?Sized, D: LanguageDetector + ?Sized>(
    cleaned_text: &str,
    entries: &[GlossaryEntry],
    has_portuguese_evidence: bool,
    zipf: &S,
    detector: &D,
) -> Vec<GlossaryEntry> {
    let scrubbed = strip_keep_terms(cleaned_text, entries);
    let sanctioned = sanctioned_retention_terms(entries);
    let mut matches: Vec<(usize, usize, usize)> = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        if entry.term_en.is_empty() || entry.term_pt.is_empty() {
            continue;
        }
        if entry.term_en.to_lowercase() == entry.term_pt.to_lowercase() {
            continue;
        }
        if is_keep_glossary_term(entry) {
            continue;
        }
        if sanctioned.contains(&entry.term_en.to_lowercase()) {
            continue;
        }
        if has_portuguese_evidence
            && !entry.term_en.contains(' ')
            && is_lexically_ambiguous_with(&entry.term_en, &NlpOptions::default(), zipf, detector)
        {
            continue;
        }
        let source = format!("(?i){}", python_escape(&entry.term_en));
        let Ok(matcher) = Regex::new(&source) else {
            continue;
        };
        for found in matcher.find_iter(&scrubbed) {
            let start = found.start();
            let end = found.end();
            let before_ok = scrubbed[..start]
                .chars()
                .next_back()
                .is_none_or(|c| !is_glossary_word_char(c));
            let after_ok = scrubbed[end..]
                .chars()
                .next()
                .is_none_or(|c| !is_glossary_word_char(c));
            if before_ok && after_ok {
                matches.push((start, end, index));
            }
        }
    }
    matches.sort_by(|a, b| a.0.cmp(&b.0).then((b.1 - b.0).cmp(&(a.1 - a.0))));
    let mut accepted: Vec<(usize, usize, usize)> = Vec::new();
    for (start, end, index) in matches {
        if accepted
            .iter()
            .any(|&(kept_start, kept_end, _)| kept_start <= start && end <= kept_end)
        {
            continue;
        }
        accepted.push((start, end, index));
    }
    accepted.sort_by_key(|&(start, _, _)| start);
    let mut found: Vec<GlossaryEntry> = Vec::new();
    let mut seen_terms: HashSet<String> = HashSet::new();
    for (_, _, index) in accepted {
        let Some(entry) = entries.get(index) else {
            continue;
        };
        if seen_terms.insert(entry.term_en.clone()) {
            found.push(entry.clone());
        }
    }
    found
}

/// Heurística rápida para varreduras: texto visível provavelmente em inglês.
///
/// Espelha `is_likely_english_label()` de `nlp.py` (linhas 297-310): ignora
/// textos curtos sem espaço e com acentos; compara a confiança EN com o
/// limiar (explícito ou `english_label_threshold`).
pub fn is_likely_english_label(
    cleaned_text: &str,
    threshold: Option<f64>,
    options: &NlpOptions,
) -> bool {
    is_likely_english_label_with(cleaned_text, threshold, options, LinguaDetector::global())
}

/// Variante com detector injetado de [`is_likely_english_label`] (para testes).
pub fn is_likely_english_label_with<D: LanguageDetector + ?Sized>(
    cleaned_text: &str,
    threshold: Option<f64>,
    options: &NlpOptions,
    detector: &D,
) -> bool {
    let limit = threshold.unwrap_or(options.english_label_threshold);
    let trimmed = cleaned_text.trim();
    if trimmed.chars().count() < 4 || !trimmed.chars().any(|c| c.is_whitespace()) {
        return false;
    }
    if has_portuguese_accents(trimmed) {
        return false;
    }
    let (english_score, _) = language_confidences_with(detector, trimmed);
    english_score >= limit
}

/// Classifica o rótulo em pt, en, misto, ambíguo ou técnico.
///
/// Espelha `classify_label_language()` de `nlp.py` (linhas 313-358) com o
/// fluxo exato: técnico sem letras; pt quando não há termos e há evidência;
/// sentinela `sem-evidencia-pt` vira ambíguo; termos + tokens pt viram
/// misto; só termos viram inglês; o resto é ambíguo.
pub fn classify_label_language(
    cleaned_text: &str,
    entries: &[GlossaryEntry],
    options: &NlpOptions,
) -> LabelVerdict {
    classify_label_language_with(
        cleaned_text,
        entries,
        options,
        EmbeddedZipf::global(),
        LinguaDetector::global(),
    )
}

/// Variante com fontes injetadas de [`classify_label_language`] (para testes).
pub fn classify_label_language_with<S: ZipfSource + ?Sized, D: LanguageDetector + ?Sized>(
    cleaned_text: &str,
    entries: &[GlossaryEntry],
    options: &NlpOptions,
    zipf: &S,
    detector: &D,
) -> LabelVerdict {
    let trimmed = cleaned_text.trim();
    if trimmed.is_empty() || !has_label_letter(trimmed) {
        return LabelVerdict {
            language: LabelLanguage::Technical,
            confidence: 1.0,
            english_terms: Vec::new(),
            portuguese_terms: Vec::new(),
            ambiguous_terms: Vec::new(),
        };
    }
    let scrubbed = strip_keep_terms(trimmed, entries);
    let (portuguese_tokens, _, ambiguous_tokens) = split_token_roles_with(&scrubbed, options, zipf);
    let (english_score, portuguese_score) = if scrubbed.chars().count() > 1 {
        language_confidences_with(detector, &scrubbed)
    } else {
        (0.0, 0.0)
    };
    let terms = english_terms_in_label_with(trimmed, entries, options, zipf, detector);
    if terms.is_empty() && portuguese_evidence_with(trimmed, &[], entries, options, zipf, detector)
    {
        return LabelVerdict {
            language: LabelLanguage::Pt,
            confidence: portuguese_score.max(0.5),
            english_terms: Vec::new(),
            portuguese_terms: portuguese_tokens,
            ambiguous_terms: ambiguous_tokens,
        };
    }
    if terms.len() == 1 && terms.first().is_some_and(|term| term == "sem-evidencia-pt") {
        return LabelVerdict {
            language: LabelLanguage::Ambiguous,
            confidence: english_score,
            english_terms: Vec::new(),
            portuguese_terms: portuguese_tokens,
            ambiguous_terms: ambiguous_tokens,
        };
    }
    if !terms.is_empty() && !portuguese_tokens.is_empty() {
        return LabelVerdict {
            language: LabelLanguage::Mixed,
            confidence: english_score.max(portuguese_score),
            english_terms: terms,
            portuguese_terms: portuguese_tokens,
            ambiguous_terms: ambiguous_tokens,
        };
    }
    if !terms.is_empty() {
        return LabelVerdict {
            language: LabelLanguage::En,
            confidence: english_score.max(0.5),
            english_terms: terms,
            portuguese_terms: Vec::new(),
            ambiguous_terms: ambiguous_tokens,
        };
    }
    LabelVerdict {
        language: LabelLanguage::Ambiguous,
        confidence: english_score,
        english_terms: Vec::new(),
        portuguese_terms: portuguese_tokens,
        ambiguous_terms: ambiguous_tokens,
    }
}

/// Escapa literais como `re.escape` do Python 3.11 (só metacaracteres ASCII).
fn python_escape(text: &str) -> String {
    const SPECIAL: &str = "()[]{}?*+-|^$\\.&~# \t\n\r\x0B\x0C";
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        if SPECIAL.contains(c) {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}

/// Caractere de palavra do glossário (`[A-Za-zÀ-ÿ0-9_]`, como no Python).
fn is_glossary_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || ('À'..='ÿ').contains(&c)
}

/// Vãos do termo com fronteira de palavra (emula look-around do Python).
///
/// O `regex` não tem look-around: localiza o literal (com `(?i)` quando
/// pedido) e valida manualmente os caracteres vizinhos — mesma semântica.
fn find_term_spans(haystack: &str, term: &str, case_insensitive: bool) -> Vec<(usize, usize)> {
    if term.is_empty() {
        return Vec::new();
    }
    let source = if case_insensitive {
        format!("(?i){}", python_escape(term))
    } else {
        python_escape(term)
    };
    let Ok(matcher) = Regex::new(&source) else {
        return Vec::new();
    };
    let mut spans = Vec::new();
    for found in matcher.find_iter(haystack) {
        let start = found.start();
        let end = found.end();
        let before_ok = haystack[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !is_glossary_word_char(c));
        let after_ok = haystack[end..]
            .chars()
            .next()
            .is_none_or(|c| !is_glossary_word_char(c));
        if before_ok && after_ok {
            spans.push((start, end));
        }
    }
    spans
}

/// Informa se há letra ASCII (`[A-Za-z]`, como em `nlp.py`).
fn has_ascii_letter(text: &str) -> bool {
    text.chars().any(|c| c.is_ascii_alphabetic())
}

/// Informa se há letra de rótulo (`[A-Za-zÀ-ÿ]`, como em `nlp.py`).
fn has_label_letter(text: &str) -> bool {
    text.chars()
        .any(|c| c.is_ascii_alphabetic() || ('À'..='ÿ').contains(&c))
}

/// Remove duplicadas preservando a ordem (como `unique_strings` do `common.py`).
fn dedup_strings(values: Vec<String>) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut unique = Vec::new();
    for value in values {
        if seen.insert(value.clone()) {
            unique.push(value);
        }
    }
    unique
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> NlpOptions {
        NlpOptions::default()
    }

    fn glossary(rows: &[(&str, &str, &str)]) -> Vec<GlossaryEntry> {
        rows.iter()
            .map(|(en, pt, notes)| GlossaryEntry::new(en, pt, notes))
            .collect()
    }

    #[test]
    fn english_label_flags_untranslated_word() {
        assert_eq!(
            english_terms_in_label("Remove", &[], &options()),
            vec!["remove".to_string()]
        );
    }

    #[test]
    fn short_portuguese_tokens_do_not_flag() {
        assert_eq!(
            english_terms_in_label("Total", &[], &options()),
            Vec::<String>::new()
        );
        assert_eq!(
            english_terms_in_label("Ola", &[], &options()),
            Vec::<String>::new()
        );
        assert_eq!(
            english_terms_in_label("Cola", &[], &options()),
            Vec::<String>::new()
        );
    }

    #[test]
    fn single_english_token_flags() {
        assert_eq!(
            english_terms_in_label("Continue", &[], &options()),
            vec!["continue".to_string()]
        );
        assert_eq!(
            english_terms_in_label("General", &[], &options()),
            vec!["general".to_string()]
        );
        assert_eq!(
            english_terms_in_label("No", &[], &options()),
            vec!["no".to_string()]
        );
    }

    #[test]
    fn portuguese_function_word_gives_evidence() {
        assert_eq!(
            english_terms_in_label("URL do", &[], &options()),
            Vec::<String>::new()
        );
        assert_eq!(
            english_terms_in_label("EULA do Minecraft", &[], &options()),
            Vec::<String>::new()
        );
        assert_eq!(
            english_terms_in_label("Ao pressionar", &[], &options()),
            Vec::<String>::new()
        );
    }

    #[test]
    fn keep_terms_are_stripped_before_detection() {
        let entries = glossary(&[("Tickets", "Tickets", "manter")]);
        assert_eq!(
            english_terms_in_label("Tickets de suporte", &entries, &options()),
            Vec::<String>::new()
        );
    }

    #[test]
    fn keep_strip_is_case_sensitive_when_term_is_lowercase() {
        let entries = glossary(&[("tickets", "tickets", "manter")]);
        assert_eq!(
            strip_keep_terms("Tickets de suporte", &entries),
            "Tickets de suporte".to_string()
        );
        assert!(!strip_keep_terms("tickets de suporte", &entries).contains("tickets"));
    }

    #[test]
    fn sanctioned_retention_does_not_flag_loanword() {
        let entries = glossary(&[(
            "Upgrade",
            "Fazer upgrade",
            "verbo; usar \"Upgrade\" como substantivo se necessario",
        )]);
        assert_eq!(
            english_terms_in_label("Fazer upgrade agora", &entries, &options()),
            Vec::<String>::new()
        );
        assert_eq!(
            english_terms_in_label("Upgrade Options", &entries, &options()),
            vec!["options".to_string()]
        );
    }

    #[test]
    fn glossary_match_wins_over_generic_detection() {
        let entries = glossary(&[("Server", "Servidor", "")]);
        let found = find_glossary_terms("Server", &entries, false);
        assert_eq!(
            found.iter().map(|e| e.term_en.as_str()).collect::<Vec<_>>(),
            vec!["Server"]
        );
    }

    #[test]
    fn ambiguous_word_skipped_inside_portuguese() {
        let entries = glossary(&[("No", "Não", "")]);
        assert!(
            find_glossary_terms("A senha deve ter no minimo 8 caracteres.", &entries, true)
                .is_empty()
        );
        let found = find_glossary_terms("No", &entries, false);
        assert_eq!(
            found.iter().map(|e| e.term_en.as_str()).collect::<Vec<_>>(),
            vec!["No"]
        );
    }

    #[test]
    fn detector_covers_project_languages() {
        assert_eq!(detector_languages(), ("pt", "en"));
        assert_eq!(SUPPORTED_LANGUAGES.principal, "pt");
        assert_eq!(SUPPORTED_LANGUAGES.secundario, "en");
    }

    #[test]
    fn portuguese_evidence_uses_accents_and_function_words() {
        assert!(portuguese_evidence(
            "Salvar alterações",
            &[],
            &[],
            &options()
        ));
        assert!(portuguese_evidence("URL do", &[], &[], &options()));
        assert!(!portuguese_evidence("No", &[], &[], &options()));
        assert!(!portuguese_evidence("General", &[], &[], &options()));
    }

    #[test]
    fn classifier_reports_expected_languages() {
        assert_eq!(
            classify_label_language("Salvar alterações", &[], &options()).language,
            LabelLanguage::Pt
        );
        assert_eq!(
            classify_label_language("Save changes", &[], &options()).language,
            LabelLanguage::En
        );
        assert!(matches!(
            classify_label_language("Upgrade Options", &[], &options()).language,
            LabelLanguage::En | LabelLanguage::Mixed
        ));
        let technical = classify_label_language("12345", &[], &options());
        assert_eq!(technical.language, LabelLanguage::Technical);
        assert_eq!(technical.confidence, 1.0);
    }

    #[test]
    fn classifier_flags_cognate_gap_without_dictionary() {
        assert!(token_zipf_gap("total").abs() < 0.5);
        assert_eq!(
            english_terms_in_label("Total", &[], &options()),
            Vec::<String>::new()
        );
    }

    #[test]
    fn likely_english_gate_for_excluded_scan() {
        assert!(is_likely_english_label("Save changes", None, &options()));
        assert!(!is_likely_english_label(
            "Salvar alterações",
            None,
            &options()
        ));
        assert!(!is_likely_english_label("OK", None, &options()));
    }

    #[test]
    fn thresholds_match_python_defaults() {
        let current = options();
        assert_eq!(current.token_margin, 1.0);
        assert_eq!(current.zipf_ambiguity_gap, 1.0);
        assert_eq!(current.ambiguity_margin, 0.4);
        assert_eq!(current.text_portuguese_threshold, 0.65);
        assert_eq!(current.english_label_threshold, 0.6);
        assert_eq!(current.single_token_en_clear, 0.6);
        assert_eq!(current.short_token_max_length, 3);
        assert_eq!(current.short_token_en_zipf, 5.0);
        assert_eq!(current.short_token_pt_ceiling, 0.7);
        assert_eq!(current.function_word_pt_zipf, 6.0);
        assert_eq!(current.function_word_max_gap, -1.0);
    }

    #[test]
    fn function_word_rescue_highlights() {
        let zipf = EmbeddedZipf::global();
        assert!((zipf.zipf("do", "pt") - 7.22).abs() < 1e-6);
        assert!((zipf.zipf("do", "en") - 6.35).abs() < 1e-6);
        assert!((zipf.zipf("total", "pt") - 5.24).abs() < 1e-6);
        assert!((zipf.zipf("total", "en") - 5.21).abs() < 1e-6);
        assert_eq!(zipf.zipf("desconhecido-xyz", "pt"), 0.0);
        assert_eq!(zipf.zipf("desconhecido-xyz", "en"), 0.0);
        assert!(portuguese_evidence("URL do", &[], &[], &options()));
        assert!(portuguese_evidence(
            "EULA do Minecraft",
            &[],
            &[],
            &options()
        ));
    }

    #[test]
    fn stub_detector_covers_sentinel_and_mixed_branches() {
        let zipf = EmbeddedZipf::global();
        let stub = StubDetector::new(0.9, 0.0);
        assert_eq!(
            english_terms_in_label_with("Xyz Abc", &[], &options(), zipf, &stub),
            vec!["sem-evidencia-pt".to_string()]
        );
        let sentinel = classify_label_language_with("Xyz Abc", &[], &options(), zipf, &stub);
        assert_eq!(sentinel.language, LabelLanguage::Ambiguous);
        assert_eq!(sentinel.confidence, 0.9);
        let mixed = classify_label_language_with("Save xyz de", &[], &options(), zipf, &stub);
        assert_eq!(mixed.language, LabelLanguage::Mixed);
        assert_eq!(mixed.english_terms, vec!["save".to_string()]);
        assert_eq!(mixed.portuguese_terms, vec!["de".to_string()]);
        let english = classify_label_language_with("Save changes", &[], &options(), zipf, &stub);
        assert_eq!(english.language, LabelLanguage::En);
    }

    #[test]
    fn language_codes_match_python_values() {
        assert_eq!(LabelLanguage::Pt.as_str(), "pt");
        assert_eq!(LabelLanguage::En.as_str(), "en");
        assert_eq!(LabelLanguage::Mixed.as_str(), "mixed");
        assert_eq!(LabelLanguage::Ambiguous.as_str(), "ambiguous");
        assert_eq!(LabelLanguage::Technical.as_str(), "technical");
    }
}
