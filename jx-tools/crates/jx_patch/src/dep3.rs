//! Cabecalhos DEP-3 de patch: parse e validacao tipada.
//!
//! Espelha as regras de `scripts/lint-patches.sh` e o gerador de
//! `scripts/dump-patch.sh`. Onde o shell aceita texto livre, aqui cada campo
//! tem um tipo fechado (`Origin`, `Forwarded`, `AppliedUpstream`, `IsoDate`).

use std::fmt;
use std::str::FromStr;

use crate::series::PatchNumber;

/// Erros de parse/validacao do patch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dep3Error {
    /// Sem linha `---` antes do primeiro `diff --git`.
    MissingTerminator,
    /// Campo obrigatorio ausente no cabecalho.
    MissingField(&'static str),
    /// Assinatura ausente: precisa de `Signed-off-by:` ou `Reviewed-by:`.
    MissingSignoff,
    /// Mais de uma linha `Subject:`.
    MultipleSubjects,
    /// `Subject:` vazio.
    EmptySubject,
    /// `Description:` sem nenhuma linha de texto.
    EmptyDescription,
    /// Valor de `Origin:` fora do vocabulario.
    InvalidOrigin(String),
    /// Valor de `Forwarded:` fora do vocabulario.
    InvalidForwarded(String),
    /// Valor de `Applied-Upstream:` fora do vocabulario.
    InvalidAppliedUpstream(String),
    /// `Last-Update:` fora do formato `YYYY-MM-DD` valido.
    InvalidLastUpdate(String),
    /// Corpo sem nenhuma linha `diff --git`.
    NoDiffBody,
}

impl fmt::Display for Dep3Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTerminator => write!(f, "no '---' terminator before the diff (DEP-3)"),
            Self::MissingField(field) => write!(f, "missing the '{field}' header field"),
            Self::MissingSignoff => write!(f, "missing the 'Signed-off-by:' field"),
            Self::MultipleSubjects => write!(f, "more than one Subject line"),
            Self::EmptySubject => write!(f, "empty Subject"),
            Self::EmptyDescription => write!(f, "empty Description"),
            Self::InvalidOrigin(value) => write!(f, "invalid Origin value: {value}"),
            Self::InvalidForwarded(value) => write!(f, "invalid Forwarded value: {value}"),
            Self::InvalidAppliedUpstream(value) => {
                write!(f, "invalid Applied-Upstream value: {value}")
            }
            Self::InvalidLastUpdate(value) => write!(
                f,
                "invalid Last-Update value (expected YYYY-MM-DD): {value}"
            ),
            Self::NoDiffBody => write!(f, "no git diff body"),
        }
    }
}

impl std::error::Error for Dep3Error {}

/// Origem DEP-3 do patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Origin {
    /// Mudanca vinda do upstream.
    Upstream,
    /// Backport de versao posterior.
    Backport,
    /// Mudanca propria do overlay.
    Vendor,
    /// Outra origem documentada.
    Other,
}

impl FromStr for Origin {
    type Err = Dep3Error;

    /// Aceita sufixo `, ...` apos o vocabulario, como a regex do lint.
    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let head = raw
            .split_once(',')
            .map(|(head, _)| head)
            .unwrap_or(raw)
            .trim();
        match head {
            "upstream" => Ok(Self::Upstream),
            "backport" => Ok(Self::Backport),
            "vendor" => Ok(Self::Vendor),
            "other" => Ok(Self::Other),
            _ => Err(Dep3Error::InvalidOrigin(raw.to_string())),
        }
    }
}

impl fmt::Display for Origin {
    /// Vocabulario DEP-3 em minusculas.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Upstream => "upstream",
            Self::Backport => "backport",
            Self::Vendor => "vendor",
            Self::Other => "other",
        };
        write!(f, "{text}")
    }
}

