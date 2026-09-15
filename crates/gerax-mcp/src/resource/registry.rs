//! Registry de Resources.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::sync::RwLock;

use async_trait::async_trait;

use super::trait_def::{Resource, ResourceContents, ResourceError};

/// Registro de resources expostos pelo servidor.
///
/// `ResourceRegistry` é um `Arc<dyn Resource>` armazenado em
/// `RwLock`, permitindo registros concorrentes.
#[derive(Clone, Default)]
pub struct ResourceRegistry {
    resources: Arc<RwLock<HashMap<String, Arc<dyn Resource>>>>,
}

impl ResourceRegistry {
    /// Cria um registry vazio.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra um resource.
    ///
    /// # Errors
    ///
    /// * [`ResourceError::InvalidResource`] se o URI for vazio.
    /// * [`ResourceError::Duplicate`] se o URI já existir.
    pub fn register(&self, resource: Arc<dyn Resource>) -> Result<(), ResourceError> {
        let uri = resource.uri();
        if uri.is_empty() {
            return Err(ResourceError::InvalidResource(
                "URI não pode ser vazio".to_string(),
            ));
        }

        let mut resources = self
            .resources
            .write()
            .map_err(|_| ResourceError::Internal("registry travado".to_string()))?;

        if resources.contains_key(uri) {
            return Err(ResourceError::Duplicate(uri.to_string()));
        }

        resources.insert(uri.to_string(), resource);
        Ok(())
    }

    /// Remove um resource pelo URI.
    ///
    /// Retorna `true` se o resource existia.
    pub fn unregister(&self, uri: &str) -> bool {
        self.resources
            .write()
            .map(|mut resources| resources.remove(uri).is_some())
            .unwrap_or(false)
    }

    /// Busca um resource pelo URI.
    pub fn get(&self, uri: &str) -> Option<Arc<dyn Resource>> {
        self.resources
            .read()
            .ok()
            .and_then(|resources| resources.get(uri).cloned())
    }

    /// Lista todos os resources ordenados por URI.
    pub fn list(&self) -> Vec<Arc<dyn Resource>> {
        let mut resources: Vec<_> = self
            .resources
            .read()
            .map(|resources| resources.values().cloned().collect())
            .unwrap_or_default();
        resources.sort_by(|a, b| a.uri().cmp(b.uri()));
        resources
    }

    /// Lista os URIs dos resources registrados, ordenados.
    pub fn uris(&self) -> Vec<String> {
        self.list()
            .iter()
            .map(|resource| resource.uri().to_string())
            .collect()
    }

    /// Lê o conteúdo de um resource.
    ///
    /// # Errors
    ///
    /// * [`ResourceError::NotFound`] se o URI não existir.
    /// * Erro propagado do [`Resource::read`].
    pub async fn read(&self, uri: &str) -> Result<ResourceContents, ResourceError> {
        match self.get(uri) {
            Some(resource) => resource.read().await,
            None => Err(ResourceError::NotFound(uri.to_string())),
        }
    }
}

impl fmt::Debug for ResourceRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResourceRegistry")
            .field("resources", &self.uris())
            .finish()
    }
}

/// Resource de teste com conteúdo fixo.
///
/// Útil para testes e exemplos de integração.
#[derive(Clone)]
pub struct StaticResource {
    uri: String,
    name: String,
    description: Option<String>,
    mime_type: Option<String>,
    contents: ResourceContents,
}

impl StaticResource {
    /// Cria um resource textual com conteúdo fixo.
    pub fn text(
        uri: impl Into<String>,
        name: impl Into<String>,
        description: Option<impl Into<String>>,
        mime_type: Option<impl Into<String>>,
        text: impl Into<String>,
    ) -> Self {
        let uri = uri.into();
        Self {
            uri: uri.clone(),
            name: name.into(),
            description: description.map(Into::into),
            mime_type: mime_type.map(Into::into),
            contents: ResourceContents::text(uri, text),
        }
    }
}

