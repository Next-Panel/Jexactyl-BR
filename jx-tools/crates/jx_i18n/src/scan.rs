//! Varredura de fontes para extrair comentarios, esqueleto e labels.
//!
//! Port fiel de `scripts/i18n/i18n_toolkit/scanner.py`: classifica strings
//! como labels traduziveis, extrai `lang_keys` (`__`, `trans`,
//! `trans_choice`, `Lang::get`, `@lang`, `@choice`), comentarios e o
//! esqueleto normalizado usado pela auditoria.
//!
//! Equivalencias e divergencias conscientes em relacao ao Python:
//!
//! - Janela de contexto: o Python mantem `code_tail` com os ultimos 240
//!   caracteres (fatia de `str`, ou seja, pontos de codigo Unicode) e usa
//!   `code_tail[-160:]` como `context` na classificacao. Aqui as duas
//!   janelas sao mantidas em contagem de `char` (ponto de codigo Unicode,
//!   mesma unidade do Python) com os mesmos limites 240/160.
//! - `(?<![...])`: o crate `regex` nao suporta lookbehind; a verificacao do
//!   caractere anterior e feita a mao (`preceded_by` + `is_word_char`).
//! - Backreferences (`\1`, `\3`) e lookahead negativo (`(?!...)`) usados em
//!   `extract_category_labels`, `read_blade_echo` e `is_tab_label_context`
//!   nao existem no crate `regex`; esses trechos usam varredura manual com
//!   tratamento de escape (`\`) equivalente.
//! - `normalize_blade_echo_labels` do Python (`scanner.py`, linhas 1274-1286)
//!   tem um defeito: `replace_statement` recebe o `re.Match` (nao a string)
//!   e o `re.sub` interno falha com `TypeError` quando o texto contem
//!   `echo ...;`. Aqui o comportamento pretendido e implementado (apaga
//!   labels dentro de sentencas `echo ...;`) sem o `TypeError`.
//! - `scan_source` infere `lang_file` com a regex
//!   `resources/lang/(?:en|pt-BR)/` aplicada ao parametro `kind` (nao ao
//!   `file_name`), exatamente como no Python — inclusive a esquisitice.
//! - Indices de `extract_category_labels` / `blank_preserving_newlines` /
//!   `line_at` sao contagens de `char` (pontos de codigo), a mesma unidade
//!   dos indices de `str` do Python; para ASCII sao identicos a bytes.
//! - `code_string_token` usa `serde_json` no lugar de `json.dumps` com
//!   `ensure_ascii=False`; ambos preservam nao-ASCII e escapam `"`, `\` e
//!   controles da mesma forma para os casos do inventario.
//! - `data-testid` como contexto de chave (`data-testid: `) classifica como
//!   label quando o texto e humano: o identificador capturado e `testid`
//!   (o hifen quebra o nome), que nao consta em `CODE_KEYS` — quirk portado
//!   sem alteracao.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use regex::Regex;
use sha2::{Digest, Sha256};

use crate::text as text_helpers;

/// Atributos HTML/JSX cujo conteudo e label traduzivel.
///
/// Copiado verbatim de `LABEL_ATTRIBUTES` (`scanner.py`).
pub const LABEL_ATTRIBUTES: [&str; 19] = [
    "title",
    "content",
    "placeholder",
    "label",
    "description",
    "message",
    "alt",
    "aria-label",
    "tooltip",
    "heading",
    "caption",
    "summary",
    "confirm",
    "cancel",
    "submit",
    "text",
    "value",
    "children",
    "error",
];

/// Chaves de objeto cujo valor e label traduzivel (`LABEL_ATTRIBUTES` + extras).
///
/// Copiado verbatim de `LABEL_KEYS` (`scanner.py`).
pub const LABEL_KEYS: [&str; 35] = [
    "title",
    "content",
    "placeholder",
    "label",
    "description",
    "message",
    "alt",
    "aria-label",
    "tooltip",
    "heading",
    "caption",
    "summary",
    "confirm",
    "cancel",
    "submit",
    "text",
    "value",
    "children",
    "error",
    "empty",
    "emptyText",
    "helper",
    "helperText",
    "hint",
    "noResults",
    "okText",
    "cancelText",
    "confirmText",
    "successMessage",
    "errorMessage",
    "warningMessage",
    "greeting",
    "subtitle",
    "headline",
    "ariaLabel",
];

/// Chaves de objeto cujo valor e codigo (nunca label).
///
/// Copiado verbatim de `CODE_KEYS` (`scanner.py`).
pub const CODE_KEYS: [&str; 44] = [
    "id",
    "key",
    "name",
    "type",
    "className",
    "class",
    "css",
    "style",
    "path",
    "url",
    "href",
    "src",
    "to",
    "route",
    "query",
    "selector",
    "token",
    "permission",
    "rule",
    "pattern",
    "format",
    "locale",
    "mime",
    "extension",
    "event",
    "action",
    "method",
    "color",
    "variant",
    "size",
    "mode",
    "modules",
    "status",
    "icon",
    "target",
    "rel",
    "role",
    "tabIndex",
    "testId",
    "data-testid",
    "sql",
    "html",
    "gql",
    "graphql",
];

/// Funcoes cuja chamada com string indica label (inclui validadores yup).
///
/// Copiado verbatim de `LABEL_FUNCTIONS` (`scanner.py`).
pub const LABEL_FUNCTIONS: [&str; 23] = [
    "alert",
    "confirm",
    "notify",
    "toast",
    "success",
    "error",
    "warning",
    "info",
    "message",
    "required",
    "min",
    "max",
    "email",
    "matches",
    "oneOf",
    "notOneOf",
    "url",
    "uuid",
    "integer",
    "positive",
    "label",
    "text",
    "typeError",
];

/// Funcoes cuja chamada com string indica codigo (nunca label).
///
/// Copiado verbatim de `CODE_FUNCTIONS` (`scanner.py`).
pub const CODE_FUNCTIONS: [&str; 35] = [
    "t",
    "i18n",
    "translate",
    "trans",
    "__",
    "require",
    "import",
    "querySelector",
    "getElementById",
    "createElement",
    "fetch",
    "axios",
    "request",
    "get",
    "post",
    "put",
    "patch",
    "delete",
    "useState",
    "useRef",
    "useMemo",
    "useCallback",
    "useEffect",
    "styled",
    "css",
    "tw",
    "keyframes",
    "format",
    "parse",
    "stringify",
    "join",
    "split",
    "replace",
    "match",
    "test",
];

/// Props de componentes cujo valor e label, por tag em minusculas.
///
/// Copiado verbatim de `LABEL_COMPONENT_PROPS` (`scanner.py`); a comparacao
/// e case-insensitive nos dois lados (tag e atributo sao minusculizados).
pub const LABEL_COMPONENT_PROPS: [(&str, &[&str]); 7] = [
    ("tableheader", &["name"]),
    ("money", &["suffix"]),
    ("confirmationmodal", &["buttontext"]),
    ("permissionrow", &["name"]),
    ("suggestioncard", &["action"]),
    ("box", &["recommended"]),
    ("subnavigationlink", &["name"]),
];

/// Precedentes que fecham parentese no esqueleto (paridade com o Python).
///
/// Copiado verbatim de `WRAP_PAREN_PRECEDERS` (`scanner.py`).
pub const WRAP_PAREN_PRECEDERS: [&str; 7] = ["&&", "||", "??", "?", ":", "return", "=>"];

/// Mensagem padrao de correcao para achados de codigo traduzido.
///
/// Copiado verbatim de `CODE_FIX_MESSAGE` (`scanner.py`).
pub const CODE_FIX_MESSAGE: &str =
    "Mantenha identificadores, tipos, chaves e logica intactos; traduza apenas labels.";

/// Fonte da regex de assercoes de teste (`TEST_ASSERTION_PATTERN`).
pub const TEST_ASSERTION_PATTERN_SOURCE: &str = r"\b(?:getByText|getByLabelText|getByPlaceholderText|getByRole|findByText|findByLabelText|findByPlaceholderText|queryByText|queryByLabelText|toHaveTextContent|toContainText|toContain|toMatchText|textContent|innerText|screen\.get|screen\.find|within)\s*\(";

/// Tamanho da janela rolante de codigo (equivalente aos 240 chars do Python).
pub const CODE_TAIL_WINDOW: usize = 240;
/// Tamanho do contexto usado na classificacao (equivale ao `[-160:]`).
pub const CLASSIFY_CONTEXT_WINDOW: usize = 160;

/// Compila padrao estatico.
///
/// O `expect` aqui e proposital e localizado: os padroes sao constantes
/// revisadas, e qualquer regressao quebra os testes de unidade na hora.
#[allow(clippy::expect_used)]
fn pattern(raw: &str) -> Regex {
    // So padroes constantes chegam aqui; regressao quebra os testes.
    Regex::new(raw).expect("padrao regex estatico invalido")
}

/// Regex estatica com inicializacao unica (sem panico em nenhum caminho alem
/// do `pattern` central, ja permitido).
fn static_regex(cell: &'static OnceLock<Regex>, raw: &str) -> &'static Regex {
    cell.get_or_init(|| pattern(raw))
}

/// Ultimos `n` caracteres (pontos de codigo) de `s`.
fn last_n_chars(s: &str, n: usize) -> String {
    let count = s.chars().count();
    if count <= n {
        return s.to_string();
    }
    let skip = count - n;
    let byte_pos: usize = s.chars().take(skip).map(|c| c.len_utf8()).sum();
    s.get(byte_pos..).unwrap_or("").to_string()
}

/// Acrescenta `addition` a `tail` mantendo no maximo `max` caracteres.
fn push_bounded(tail: &mut String, addition: &str, max: usize) {
    tail.push_str(addition);
    let count = tail.chars().count();
    if count > max {
        let skip = count - max;
        let byte_pos: usize = tail.chars().take(skip).map(|c| c.len_utf8()).sum();
        if byte_pos <= tail.len() {
            tail.drain(..byte_pos);
        }
    }
}

/// Equivalente proximo de `\w` do Python (letras Unicode + digitos + `_`).
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Anterior e proibido ou e caractere de palavra (emula `(?<![...])` + `\w`).
fn blocked_before(text: &str, byte_start: usize, forbidden: &[char]) -> bool {
    let Some(prefix) = text.get(..byte_start) else {
        return false;
    };
    prefix
        .chars()
        .next_back()
        .is_some_and(|c| forbidden.contains(&c) || is_word_char(c))
}

/// Especie de fonte reconhecida pelo scanner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceKind {
    /// JavaScript / JSX (`js`, `javascript`, `jsx`, `tsx` -> `js`).
    Js,
    /// TypeScript puro (sem JSX).
    Ts,
    /// Folha de estilo (bloco `<style>`).
    Css,
    /// Arquivo PHP.
    Php,
    /// Template Blade.
    Blade,
    /// HTML / Vue / Svelte.
    Html,
    /// Texto sem regra (vira codigo integral).
    Plain,
}

impl SourceKind {
    /// Nome canonico em minusculas (`js`, `ts`, `css`, `php`, ...).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Js => "js",
            Self::Ts => "ts",
            Self::Css => "css",
            Self::Php => "php",
            Self::Blade => "blade",
            Self::Html => "html",
            Self::Plain => "plain",
        }
    }
}

/// Detecta a especie pela extensao do arquivo (`source_kind` do Python).
pub fn source_kind(file_name: Option<&str>) -> SourceKind {
    let lowered = file_name.unwrap_or("").to_lowercase();
    if lowered.ends_with(".blade.php") {
        return SourceKind::Blade;
    }
    if lowered.ends_with(".php") {
        return SourceKind::Php;
    }
    if lowered.ends_with(".ts") || lowered.ends_with(".mts") || lowered.ends_with(".cts") {
        return SourceKind::Ts;
    }
    if lowered.ends_with(".tsx")
        || lowered.ends_with(".jsx")
        || lowered.ends_with(".js")
        || lowered.ends_with(".mjs")
        || lowered.ends_with(".cjs")
    {
        return SourceKind::Js;
    }
    if lowered.ends_with(".html")
        || lowered.ends_with(".htm")
        || lowered.ends_with(".xhtml")
        || lowered.ends_with(".vue")
        || lowered.ends_with(".svelte")
    {
        return SourceKind::Html;
    }
    SourceKind::Plain
}

/// Normaliza `kind` ou nome de arquivo para a especie (`normalize_kind`).
pub fn normalize_kind(kind_or_file: Option<&str>) -> SourceKind {
    let raw = kind_or_file.unwrap_or("");
    if raw.is_empty() {
        return SourceKind::Js;
    }
    match raw.to_lowercase().as_str() {
        "js" | "javascript" | "jsx" | "tsx" => SourceKind::Js,
        "ts" | "typescript" => SourceKind::Ts,
        "css" => SourceKind::Css,
        "php" => SourceKind::Php,
        "blade" | "blade.php" => SourceKind::Blade,
        "html" => SourceKind::Html,
        "plain" => SourceKind::Plain,
        _ => source_kind(Some(raw)),
    }
}

