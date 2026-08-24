# Skill: melhorar a conexão PostgreSQL

Use este documento quando a tarefa for implementar melhorias em
`src/postgres/connection.rs` do crate `gerax-postgres`. O objetivo é tornar a
criação e a operação da conexão PostgreSQL configuráveis, observáveis e
adequadas para desenvolvimento e produção, sem alterar comportamento não
relacionado do workspace.

## Contexto do código

- `PostgresConnection` hoje armazena um `tokio_postgres::Client`.
- `Connection::connect()` é um trait de `gerax-db` e deve continuar disponível
  para preservar compatibilidade.
- A conexão atual carrega somente `.env` com `Config::builder().env()`, usa
  `DatabaseConfig`, conecta com `NoTls` e executa o driver em uma task
  destacada cujo erro é impresso com `eprintln!`.
- `PostgresRepository` recebe `Arc<PostgresConnection>` e usa `client()` para
  executar consultas. Preserve essa integração ou forneça uma migração local e
  completa para ela.
- `DatabaseConfig` pertence a `gerax-db`. Não faça alterações globais nesse
  tipo sem que elas sejam indispensáveis para o contrato final.

## Resultado esperado

Implemente uma conexão que possa ser criada tanto por configuração explícita
quanto pela configuração padrão da aplicação; que suporte TLS de forma
intencional; que exponha falhas do driver de modo apropriado; e que tenha
testes que não dependam de um PostgreSQL externo para os caminhos de erro e
configuração.

## Roteiro de implementação

1. Antes de editar, leia:
   - `src/postgres/connection.rs`;
   - `src/postgres/repository.rs` e `src/postgres/builder.rs`;
   - `crates/gerax-db/src/{connection,config,error}.rs`;
   - o carregador de ambiente em `crates/gerax-config/src/source/env.rs`.
   Verifique o estado do worktree e não modifique alterações de outros autores.

2. Defina um contrato de configuração pequeno e explícito.
   - Adicione uma entrada como `PostgresConnection::connect_with_config(...)`
     para que aplicações e testes não dependam de `.env`.
   - Preserve `Connection::connect()` como atalho. Ele deve considerar `.env` e
     variáveis do processo, com uma precedência documentada; a configuração do
     processo deve poder ser usada em CI e containers sem criar um arquivo.
   - Remova a ambiguidade entre `DatabaseConfig.url` e `DatabaseConfig.database`:
     não exija um campo que a conexão não usa. Se o tipo compartilhado precisar
     continuar compatível, documente ou adapte o carregamento localmente em vez
     de propagar uma exigência artificial.
   - Valide dados inválidos antes de tentar abrir a conexão, produzindo um erro
     que permita diagnosticar a configuração.

3. Faça de TLS uma decisão de configuração.
   - Não mantenha `NoTls` como comportamento implícito de produção.
   - Escolha a alternativa que se encaixe nas dependências do workspace: por
     exemplo, modo sem TLS explicitamente solicitado e modo TLS suportado por
     um conector apropriado.
   - Não introduza dependências de TLS ou alterações na URL sem confirmar sua
     compatibilidade com `tokio-postgres` e a versão Rust do workspace.
   - Documente o comportamento padrão e como habilitar cada modo.

4. Reestruture o ciclo de vida do driver PostgreSQL.
   - O futuro `connection` retornado por `tokio_postgres::connect` precisa
     continuar sendo executado enquanto o `Client` estiver em uso.
   - Substitua o `eprintln!` por observabilidade compatível com o projeto, ou
     armazene uma forma de observar a falha. Não esconda a falha em uma task
     destacada sem nenhum caminho de diagnóstico para o chamador.
   - Se introduzir um `JoinHandle`, defina claramente a política de desligamento:
     não bloqueie no `Drop` e não cancele a task enquanto clones da conexão ainda
     puderem usar o cliente.
   - Não implemente reconexão automática sem um requisito explícito; ela altera
     semântica de falhas e transações.

5. Preserve erros úteis.
   - Hoje `DbError::connection` converte erros em texto e perde a causa original.
   - Avalie uma extensão mínima e compatível de `DbError` que mantenha a cadeia
     de causas, ou encapsule contexto suficiente para distinguir configuração,
     TLS e rede.
   - Não classifique falhas de serialização ou operações de repositório como
     falhas de conexão apenas por conveniência.

6. Mantenha a API do repositório estável.
   - `PostgresConnection::client()` deve continuar suficiente para
     `PostgresRepository`, ou toda alteração correspondente deve ser aplicada e
     testada no mesmo conjunto de mudanças.
   - `PostgresRepositoryBuilder::with_connection()` deve continuar aceitando uma
     conexão previamente aberta.

## Testes e validação

- Acrescente testes unitários para a construção/configuração explícita, campos
  ausentes, URL inválida e seleção da fonte de configuração. Isole variáveis de
  ambiente e arquivos temporários para evitar interferência entre testes.
- Para testes de conectividade real, use uma estratégia opt-in (por exemplo,
  variável que aponta para PostgreSQL de teste) ou infraestrutura já existente;
  não assuma um servidor local nem inclua credenciais.
- Teste `ping()` quando houver conexão real disponível e teste o mapeamento de
  erro quando ela não estiver disponível.
- Execute `cargo test -p gerax-postgres`. Execute Clippy quando o workspace
  permitir; se ele falhar em outro crate, registre com precisão o bloqueio em
  vez de mascará-lo.
- Rode `git diff --check` limitado aos arquivos modificados antes de concluir.

## Documentação e entrega

- Atualize `README.md` do crate somente após definir a API final: variáveis de
  configuração, precedência, TLS e um exemplo compilável devem coincidir com a
  implementação.
- Informe arquivos alterados, decisões de compatibilidade, comandos executados
  e limitações restantes.
- Não altere migrações, schema do repositório, outros backends de banco ou a
  API pública global além do necessário para cumprir este objetivo.