/// Estado de encaminhamento ao upstream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Forwarded {
    /// Nao sera encaminhado.
    No,
    /// Encaminhamento desnecessario (padrao do gerador).
    NotNeeded,
    /// Ja encaminhado.
    Yes,
    /// URL do encaminhamento.
    Url(String),
}

impl FromStr for Forwarded {
    type Err = Dep3Error;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let value = raw.trim();
        match value {
            "no" => Ok(Self::No),
            "not-needed" => Ok(Self::NotNeeded),
            "yes" => Ok(Self::Yes),
            _ if value.starts_with("http://") || value.starts_with("https://") => {
                Ok(Self::Url(value.to_string()))
            }
            _ => Err(Dep3Error::InvalidForwarded(raw.to_string())),
        }
    }
}

impl fmt::Display for Forwarded {
    /// Vocabulario DEP-3 (`not-needed` e o padrao do gerador).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::No => write!(f, "no"),
            Self::NotNeeded => write!(f, "not-needed"),
            Self::Yes => write!(f, "yes"),
            Self::Url(url) => write!(f, "{url}"),
        }
    }
}

/// Estado de aplicacao no upstream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppliedUpstream {
    /// Nao aplicado.
    No,
    /// Ja aplicado.
    Yes,
    /// Aplicado a partir da versao (ex.: `v4.0.8`).
    Version(String),
    /// Aplicado via commit (hash de 7 a 40 hex).
    Commit(String),
    /// Referencia por URL.
    Url(String),
}

impl FromStr for AppliedUpstream {
    type Err = Dep3Error;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let value = raw.trim();
        if value.starts_with("http://") || value.starts_with("https://") {
            return Ok(Self::Url(value.to_string()));
        }
        let head = value
            .split_once(',')
            .map(|(head, _)| head)
            .unwrap_or(value)
            .trim();
        match head {
            "no" => Ok(Self::No),
            "yes" => Ok(Self::Yes),
            _ => {
                if let Some(sha) = head.strip_prefix("commit:") {
                    if (7..=40).contains(&sha.len()) && sha.chars().all(|c| c.is_ascii_hexdigit()) {
                        return Ok(Self::Commit(sha.to_string()));
                    }
                } else if is_dotted_version(head) {
                    return Ok(Self::Version(head.to_string()));
                }
                Err(Dep3Error::InvalidAppliedUpstream(raw.to_string()))
            }
        }
    }
}

impl fmt::Display for AppliedUpstream {
    /// Forma canonica (`commit:<sha>`, versao ou URL).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::No => write!(f, "no"),
            Self::Yes => write!(f, "yes"),
            Self::Version(version) => write!(f, "{version}"),
            Self::Commit(sha) => write!(f, "commit:{sha}"),
            Self::Url(url) => write!(f, "{url}"),
        }
    }
}

/// `v1`, `v4.0.7` ou `4.0`: `v?` opcional + digitos separados por ponto.
fn is_dotted_version(head: &str) -> bool {
    let digits = head.strip_prefix('v').unwrap_or(head);
    if digits.is_empty() {
        return false;
    }
    digits
        .split('.')
        .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
}

/// Data de calendario `YYYY-MM-DD`.
///
/// Paridade consciente com o lint: valida formato e faixas de mes/dia, sem
/// checar dias de cada mes (o shell aceita `2026-02-30`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IsoDate {
    year: u16,
    month: u8,
    day: u8,
}

impl IsoDate {
    /// Ano.
    pub fn year(self) -> u16 {
        self.year
    }

    /// Mes (1-12).
    pub fn month(self) -> u8 {
        self.month
    }

    /// Dia (1-31).
    pub fn day(self) -> u8 {
        self.day
    }
}