/// SHA-256 hexadecimal truncado em 16 caracteres (`hash_string`).
pub fn hash_string(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(16);
    for byte in digest.iter().take(8) {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Token de codigo para strings nao-label (`code_string_token`).
///
/// Strings longas (>120 chars) ou multilinha viram `STR#<hash>`; as demais
/// viram literal JSON com `ensure_ascii=False` (via `serde_json`).
pub fn code_string_token(text: &str) -> String {
    let normalized = text_helpers::normalize_newlines(text);
    if normalized.chars().count() > 120 || normalized.contains('\n') {
        return format!("STR#{}", hash_string(&normalized));
    }
    serde_json::to_string(&normalized).unwrap_or_else(|_| format!("{normalized:?}"))
}

/// Natureza do identificador no fim do contexto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextKind {
    /// `chave:` ou `chave =` no fim do contexto.
    Key,
    /// `chamada(` no fim do contexto.
    Call,
}

impl ContextKind {
    /// Nome canonico (`key` ou `call`, como no dicionario Python).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Key => "key",
            Self::Call => "call",
        }
    }
}

/// Identificador extraido do fim do contexto (`context_identifier`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextIdentifier {
    /// Se e chave (`key`) ou chamada (`call`).
    pub kind: ContextKind,
    /// Nome sem `$` inicial.
    pub name: String,
}

impl ContextIdentifier {
    /// Cria um identificador ja sem `$` inicial.
    pub fn new(kind: ContextKind, name: &str) -> Self {
        Self {
            kind,
            name: name.trim_start_matches('$').to_string(),
        }
    }
}

/// Extrai `chave:`/`chave =` ou `chamada(` do fim do contexto.
pub fn context_identifier(tail: &str) -> Option<ContextIdentifier> {
    let trimmed_end_len = tail.trim_end().len();
    let trimmed = tail.get(..trimmed_end_len).unwrap_or(tail);
    static KEY_CELL: OnceLock<Regex> = OnceLock::new();
    static CALL_CELL: OnceLock<Regex> = OnceLock::new();
    let key_re = static_regex(&KEY_CELL, r"([A-Za-z_$][\w$]*)\s*(?::|=)\s*$");
    if let Some(caps) = key_re.captures(trimmed) {
        let whole = caps.get(0).map_or("", |m| m.as_str());
        let start = caps.get(0).map_or(0, |m| m.start());
        let name = caps.get(1).map_or("", |m| m.as_str());
        let _ = whole;
        if !blocked_before(trimmed, start, &['.', '$']) {
            return Some(ContextIdentifier::new(ContextKind::Key, name));
        }
    }
    let call_re = static_regex(&CALL_CELL, r"\.?([A-Za-z_$][\w$]*)\(\s*$");
    if let Some(caps) = call_re.captures(trimmed) {
        let name = caps.get(1).map_or("", |m| m.as_str());
        return Some(ContextIdentifier::new(ContextKind::Call, name));
    }
    None
}

/// Informa se o contexto e codigo de template (`styled`, `css`, ...) .
pub fn is_template_code_context(context: &str) -> bool {
    static CELL: OnceLock<Regex> = OnceLock::new();
    let re = static_regex(
        &CELL,
        r"(?:^|[^\w$])(?:styled(?:\.[A-Za-z_$][\w$]*)?|css|tw|keyframes|html|sql|gql|graphql)\s*$",
    );
    re.is_match(context)
}

/// Informa se o contexto e `route(... name:` (label de rota traduzivel).
pub fn is_route_name_context(context: &str) -> bool {
    static CELL: OnceLock<Regex> = OnceLock::new();
    let re = static_regex(&CELL, r"\broute\s*\([^;]*\bname\s*:\s*$");
    re.is_match(context)
}

/// Informa se o contexto e `{path: '...', name:` (aba traduzivel).
pub fn is_tab_label_context(context: &str) -> bool {
    // A regex original usa backreference (`\1`); aqui a aspa e casada a mao.
    static CELL: OnceLock<Regex> = OnceLock::new();
    let prefix = static_regex(&CELL, r#"[{,]\s*path\s*:\s*(['"])"#);
    let mut last: Option<(usize, char, usize)> = None;
    for caps in prefix.captures_iter(context) {
        let quote = caps
            .get(1)
            .map_or('\0', |m| m.as_str().chars().next().unwrap_or('\0'));
        let end = caps.get(0).map_or(0, |m| m.end());
        last = Some((caps.get(0).map_or(0, |m| m.start()), quote, end));
    }
    let Some((_, quote, mut cursor)) = last else {
        return false;
    };
    let bytes = context.as_bytes();
    while cursor < context.len() {
        let c = bytes[cursor] as char;
        if c == quote {
            let rest = context.get(cursor + 1..).unwrap_or("");
            static REST_CELL: OnceLock<Regex> = OnceLock::new();
            let rest_re = static_regex(&REST_CELL, r"^\s*,\s*name\s*:\s*$");
            return rest_re.is_match(rest);
        }
        if c == '\'' || c == '"' {
            return false;
        }
        cursor += 1;
    }
    false
}

/// Informa se o atributo e prop traduzivel do componente (`LABEL_COMPONENT_PROPS`).
///
/// Com `jsx_tag`, compara direto (tag e atributo em minusculas); sem tag,
/// usa a ultima `<Tag` encontrada no `context`, como no Python.
pub fn is_label_component_prop(
    context: Option<&str>,
    attribute_name: Option<&str>,
    jsx_tag: Option<&str>,
) -> bool {
    let lowered = attribute_name.unwrap_or("").to_lowercase();
    if let Some(tag) = jsx_tag {
        let wanted = tag.to_lowercase();
        for (name, props) in LABEL_COMPONENT_PROPS {
            if name == wanted {
                return props.contains(&lowered.as_str());
            }
        }
        return false;
    }
    let ctx = context.unwrap_or("");
    static CELL: OnceLock<Regex> = OnceLock::new();
    let re = static_regex(&CELL, r"<([A-Za-z][A-Za-z0-9]*)\b");
    let mut last: Option<String> = None;
    for caps in re.captures_iter(ctx) {
        last = Some(caps.get(1).map_or("", |m| m.as_str()).to_lowercase());
    }
    let Some(tag) = last else {
        return false;
    };
    for (name, props) in LABEL_COMPONENT_PROPS {
        if name == tag {
            return props.contains(&lowered.as_str());
        }
    }
    false
}

/// Contexto JSX ativo durante `{...}` (`JsxContext` do Python).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JsxContext {
    /// `child` ou `attribute`.
    pub kind: String,
    /// Nome do atributo ao redor, quando houver.
    pub attribute: Option<String>,
    /// Tag ao redor, quando houver.
    pub tag: Option<String>,
}

impl JsxContext {
    /// Cria um contexto JSX (`kind` vazio equivale ao padrao do Python).
    pub fn new(kind: &str, attribute: Option<&str>, tag: Option<&str>) -> Self {
        Self {
            kind: kind.to_string(),
            attribute: attribute.map(str::to_string),
            tag: tag.map(str::to_string),
        }
    }
}

/// Peca de codigo (afeta o esqueleto como texto literal).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodePiece {
    /// Trecho de codigo.
    pub text: String,
    /// Linha inicial (1-based).
    pub line: u32,
}

/// String localizada com o veredito de label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringPiece {
    /// Conteudo sem aspas.
    pub text: String,
    /// Linha (1-based).
    pub line: u32,
    /// Atributo HTML/JSX ao redor, quando houver.
    pub attribute: Option<String>,
    /// Se veio de template literal / heredoc.
    pub template: bool,
    /// Se e label traduzivel.
    pub label: bool,
    /// Janela de contexto (ultimos 160 chars de `code_tail`).
    pub context: String,
}

/// Texto JSX/HTML visivel (sempre label).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextPiece {
    /// Texto visivel.
    pub text: String,
    /// Linha (1-based, deslocada por quebras internas).
    pub line: u32,
    /// Tag ao redor.
    pub tag: String,
}

/// Peca emitida pelo scanner (codigo, string ou texto).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    /// Trecho de codigo.
    Code(CodePiece),
    /// String com veredito.
    Str(StringPiece),
    /// Texto visivel.
    Text(TextPiece),
}

impl Piece {
    /// Linha inicial da peca.
    pub fn line(&self) -> u32 {
        match self {
            Self::Code(p) => p.line,
            Self::Str(p) => p.line,
            Self::Text(p) => p.line,
        }
    }
}

/// Comentario extraido.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentRecord {
    /// `line` ou `block`.
    pub kind: String,
    /// Sintaxe (`js-line`, `php-block`, `blade`, `html`, ...).
    pub syntax: String,
    /// Texto normalizado.
    pub text: String,
    /// Linha (1-based).
    pub line: u32,
}

/// Chave de idioma (`__`, `trans`, `Lang::get`, `@lang`, ...).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LangKey {
    /// Chave (ex.: `messages.title`).
    pub key: String,
    /// Linha (1-based).
    pub line: u32,
}

/// Linha do esqueleto normalizado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkeletonLine {
    /// Linha original (1-based).
    pub line: u32,
    /// Texto normalizado (`""` = label, `TEXT` = texto visivel).
    pub text: String,
}

/// Resultado da varredura (`ScanResult` do Python).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScanResult {
    /// Especie normalizada (`js`, `ts`, `php`, `blade`, `html`, ...).
    pub kind: String,
    /// Comentarios normalizados.
    pub comments: Vec<CommentRecord>,
    /// Esqueleto por linha.
    pub lines: Vec<SkeletonLine>,
    /// Labels (strings com `label == true` + todos os textos visiveis).
    pub labels: Vec<Piece>,
    /// Chaves de idioma encontradas.
    pub lang_keys: Vec<LangKey>,
}

