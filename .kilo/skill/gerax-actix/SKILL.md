# gerax-actix — Habilidade em Português

## Especificação (Spec)
- Implementar as abstrações de `gerax-http` para o framework Actix Web.
- Servidor HTTP recebe estado compartilhado na inicialização.
- Método de inicialização assíncrono bloqueia até encerramento ou erro.
- Rotas construídas a partir do estado compartilhado.
- Configuração de rotas via encadeamento (builder pattern).
- Inicialização padrão não altera o estado quando nenhuma rota configurada.
- Dependências apenas de `gerax-core`, `gerax-config`, `gerax-http`  e `actix-web`.
- Não vazar tipos específicos do Actix na API pública.
- Porta configurável com os recursos da crate `gerax-config` com padrao `0.0.0.0:8080`.
- Estado compartilhado seguro para concorrência.
- Erros usam hierarquia de `gerax-http`.

## Implementação (Impl)
1. Definir crate `gerax-actix` com dependências necessárias.
2. Reexportar ou usar traits de `gerax-http`: `HttpServer`, `HttpServerBuilder`, `GeraxHttpError`.
3. Criar struct `ActixHttpServer<S>` contendo:
   - Configurações de endereço (host/port).
   - Referência ao `actix_web::App` ou `HttpServer` interno.
   - Estado compartilhado `S` (via Arc etc).
4. Implementar trait `HttpServer<S>` para `ActixHttpServer<S>`:
   - Método `listen(&mut self, state: S)` assíncrono:
     * Construir `actix_web::App` usando estado (rotas, middlewares) via `configure_routes`.
     * Vincular (`bind`) ao endereço configurado.
     * Iniciar servidor (`run`) e aguardar shutdown/erro.
     * Converter erros do Actix para `GeraxHttpError` (Bind, Runtime, Shutdown).
   - Método `configure_routes(&mut self, state: &S)`:
     * Chamar função de rotas fornecida pelo usuário via closure/trait.
     * Permitir extensão de rotas via state (ex: rotas definidas no estado).
5. Criar builder `ActixHttpServerBuilder<S>` implementando `HttpServerBuilder<S>`:
   - Métodos `with_middleware`, `with_option` para armazenar configurações.
   - Método `build` produzindo `ActixHttpServer<S>`.
6. Garantir que tipos internos do Actix não vazem: usar apenas tipos públicos da trait.
7. Porta padrão `0.0.0.0:8080` configurável via `with_option`.
8. Usar `actix_web::App` com `data<S>` para compartilhar estado.
9. Lidar com graceful shutdown usando actix signals.

## Testes (Test)
- Teste unitário: garantir que `configure_routes` padrão (no-op) não muta estado.
- Teste unitário: builder permite encadeamento de middlewares e opções.
- Teste de integração: usar porta aleatória (`0.0.0.0:0`), subir servidor, fazer requisição HTTP (ex: `/`) e validar status 200.
- Teste de rota protegida (se autenticação habilitada): validar que rota requer auth retorna 401/403 adequadamente.
- Teste: inicialização sem rotas não altera estado compartilhado.

## Pular (Skip)
- Integração com outros frameworks além do Actix Web.
- Suporte a HTTPS nativo (deixar para camada de proxy ou opções avançadas).
- Recursos avançados do Actix como WebSockets, HTTP/2 (futuras extensões).
- Geração automática de documentação OpenAPI.
- Middleware de logging detalhado (pode ser adicionado via opções).