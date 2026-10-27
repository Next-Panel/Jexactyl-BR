//! Tipos centrais do ferramental i18n: severidade e achado.
//!
//! Espelha `scripts/i18n/i18n_toolkit/models.py` (pydantic) com tipos
//! fechados: linha nunca e zero (`NonZeroU32`), textos nunca vazios.

use std::fmt;
use std::num::NonZeroU32;
use std::str::FromStr;

/// Severidade de um achado de auditoria.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    /// Bloqueia o gate.
    Blocker,
    /// Falha grave de traducao.
    Major,
    /// Falha leve (ex.: glossario, label em ingles).
    Minor,
    /// Detalhe de estilo.
    Nit,
}

impl Severity {
    /// Nome canonico em minusculas (igual ao JSON do bridge).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blocker => "blocker",
            Self::Major => "major",
            Self::Minor => "minor",
            Self::Nit => "nit",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Erro de parse de `Severity`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidSeverity(String);

impl fmt::Display for InvalidSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid severity: {}", self.0)
    }
}

impl std::error::Error for InvalidSeverity {}

impl FromStr for Severity {
    type Err = InvalidSeverity;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw {
            "blocker" => Ok(Self::Blocker),
            "major" => Ok(Self::Major),
            "minor" => Ok(Self::Minor),
            "nit" => Ok(Self::Nit),
            _ => Err(InvalidSeverity(raw.to_string())),
        }
    }
}

/// Erro de construcao de `Finding`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindingError {
    /// Codigo vazio.
    EmptyCode,
    /// Arquivo vazio.
    EmptyFile,
    /// Linha zero (usar `NonZeroU32`).
    ZeroLine,
    /// Problema vazio.
    EmptyProblem,
    /// Correcao vazia.
    EmptyFix,
}

impl fmt::Display for FindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCode => write!(f, "finding code must be a non-empty string"),
            Self::EmptyFile => write!(f, "finding file must be a non-empty string"),
            Self::ZeroLine => write!(f, "finding line must be a positive integer"),
            Self::EmptyProblem => write!(f, "finding problem must be a non-empty string"),
            Self::EmptyFix => write!(f, "finding fix must be a non-empty string"),
        }
    }
}

impl std::error::Error for FindingError {}

/// Achado de auditoria: severidade, codigo, local e correcao sugerida.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    severity: Severity,
    code: String,
    file: String,
    line: NonZeroU32,
    problem: String,
    fix: String,
}

impl Finding {
    /// Constroi validando os invariantes do `models.py`.
    pub fn new(
        severity: Severity,
        code: &str,
        file: &str,
        line: u32,
        problem: &str,
        fix: &str,
    ) -> Result<Self, FindingError> {
        if code.is_empty() {
            return Err(FindingError::EmptyCode);
        }
        if file.is_empty() {
            return Err(FindingError::EmptyFile);
        }
        let line = NonZeroU32::new(line).ok_or(FindingError::ZeroLine)?;
        if problem.is_empty() {
            return Err(FindingError::EmptyProblem);
        }
        if fix.is_empty() {
            return Err(FindingError::EmptyFix);
        }
        Ok(Self {
            severity,
            code: code.to_string(),
            file: file.to_string(),
            line,
            problem: problem.to_string(),
            fix: fix.to_string(),
        })
    }

    /// Severidade.
    pub fn severity(&self) -> Severity {
        self.severity
    }

    /// Codigo da regra (ex.: `code-changed`, `english-label`).
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Arquivo do achado.
    pub fn file(&self) -> &str {
        &self.file
    }

    /// Linha (sempre >= 1).
    pub fn line(&self) -> u32 {
        self.line.get()
    }

    /// Descricao do problema.
    pub fn problem(&self) -> &str {
        &self.problem
    }

    /// Correcao sugerida.
    pub fn fix(&self) -> &str {
        &self.fix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_round_trips() {
        for (raw, expected) in [
            ("blocker", Severity::Blocker),
            ("major", Severity::Major),
            ("minor", Severity::Minor),
            ("nit", Severity::Nit),
        ] {
            assert_eq!(raw.parse::<Severity>().unwrap(), expected);
            assert_eq!(expected.as_str(), raw);
        }
        assert!("fatal".parse::<Severity>().is_err());
    }

    #[test]
    fn finding_rejects_empty_fields_and_zero_line() {
        let good = || Finding::new(Severity::Minor, "english-label", "a.tsx", 3, "p", "f");
        assert!(good().is_ok());
        assert_eq!(
            Finding::new(Severity::Minor, "", "a.tsx", 3, "p", "f"),
            Err(FindingError::EmptyCode)
        );
        assert_eq!(
            Finding::new(Severity::Minor, "c", "", 3, "p", "f"),
            Err(FindingError::EmptyFile)
        );
        assert_eq!(
            Finding::new(Severity::Minor, "c", "a.tsx", 0, "p", "f"),
            Err(FindingError::ZeroLine)
        );
        assert_eq!(
            Finding::new(Severity::Minor, "c", "a.tsx", 3, "", "f"),
            Err(FindingError::EmptyProblem)
        );
        assert_eq!(
            Finding::new(Severity::Minor, "c", "a.tsx", 3, "p", ""),
            Err(FindingError::EmptyFix)
        );
    }
}