/// Classifica uma string como label traduzivel (`classify_string`).
///
/// `attribute` e `None` quando nao ha atributo; `context` sao os ultimos
/// 160 caracteres de codigo; `jsx` e o `JsxContext` ativo em `{...}`.
pub fn classify_string(
    text: &str,
    attribute: Option<&str>,
    context: &str,
    lang_file: bool,
    test_file: bool,
    jsx_context: Option<&JsxContext>,
) -> bool {
    if lang_file {
        return true;
    }
    if let Some(attr) = attribute {
        let name = attr.to_lowercase();
        if is_label_component_prop(Some(context), Some(&name), None) {
            return true;
        }
        if name == "defaultvalue" && !LABEL_ATTRIBUTES.contains(&name.as_str()) {
            let trimmed = text.trim();
            return text_helpers::is_human_text(text)
                || starts_capital(trimmed)
                || !text_helpers::extract_placeholders(text).is_empty();
        }
        if !LABEL_ATTRIBUTES.contains(&name.as_str()) {
            return false;
        }
        if name == "value" {
            let trimmed = text.trim();
            return text_helpers::is_human_text(text)
                || starts_capital(trimmed)
                || !text_helpers::extract_placeholders(text).is_empty();
        }
        return true;
    }
    if test_file {
        static CELL: OnceLock<Regex> = OnceLock::new();
        let re = static_regex(&CELL, TEST_ASSERTION_PATTERN_SOURCE);
        return re.is_match(context);
    }
    static CONSOLE_CELL: OnceLock<Regex> = OnceLock::new();
    if static_regex(
        &CONSOLE_CELL,
        r"console\.(?:log|warn|error|debug|info|trace|dir|table|assert)\s*\([^)]*$",
    )
    .is_match(context)
    {
        return false;
    }
    static IMPORT_CELL: OnceLock<Regex> = OnceLock::new();
    if static_regex(&IMPORT_CELL, r"\b(?:import|export)\b[^;]*\bfrom\s*$").is_match(context) {
        return false;
    }
    static REQUIRE_CELL: OnceLock<Regex> = OnceLock::new();
    if static_regex(&REQUIRE_CELL, r"\b(?:require|import)\s*\(\s*$").is_match(context) {
        return false;
    }
    if is_template_code_context(context) {
        return false;
    }
    static ADDERR_CELL: OnceLock<Regex> = OnceLock::new();
    if static_regex(&ADDERR_CELL, r"addError\s*\([^()]*,\s*$").is_match(context) {
        return true;
    }
    static KEY_CELL: OnceLock<Regex> = OnceLock::new();
    if let Some(caps) = static_regex(&KEY_CELL, r"KEY\(([^)]*)\)\s*=>\s*$").captures(context) {
        let key = caps.get(1).map_or("", |m| m.as_str());
        return LABEL_KEYS.contains(&key);
    }
    if let Some(info) = context_identifier(context) {
        if info.kind == ContextKind::Key && info.name == "name" && is_route_name_context(context) {
            return true;
        }
        if info.kind == ContextKind::Key && info.name == "name" && is_tab_label_context(context) {
            return true;
        }
        if info.kind == ContextKind::Key && LABEL_KEYS.contains(&info.name.as_str()) {
            return true;
        }
        if info.kind == ContextKind::Call && LABEL_FUNCTIONS.contains(&info.name.as_str()) {
            return true;
        }
        if CODE_KEYS.contains(&info.name.as_str()) || CODE_FUNCTIONS.contains(&info.name.as_str()) {
            return false;
        }
    }
    let trimmed_text = text.trim();
    static URL_CELL: OnceLock<Regex> = OnceLock::new();
    static FILE_CELL: OnceLock<Regex> = OnceLock::new();
    let url_re = static_regex(
        &URL_CELL,
        r"(?i)^(?:(?:https?|ftps?|sftp|file)://|(?:mailto|tel):|//)",
    );
    let file_re = static_regex(
        &FILE_CELL,
        r"(?i)^[A-Za-z0-9_./-]+\.(?:tsx?|jsx?|php|css|scss|json|svg|png|jpe?g|gif|webp|woff2?|ttf|eot)$",
    );
    if url_re.is_match(trimmed_text)
        || trimmed_text.contains("://")
        || file_re.is_match(trimmed_text)
    {
        static LETTER_CELL: OnceLock<Regex> = OnceLock::new();
        let letter = static_regex(&LETTER_CELL, r"[A-Za-zÀ-ÿ]");
        if !letter.is_match(&text_helpers::strip_urls(trimmed_text)) {
            return false;
        }
    }
    if let Some(jsx) = jsx_context {
        let outer = jsx.attribute.as_deref().map(str::to_lowercase);
        static TECH_CELL: OnceLock<Regex> = OnceLock::new();
        let technical = static_regex(&TECH_CELL, r"^[a-z0-9_.:/#-]+$").is_match(trimmed_text);
        static PUNCT_CELL: OnceLock<Regex> = OnceLock::new();
        let punct = static_regex(&PUNCT_CELL, r"[./#:-]");
        if let Some(outer_attribute) = outer {
            if is_label_component_prop(Some(context), Some(&outer_attribute), jsx.tag.as_deref()) {
                return true;
            }
            if outer_attribute == "defaultvalue"
                && !LABEL_ATTRIBUTES.contains(&outer_attribute.as_str())
            {
                return text_helpers::is_human_text(text)
                    || starts_capital(trimmed_text)
                    || !text_helpers::extract_placeholders(text).is_empty();
            }
            if !LABEL_ATTRIBUTES.contains(&outer_attribute.as_str()) {
                return false;
            }
            if technical && punct.is_match(trimmed_text) {
                return false;
            }
            if outer_attribute == "value" || outer_attribute == "children" {
                return text_helpers::is_human_text(text)
                    || starts_capital(trimmed_text)
                    || !text_helpers::extract_placeholders(text).is_empty();
            }
            return true;
        }
        if jsx.kind == "child" {
            if technical && punct.is_match(trimmed_text) {
                return false;
            }
            if trimmed_text.starts_with('/') {
                return false;
            }
            let mut unprefixed = trimmed_text.strip_prefix('(').unwrap_or(trimmed_text);
            static LEAD_CELL: OnceLock<Regex> = OnceLock::new();
            let lead = static_regex(&LEAD_CELL, r"^(\{[^{}]*\}\s*)+");
            if let Some(m) = lead.find(unprefixed) {
                unprefixed = unprefixed.get(m.end()..).unwrap_or("");
            }
            static LQ_CELL: OnceLock<Regex> = OnceLock::new();
            static TQ_CELL: OnceLock<Regex> = OnceLock::new();
            let lq = static_regex(&LQ_CELL, r#"^['"“”‘’«»]+"#);
            let tq = static_regex(&TQ_CELL, r#"['"“”‘’«»]+$"#);
            let mut unquoted = lq.replace(unprefixed, "").into_owned();
            unquoted = tq.replace(&unquoted, "").into_owned();
            let unquoted = unquoted.trim();
            static HUMAN_CELL: OnceLock<Regex> = OnceLock::new();
            let human = static_regex(
                &HUMAN_CELL,
                r"^[A-Za-zÀ-ÿ][A-Za-zÀ-ÿ0-9 ,.'!?:;()/&+\-—–«»¿¡]*$",
            );
            if human.is_match(unquoted) {
                static LOWER_CELL: OnceLock<Regex> = OnceLock::new();
                static DASH_CELL: OnceLock<Regex> = OnceLock::new();
                let lower = static_regex(&LOWER_CELL, r"^[a-z0-9_:\s-]+$");
                let dash = static_regex(&DASH_CELL, r"[-:]");
                if lower.is_match(trimmed_text) && dash.is_match(trimmed_text) {
                    return false;
                }
                return true;
            }
            return false;
        }
    }
    static SLASH_CELL: OnceLock<Regex> = OnceLock::new();
    if static_regex(&SLASH_CELL, r"^\s*/").is_match(text) {
        return false;
    }
    if text.contains("${") || text.contains("{$") {
        static DOLLAR_CELL: OnceLock<Regex> = OnceLock::new();
        static VAR_CELL: OnceLock<Regex> = OnceLock::new();
        static LETTER2_CELL: OnceLock<Regex> = OnceLock::new();
        let dollar = static_regex(&DOLLAR_CELL, r"\$\{[^}]*\}");
        let var_re = static_regex(&VAR_CELL, r"\{\$[^}]*\}");
        let visible = var_re
            .replace_all(&dollar.replace_all(text, " "), " ")
            .into_owned();
        let letter = static_regex(&LETTER2_CELL, r"[A-Za-zÀ-ÿ]");
        return letter.is_match(&visible) && text_helpers::is_human_text(&visible);
    }
    text_helpers::is_human_text(text)
}

/// Informa se o texto aparado comeca com maiuscula (`^[A-ZÀ-Ý]`).
fn starts_capital(trimmed: &str) -> bool {
    static CELL: OnceLock<Regex> = OnceLock::new();
    static_regex(&CELL, r"^[A-ZÀ-Ý]").is_match(trimmed)
}

/// Varredor de fontes (`SourceScanner` do Python).
///
/// Reestrutura o varredor caractere a caractere original com um cursor sobre
/// `Vec<char>`; todas as regras de emissao e classificacao sao as mesmas.
pub struct SourceScanner {
    /// Caracteres normalizados (`\r\n` -> `\n`, sem BOM inicial).
    chars: Vec<char>,
    /// Especie normalizada.
    kind: SourceKind,
    /// Nome do arquivo (guardado por paridade; sem efeito na varredura).
    file_name: String,
    /// Tudo e label (arquivo de idioma).
    lang_file: bool,
    /// So assercoes contam como label.
    test_file: bool,
    /// Cursor em caracteres.
    index: usize,
    /// Linha atual (1-based).
    line: u32,
    /// Comentarios na ordem de aparicao.
    comments: Vec<CommentRecord>,
    /// Pecas na ordem de aparicao.
    pieces: Vec<Piece>,
    /// Chaves de idioma na ordem de aparicao.
    lang_keys: Vec<LangKey>,
    /// Ultimos 240 caracteres de codigo (ou `""`/`TEXT`/tokens).
    code_tail: String,
    /// Tag HTML atual (para `TextPiece.tag`).
    current_html_tag: String,
    /// Pilha de contextos JSX em `{...}`.
    jsx_contexts: Vec<JsxContext>,
}

impl SourceScanner {
    /// Cria o varredor (`text`, `kind`, `file_name`, `lang_file`, `test_file`).
    pub fn new(text: &str, kind: &str, file_name: &str, lang_file: bool, test_file: bool) -> Self {
        let normalized = text_helpers::normalize_newlines(text);
        let stripped = normalized.trim_start_matches('﻿').to_string();
        Self {
            chars: stripped.chars().collect(),
            kind: normalize_kind(Some(kind)),
            file_name: file_name.to_string(),
            lang_file,
            test_file,
            index: 0,
            line: 1,
            comments: Vec::new(),
            pieces: Vec::new(),
            lang_keys: Vec::new(),
            code_tail: String::new(),
            current_html_tag: String::new(),
            jsx_contexts: Vec::new(),
        }
    }

    /// Especie normalizada do varredor.
    pub fn kind(&self) -> SourceKind {
        self.kind
    }

    /// Nome do arquivo (guardado por paridade; sem efeito na varredura).
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// Fim do texto.
    fn eof(&self) -> bool {
        self.index >= self.chars.len()
    }

    /// Caractere em `index + offset` (`None` no EOF).
    fn peek(&self, offset: usize) -> Option<char> {
        self.chars.get(self.index + offset).copied()
    }

    /// Informa se o texto a partir do cursor comeca com `value`.
    fn starts_with(&self, value: &str) -> bool {
        for (offset, c) in value.chars().enumerate() {
            if self.chars.get(self.index + offset) != Some(&c) {
                return false;
            }
        }
        true
    }

    /// Avanca `count` caracteres contando `\n`.
    fn advance(&mut self, count: usize) {
        let end = (self.index + count).min(self.chars.len());
        while self.index < end {
            if self.chars.get(self.index) == Some(&'\n') {
                self.line += 1;
            }
            self.index += 1;
        }
    }

    /// Texto restante a partir do cursor (para casar regexes multilinha).
    fn rest_from(&self, start: usize) -> String {
        self.chars.iter().skip(start).collect()
    }

    /// Emite codigo e atualiza a janela de 240 chars.
    fn emit_code(&mut self, text: &str, line: Option<u32>) {
        if text.is_empty() {
            return;
        }
        let active = line.unwrap_or(self.line);
        self.pieces.push(Piece::Code(CodePiece {
            text: text.to_string(),
            line: active,
        }));
        push_bounded(&mut self.code_tail, text, CODE_TAIL_WINDOW);
    }

    /// Emite texto visivel (conta quebras iniciais na linha).
    fn emit_text(&mut self, text: &str, line: Option<u32>, tag: &str) {
        if text.is_empty() {
            return;
        }
        let active = line.unwrap_or(self.line);
        let trimmed_len = text.trim_start().len();
        let leading = text.len().saturating_sub(trimmed_len);
        let leading_nl = text
            .get(..leading)
            .unwrap_or("")
            .chars()
            .filter(|c| *c == '\n')
            .count() as u32;
        self.pieces.push(Piece::Text(TextPiece {
            text: text.to_string(),
            line: active + leading_nl,
            tag: tag.to_string(),
        }));
        push_bounded(&mut self.code_tail, "TEXT", CODE_TAIL_WINDOW);
    }

    /// Emite string classificando como label (ou com `forced_label`).
    fn emit_string(
        &mut self,
        text: &str,
        line: u32,
        attribute: Option<&str>,
        template: bool,
        forced_label: Option<bool>,
    ) {
        let context = last_n_chars(&self.code_tail, CLASSIFY_CONTEXT_WINDOW);
        let label = forced_label.unwrap_or_else(|| {
            classify_string(
                text,
                attribute,
                &context,
                self.lang_file,
                self.test_file,
                self.jsx_contexts.last(),
            )
        });
        self.pieces.push(Piece::Str(StringPiece {
            text: text.to_string(),
            line,
            attribute: attribute.map(str::to_string),
            template,
            label,
            context,
        }));
        let rendered = if label {
            "\"\"".to_string()
        } else {
            code_string_token(text)
        };
        push_bounded(&mut self.code_tail, &rendered, CODE_TAIL_WINDOW);
    }

    /// Le comentario de linha (`//...` ate `\n`).
    fn read_line_comment(&mut self, marker_length: usize, syntax: &str) {
        let line = self.line;
        self.advance(marker_length);
        let start = self.index;
        while !self.eof() && self.peek(0) != Some('\n') {
            self.advance(1);
        }
        let text: String = self
            .chars
            .iter()
            .skip(start)
            .take(self.index - start)
            .collect();
        self.comments.push(CommentRecord {
            kind: "line".to_string(),
            syntax: syntax.to_string(),
            text,
            line,
        });
    }

    /// Le comentario de bloco ate `closer` (ou EOF).
    fn read_block_comment(&mut self, opener: &str, closer: &str, syntax: &str) {
        let line = self.line;
        self.advance(opener.chars().count());
        let start = self.index;
        let rest: String = self.rest_from(start);
        let end = rest.find(closer).unwrap_or(rest.len());
        let end_chars: usize = rest.get(..end).unwrap_or("").chars().count();
        self.advance(end_chars);
        let content: String = self.chars.iter().skip(start).take(end_chars).collect();
        if self.starts_with(closer) {
            self.advance(closer.chars().count());
        }
        self.comments.push(CommentRecord {
            kind: "block".to_string(),
            syntax: syntax.to_string(),
            text: content,
            line,
        });
    }

    /// Le comentario de linha PHP (para em `\n` ou `?>`).
    fn read_php_line_comment(&mut self, marker_length: usize, syntax: &str) {
        let line = self.line;
        self.advance(marker_length);
        let start = self.index;
        while !self.eof()
            && self.peek(0) != Some('\n')
            && !(self.peek(0) == Some('?') && self.peek(1) == Some('>'))
        {
            self.advance(1);
        }
        let text: String = self
            .chars
            .iter()
            .skip(start)
            .take(self.index - start)
            .collect();
        self.comments.push(CommentRecord {
            kind: "line".to_string(),
            syntax: syntax.to_string(),
            text,
            line,
        });
    }

    /// Ponto de entrada (`scan` do Python).
    pub fn scan(&mut self) {
        match self.kind {
            SourceKind::Js => self.scan_javascript(true),
            SourceKind::Ts => self.scan_javascript(false),
            SourceKind::Css => self.scan_css(),
            SourceKind::Php => self.scan_php_file(),
            SourceKind::Blade => self.scan_html_region(true, false),
            SourceKind::Html => self.scan_html_region(false, false),
            SourceKind::Plain => {
                let rest = self.rest_from(0);
                self.emit_code(&rest, Some(1));
                self.advance(self.chars.len());
            }
        }
    }

    /// Varre JS/TS com ou sem JSX.
    fn scan_javascript(&mut self, allow_jsx: bool) {
        self.scan_js_code(false, allow_jsx);
    }

    /// Varre CSS (comentarios + strings).
    fn scan_css(&mut self) {
        while !self.eof() {
            let char = self.peek(0).unwrap_or('\0');
            if char == '/' && self.peek(1) == Some('*') {
                self.read_block_comment("/*", "*/", "css-block");
                continue;
            }
            if char == '\'' || char == '"' {
                self.read_js_string(char, None);
                continue;
            }
            let mut buf = String::new();
            buf.push(char);
            self.emit_code(&buf, None);
            self.advance(1);
        }
    }

    /// Nucleo JS (`stop_at_brace` para `{...}`).
    fn scan_js_code(&mut self, stop_at_brace: bool, allow_jsx: bool) {
        let mut depth: usize = 0;
        while !self.eof() {
            let char = self.peek(0).unwrap_or('\0');
            if stop_at_brace {
                if char == '{' {
                    self.emit_code("{", None);
                    self.advance(1);
                    depth += 1;
                    continue;
                }
                if char == '}' {
                    self.emit_code("}", None);
                    self.advance(1);
                    if depth == 0 {
                        return;
                    }
                    depth = depth.saturating_sub(1);
                    continue;
                }
            }
            if char.is_whitespace() {
                let mut buf = String::new();
                buf.push(char);
                self.emit_code(&buf, None);
                self.advance(1);
                continue;
            }
            if char == '/' && self.peek(1) == Some('/') {
                self.read_line_comment(2, "js-line");
                continue;
            }
            if char == '/' && self.peek(1) == Some('*') {
                self.read_block_comment("/*", "*/", "js-block");
                continue;
            }
            if char == '\'' || char == '"' {
                self.read_js_string(char, None);
                continue;
            }
            if char == '`' {
                self.read_template();
                continue;
            }
            if allow_jsx && char == '<' && self.looks_like_jsx_start() {
                self.parse_jsx_element();
                continue;
            }
            if char == '/' && self.looks_like_regex_start() {
                self.read_regex();
                continue;
            }
            let mut buf = String::new();
            buf.push(char);
            self.emit_code(&buf, None);
            self.advance(1);
        }
    }

    /// Heuristica de abertura JSX (port fiel, limite de 300 chars).
    fn looks_like_jsx_start(&self) -> bool {
        let following = self.peek(1).unwrap_or('\0');
        if following == '\0' {
            return false;
        }
        if !(following == '>' || matches!(following, 'A'..='Z' | 'a'..='z' | '_' | '$')) {
            return false;
        }
        let tail = self.code_tail.trim_end().to_string();
        if !tail.is_empty() {
            let previous = tail.chars().next_back().unwrap_or('\0');
            static WORD_CELL: OnceLock<Regex> = OnceLock::new();
            let word_re = static_regex(&WORD_CELL, r"([A-Za-z_$][A-Za-z0-9_$]*)$");
            let previous_word = word_re
                .captures(&tail)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string());
            let allowed_punct = "(=[:?,;!&|{}>";
            let allowed_words = [
                "return", "yield", "await", "case", "else", "do", "in", "of", "new", "typeof",
                "void", "delete", "default", "throw",
            ];
            let word_ok = previous_word
                .as_deref()
                .is_some_and(|w| allowed_words.contains(&w));
            if !allowed_punct.contains(previous) && !word_ok {
                return false;
            }
        }
        let mut depth: usize = 0;
        let mut closing_offset: Option<usize> = None;
        let limit = (self.index + 300).min(self.chars.len());
        let mut cursor = self.index + 1;
        while cursor < limit {
            let char = self.chars.get(cursor).copied().unwrap_or('\0');
            if char == '>' && depth == 0 {
                closing_offset = Some(cursor - self.index);
                break;
            }
            if char == '{' {
                depth += 1;
            } else if char == '}' {
                depth = depth.saturating_sub(1);
            } else if char == ',' && depth == 0 {
                return false;
            } else if char.is_whitespace() {
                let rest: String = self.chars.iter().skip(cursor).take(20).collect();
                static EXT_CELL: OnceLock<Regex> = OnceLock::new();
                if static_regex(&EXT_CELL, r"^\s+extends\b").is_match(&rest) {
                    return false;
                }
            }
            cursor += 1;
        }
        if let Some(offset) = closing_offset {
            let after: String = self.chars.iter().skip(self.index + offset + 1).collect();
            let after_trimmed = after.trim_start();
            if after_trimmed.starts_with("=>") {
                return false;
            }
            if after_trimmed.starts_with('(') {
                let chunk: String = self
                    .chars
                    .iter()
                    .skip(self.index)
                    .take(offset + 1)
                    .collect();
                static TAG_CELL: OnceLock<Regex> = OnceLock::new();
                if static_regex(&TAG_CELL, r"^<[A-Za-z][A-Za-z0-9_$]*>$").is_match(&chunk) {
                    return false;
                }
            }
        }
        true
    }

    /// Heuristica de inicio de regex JS.
    fn looks_like_regex_start(&self) -> bool {
        let tail = self.code_tail.trim_end().to_string();
        if tail.is_empty() {
            return true;
        }
        static WORD_CELL: OnceLock<Regex> = OnceLock::new();
        let word_re = static_regex(&WORD_CELL, r"([A-Za-z_$][A-Za-z0-9_$]*)$");
        let previous_word = word_re
            .captures(&tail)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string());
        let keywords = [
            "return",
            "yield",
            "await",
            "case",
            "typeof",
            "instanceof",
            "in",
            "of",
            "new",
            "delete",
            "void",
            "throw",
            "do",
            "else",
        ];
        if previous_word
            .as_deref()
            .is_some_and(|w| keywords.contains(&w))
        {
            return true;
        }
        tail.chars()
            .next_back()
            .is_some_and(|c| "([{=,:;!&|?+-*%^~".contains(c))
    }

    /// Le literal de regex ate a barra de fechamento + flags ASCII.
    fn read_regex(&mut self) {
        let line = self.line;
        let start = self.index;
        self.advance(1);
        let mut in_class = false;
        while !self.eof() {
            let char = self.peek(0).unwrap_or('\0');
            if char == '\\' {
                self.advance(2);
                continue;
            }
            if char == '[' {
                in_class = true;
            } else if char == ']' {
                in_class = false;
            } else if char == '/' && !in_class {
                self.advance(1);
                while !self.eof()
                    && self.peek(0).unwrap_or('\0').is_ascii_alphabetic()
                    && self.peek(0).unwrap_or('\0').is_ascii()
                {
                    self.advance(1);
                }
                break;
            } else if char == '\n' {
                break;
            }
            self.advance(1);
        }
        let text: String = self
            .chars
            .iter()
            .skip(start)
            .take(self.index - start)
            .collect();
        self.emit_code(&text, Some(line));
    }

    /// Le string JS com `quote` (para em `\n` sem consumir).
    fn read_js_string(&mut self, quote: char, attribute: Option<&str>) {
        let line = self.line;
        self.advance(1);
        let start = self.index;
        while !self.eof() {
            let char = self.peek(0).unwrap_or('\0');
            if char == '\\' {
                self.advance(2);
                continue;
            }
            if char == quote {
                break;
            }
            if char == '\n' {
                break;
            }
            self.advance(1);
        }
        let end = self.index;
        let text: String = self.chars.iter().skip(start).take(end - start).collect();
        if self.peek(0) == Some(quote) {
            self.advance(1);
        }
        self.emit_string(&text, line, attribute, false, None);
    }

    /// Le template literal com a regra de texto visivel do Python.
    fn read_template(&mut self) {
        let line = self.line;
        let context = last_n_chars(&self.code_tail, CLASSIFY_CONTEXT_WINDOW);
        self.advance(1);
        let start = self.index;
        while !self.eof() {
            let char = self.peek(0).unwrap_or('\0');
            if char == '\\' {
                self.advance(2);
                continue;
            }
            if char == '`' {
                break;
            }
            self.advance(1);
        }
        let end = self.index;
        let text: String = self.chars.iter().skip(start).take(end - start).collect();
        if self.peek(0) == Some('`') {
            self.advance(1);
        }
        static DOLLAR_CELL: OnceLock<Regex> = OnceLock::new();
        static VAR_CELL: OnceLock<Regex> = OnceLock::new();
        static SPACE_CELL: OnceLock<Regex> = OnceLock::new();
        static LETTER_CELL: OnceLock<Regex> = OnceLock::new();
        let dollar = static_regex(&DOLLAR_CELL, r"\$\{[^}]*\}");
        let var_re = static_regex(&VAR_CELL, r"\{\$[^}]*\}");
        let visible = var_re
            .replace_all(&dollar.replace_all(&text, " "), " ")
            .into_owned();
        let info = context_identifier(&context);
        let code_context = is_template_code_context(&context)
            || info.is_some_and(|i| {
                CODE_KEYS.contains(&i.name.as_str()) || CODE_FUNCTIONS.contains(&i.name.as_str())
            });
        let forced = if code_context {
            None
        } else {
            let has_space = static_regex(&SPACE_CELL, r"\s").is_match(&visible);
            let has_letter = static_regex(&LETTER_CELL, r"[A-Za-zÀ-ÿ]").is_match(&visible);
            if has_space && has_letter {
                Some(true)
            } else {
                None
            }
        };
        self.emit_string(&text, line, None, true, forced);
    }

    /// Le nome de tag JSX e emite como codigo.
    fn read_tag_name(&mut self) -> String {
        let line = self.line;
        let start = self.index;
        while !self.eof()
            && matches!(
                self.peek(0).unwrap_or('\0'),
                'A'..='Z' | 'a'..='z' | '0'..='9' | '_' | '$' | '.' | ':' | '-'
            )
        {
            self.advance(1);
        }
        let name: String = self
            .chars
            .iter()
            .skip(start)
            .take(self.index - start)
            .collect();
        self.emit_code(&name, Some(line));
        name
    }

    /// Elemento JSX (`<Tag ...>filhos</Tag>` ou fragmento).
    fn parse_jsx_element(&mut self) {
        self.emit_code("<", None);
        self.advance(1);
        if self.peek(0) == Some('>') {
            self.emit_code(">", None);
            self.advance(1);
            self.parse_jsx_children("");
            return;
        }
        let name = self.read_tag_name();
        if !self.parse_jsx_attributes(&name) {
            self.parse_jsx_children(&name);
        }
    }

    /// Atributos JSX; devolve `true` se autofechado (`/>`).
    fn parse_jsx_attributes(&mut self, tag_name: &str) -> bool {
        let mut attribute: Option<String> = None;
        while !self.eof() {
            let char = self.peek(0).unwrap_or('\0');
            if char.is_whitespace() {
                self.emit_code(" ", None);
                self.advance(1);
                continue;
            }
            if char == '/' && self.peek(1) == Some('>') {
                self.emit_code("/>", None);
                self.advance(2);
                return true;
            }
            if char == '>' {
                self.emit_code(">", None);
                self.advance(1);
                return false;
            }
            if char == '{' {
                let attr = attribute.clone();
                self.parse_js_expression("attribute", attr.as_deref(), Some(tag_name));
                attribute = None;
                continue;
            }
            if char == '\'' || char == '"' {
                let attr = attribute.clone();
                self.read_js_string(char, attr.as_deref());
                continue;
            }
            if char == '=' {
                self.emit_code("=", None);
                self.advance(1);
                continue;
            }
            let start = self.index;
            while !self.eof()
                && matches!(
                    self.peek(0).unwrap_or('\0'),
                    'A'..='Z' | 'a'..='z' | '0'..='9' | '_' | '$' | ':' | '@' | '.' | '-'
                )
            {
                self.advance(1);
            }
            if self.index == start {
                let mut buf = String::new();
                buf.push(char);
                self.emit_code(&buf, None);
                self.advance(1);
                continue;
            }
            let name: String = self
                .chars
                .iter()
                .skip(start)
                .take(self.index - start)
                .collect();
            attribute = Some(name.clone());
            self.emit_code(&name, None);
        }
        false
    }

    /// Filhos JSX (texto, elementos e `{...}`).
    fn parse_jsx_children(&mut self, tag_name: &str) {
        let tag = tag_name.to_string();
        while !self.eof() {
            if self.peek(0) == Some('<') && self.peek(1) == Some('/') {
                self.parse_jsx_closing_tag();
                return;
            }
            if self.peek(0) == Some('<')
                && (self.peek(1) == Some('>')
                    || matches!(self.peek(1).unwrap_or('\0'), 'A'..='Z' | 'a'..='z' | '_' | '$'))
            {
                self.parse_jsx_element();
                continue;
            }
            if self.peek(0) == Some('{') {
                self.parse_js_expression("child", None, Some(&tag));
                continue;
            }
            let line = self.line;
            let start = self.index;
            while !self.eof() {
                if self.peek(0) == Some('{') {
                    break;
                }
                if self.peek(0) == Some('<')
                    && (self.peek(1) == Some('/')
                        || self.peek(1) == Some('>')
                        || matches!(
                            self.peek(1).unwrap_or('\0'),
                            'A'..='Z' | 'a'..='z' | '_' | '$'
                        ))
                {
                    break;
                }
                self.advance(1);
            }
            let text: String = self
                .chars
                .iter()
                .skip(start)
                .take(self.index - start)
                .collect();
            if !text.trim().is_empty() {
                self.emit_text(&text, Some(line), &tag);
            }
        }
    }

    /// Tag de fechamento JSX (`</Tag>`).
    fn parse_jsx_closing_tag(&mut self) {
        self.emit_code("</", None);
        self.advance(2);
        if self.peek(0) == Some('>') {
            self.emit_code(">", None);
            self.advance(1);
            return;
        }
        self.read_tag_name();
        while !self.eof() && self.peek(0) != Some('>') {
            let char = self.peek(0).unwrap_or('\0');
            let mut buf = String::new();
            buf.push(char);
            self.emit_code(&buf, None);
            self.advance(1);
        }
        if self.peek(0) == Some('>') {
            self.emit_code(">", None);
            self.advance(1);
        }
    }

    /// Expressao `{...}` com contexto JSX na pilha.
    fn parse_js_expression(&mut self, kind: &str, attribute: Option<&str>, tag: Option<&str>) {
        self.emit_code("{", None);
        self.advance(1);
        self.jsx_contexts
            .push(JsxContext::new(kind, attribute, tag));
        self.scan_js_code(true, true);
        self.jsx_contexts.pop();
    }

    /// Arquivo PHP (alterna HTML e blocos `<?php`).
    fn scan_php_file(&mut self) {
        let full: String = self.chars.iter().collect();
        let mut in_php = {
            static CELL: OnceLock<Regex> = OnceLock::new();
            static_regex(&CELL, r"(?i)^\s*<\?(?:php|=)").is_match(&full)
        };
        while !self.eof() {
            if !in_php {
                self.scan_html_region(false, true);
                if self.eof() {
                    break;
                }
                if self.starts_with("<?php") {
                    self.emit_code("<?php", None);
                    self.advance(5);
                } else if self.starts_with("<?=") {
                    self.emit_code("<?=", None);
                    self.advance(3);
                }
                in_php = true;
                continue;
            }
            if !self.scan_php_code(true, false) {
                break;
            }
            in_php = false;
        }
    }

    /// Codigo PHP ate `?>` ou `@endphp`.
    fn scan_php_code(&mut self, stop_at_close_tag: bool, stop_at_endphp: bool) -> bool {
        while !self.eof() {
            if stop_at_endphp && self.starts_with("@endphp") {
                self.emit_code("@endphp", None);
                self.advance(7);
                return true;
            }
            if stop_at_close_tag && self.peek(0) == Some('?') && self.peek(1) == Some('>') {
                self.emit_code("?>", None);
                self.advance(2);
                return true;
            }
            let char = self.peek(0).unwrap_or('\0');
            if char == '/' && self.peek(1) == Some('/') {
                self.read_php_line_comment(2, "php-slash-line");
                continue;
            }
            if char == '#' && self.peek(1) != Some('[') {
                self.read_php_line_comment(1, "php-hash-line");
                continue;
            }
            if char == '/' && self.peek(1) == Some('*') {
                self.read_block_comment("/*", "*/", "php-block");
                continue;
            }
            if char == '\'' || char == '"' {
                self.read_php_string(char, true);
                continue;
            }
            if char == '`' {
                self.read_php_string(char, false);
                continue;
            }
            if self.starts_with("<<<") && self.read_heredoc() {
                continue;
            }
            let mut buf = String::new();
            buf.push(char);
            self.emit_code(&buf, None);
            self.advance(1);
        }
        false
    }

    /// String PHP; gera pseudo `KEY(k)` antes de `=>` e coleta `lang_keys`.
    fn read_php_string(&mut self, quote: char, detect_array_key: bool) {
        let line = self.line;
        self.advance(1);
        let start = self.index;
        while !self.eof() {
            let char = self.peek(0).unwrap_or('\0');
            if char == '\\' {
                self.advance(2);
                continue;
            }
            if char == quote {
                break;
            }
            self.advance(1);
        }
        let end = self.index;
        let text: String = self.chars.iter().skip(start).take(end - start).collect();
        if self.peek(0) == Some(quote) {
            self.advance(1);
        }
        if detect_array_key {
            let rest = self.rest_from(self.index);
            static CELL: OnceLock<Regex> = OnceLock::new();
            if static_regex(&CELL, r"^\s*=>").is_match(&rest) {
                self.emit_code(&format!("KEY({})", text.trim()), Some(line));
                return;
            }
        }
        static LANG_CELL: OnceLock<Regex> = OnceLock::new();
        if static_regex(
            &LANG_CELL,
            r"(?:\b__|\btrans(?:_choice)?|Lang::get)\s*\(\s*$",
        )
        .is_match(&self.code_tail)
        {
            self.lang_keys.push(LangKey {
                key: text.clone(),
                line,
            });
        }
        self.emit_string(&text, line, None, false, None);
    }

    /// Heredoc `<<<ID ... ID`; devolve `false` se nao for heredoc valido.
    fn read_heredoc(&mut self) -> bool {
        let line = self.line;
        let rest = self.rest_from(self.index);
        static OPEN_CELL: OnceLock<Regex> = OnceLock::new();
        let open_re = static_regex(
            &OPEN_CELL,
            r#"<<<[ \t]*(?:'([A-Za-z_][A-Za-z0-9_]*)'|"([A-Za-z_][A-Za-z0-9_]*)"|([A-Za-z_][A-Za-z0-9_]*))[ \t]*\n"#,
        );
        let Some(caps) = open_re.captures(&rest) else {
            return false;
        };
        if caps.get(0).map_or(0, |m| m.start()) != 0 {
            return false;
        }
        let identifier = caps
            .get(1)
            .or_else(|| caps.get(2))
            .or_else(|| caps.get(3))
            .map_or("", |m| m.as_str())
            .to_string();
        let whole_bytes = caps.get(0).map_or(0, |m| m.end());
        let whole_chars: usize = rest.get(..whole_bytes).unwrap_or("").chars().count();
        self.advance(whole_chars);
        let start = self.index;
        let after = self.rest_from(start);
        let closer = Regex::new(&format!(
            r"(?m)^[ \t]*{}[ \t]*;?[ \t]*(?://.*|#.*)?$",
            regex::escape(&identifier)
        ))
        .ok();
        let (content_end, advance_count) = closer
            .as_ref()
            .and_then(|re| re.find(&after))
            .map_or((after.len(), after.len()), |m| {
                (m.start(), m.start() + m.len())
            });
        let content_end_chars: usize = after.get(..content_end).unwrap_or("").chars().count();
        let advance_chars: usize = after.get(..advance_count).unwrap_or("").chars().count();
        // Consome so o conteudo; a linha de fechamento fica para o fluxo.
        let text: String = self
            .chars
            .iter()
            .skip(start)
            .take(content_end_chars)
            .collect();
        self.advance(content_end_chars);
        let _ = advance_chars;
        // Paridade: o Python avanca conteudo + linha de fechamento e emite a
        // string; aqui avancamos o conteudo e a linha de fechamento quando
        // houver match, sem emitir a linha como codigo.
        if closer.as_ref().and_then(|re| re.find(&after)).is_some() {
            // Linha de fechamento: consome ate o fim da linha (sem o `\n`).
            let tail: String = self.chars.iter().skip(self.index).collect();
            let line_len: usize = tail.find('\n').unwrap_or(tail.len());
            let line_chars: usize = tail.get(..line_len).unwrap_or("").chars().count();
            self.advance(line_chars);
        }
        self.emit_string(&text, line, None, true, None);
        true
    }

    /// Regiao HTML/Blade (`blade`, `stop_at_php` como no Python).
    fn scan_html_region(&mut self, blade: bool, stop_at_php: bool) {
        while !self.eof() {
            if stop_at_php && self.starts_with("<?php") {
                return;
            }
            if stop_at_php && self.starts_with("<?=") {
                return;
            }
            if blade && self.starts_with("{{--") {
                self.read_block_comment("{{--", "--}}", "blade");
                continue;
            }
            if self.starts_with("<!--") {
                self.read_block_comment("<!--", "-->", "html");
                continue;
            }
            if self.starts_with("<?php") {
                self.emit_code("<?php", None);
                self.advance(5);
                self.scan_php_code(true, false);
                continue;
            }
            if self.starts_with("<?=") {
                self.emit_code("<?=", None);
                self.advance(3);
                self.scan_php_code(true, false);
                continue;
            }
            if blade {
                let rest = self.rest_from(self.index);
                static PHPCELL: OnceLock<Regex> = OnceLock::new();
                if static_regex(&PHPCELL, r"^@php\b").is_match(&rest) {
                    self.emit_code("@php", None);
                    self.advance(4);
                    self.scan_php_code(false, true);
                    continue;
                }
            }
            if blade && self.starts_with("{{") {
                self.read_blade_echo("{{", "}}");
                continue;
            }
            if blade && self.starts_with("{!!") {
                self.read_blade_echo("{!!", "!!}");
                continue;
            }
            if blade
                && self.peek(0) == Some('@')
                && matches!(self.peek(1).unwrap_or('\0'), 'A'..='Z' | 'a'..='z' | '_')
            {
                self.read_blade_directive();
                continue;
            }
            if self.peek(0) == Some('<')
                && (self.peek(1) == Some('!')
                    || self.peek(1) == Some('/')
                    || self.peek(1).unwrap_or('\0').is_ascii_alphabetic())
            {
                self.parse_html_tag(blade);
                continue;
            }
            let line = self.line;
            let start = self.index;
            while !self.eof() {
                if self.peek(0) == Some('<') {
                    break;
                }
                if self.starts_with("<!--") {
                    break;
                }
                if blade && self.starts_with("{{--") {
                    break;
                }
                if blade && (self.starts_with("{{") || self.starts_with("{!!")) {
                    break;
                }
                if blade
                    && self.peek(0) == Some('@')
                    && matches!(self.peek(1).unwrap_or('\0'), 'A'..='Z' | 'a'..='z' | '_')
                {
                    break;
                }
                self.advance(1);
            }
            let text: String = self
                .chars
                .iter()
                .skip(start)
                .take(self.index - start)
                .collect();
            if !text.trim().is_empty() {
                let tag = self.current_html_tag.clone();
                self.emit_text(&text, Some(line), &tag);
            }
            if self.index == start {
                let char = self.peek(0).unwrap_or('\0');
                let mut buf = String::new();
                buf.push(char);
                self.emit_code(&buf, None);
                self.advance(1);
            }
        }
    }

    /// Eco Blade `{{...}}` / `{!!...!!}` com coleta de `lang_keys`.
    fn read_blade_echo(&mut self, opener: &str, closer: &str) {
        let line = self.line;
        self.advance(opener.chars().count());
        let start = self.index;
        let mut quote: Option<char> = None;
        while !self.eof() {
            let char = self.peek(0).unwrap_or('\0');
            if let Some(q) = quote {
                if char == '\\' {
                    self.advance(2);
                    continue;
                }
                if char == q {
                    quote = None;
                }
                self.advance(1);
                continue;
            }
            if char == '\'' || char == '"' {
                quote = Some(char);
                self.advance(1);
                continue;
            }
            if self.starts_with(closer) {
                break;
            }
            self.advance(1);
        }
        let inner: String = self
            .chars
            .iter()
            .skip(start)
            .take(self.index - start)
            .collect();
        if self.starts_with(closer) {
            self.advance(closer.chars().count());
        }
        for (key, key_line) in find_blade_lang_keys(&inner, line) {
            self.lang_keys.push(LangKey {
                key,
                line: key_line,
            });
        }
        self.emit_code(opener, Some(line));
        let mut cursor: usize = 0;
        for (span_start, span_end, value) in quoted_spans(&inner) {
            if span_start > cursor {
                let gap = inner.get(cursor..span_start).unwrap_or("");
                self.emit_code(gap, Some(line));
            }
            let offset = inner
                .get(..span_start)
                .unwrap_or("")
                .chars()
                .filter(|c| *c == '\n')
                .count() as u32;
            if text_helpers::is_human_text(&value) || starts_capital(value.trim_start()) {
                self.emit_string(&value, line + offset, None, false, Some(true));
            } else {
                let raw = inner.get(span_start..span_end).unwrap_or("");
                self.emit_code(raw, Some(line + offset));
            }
            cursor = span_end;
        }
        if cursor < inner.len() {
            let rest = inner.get(cursor..).unwrap_or("");
            self.emit_code(rest, Some(line));
        }
        self.emit_code(closer, Some(line));
    }

    /// Diretiva Blade (`@nome` + argumentos opcionais).
    fn read_blade_directive(&mut self) {
        let line = self.line;
        let start = self.index;
        self.advance(1);
        while !self.eof()
            && matches!(
                self.peek(0).unwrap_or('\0'),
                'A'..='Z' | 'a'..='z' | '0'..='9' | '_'
            )
        {
            self.advance(1);
        }
        let name: String = self
            .chars
            .iter()
            .skip(start + 1)
            .take(self.index - start - 1)
            .collect();
        let raw: String = self
            .chars
            .iter()
            .skip(start)
            .take(self.index - start)
            .collect();
        self.emit_code(&raw, Some(line));
        if self.peek(0) == Some('(') {
            self.read_blade_arguments(&name.to_lowercase());
        }
    }

    /// String entre aspas a partir do cursor (conteudo + linha).
    fn consume_quoted(&mut self, quote: char) -> (String, u32) {
        let line = self.line;
        self.advance(1);
        let start = self.index;
        while !self.eof() {
            let char = self.peek(0).unwrap_or('\0');
            if char == '\\' {
                self.advance(2);
                continue;
            }
            if char == quote {
                break;
            }
            self.advance(1);
        }
        let text: String = self
            .chars
            .iter()
            .skip(start)
            .take(self.index - start)
            .collect();
        if self.peek(0) == Some(quote) {
            self.advance(1);
        }
        (text, line)
    }

    /// Argumentos de diretiva Blade com as regras de cada grupo.
    fn read_blade_arguments(&mut self, name: &str) {
        let line = self.line;
        self.emit_code("(", Some(line));
        self.advance(1);
        let section_directives = ["section", "yield", "slot"];
        let data_directives = [
            "include",
            "includeif",
            "includewhen",
            "includeunless",
            "component",
            "componentfirst",
        ];
        let key_directives = ["lang", "choice"];
        let mut paren_depth: usize = 1;
        let mut bracket_depth: usize = 0;
        let mut argument_index: usize = 0;
        let mut pending_array_key: Option<String> = None;
        while !self.eof() {
            let char = self.peek(0).unwrap_or('\0');
            if char == '(' {
                paren_depth += 1;
                self.emit_code("(", None);
                self.advance(1);
                continue;
            }
            if char == ')' {
                paren_depth = paren_depth.saturating_sub(1);
                self.emit_code(")", None);
                self.advance(1);
                if paren_depth == 0 {
                    return;
                }
                continue;
            }
            if char == '[' {
                bracket_depth += 1;
                self.emit_code("[", None);
                self.advance(1);
                continue;
            }
            if char == ']' {
                bracket_depth = bracket_depth.saturating_sub(1);
                self.emit_code("]", None);
                self.advance(1);
                pending_array_key = None;
                continue;
            }
            if char == ',' {
                if paren_depth == 1 && bracket_depth == 0 {
                    argument_index += 1;
                    pending_array_key = None;
                }
                self.emit_code(",", None);
                self.advance(1);
                continue;
            }
            if char == '=' && self.peek(1) == Some('>') {
                self.emit_code("=>", None);
                self.advance(2);
                continue;
            }
            if char == '/' && self.peek(1) == Some('/') {
                self.read_line_comment(2, "blade-line");
                continue;
            }
            if char == '#' && self.peek(1) != Some('[') {
                self.read_line_comment(1, "blade-line");
                continue;
            }
            if char == '/' && self.peek(1) == Some('*') {
                self.read_block_comment("/*", "*/", "blade-block");
                continue;
            }
            if char == '\'' || char == '"' {
                let (string_text, string_line) = self.consume_quoted(char);
                let rest = self.rest_from(self.index);
                static ARROW_CELL: OnceLock<Regex> = OnceLock::new();
                let is_arrow = static_regex(&ARROW_CELL, r"^\s*=>").is_match(&rest);
                let mut label = false;
                if section_directives.contains(&name)
                    && argument_index >= 1
                    && bracket_depth == 0
                    && paren_depth == 1
                {
                    label = true;
                } else if data_directives.contains(&name) && bracket_depth > 0 && paren_depth == 1 {
                    if is_arrow {
                        pending_array_key = Some(string_text.clone());
                    } else if pending_array_key
                        .as_deref()
                        .is_some_and(|k| LABEL_KEYS.contains(&k))
                    {
                        label = true;
                    }
                    if !is_arrow {
                        pending_array_key = None;
                    }
                } else if key_directives.contains(&name) && argument_index == 0 {
                    self.lang_keys.push(LangKey {
                        key: string_text.clone(),
                        line: string_line,
                    });
                }
                if label {
                    self.emit_string(&string_text, string_line, None, false, Some(true));
                } else {
                    let token = code_string_token(&string_text);
                    self.emit_code(&token, Some(string_line));
                }
                continue;
            }
            let mut buf = String::new();
            buf.push(char);
            self.emit_code(&buf, None);
            self.advance(1);
        }
    }

    /// Tag HTML com atributos, ecos e diretivas embutidas.
    fn parse_html_tag(&mut self, blade: bool) {
        let line = self.line;
        self.emit_code("<", Some(line));
        self.advance(1);
        if self.peek(0) == Some('!') || self.peek(0) == Some('?') {
            while !self.eof() && self.peek(0) != Some('>') {
                let char = self.peek(0).unwrap_or('\0');
                let mut buf = String::new();
                buf.push(char);
                self.emit_code(&buf, None);
                self.advance(1);
            }
            if self.peek(0) == Some('>') {
                self.emit_code(">", None);
                self.advance(1);
            }
            return;
        }
        let mut closing = false;
        if self.peek(0) == Some('/') {
            self.emit_code("/", None);
            self.advance(1);
            closing = true;
        }
        let name_line = self.line;
        let name_start = self.index;
        while !self.eof()
            && matches!(
                self.peek(0).unwrap_or('\0'),
                'A'..='Z' | 'a'..='z' | '0'..='9' | ':' | '_' | '-'
            )
        {
            self.advance(1);
        }
        let name: String = self
            .chars
            .iter()
            .skip(name_start)
            .take(self.index - name_start)
            .collect();
        let lowered = name.to_lowercase();
        self.emit_code(&name, Some(name_line));
        self.current_html_tag = if closing {
            String::new()
        } else {
            lowered.clone()
        };
        let mut attribute: Option<String> = None;
        let mut self_closing = false;
        while !self.eof() {
            let char = self.peek(0).unwrap_or('\0');
            if char.is_whitespace() {
                self.emit_code(" ", None);
                self.advance(1);
                continue;
            }
            if char == '/' && self.peek(1) == Some('>') {
                self.emit_code("/>", None);
                self.advance(2);
                self_closing = true;
                break;
            }
            if char == '>' {
                self.emit_code(">", None);
                self.advance(1);
                break;
            }
            if blade && self.starts_with("{{") {
                self.read_blade_echo("{{", "}}");
                continue;
            }
            if blade && self.starts_with("{!!") {
                self.read_blade_echo("{!!", "!!}");
                continue;
            }
            if blade
                && self.peek(0) == Some('@')
                && matches!(self.peek(1).unwrap_or('\0'), 'A'..='Z' | 'a'..='z' | '_')
            {
                self.read_blade_directive();
                continue;
            }
            if char == '\'' || char == '"' {
                let attr = attribute.clone();
                self.read_html_string(char, attr.as_deref());
                continue;
            }
            if char == '=' {
                self.emit_code("=", None);
                self.advance(1);
                continue;
            }
            let start = self.index;
            while !self.eof()
                && matches!(
                    self.peek(0).unwrap_or('\0'),
                    'A'..='Z' | 'a'..='z' | '0'..='9' | '_' | ':' | '@' | '.' | '-'
                )
            {
                self.advance(1);
            }
            if self.index == start {
                let mut buf = String::new();
                buf.push(char);
                self.emit_code(&buf, None);
                self.advance(1);
                continue;
            }
            let attr_name: String = self
                .chars
                .iter()
                .skip(start)
                .take(self.index - start)
                .collect();
            attribute = Some(attr_name.clone());
            self.emit_code(&attr_name, None);
        }
        if !closing && !self_closing && (lowered == "script" || lowered == "style") {
            let close_tag = format!("</{lowered}");
            let end =
                find_closing_tag(&self.chars, self.index, &close_tag).unwrap_or(self.chars.len());
            let content: String = self
                .chars
                .iter()
                .skip(self.index)
                .take(end - self.index)
                .collect();
            let content_line = self.line;
            if !content.is_empty() {
                let nested_kind = if lowered == "style" { "css" } else { "ts" };
                let mut nested = SourceScanner::new(&content, nested_kind, "", false, false);
                nested.scan();
                let offset = content_line.saturating_sub(1);
                for comment in &nested.comments {
                    self.comments.push(CommentRecord {
                        kind: comment.kind.clone(),
                        syntax: comment.syntax.clone(),
                        text: comment.text.clone(),
                        line: comment.line + offset,
                    });
                }
                for entry in render_skeleton(&nested.pieces) {
                    self.emit_code(&entry.text, Some(entry.line + offset));
                }
            }
            self.advance(end - self.index);
        }
    }

    /// String de atributo HTML.
    fn read_html_string(&mut self, quote: char, attribute: Option<&str>) {
        let line = self.line;
        self.advance(1);
        let start = self.index;
        while !self.eof() {
            let char = self.peek(0).unwrap_or('\0');
            if char == '\\' {
                self.advance(2);
                continue;
            }
            if char == quote {
                break;
            }
            self.advance(1);
        }
        let text: String = self
            .chars
            .iter()
            .skip(start)
            .take(self.index - start)
            .collect();
        if self.peek(0) == Some(quote) {
            self.advance(1);
        }
        self.emit_string(&text, line, attribute, false, None);
    }
}

