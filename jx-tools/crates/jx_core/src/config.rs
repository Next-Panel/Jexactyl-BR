use std::path::{Path, PathBuf};

use toml_edit::{DocumentMut, Item, Value};

use crate::error::BrError;
use crate::fs::FileSystem;

/// Chave `tabela.nome` do config.toml (ex.: `upstream.sha`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigKey {
    table: String,
    name: String,
}

impl ConfigKey {
    /// Faz o parse de `"tabela.nome"`; rejeita formas sem tabela ou com subniveis.
    pub fn parse(raw: &str) -> Result<Self, BrError> {
        let (table, name) = raw
            .split_once('.')
            .ok_or_else(|| BrError::MissingKey(raw.to_string()))?;
        if table.is_empty() || name.is_empty() || name.contains('.') {
            return Err(BrError::MissingKey(raw.to_string()));
        }
        Ok(Self {
            table: table.to_string(),
            name: name.to_string(),
        })
    }

    /// Representacao `"tabela.nome"` da chave.
    pub fn dotted(&self) -> String {
        format!("{}.{}", self.table, self.name)
    }

    /// Nome da tabela TOML.
    pub fn table(&self) -> &str {
        &self.table
    }

    /// Nome do campo dentro da tabela.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Le o valor string de `tabela.nome` preservando o resto do arquivo.
/// Espelha `config_get`: valor ausente ou nao-string e erro.
pub fn config_get<FS: FileSystem>(
    fs: &FS,
    path: &Path,
    key: &ConfigKey,
) -> Result<String, BrError> {
    let raw = fs
        .read_to_string(path)
        .map_err(|_| BrError::MissingConfig(path.display().to_string()))?;
    let doc: DocumentMut = raw.parse().map_err(|e: toml_edit::TomlError| {
        BrError::InvalidConfig(path.display().to_string(), e.to_string())
    })?;
    let value = doc
        .get(&key.table)
        .and_then(|t| t.as_table())
        .and_then(|t| t.get(&key.name))
        .and_then(|i| i.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| BrError::MissingKey(key.dotted()))?;
    Ok(value.to_string())
}

/// Grava `tabela.nome = "valor"` preservando comentarios e formato.
/// Espelha `config_set`: rejeita aspas, barras invertidas e quebras de linha.
pub fn config_set<FS: FileSystem>(
    fs: &mut FS,
    path: &Path,
    key: &ConfigKey,
    value: &str,
) -> Result<(), BrError> {
    if value.contains('"') || value.contains('\\') || value.contains('\n') {
        return Err(BrError::UnsupportedValue(key.dotted()));
    }
    let raw = fs
        .read_to_string(path)
        .map_err(|_| BrError::MissingConfig(path.display().to_string()))?;
    let mut doc: DocumentMut = raw.parse().map_err(|e: toml_edit::TomlError| {
        BrError::InvalidConfig(path.display().to_string(), e.to_string())
    })?;

    if !doc.contains_key(&key.table) {
        doc[&key.table] = Item::Table(toml_edit::Table::new());
    }
    let table = doc[&key.table].as_table_mut().ok_or_else(|| {
        BrError::InvalidConfig(
            path.display().to_string(),
            format!("{} is not a table", key.table),
        )
    })?;
    table[&key.name] = Item::Value(Value::from(value));

    let rendered = doc.to_string();
    // Revalida: o valor escrito precisa ser lido de volta.
    let check: DocumentMut = rendered.parse().map_err(|e: toml_edit::TomlError| {
        BrError::InvalidConfig(path.display().to_string(), e.to_string())
    })?;
    let back = check
        .get(&key.table)
        .and_then(|t| t.as_table())
        .and_then(|t| t.get(&key.name))
        .and_then(|i| i.as_str())
        .unwrap_or("");
    if back != value {
        return Err(BrError::InvalidConfig(
            path.display().to_string(),
            key.dotted(),
        ));
    }
    fs.write_string(path, &rendered)
}

/// Resolve caminho relativo a raiz do repo; absoluto passa direto.
/// Espelha `resolve_path`.
pub fn resolve_path(root: &Path, rel: &str) -> Result<PathBuf, BrError> {
    if rel.is_empty() {
        return Err(BrError::EmptyPath);
    }
    let p = Path::new(rel);
    if p.is_absolute() {
        Ok(p.to_path_buf())
    } else {
        Ok(root.join(p))
    }
}

/// Recusa caminhos fora da raiz (via forma canonica quando possivel).
/// Espelha `require_inside_root`.
pub fn require_inside_root(root: &Path, path: &Path) -> Result<PathBuf, BrError> {
    let canon_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let canon_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if canon_path == canon_root || canon_path.starts_with(&canon_root) {
        Ok(canon_path)
    } else {
        Err(BrError::OutsideRoot(path.display().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::MemFs;
    use std::path::Path;

    const SAMPLE: &str = "# comentario de topo\n[upstream]\nurl = \"https://exemplo.test/painel.git\"\n# comentario interno\nref = \"v4.0.7\"\nsha = \"abc123\"\n";

    fn memfs() -> MemFs {
        MemFs::default().with_file("/repo/config.toml", SAMPLE)
    }

    #[test]
    fn get_reads_string_value() {
        let fs = memfs();
        let key = ConfigKey::parse("upstream.sha").unwrap();
        assert_eq!(
            config_get(&fs, Path::new("/repo/config.toml"), &key).unwrap(),
            "abc123"
        );
    }

    #[test]
    fn get_rejects_missing_key_and_table() {
        let fs = memfs();
        assert!(matches!(
            config_get(
                &fs,
                Path::new("/repo/config.toml"),
                &ConfigKey::parse("upstream.falta").unwrap()
            ),
            Err(BrError::MissingKey(_))
        ));
        assert!(matches!(
            config_get(
                &fs,
                Path::new("/repo/config.toml"),
                &ConfigKey::parse("falta.x").unwrap()
            ),
            Err(BrError::MissingKey(_))
        ));
    }

    #[test]
    fn get_rejects_missing_file() {
        let fs = memfs();
        assert!(matches!(
            config_get(
                &fs,
                Path::new("/repo/outro.toml"),
                &ConfigKey::parse("upstream.sha").unwrap()
            ),
            Err(BrError::MissingConfig(_))
        ));
    }

    #[test]
    fn set_preserves_comments_and_format() {
        let mut fs = memfs();
        let key = ConfigKey::parse("upstream.ref").unwrap();
        config_set(&mut fs, Path::new("/repo/config.toml"), &key, "v4.0.8").unwrap();
        let out = fs.read_to_string(Path::new("/repo/config.toml")).unwrap();
        assert!(out.contains("# comentario de topo"));
        assert!(out.contains("# comentario interno"));
        assert!(out.contains("ref = \"v4.0.8\""));
        assert!(out.contains("sha = \"abc123\""));
        // Leitura de volta confirma.
        assert_eq!(
            config_get(&fs, Path::new("/repo/config.toml"), &key).unwrap(),
            "v4.0.8"
        );
    }

    #[test]
    fn set_creates_missing_table_and_key() {
        let mut fs = memfs();
        let key = ConfigKey::parse("patches.series").unwrap();
        config_set(
            &mut fs,
            Path::new("/repo/config.toml"),
            &key,
            "patches/series",
        )
        .unwrap();
        assert_eq!(
            config_get(&fs, Path::new("/repo/config.toml"), &key).unwrap(),
            "patches/series"
        );
    }

    #[test]
    fn set_rejects_quotes_backslashes_newlines() {
        let mut fs = memfs();
        for bad in ["com \"aspas\"", "com\\barra", "com\nquebra"] {
            let key = ConfigKey::parse("upstream.ref").unwrap();
            assert!(
                matches!(
                    config_set(&mut fs, Path::new("/repo/config.toml"), &key, bad),
                    Err(BrError::UnsupportedValue(_))
                ),
                "aceitou {bad:?}"
            );
        }
    }

    #[test]
    fn key_parse_rejects_shapes() {
        assert!(ConfigKey::parse("semtabela").is_err());
        assert!(ConfigKey::parse(".vazio").is_err());
        assert!(ConfigKey::parse("a.b.c").is_err());
    }

    #[test]
    fn resolve_joins_relative_and_keeps_absolute() {
        let root = Path::new("/repo");
        assert_eq!(
            resolve_path(root, "patches/series").unwrap(),
            PathBuf::from("/repo/patches/series")
        );
        assert_eq!(
            resolve_path(root, "/abs/x").unwrap(),
            PathBuf::from("/abs/x")
        );
        assert!(matches!(resolve_path(root, ""), Err(BrError::EmptyPath)));
    }
}
