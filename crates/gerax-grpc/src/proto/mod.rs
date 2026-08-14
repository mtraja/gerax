//! Geração de schemas Protobuf e builders relacionados.

pub mod builder;
pub mod mapper;

tonic::include_proto!("gerax");

pub use builder::ProtoBuilder;
pub use mapper::RustType;

use std::ops::Deref;

/// Schema Protobuf gerado.
///
/// Encapsula o conteúdo textual de um arquivo `.proto` e permite inspeção
/// básica de pacotes e serviços sem parsear o schema novamente.
#[derive(Debug, Clone)]
pub struct ProtoSchema {
    /// Nome do arquivo `.proto` gerado.
    pub file_name: String,
    /// Conteúdo completo do arquivo `.proto`.
    pub content: String,
}

impl ProtoSchema {
    /// Cria um novo schema a partir do conteúdo e nome de arquivo.
    pub fn new(file_name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            file_name: file_name.into(),
            content: content.into(),
        }
    }

    /// Retorna o nome do pacote declarado no schema, se encontrado.
    ///
    /// Busca pela linha `package <nome>;`.
    pub fn package_name(&self) -> Option<&str> {
        for line in self.content.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("package ") {
                let name = rest.trim_end_matches(';').trim();
                if !name.is_empty() {
                    return Some(name);
                }
            }
        }
        None
    }

    /// Itera sobre os nomes de serviços declarados no schema.
    ///
    /// Busca por blocos `service <Nome> { ... }`.
    pub fn services(&self) -> impl Iterator<Item = &str> {
        self.content.lines().filter_map(|line| {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("service ") {
                let name = rest.split_whitespace().next()?.trim_end_matches('{').trim();
                if !name.is_empty() {
                    return Some(name);
                }
            }
            None
        })
    }
}

impl From<String> for ProtoSchema {
    fn from(content: String) -> Self {
        Self {
            file_name: String::new(),
            content,
        }
    }
}

impl Deref for ProtoSchema {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.content
    }
}
