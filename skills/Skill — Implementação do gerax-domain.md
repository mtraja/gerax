# Gerax Domain — AI Agent Skill

## Objetivo

Implementar o crate `gerax-domain`, responsável por representar de forma declarativa e independente de infraestrutura o **modelo de domínio** de uma aplicação.

O `gerax-domain` deverá permitir representar:

* Domínio;
* Entidades;
* Atributos;
* Tipos;
* Identificadores;
* Value Objects;
* Relacionamentos;
* Cardinalidades;
* Agregados;
* Regras básicas de integridade do modelo;
* Metadados.

O modelo poderá ser carregado a partir de:

* YAML;
* JSON;
* TOML;
* XML.

Todos os formatos devem produzir o **mesmo modelo interno canônico**.

O `gerax-domain` NÃO deve depender de:

* HTTP;
* REST;
* GraphQL;
* MCP;
* CQRS;
* banco de dados;
* ORM;
* framework web.

Esses componentes deverão posteriormente consumir o modelo produzido pelo `gerax-domain`.

---

# 1. Visão arquitetural

Implementar a seguinte arquitetura:

```text
                 Domain Definition
                       │
          ┌────────────┼────────────┐
          │            │            │
         YAML         JSON         TOML
          │            │            │
          └────────────┼────────────┘
                       │
                      XML
                       │
                       ▼
                   Parsers
                       │
                       ▼
                Domain Model / AST
                       │
                       ▼
                   Validator
                       │
                       ▼
                Valid Domain Model
                       │
        ┌──────────────┼──────────────┐
        ▼              ▼              ▼
      CQRS            MCP            API
    futuro           futuro          futuro
```

A regra fundamental é:

> YAML, JSON, TOML e XML são apenas formatos de entrada. O modelo de domínio interno é único.

---

# 2. Estrutura sugerida do crate

Criar:

```text
gerax-domain/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   │
│   ├── model/
│   │   ├── mod.rs
│   │   ├── domain.rs
│   │   ├── entity.rs
│   │   ├── attribute.rs
│   │   ├── relationship.rs
│   │   ├── aggregate.rs
│   │   ├── value_object.rs
│   │   ├── type_ref.rs
│   │   └── cardinality.rs
│   │
│   ├── parser/
│   │   ├── mod.rs
│   │   ├── yaml.rs
│   │   ├── json.rs
│   │   ├── toml.rs
│   │   └── xml.rs
│   │
│   ├── validation/
│   │   ├── mod.rs
│   │   └── validator.rs
│   │
│   └── error.rs
│
└── tests/
    ├── yaml.rs
    ├── json.rs
    ├── toml.rs
    ├── xml.rs
    ├── validation.rs
    └── equivalence.rs
```

A estrutura pode ser ajustada caso o workspace Gerax já possua convenções diferentes.

---

# 3. Modelo conceitual

O modelo inicial deverá ser:

```text
Domain
 │
 ├── entities
 │    ├── Entity
 │    │    ├── attributes
 │    │    ├── value_objects
 │    │    └── aggregate metadata
 │
 ├── relationships
 │    └── Relationship
 │
 └── metadata
```

---

# 4. Domain

Implementar uma estrutura equivalente a:

```rust
pub struct Domain {
    pub name: String,
    pub version: Option<String>,
    pub entities: Vec<Entity>,
    pub value_objects: Vec<ValueObject>,
    pub relationships: Vec<Relationship>,
}
```

Evitar colocar conceitos específicos de CQRS, HTTP ou MCP.

---

# 5. Entity

Implementar:

```rust
pub struct Entity {
    pub name: String,
    pub attributes: Vec<Attribute>,
    pub aggregate: Option<AggregateInfo>,
}
```

Uma entidade deve possuir:

* nome;
* atributos;
* identificação;
* possibilidade de ser aggregate root.

Não assumir que toda entidade é aggregate root.

---

# 6. Attribute

Implementar algo equivalente a:

```rust
pub struct Attribute {
    pub name: String,
    pub type_ref: TypeRef,
    pub required: bool,
    pub unique: bool,
    pub primary_key: bool,
    pub nullable: bool,
}
```

O modelo deve permitir representar:

```text
id: UUID
nome: String
email: Email
data_nascimento: Date
ativo: Boolean
```

---

# 7. TypeRef

Não limitar inicialmente os tipos a strings arbitrárias.

Criar uma abstração:

