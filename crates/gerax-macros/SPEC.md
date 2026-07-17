# gerax-macros — Specification

## Contratos

Macros proc-macro para reduzir boilerplate.

- implementa automaticamente os metodos das entidades.
- registra rotas de forma declarativa.

## Regras
- Erros de macro devem apontar para a linha/coluna correta.
- Deve gerar código que compila.

## Testes Esperados
- Teste de macro em camada separada.
- Teste de expansão de macro com entidade complexa (com generics, lifetimes).
