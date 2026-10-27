//! Parser de arquivos `resources/lang/*.php` e comparacao de chaves.
//!
//! Port fiel de `scripts/i18n/i18n_toolkit/php_lang.py`: lexer manual,
//! `return [...]` ou primeiro array, chaves com pontos escapados e
//! comparacao com contagem de duplicatas.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use regex::Regex;

/// Token lexico do fonte PHP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhpToken {
    /// `string`, `symbol`, `ident` ou `other`.
    pub kind: PhpKind,
    /// Texto (conteudo sem aspas para strings).
    pub value: String,
    /// Linha inicial (1-based).
    pub line: u32,
    /// Aspa de origem (`'`, `"` ou `heredoc`).
    pub quote: Option<String>,
}

/// Classe do token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhpKind {
    /// Literal de string.
    Str,
    /// Simbolo (`[](),;`, `=>`, `.`).
    Symbol,
    /// Identificador.
    Ident,
    /// Qualquer outro caractere.
    Other,
}

/// Entrada de lang: caminho de segmentos, chave exibida e valor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LangEntry {
    /// Segmentos aninhados.
    pub segments: Vec<String>,
    /// Chave com pontos escapados (`a\.b`).
    pub key: String,
    /// Linha da chave.
    pub line: u32,
    /// Valor literal ou `None`.
    pub value: Option<String>,
    /// Linha do valor.
    pub value_line: u32,
}

/// Comparacao entre lang EN e PT.
#[derive(Debug, Clone, Default)]
pub struct LangComparison {
    /// Entradas do ingles.
    pub english_entries: Vec<LangEntry>,
    /// Entradas do portugues.
    pub portuguese_entries: Vec<LangEntry>,
    /// Chaves sem traducao (inclui duplicatas EN).
    pub missing: Vec<LangEntry>,
    /// Chaves sem origem (inclui duplicatas PT).
    pub extra: Vec<LangEntry>,
    /// Pares por chave.
    pub common: Vec<(LangEntry, LangEntry)>,
    /// Chaves comparadas.
    pub keys_compared: usize,
}

#[allow(clippy::expect_used)]
fn pattern(raw: &str) -> Regex {
    // So padroes constantes chegam aqui; regressao quebra os testes.
    Regex::new(raw).expect("padrao regex estatico invalido")
}

/// Compila uma vez e reusa (Mutex + clone; sem panico em nenhum caminho).
fn cached(raw: &'static str) -> Regex {
    static CELL: OnceLock<Mutex<HashMap<&'static str, Regex>>> = OnceLock::new();
    let map = CELL.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = map.lock() {
        if let Some(compiled) = guard.get(raw) {
            return compiled.clone();
        }
    }
    let compiled = pattern(raw);
    if let Ok(mut guard) = map.lock() {
        guard.insert(raw, compiled.clone());
    }
    compiled
}

fn heredoc_open_pattern() -> Regex {
    cached(
        r#"<<<[ \t]*(?:'([A-Za-z_][A-Za-z0-9_]*)'|"([A-Za-z_][A-Za-z0-9_]*)"|([A-Za-z_][A-Za-z0-9_]*))[ \t]*\n"#,
    )
}

fn ident_pattern() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| pattern(r"[A-Za-z_][A-Za-z0-9_]*"))
}

fn heredoc_close_pattern(identifier: &str) -> Option<Regex> {
    Regex::new(&format!(
        r"(?m)^[ \t]*{}[ \t]*;?[ \t]*(?://.*|#.*)?$",
        regex::escape(identifier)
    ))
    .ok()
}

