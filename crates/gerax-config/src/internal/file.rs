use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::error::{ConfigError, ConfigResult};

/// Lê o conteúdo de um arquivo.
///
/// Toda leitura de arquivo do crate passa por este módulo.
pub(crate) fn read(
    path: impl AsRef<Path>,
) -> ConfigResult<String> {

    let path = path.as_ref();

    fs::read_to_string(path)
        .map_err(|source| {

            ConfigError::Io {
                path: path
                    .display()
                    .to_string(),

                source,
            }

        })
}


/// Verifica se um arquivo existe.
#[allow(dead_code)]
pub(crate) fn exists(
    path: impl AsRef<Path>,
) -> bool {

    path.as_ref().exists()
}


/// Retorna o caminho absoluto.
///
/// Útil para logs e mensagens de erro.
#[allow(dead_code)]
pub(crate) fn absolute(
    path: impl AsRef<Path>,
) -> ConfigResult<PathBuf> {

    std::fs::canonicalize(path)
        .map_err(|source| {

            ConfigError::Io {
                path: "<unknown>".into(),

                source,
            }

        })
}