# Skill: Implementar o crate `gerax-rpc`

## Objetivo

Implementar o crate `crates/gerax-rpc`, responsável por definir as
abstrações RPC reutilizadas por todas as implementações do Gerax.

O crate **não implementa** gRPC, Cap'n Proto RPC, HTTP, QUIC, TCP ou
qualquer protocolo específico.

## Objetivos Arquiteturais

-   Independente de protocolos.
-   Independente de serialização.
-   Independente de runtime assíncrono.
-   Reutilizável por `gerax-grpc`, `gerax-capnp` e futuras
    implementações.

## Estrutura

``` text
gerax-rpc
├── src
│   ├── client.rs
│   ├── server.rs
│   ├── service.rs
│   ├── method.rs
│   ├── transport.rs
│   ├── request.rs
│   ├── response.rs
│   ├── stream.rs
│   ├── status.rs
│   ├── error.rs
│   ├── metadata.rs
│   ├── context.rs
│   ├── extensions.rs
│   └── lib.rs
├── tests
├── examples
└── Cargo.toml
```

## Dependências Permitidas

-   std
-   gerax-core

Não utilizar:

-   prost
-   tonic
-   capnp
-   hyper
-   tokio
-   serde

## Backlog

1.  Criar o crate.
2.  Implementar `RpcRequest<T>`.
3.  Implementar `RpcResponse<T>`.
4.  Implementar `RpcStatus`.
5.  Implementar `RpcMetadata`.
6.  Implementar `RpcContext`.
7.  Implementar `RpcExtensions`.
8.  Implementar `RpcMethod`.
9.  Implementar `RpcService`.
10. Implementar `RpcServer`.
11. Implementar `RpcClient`.
12. Implementar `RpcTransport`.
13. Implementar `RpcStream`.
14. Implementar `RpcError`.
15. Implementar builders.
16. Criar testes.
17. Criar exemplos.
18. Documentar toda a API.

## Integração

``` text
gerax-grpc
      │
      ▼
  gerax-rpc
      ▲
      │
gerax-capnp
```

## Critérios de Aceitação

-   Todos os testes passam.
-   `cargo fmt --check`
-   `cargo clippy --workspace --all-features -D warnings`
-   `cargo doc --workspace --all-features`
-   Sem dependências de transporte ou protocolo.
-   API pública documentada.
-   Pronto para ser implementado por `gerax-grpc` e `gerax-capnp`.

## Restrições Arquiteturais

-   Não implementar transporte.
-   Não implementar serialização.
-   Não implementar protocolo RPC específico.
-   Apenas contratos, modelos e abstrações.
