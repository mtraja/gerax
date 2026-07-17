#name: gerax-http
description: Gera e mantém o módulo gerax-http em Rust — uma camada de abstração (porta/port, no estilo hexagonal) para conexão com frameworks HTTP, totalmente independente de tecnologia. Use esta skill sempre que o usuário pedir para criar, revisar ou estender o módulo/crate "gerax-http", pedir uma "interface abstrata de servidor HTTP em Rust" desacoplada de framework, mencionar termos como "estado compartilhado", "trait HttpServer", "builder de servidor HTTP", ou pedir para integrar um framework concreto (axum, actix-web, warp, etc.) como adaptador dessa abstração. Também use ao gerar testes que verifiquem que a implementação padrão de configuração de rotas não altera o estado.
---

# gerax-http (Rust)

## O que é

`gerax-http` é a camada de domínio/porta de uma arquitetura hexagonal (ports &
adapters) para servidores HTTP. Ela define **contratos** (traits) que
descrevem o que é um servidor HTTP e como ele é configurado, sem nunca saber
qual framework concreto (axum, actix-web, warp, hyper puro, etc.) vai
implementá-los. A aplicação final é quem escolhe a tecnologia, criando um
**adaptador** separado que implementa essas traits.

Sempre que for gerar código para esta skill, gere primeiro a **porta**
(traits abstratas, sem dependência de framework) e só depois, se pedido,
um **adaptador** concreto em outro módulo/crate.

## Por que a separação importa

Se a trait `HttpServer` importar `axum::Router` ou `actix_web::App`, a
abstração deixa de ser abstração — vira apenas uma casca em torno de um
framework específico, e trocar de framework exige reescrever o domínio
inteiro. Mantendo a porta livre de qualquer `use` de framework HTTP, a
aplicação pode trocar de axum para actix-web (ou usar os dois em paralelo,
por exemplo em testes) sem tocar na lógica de negócio.

## Regras de geração de código

Ao gerar ou revisar código para este módulo, siga estas regras (todas vêm da
especificação original do projeto):

1. **Zero conhecimento de framework.** Nenhum tipo, trait ou função deste
   módulo pode referenciar diretamente um crate de framework HTTP. Se o
   código gerado precisar de algo de um framework específico, isso pertence
   a um crate adaptador separado, não a `gerax-http`.
2. **Direção de dependência correta.** Adaptadores dependem da porta
   (`gerax-http`); a porta nunca depende de um adaptador. Ao adicionar um
   novo adaptador (ex: `gerax-http-axum`), ele deve importar as traits daqui
   — nunca o contrário.
3. **Estado compartilhado como parâmetro de inicialização.** O método que
   inicializa o servidor (`listen`) recebe o estado compartilhado (`S`) como
   parâmetro — o estado nunca é campo interno fixo do servidor antes disso.
4. **`listen` bloqueia até encerramento ou erro.** Semanticamente, `listen`
   só retorna quando o servidor para (`Ok(())`) ou falha (`Err(GeraxHttpError)`).
   Não modele isso como fire-and-forget.
5. **Estado seguro para concorrência.** O tipo genérico de estado deve ter
   bounds `Clone + Send + Sync + 'static` (tipicamente um wrapper fino sobre
   `Arc<...>` por dentro). Nunca assuma acesso exclusivo/single-threaded.
6. **Erros em hierarquia própria.** Nunca propague tipos de erro de
   framework (`hyper::Error`, `axum::Error`, etc.) através da porta. Modele
   um enum próprio (ex: `GeraxHttpError`) que implemente `std::error::Error`.
7. **Assincronia sempre que a plataforma suportar.** Métodos que fazem I/O
   (`listen`, e outros que a aplicação adicionar) devem ser `async`. Use
   `async-trait` se a trait precisar suportar `dyn Trait`/trait objects;
   caso contrário, `async fn` nativo na trait (Rust 1.75+) é suficiente e
   evita a dependência extra.
