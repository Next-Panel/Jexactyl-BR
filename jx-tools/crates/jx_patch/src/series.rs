//! Serie quilt: parse, numeracao e insercao ordenada.
//!
//! Espelha `series_entries`, `patch_number`, `next_patch_number` e
//! `series_insert` de `scripts/lib/common.sh`.

use std::fmt;
use std::num::NonZeroU16;

/// Erros da manipulacao da serie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeriesError {
    /// Prefixo numerico ilegivel (sem digitos para extrair).
    UnreadableNumber(String),
    /// Entrada invalida: vazia ou com caracteres de controle.
    InvalidEntry(String),
    /// Sequencia cheia: 9999 patches ja existem.
    SequenceFull,
}

impl fmt::Display for SeriesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnreadableNumber(entry) => write!(f, "cannot read the number of {entry}"),
            Self::InvalidEntry(entry) => write!(f, "invalid series entry: {entry:?}"),
            Self::SequenceFull => write!(f, "the patch sequence is full (9999 patches already)"),
        }
    }
}

impl std::error::Error for SeriesError {}

/// Numero de patch na sequencia global: 1 a 9999.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PatchNumber(NonZeroU16);

impl PatchNumber {
    /// Maior numero permitido na serie.
    pub const MAX: u16 = 9999;

    /// Constroi a partir do valor bruto; rejeita zero e valores acima de `MAX`.
    pub fn new(value: u16) -> Option<Self> {
        NonZeroU16::new(value)
            .filter(|n| n.get() <= Self::MAX)
            .map(Self)
    }

    /// Valor numerico.
    pub fn get(self) -> u16 {
        self.0.get()
    }

    /// Extrai o numero do nome do arquivo.
    ///
    /// Regra leniente herdada do shell (`cut -d- -f1 | tr -cd 0-9` com
    /// `10#`): pega o texto antes do primeiro `-` no nome base e filtra so
    /// os digitos. `patches/0001-x.patch` vira 1.
    pub fn from_file_name(name: &str) -> Result<Self, SeriesError> {
        let base = name.rsplit('/').next().unwrap_or(name);
        let segment = base.split('-').next().unwrap_or("");
        let digits: String = segment.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            return Err(SeriesError::UnreadableNumber(name.to_string()));
        }
        let value: u32 = digits
            .parse()
            .map_err(|_| SeriesError::UnreadableNumber(name.to_string()))?;
        u16::try_from(value)
            .ok()
            .and_then(Self::new)
            .ok_or_else(|| SeriesError::UnreadableNumber(name.to_string()))
    }

    /// Formata com quatro digitos, como nos nomes de arquivo.
    pub fn padded(self) -> String {
        format!("{:04}", self.get())
    }
}

impl fmt::Display for PatchNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get())
    }
}

/// Uma linha valida do arquivo `series`: caminho do patch, sem comentarios.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SeriesEntry(String);

impl SeriesEntry {
    /// Valida texto ja sem comentarios: nao vazio e sem controle.
    pub fn new(raw: &str) -> Result<Self, SeriesError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.chars().any(|c| c.is_control()) {
            return Err(SeriesError::InvalidEntry(raw.to_string()));
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Texto da entrada.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Numero da entrada, pela regra leniente de `from_file_name`.
    pub fn number(&self) -> Result<PatchNumber, SeriesError> {
        PatchNumber::from_file_name(&self.0)
    }
}

impl fmt::Display for SeriesEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Remove comentario estilo quilt: `#` precedido de branco ate o fim.
///
/// Um `#` colado no nome (`foo#bar.patch`) e preservado, como no `sed`.
fn strip_quilt_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'#'
            && (index == 0 || bytes[index - 1] == b' ' || bytes[index - 1] == b'\t')
        {
            return line[..index].trim_end();
        }
        index += 1;
    }
    line
}

/// Faz o parse do texto do `series` em entradas, ignorando comentarios e vazios.
pub fn parse_series(text: &str) -> Vec<SeriesEntry> {
    text.lines()
        .map(|line| strip_quilt_comment(line).trim())
        .filter(|stripped| !stripped.is_empty())
        .filter_map(|stripped| SeriesEntry::new(stripped).ok())
        .collect()
}

/// Proximo numero livre: maior existente + 1, ou 1 na serie vazia.
pub fn next_after(existing: &[PatchNumber]) -> Result<PatchNumber, SeriesError> {
    match existing.iter().max() {
        None => PatchNumber::new(1).ok_or(SeriesError::SequenceFull),
        Some(number) => {
            PatchNumber::new(number.get().saturating_add(1)).ok_or(SeriesError::SequenceFull)
        }
    }
}

/// Rotulo do dominio pelo prefixo de dois digitos.
///
/// `01` i18n, `02` gateway, `03` billing, `04`-`09` core, demais ui.
/// Espelha `domain_label` de `scripts/lib/common.sh`.
pub fn domain_label(domain: &str) -> &'static str {
    match domain {
        "01" => "i18n",
        "02" => "gateway",
        "03" => "billing",
        "04" | "05" | "06" | "07" | "08" | "09" => "core",
        _ => "ui",
    }
}

