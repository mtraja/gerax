# Skill: Implementar o crate `gerax-codec`

## Objetivo

Implementar o crate **`crates/gerax-codec`**, responsável por fornecer
uma abstração unificada para serialização e desserialização de dados no
framework Gerax.

### Requisitos

-   Independente de `gerax-http`, `gerax-grpc`, `gerax-capnp`,
    `gerax-db` e `gerax-openapi`.
-   Suportar múltiplos formatos de serialização.
-   Registro dinâmico de codecs.
-   Arquitetura extensível e de baixo acoplamento.

## Estrutura esperada

``` text
gerax-codec
├── src
│   ├── codec.rs
│   ├── encoder.rs
│   ├── decoder.rs
│   ├── registry.rs
│   ├── media_type.rs
│   ├── negotiation.rs
│   ├── error.rs
│   ├── serde/
│   ├── schema/
│   └── lib.rs
├── examples/
├── tests/
└── Cargo.toml
```

## Arquitetura

``` text
          CodecRegistry
                 │
      ┌──────────┼──────────────┐
      ▼          ▼              ▼
   JsonCodec  CborCodec   ProtobufCodec
                              │
                       CapnpCodec
```

## Tarefas

1.  Criar o crate `gerax-codec`.
2.  Implementar `Codec`.
3.  Implementar `Encoder<T>`.
4.  Implementar `Decoder<T>`.
5.  Implementar `MediaType`.
6.  Implementar `CodecRegistry`.
7.  Implementar `CodecNegotiator`.
8.  Implementar `CodecError`.
9.  Implementar `JsonCodec`.
10. Implementar `CborCodec`.
11. Implementar `MessagePackCodec`.
12. Implementar `BsonCodec`.
13. Implementar `ProtobufCodec` usando `prost`.
14. Implementar `CapnpCodec` usando `capnp`.
15. Preparar infraestrutura para `FlatBuffers`.
16. Configurar Cargo Features.
17. Criar testes unitários e de integração.
18. Criar benchmarks com Criterion.
19. Documentar toda a API pública.
20. Criar exemplos completos.

## Cargo Features

``` toml
[features]

default = ["json"]

json = []
cbor = []
msgpack = []
bson = []
protobuf = []
capnp = []
flatbuffers = []
```

## Critérios de Aceitação

-   Todos os testes passam.
-   `cargo fmt --check`.
-   `cargo clippy --workspace --all-features -D warnings`.
-   `cargo doc --workspace --all-features`.
-   Registro dinâmico de codecs funcionando.
-   Negociação por `Accept` e `Content-Type`.
-   Sem dependência de protocolos de transporte.

## Restrições Arquiteturais

-   Não depender de HTTP, gRPC ou QUIC.
-   Utilizar `CodecRegistry` para resolução de codecs.
-   Separar codecs baseados em `serde` dos baseados em schema.
-   Facilitar adição de novos formatos sem alterar o núcleo.
