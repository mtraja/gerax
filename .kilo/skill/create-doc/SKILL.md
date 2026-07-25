---
name: create-doc
description: Cria ou atualiza a documentação de interface pública de qualquer crate do Gerax no formato usado por gerax-config (README.md)
---

# Skill: create-doc

Use quando precisar gerar, atualizar ou revisar a documentação de crate do Gerax.

## Objetivo

Gerar um `README.md` no diretório do crate contendo **somente a interface pública**: tipos públicos, traits públicas, métodos públicos, fontes (se aplicável), erros e exemplos.

## Regras

- Não documente módulos internos, helpers privados ou estruturas internas.
- Não inclua roadmap, TODO ou decisões de implementação.
- Não adicione explicações sobre o funcionamento interno.
- Mantenha o formato consistente com `crates/gerax-config/README.md`.

## Passos

1. **Explorar público**

   Leia os arquivos fonte do crate para identificar tudo que é marcado como `pub`:
   - `pub struct`
   - `pub trait`
   - `pub enum`
   - `pub type`
   - `pub fn`
   - Implementações públicas em tipos públicos

   Priorize: `src/lib.rs`, `src/*.rs`, `src/**/*.rs`.

2. **Ler o manifesto**

   Abra `Cargo.toml` da crate para incluir:
   - Dependências gerais (gerax-core, async-trait, serde, thiserror, dotenvy, etc.)
   - Optional dependencies seguidas de suas features
   - Tabela de features com descrição curta

3. **Montar seções obrigatórias**

   Crie o arquivo `README.md` no diretório da crate seguindo esta ordem:

   ### Cabeçalho

   ```markdown
   # nome-do-crate

   Descrição curta do propósito da crate.
   ```

   ### Dependências

   ```rust
   nome-do-crate = { path = "../crates/nome-do-crate" }
   ```

   ### Features

   Tabela com feature, dependência e descrição. Inclua exemplo de uso customizado.

   ### Visão Geral

   Um ou dois parágrafos sobre o papel da crate no ecossistema. Inclua fluxo ASCII se ajudar, mas apenas com tipos públicos:

   ```
   PublicApi -> PublicBuilder -> PublicLoader -> T
   ```

   ### API

   Liste **exatamente** os itens públicos encontrados na etapa 1:
   - Tipo público como título `### Tipo`
   - Lista de métodos públicos com assinatura resumida e propósito
   - Traits públicas com seus métodos obrigatórios

   Nome de métodos e tipos devem ser codificados em backticks com a letra minúscula conforme nome real em Rust.

   ### Fontes (somente para crates com sources)

   Se a crate tiver implementações `ConfigSource` ou fontes plugáveis, documente cada uma com exemplo mínimo.

   ### Erros

   Liste a `ConfigResult` / `Result` usada e todas as variantes públicas do enum de erro, se existir.

   ### Exemplos

   Inclua no mínimo um exemplo funcional ou estático que compile e use apenas a interface pública.

4. **Validar**

   - Todos os nomes públicos do crate devem aparecer no README.
   - Nenhum módulo interno deve aparecer.
   - Nenhum item privado deve ser citado.
   - Não deve conter informações sensíveis ou internas de workspace além das públicas.
   - Não deve conter dicas de implementação, nem comentários tipo "fazer", "TODO", "futuro".

5. **Escrever**

   Crie ou sobrescreva o `README.md` do crate com o conteúdo final.

## Formato da seção de API (padrão)

```markdown
### NomeDoTipo

Propósito breve.

```rust
// exemplo mínimo se couber
```
```

Liste métodos públicos embaixo:

- `metodo(self, arg: Tipo) -> Retorno` — descrição curta.

## Formato da seção de erros (padrão)

```markdown
NomeResultado<T> = Result<T, NomeError>

Variações de NomeError:
- `Variant { campo, ... }` — quando ocorre.
```
