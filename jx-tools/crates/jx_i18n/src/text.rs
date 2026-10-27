//! Normalizacao de texto, placeholders, URLs e multiconjuntos.
//!
//! Port fiel de `scripts/i18n/i18n_toolkit/text.py`. As tres regras com
//! *lookbehind* do Python (`(?<![...])`, sem suporte no crate `regex`)
//! sao implementadas como busca + verificacao do caractere anterior em Rust,
//! com a mesma semantica. A faixa `À-ÿ` inclui `×` e `÷` como no original.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use regex::Regex;

/// Compila padrao estatico.
///
/// O `expect` aqui e proposital e localizado: os padroes sao constantes
/// revisadas, e qualquer regressao quebra os testes de unidade na hora.
#[allow(clippy::expect_used)]
fn pattern(raw: &str) -> Regex {
    // So padroes constantes chegam aqui; regressao quebra os testes.
    // So padroes constantes chegam aqui; regressao quebra os testes.
    Regex::new(raw).expect("padrao regex estatico invalido")
}

/// Compila uma vez e reusa (Mutex + clone; sem panico em nenhum caminho).
pub(crate) fn cached_regex(raw: &str) -> Regex {
    static CELL: OnceLock<Mutex<HashMap<String, Regex>>> = OnceLock::new();
    let map = CELL.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = map.lock() {
        if let Some(compiled) = guard.get(raw) {
            return compiled.clone();
        }
    }
    let compiled = pattern(raw);
    if let Ok(mut guard) = map.lock() {
        guard.insert(raw.to_string(), compiled.clone());
    }
    compiled
}

fn url_pattern() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| pattern(r"(?i)^(?:(?:https?|ftps?|sftp|file)://|(?:mailto|tel):|//)"))
}

fn file_path_pattern() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| {
        pattern(r"(?i)^[A-Za-z0-9_./-]+\.(?:tsx?|jsx?|php|css|scss|json|svg|png|jpe?g|gif|webp|woff2?|ttf|eot)$")
    })
}

fn email_pattern() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| pattern(r"^[^\s@]+@[^\s@]+\.[^\s@]+$"))
}

fn domain_pattern() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| pattern(r"^(?:[A-Za-z0-9-]+\.)+[A-Za-z]{2,}(?::\d+)?(?:/.*)?$"))
}

fn embedded_url_pattern() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| pattern(r#"(?i)https?://[^\s"'`<>]+"#))
}

fn embedded_contact_pattern() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| pattern(r#"(?i)\b(?:mailto|tel):[^\s"'`<>]+"#))
}

fn portuguese_accents_pattern() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| pattern(r"(?i)[ãõçáéíóúêôâà]"))
}

fn word_token_pattern() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| pattern(r"[A-Za-zÀ-ÿ]+"))
}

fn placeholder_dollar_brace() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| pattern(r"\$\{([^}]*)\}"))
}

fn placeholder_double_brace() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| pattern(r"\{\{([^}]*)\}\}"))
}

fn placeholder_dollar_var() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| pattern(r"\{\$([A-Za-z_][A-Za-z0-9_.]*)\}"))
}

/// `{nome}` sem `${`, palavra/`{` antes e sem `}}` depois.
///
/// Lookbehind e lookahead verificados a mao (crate sem look-around).
fn placeholder_brace() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| pattern(r"\{([A-Za-z_][A-Za-z0-9_.-]*)\}"))
}

/// `$var` sem `\`, palavra, `$` ou `{` antes.
fn placeholder_dollar() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| pattern(r"\$([A-Za-z_][A-Za-z0-9_]*)"))
}

/// `:var` sem `:` ou palavra antes.
fn placeholder_colon() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| pattern(r":([A-Za-z_][A-Za-z0-9_]*)"))
}

fn placeholder_printf() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| pattern(r"%(?:\d+\$)?[sdif]"))
}

/// Normaliza quebras de linha para `\n`.
pub fn normalize_newlines(value: &str) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n")
}

