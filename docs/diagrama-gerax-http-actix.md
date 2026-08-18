# Gerax HTTP + Actix: diagrama didático

`gerax-http` é a camada de abstração: descreve como um servidor, as rotas,
os middlewares e os handlers se comportam, sem depender de um servidor web
concreto. `gerax-actix` é um adaptador que materializa essas abstrações em
`actix-web`.

## 1. Construção do servidor

```mermaid
flowchart LR
    APP[Aplicação\nState + Router + configuração]
    B[ActixHttpServerBuilder]
    C[ServerConfig\nhost e port]
    R[Router<State>]
    M[Middlewares de servidor]
    S[ActixHttpServer]
    A[actix_web::HttpServer]

    APP --> B
    C --> B
    R --> B
    M --> B
    B -->|build| S
    S -->|run| A
```

O `ActixHttpServerBuilder` implementa o trait `HttpServerBuilder<State>` de
`gerax-http`. Ao construir o servidor, o estado passa a ser compartilhado por
`Arc<State>` e o `Router<State>` também é guardado em `Arc`.

## 2. Modelo de rotas em `gerax-http`

```mermaid
flowchart TD
    Router[Router<State>]
    RootRoute[Route\nGET /health]
    Scope[Scope /api]
    ChildRoute[Route\nGET /users/:id]
    Nested[Scope /v1]
    NestedRoute[Route\nPOST /orders]
    Trie[matchit::Router\nradix trie]

    Router --> RootRoute
    Router --> Scope
    Scope --> ChildRoute
    Scope --> Nested
    Nested --> NestedRoute
    Router -->|achata rotas e registra padrões| Trie
```

Cada `Route` reúne método HTTP, padrão de caminho, handler e middlewares
próprios. Um `Scope` adiciona prefixo e pode acumular rotas, subescopos e
middlewares. Para o roteamento puro, o `Router` achata essa árvore e cria o
matcher `matchit`; no match, ele também preenche os parâmetros de rota no
`Context`.

## 3. Caminho de uma requisição no adaptador Actix

```mermaid
sequenceDiagram
    participant Client as Cliente HTTP
    participant Actix as actix-web
    participant Adapter as gerax-actix::route_handler
    participant Context as gerax_http::Context<State>
    participant Chain as Cadeia de middleware
    participant Handler as Handler

    Client->>Actix: GET /api/users/42?full=true
    Actix->>Adapter: HttpRequest + Bytes do body
    Adapter->>Adapter: converte método e cria Request
    Adapter->>Context: Context::new(Arc<State>, Request)
    Adapter->>Chain: Route::execute(context)
    Chain->>Chain: middleware 1 (pré)
    Chain->>Chain: middleware 2 (pré)
    Chain->>Handler: call(context)
    Handler-->>Chain: ServerResult<Response>
    Chain-->>Chain: middleware 2 (pós)
    Chain-->>Chain: middleware 1 (pós)
    Chain-->>Adapter: Response { status, body }
    Adapter-->>Actix: HttpResponse
    Actix-->>Client: resposta HTTP
```

Para uma rota direta, os middlewares são concatenados nesta ordem:

```text
middlewares da rota -> middlewares do Router -> middlewares do servidor
```

Como `Route::execute` monta a continuação em ordem reversa, a execução de
entrada ocorre exatamente na ordem acima; o retorno percorre a ordem inversa.
Um middleware pode não chamar `next.call(ctx)` e devolver uma `Response`,
encerrando a requisição antecipadamente.

Em rotas dentro de um `Scope`, o adaptador usa esta ordem de composição:

```text
middlewares da rota -> middlewares do Scope -> middlewares do Router -> middlewares do servidor
```

## 4. Dados disponíveis ao handler

```mermaid
classDiagram
    class Context~State~ {
        +Arc~State~ state
        +Request request
        +PathParams params
        +Extensions extensions
    }
    class Request {
        +HttpMethod method
        +String path
        +HeaderMap headers
        +String query
        +Vec~u8~ body
    }
    class FromContext~S~ {
        <<trait>>
        +from_context(Context) Result
    }
    class State~S~
    class Path~T~
    class Query~T~
    class Json~T~
    class Form~T~
    class RawBody
    class Header~T~

    Context *-- Request
    FromContext~S~ <|.. State~S~
    FromContext~S~ <|.. Path~T~
    FromContext~S~ <|.. Query~T~
    FromContext~S~ <|.. Json~T~
    FromContext~S~ <|.. Form~T~
    FromContext~S~ <|.. RawBody
    Header~T~ ..> Context : from_name(ctx, nome)
```

`Path`, `Query`, `Json`, `Form` e `RawBody` implementam `FromContext`.
`Header<T>` é extraído explicitamente com `Header::from_name(&ctx, "nome")`.

## Observações sobre a implementação atual

- O `Router::handle` de `gerax-http` usa a radix trie, faz o match por caminho
  e método, e coloca parâmetros como `:id` no `Context`.
- No caminho executado por `gerax-actix`, o Actix faz o match das rotas
  registradas e chama diretamente `Route::execute`; portanto, o adaptador não
  chama `Router::handle` e não transfere os headers do `HttpRequest` para o
  `gerax_http::Request`.
- `gerax-actix` registra as rotas diretas do `Router` e as rotas diretas de
  cada `Scope` de primeiro nível. Subescopos existentes no modelo puro de
  `gerax-http` não são registrados pelo adaptador atual.
- `Options` é convertido para `Method::OPTIONS`, mas o registro de handlers do
  adaptador não tem um ramo específico para ele (cai no fallback `GET`).

Esses últimos pontos descrevem o código atual e são bons candidatos a testes
de integração ou a uma evolução do adaptador.
