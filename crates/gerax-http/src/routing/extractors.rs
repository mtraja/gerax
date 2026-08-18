use bytes::Bytes;
use serde::de::DeserializeOwned;
use std::convert::Infallible;
use std::str::FromStr;
use std::sync::Arc;

use super::Context;

/// Trait base para extrair valores do `Context<S>`.
///
/// Um tipo implementa `FromContext` para declarar como é obtido a partir
/// da requisição. O tipo associado `Rejection` representa o erro quando a
/// extração falha.
///
/// # Sintaxe em handlers
///
/// ```ignore
/// async fn handler(Path((id, name)): Path<(u64, String)>, State(state): State<AppState>) {}
/// ```
///
/// Quando o parâmetro do handler implementa `FromContext`, o framework
/// injeta automaticamente o valor extraído do contexto da requisição.
pub trait FromContext<S>: Sized {
    type Rejection;

    fn from_context(ctx: &Context<S>) -> Result<Self, Self::Rejection>;
}

use std::fmt;

/// Erros de extração de dados da requisição.
///
/// - `Deserialize`: falha ao converter os dados brutos no tipo alvo.
/// - `Missing`: parâmetro obrigatório não encontrado.
/// - `Invalid`: valor presente, porém malformado.
#[derive(Debug)]
pub enum ExtractError {
    Deserialize(String),
    Missing(String),
    Invalid(String),
}

impl fmt::Display for ExtractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Deserialize(message) => {
                write!(f, "failed to deserialize request data: {message}")
            }

            Self::Missing(name) => {
                write!(f, "missing request parameter: {name}")
            }

            Self::Invalid(message) => {
                write!(f, "invalid request data: {message}")
            }
        }
    }
}

impl From<serde_json::Error> for ExtractError {
    fn from(error: serde_json::Error) -> Self {
        Self::Deserialize(error.to_string())
    }
}

impl From<serde_urlencoded::de::Error> for ExtractError {
    fn from(error: serde_urlencoded::de::Error) -> Self {
        Self::Deserialize(error.to_string())
    }
}

impl std::error::Error for ExtractError {}

/// Extrai o estado compartilhado da aplicação.
///
/// O estado é injetado no handler como `State<S>`, onde `S` é o tipo do
/// estado configurado no servidor.
///
/// # Sintaxe
///
/// ```ignore
/// async fn handler(State(state): State<AppState>) {}
/// ```
///
/// # Uso
///
/// Dentro do handler você acessa o estado via `.0`:
///
/// ```ignore
/// let db = state.db.clone();
/// ```
pub struct State<S>(pub Arc<S>);

impl<S> FromContext<S> for State<S>
where
    S: Send + Sync + 'static,
{
    type Rejection = Infallible;

    fn from_context(ctx: &Context<S>) -> Result<Self, Self::Rejection> {
        Ok(State(ctx.state()))
    }
}

/// Extrai parâmetros de rota (`path parameters`).
///
/// Os parâmetros definidos no padrão da rota (ex: `/users/:id`) são
/// desserializados no tipo `T`.
///
/// # Sintaxe
///
/// ```ignore
/// async fn handler(Path(params): Path<MyParams>) {}
/// ```
///
/// Para múltiplos parâmetros, use uma tupla ou struct:
///
/// ```ignore
/// async fn handler(Path((id, name)): Path<(u64, String)>) {}
/// ```
///
/// # Uso
///
/// ```ignore
/// #[derive(Deserialize)]
/// struct MyParams { id: u64 }
///
/// async fn handler(Path(params): Path<MyParams>) {
///     println!("id = {}", params.id);
/// }
/// ```
pub struct Path<T>(pub T);

impl<S, T> FromContext<S> for Path<T>
where
    T: DeserializeOwned,
{
    type Rejection = ExtractError;

    fn from_context(ctx: &Context<S>) -> Result<Self, Self::Rejection> {
        let value = ctx.params().deserialize()?;

        Ok(Path(value))
    }
}

/// Extrai parâmetros da query string da URL.
///
/// A query string (parte após `?` na URL) é desserializada no tipo `T`
/// usando `serde_urlencoded`.
///
/// # Sintaxe
///
/// ```ignore
/// async fn handler(Query(params): Query<PaginationQuery>) {}
/// ```
///
/// # Uso
///
/// ```ignore
/// #[derive(Deserialize)]
/// struct PaginationQuery { page: u32, limit: u32 }
///
/// // GET /items?page=1&limit=10
/// async fn handler(Query(params): Query<PaginationQuery>) {
///     println!("page = {}, limit = {}", params.page, params.limit);
/// }
/// ```
pub struct Query<T>(pub T);