```rust
pub enum TypeRef {
    String,
    Integer,
    Decimal,
    Boolean,
    Uuid,
    Date,
    DateTime,
    Email,
    Custom(String),
}
```

Permitir posteriormente expansão para:

```text
Array<T>
Map<K,V>
Optional<T>
Enum
ValueObject
EntityReference
```

Não implementar funcionalidades futuras sem necessidade. Preparar uma estrutura extensível.

---

# 8. Value Object

O domínio deve permitir:

```yaml
value_objects:

  - name: Email

    attributes:
      - name: value
        type: string
        required: true
```

Representar:

```rust
pub struct ValueObject {
    pub name: String,
    pub attributes: Vec<Attribute>,
}
```

Um Value Object não possui identidade própria.

---

# 9. Identidade

A DSL deve permitir:

```yaml
- name: id
  type: uuid
  primary_key: true
```

O validator deve garantir:

* uma entidade deve possuir no máximo um atributo `primary_key`;
* uma entidade que exija identidade deve possuir uma chave primária;
* dois atributos não podem possuir o mesmo nome.

Não impor regras que pertençam ao banco de dados.

---

# 10. Relationships

Implementar:

```rust
pub struct Relationship {
    pub name: String,
    pub source: EntityReference,
    pub target: EntityReference,
}
```

Com:

```rust
pub struct EntityReference {
    pub entity: String,
    pub cardinality: Cardinality,
}
```

E:

```rust
pub enum Cardinality {
    One,
    ZeroOrOne,
    Many,
    OneOrMany,
}
```

---

# 11. Exemplos de relacionamentos

## 1:1

```yaml
relationships:

  - name: aluno_documento

    source:
      entity: Aluno
      cardinality: one

    target:
      entity: Documento
      cardinality: one
```

## 1:N

```yaml
relationships:

  - name: aluno_matriculas

    source:
      entity: Aluno
      cardinality: one

    target:
      entity: Matricula
      cardinality: many
```

## N:N

```yaml
relationships:

  - name: alunos_turmas

    source:
      entity: Aluno
      cardinality: many

    target:
      entity: Turma
      cardinality: many
```

---

# 12. Aggregate

Preparar o modelo para DDD:

```rust
pub struct AggregateInfo {
    pub root: bool,
}
```

Exemplo:

```yaml
- name: Aluno

  aggregate:
    root: true
```

Não implementar ainda comportamento de Aggregate, Event Sourcing ou Command Handler.

O objetivo desta fase é apenas representar a informação.

---

# 13. DSL YAML

Implementar suporte para:

```yaml
version: "1"

domain:
  name: escola

  entities:

    - name: Aluno

      aggregate:
        root: true

      attributes:

        - name: id
          type: uuid
          primary_key: true

        - name: nome
          type: string
          required: true

        - name: email
          type: Email
          required: true
          unique: true

    - name: Turma

      attributes:

        - name: id
          type: uuid
          primary_key: true

        - name: nome
          type: string
          required: true

  relationships:

    - name: aluno_turmas

      source:
        entity: Aluno
        cardinality: many

      target:
        entity: Turma
        cardinality: many
```

---

# 14. JSON

O mesmo domínio deve poder ser representado como:

```json
{
  "version": "1",
  "domain": {
    "name": "escola",
    "entities": [
      {
        "name": "Aluno",
        "aggregate": {
          "root": true
        },
        "attributes": [
          {
            "name": "id",
            "type": "uuid",
            "primary_key": true
          },
          {
            "name": "nome",
            "type": "string",
            "required": true
          }
        ]
      }
    ],
    "relationships": []
  }
}
```

---

# 15. TOML

Implementar suporte equivalente:

```toml
version = "1"

[domain]
name = "escola"

[[domain.entities]]
name = "Aluno"

[domain.entities.aggregate]
root = true

[[domain.entities.attributes]]
name = "id"
type = "uuid"
primary_key = true

[[domain.entities.attributes]]
name = "nome"
type = "string"
required = true
```

---

# 16. XML

Implementar uma representação equivalente:

```xml
<?xml version="1.0" encoding="UTF-8"?>

<domain-model version="1">

    <domain name="escola">

        <entities>

            <entity name="Aluno">

                <aggregate root="true"/>

                <attributes>

                    <attribute
                        name="id"
                        type="uuid"
                        primary-key="true"/>

                    <attribute
                        name="nome"
                        type="string"
                        required="true"/>

                </attributes>

            </entity>

        </entities>

    </domain>

</domain-model>
```

