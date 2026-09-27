# gerax-codec

Integração do Gerax com codec.

Fornece a trait `Codec<T>` e implementações para JSON, YAML, TOML, XML, MessagePack, Protobuf e Cap'n Proto.

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
| `XmlCodec` | XML | `quick-xml` |
| `MessagePackCodec` | MessagePack | `rmp-serde` |
| `ProtobufCodec` | Protobuf | `prost` |
| `CapnpCodec` | Cap'n Proto | `capnp` |
| `WincodeCodec` | binário | `wincode` |

## MessagePack

```rust
use gerax_codec::{Codec, MessagePackCodec};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Event { id: u64, tags: Vec<String> }

let codec = MessagePackCodec;
let bytes = codec.serialize(&Event { id: 1, tags: vec!["a".into()] })?;
let event: Event = codec.deserialize(&bytes)?;
assert_eq!(event.id, 1);
```

A serialização padrão usa a representação **compacta** (structs como arrays).
Para interoperabilidade com consumidores que esperam mapas nomeados, use
`MessagePackCodec::serialize_named`. A desserialização aceita ambas as formas.

## Erros

Todos os codecs retornam `CodecError`, que implementa `From` para os erros das bibliotecas suportadas, permitindo uso de `?` diretamente.

## Notas

- `CapnpSerializable` é um stub. Requer integração com `capnpc` ou gerador Rust customizado para ser funcional.
- `ProtobufCodec` exige `Message + Default` porque `prost::Message::decode` (v0.14) requer `Self: Default` internamente.