/// Localiza `</tag` sem diferenciar maiusculas, em indices de `char`.
fn find_closing_tag(chars: &[char], from: usize, close_tag: &str) -> Option<usize> {
    let needle: Vec<char> = close_tag.chars().collect();
    if needle.is_empty() {
        return Some(from);
    }
    let mut i = from;
    while i + needle.len() <= chars.len() {
        let mut ok = true;
        for (k, n) in needle.iter().enumerate() {
            let Some(c) = chars.get(i + k) else {
                ok = false;
                break;
            };
            if c.to_lowercase().next() != n.to_lowercase().next() {
                ok = false;
                break;
            }
        }
        if ok {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Faixas de aspas com escape (`(['"])((?:\\.|(?!\1).)*)\1` sem backreference).
fn quoted_spans(inner: &str) -> Vec<(usize, usize, String)> {
    let chars: Vec<(usize, char)> = inner.char_indices().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let (byte_pos, c) = chars[i];
        if c == '\'' || c == '"' {
            let quote = c;
            let mut j = i + 1;
            let mut value = String::new();
            let mut closed = false;
            while j < chars.len() {
                let (_, d) = chars[j];
                if d == '\\' {
                    // Preserva o par escapado no valor bruto (paridade).
                    value.push(d);
                    if j + 1 < chars.len() {
                        value.push(chars[j + 1].1);
                        j += 2;
                    } else {
                        j += 1;
                    }
                    continue;
                }
                if d == quote {
                    closed = true;
                    break;
                }
                value.push(d);
                j += 1;
            }
            if closed {
                let end_byte = if j + 1 < chars.len() {
                    chars[j + 1].0
                } else {
                    inner.len()
                };
                out.push((byte_pos, end_byte, value));
                i = j + 1;
                continue;
            }
            i += 1;
            continue;
        }
        i += 1;
    }
    out
}

/// Chaves `__`/`trans`/`trans_choice`/`Lang::get` dentro de eco Blade.
fn find_blade_lang_keys(inner: &str, base_line: u32) -> Vec<(String, u32)> {
    static CELL: OnceLock<Regex> = OnceLock::new();
    let re = static_regex(
        &CELL,
        r#"\b(?:__|trans_choice|trans|Lang::get)\s*\(\s*(['"])"#,
    );
    let mut out = Vec::new();
    for caps in re.captures_iter(inner) {
        let quote = caps
            .get(1)
            .map_or('\0', |m| m.as_str().chars().next().unwrap_or('\0'));
        let cursor_byte = caps.get(1).map_or(0, |m| m.end());
        let tail = inner.get(cursor_byte..).unwrap_or("");
        let tail_chars: Vec<(usize, char)> = tail.char_indices().collect();
        let mut value = String::new();
        let mut closed = false;
        let mut j = 0;
        while j < tail_chars.len() {
            let (_, c) = tail_chars[j];
            if c == '\\' {
                value.push(c);
                if j + 1 < tail_chars.len() {
                    value.push(tail_chars[j + 1].1);
                    j += 2;
                } else {
                    j += 1;
                }
                continue;
            }
            if c == quote {
                closed = true;
                break;
            }
            value.push(c);
            j += 1;
        }
        if closed {
            let start = caps.get(0).map_or(0, |m| m.start());
            let offset = inner
                .get(..start)
                .unwrap_or("")
                .chars()
                .filter(|c| *c == '\n')
                .count() as u32;
            // Desescapa o par minimo `\"`/`\'`/`\\` como o Python (fatia crua).
            let unescaped = value.replace("\\\\", "\\");
            out.push((unescaped, base_line + offset));
        }
    }
    out
}

/// Normaliza texto de comentario (remove `*`, colapsa espacos).
pub fn normalize_comment_text(text: &str) -> String {
    static STAR_CELL: OnceLock<Regex> = OnceLock::new();
    static TRAIL_CELL: OnceLock<Regex> = OnceLock::new();
    static SPACE_CELL: OnceLock<Regex> = OnceLock::new();
    let star = static_regex(&STAR_CELL, r"^\s*\*+\s?");
    let trail = static_regex(&TRAIL_CELL, r"\s+$");
    let space = static_regex(&SPACE_CELL, r"[ \t]+");
    let mut cleaned: Vec<String> = Vec::new();
    for raw in text.split('\n') {
        let mut stripped = star.replace(raw, "").into_owned();
        stripped = trail.replace(&stripped, "").into_owned();
        stripped = stripped.trim_start().to_string();
        if !stripped.is_empty() {
            cleaned.push(stripped);
        }
    }
    space
        .replace_all(&cleaned.join("\n"), " ")
        .trim()
        .to_string()
}

/// Normaliza codigo (colapsa espacos).
pub fn normalize_code(text: &str) -> String {
    static CELL: OnceLock<Regex> = OnceLock::new();
    static_regex(&CELL, r"\s+")
        .replace_all(text, " ")
        .trim()
        .to_string()
}

/// Funde `TEXT{""}TEXT` em `TEXT` ate estabilizar.
pub fn merge_inline_label_strings(text: &str) -> String {
    let mut output = text.to_string();
    loop {
        let replaced = output.replace("TEXT{\"\"}TEXT", "TEXT");
        if replaced == output {
            return output;
        }
        output = replaced;
    }
}

/// Agrupa pecas por linha no esqueleto (`render_skeleton`).
pub fn render_skeleton(pieces: &[Piece]) -> Vec<SkeletonLine> {
    let mut buckets: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for piece in pieces {
        match piece {
            Piece::Code(p) => {
                let parts: Vec<&str> = p.text.split('\n').collect();
                for (offset, part) in parts.iter().enumerate() {
                    buckets
                        .entry(p.line + offset as u32)
                        .or_default()
                        .push((*part).to_string());
                }
            }
            Piece::Str(p) => {
                let token = if p.label {
                    "\"\"".to_string()
                } else {
                    code_string_token(&p.text)
                };
                buckets.entry(p.line).or_default().push(token);
            }
            Piece::Text(_) => {
                buckets
                    .entry(piece.line())
                    .or_default()
                    .push("TEXT".to_string());
            }
        }
    }
    let mut entries = Vec::new();
    for (line, parts) in buckets {
        let merged = merge_inline_label_strings(&normalize_code(&parts.concat()));
        if !merged.is_empty() {
            entries.push(SkeletonLine { line, text: merged });
        }
    }
    entries
}

/// Varre a fonte e devolve comentarios, esqueleto, labels e chaves.
pub fn scan_source(
    text: &str,
    kind: &str,
    file_name: &str,
    lang_file: bool,
    test_file: bool,
) -> ScanResult {
    let inferred =
        lang_file || kind.contains("resources/lang/en/") || kind.contains("resources/lang/pt-BR/");
    let mut scanner = SourceScanner::new(text, kind, file_name, inferred, test_file);
    scanner.scan();
    let comments = scanner
        .comments
        .iter()
        .map(|c| CommentRecord {
            kind: c.kind.clone(),
            syntax: c.syntax.clone(),
            text: normalize_comment_text(&c.text),
            line: c.line,
        })
        .collect::<Vec<_>>();
    let lines = render_skeleton(&scanner.pieces);
    let labels = scanner
        .pieces
        .iter()
        .filter(|p| match p {
            Piece::Str(s) => s.label,
            Piece::Text(_) => true,
            Piece::Code(_) => false,
        })
        .cloned()
        .collect();
    ScanResult {
        kind: scanner.kind.as_str().to_string(),
        comments,
        lines,
        labels,
        lang_keys: scanner.lang_keys.clone(),
    }
}

/// Extrai so os comentarios (`extract_comments`).
pub fn extract_comments(text: &str, kind: &str) -> Vec<CommentRecord> {
    scan_source(text, kind, "", false, false).comments
}

/// Remove mensagens yup deixando so a assinatura (`normalize_yup_label_args`).
///
/// So aspas simples sao normalizadas, como no Python (aspas duplas passam).
pub fn normalize_yup_label_args(text: &str) -> String {
    static MIN_CELL: OnceLock<Regex> = OnceLock::new();
    static EMAIL_CELL: OnceLock<Regex> = OnceLock::new();
    static ONE_CELL: OnceLock<Regex> = OnceLock::new();
    static REQ_CELL: OnceLock<Regex> = OnceLock::new();
    let step = static_regex(&MIN_CELL, r"\.(min|max)\(([^,]+),\s*'[^']*'\)")
        .replace_all(text, ".$1($2)")
        .into_owned();
    let step = static_regex(&EMAIL_CELL, r"\.email\('[^']*'\)")
        .replace_all(&step, ".email()")
        .into_owned();
    let step = static_regex(&ONE_CELL, r"\.oneOf\((\[[^\]]*\]),\s*'[^']*'\)")
        .replace_all(&step, ".oneOf($1)")
        .into_owned();
    static_regex(&REQ_CELL, r"\.required\('[^']*'\)")
        .replace_all(&step, ".required()")
        .into_owned()
}

/// Apaga labels dentro de sentencas `echo ...;` (`normalize_blade_echo_labels`).
///
/// Implementa o comportamento pretendido do Python sem o `TypeError`
/// (`scanner.py` linhas 1274-1286): strings com letra e (espaco ou
/// maiuscula inicial) viram `""`.
pub fn normalize_blade_echo_labels(text: &str) -> String {
    static ECHO_CELL: OnceLock<Regex> = OnceLock::new();
    static LETTER_CELL: OnceLock<Regex> = OnceLock::new();
    static SPACE_CELL: OnceLock<Regex> = OnceLock::new();
    static CAP_CELL: OnceLock<Regex> = OnceLock::new();
    let echo_re = static_regex(&ECHO_CELL, r"\becho\s+[^;]+;");
    let letter = static_regex(&LETTER_CELL, r"[A-Za-zÀ-ÿ]");
    let space = static_regex(&SPACE_CELL, r"\s");
    let cap = static_regex(&CAP_CELL, r"^[A-ZÀ-Ý]");
    let mut out = String::new();
    let mut last = 0;
    for m in echo_re.find_iter(text) {
        out.push_str(text.get(last..m.start()).unwrap_or(""));
        let statement = m.as_str();
        let mut rebuilt = String::new();
        let mut cursor = 0;
        for (start, end, _) in quoted_spans(statement) {
            rebuilt.push_str(statement.get(cursor..start).unwrap_or(""));
            let raw = statement.get(start..end).unwrap_or("");
            let inner_len = raw.len().saturating_sub(2);
            let value = raw.get(1..1 + inner_len).unwrap_or("");
            if letter.is_match(value) && (space.is_match(value) || cap.is_match(value.trim_start()))
            {
                rebuilt.push_str("\"\"");
            } else {
                rebuilt.push_str(raw);
            }
            cursor = end;
        }
        rebuilt.push_str(statement.get(cursor..).unwrap_or(""));
        out.push_str(&rebuilt);
        last = m.end();
    }
    out.push_str(text.get(last..).unwrap_or(""));
    out
}

/// Esqueleto normalizado (`skeleton`, com yup + categoryLabels + blade-echo).
pub fn skeleton(text: &str, kind: &str) -> String {
    let resolved = normalize_kind(Some(kind));
    let source = if resolved == SourceKind::Blade {
        normalize_blade_echo_labels(text)
    } else {
        normalize_yup_label_args(text)
    };
    let normalized = normalize_category_labels_for_skeleton(&source);
    scan_source(&normalized, resolved.as_str(), "", false, false)
        .lines
        .iter()
        .map(|entry| entry.text.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Esqueleto por linha (`skeleton_lines`).
pub fn skeleton_lines(text: &str, kind: &str) -> Vec<SkeletonLine> {
    let normalized = normalize_category_labels_for_skeleton(text);
    scan_source(&normalized, kind, "", false, false).lines
}

/// Entrada do mapa `categoryLabels`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryEntry {
    /// Chave (ex.: `servers`).
    pub key: String,
    /// Valor (label).
    pub value: String,
    /// Linha (1-based).
    pub line: u32,
}

/// Bloco `categoryLabels` localizado (`extract_category_labels`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryLabels {
    /// Inicio em caracteres.
    pub start: usize,
    /// Fim em caracteres.
    pub end: usize,
    /// Linha do inicio (1-based).
    pub line: u32,
    /// Tipo declarado (`Record<...>` ou vazio).
    pub type_name: String,
    /// Chaves na ordem.
    pub keys: Vec<String>,
    /// Entradas com linha.
    pub entries: Vec<CategoryEntry>,
    /// Bloco bruto.
    pub raw_block: String,
}

/// Linha (1-based) do indice em caracteres (`line_at`).
pub fn line_at(text: &str, index: usize) -> u32 {
    let chars: Vec<char> = text.chars().collect();
    let end = index.min(chars.len());
    1 + chars.iter().take(end).filter(|c| **c == '\n').count() as u32
}

/// Apaga preservando `\n` (`blank_preserving_newlines`, indices em chars).
pub fn blank_preserving_newlines(text: &str, start: usize, end: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let from = start.min(len);
    let to = end.min(len).max(from);
    let mut out = String::new();
    for c in chars.iter().take(from) {
        out.push(*c);
    }
    for c in chars.iter().skip(from).take(to - from) {
        out.push(if *c == '\n' { '\n' } else { ' ' });
    }
    for c in chars.iter().skip(to) {
        out.push(*c);
    }
    out
}

/// Localiza `const categoryLabels = {...};` (`extract_category_labels`).
pub fn extract_category_labels(text: &str) -> Option<CategoryLabels> {
    static CELL: OnceLock<Regex> = OnceLock::new();
    let re = static_regex(
        &CELL,
        r"(^|\n)([ \t]*)(?:export[ \t]+)?const[ \t]+categoryLabels(?:[ \t]*:([^=\n]+?))?[ \t]*=[ \t]*\{([\s\S]*?)\n[ \t]*\};\n?",
    );
    let caps = re.captures(text)?;
    let whole = caps.get(0).map_or("", |m| m.as_str());
    let group1_len: usize = caps.get(1).map_or("", |m| m.as_str()).chars().count();
    // Byte offsets do regex -> converte para indices em chars.
    let match_start_byte = caps.get(0).map_or(0, |m| m.start());
    let match_end_byte = caps.get(0).map_or(0, |m| m.end());
    let to_chars = |bytes: usize| text.get(..bytes).unwrap_or("").chars().count();
    let match_start = to_chars(match_start_byte);
    let match_end = to_chars(match_end_byte);
    let start = match_start + group1_len;
    let declared = caps.get(3).map_or("", |m| m.as_str());
    static SPACE_CELL: OnceLock<Regex> = OnceLock::new();
    let collapsed = static_regex(&SPACE_CELL, r"\s+")
        .replace_all(declared, " ")
        .trim()
        .to_string();
    let body = caps.get(4).map_or("", |m| m.as_str()).to_string();
    // `body_start`: primeira `{` apos o primeiro `=` (paridade com o Python).
    let equals = whole.find('=').unwrap_or(0);
    let brace = whole.get(equals..).unwrap_or("").find('{').unwrap_or(0);
    let body_start_byte = match_start_byte + equals + brace + 1;
    let body_start = to_chars(body_start_byte);
    let body_line = line_at(text, body_start);
    let mut entries = Vec::new();
    for (key, value, entry_start_chars) in parse_category_entries(&body) {
        let before_nl = body
            .chars()
            .take(entry_start_chars)
            .filter(|c| *c == '\n')
            .count() as u32;
        entries.push(CategoryEntry {
            key,
            value,
            line: body_line + before_nl,
        });
    }
    let keys = entries.iter().map(|e| e.key.clone()).collect();
    Some(CategoryLabels {
        start,
        end: match_end,
        line: line_at(text, start),
        type_name: collapsed,
        keys,
        entries,
        raw_block: text
            .chars()
            .skip(start)
            .take(match_end.saturating_sub(start))
            .collect(),
    })
}

/// Entradas `chave: 'valor'` do corpo do mapa (equivale a `entry_pattern`).
fn parse_category_entries(body: &str) -> Vec<(String, String, usize)> {
    let chars: Vec<char> = body.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        // Pula espacos, quebras e virgulas entre entradas.
        while i < chars.len() && (chars[i].is_whitespace() || chars[i] == ',') {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }
        let entry_start = i;
        // Aspeamento opcional da chave com casamento (`(['\"]?)...\1`).
        let mut key_quote: Option<char> = None;
        if chars[i] == '\'' || chars[i] == '"' {
            key_quote = Some(chars[i]);
            i += 1;
        }
        let key_start = i;
        while i < chars.len() && matches!(chars[i], 'A'..='Z' | 'a'..='z' | '_' | '$' | '0'..='9') {
            i += 1;
        }
        if key_start == i {
            i = entry_start + 1;
            continue;
        }
        let key: String = chars[key_start..i].iter().collect();
        if let Some(q) = key_quote {
            if chars.get(i) != Some(&q) {
                i = entry_start + 1;
                continue;
            }
            i += 1;
        }
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if chars.get(i) != Some(&':') {
            i = entry_start + 1;
            continue;
        }
        i += 1;
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() || (chars[i] != '\'' && chars[i] != '"') {
            i = entry_start + 1;
            continue;
        }
        let quote = chars[i];
        i += 1;
        let mut value = String::new();
        let mut closed = false;
        while i < chars.len() {
            if chars[i] == '\\' {
                value.push(chars[i]);
                if i + 1 < chars.len() {
                    value.push(chars[i + 1]);
                    i += 2;
                } else {
                    i += 1;
                }
                continue;
            }
            if chars[i] == quote {
                closed = true;
                i += 1;
                break;
            }
            value.push(chars[i]);
            i += 1;
        }
        if !closed {
            i = entry_start + 1;
            continue;
        }
        // Virgula opcional apos o valor (parte do padrao original).
        let mut j = i;
        while j < chars.len() && chars[j].is_whitespace() {
            j += 1;
        }
        if chars.get(j) == Some(&',') {
            j += 1;
        }
        out.push((key, value, entry_start));
        i = j;
    }
    out
}

/// Canoniza acessos a `categoryLabels[category]` e variantes.
pub fn replace_category_lookups(text: &str) -> String {
    static A_CELL: OnceLock<Regex> = OnceLock::new();
    static B_CELL: OnceLock<Regex> = OnceLock::new();
    static C_CELL: OnceLock<Regex> = OnceLock::new();
    static D_CELL: OnceLock<Regex> = OnceLock::new();
    let step = static_regex(&A_CELL, r"\bcategoryLabels\s*\[\s*category\s*\]\s*!?")
        .replace_all(text, "CATEGORY_LABEL_LOOKUP")
        .into_owned();
    let step = static_regex(
        &B_CELL,
        r"\bcategory\s*\[\s*0\s*\]\s*!?\s*\.toUpperCase\s*\(\s*\)\s*\+\s*category\s*\.slice\s*\(\s*1\s*\)",
    )
    .replace_all(&step, "CATEGORY_LABEL_LOOKUP")
    .into_owned();
    let step = static_regex(
        &C_CELL,
        r"\bcategory\s*\.charAt\s*\(\s*0\s*\)\s*\.toUpperCase\s*\(\s*\)\s*\+\s*category\s*\.slice\s*\(\s*1\s*\)",
    )
    .replace_all(&step, "CATEGORY_LABEL_LOOKUP")
    .into_owned();
    static_regex(&D_CELL, r"\bcapitalize\s*\(\s*category\s*\)")
        .replace_all(&step, "CATEGORY_LABEL_LOOKUP")
        .into_owned()
}

/// Apaga o mapa e canoniza lookups antes do esqueleto.
pub fn normalize_category_labels_for_skeleton(text: &str) -> String {
    let found = extract_category_labels(text);
    let normalized = found.map_or_else(
        || text.to_string(),
        |block| blank_preserving_newlines(text, block.start, block.end),
    );
    replace_category_lookups(&normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels_text(result: &ScanResult) -> Vec<String> {
        result
            .labels
            .iter()
            .map(|p| match p {
                Piece::Str(s) => s.text.clone(),
                Piece::Text(t) => t.text.clone(),
                Piece::Code(c) => c.text.clone(),
            })
            .collect()
    }

    #[test]
    fn label_attributes_matrix_all_label() {
        for attr in LABEL_ATTRIBUTES {
            assert!(
                classify_string("Hello World", Some(attr), "x", false, false, None),
                "atributo {attr} deveria rotular"
            );
        }
    }

    #[test]
    fn code_keys_negatives_do_not_label() {
        for key in [
            "id",
            "key",
            "name",
            "type",
            "className",
            "href",
            "src",
            "route",
            "sql",
            "graphql",
            "to",
        ] {
            let context = format!("{key}: ");
            assert!(
                !classify_string("Hello World", None, &context, false, false, None),
                "chave {key} nao deveria rotular"
            );
            assert!(
                !classify_string("Hello World", Some(key), "x", false, false, None)
                    || LABEL_ATTRIBUTES.contains(&key),
                "atributo {key} nao deveria rotular"
            );
        }
    }

    #[test]
    fn label_component_props_matrix() {
        let positives = [
            ("<TableHeader ", "name"),
            ("<Money ", "suffix"),
            ("<ConfirmationModal ", "buttonText"),
            ("<PermissionRow ", "name"),
            ("<SuggestionCard ", "action"),
            ("<Box ", "recommended"),
            ("<SubNavigationLink ", "name"),
        ];
        for (context, attr) in positives {
            assert!(
                classify_string("Hello World", Some(attr), context, false, false, None),
                "{context} {attr} deveria rotular"
            );
        }
        assert!(!classify_string(
            "Hello World",
            Some("name"),
            "<Dialog ",
            false,
            false,
            None
        ));
        assert!(!classify_string(
            "Hello World",
            Some("name"),
            "<Box ",
            false,
            false,
            None
        ));
    }

    #[test]
    fn yup_message_extraction() {
        assert_eq!(normalize_yup_label_args(".min(5, 'Too short')"), ".min(5)");
        assert_eq!(
            normalize_yup_label_args(".required('Req').email('bad').oneOf(['a'], 'pick')"),
            ".required().email().oneOf(['a'])"
        );
        let result = scan_source(
            "yup.string().min(5, 'Too short').required('Required field')",
            "js",
            "",
            false,
            false,
        );
        let texts = labels_text(&result);
        assert!(texts.contains(&"Too short".to_string()));
        assert!(texts.contains(&"Required field".to_string()));
        assert_eq!(
            skeleton(
                "yup.string().email('Enter a valid email').required('Email is required')",
                "js"
            ),
            "yup.string().email().required()"
        );
    }

    #[test]
    fn category_labels_map_and_lookup() {
        let text = "export const categoryLabels = {\n servers: \"My Servers\",\n databases: \"Databases\",\n};\nconst t = categoryLabels[category];";
        let found = extract_category_labels(text).expect("mapa categoryLabels");
        assert_eq!(
            found.keys,
            vec!["servers".to_string(), "databases".to_string()]
        );
        assert_eq!(found.entries[0].value, "My Servers");
        assert_eq!(found.entries[0].line, 2);
        let normalized = normalize_category_labels_for_skeleton(text);
        assert!(normalized.contains("CATEGORY_LABEL_LOOKUP"));
        assert!(!normalized.contains("My Servers"));
        assert_eq!(
            skeleton(
                "export const categoryLabels: Record<string,string> = {\n  servers: \"My Servers\",\n};\nlabel(categoryLabels[category])",
                "js"
            ),
            "label(CATEGORY_LABEL_LOOKUP)"
        );
        assert_eq!(
            replace_category_lookups("a = capitalize(category)"),
            "a = CATEGORY_LABEL_LOOKUP"
        );
    }

    #[test]
    fn blade_section_include_lang() {
        let section = scan_source(
            "@section(\"title\", \"Hello World\")",
            "blade",
            "",
            false,
            false,
        );
        assert!(labels_text(&section).contains(&"Hello World".to_string()));
        assert_eq!(section.lines[0].text, "@section(\"title\", \"\")");

        let include = scan_source(
            "@include(\"partials.x\", [\"title\" => \"Hello World\", \"id\" => \"x\"])",
            "blade",
            "",
            false,
            false,
        );
        let texts = labels_text(&include);
        assert!(texts.contains(&"Hello World".to_string()));
        assert!(!texts.contains(&"x".to_string()));

        let lang = scan_source("@lang(\"greeting.hello\")", "blade", "", false, false);
        assert_eq!(lang.lang_keys.len(), 1);
        assert_eq!(lang.lang_keys[0].key, "greeting.hello");

        let choice = scan_source("@choice('messages.items', 2)", "blade", "", false, false);
        assert_eq!(choice.lang_keys[0].key, "messages.items");
    }

    #[test]
    fn test_assertion_gating() {
        assert!(classify_string(
            "Hello",
            None,
            "expect(getByText(",
            false,
            true,
            None
        ));
        assert!(!classify_string(
            "Hello World",
            None,
            "const x = ",
            false,
            true,
            None
        ));
        // Fora de arquivo de teste, `getByText(` sozinho nao rotula `Hello`.
        assert!(!classify_string(
            "Hello",
            None,
            "getByText(",
            false,
            false,
            None
        ));
    }

    #[test]
    fn lang_keys_for_magic_functions() {
        let result = scan_source(
            "<?php echo __('hello.world'); trans('a.b'); trans_choice('c.d', 2); Lang::get('e.f');",
            "php",
            "",
            false,
            false,
        );
        let keys: Vec<&str> = result.lang_keys.iter().map(|k| k.key.as_str()).collect();
        assert_eq!(keys, vec!["hello.world", "a.b", "c.d", "e.f"]);
    }

    #[test]
    fn skeleton_stability_reflow_pair() {
        // Refluxo so com espacos na mesma linha: esqueleto identico.
        assert_eq!(skeleton("const x = 1;", "js"), "const x = 1;");
        assert_eq!(skeleton("const   x   =   1;  ", "js"), "const x = 1;");
        // `TEXT{""}TEXT` na mesma linha funde em `TEXT` (paridade prettier).
        assert_eq!(
            skeleton("<p>Hello{'Hello World'}World</p>", "js"),
            "<p>TEXT</p>"
        );
        assert_eq!(
            skeleton("<div title=\"Hello World\">Hello World</div>", "html"),
            "<div title=\"\">TEXT</div>"
        );
    }

    #[test]
    fn jsx_child_vs_attribute_and_template_rules() {
        let child = JsxContext::new("child", None, Some("div"));
        assert!(classify_string(
            "Hello World",
            None,
            "",
            false,
            false,
            Some(&child)
        ));
        assert!(!classify_string(
            "api/v1/list",
            None,
            "",
            false,
            false,
            Some(&child)
        ));
        assert!(!classify_string(
            "/dashboard",
            None,
            "",
            false,
            false,
            Some(&child)
        ));
        // Template `styled` nao rotula; template comum com texto visivel rotula.
        let styled = scan_source("const x = styled`color: red;`;", "js", "", false, false);
        assert!(labels_text(&styled).is_empty());
        let plain = scan_source("const x = `Hello World`;", "js", "", false, false);
        assert!(labels_text(&plain).contains(&"Hello World".to_string()));
    }

    #[test]
    fn value_and_default_value_gates() {
        assert!(classify_string(
            "Hello World",
            Some("value"),
            "x",
            false,
            false,
            None
        ));
        assert!(!classify_string(
            "abc123",
            Some("value"),
            "x",
            false,
            false,
            None
        ));
        assert!(classify_string(
            "Save",
            Some("value"),
            "x",
            false,
            false,
            None
        ));
        assert!(!classify_string(
            "abc123",
            Some("id"),
            "x",
            false,
            false,
            None
        ));
    }

    #[test]
    fn comments_and_kinds() {
        assert_eq!(normalize_kind(Some("TS")), SourceKind::Ts);
        assert_eq!(normalize_kind(Some("blade.php")), SourceKind::Blade);
        assert_eq!(normalize_kind(None), SourceKind::Js);
        assert_eq!(source_kind(Some("a.blade.php")), SourceKind::Blade);
        assert_eq!(source_kind(Some("x.txt")), SourceKind::Plain);
        let comments = extract_comments("// hi\n/* there */ const x=1;", "js");
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].text, "hi");
    }

    #[test]
    fn context_identifier_matches_unicode_tails_like_python_w() {
        // (?<![.\w$]) do Python: `objeto.café:` nao e chave nos dois.
        assert!(context_identifier("objeto.café: ").is_none());
        let key = context_identifier("x café: ").expect("chave com cauda unicode");
        assert!(matches!(key.kind, ContextKind::Key));
        assert_eq!(key.name, "café");
        let call = context_identifier("até(").expect("chamada com cauda unicode");
        assert!(matches!(call.kind, ContextKind::Call));
        assert_eq!(call.name, "até");
    }
}
