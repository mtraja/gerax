# gerax-codec

Integração do Gerax com codec.

Fornece a trait `Codec<T>` e implementações para JSON, YAML, TOML, Protobuf e Cap'n Proto.

## Uso

```rust
use gerax_codec::{Codec, JsonCodec};

#[derive(serde::Serialize, serde::Deserialize)]
struct Message { data: String }

let codec = JsonCodec;
let bytes = codec.serialize(&Message { data: "hello".into() })?;
let msg = codec.deserialize(&bytes)?;
```

## Codecs

| Codec | Formato | Biblioteca |
|-------|---------|-----------|
| `JsonCodec` | JSON | `serde_json` |
| `YamlCodec` | YAML | `serde_yml` |
| `TomlCodec` | TOML | `toml` |
| `ProtobufCodec` | Protobuf | `prost` |
| `CapnpCodec` | Cap'n Proto | `capnp` |

## Erros

Todos os codecs retornam `CodecError`, que implementa `From` para os erros das bibliotecas suportadas, permitindo uso de `?` diretamente.

## Notas

- `CapnpSerializable` é um stub. Requer integração com `capnpc` ou gerador Rust customizado para ser funcional.
- `ProtobufCodec` exige `Message + Default` porque `prost::Message::decode` (v0.14) requer `Self: Default` internamente.
