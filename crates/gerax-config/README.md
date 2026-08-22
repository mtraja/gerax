# gerax-config

Crate de gerenciamento de configuração do framework Gerax.

## Dependências

```toml
gerax-config = { path = "../crates/gerax-config" }
```

## Features

| Feature   | Descrição                           |
|-----------|-------------------------------------|
| `default` | Ativa `toml`, `yaml` e `json`       |
| `toml`    | Suporte a arquivos `.toml`          |
| `yaml`    | Suporte a arquivos `.yaml` e `.yml` |
| `json`    | Suporte a arquivos `.json`          |
| `full`    | Ativa todas as features acima       |

```toml
gerax-config = { path = "../crates/gerax-config", features = ["toml", "json"] }
```

## Visão Geral

Todas as fontes de configuração são carregadas como `ConfigDocument` e mescladas
em ordem. A última fonte sobrescreve as anteriores.

```
Config -> ConfigBuilder -> ConfigLoader -> T
```

## API

### Config

Entrada principal do sistema de configuração.

```rust
use gerax_config::Config;

let config = Config::builder()
    .env()
    .toml("config.toml")
    .json("default.json")
    .yaml("default.yaml")
    .memory(MemorySource::new())
    .build::<AppConfig>()?;
```

Métodos:

- `Config::builder() -> ConfigBuilder` — cria um novo construtor.

---

### ConfigBuilder

Constroi configuração de forma fluida.

Métodos:

- `new() -> Self`
- `source<S: ConfigSource + 'static>(self, source: S) -> Self`
- `toml(self, path: impl Into<PathBuf>) -> Self`
- `yaml(self, path: impl Into<PathBuf>) -> Self`
- `json(self, path: impl Into<PathBuf>) -> Self`
- `env(self) -> Self` — carrega `.env`
- `system_env(self) -> Self` — carrega variáveis do sistema
- `memory(self, source: MemorySource) -> Self`
- `loader(self) -> ConfigLoader`
- `build<T: DeserializeOwned>(self) -> ConfigResult<T>`

---

### ConfigLoader

Carrega configuração diretamente.

Métodos:

- `new() -> Self`
- `with_source<S: ConfigSource + 'static>(self, source: S) -> Self`
- `with_boxed_source(self, source: Arc<dyn ConfigSource>) -> Self`
- `load_document(&self) -> ConfigResult<ConfigDocument>`
- `load<T: DeserializeOwned>(&self) -> ConfigResult<T>`

---

### ConfigDocument

Documento interno de configuração (formato JSON).

Métodos:

- `new() -> Self`
- `value(&self) -> &Value`
- `into_value(self) -> Value`
- `deserialize<T: DeserializeOwned>(&self) -> ConfigResult<T>`

---

### ConfigSource

Trait que representa uma fonte de configuração.

```rust
pub trait ConfigSource: Debug + Send + Sync {
    fn name(&self) -> &'static str;
    fn load(&self) -> ConfigResult<ConfigDocument>;
}
```

---

### Fontes

#### EnvSource

Carrega variáveis de ambiente.

```rust
EnvSource::dotenv()       // lê .env
EnvSource::system()       // variáveis do sistema
EnvSource::from_file(path)
```

Variáveis com `__` são convertidas em objetos aninhados.

```
MERX__HOST -> {"merx": {"host": ...}}
```

#### MemorySource

Configuração em memória.

```rust
MemorySource::new()
MemorySource::from_document(document)
MemorySource::from_value(&config)?
MemorySource::new().insert("server.port", 8080)?
```

#### TomlSource

```rust
TomlSource::new("config.toml")
```

#### YamlSource

```rust
YamlSource::new("config.yaml")
```

#### JsonSource

```rust
JsonSource::new("config.json")
```

---

### Erros

`ConfigResult<T> = Result<T, ConfigError>`

Variações de `ConfigError`:

- `Io { path, source }`
- `Toml { path, source }` _(requer feature `toml`)_
- `Yaml { path, source }` _(requer feature `yaml`)_
- `Json { path, source }` _(requer feature `json`)_
- `Env { path, source }`
- `Deserialize(String)`
- `InvalidConfiguration(String)`
- `Missing(String)`

---

## Exemplos

### Retaguarda Síncrona

```rust
use gerax_config::{Config, EnvSource, MemorySource};

#[derive(serde::Deserialize)]
struct AppConfig {
    server: Server,
}

#[derive(serde::Deserialize)]
struct Server {
    host: String,
    port: u16,
}

fn main() -> gerax_config::ConfigResult<()> {
    let config = Config::builder()
        .env()
        .toml("config.toml")?
        .memory(
            MemorySource::new()
                .insert("server.host", "127.0.0.1")?
                .insert("server.port", 8080)?,
        )
        .build::<AppConfig>()?;

    println!("{}:{}", config.server.host, config.server.port);
    Ok(())
}
```

### Documento Bruto

```rust
use gerax_config::{Config, ConfigDocument};

let document = Config::builder()
    .env()
    .toml("config.toml")?
    .load_document()?;

println!("{:#}", document.value());
```
