# gerax-mongodb — Specification

## Contratos

Implementa a interface de persistência de entidades para banco de dados PostgreSQL, atendendo todos os metodos da interface Repository para atender os requisitos definidos.

## Regras
- Ajuste das camadas dependentes
- Não deve bloquear o thread (tudo async).
- Deve tratar erros de conexão e prover as devidas conversoes.
- A conexao deve atender aos parametros do SGBD.
- Os parametros de autenticao deve ser definidos em arquivos para garante seguranca.
- O nome do bando de dados sera definido em arquivo.

## Testes Esperados