/// Divide o fonte PHP em tokens simples.
pub fn tokenize_php(text: &str) -> Vec<PhpToken> {
    let source = text.replace("\r\n", "\n").replace('\r', "\n");
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    let mut line: u32 = 1;
    let advance = |index: &mut usize, line: &mut u32, count: usize| {
        let end = (*index + count).min(source.len());
        while *index < end {
            if source.as_bytes()[*index] == b'\n' {
                *line += 1;
            }
            *index += 1;
        }
    };
    while index < source.len() {
        let Some(char) = source[index..].chars().next() else {
            break;
        };
        if char == '\n' {
            line += 1;
            index += 1;
            continue;
        }
        if char == ' ' || char == '\t' || char == '\r' {
            index += 1;
            continue;
        }
        if source[index..].starts_with("<?php") {
            index += 5;
            continue;
        }
        if source[index..].starts_with("<?=") {
            index += 3;
            continue;
        }
        if source[index..].starts_with("?>") {
            index += 2;
            continue;
        }
        if source[index..].starts_with("//") {
            while index < source.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if char == '#' && (index + 1 >= source.len() || bytes[index + 1] != b'[') {
            while index < source.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if source[index..].starts_with("/*") {
            let stop = source[index + 2..]
                .find("*/")
                .map_or(source.len(), |end| index + 2 + end + 2);
            line += source[index..stop].bytes().filter(|b| *b == b'\n').count() as u32;
            index = stop;
            continue;
        }
        if source[index..].starts_with("<<<") {
            let rest = &source[index..];
            if let Some(capture) = heredoc_open_pattern().captures(rest) {
                let Some(whole) = capture.get(0) else {
                    continue;
                };
                let identifier = capture
                    .get(1)
                    .or_else(|| capture.get(2))
                    .or_else(|| capture.get(3))
                    .map_or("", |m| m.as_str())
                    .to_string();
                let start_line = line;
                advance(&mut index, &mut line, whole.as_str().len());
                let content_start = index;
                let Some(closer) = heredoc_close_pattern(&identifier) else {
                    continue;
                };
                let content_end = closer
                    .find(&source[index..])
                    .map_or(source.len(), |m| index + m.start());
                let value = source[content_start..content_end].to_string();
                let delta = content_end - index;
                advance(&mut index, &mut line, delta);
                if source[index..].starts_with('\n') {
                    advance(&mut index, &mut line, 1);
                }
                tokens.push(PhpToken {
                    kind: PhpKind::Str,
                    value,
                    line: start_line,
                    quote: Some("heredoc".to_string()),
                });
                continue;
            }
        }
        if char == '\'' || char == '"' {
            let quote = char;
            let start_line = line;
            advance(&mut index, &mut line, 1);
            let content_start = index;
            while index < source.len() {
                let Some(current) = source[index..].chars().next() else {
                    break;
                };
                if current == '\\' {
                    let step = source[index + 1..]
                        .chars()
                        .next()
                        .map_or(1, |next_ch| 1 + next_ch.len_utf8());
                    advance(&mut index, &mut line, step);
                    continue;
                }
                if current == quote {
                    break;
                }
                if current == '\n' {
                    line += 1;
                }
                index += current.len_utf8();
            }
            let value = source[content_start..index].to_string();
            if index < source.len() && source[index..].starts_with(quote) {
                advance(&mut index, &mut line, quote.len_utf8());
            }
            tokens.push(PhpToken {
                kind: PhpKind::Str,
                value,
                line: start_line,
                quote: Some(quote.to_string()),
            });
            continue;
        }
        if "[](),;.".contains(char) {
            tokens.push(PhpToken {
                kind: PhpKind::Symbol,
                value: char.to_string(),
                line,
                quote: None,
            });
            index += 1;
            continue;
        }
        if char == '=' && index + 1 < source.len() && bytes[index + 1] == b'>' {
            tokens.push(PhpToken {
                kind: PhpKind::Symbol,
                value: "=>".to_string(),
                line,
                quote: None,
            });
            index += 2;
            continue;
        }
        if let Some(word) = ident_pattern().find(&source[index..]) {
            if word.start() == 0 {
                tokens.push(PhpToken {
                    kind: PhpKind::Ident,
                    value: word.as_str().to_string(),
                    line,
                    quote: None,
                });
                index += word.as_str().len();
                continue;
            }
        }
        tokens.push(PhpToken {
            kind: PhpKind::Other,
            value: char.to_string(),
            line,
            quote: None,
        });
        index += char.len_utf8();
    }
    tokens
}

fn token_at(tokens: &[PhpToken], index: usize) -> Option<&PhpToken> {
    tokens.get(index)
}

/// Informa se ha inicio de array em `index` (`[` ou `array(`).
pub fn is_array_start(tokens: &[PhpToken], index: usize) -> bool {
    let Some(token) = token_at(tokens, index) else {
        return false;
    };
    if token.kind == PhpKind::Symbol && token.value == "[" {
        return true;
    }
    let following = token_at(tokens, index + 1);
    token.kind == PhpKind::Ident
        && token.value.to_lowercase() == "array"
        && following.is_some_and(|next| next.kind == PhpKind::Symbol && next.value == "(")
}

/// Pula um array inteiro, devolvendo o indice apos o fechamento.
pub fn skip_array(tokens: &[PhpToken], index: usize) -> usize {
    let opener = if token_at(tokens, index).is_some_and(|t| t.value == "[") {
        "["
    } else {
        "("
    };
    let closer = if opener == "[" { "]" } else { ")" };
    let mut depth = 0;
    let mut cursor = index;
    while cursor < tokens.len() {
        let token = &tokens[cursor];
        if token.kind == PhpKind::Symbol && token.value == opener {
            depth += 1;
        } else if token.kind == PhpKind::Symbol && token.value == closer {
            depth -= 1;
            if depth == 0 {
                return cursor + 1;
            }
        }
        cursor += 1;
    }
    cursor
}

/// Le valor literal com concatenacao; devolve valor, linha e proximo indice.
pub fn read_literal_value(tokens: &[PhpToken], index: usize) -> (Option<String>, u32, usize) {
    let mut cursor = index;
    let mut parts: Vec<String> = Vec::new();
    let first = token_at(tokens, cursor);
    let value_line = first.map_or(1, |token| token.line);
    while cursor < tokens.len() {
        let token = &tokens[cursor];
        if token.kind == PhpKind::Str {
            parts.push(token.value.clone());
            cursor += 1;
            let following = token_at(tokens, cursor);
            let after = token_at(tokens, cursor + 1);
            if following.is_some_and(|next| next.kind == PhpKind::Symbol && next.value == ".")
                && after.is_some_and(|next| next.kind == PhpKind::Str)
            {
                cursor += 1;
                continue;
            }
            return (Some(parts.concat()), value_line, cursor);
        }
        break;
    }
    (None, value_line, cursor)
}

/// Pula valor escalar ate `,` ou fechamento no mesmo nivel.
pub fn skip_scalar_value(tokens: &[PhpToken], index: usize) -> usize {
    let mut cursor = index;
    let mut depth = 0;
    while cursor < tokens.len() {
        let token = &tokens[cursor];
        if token.kind == PhpKind::Symbol {
            if token.value == "[" || token.value == "(" {
                depth += 1;
            } else if token.value == "]" || token.value == ")" {
                if depth == 0 {
                    return cursor;
                }
                depth -= 1;
            } else if depth == 0 && token.value == "," {
                return cursor;
            }
        }
        cursor += 1;
    }
    cursor
}

/// Chave exibida com pontos escapados (`a.b` vira `a\.b`).
pub fn display_key(segments: &[String]) -> String {
    segments
        .iter()
        .map(|segment| segment.replace('\\', "\\\\").replace('.', "\\."))
        .collect::<Vec<_>>()
        .join(".")
}

/// Chave de comparacao (JSON dos segmentos).
pub fn entry_path_key(segments: &[String]) -> String {
    let mut out = String::from("[");
    for (index, segment) in segments.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('"');
        for c in segment.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                c => out.push(c),
            }
        }
        out.push('"');
    }
    out.push(']');
    out
}