impl FromStr for IsoDate {
    type Err = Dep3Error;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let invalid = || Dep3Error::InvalidLastUpdate(raw.to_string());
        let mut parts = raw.trim().split('-');
        let (year, month, day) = match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (Some(year), Some(month), Some(day), None) => (year, month, day),
            _ => return Err(invalid()),
        };
        if year.len() != 4 || month.len() != 2 || day.len() != 2 {
            return Err(invalid());
        }
        let year: u16 = year.parse().map_err(|_| invalid())?;
        let month: u8 = month.parse().map_err(|_| invalid())?;
        let day: u8 = day.parse().map_err(|_| invalid())?;
        if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
            return Err(invalid());
        }
        Ok(Self { year, month, day })
    }
}

impl fmt::Display for IsoDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// Assunto de uma linha do patch; nunca vazio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subject(String);

impl Subject {
    /// Constroi a partir do texto bruto; rejeita vazio.
    pub fn new(raw: &str) -> Result<Self, Dep3Error> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(Dep3Error::EmptySubject);
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Texto do assunto.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Subject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Assinatura do patch: `Signed-off-by` preferido, `Reviewed-by` aceito.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signoff {
    /// Linha `Signed-off-by:`.
    SignedOffBy(String),
    /// Linha `Reviewed-by:` (fallback aceito pelo lint).
    ReviewedBy(String),
}

impl Signoff {
    /// Texto da assinatura.
    pub fn as_str(&self) -> &str {
        match self {
            Self::SignedOffBy(value) | Self::ReviewedBy(value) => value,
        }
    }
}

/// Nome de arquivo de patch: `NNNN-kebab-case.patch`, com numero valido.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PatchFileName(String);

impl PatchFileName {
    /// Valida o nome base (apos a ultima `/`) contra o padrao do lint.
    pub fn from_path(path: &str) -> Result<Self, Dep3Error> {
        let base = path.rsplit('/').next().unwrap_or(path);
        let stem = base
            .strip_suffix(".patch")
            .ok_or_else(|| Dep3Error::InvalidOrigin(base.to_string()))?;
        if is_patch_stem(stem) {
            Ok(Self(base.to_string()))
        } else {
            Err(Dep3Error::InvalidOrigin(base.to_string()))
        }
    }

    /// Nome base do arquivo.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Numero da sequencia global.
    pub fn number(&self) -> Result<PatchNumber, Dep3Error> {
        PatchNumber::from_file_name(&self.0).map_err(|_| Dep3Error::InvalidOrigin(self.0.clone()))
    }
}

/// `NNNN` + segmentos kebab `[a-z0-9]+`, sem vazios nem maiusculas.
fn is_patch_stem(stem: &str) -> bool {
    let mut segments = stem.split('-');
    match segments.next() {
        Some(first) if first.len() == 4 && first.chars().all(|c| c.is_ascii_digit()) => {}
        _ => return false,
    }
    let mut count = 0;
    for segment in segments {
        count += 1;
        if segment.is_empty()
            || !segment
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        {
            return false;
        }
    }
    count > 0
}

/// Cabecalho DEP-3 validado de um patch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchHeader {
    /// Remetente bruto (`From:`).
    pub from: String,
    /// Data bruta (`Date:`).
    pub date: String,
    /// Assunto.
    pub subject: Subject,
    /// Descricao longa (linhas de texto, sem as chaves).
    pub description: String,
    /// Origem.
    pub origin: Origin,
    /// Encaminhamento.
    pub forwarded: Forwarded,
    /// Estado no upstream.
    pub applied_upstream: AppliedUpstream,
    /// Ultima atualizacao.
    pub last_update: IsoDate,
    /// Assinatura.
    pub signoff: Signoff,
}

/// Patch com cabecalho validado e corpo diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchFile {
    /// Cabecalho validado.
    pub header: PatchHeader,
    /// Corpo diff (a partir do primeiro `diff --git`).
    pub body: String,
}

/// Primeira ocorrencia de `Campo: valor` no cabecalho.
pub(crate) fn header_value(header: &str, field: &str) -> Option<String> {
    let prefix = format!("{field}:");
    header
        .lines()
        .find_map(|line| line.strip_prefix(prefix.as_str()))
        .map(|rest| rest.trim().to_string())
}

