use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::BrError;

/// Sistema de arquivos abstrato: producao usa disco, testes usam memoria.
pub trait FileSystem {
    /// Le o arquivo inteiro como texto.
    fn read_to_string(&self, path: &Path) -> Result<String, BrError>;
    /// Grava texto no arquivo, criando os pais quando necessario.
    fn write_string(&mut self, path: &Path, content: &str) -> Result<(), BrError>;
    /// Informa se o caminho existe.
    fn exists(&self, path: &Path) -> bool;
    /// Lista arquivos sob o diretorio, com caminhos relativos e ordenados.
    ///
    /// Diretorio ausente rende lista vazia (como `[ -d ] || return 0`).
    fn list_files_recursive(&self, dir: &Path) -> Result<Vec<PathBuf>, BrError>;
    /// Copia arquivo preservando conteudo, criando os pais do destino.
    fn copy_file(&mut self, from: &Path, to: &Path) -> Result<(), BrError>;
    /// Remove arquivo (`rm -f`: ausente nao e erro).
    fn remove_file(&mut self, path: &Path) -> Result<(), BrError>;
    /// Cria o diretorio e os pais (`mkdir -p`).
    fn create_dir_all(&mut self, path: &Path) -> Result<(), BrError>;
    /// Remove diretorio recursivo (`rm -rf`: ausente nao e erro).
    fn remove_dir_all(&mut self, path: &Path) -> Result<(), BrError>;
}

/// Erro de I/O anotado com o caminho.
fn io_err(path: &Path, kind: std::io::ErrorKind, msg: &str) -> BrError {
    BrError::Io(
        path.display().to_string(),
        std::io::Error::new(kind, msg.to_string()),
    )
}

/// Implementacao real sobre o disco, com escrita atomica.
pub struct RealFs;

impl RealFs {
    fn ensure_parent(&self, path: &Path) -> Result<(), BrError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| BrError::Io(parent.display().to_string(), e))?;
            }
        }
        Ok(())
    }

    fn walk(&self, dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), BrError> {
        let mut stack = vec![dir.to_path_buf()];
        while let Some(current) = stack.pop() {
            let entries = std::fs::read_dir(&current)
                .map_err(|e| BrError::Io(current.display().to_string(), e))?;
            for entry in entries {
                let entry = entry.map_err(|e| BrError::Io(current.display().to_string(), e))?;
                let path = entry.path();
                let kind = entry
                    .file_type()
                    .map_err(|e| BrError::Io(path.display().to_string(), e))?;
                if kind.is_dir() {
                    stack.push(path);
                } else if kind.is_file() {
                    let rel = path
                        .strip_prefix(dir)
                        .map_err(|_| BrError::OutsideRoot(path.display().to_string()))?;
                    out.push(rel.to_path_buf());
                }
            }
        }
        Ok(())
    }
}

impl FileSystem for RealFs {
    fn read_to_string(&self, path: &Path) -> Result<String, BrError> {
        std::fs::read_to_string(path).map_err(|e| BrError::Io(path.display().to_string(), e))
    }

    fn write_string(&mut self, path: &Path, content: &str) -> Result<(), BrError> {
        self.ensure_parent(path)?;
        // Escrita atomica: arquivo temporario + rename.
        let tmp = path.with_extension("tmp-jx");
        std::fs::write(&tmp, content).map_err(|e| BrError::Io(tmp.display().to_string(), e))?;
        std::fs::rename(&tmp, path).map_err(|e| BrError::Io(path.display().to_string(), e))
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn list_files_recursive(&self, dir: &Path) -> Result<Vec<PathBuf>, BrError> {
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        self.walk(dir, &mut out)?;
        out.sort();
        Ok(out)
    }

    fn copy_file(&mut self, from: &Path, to: &Path) -> Result<(), BrError> {
        self.ensure_parent(to)?;
        std::fs::copy(from, to)
            .map_err(|e| BrError::Io(format!("{} -> {}", from.display(), to.display()), e))?;
        Ok(())
    }

    fn remove_file(&mut self, path: &Path) -> Result<(), BrError> {
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(BrError::Io(path.display().to_string(), e)),
        }
    }

    fn create_dir_all(&mut self, path: &Path) -> Result<(), BrError> {
        std::fs::create_dir_all(path).map_err(|e| BrError::Io(path.display().to_string(), e))
    }

    fn remove_dir_all(&mut self, path: &Path) -> Result<(), BrError> {
        match std::fs::remove_dir_all(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(BrError::Io(path.display().to_string(), e)),
        }
    }
}

