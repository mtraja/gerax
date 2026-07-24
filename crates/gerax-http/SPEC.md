# gerax-http — Specification

## Contratos

- Define uma interface abstrata para conexão com frameworks HTTP.
- Um servidor HTTP recebe um estado compartilhado no momento de inicialização.
- O estado compartilhado é fornecido como parâmetro do método de inicialização.
- A inicialização inicia o servidor e bloqueia até encerramento ou erro.
- Permite encadeamento de configuração.
- Emprega padrões de projeto como builder pattern, facade, etc.
- As rotas são construídas a partir do estado compartilhado.
- Aplica configuração de rotas; a implementação padrão é no-op.

## Regras

- Não deve conhecer detalhes de framework.
- Deve ser assegurado as dependências entre camadas e módulos.
- A codificação deve ser independente de tecnologia.
- A aplicação escolhe a tecnologia HTTP.
- O estado compartilhado deve ser seguro para uso concorrente quando aplicável.
- Erros são representados por uma hierarquia própria.
- Métodos são assíncronos quando a linguagem/plataforma suportar.
- Permite configurar middlewares ou opções antes de rodar
- Estrutura de Arquivos do Módulo segue a organização canônica e modular do ecossistema:
   gerax-http
    └── src
        ├── error
        ├── builder
        ├── middleware
        ├── router
        └── server

## Testes Esperados

- Mock que apenas registra chamadas.
- Teste garantindo que a implementação padrão não altera o estado.
