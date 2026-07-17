---
name: gerax-macros
description: Implementa proc-macros para a crate gerax-macros seguindo o spec SPEC.md
---

# Skill: gerax-macros

Use quando precisar implementar ou modificar as proc-macros da crate `gerax-macros`.

## Especificação

Leia `crates/gerax-macros/SPEC.md` antes de implementar.

## Contratos obrigatórios

Implemente proc-macros que reduzem boilerplate:

1. **Macro de entidade**: implementa automaticamente a trait `gerax_core::Entity` para structs.
   - Gera `collection_name()` a partir do nome da struct em snake_case + `s` (ex: `User` -> `"users"`).
   - Gera `id(&self)` retornando `self.id.clone()` quando o campo `id` for `Option<String>`.
   - Gera `set_id(&mut self, id: String)` atribuindo `self.id = Some(id)`.

2. **Macro de rotas**: registra rotas de forma declarativa (quando aplicável ao framework escolhido).

## Regras

- Use `syn` para parsear entrada, `quote` para gerar saída e `proc-macro2` para spans.
- Erros de macro devem usar `syn::Error::new_spanned` ou `proc_macro::Diagnostic` para apontar linha/coluna correta.
- Código gerado **deve compilar**.
- A macro de entidade deve aceitar atributos como `#[entity(collection_name = "custom_name")]` para sobrescrever o nome da coleção.
- A macro deve preservar derives existentes (ex: `#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]`).

## Casos de erro esperados

- Struct sem campo `id` do tipo `Option<String>` (erro apontando o struct).
- Atributo `#[entity(...)]` malformado.

## Testes esperados

- Teste de macro em camada separada usando `trybuild` ou `syn` parse para verificar spans de erro.
- Teste de expansão de macro com entidade complexa (struct com generics, lifetimes).
- Teste unitário verificando que `collection_name()`, `id()` e `set_id()` funcionam corretamente para uma entidade simples.
- Teste que código gerado compila (`cargo check` no diretório de testes).