#[async_trait]
impl Resource for StaticResource {
    fn uri(&self) -> &str {
        &self.uri
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn mime_type(&self) -> Option<&str> {
        self.mime_type.as_deref()
    }

    async fn read(&self) -> Result<ResourceContents, ResourceError> {
        Ok(self.contents.clone())
    }
}

/// Resource de teste que falha ao ler.
///
/// Útil para testar propagação de erros.
#[derive(Clone)]
pub struct FailingResource {
    uri: String,
    message: String,
}

impl FailingResource {
    /// Cria um resource cuja leitura sempre falha.
    pub fn new(uri: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            message: message.into(),
        }
    }
}

#[async_trait]
impl Resource for FailingResource {
    fn uri(&self) -> &str {
        &self.uri
    }

    fn name(&self) -> &str {
        "Failing resource"
    }

    async fn read(&self) -> Result<ResourceContents, ResourceError> {
        Err(ResourceError::Read(self.uri.clone(), self.message.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_resource(uri: impl Into<String>) -> Arc<dyn Resource> {
        Arc::new(StaticResource::text(
            uri,
            "Teste",
            None::<String>,
            None::<String>,
            "Olá",
        ))
    }

    #[tokio::test]
    async fn register_and_get() {
        let registry = ResourceRegistry::new();
        let resource = text_resource("file:///ola.txt");

        registry.register(resource.clone()).unwrap();

        assert_eq!(
            registry.get("file:///ola.txt").unwrap().uri(),
            "file:///ola.txt"
        );
        assert_eq!(
            registry.read("file:///ola.txt").await.unwrap(),
            ResourceContents::text("file:///ola.txt", "Olá")
        );
    }

    #[tokio::test]
    async fn duplicate_uri_is_rejected() {
        let registry = ResourceRegistry::new();
        let resource = text_resource("file:///ola.txt");

        registry.register(resource).unwrap();
        let error = registry
            .register(text_resource("file:///ola.txt"))
            .unwrap_err();

        assert!(matches!(error, ResourceError::Duplicate(uri) if uri == "file:///ola.txt"));
    }

    #[test]
    fn empty_uri_is_rejected() {
        let registry = ResourceRegistry::new();

        let error = registry.register(text_resource("")).unwrap_err();

        assert!(matches!(error, ResourceError::InvalidResource(_)));
    }

    #[test]
    fn list_is_ordered_by_uri() {
        let registry = ResourceRegistry::new();
        registry.register(text_resource("file:///b.txt")).unwrap();
        registry.register(text_resource("file:///a.txt")).unwrap();
        registry.register(text_resource("file:///c.txt")).unwrap();

        assert_eq!(
            registry.uris(),
            vec!["file:///a.txt", "file:///b.txt", "file:///c.txt"]
        );
    }

    #[tokio::test]
    async fn read_unknown_uri_is_not_found() {
        let registry = ResourceRegistry::new();

        let error = registry.read("file:///missing.txt").await.unwrap_err();

        assert!(matches!(error, ResourceError::NotFound(uri) if uri == "file:///missing.txt"));
    }

    #[tokio::test]
    async fn read_error_is_propagated() {
        let registry = ResourceRegistry::new();
        let resource = Arc::new(FailingResource::new("file:///erro.txt", "IO falhou"));
        registry.register(resource).unwrap();

        let error = registry.read("file:///erro.txt").await.unwrap_err();

        assert!(
            matches!(error, ResourceError::Read(uri, reason) if uri == "file:///erro.txt" && reason == "IO falhou")
        );
    }

    #[test]
    fn unregister() {
        let registry = ResourceRegistry::new();
        registry.register(text_resource("file:///x.txt")).unwrap();

        assert!(registry.unregister("file:///x.txt"));
        assert!(!registry.unregister("file:///x.txt"));
        assert!(registry.get("file:///x.txt").is_none());
    }
}