A estrutura XML pode ser adaptada às limitações/convenções da biblioteca escolhida, mas deve produzir o mesmo `Domain`.

---

# 17. Parser abstraction

Criar uma abstração comum:

```rust
pub trait DomainParser {
    fn parse(&self, source: &str) -> Result<Domain, DomainError>;
}
```

Implementar:

```rust
YamlParser
JsonParser
TomlParser
XmlParser
```

Todos devem retornar:

```rust
Result<Domain, DomainError>
```

Nunca retornar modelos diferentes por formato.

---

# 18. DomainLoader

Criar uma API de alto nível:

```rust
let domain = DomainLoader::from_file("escola.yaml")?;
```

O loader deve detectar o formato pela extensão:

```text
.yaml / .yml → YAML
.json         → JSON
.toml         → TOML
.xml          → XML
```

Também permitir especificação explícita quando necessário:

```rust
DomainLoader::from_str(source, DomainFormat::Yaml)?;
```

---

# 19. Validation

Criar um validator independente do parser:

```rust
pub trait DomainValidator {
    fn validate(&self, domain: &Domain) -> Result<(), DomainValidationError>;
}
```

Validar inicialmente:

### Domínio

* nome obrigatório;
* nome não vazio.

### Entidades

* nome obrigatório;
* nomes únicos.

### Atributos

* nome obrigatório;
* nomes únicos dentro da entidade;
* tipo válido;
* no máximo uma primary key.

### Relationships

* entidade source existe;
* entidade target existe;
* nome do relacionamento é válido;
* não permitir referência a entidade inexistente.

### Value Objects

* nomes únicos;
* atributos válidos.

---

# 20. Erros

Criar erros específicos.

Exemplo:

```rust
pub enum DomainError {
    Parse(ParseError),
    Validation(DomainValidationError),
    UnsupportedFormat,
    Io(std::io::Error),
}
```

E:

```rust
pub enum DomainValidationError {
    EmptyDomainName,
    DuplicateEntity(String),
    DuplicateAttribute {
        entity: String,
        attribute: String,
    },
    UnknownEntity(String),
    DuplicatePrimaryKey(String),
    InvalidType(String),
}
```

Os erros devem ser suficientemente informativos para agentes de IA e ferramentas CLI.

---

# 21. Equivalência entre formatos

Criar testes que demonstrem que:

```text
escola.yaml
escola.json
escola.toml
escola.xml
```

produzem modelos semanticamente equivalentes.

Exemplo:

```rust
let yaml = YamlParser::parse(...)?;
let json = JsonParser::parse(...)?;
let toml = TomlParser::parse(...)?;
let xml = XmlParser::parse(...);

assert_eq!(yaml, json);
assert_eq!(yaml, toml);
assert_eq!(yaml, xml);
```

Se detalhes de serialização impedirem igualdade direta, criar uma comparação semântica.

Este teste é obrigatório.

---

# 22. Serialização

Além de ler os quatro formatos, preparar a API para serialização:

```rust
trait DomainSerializer {
    fn serialize(
        &self,
        domain: &Domain,
    ) -> Result<String, DomainError>;
}
```

Implementações:

```text
YamlSerializer
JsonSerializer
TomlSerializer
XmlSerializer
```

Objetivo:

```text
YAML
 ↓
Domain
 ↓
JSON
```

e:

```text
XML
 ↓
Domain
 ↓
YAML
```

sem perda semântica.

---

# 23. Não acoplar a AST ao formato

Evitar:

```rust
pub struct Entity {
    pub yaml_name: ...
}
```

ou:

```rust
pub struct Entity {
    pub xml_attribute: ...
}
```

O modelo deve conhecer apenas conceitos de domínio.

Correto:

```rust
pub struct Entity {
    pub name: String,
    pub attributes: Vec<Attribute>,
}
```

O parser é responsável pela tradução.

---

# 24. Schema

Preparar suporte para validação estrutural dos documentos.

Criar futuramente:

```text
schemas/
└── gerax-domain.schema.json
```

O schema deve validar a estrutura externa antes ou durante o carregamento.

Entretanto:

> JSON Schema não substitui o DomainValidator.

JSON Schema valida estrutura.

DomainValidator valida semântica do domínio.

Exemplo:

```text
JSON Schema
    │
    ├── "name" existe?
    ├── "type" é string?
    └── "entities" é array?
    
DomainValidator
    │
    ├── entidade existe?
    ├── relacionamento aponta para entidade?
    ├── primary key é válida?
    └── nomes são únicos?
```