/// Insere a entrada mantendo a ordem numerica e preservando comentarios.
///
/// Espelha `series_insert`: a nova linha entra antes da primeira entrada com
/// numero maior ou igual; sem candidata, e anexada no fim. Duplicadas nao sao
/// verificadas aqui (o chamador faz `grep -qF`, como no shell).
pub fn insert_into_text(original: &str, entry: &SeriesEntry) -> Result<String, SeriesError> {
    let number = entry.number()?;
    let mut out: Vec<String> = Vec::new();
    let mut inserted = false;
    for line in original.lines() {
        let stripped = strip_quilt_comment(line).trim();
        if !stripped.is_empty() && !inserted {
            if let Ok(current) = PatchNumber::from_file_name(stripped) {
                if current >= number {
                    out.push(entry.as_str().to_string());
                    inserted = true;
                }
            }
        }
        out.push(line.to_string());
    }
    if !inserted {
        out.push(entry.as_str().to_string());
    }
    let mut text = out.join("\n");
    text.push('\n');
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_only_blank_preceded_comments() {
        assert_eq!(
            strip_quilt_comment("patches/0001-x.patch # fix"),
            "patches/0001-x.patch"
        );
        assert_eq!(
            strip_quilt_comment("patches/0001-x.patch\t# fix"),
            "patches/0001-x.patch"
        );
        assert_eq!(strip_quilt_comment("foo#bar.patch"), "foo#bar.patch");
        assert_eq!(strip_quilt_comment("# full line"), "");
        assert_eq!(strip_quilt_comment("   # indented"), "");
    }

    #[test]
    fn parses_series_ignoring_comments_and_blanks() {
        let text = "# topo\n\npatches/0001-a.patch\npatches/0002-b.patch # nota\n   \n";
        let entries = parse_series(text);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].as_str(), "patches/0001-a.patch");
        assert_eq!(entries[1].as_str(), "patches/0002-b.patch");
    }

    #[test]
    fn number_extraction_is_lenient_like_shell() {
        assert_eq!(
            PatchNumber::from_file_name("patches/0001-x.patch")
                .unwrap()
                .get(),
            1
        );
        assert_eq!(
            PatchNumber::from_file_name("0042-x.patch").unwrap().get(),
            42
        );
        assert!(PatchNumber::from_file_name("patches/no-number.patch").is_err());
        assert!(PatchNumber::from_file_name("patches/10000-x.patch").is_err());
        assert!(PatchNumber::new(0).is_none());
        assert!(PatchNumber::new(10000).is_none());
        assert_eq!(PatchNumber::new(7).unwrap().padded(), "0007");
    }

    #[test]
    fn next_number_follows_max_plus_one() {
        assert_eq!(next_after(&[]).unwrap().get(), 1);
        let existing = [PatchNumber::new(1).unwrap(), PatchNumber::new(41).unwrap()];
        assert_eq!(next_after(&existing).unwrap().get(), 42);
        assert_eq!(
            next_after(&[PatchNumber::new(9999).unwrap()]),
            Err(SeriesError::SequenceFull)
        );
    }

    #[test]
    fn insert_keeps_order_and_comments() {
        let original = "# cabecalho\npatches/0001-a.patch\npatches/0003-c.patch\n";
        let entry = SeriesEntry::new("patches/0002-b.patch").unwrap();
        let out = insert_into_text(original, &entry).unwrap();
        assert_eq!(
            out,
            "# cabecalho\npatches/0001-a.patch\npatches/0002-b.patch\npatches/0003-c.patch\n"
        );
    }

    #[test]
    fn insert_appends_without_candidate_and_rejects_unreadable() {
        let original = "patches/0001-a.patch\n";
        let entry = SeriesEntry::new("patches/0009-z.patch").unwrap();
        assert_eq!(
            insert_into_text(original, &entry).unwrap(),
            "patches/0001-a.patch\npatches/0009-z.patch\n"
        );
        let bad = SeriesEntry::new("patches/slug-sem-numero.patch").unwrap();
        assert_eq!(
            insert_into_text(original, &bad),
            Err(SeriesError::UnreadableNumber(
                "patches/slug-sem-numero.patch".to_string()
            ))
        );
    }

    #[test]
    fn insert_into_empty_series() {
        let entry = SeriesEntry::new("patches/0001-a.patch").unwrap();
        assert_eq!(
            insert_into_text("", &entry).unwrap(),
            "patches/0001-a.patch\n"
        );
    }

    #[test]
    fn entry_rejects_empty_and_control() {
        assert!(SeriesEntry::new("   ").is_err());
        assert!(SeriesEntry::new("a\nb").is_err());
        assert!(SeriesEntry::new("patches/0001-a.patch").is_ok());
    }
}