/// Linha no formato `Nome-Qualquer: valor` (fim da descricao).
pub(crate) fn is_field_line(line: &str) -> bool {
    let bytes = line.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_alphabetic() {
        return false;
    }
    let mut index = 1;
    while index < bytes.len() && (bytes[index].is_ascii_alphabetic() || bytes[index] == b'-') {
        index += 1;
    }
    index < bytes.len()
        && bytes[index] == b':'
        && bytes
            .get(index + 1)
            .is_some_and(|b| b.is_ascii_whitespace())
}

/// Faz o parse e valida um arquivo de patch completo.
pub fn parse_patch(text: &str) -> Result<PatchFile, Dep3Error> {
    let mut header_lines: Vec<&str> = Vec::new();
    let mut body_start: Option<usize> = None;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let content = line.strip_suffix('\n').unwrap_or(line);
        let content = content.strip_suffix('\r').unwrap_or(content);
        if content == "---" {
            body_start = Some(offset + line.len());
            break;
        }
        if content.starts_with("diff --git ") {
            return Err(Dep3Error::MissingTerminator);
        }
        header_lines.push(content);
        offset += line.len();
    }
    let body_start = body_start.ok_or(Dep3Error::MissingTerminator)?;
    let header = header_lines.join("\n");
    let body = text[body_start..].to_string();
    if !body.lines().any(|line| line.starts_with("diff --git ")) {
        return Err(Dep3Error::NoDiffBody);
    }

    let subject_count = header
        .lines()
        .filter(|line| line.starts_with("Subject:"))
        .count();
    if subject_count > 1 {
        return Err(Dep3Error::MultipleSubjects);
    }
    let subject = Subject::new(header_value(&header, "Subject").as_deref().unwrap_or(""))?;
    let description = collect_description(&header);
    if description.trim().is_empty() {
        return Err(Dep3Error::EmptyDescription);
    }
    let origin = header_value(&header, "Origin")
        .ok_or(Dep3Error::MissingField("Origin"))?
        .parse()?;
    let forwarded = header_value(&header, "Forwarded")
        .ok_or(Dep3Error::MissingField("Forwarded"))?
        .parse()?;
    let applied_upstream = header_value(&header, "Applied-Upstream")
        .ok_or(Dep3Error::MissingField("Applied-Upstream"))?
        .parse()?;
    let last_update = header_value(&header, "Last-Update")
        .ok_or(Dep3Error::MissingField("Last-Update"))?
        .parse()?;
    let signoff = match (
        header_value(&header, "Signed-off-by"),
        header_value(&header, "Reviewed-by"),
    ) {
        (Some(value), _) => Signoff::SignedOffBy(value),
        (None, Some(value)) => Signoff::ReviewedBy(value),
        (None, None) => return Err(Dep3Error::MissingSignoff),
    };
    let header = PatchHeader {
        from: header_value(&header, "From").ok_or(Dep3Error::MissingField("From"))?,
        date: header_value(&header, "Date").ok_or(Dep3Error::MissingField("Date"))?,
        subject,
        description,
        origin,
        forwarded,
        applied_upstream,
        last_update,
        signoff,
    };
    Ok(PatchFile { header, body })
}