---

# 25. Não implementar ainda

Nesta skill NÃO implementar:

```text
Command
Query
CommandBus
QueryBus
Handler
MCP Tool
MCP Resource
REST endpoint
GraphQL
gRPC
Repository
Database migration
ORM
EventStore
Event Sourcing
OpenAPI generation
```

Esses recursos serão consumidores futuros do `gerax-domain`.

O objetivo desta skill é construir uma fundação limpa.

---

# 26. API pública desejada

Ao final, algo próximo de:

```rust
use gerax_domain::{
    Domain,
    DomainLoader,
    DomainFormat,
};

let domain = DomainLoader::from_file("escola.yaml")?;

println!("{}", domain.name);

for entity in &domain.entities {
    println!("{}", entity.name);
}
```

E:

```rust
let domain = DomainLoader::from_str(
    source,
    DomainFormat::Yaml,
)?;
```

Também deve existir acesso direto aos parsers:

```rust
let domain = YamlParser::parse(source)?;
```

---

# 27. Cargo dependencies

Utilizar apenas dependências necessárias.

Avaliar:

```toml
serde
serde_yaml
serde_json
toml
quick-xml
thiserror
```

Para schema/validação estrutural, avaliar uma biblioteca JSON Schema compatível com a versão atual do workspace.

Não adicionar dependências sem necessidade.

Respeitar as versões e convenções já utilizadas no workspace Gerax.

---

# 28. Compatibilidade com Rust

Respeitar a versão Rust definida pelo workspace.

Não utilizar recursos de versões futuras.

Seguir:

* Rust 2024;
* `rustfmt`;
* `clippy`;
* documentação pública;
* erros idiomáticos;
* `Result` em operações que podem falhar.

---

# 29. Testes obrigatórios

Implementar testes para:

### Model

```text
Domain
Entity
Attribute
ValueObject
Relationship
Cardinality
```

### Parser

```text
YAML
JSON
TOML
XML
```

### Validation

```text
domain válido
entidade duplicada
atributo duplicado
entidade inexistente
relationship inválido
primary key duplicada
tipo inválido
```

### Equivalence

```text
YAML == JSON
YAML == TOML
YAML == XML
```

### Round-trip

```text
YAML
 ↓
Domain
 ↓
JSON
 ↓
Domain
```

O modelo final deve permanecer semanticamente equivalente.

---

# 30. Exemplo completo de domínio

Criar um fixture:

```text
tests/fixtures/escola.yaml
```

Com:

```yaml
version: "1"

domain:
  name: escola

  value_objects:

    - name: Email

      attributes:
        - name: value
          type: string
          required: true

  entities:

    - name: Aluno

      aggregate:
        root: true

      attributes:

        - name: id
          type: uuid
          primary_key: true

        - name: nome
          type: string
          required: true

        - name: email
          type: Email
          required: true
          unique: true

    - name: Turma

      attributes:

        - name: id
          type: uuid
          primary_key: true

        - name: nome
          type: string
          required: true

    - name: Matricula

      attributes:

        - name: id
          type: uuid
          primary_key: true

        - name: data
          type: date
          required: true

  relationships:

    - name: aluno_matriculas

      source:
        entity: Aluno
        cardinality: one

      target:
        entity: Matricula
        cardinality: many

    - name: turma_matriculas

      source:
        entity: Turma
        cardinality: one

      target:
        entity: Matricula
        cardinality: many
```

Esse fixture deverá ser carregado corretamente por todos os parsers.

---

# 31. Critérios de aceite

A implementação somente será considerada concluída quando:

* [ ] `gerax-domain` compila;
* [ ] domínio pode ser representado por YAML;
* [ ] domínio pode ser representado por JSON;
* [ ] domínio pode ser representado por TOML;
* [ ] domínio pode ser representado por XML;
* [ ] os quatro formatos produzem o mesmo modelo interno;
* [ ] entidades são suportadas;
* [ ] atributos são suportados;
* [ ] tipos são suportados;
* [ ] primary keys são suportadas;
* [ ] Value Objects são suportados;
* [ ] Aggregates são representados;
* [ ] relacionamentos são suportados;
* [ ] cardinalidades são suportadas;
* [ ] validação semântica funciona;
* [ ] erros são claros;
* [ ] testes unitários existem;
* [ ] testes de integração existem;
* [ ] testes de equivalência entre formatos existem;
* [ ] documentação básica existe;
* [ ] `cargo fmt --check` passa;
* [ ] `cargo clippy` passa;
* [ ] `cargo test` passa;
* [ ] nenhuma dependência de HTTP, CQRS, MCP ou banco foi introduzida.