/// Implementacao em memoria para testes.
#[derive(Debug, Default)]
pub struct MemFs {
    files: HashMap<PathBuf, String>,
}

impl MemFs {
    /// Cria o fs ja contendo `path` com `content`.
    pub fn with_file(mut self, path: &str, content: &str) -> Self {
        self.files.insert(PathBuf::from(path), content.to_string());
        self
    }

    /// Le o conteudo em memoria sem erro de I/O.
    pub fn get(&self, path: &Path) -> Option<&str> {
        self.files.get(path).map(String::as_str)
    }
}

impl FileSystem for MemFs {
    fn read_to_string(&self, path: &Path) -> Result<String, BrError> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| io_err(path, std::io::ErrorKind::NotFound, "not found in MemFs"))
    }

    fn write_string(&mut self, path: &Path, content: &str) -> Result<(), BrError> {
        self.files.insert(path.to_path_buf(), content.to_string());
        Ok(())
    }

    fn exists(&self, path: &Path) -> bool {
        self.files.contains_key(path)
            || self.files.keys().any(|key| {
                key.strip_prefix(path)
                    .is_ok_and(|rel| !rel.as_os_str().is_empty())
            })
    }

    fn list_files_recursive(&self, dir: &Path) -> Result<Vec<PathBuf>, BrError> {
        let mut out: Vec<PathBuf> = self
            .files
            .keys()
            .filter_map(|path| path.strip_prefix(dir).ok())
            .map(Path::to_path_buf)
            .filter(|rel| !rel.as_os_str().is_empty())
            .collect();
        out.sort();
        Ok(out)
    }

    fn copy_file(&mut self, from: &Path, to: &Path) -> Result<(), BrError> {
        let content = self
            .files
            .get(from)
            .cloned()
            .ok_or_else(|| io_err(from, std::io::ErrorKind::NotFound, "not found in MemFs"))?;
        self.files.insert(to.to_path_buf(), content);
        Ok(())
    }

    fn remove_file(&mut self, path: &Path) -> Result<(), BrError> {
        self.files.remove(path);
        Ok(())
    }

    fn create_dir_all(&mut self, _path: &Path) -> Result<(), BrError> {
        Ok(())
    }

    fn remove_dir_all(&mut self, path: &Path) -> Result<(), BrError> {
        self.files
            .retain(|key, _| key != path && key.strip_prefix(path).is_err());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memfs_round_trip() {
        let mut fs = MemFs::default().with_file("/x/a.txt", "ola");
        assert_eq!(fs.read_to_string(Path::new("/x/a.txt")).unwrap(), "ola");
        assert!(fs.exists(Path::new("/x/a.txt")));
        assert!(!fs.exists(Path::new("/x/b.txt")));
        fs.write_string(Path::new("/x/b.txt"), "novo").unwrap();
        assert_eq!(fs.get(Path::new("/x/b.txt")), Some("novo"));
    }

    #[test]
    fn memfs_missing_is_not_found() {
        let fs = MemFs::default();
        let err = fs.read_to_string(Path::new("/nada")).unwrap_err();
        assert!(matches!(err, BrError::Io(_, _)));
    }

    #[test]
    fn memfs_lists_copies_and_removes() {
        let mut fs = MemFs::default()
            .with_file("/ws/over/b.txt", "b")
            .with_file("/ws/over/a.txt", "a")
            .with_file("/ws/outro.txt", "o");
        let listed = fs.list_files_recursive(Path::new("/ws/over")).unwrap();
        assert_eq!(listed, vec![PathBuf::from("a.txt"), PathBuf::from("b.txt")]);
        assert!(fs
            .list_files_recursive(Path::new("/ws/ausente"))
            .unwrap()
            .is_empty());
        fs.copy_file(Path::new("/ws/over/a.txt"), Path::new("/ws/dest/a.txt"))
            .unwrap();
        assert_eq!(fs.get(Path::new("/ws/dest/a.txt")), Some("a"));
        fs.remove_file(Path::new("/ws/over/a.txt")).unwrap();
        fs.remove_file(Path::new("/ws/ausente.txt")).unwrap();
        assert!(!fs.exists(Path::new("/ws/over/a.txt")));
        fs.remove_dir_all(Path::new("/ws/over")).unwrap();
        assert!(
            fs.list_files_recursive(Path::new("/ws")).unwrap()
                == vec![PathBuf::from("dest/a.txt"), PathBuf::from("outro.txt")]
        );
    }
}