/// Faz o parse recursivo do array em `entries`.
pub fn parse_lang_array(
    tokens: &[PhpToken],
    index: usize,
    path: &[String],
    entries: &mut Vec<LangEntry>,
) -> usize {
    let opener = if token_at(tokens, index).is_some_and(|t| t.value == "[") {
        "["
    } else {
        "("
    };
    let closer = if opener == "[" { "]" } else { ")" };
    let mut cursor = index + if opener == "[" { 1 } else { 2 };
    while cursor < tokens.len() {
        let token = &tokens[cursor].clone();
        if token.kind == PhpKind::Symbol && token.value == closer {
            return cursor + 1;
        }
        let following = token_at(tokens, cursor + 1).cloned();
        if token.kind == PhpKind::Str
            && following
                .as_ref()
                .is_some_and(|next| next.kind == PhpKind::Symbol && next.value == "=>")
        {
            let key = token.value.clone();
            let key_line = token.line;
            cursor += 2;
            if is_array_start(tokens, cursor) {
                let mut child = path.to_vec();
                child.push(key);
                cursor = parse_lang_array(tokens, cursor, &child, entries);
                continue;
            }
            let (value, value_line, next_index) = read_literal_value(tokens, cursor);
            let mut segments = path.to_vec();
            segments.push(key);
            entries.push(LangEntry {
                key: display_key(&segments),
                segments,
                line: key_line,
                value: value.clone(),
                value_line,
            });
            cursor = if value.is_some() {
                next_index
            } else {
                skip_scalar_value(tokens, cursor)
            };
            continue;
        }
        if is_array_start(tokens, cursor) {
            cursor = skip_array(tokens, cursor);
            continue;
        }
        cursor += 1;
    }
    cursor
}

/// Extrai as entradas do primeiro `return [...]` (ou primeiro array).
pub fn extract_php_lang_entries(text: &str) -> Vec<LangEntry> {
    let tokens = tokenize_php(text);
    let mut entries = Vec::new();
    let mut start: Option<usize> = None;
    for (position, token) in tokens.iter().enumerate() {
        if token.kind == PhpKind::Ident
            && token.value == "return"
            && is_array_start(&tokens, position + 1)
        {
            start = Some(position + 1);
            break;
        }
    }
    if start.is_none() {
        for position in 0..tokens.len() {
            if is_array_start(&tokens, position) {
                start = Some(position);
                break;
            }
        }
    }
    if let Some(begin) = start {
        parse_lang_array(&tokens, begin, &[], &mut entries);
    }
    entries
}

