# Plano de Construção — `gerax-codec`

## 1. Visão Geral

`gerax-codec` é a camada de serialização/deserialização do ecossistema Gerax.
Fornece a trait `Codec<T>` e implementações para múltiplos formatos, permitindo que
tipos de domínio sejam codificados/decodificados sem acoplamento a um formato específico.

**Estado atual:** Core funcional (JSON, YAML, TOML, Protobuf), Cap'n Proto como stub,
sem testes, sem fábrica de codecs, sem feature flags.

---

## 2. Objetivos de Construção

- **Abstração:** `Codec<T>` como ponto único de integração.
- **Extensibilidade:** novos formatos devem ser adicionados sem modificar código existente.
- **Testabilidade:** cada codec deve ter testes unitários e de integração.
- **Documentação:** cada público item deve ter doc examples e README atualizado.
- **Performance:** codecs binários devem usar buffers previamente alocados quando possível.

---

## 3. Fases de Construção

### Fase 1 — Core Sólido

**Tarefas:**
1. Revisar `Codec<T>` trait: adicionar doc comments em todos os métodos.
2. Definir estratégia de erro: manter `CodecError` como enum ou `thiserror`.
3. Adicionar feature flags em `Cargo.toml` para dependências opcionais.
4. Criar módulo `src/error.rs` e mover `CodecError` para lá.
5. Adicionar `#[must_use]` em todos os retornos de `Result` na trait.

**Critério de aceitação:** `cargo check --all-features` sem warnings.

---

### Fase 2 — Codecs Base

**Tarefas:**
1. **JSON (`src/json.rs`)** — Pronto. Adicionar testes de round-trip.
2. **YAML (`src/yaml.rs`)** — Pronto. Adicionar testes de round-trip.
3. **TOML (`src/toml.rs`)** — Pronto. Adicionar testes de round-trip.
4. **Protobuf (`src/protobuf.rs`)** — Pronto. Adicionar doc comment no bound `Default`.
5. **Cap'n Proto (`src/capnp.rs`)** — Manter como stub documentado até schema/capnpc.

**Para cada codec:**
- Teste serialize + deserialize round-trip com struct simples.
- Teste round-trip com struct contendo `Vec`, `Option`, `HashMap`.
- Teste erro de deserialização com bytes inválidos.
- Doc example em `lib.rs` re-exportando os codecs públicos.

**Critério de aceitação:** `cargo test --all-features` com cobertura de codecs de texto.

---

### Fase 3 — Extensibilidade

**Tarefas:**
1. Criar `src/factory.rs` com enum `CodecKind` e função `fn codec(kind: CodecKind) -> Box<dyn Codec<T>>`.
2. Suportar codecs habilitados via feature flags na factory.
3. Avaliar necessidade de `BincodeCodec` (feature `bincode` opcional).
4. Avaliar necessidade de `MessagePackCodec` (feature `rmp` opcional).

**Critério de aceitação:** `CodecKind::Json` retorna `JsonCodec`, etc., sem alocação desnecessária para tipos concretos.

---

### Fase 4 — Testes e Benchmarks

**Tarefas:**
1. Testes de integração em `tests/`:
   - `codec_text.rs` — JSON, YAML, TOML round-trip.
   - `codec_binary.rs` — Protobuf round-trip.
   - `codec_error.rs` — verificar `From` impls e mensagens.
2. Adicionar `criterion` como dev-dependency (feature `bench`).
3. Benchmark serialize/deserialize para cada codec com struct média (3 campos, 1 Vec).

**Critério de aceitação:** `cargo test --all-features` verde, benchmarks rodam sem crash.

---

### Fase 5 — Documentação

**Tarefas:**
1. Manter `README.md` atualizado com exemplos.
2. Documentar cada codec no nível do módulo (`//!` comments).
3. Adicionar guia "Adicionando um novo codec" no `README.md`.
4. Revisar `docs.rs` gerada (após publish).

**Critério de aceitação:** `cargo doc --no-deps` sem warnings, README com seção de extensão.

---

## 4. Padrões de Arquitetura

```
gerax-codec/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs          # re-exports públicos
│   ├── codec.rs        # trait Codec<T>
│   ├── error.rs        # CodecError + From impls
│   ├── factory.rs      # CodecKind + codec() (futuro)
│   ├── json.rs
│   ├── yaml.rs
│   ├── toml.rs
│   ├── protobuf.rs
│   ├── capnp.rs
│   └── bincode.rs      # futuro, feature-gated
├── tests/
│   ├── codec_text.rs
│   ├── codec_binary.rs
│   └── codec_error.rs
└── benches/
    └── codec_bench.rs
```

**Convenções:**
- Cada codec deve implementar `Codec<T>` e ser `pub struct` sem campos.
- Erros devem ser convertidos via `?` graças a `From` impls em `CodecError`.
- Features devem ser nomeadas por formato: `json`, `yaml`, `toml`, `protobuf`, `capnp`.
- Feature default: `["json", "protobuf"]`.

---

## 5. Critérios de Aceitação Finais

- `cargo check --all-features` sem warnings.
- `cargo test --all-features` 100% verde.
- `cargo doc --no-deps` sem warnings.
- `README.md` atualizado com tabela de codecs e exemplo de extensão.
- Nenhuma breaking change em `Codec<T>` ou `CodecError` após Fase 2.

---

## 6. Riscos e Mitigações

| Risco | Mitigação |
|-------|-----------|
| Mudança em `prost` quebra bound `Default` | Version lock em `Cargo.toml` + CI com `cargo outdated` |
| `capnp` sem schema real | Manter como stub ativamente documentado |
| Crate cresce muito | Feature flags por formato, manter default enxuto |
