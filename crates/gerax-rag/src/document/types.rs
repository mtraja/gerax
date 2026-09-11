//! Tipos centrais do domínio de documentos: representação de um documento e
//! seus metadados, utilizados por todos os estágios do pipeline RAG.

use std::collections::HashMap;

use thiserror::Error;

/// Metadados de um documento, modelados como um mapa de chave/valor.
///
/// Os valores são JSON (`serde_json::Value`), permitindo armazenar qualquer
/// tipo serializável (strings, números, booleanos, listas e objetos) e
/// facilitando a integração com providers e bases vetoriais.
pub type DocumentMetadata = HashMap<String, serde_json::Value>;

/// Erros possíveis ao validar/construir um [`Document`].
#[derive(Debug, Error)]
pub enum DocumentError {
    /// O `id` informado está vazio ou ausente.
    #[error("document id cannot be empty")]
    EmptyId,
    /// O `content` informado está vazio ou ausente.
    #[error("document content cannot be empty")]
    EmptyContent,
}

/// Um documento de texto a ser indexado e consultado pelo pipeline RAG.
///
/// Representa a unidade atômica de indexação: cada documento recebe um `id`
/// único, um `content` textual e metadados opcionais que permitem filtros e
/// enriquecimento na recuperação.
///
/// Prefira construir via [`Document::builder`] para validar os pré-requisitos
/// (`id` e `content` não vazios).
#[derive(Debug, Clone)]
pub struct Document {
    /// Identificador único do documento.
    pub id: String,
    /// Conteúdo textual indexado pelos chunkers e armazenado no vector store.
    pub content: String,
    /// Metadados associados, usados para filtragem e pós-processamento.
    pub metadata: DocumentMetadata,
}

/// Builder incremental para [`Document`].
///
/// Valida que `id` e `content` estejam presentes e não vazios antes de
/// construir o documento final.
#[derive(Debug, Default)]
pub struct DocumentBuilder {
    id: Option<String>,
    content: Option<String>,
    metadata: DocumentMetadata,
}

impl DocumentBuilder {
    /// Cria um novo builder vazio.
    pub fn new() -> Self {
        Self::default()
    }

    /// Define o identificador do documento.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Define o conteúdo textual do documento.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Adiciona um par chave/valor aos metadados do documento.
    ///
    /// Pode ser chamado várias vezes para acumular múltiplos metadados.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Constrói o [`Document`], validando `id` e `content`.
    ///
    /// Falha com [`DocumentError::EmptyId`] quando o `id` está ausente ou vazio
    /// e com [`DocumentError::EmptyContent`] quando o conteúdo está ausente ou
    /// vazio.
    pub fn build(self) -> Result<Document, DocumentError> {
        let id = self.id.ok_or(DocumentError::EmptyId)?;
        if id.is_empty() {
            return Err(DocumentError::EmptyId);
        }

        let content = self.content.ok_or(DocumentError::EmptyContent)?;
        if content.is_empty() {
            return Err(DocumentError::EmptyContent);
        }

        Ok(Document {
            id,
            content,
            metadata: self.metadata,
        })
    }
}

impl Document {
    /// Inicia a construção de um novo [`Document`] via [`DocumentBuilder`].
    pub fn builder() -> DocumentBuilder {
        DocumentBuilder::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn build_document_successfully() {
        let doc = Document::builder()
            .id("manual-alunos")
            .content("Conteudo do manual")
            .metadata("source", "manual")
            .metadata("language", "pt-BR")
            .build()
            .unwrap();

        assert_eq!(doc.id, "manual-alunos");
        assert_eq!(doc.content, "Conteudo do manual");
        assert_eq!(doc.metadata["source"], json!("manual"));
        assert_eq!(doc.metadata["language"], json!("pt-BR"));
    }

    #[test]
    fn fail_on_empty_id() {
        let result = Document::builder().id("").content("conteudo").build();
        assert!(matches!(result, Err(DocumentError::EmptyId)));
    }

    #[test]
    fn fail_on_missing_id() {
        let result = Document::builder().content("conteudo").build();
        assert!(matches!(result, Err(DocumentError::EmptyId)));
    }

    #[test]
    fn fail_on_empty_content() {
        let result = Document::builder().id("doc1").content("").build();
        assert!(matches!(result, Err(DocumentError::EmptyContent)));
    }

    #[test]
    fn fail_on_missing_content() {
        let result = Document::builder().id("doc1").build();
        assert!(matches!(result, Err(DocumentError::EmptyContent)));
    }
}