/// Texto da descricao: inline apos `Description:` mais continuacoes.
///
/// Exposta para o lint acumular falhas por campo sem abortar no primeiro erro.
pub fn collect_description(header: &str) -> String {
    let mut lines: Vec<&str> = Vec::new();
    let mut active = false;
    for line in header.lines() {
        if let Some(inline) = line.strip_prefix("Description:") {
            active = true;
            let inline = inline.trim();
            if !inline.is_empty() {
                lines.push(inline);
            }
            continue;
        }
        if !active {
            continue;
        }
        if is_field_line(line) {
            break;
        }
        if !line.trim().is_empty() {
            lines.push(line.trim());
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = "From: A <a@exemplo.test>\nDate: seg, 1 jan 2026\nSubject: teste: algo\n\nDescription:\nLinha um.\nLinha dois.\n\nOrigin: vendor\nForwarded: not-needed\nApplied-Upstream: no\nLast-Update: 2026-10-26\nSigned-off-by: A <a@exemplo.test>\n\n---\n\ndiff --git a/x b/x\n";

    #[test]
    fn parses_valid_patch() {
        let file = parse_patch(VALID).unwrap();
        assert_eq!(file.header.subject.as_str(), "teste: algo");
        assert_eq!(file.header.description, "Linha um.\nLinha dois.");
        assert_eq!(file.header.origin, Origin::Vendor);
        assert_eq!(file.header.last_update.to_string(), "2026-10-26");
        assert!(file.body.contains("diff --git a/x b/x"));
    }

    #[test]
    fn rejects_missing_terminator_and_empty_body() {
        assert_eq!(
            parse_patch("Subject: x\ndiff --git a/x b/x\n"),
            Err(Dep3Error::MissingTerminator)
        );
        assert_eq!(
            parse_patch("From: a\nDate: b\nSubject: c\n\nDescription:\nd\n\nOrigin: vendor\nForwarded: no\nApplied-Upstream: no\nLast-Update: 2026-01-01\nSigned-off-by: s\n\n---\n\nsem diff\n"),
            Err(Dep3Error::NoDiffBody)
        );
    }

    #[test]
    fn validates_subject_rules() {
        let multi = VALID.replacen("Description:", "Subject: outra\nDescription:", 1);
        assert_eq!(parse_patch(&multi), Err(Dep3Error::MultipleSubjects));
        assert_eq!(
            parse_patch(&VALID.replace("Subject: teste: algo", "Subject:   ")),
            Err(Dep3Error::EmptySubject)
        );
    }

    #[test]
    fn validates_description_and_signoff() {
        assert_eq!(
            parse_patch(&VALID.replace("Linha um.\nLinha dois.", "")),
            Err(Dep3Error::EmptyDescription)
        );
        assert_eq!(
            parse_patch(&VALID.replace("Signed-off-by:", "X-Signed:")),
            Err(Dep3Error::MissingSignoff)
        );
    }

    #[test]
    fn parses_origin_forwarded_applied_vocabularies() {
        assert_eq!("vendor, extra".parse::<Origin>(), Ok(Origin::Vendor));
        assert!("invalido".parse::<Origin>().is_err());
        assert_eq!(
            "https://exemplo.test/1".parse::<Forwarded>(),
            Ok(Forwarded::Url("https://exemplo.test/1".to_string()))
        );
        assert!("talvez".parse::<Forwarded>().is_err());
        assert_eq!(
            "commit:5927ca4".parse::<AppliedUpstream>(),
            Ok(AppliedUpstream::Commit("5927ca4".to_string()))
        );
        assert!("commit:xyz".parse::<AppliedUpstream>().is_err());
        assert_eq!(
            "v4.0.7".parse::<AppliedUpstream>(),
            Ok(AppliedUpstream::Version("v4.0.7".to_string()))
        );
        assert_eq!(
            "2026-13-01".parse::<IsoDate>(),
            Err(Dep3Error::InvalidLastUpdate("2026-13-01".to_string()))
        );
        assert!("26-10-2026".parse::<IsoDate>().is_err());
    }

    #[test]
    fn validates_patch_file_names() {
        assert!(PatchFileName::from_path("patches/0001-i18n-x.patch").is_ok());
        assert_eq!(
            PatchFileName::from_path("patches/0001-i18n-x.patch")
                .unwrap()
                .number()
                .unwrap()
                .get(),
            1
        );
        for bad in [
            "1-x.patch",
            "0001-X.patch",
            "0001-.patch",
            "0001-x.diff",
            "0001-x_.patch",
            "0001--x.patch",
        ] {
            assert!(PatchFileName::from_path(bad).is_err(), "aceitou {bad}");
        }
    }
}