impl<S, T> FromContext<S> for Query<T>
where
    T: DeserializeOwned,
{
    type Rejection = ExtractError;

    fn from_context(ctx: &Context<S>) -> Result<Self, Self::Rejection> {
        let value = serde_urlencoded::from_str(&ctx.request().query)?;

        Ok(Query(value))
    }
}
/// Extrai o corpo da requisição como JSON.
///
/// O body é desserializado diretamente via `serde_json`. O tipo `T`
/// deve implementar `DeserializeOwned`.
///
/// # Sintaxe
///
/// ```ignore
/// async fn handler(Json(payload): Json<CreateUserInput>) {}
/// ```
///
/// # Uso
///
/// ```ignore
/// #[derive(Deserialize)]
/// struct CreateUserInput { name: String, email: String }
///
/// // POST /users com body: {"name":"Maria","email":"maria@example.com"}
/// async fn handler(Json(payload): Json<CreateUserInput>) {
///     let name = payload.name;
/// }
/// ```
pub struct Json<T>(pub T);

impl<S, T> FromContext<S> for Json<T>
where
    T: DeserializeOwned,
{
    type Rejection = ExtractError;

    fn from_context(ctx: &Context<S>) -> Result<Self, Self::Rejection> {
        let value = serde_json::from_slice(&ctx.request().body)?;

        Ok(Json(value))
    }
}

/// Extrai o corpo da requisição como formulário URL-encoded.
///
/// O body é desserializado com `serde_urlencoded`. Útil para formulários
/// HTML tradicionais (`application/x-www-form-urlencoded`).
///
/// # Sintaxe
///
/// ```ignore
/// async fn handler(Form(form): Form<LoginForm>) {}
/// ```
///
/// # Uso
///
/// ```ignore
/// #[derive(Deserialize)]
/// struct LoginForm { username: String, password: String }
///
/// // POST /login com body: username=admin&password=123
/// async fn handler(Form(form): Form<LoginForm>) {
///     println!("user = {}", form.username);
/// }
/// ```
pub struct Form<T>(pub T);

impl<S, T> FromContext<S> for Form<T>
where
    T: DeserializeOwned,
{
    type Rejection = ExtractError;

    fn from_context(ctx: &Context<S>) -> Result<Self, Self::Rejection> {
        let value = serde_urlencoded::from_bytes(&ctx.request().body)
            .map_err(|err| ExtractError::Deserialize(err.to_string()))?;

        Ok(Form(value))
    }
}

/// Extrai um header HTTP específico pelo nome.
///
/// O valor do header é convertido para o tipo `T` via `FromStr`.
///
/// # Sintaxe
///
/// ```ignore
/// async fn handler(Header(content_type): Header<String>) {}
/// ```
///
/// # Uso
///
/// ```ignore
/// // GET /qualquer-coisa com header `Authorization: Bearer abc123`
/// async fn handler(Header(token): Header<String>) {
///     println!("token = {}", token);
/// }
///
/// // Para headers com nomes específicos:
/// async fn handler(Header(ua): Header<String>) {
///     let ua = ctx.request().headers().get("User-Agent").unwrap();
/// }
/// ```
pub struct Header<T>(pub T);

impl<T> Header<T>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    pub fn from_name<S>(ctx: &Context<S>, header_name: &str) -> Result<Self, ExtractError> {
        let value = ctx
            .request()
            .headers()
            .get(header_name)
            .ok_or_else(|| ExtractError::Missing(header_name.to_string()))?;

        value
            .parse::<T>()
            .map(Header)
            .map_err(|err| ExtractError::Deserialize(err.to_string()))
    }
}

/// Extrai o corpo bruto da requisição como `Bytes`.
///
/// Útil quando você precisa acessar os dados crus sem desserialização,
/// por exemplo para fazer upload de arquivos ou processar payloads
/// customizados.
///
/// # Sintaxe
///
/// ```ignore
/// async fn handler(RawBody(body): RawBody) {}
/// ```
///
/// # Uso
///
/// ```ignore
/// async fn handler(RawBody(body): RawBody) {
///     // body é do tipo `Bytes`
///     println!("tamanho do body: {}", body.len());
/// }
/// ```
pub struct RawBody(pub Bytes);

impl<S> FromContext<S> for RawBody {
    type Rejection = ExtractError;

    fn from_context(ctx: &Context<S>) -> Result<Self, Self::Rejection> {
        Ok(RawBody(Bytes::from(ctx.request().body.clone())))
    }
}

use super::Request;

/// Extrai a requisição completa.
///
/// Permite acesso direto a todos os campos da requisição: método, path,
/// headers, query, body, etc.
///
/// # Sintaxe
///
/// ```ignore
/// async fn handler(Request(req): Request) {}
/// ```
///
/// # Uso
///
/// ```ignore
/// async fn handler(Request(req): Request) {
///     println!("method = {:?}", req.method());
///     println!("path = {}", req.path());
///     println!("query = {}", req.query());
/// }
/// ```
impl<S> FromContext<S> for Request {
    type Rejection = Infallible;

    fn from_context(ctx: &Context<S>) -> Result<Self, Self::Rejection> {
        Ok(ctx.request().clone())
    }
}
