use super::codec::{Codec, CodecError};
use capnp::message::{Allocator, Builder, HeapAllocator, ReaderOptions,};
use capnp::serialize;
use capnp::serialize::OwnedSegments;

pub trait CapnpSerializable: Sized {
    /// Pega os dados da struct Rust e preenche o Builder do Cap'n Proto.
    fn build_capnp_message(
        &self, 
        builder: &mut Builder<HeapAllocator>
    ) -> Result<(), CodecError>;

    /// Lê os dados do Reader do Cap'n Proto e constrói uma instância da struct Rust.
    fn from_capnp_reader(
        reader: &capnp::message::Reader<OwnedSegments>
    ) -> Result<Self, CodecError>;
}

pub struct CapnpCodec;

impl CapnpCodec {
    /// Serializa uma mensagem Cap'n Proto a partir do seu Builder
    pub fn serialize_builder<A: Allocator>(
        message: &mut Builder<A>,
    ) -> Result<Vec<u8>, CodecError> {
        let mut buffer = Vec::new();
        serialize::write_message(&mut buffer, message)
            .map_err(|e| CodecError(format!("Erro ao escrever Capnp: {}", e)))?;
        Ok(buffer)
    }

    /// Lógica de deserialização Zero-Copy passando uma closure para consumir o Reader
    pub fn with_reader<F, R>(bytes: &[u8], f: F) -> Result<R, CodecError>
    where
        F: FnOnce(&capnp::message::Reader<capnp::serialize::OwnedSegments>) -> Result<R, CodecError>,
    {
        let reader = serialize::read_message(bytes, ReaderOptions::new())
            .map_err(|e| CodecError(format!("Erro ao ler mensagem Capnp: {}", e)))?;
        f(&reader)
    }
}

// Exemplo adaptando para o tipo que você define no esquema Cap'n Proto
impl<T> Codec<T> for CapnpCodec
where
    T: CapnpSerializable, // Trait fictícia para ponte entre o gerador capnp e o Codec
{
    fn serialize(&self, value: &T) -> Result<Vec<u8>, CodecError> {
        let mut message = Builder::new_default();
        value.build_capnp_message(&mut message)?;
        Self::serialize_builder(&mut message)
    }

    fn deserialize(&self, bytes: &[u8]) -> Result<T, CodecError> {
        Self::with_reader(bytes, |reader| T::from_capnp_reader(reader))
    }
}

