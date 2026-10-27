use thiserror::Error;

/// Erros da borda de I/O e configuracao do nucleo jx.
#[derive(Debug, Error)]
pub enum BrError {
    /// Arquivo de config ausente no caminho.
    #[error("missing config file: {0}")]
    MissingConfig(String),
    /// TOML invalido ou ilegivel.
    #[error("invalid config file {0}: {1}")]
    InvalidConfig(String, String),
    /// Chave tabela.nome ausente ou com valor nao-string.
    #[error("missing key in config: {0}")]
    MissingKey(String),
    /// Caminho fora da raiz do repositorio.
    #[error("refusing to touch a path outside the repository: {0}")]
    OutsideRoot(String),
    /// Caminho vazio para resolucao.
    #[error("cannot resolve an empty path")]
    EmptyPath,
    /// Valor com aspas, barra invertida ou quebra de linha.
    #[error("unsupported value for {0}: quotes, backslashes and newlines are rejected")]
    UnsupportedValue(String),
    /// Erro de sistema de arquivos anotado com o caminho.
    #[error("i/o error at {0}: {1}")]
    Io(String, #[source] std::io::Error),
}
