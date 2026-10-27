//! Parse de `git status --porcelain=v1` em entradas tipadas.

use std::fmt;

/// Erro de parse do status porcelain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidStatus(String);

impl fmt::Display for InvalidStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid porcelain status line: {}", self.0)
    }
}

impl std::error::Error for InvalidStatus {}

/// Uma linha `XY caminho` do porcelain v1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusEntry {
    /// Estado no indice.
    pub index: char,
    /// Estado na worktree.
    pub worktree: char,
    /// Caminho atual.
    pub path: String,
    /// Origem em renomeacoes (`origem -> destino`).
    pub renamed_from: Option<String>,
}

impl StatusEntry {
    /// Worktree limpa: nenhuma entrada.
    pub fn is_clean(entries: &[Self]) -> bool {
        entries.is_empty()
    }
}

/// Faz o parse da saida de `git status --porcelain=v1`.
///
/// Formatos: `XY caminho`, `XY origem -> destino` (renomeado) e
/// `"XY caminho com espaco"` entre aspas. Linhas malformadas sao erro.
pub fn parse_status_porcelain(text: &str) -> Result<Vec<StatusEntry>, InvalidStatus> {
    let mut entries = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let mut chars = line.chars();
        let x = chars
            .next()
            .ok_or_else(|| InvalidStatus(line.to_string()))?;
        let y = chars
            .next()
            .ok_or_else(|| InvalidStatus(line.to_string()))?;
        let rest: String = chars.collect();
        let rest = rest.strip_prefix(' ').unwrap_or(rest.as_str());
        let (path, renamed_from) = match rest.split_once(" -> ") {
            Some((from, to)) => (
                unquote(to).ok_or_else(|| InvalidStatus(line.to_string()))?,
                Some(unquote(from).ok_or_else(|| InvalidStatus(line.to_string()))?),
            ),
            None => (
                unquote(rest).ok_or_else(|| InvalidStatus(line.to_string()))?,
                None,
            ),
        };
        if path.is_empty() {
            return Err(InvalidStatus(line.to_string()));
        }
        entries.push(StatusEntry {
            index: x,
            worktree: y,
            path,
            renamed_from,
        });
    }
    Ok(entries)
}

/// Remove aspas do porcelain quando o caminho tem espacos.
fn unquote(raw: &str) -> Option<String> {
    let quoted = raw
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'));
    match quoted {
        Some(inner) => Some(inner.replace("\\\"", "\"")),
        None => {
            if raw.contains('"') {
                None
            } else {
                Some(raw.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_states() {
        let entries = parse_status_porcelain(" M src/a.ts\nA  src/b.ts\n?? novo/\n").unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].index, ' ');
        assert_eq!(entries[0].worktree, 'M');
        assert_eq!(entries[0].path, "src/a.ts");
        assert_eq!(entries[2].index, '?');
        assert!(entries[2].renamed_from.is_none());
        assert!(StatusEntry::is_clean(&[]));
        assert!(!StatusEntry::is_clean(&entries));
        assert!(parse_status_porcelain("").unwrap().is_empty());
    }

    #[test]
    fn parses_renames_and_quoted_paths() {
        let entries =
            parse_status_porcelain("R  velho.ts -> novo.ts\n M \"com espaco.ts\"\n").unwrap();
        assert_eq!(entries[0].renamed_from.as_deref(), Some("velho.ts"));
        assert_eq!(entries[0].path, "novo.ts");
        assert_eq!(entries[1].path, "com espaco.ts");
    }

    #[test]
    fn rejects_malformed_lines() {
        assert!(parse_status_porcelain("M").is_err());
        assert!(parse_status_porcelain("M ").is_err());
    }
}