8. **Builder pattern + Facade.** Configuração (middlewares, opções, etc.)
   deve ser encadeável: métodos consomem e retornam `Self`. O builder atua
   como facade, escondendo a complexidade de montar a implementação
   concreta atrás de uma API fluente e uniforme.
9. **Rotas construídas a partir do estado.** A montagem de rotas deve
   derivar do estado compartilhado (ex: handlers fecham sobre uma cópia do
   estado). `configure_routes` recebe `&S` para isso.
10. **`configure_routes` tem implementação padrão no-op.** A trait
    `HttpServer` fornece um corpo default vazio para `configure_routes` —
    implementações concretas só sobrescrevem quando realmente precisam de
    configuração de rotas além da montagem padrão.

## Estrutura de referência

O arquivo `reference.rs` na raiz do crate gerax-http contém a implementação
canônica completa: hierarquia de erros, trait `HttpServer<S>`, trait
`HttpServerBuilder<S>`, um adaptador "noop" mínimo (`NoopHttpServer`) usado
só para exercitar o contrato em testes, e os testes esperados. Leia esse
arquivo antes de gerar código novo para garantir consistência de nomes e
assinaturas — reaproveite os mesmos nomes de tipos/traits a menos que o
usuário peça explicitamente para renomear.

Ao criar um projeto novo do zero, organize assim:

```
gerax-http/                  # crate da porta (esta skill)
├── Cargo.toml
└── src/
    ├── lib.rs               # re-exporta error, server, builder
    ├── error.rs             # GeraxHttpError
    ├── server.rs            # trait HttpServer<S>
    └── builder.rs           # trait HttpServerBuilder<S>

Se o usuário só quer o módulo dentro de um crate maior já existente, gere um
único módulo `gerax_http` com os mesmos arquivos internos como submódulos,
em vez de um crate separado — pergunte se não estiver claro pelo contexto
(monorepo com múltiplos crates vs. módulo dentro de uma aplicação única).

## Testes esperados

Sempre gere (ou verifique que já existe) um teste que comprove que a
implementação padrão de `configure_routes` **não altera o estado**: clone o
estado antes, chame o default no-op, e compare (`assert_eq!`) com o estado
original. Veja `default_configure_routes_does_not_mutate_state` em
`references/gerax_http_reference.rs` como modelo. Se a aplicação adicionar
lógica própria em `configure_routes` (sobrescrevendo o default), este teste
específico deixa de se aplicar a essa implementação — mas o teste do
default no-op na trait/porta continua valendo.

Outros testes úteis a considerar (nem sempre exigidos, use julgamento):
- Encadeamento do builder produz a configuração esperada (middlewares/opções
  na ordem correta).
- `listen` retorna `Err(GeraxHttpError::Config(..))` quando a configuração é
  inválida, sem chegar a bloquear.
- Um adaptador concreto de teste (não-noop) realmente deriva as rotas do
  estado passado (ex: uma rota só existe se um campo do estado permitir).

## Ao adicionar um adaptador concreto (axum, actix-web, etc.)

1. Crie um crate/módulo separado que dependa do crate da porta.
2. Implemente `HttpServer<S>` e, se fizer sentido, `HttpServerBuilder<S>`
   usando os tipos do framework escolhido *apenas dentro deste adaptador*.
3. Sobrescreva `configure_routes` só se o adaptador precisar registrar algo
   além do que os handlers já fazem ao fechar sobre o estado.
4. Implemente `listen` para: montar rotas via `self.configure_routes(&state)`,
   iniciar o servidor real do framework, e bloquear até shutdown/erro,
   convertendo os erros do framework para `GeraxHttpError` (nunca vazando o
   tipo de erro original através da porta).
5. Não exporte nada do framework subjacente na API pública do adaptador que
   force o código da aplicação a importar aquele framework diretamente —
   isso reintroduziria o acoplamento que a porta existe para evitar.
