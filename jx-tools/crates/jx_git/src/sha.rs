//! SHA-1 de commit como newtype validado (40 hex minusculos).

use std::fmt;

/// Erro de parse de `Sha40`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidSha(String);

impl fmt::Display for InvalidSha {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not a 40 character hex commit: {}", self.0)
    }
}

impl std::error::Error for InvalidSha {}

/// Hash de commit: exatamente 40 hex minusculos, como `rev-parse` emite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sha40([u8; 20]);

impl Sha40 {
    /// Faz o parse validando tamanho e alfabeto (minusculos, como o lint).
    pub fn parse(raw: &str) -> Result<Self, InvalidSha> {
        let trimmed = raw.trim();
        if trimmed.len() != 40
            || !trimmed
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        {
            return Err(InvalidSha(raw.to_string()));
        }
        let mut bytes = [0u8; 20];
        for (index, chunk) in trimmed.as_bytes().chunks(2).enumerate() {
            let text = std::str::from_utf8(chunk).map_err(|_| InvalidSha(raw.to_string()))?;
            bytes[index] = u8::from_str_radix(text, 16).map_err(|_| InvalidSha(raw.to_string()))?;
        }
        Ok(Self(bytes))
    }

    /// Hex canonico em minusculos.
    pub fn hex(self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Prefixo curto de 12 hex, como o `rebase.sh` sintetiza.
    pub fn short(self) -> String {
        self.hex()[..12].to_string()
    }
}

impl fmt::Display for Sha40 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.hex())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PIN: &str = "5927ca45f62a2ce288f001f8c07cf6c06bcbbf3d";

    #[test]
    fn parses_pinned_sha() {
        let sha = Sha40::parse(PIN).unwrap();
        assert_eq!(sha.to_string(), PIN);
        assert_eq!(sha.short(), &PIN[..12]);
    }

    #[test]
    fn rejects_shapes() {
        assert!(Sha40::parse("").is_err());
        assert!(Sha40::parse("5927ca45").is_err());
        assert!(Sha40::parse(&format!("{PIN}x")).is_err());
        assert!(Sha40::parse(&PIN.to_uppercase()).is_err());
        assert!(Sha40::parse("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz").is_err());
    }
}