/// Compara lang EN com PT (faltantes, extras, comuns, chaves).
pub fn compare_lang_entries(english_text: &str, portuguese_text: &str) -> LangComparison {
    use std::collections::HashMap;
    let english_entries = extract_php_lang_entries(english_text);
    let portuguese_entries = extract_php_lang_entries(portuguese_text);
    let mut english_map: HashMap<String, (LangEntry, usize)> = HashMap::new();
    for entry in &english_entries {
        let path_key = entry_path_key(&entry.segments);
        english_map
            .entry(path_key)
            .and_modify(|slot| slot.1 += 1)
            .or_insert_with(|| (entry.clone(), 1));
    }
    let mut portuguese_map: HashMap<String, (LangEntry, usize)> = HashMap::new();
    for entry in &portuguese_entries {
        let path_key = entry_path_key(&entry.segments);
        portuguese_map
            .entry(path_key)
            .and_modify(|slot| slot.1 += 1)
            .or_insert_with(|| (entry.clone(), 1));
    }
    let mut missing = Vec::new();
    let mut extra = Vec::new();
    let mut common = Vec::new();
    let mut keys_compared = 0;
    for (path_key, (entry, count)) in &english_map {
        let Some((translated, translated_count)) = portuguese_map.get(path_key) else {
            missing.push(entry.clone());
            continue;
        };
        keys_compared += 1;
        if count > translated_count {
            missing.push(entry.clone());
        }
        common.push((entry.clone(), translated.clone()));
    }
    for (path_key, (entry, count)) in &portuguese_map {
        match english_map.get(path_key) {
            None => extra.push(entry.clone()),
            Some((_, source_count)) if count > source_count => extra.push(entry.clone()),
            _ => {}
        }
    }
    LangComparison {
        english_entries,
        portuguese_entries,
        missing,
        extra,
        common,
        keys_compared,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_strings_comments_and_heredoc() {
        let tokens = tokenize_php("<?php // oi\n'a' . 'b'; /* x */\n");
        let kinds: Vec<&str> = tokens
            .iter()
            .map(|t| match t.kind {
                PhpKind::Str => "str",
                PhpKind::Symbol => "sym",
                PhpKind::Ident => "ident",
                PhpKind::Other => "other",
            })
            .collect();
        assert!(kinds.contains(&"str"));
        assert!(!kinds.contains(&"ident"));

        let heredoc = "<?php return [\n'v' => <<<EOT\nlinha1\nlinha2\nEOT\n];";
        let entries = extract_php_lang_entries(heredoc);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].value.as_deref(), Some("linha1\nlinha2\n"));
    }

    #[test]
    fn extracts_nested_arrays_with_escaped_dots() {
        let text = "<?php return [\n'a.b' => 'x',\n'grupo' => [\n'c' => 'y',\n],\n];";
        let entries = extract_php_lang_entries(text);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, "a\\.b");
        assert_eq!(entries[1].key, "grupo.c");
        assert_eq!(
            entries[1].segments,
            vec!["grupo".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn concatenates_literals_and_skips_non_literals() {
        let text = "\u{feff}<?php return ['a' => 'x' . 'y', 'b' => SOME_CONST, 'c' => 'ol\\á'];";
        let entries = extract_php_lang_entries(text);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].value.as_deref(), Some("xy"));
        assert_eq!(entries[1].value, None);
        assert_eq!(entries[2].value.as_deref(), Some("ol\\á"));
    }

    #[test]
    fn keeps_interpolation_literal_and_array_syntax() {
        let text = "<?php return ['a' => \"ola {$name}, x\", 'b' => array('c' => 'y')];";
        let entries = extract_php_lang_entries(text);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].value.as_deref(), Some("ola {$name}, x"));
        assert_eq!(entries[1].segments, vec!["b".to_string(), "c".to_string()]);
    }

    #[test]
    fn compares_with_duplicate_counts() {
        let en_text = "<?php return ['a' => 'x', 'a' => 'x2', 'b' => 'y'];";
        let pt_text = "<?php return ['a' => 'x', 'c' => 'z'];";
        let compared = compare_lang_entries(en_text, pt_text);
        assert_eq!(compared.keys_compared, 1);
        // Duplicata EN conta como faltante.
        assert_eq!(compared.missing.len(), 2);
        assert_eq!(compared.extra.len(), 1);
        assert_eq!(compared.common.len(), 1);
    }

    #[test]
    fn entry_path_key_is_json() {
        assert_eq!(
            entry_path_key(&["a".to_string(), "b.c".to_string()]),
            r#"["a","b.c"]"#
        );
    }
}