/// Normaliza literais dentro de placeholders (`'x'` -> `''`).
pub fn normalize_placeholder_literals(placeholder: &str) -> String {
    let single = cached_regex(r"'[^']*'");
    let double = cached_regex(r#""[^"]*""#);
    let step = single.replace_all(placeholder, "''");
    double.replace_all(&step, r#""""#).into_owned()
}

/// Caractere anterior e algum dos proibidos (emulacao de `(?<![...])`).
fn preceded_by(text: &str, start: usize, forbidden: &[char]) -> bool {
    text[..start]
        .chars()
        .next_back()
        .is_some_and(|c| forbidden.contains(&c))
}

fn is_word_char(c: char) -> bool {
    // Proximo de `\w` do Python (letras Unicode + digitos + `_`).
    c.is_alphanumeric() || c == '_'
}

/// Extrai placeholders na ordem de aparicao.
pub fn extract_placeholders(text: &str) -> Vec<String> {
    let mut found: Vec<(usize, String)> = Vec::new();
    for capture in placeholder_dollar_brace().captures_iter(text) {
        let Some(whole) = capture.get(0) else {
            continue;
        };
        let inner = capture.get(1).map_or("", |m| m.as_str());
        let inner = inner.split_whitespace().collect::<Vec<_>>().join(" ");
        found.push((whole.start(), format!("${{{inner}}}")));
    }
    for capture in placeholder_double_brace().captures_iter(text) {
        let Some(whole) = capture.get(0) else {
            continue;
        };
        let inner = capture.get(1).map_or("", |m| m.as_str());
        let inner = inner.split_whitespace().collect::<Vec<_>>().join(" ");
        if inner.is_empty() || inner.starts_with("--") {
            continue;
        }
        found.push((whole.start(), format!("{{{{{inner}}}}}")));
    }
    for capture in placeholder_dollar_var().captures_iter(text) {
        let Some(whole) = capture.get(0) else {
            continue;
        };
        found.push((whole.start(), whole.as_str().to_string()));
    }
    for capture in placeholder_brace().captures_iter(text) {
        let Some(whole) = capture.get(0) else {
            continue;
        };
        if preceded_by(text, whole.start(), &['$', '{'])
            || text[..whole.start()]
                .chars()
                .next_back()
                .is_some_and(is_word_char)
            || text[whole.end()..].starts_with('}')
        {
            continue;
        }
        found.push((whole.start(), whole.as_str().to_string()));
    }
    for capture in placeholder_dollar().captures_iter(text) {
        let Some(whole) = capture.get(0) else {
            continue;
        };
        if preceded_by(text, whole.start(), &['\\', '$', '{'])
            || text[..whole.start()]
                .chars()
                .next_back()
                .is_some_and(is_word_char)
        {
            continue;
        }
        found.push((whole.start(), whole.as_str().to_string()));
    }
    for capture in placeholder_colon().captures_iter(text) {
        let Some(whole) = capture.get(0) else {
            continue;
        };
        if preceded_by(text, whole.start(), &[':'])
            || text[..whole.start()]
                .chars()
                .next_back()
                .is_some_and(is_word_char)
        {
            continue;
        }
        found.push((whole.start(), whole.as_str().to_string()));
    }
    for capture in placeholder_printf().find_iter(text) {
        found.push((capture.start(), capture.as_str().to_string()));
    }
    found.sort_by_key(|(start, _)| *start);
    found.into_iter().map(|(_, value)| value).collect()
}

/// Extrai URLs embutidas, sem pontuacao final.
pub fn extract_urls(text: &str) -> Vec<String> {
    embedded_url_pattern()
        .find_iter(text)
        .map(|m| {
            m.as_str()
                .trim_end_matches([',', '.', ';', ':', '!', '?', ')', ']'])
                .to_string()
        })
        .collect()
}

/// Remove URLs e contatos embutidos.
pub fn strip_urls(text: &str) -> String {
    let step = embedded_url_pattern().replace_all(text, " ");
    embedded_contact_pattern()
        .replace_all(&step, " ")
        .into_owned()
}

/// Elementos de `left` que sobram apos remover os de `right` (multiconjunto).
pub fn multiset_difference(left: &[String], right: &[String]) -> Vec<String> {
    let mut remaining: HashMap<&str, usize> = HashMap::new();
    for value in right {
        *remaining.entry(value.as_str()).or_insert(0) += 1;
    }
    let mut difference = Vec::new();
    for value in left {
        match remaining.get_mut(value.as_str()) {
            Some(count) if *count > 0 => *count -= 1,
            _ => difference.push(value.clone()),
        }
    }
    difference
}

/// Informa se duas listas tem o mesmo multiconjunto ordenado.
pub fn same_sorted_multiset(left: &[String], right: &[String]) -> bool {
    let mut left_sorted = left.to_vec();
    let mut right_sorted = right.to_vec();
    left_sorted.sort();
    right_sorted.sort();
    left_sorted == right_sorted
}

/// Tokens de palavra em minusculas (faixa `À-ÿ` como no original).
pub fn word_tokens(text: &str) -> Vec<String> {
    word_token_pattern()
        .find_iter(text)
        .map(|m| m.as_str().to_lowercase())
        .collect()
}

/// Substitui ocorrencias do padrao por espaco, salvo quando o caractere
/// anterior esta em `forbidden` ou e palavra (emulacao de `(?<![...])`).
fn replace_unless_preceded(haystack: &str, pattern: &'static str, forbidden: &[char]) -> String {
    let matcher = cached_regex(pattern);
    let mut out = String::with_capacity(haystack.len());
    let mut last = 0;
    for m in matcher.find_iter(haystack) {
        let blocked = preceded_by(haystack, m.start(), forbidden)
            || haystack[..m.start()]
                .chars()
                .next_back()
                .is_some_and(is_word_char);
        if !blocked {
            out.push_str(&haystack[last..m.start()]);
            out.push(' ');
            last = m.end();
        }
    }
    out.push_str(&haystack[last..]);
    out
}

/// Remove sintaxes de placeholder e colapsa espacos.
pub fn clean_label_text(text: &str) -> String {
    let step = cached_regex(r"\$\{[^}]*\}").replace_all(text, " ");
    let step = cached_regex(r"\{\$[^}]*\}").replace_all(&step, " ");
    let step = cached_regex(r"\{\{[^}]*\}\}").replace_all(&step, " ");
    let step = cached_regex(r"\{[A-Za-z_][A-Za-z0-9_.-]*\}").replace_all(&step, " ");
    // Sem suporte a lookbehind no crate `regex`: `:var` com `:` ou palavra
    // antes e preservado, como em `(?<![:\w])`.
    let step = replace_unless_preceded(&step, r":[A-Za-z_][A-Za-z0-9_]*", &[':']);
    let step = cached_regex(r"%(?:\d+\$)?[sdif]").replace_all(&step, " ");
    // Sem suporte a lookbehind no crate `regex`: checagem manual do anterior.
    let dollar = replace_unless_preceded(&step, r"\$[A-Za-z_][A-Za-z0-9_]*", &['\\', '$', '{']);
    cached_regex(r"\s+")
        .replace_all(&dollar, " ")
        .trim()
        .to_string()
}

/// Remove entidades, flags `--opt` e restos de placeholder.
pub fn strip_non_label_tokens(text: &str) -> String {
    let step = cached_regex(r"(?i)&[a-z]+;").replace_all(text, " ");
    let step = cached_regex(r"(?i)--[a-z0-9-]+").replace_all(&step, " ");
    let step = cached_regex(r"\$\{[^}]*\}").replace_all(&step, " ");
    let step = cached_regex(r"\{[^}]*\}").replace_all(&step, " ");
    let step = cached_regex(r":[A-Za-z_][\w.]*").replace_all(&step, " ");
    cached_regex(r"\s+")
        .replace_all(&step, " ")
        .trim()
        .to_string()
}

/// Informa se o texto parece rotulo humano traduzivel.
pub fn is_human_text(text: &str) -> bool {
    let trimmed = text.trim();
    if !trimmed
        .chars()
        .any(|c| c.is_ascii_alphabetic() || ('À'..='ÿ').contains(&c))
    {
        return false;
    }
    if (url_pattern().is_match(trimmed) || trimmed.contains("://"))
        && !strip_urls(trimmed)
            .chars()
            .any(|c| c.is_ascii_alphabetic() || ('À'..='ÿ').contains(&c))
    {
        return false;
    }
    if file_path_pattern().is_match(trimmed) {
        return false;
    }
    if cached_regex(r"^[a-z0-9_.:/#-]+$").is_match(trimmed) {
        return false;
    }
    if email_pattern().is_match(trimmed) || domain_pattern().is_match(trimmed) {
        return false;
    }
    if trimmed.chars().any(|c| c.is_whitespace()) {
        return true;
    }
    !extract_placeholders(trimmed).is_empty()
}

/// Informa se o texto contem acento portugues.
pub fn has_portuguese_accents(text: &str) -> bool {
    portuguese_accents_pattern().is_match(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_all_placeholder_shapes_in_order() {
        assert_eq!(
            extract_placeholders("ola ${nome}, {{ titulo }} e :coisa"),
            vec!["${nome}", "{{titulo}}", ":coisa"]
        );
        assert_eq!(
            extract_placeholders("{$var} {chave} $solto %s %2$d"),
            vec!["{$var}", "{chave}", "$solto", "%s", "%2$d"]
        );
        // Comentario Blade {{----}} e ignorado; espacos internos colapsam.
        assert_eq!(
            extract_placeholders("{{  a   b  }} {{-- x --}}"),
            vec!["{{a b}}"]
        );
        assert_eq!(extract_placeholders("sem nada aqui"), Vec::<String>::new());
    }

    #[test]
    fn lookbehind_rules_match_python() {
        // `{x}` apos `$` e parte de `${x}`: so um placeholder, no offset do `$`.
        assert_eq!(extract_placeholders("${x}"), vec!["${x}"]);
        // Palavra colada impede `{chave}` e `:v` e `$v`.
        assert_eq!(extract_placeholders("a{chave}"), Vec::<String>::new());
        assert_eq!(extract_placeholders("a:b"), Vec::<String>::new());
        assert_eq!(extract_placeholders("a$b"), Vec::<String>::new());
        // `{{x}}` nao gera `{x}` interno (lookahead `}}`).
        assert_eq!(extract_placeholders("{{x}}"), vec!["{{x}}"]);
        // Chave simples isolada vale.
        assert_eq!(extract_placeholders("ver {itens} hoje"), vec!["{itens}"]);
    }

    #[test]
    fn normalizes_placeholder_literals() {
        assert_eq!(normalize_placeholder_literals("{'a'}"), "{''}");
        assert_eq!(normalize_placeholder_literals("{\"a\"}"), "{\"\"}");
    }

    #[test]
    fn extracts_and_strips_urls() {
        assert_eq!(
            extract_urls("veja https://exemplo.test/a, ok"),
            vec!["https://exemplo.test/a"]
        );
        assert!(strip_urls("veja https://exemplo.test/a!").contains("veja"));
        assert!(!strip_urls("veja https://exemplo.test/a!").contains("https"));
    }

    #[test]
    fn multisets_compare_with_counts() {
        let left = vec!["a".to_string(), "a".to_string(), "b".to_string()];
        let right = vec!["a".to_string(), "c".to_string()];
        assert_eq!(
            multiset_difference(&left, &right),
            vec!["a".to_string(), "b".to_string()]
        );
        assert!(!same_sorted_multiset(&left, &right));
        assert!(same_sorted_multiset(
            &["b".to_string(), "a".to_string()],
            &["a".to_string(), "b".to_string()]
        ));
    }

    #[test]
    fn cleans_labels_like_python() {
        assert_eq!(clean_label_text("ola ${nome} e :coisa!"), "ola e !");
        assert_eq!(clean_label_text("itens: {n} de %d"), "itens: de");
        assert_eq!(strip_non_label_tokens("a &amp; b --xx c"), "a b c");
        assert_eq!(word_tokens("Olá Mundo 123"), vec!["olá", "mundo"]);
        assert!(has_portuguese_accents("alterações"));
        assert!(!has_portuguese_accents("changes"));
    }

    #[test]
    fn human_text_gates_match_python() {
        assert!(!is_human_text(""));
        assert!(!is_human_text("12345"));
        assert!(!is_human_text("https://exemplo.test"));
        assert!(!is_human_text("contato@exemplo.test"));
        assert!(!is_human_text("exemplo.test/painel"));
        assert!(!is_human_text("App.tsx"));
        assert!(!is_human_text("api/v1/rotas"));
        assert!(is_human_text("Salvar alterações"));
        assert!(is_human_text("ver {itens} hoje"));
        assert!(is_human_text("{itens}"));
    }
}