---

# 32. Ordem de implementação

Implementar em fases.

## Fase 1 — Model

Implementar:

```text
Domain
Entity
Attribute
TypeRef
ValueObject
Relationship
EntityReference
Cardinality
AggregateInfo
```

### Resultado

Modelo Rust completo e independente de parser.

---

## Fase 2 — Validation

Implementar:

```text
DomainValidator
DomainValidationError
```

### Resultado

Um `Domain` inválido não pode ser aceito.

---

## Fase 3 — YAML

Implementar primeiro YAML por ser o formato mais amigável para autoria humana.

### Resultado

```text
YAML → Domain
```

---

## Fase 4 — JSON

Implementar:

```text
JSON → Domain
```

Garantir equivalência com YAML.

---

## Fase 5 — TOML

Implementar:

```text
TOML → Domain
```

Garantir equivalência.

---

## Fase 6 — XML

Implementar:

```text
XML → Domain
```

Garantir equivalência.

---

## Fase 7 — Serialization

Implementar:

```text
Domain → YAML
Domain → JSON
Domain → TOML
Domain → XML
```

---

## Fase 8 — Loader

Implementar:

```rust
DomainLoader::from_file(...)
DomainLoader::from_str(...)
```

com detecção de formato.

---

## Fase 9 — Schema

Adicionar:

```text
gerax-domain.schema.json
```

e documentação da DSL.

---

## Fase 10 — Documentação

Criar:

```text
docs/domain-dsl.md
```

documentando:

* sintaxe;
* entidades;
* atributos;
* tipos;
* Value Objects;
* Aggregates;
* relacionamentos;
* cardinalidades;
* exemplos YAML;
* exemplos JSON;
* exemplos TOML;
* exemplos XML.

---

# 33. Regra arquitetural fundamental

O agente deve manter esta separação:

```text
                 gerax-domain
                      │
                      ▼
                 Domain Model
                      │
       ┌──────────────┼──────────────┐
       ▼              ▼              ▼
   gerax-cqrs     gerax-mcp     gerax-openapi
```

O `gerax-domain` **não deve conhecer seus consumidores**.

Os consumidores conhecem o `gerax-domain`.

Isso garante dependência direcionada:

```text
CQRS ────────► Domain
MCP ─────────► Domain
OpenAPI ─────► Domain
GraphQL ─────► Domain
HTTP ────────► Domain
```

e nunca:

```text
Domain ──────► CQRS
Domain ──────► MCP
Domain ──────► HTTP
```

---

# 34. Evolução futura

Depois que esta skill estiver concluída, outras skills poderão consumir o modelo:

```text
gerax-domain
      │
      ├── gerax-cqrs
      │      ├── Commands
      │      ├── Queries
      │      └── Events
      │
      ├── gerax-mcp
      │      ├── Tools
      │      └── Resources
      │
      ├── gerax-openapi
      │      └── REST specification
      │
      ├── gerax-graphql
      │
      └── gerax-codegen
             └── Rust code
```

A implementação do `gerax-domain` deve, portanto, priorizar **estabilidade da API, clareza do modelo e independência de infraestrutura**.

---

# Instrução final ao agente

Antes de implementar:

1. Inspecione o workspace atual do Gerax.
2. Verifique crates existentes e suas convenções.
3. Verifique versões de Rust e dependências já utilizadas.
4. Identifique se existe algum modelo semelhante que possa ser reutilizado.
5. Não duplique abstrações existentes.
6. Apresente um plano curto de implementação.
7. Implemente fase por fase.
8. Execute os testes após cada fase.
9. Não avance para CQRS, MCP ou API.
10. Ao finalizar, apresente:

* arquivos criados;
* arquivos modificados;
* API pública;
* decisões arquiteturais;
* testes executados;
* limitações conhecidas;
* próximos passos recomendados.

O resultado esperado é uma fundação chamada **`gerax-domain`**, capaz de transformar:

```text
YAML
JSON
TOML
XML
```

em um **único modelo de domínio tipado e validado**, pronto para posteriormente ser consumido por CQRS, MCP, OpenAPI, GraphQL e geração de código.
